# Win32 CompositorDriver — design

Status: shipped (2026-05-22)
Owners: desktop-win32 native bindings
Scope: `CompositorDriver` — drives `CompositorController::Commit` via the `CommitNeeded` event with a per-tick manual-commit fallback during the ANGLE surface-resize handshake.

**TL;DR**
- `CompositorDriver` wraps `CompositorController` and subscribes to `CommitNeeded` once; it drives `Commit()` via a UI-thread fast-path (inline in the same wndproc invocation) or a dispatcher-queue drain for off-thread fires.
- Closes the idle-window caption-animation stutter defect: without `CommitNeeded`, no `Commit()` fired between animation frames when no ANGLE swap was in progress.
- During resize, `AngleDevice` calls `pause_autocommit` then `do_resize`; the matching `swap_buffers` calls `publish_and_resume_autocommit` so `visual.SetSize` and the `eglSwapBuffers` Present land on the same DWM tick.

## 1. Summary

`CompositorDriver` is owned by `Application` and plumbed as `Arc<CompositorDriver>` into every consumer (`Window`, `AngleDevice`, `CaptionButtonStrip`). The driver:

- subscribes to `CompositorController::CommitNeeded` once,
- coalesces the agile callback through an atomic latch,
- enqueues a drain onto the UI-thread `DispatcherQueue` via `TryEnqueueWithPriority(High, …)`,
- calls `Commit()` from the drain unless `autocommit_enabled` is false,
- exposes a `pause_autocommit` / `publish_and_resume_autocommit` pair so `AngleDevice` keeps `visual.SetSize` and the matching `eglSwapBuffers` Present in the same atomic batch during resize.

Resulting publish surface:

| Site | When |
|---|---|
| `AngleDevice::swap_buffers` | every paint; also the resize fallback after the pause-gate |
| `CommitNeeded` handler (fast-path / drain) | all other publishing — hover, press, DPI, appearance, backdrop, etc. |

Publishing surface: `AngleDevice::swap_buffers` is the sole explicit `Commit()` site outside the driver. All other publishing flows through the `CommitNeeded` handler — UI-thread fast-path or dispatcher drain.

## 2. Goals

- Close the idle-window caption-animation stutter defect: caption-button hover-fade animations stuttered on idle windows because no `eglSwapBuffers` drove `Commit()` between animation frames.
- Concentrate the publish surface from 17 scattered `Commit()` sites to one (`swap_buffers`) plus the driver-internal fast-path / drain.
- Keep live-resize visually identical: `visual.SetSize` + `eglSwapBuffers` Present still publish atomically on the same DWM tick via the pause-gate.
- Stay small: one new file, ≈100 lines, no new dispatcher infrastructure.

## 3. Non-goals

- Visual-tree reorder + `RelativeSizeAdjustment` work — independent. Dropping `CompositorController` for auto-commit was tried and reverted: the visual-tree reorder it required introduced whole-window Mica flicker during resize (DXGI flip-model resize glitch under `EGL_EXPERIMENTAL_PRESENT_PATH_FAST_ANGLE`, exposed without the controller's atomic batching).
- `glFinish` removal in `swap_buffers` — tracked in TODO.md. Removing it may surface the flip-model resize glitch; investigate before any removal.
- Per-window controllers — see §7.1.
- Composition Swapchain API — deferred.

## 4. Architecture

```
Application
 ├── dispatcher_queue_controller: DispatcherQueueController
 └── compositor_driver: Arc<CompositorDriver>

Window        → holds Compositor (visual-creation only; never calls Commit())
AngleDevice   → holds Arc<CompositorDriver>; calls pause / publish_and_resume around resize + swap
CaptionButtonStrip → takes &Compositor at construction; never calls Commit()
```

The driver subscribes to `CommitNeeded` once. The agile callback captures `Weak<CompositorDriver>` to avoid a reference cycle. The drain runs on the UI thread via `DispatcherQueue::TryEnqueueWithPriority(High, …)` and is the sole site that calls `controller.Commit()` outside the resize handshake.

## 5. Component

File: `native/desktop-win32/src/win32/compositor_driver.rs`.

```rust
pub struct CompositorDriver {
    controller: CompositorController,
    dispatcher_queue: DispatcherQueue,
    ui_thread_id: u32,
    autocommit_enabled: AtomicBool,
    enqueue_pending: AtomicBool,
    commit_needed_token: AtomicI64,
}

static_assertions::assert_impl_all!(CompositorDriver: Send, Sync);
```

Public API:

```rust
pub fn new(controller: &CompositorController, dispatcher_queue: DispatcherQueue) -> anyhow::Result<Arc<Self>>;
pub fn compositor(&self) -> anyhow::Result<Compositor>;
pub fn pause_autocommit(&self);
pub fn publish_and_resume_autocommit(&self) -> anyhow::Result<()>;
pub fn shutdown(&self);
```

### 5.1 Coalescing and fast-path model

`on_commit_needed` (agile, may run on any thread) branches on `GetCurrentThreadId() == ui_thread_id`:

- **Fast-path** (UI thread): call `drain()` inline. Publishes within the same wndproc invocation as the triggering mutation.
- **Off-thread**: `enqueue_pending.swap(true, AcqRel)`. If already `true`, an enqueue is pending — do nothing. Otherwise `TryEnqueueWithPriority(High, drain_handler)`. On `Ok(false)` / `Err` (queue shut down), clear the latch.

`drain()`:
1. `store(false, Release)` on `enqueue_pending` *before* `Commit()`. A `CommitNeeded` fired during Commit re-enters `on_commit_needed`; the latch swap sees `false` and enqueues a fresh drain. Every dirty cycle gets at least one Commit.
2. `load(Acquire)` on `autocommit_enabled` — skips Commit if the resize handshake has paused publishing.
3. Calls `controller.Commit()`, logs on failure.

`autocommit_enabled` ordering: `Release` on `pause_autocommit`'s store, `AcqRel` on `publish_and_resume_autocommit`'s swap, `Acquire` on the drain's load. This pairing ensures the drain sees the pause before Commit runs.

### 5.2 Subscription lifetime

Two-step construction: build `Arc` with `commit_needed_token: AtomicI64::new(0)`, downgrade to `Weak`, subscribe, store the returned token. Zero is a safe "unset" sentinel — WinRT documents *"A valid reference will not have a value of zero"* ([Windows.Foundation.EventRegistrationToken.Value](https://learn.microsoft.com/uwp/api/windows.foundation.eventregistrationtoken)). `shutdown()` swaps to 0 and calls `RemoveCommitNeeded` only when the prior value was non-zero — second call (Drop after explicit shutdown) is a no-op.

### 5.3 Send / Sync

`static_assertions::assert_impl_all!(CompositorDriver: Send, Sync)`. `CompositorController` and `DispatcherQueue` are agile WinRT types; the `windows-0.62.2` binding emits explicit `unsafe impl Send/Sync for CompositorController {}`. The driver uses `Arc` (not `Rc`) because the `CommitNeeded` `TypedEventHandler` closure must be `Send + 'static` and captures `Weak<Self>`. `Rc::Weak` is `!Send`.

### 5.4 Dispatcher priority choice

`TryEnqueueWithPriority(High, …)` is a preference, not a requirement. High is chosen for frame-pacing alignment: `CommitNeeded` signals dirty state that wants to ship before the next DWM present, and the High band already carries system frame-cadence work. [`Application::invoke_on_dispatcher_queue`](../../src/win32/application.rs#L46-L55) deliberately stays at Normal — application work should not preempt input/composition.

## 6. Component integration

### 6.1 `Application`, build plumbing, lifecycle

`Application` owns `compositor_driver: Arc<CompositorDriver>` alongside `dispatcher_queue_controller: DispatcherQueueController`. The `DispatcherQueue` is plumbed explicitly to the driver because `Compositor::DispatcherQueue` (build 20348) is above the toolkit's minimum target (build 17763).

Teardown: `Application::shutdown` (called from Kotlin's normal close path):

```rust
pub fn shutdown(&self) -> anyhow::Result<()> {
    self.compositor_driver.shutdown();
    let _ = self.dispatcher_queue_controller.ShutdownQueueAsync()?;
    Ok(())
}
```

`CompositorDriver::Drop` calls `shutdown()` — idempotent if explicit shutdown already ran. Any `TryEnqueueWithPriority` racing with queue teardown returns `Ok(false)` and clears the latch.

Build dependencies:
- `static_assertions = "1"` in `[dependencies]` in `native/desktop-win32/Cargo.toml`.
- `"Win32_System_Threading"` in the `windows` feature list (for `GetCurrentThreadId`).
- `pub mod compositor_driver;` in `native/desktop-win32/src/win32/mod.rs`.

### 6.2 `Window`

`Window` owns `compositor: Compositor`. The compositor is created by the driver via `compositor()`. `Window` does not call `Commit()` — backdrop mutations rely on the `CommitNeeded` fast-path.

- `add_visual` uses `self.compositor.CreateSpriteVisual()?` directly.
- `set_backdrop_tint` / `remove_backdrop_tint` — `CommitNeeded` fast-path publishes inline; no explicit `Commit()`.
- `set_content_top_offset` mutations fire `CommitNeeded`; the driver's UI-thread fast-path publishes inline before the wndproc handler returns.

### 6.3 `AngleDevice`

`AngleDevice` owns `Arc<CompositorDriver>`. `resize_surface` and `swap_buffers` wrap the pause-gate so an error path resumes the gate:

```rust
pub fn resize_surface(&self, ...) -> anyhow::Result<EglSurfaceData> {
    self.compositor_driver.pause_autocommit();
    self.do_resize(...).inspect_err(|_| {
        let _ = self.compositor_driver.publish_and_resume_autocommit();
    })
}

pub fn swap_buffers(&self) -> anyhow::Result<()> {
    self.do_swap().inspect_err(|_| {
        let _ = self.compositor_driver.publish_and_resume_autocommit();
    })
}
```

`do_resize` runs EGL calls first (fallible), then `visual.SetSize` last — an EGL error returns without queueing an orphan visual-size mutation. `do_swap` calls `compositor_driver.publish_and_resume_autocommit()` as its last step: if the gate was paused (prior value `false`), it Commits and atomically publishes the new `visual.SetSize` + the `eglSwapBuffers` Present on the same DWM tick. If the gate was already open (non-resize swap), it skips the extra Commit and lets `CommitNeeded` handle any pending mutations.

**ANGLE present-then-resize invariant.** [`eglPostSubBufferNV`](https://chromium.googlesource.com/angle/angle/+/HEAD/extensions/EGL_ANGLE_post_sub_buffer.txt) issues a `Present1` at the OLD swapchain size before `IDXGISwapChain::ResizeBuffers` runs. `pause_autocommit` keeps the WUC visual at OLD size for the duration of `do_resize`, so the OLD-size Present1 composes 1:1 into the OLD-size visual rect. The matching `swap_buffers` publishes the NEW size on the same DWM tick as the Skia draw that fills it.

**Kotlin-side gap.** `resize_surface` and `swap_buffers` are separate FFI calls. If Kotlin code between them panics or skips `swap_buffers`, the gate stays paused — all subsequent drain Commits skip. Self-heals on the next successful `swap_buffers`. See §7.9.

**`AngleDevice::Drop`** calls `publish_and_resume_autocommit` before the EGL teardown so a `Drop` between `pause_autocommit` and the matching `swap_buffers` (device-loss recovery, window close mid-resize) does not strand the shared `CompositorController` gate in the paused state for surviving windows.

### 6.4 `CaptionButtonStrip`

The strip takes `&Compositor` at construction; no field stored. Mutations fire `CommitNeeded` and publish via the driver's fast-path on the UI thread.

Methods that do not need `Commit()` return `()`. `on_pointer_up` returns `Option<CaptionButtonAction>`. `on_dpi_change` returns `anyhow::Result<()>` because `glyph_surface.Resize()` is independently fallible.

## 7. Risks and known limitations

### 7.1 Multi-window cross-resume race (HIGH, accepted)

One `CompositorController` is shared across every `Window`. Window A's `pause_autocommit` is a global flag; Window B's `swap_buffers.publish_and_resume_autocommit` re-enables the gate. If A's `visual.SetSize` is dirty and B's `Commit()` lands before A's matching `swap_buffers`, A's resize publishes prematurely — one-frame Mica flash. Accepted; the race window is single-frame and only triggers when two windows resize concurrently. Fix: promote `autocommit_enabled` to a pause-counter plus per-`AngleDevice` ownership flag — deferred.

### 7.2 Tactile press latency (LOW)

Press-down mutates → `CommitNeeded` fires → `on_commit_needed` (`GetCurrentThreadId() == ui_thread_id`) → `drain()` → `Commit()` — all inline on the same wndproc invocation. If `CommitNeeded` fires off-thread instead of synchronously, the dispatcher round-trip costs ≤ 1 message-pump iteration — well below the 6.9 ms 144Hz frame budget.

### 7.3 Commit cadence under resize (MEDIUM)

While the gate is paused, `CommitNeeded` continues firing for unrelated mutations (caption hover animation started mid-drag). The drain runs but skips the Commit. Pending animation publishes on the next `swap_buffers`. Worst case: one-DWM-tick start lag on an animation that began during a resize tick.

### 7.4 Commit failure inside drain (LOW)

`drain` logs Commit errors and returns. D2D device-loss recovery is independent of the WUC commit path (handled in `composition.rs`). A purely WUC-side Commit failure would log-spin until the next mutation fires `CommitNeeded` afresh. WUC controller Commit failures in practice indicate an unrecoverable compositor device.

### 7.5 Subscription teardown vs in-flight drain (LOW)

A drain enqueued just before `RemoveCommitNeeded` still runs and calls `Commit()` once more. Benign no-op. Once the queue enters shutdown (`ShutdownQueueAsync` fired), subsequent `TryEnqueueWithPriority` returns `Ok(false)` and clears the latch.

### 7.6 Subscription-time fire (LOW, assumption)

`CommitNeeded` is assumed not raised at subscription time. If it were: the handler upgrades `Weak` (alive), reads `autocommit_enabled = true`, enqueues a drain. The drain calls `Commit()` on a clean queue (no-op). The `enqueue_pending` latch coalesces any fires before `store(token)` completes. Safe under both interpretations.

### 7.7 Single-window pause reentrancy (LOW)

`pause_autocommit` is a single bool — two paused callers coalesce. Reentrancy is unlikely in current code (Win32 wndproc is single-threaded and `resize_surface` is called from Kotlin's `performDrawing`). The first resume re-opens the gate; the next `swap_buffers` heals any mis-sequenced publish.

### 7.8 Strip init publish lag (LOW)

`CaptionButtonStrip::new` does not explicitly Commit. The strip becomes visible on the next drain — typically before the first `WM_PAINT`. Hit-testing is unaffected (reads struct geometry, not the WUC tree).

### 7.9 Pause-leak across the Rust/Kotlin FFI boundary (HIGH)

If Kotlin panics, throws, or skips `swap_buffers`, the gate stays paused. Effect: caption-button animations and backdrop mutations are stranded until the next successful `swap_buffers`.

Healing paths:
- **Common case** (recoverable Kotlin error): next `WM_PAINT` → `performDrawing` → `resize_surface` (re-pauses idempotently) → draw → `swap_buffers` → `publish_and_resume_autocommit`. Self-heals in one frame.
- **Stuck case** (no further paints): gate stays paused for the lifetime of the `Application`.

## 8. Alternatives considered

- **A. Pause counter (`AtomicUsize` + per-`AngleDevice` ownership flag).** Eliminates the §7.1 multi-window race. Adds ~30 LoC. Future work when a real multi-window scenario emerges.
- **B. Drop `CompositorController` entirely (auto-commit).** Reverted — see §3 non-goal.
- **C. Keep all 17 explicit Commits and add `CommitNeeded` as a safety net only.** Rejected — does not fix the idle-animation stutter, just plugs the idle-animation hole.
- **D. `DispatcherQueue::GetForCurrentThread()` instead of plumbing.** Returns null on threads without a queue. Plumbing the `Application`-owned queue is explicit. `Compositor::DispatcherQueue()` is unavailable at the toolkit's minimum target (build 17763; the API requires build 20348).
- **E. Per-frame transparent gutter clear** (Electron pattern). Deferred; apply if resize-edge artifacts surface.
- **F. `ICompositorInterop::CreateCompositionSurfaceForSwapChain`.** Eliminates the manual `SetContent` cycle. Requires opening ANGLE's underlying `IDXGISwapChain`, hidden behind the EGL abstraction, or replacing ANGLE with an app-owned D3D11 path. Deferred.

## 9. Validation

- **Idle-animation smoke**: hover a caption button on a window with no active ANGLE rendering. The 150 ms `ColorKeyFrameAnimation` backplate fade must run at smooth 60 Hz cadence.
- **Fast-path firing semantics**: instrument `on_commit_needed` to record UI-thread vs off-thread fires. Expectation: ≥ 95% UI-thread for UI-thread-triggered mutations.
- **Resize regression**: slow edge drag, rapid drag, maximize/restore, DPI change mid-drag, Aero Snap, Aero Shake. Pass: no observable backdrop flicker, no anchor drift, no Mica gutter beyond a single frame at the trailing edge.
- **Caption-button latency**: hover-fade start, press-down feedback, release. Pass: no perceptible delay relative to the wndproc dispatch cycle.
- **Induced resize-error gate-leak smoke**: force `egl::Instance::swap_interval` or `eglSwapBuffers` to return Err on one frame; confirm gate is restored and the next paint Commits normally.
- **Shutdown smoke**: open and close a sample window 100 times; confirm no `CommitNeeded` callback fires after `Application` drops and no dangling `RemoveCommitNeeded` panic.
- `gradlew lint` clean — `.git-hooks/pre-push` runs `./gradlew lint`.

## 10. References

- [`CompositorController.CommitNeeded`](https://learn.microsoft.com/uwp/api/windows.ui.composition.core.compositorcontroller.commitneeded)
- [`CompositorController.Commit`](https://learn.microsoft.com/uwp/api/windows.ui.composition.core.compositorcontroller.commit)
- [`DispatcherQueue.TryEnqueueWithPriority`](https://learn.microsoft.com/uwp/api/windows.system.dispatcherqueue.tryenqueuewithpriority)
- [`CreateDispatcherQueueController`](https://learn.microsoft.com/windows/win32/api/dispatcherqueue/nf-dispatcherqueue-createdispatcherqueuecontroller)
- [`Windows.Foundation.EventRegistrationToken.Value`](https://learn.microsoft.com/uwp/api/windows.foundation.eventregistrationtoken)
- `RenderingDeviceReplacedRegistration` in `composition.rs` — RAII registration wrapper this spec follows for `CommitNeeded` token revocation (extended with an idempotent take for explicit shutdown).
- `glFinish before every eglSwapBuffers` in TODO.md — perf concern; do not address independently of this spec (§3 non-goal).
