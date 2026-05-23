# Win32 CompositorDriver — design

Status: draft (2026-05-22)
Owners: desktop-win32 native bindings
Scope: introduce a `CompositorDriver` component that drives `CompositorController::Commit` via the `CommitNeeded` event (effective auto-commit), with a per-tick manual-commit fallback during the ANGLE surface-resize handshake. Concentrates 17 scattered `Commit()` call-sites to one: the `swap_buffers` resize-fallback publish. The `CommitNeeded` handler runs `Commit()` inline when invoked on the UI thread (`GetCurrentThreadId` fast-path) and falls back to a dispatcher-queue drain otherwise.

A prior attempt — dropping `CompositorController` entirely in favour of an auto-commit `Compositor` and reordering visual mutations to land after Present — was implemented and failed: it introduced whole-window Mica flicker during resize (the DXGI flip-model "resize glitch" with `EGL_EXPERIMENTAL_PRESENT_PATH_FAST_ANGLE`, masked today by `glFinish` and the controller's atomic batching). This spec is the lighter alternative that keeps the controller. The fix it targets is the same defect surfaced by [TODO.md:85–89](../TODO.md) — `ColorKeyFrameAnimation` stutter on idle windows because no `Commit()` fires between animation frames. The mechanism here: keep the controller, subscribe to `CommitNeeded`, marshal back to the UI thread via the existing `DispatcherQueue`, and call `Commit()` from the drain. The system compositor's own frame thread advances animations once it has been committed.

## 1. Summary

Today, 17 sites in `native/desktop-win32/src/win32/` call `CompositorController::Commit()` directly: 13 in `caption_buttons.rs` (init, hover, press, release, cancel, leave, activate, dpi, appearance, device-replaced, max-state, resize), 1 in `renderer_angle.rs::AngleDevice::swap_buffers`, and 3 in `window.rs` (`set_backdrop_tint`, `remove_backdrop_tint`, `commit_composition`). One shared `CompositorController` lives on `Application` next to a `DispatcherQueueController` created on the UI thread (see [application.rs:23–26,70–86](../../src/win32/application.rs)).

The new design introduces `CompositorDriver`, owned by `Application` and plumbed as `Arc<CompositorDriver>` into every consumer (`Window`, `AngleDevice`, `CaptionButtonStrip`). The driver:

- subscribes to `CompositorController::CommitNeeded` once,
- coalesces the agile callback through an atomic latch,
- enqueues a drain onto the UI-thread `DispatcherQueue` via `TryEnqueueWithPriority(High, …)` (frame-pacing preference, not required by the API — see §5.5),
- calls `Commit()` from the drain unless `autocommit_enabled` is false,
- exposes a `pause_autocommit` / `publish_and_resume_autocommit` pair so `AngleDevice` can keep `visual.SetSize` and the matching `eglSwapBuffers` Present in the same atomic batch during a resize tick. `publish_and_resume_autocommit` is the only resume path: it swap-resumes the gate and Commits only when the gate had been paused, so non-resize swaps don't pay an extra Commit.

Resulting publish surface:
- **`AngleDevice::swap_buffers`** — always-on publisher on every paint (also the resize fallback after the pause-gate). Sole explicit `Commit()` call-site outside the driver.
- **`CommitNeeded` handler with UI-thread fast-path** — drives all other publishing. When the agile fire lands on the UI thread (the common case for UI-thread mutations triggering the dirty queue), the handler `Commit()`s inline in the same wndproc invocation. When it lands off-thread, it marshals to the UI thread via `DispatcherQueue.TryEnqueueWithPriority(High, …)`. Both paths converge on the same `drain()` function.
- **`Window::commit_composition` helper — deleted.** No event-handler-boundary publish needed: `set_content_top_offset` and `strip.on_resize` mutations after the Kotlin draw chain fire `CommitNeeded` on the UI thread, which the fast-path Commits inline before the wndproc returns.
- **All 17 `CompositorController::Commit()` call-sites except `swap_buffers`** — removed. Interactive strip mutations (hover, press, release, cancel, leave, activate, appearance), init `InsertAtTop`, event-handler-boundary mutations (dpi, max-state, device-replaced, resize), and backdrop tint set/remove all rely on `CommitNeeded` + fast-path for publishing.

Net Commit sites: 17 → 1 explicit + 1 driver-internal fast-path/drain entry point. Stale comments at [event_loop.rs:472-477](../../src/win32/event_loop.rs#L472-L477) and [window.rs:440-446](../../src/win32/window.rs#L440-L446) are updated to describe the new publish ownership (fast-path inline Commit instead of synchronous-helper Commit).

## 2. Goals

- Close the [TODO.md:85–89](../TODO.md) defect: caption-button hover-fade animations stutter on idle windows because no `eglSwapBuffers` drives `Commit()` between animation frames.
- Concentrate the explicit publish surface from 17 scattered `Commit()` sites to one (`swap_buffers`) plus the driver-internal fast-path / drain. All other publishing is `CommitNeeded`-driven, with a `GetCurrentThreadId`-based fast-path that publishes inline on UI-thread mutations so chrome doesn't lag the ANGLE frame.
- Keep live-resize visually identical or better: the ANGLE `visual.SetSize` + `eglSwapBuffers` Present pair still publishes atomically on the same DWM tick via the pause-gate.
- Stay small. One new file, ≈100 lines. No visual-tree reordering. No new dispatcher infrastructure (the `DispatcherQueueController` already exists).

## 3. Non-goals

- The visual-tree reorder + `RelativeSizeAdjustment` work — independent.
- `glFinish` removal in `swap_buffers` — out of scope here. Tracked at [TODO.md:168](../TODO.md) as a perf concern. The prior failed auto-commit attempt suggested removing it might surface a DXGI flip-model resize glitch (back-buffers uninitialised after `IDXGISwapChain::ResizeBuffers` under `EGL_EXPERIMENTAL_PRESENT_PATH_FAST_ANGLE`); that mechanism is unverified — investigate before any removal.
- Per-window controllers — see §7.1.
- Composition Swapchain API / atomic Present primitives — deferred future option.

## 4. Architecture

```
Application
 ├── dispatcher_queue_controller: DispatcherQueueController  (existing)
 └── compositor_driver: Arc<CompositorDriver>                (new — owns the controller + the gate + CommitNeeded subscription)

Window           → holds Compositor (no driver — Window's needs are visual-creation only)
AngleDevice      → holds Arc<CompositorDriver>; calls pause / publish_and_resume around the resize + swap handshake
CaptionButtonStrip → takes &Compositor at construction (no field stored); never calls Commit()
```

The driver subscribes to `CommitNeeded` once. The agile callback is captured as `Weak<CompositorDriver>`, never `Arc` (no cycle). The drain runs on the UI thread via `DispatcherQueue::TryEnqueueWithPriority(High, …)` and is the sole site that calls `controller.Commit()` outside the resize handshake.

## 5. Component

File: `native/desktop-win32/src/win32/compositor_driver.rs`.

```rust
use std::sync::{Arc, Weak, atomic::{AtomicBool, AtomicI64, Ordering}};
use anyhow::Context;
use windows::{
    Foundation::TypedEventHandler,
    System::{DispatcherQueue, DispatcherQueueHandler, DispatcherQueuePriority},
    UI::Composition::{Compositor, Core::CompositorController},
    Win32::System::Threading::GetCurrentThreadId,
};

pub struct CompositorDriver {
    controller: CompositorController,
    dispatcher_queue: DispatcherQueue,
    ui_thread_id: u32,
    autocommit_enabled: AtomicBool,
    enqueue_pending: AtomicBool,
    commit_needed_token: AtomicI64,
}

static_assertions::assert_impl_all!(CompositorDriver: Send, Sync);

impl CompositorDriver {
    pub fn new(
        controller: &CompositorController,
        dispatcher_queue: DispatcherQueue,
    ) -> anyhow::Result<Arc<Self>> {
        let driver = Arc::new(Self {
            controller: controller.clone(),
            dispatcher_queue,
            ui_thread_id: unsafe { GetCurrentThreadId() },
            autocommit_enabled: AtomicBool::new(true),
            enqueue_pending: AtomicBool::new(false),
            commit_needed_token: AtomicI64::new(0),
        });
        let weak = Arc::downgrade(&driver);
        let handler = TypedEventHandler::new(move |_, _| {
            if let Some(driver) = weak.upgrade() {
                driver.on_commit_needed();
            }
            Ok(())
        });
        let token = controller
            .CommitNeeded(&handler)
            .context("CompositorController::CommitNeeded subscribe")?;
        driver.commit_needed_token.store(token, Ordering::Release);
        Ok(driver)
    }

    pub fn compositor(&self) -> anyhow::Result<Compositor> {
        Ok(self.controller.Compositor()?)
    }

    pub fn pause_autocommit(&self) {
        self.autocommit_enabled.store(false, Ordering::Release);
    }

    pub fn publish_and_resume_autocommit(&self) -> anyhow::Result<()> {
        if !self.autocommit_enabled.swap(true, Ordering::AcqRel) {
            self.controller.Commit()?;
        }
        Ok(())
    }

    pub fn shutdown(&self) {
        let token = self.commit_needed_token.swap(0, Ordering::AcqRel);
        if token != 0 {
            if let Err(e) = self.controller.RemoveCommitNeeded(token) {
                log::warn!("CompositorDriver::RemoveCommitNeeded failed: {e}");
            }
        }
    }

    fn on_commit_needed(self: &Arc<Self>) {
        if unsafe { GetCurrentThreadId() } == self.ui_thread_id {
            self.drain();
            return;
        }
        if self.enqueue_pending.swap(true, Ordering::AcqRel) {
            return;
        }
        let weak = Arc::downgrade(self);
        let handler = DispatcherQueueHandler::new(move || {
            if let Some(driver) = weak.upgrade() {
                driver.drain();
            }
            Ok(())
        });
        if !matches!(
            self.dispatcher_queue
                .TryEnqueueWithPriority(DispatcherQueuePriority::High, &handler),
            Ok(true)
        ) {
            self.enqueue_pending.store(false, Ordering::Release);
        }
    }

    fn drain(&self) {
        self.enqueue_pending.store(false, Ordering::Release);
        if self.autocommit_enabled.load(Ordering::Acquire) {
            if let Err(e) = self.controller.Commit() {
                log::warn!("CompositorDriver drain Commit failed: {e}");
            }
        }
    }
}

impl Drop for CompositorDriver {
    fn drop(&mut self) {
        self.shutdown();
    }
}
```

### 5.1 Coalescing and fast-path model

`on_commit_needed` (agile, may run on any thread) branches on a `GetCurrentThreadId() == ui_thread_id` check:

- **Fast-path** (UI thread): call `drain()` inline. Skips the dispatcher round-trip and publishes within the same wndproc invocation as the triggering mutation. Doesn't latch — if `Commit()` re-fires `CommitNeeded` synchronously, the reentrant `on_commit_needed` call sees the same thread again and would Commit a clean queue (no-op, cheap). The latch in `drain()` itself short-circuits any inner enqueue attempt.
- **Off-thread**: `enqueue_pending.swap(true, AcqRel)`. If already `true`, an enqueue is pending — do nothing. Otherwise `TryEnqueueWithPriority(High, drain_handler)`. On `Ok(false)` / `Err` (queue shut down), clear the latch.

Both paths converge on `drain()`, which:
- `store(false, Release)` on `enqueue_pending` *before* `Commit()`. A `CommitNeeded` fired during the Commit re-enters `on_commit_needed`; the off-thread path's latch swap sees `false` and enqueues a fresh drain. Net guarantee: every dirty cycle gets at least one Commit.
- `load(Acquire)` on `autocommit_enabled` — skips Commit if the resize handshake has paused publishing.
- Calls `controller.Commit()`, logs on failure.

The latch's correctness rests on the atomic swap *returning the prior value*, not on a happens-before edge between threads. The latch alone could use `Relaxed`. `AcqRel` on the swap is defensive — negligible cost.

`autocommit_enabled` IS load-bearing: the drain reads it after a `TryEnqueue` cross-thread hop (off-thread path) or on the UI thread (fast-path), and the UI-thread writers (`pause_autocommit` / `publish_and_resume_autocommit`) need their mutation visible to that drain. `Release` on `pause_autocommit`'s store, `AcqRel` on `publish_and_resume_autocommit`'s swap, paired with `Acquire` on the drain's load. This pairing is the real synchronisation in the design.

### 5.2 Subscription lifetime

`Arc<Self>` is constructed before the subscription token exists. Two-step pattern: build the `Arc` with `commit_needed_token: AtomicI64::new(0)`, downgrade to `Weak`, subscribe with a handler that captures the `Weak`, store the returned token.

Zero is a safe "unset" sentinel: WinRT documents *"A valid reference will not have a value of zero"* ([Windows.Foundation.EventRegistrationToken.Value](https://learn.microsoft.com/uwp/api/windows.foundation.eventregistrationtoken)). `shutdown()` swaps to 0 and only calls `RemoveCommitNeeded` when the prior value was non-zero — second call (Drop after explicit shutdown) is a no-op.

Slightly different from [composition.rs:229–242](../../src/win32/composition.rs#L229) `RenderingDeviceReplacedRegistration`, which unconditionally removes the registration in its own `Drop`; here we add an idempotent take-pattern so explicit shutdown can revoke first and the `Drop` is a no-op afterwards.

### 5.3 Send / Sync

`static_assertions::assert_impl_all!(CompositorDriver: Send, Sync)` asserts the struct is agile. `CompositorController` and `DispatcherQueue` are agile WinRT types (`Threading(Both)` / `MarshalingBehavior(Agile)`); the `windows-0.62.2` binding emits explicit `unsafe impl Send/Sync for CompositorController {}`. No `Rc` / `Cell` / `RefCell` on this struct.

The driver is reference-counted via `Arc`, not `Rc`, even though `Application` itself is UI-thread-only. Reason: the `CommitNeeded` `TypedEventHandler` closure must be `Send + 'static` (the binding signature is `Fn(Ref<TSender>, Ref<TResult>) -> Result<()> + Send + 'static`), and it captures `Weak<Self>` to break the reference cycle. `Rc::Weak` is `!Send`, so the closure would not satisfy the bound. `Arc::Weak` does.

### 5.4 No Arc cycle

The `TypedEventHandler` closure captures `Weak<CompositorDriver>`, not `Arc`. The driver field that holds the controller is an `Arc<CompositorDriver>` → owns one `CompositorController` clone, but the controller's event-list holds the handler which holds only a `Weak` back. The drain closure inside `TryEnqueueWithPriority` also captures `Weak`. No reference cycle.

### 5.5 Dispatcher priority choice

`TryEnqueueWithPriority(High, …)` is a **preference**, not a requirement. MS docs ([CompositorController.Commit](https://learn.microsoft.com/uwp/api/windows.ui.composition.core.compositorcontroller.commit)) impose no priority constraint on `Commit()` — only the implicit "on the compositor's dispatcher-queue thread" rule. The default `TryEnqueue` (Normal priority) is functionally correct.

High is chosen for frame-pacing alignment: `CommitNeeded` signals dirty state that wants to ship before the next DWM present. The High band already carries system frame-cadence work (input dispatch, system backdrops plumbing when `EnsureSystemDispatcherQueue` is in play). Posting Commit drains in the same band keeps frame-affecting work clustered rather than queued behind arbitrary app callbacks. The existing general-purpose path [`Application::invoke_on_dispatcher_queue`](../../src/win32/application.rs#L46-L55) deliberately stays at the default Normal — application work should not preempt input/composition.

If telemetry shows the drain is starved by other High-band traffic, fall back to Normal; if Commit work needs to back off under load, fall back to Low. Neither is currently observed.

## 6. Integration

### 6.1 `Application`, build plumbing, lifecycle

`Application::new` already creates `dispatcher_queue_controller` then `compositor_controller`. Replace `compositor_controller` with `compositor_driver` (the controller is moved into the driver's constructor and no longer needs a direct `Application` field — every consumer now goes through the driver):

```rust
pub struct Application {
    compositor_driver: Arc<CompositorDriver>,
    dispatcher_queue_controller: DispatcherQueueController,
    event_loop: Rc<EventLoop>,
}
```

Constructed via `CompositorDriver::new(&compositor_controller, dispatcher_queue_controller.DispatcherQueue()?)?` — the driver clones the controller internally; the local `compositor_controller` is dropped after the constructor returns. The queue must be plumbed explicitly — `Compositor::DispatcherQueue` would be the natural derivation but it was introduced in Windows 10 21H2 (build 20348), and this toolkit supports down to build 17763.

`Application::create_angle_device` passes `Arc::clone(&self.compositor_driver)` into the constructed `AngleDevice`. `Application::new_window` extracts a `Compositor` from the driver — `self.compositor_driver.compositor()?` — and passes that to `Window::new` (Window doesn't need driver methods, only `Compositor` for visual creation; see §6.2).

**Teardown ordering.** No `impl Drop for Application` is required. `CompositorDriver`'s own `Drop` calls `shutdown()` when the last `Arc` clone (held by `Application`, `AngleDevice`, or `Window` indirectly via cached `Compositor` ... actually only `Application` and `AngleDevice`) is released. `RemoveCommitNeeded` is a `CompositorController` method — it doesn't depend on the dispatcher queue being alive. So even on the abnormal teardown path (panic / `application_drop` without `application_stop_event_loop`), the subscription is revoked when the driver finally drops; any agile fire that races the teardown finds `TryEnqueueWithPriority` returning `Ok(false)` (queue shutting down) and clears the latch (§7.5).

Update the existing [`Application::shutdown`](../../src/win32/application.rs#L65) (called from Kotlin's normal close path via [`application_stop_event_loop`](../../src/win32/application_api.rs#L52-L55)):

```rust
pub fn shutdown(&self) -> anyhow::Result<()> {
    self.compositor_driver.shutdown();
    let _ = self.dispatcher_queue_controller.ShutdownQueueAsync()?;
    Ok(())
}
```

`shutdown()` is idempotent (§5.2): if `CompositorDriver::Drop` runs later, it calls `shutdown()` again, sees `token == 0`, and no-ops. The normal close path revokes before the queue tears down; the abnormal path revokes whenever the Arc dies, with the latch handling the gap.

**Build plumbing.** Three changes:

- Add `static_assertions = "1"` to `[dependencies]` in [native/desktop-win32/Cargo.toml](../../Cargo.toml).
- Add `"Win32_System_Threading"` to the `windows` crate feature list in the same `Cargo.toml` (for `GetCurrentThreadId`).
- Add `pub mod compositor_driver;` to [native/desktop-win32/src/win32/mod.rs](../../src/win32/mod.rs).

### 6.2 `Window`

`Window` swaps its `compositor_controller` field for `compositor: Compositor` (not the driver — Window only needs the compositor for visual creation, never calls `Commit()` directly). All call-sites migrate:

- [window.rs:113,134,151](../../src/win32/window.rs#L113) — struct field + constructor signature + assignment. `Window::new` now takes `Compositor` and stores it. `Application::new_window` builds this via `self.compositor_driver.compositor()?` and passes it in.
- [window.rs:210](../../src/win32/window.rs#L210) `add_visual` — replace `self.compositor_controller.Compositor()?` with `self.compositor.CreateSpriteVisual()?` (no driver hop).
- [window.rs:407](../../src/win32/window.rs#L407) `set_backdrop_tint` — `self.compositor.CreateColorBrushWithColor(...)`. Drop the trailing `.Commit()?` at line 411 (fast-path publishes inline).
- [window.rs:418](../../src/win32/window.rs#L418) `remove_backdrop_tint` — drop the `.Commit()?`.
- [window.rs:423–428](../../src/win32/window.rs#L423) `commit_composition` — **delete the helper**. The `CommitNeeded` fast-path publishes post-handler mutations inline on the same UI thread.
- [window.rs:547–552](../../src/win32/window.rs#L547) `rebuild_caption_strip` — pass `&self.compositor` (`CaptionButtonStrip::new` takes `&Compositor` — see §6.4).
- [window.rs:734–739](../../src/win32/window.rs#L734) initial `CaptionButtonStrip::new` call — same `&window.compositor` swap.
- [window.rs:750](../../src/win32/window.rs#L750) `initialize_content` — drop the previous `let compositor = window.compositor_controller.Compositor()?;` line; use `window.compositor` directly at each `CreateXxx` / `cast()?` call.
- [window.rs:440–446](../../src/win32/window.rs#L440-L446) — doc comment on `set_content_top_offset` is stale (mentions strip piggy-back and `commit_composition`). Rewrite: "Does not commit. The mutation fires `CommitNeeded`; the driver's UI-thread fast-path publishes inline before the wndproc handler returns."
- [event_loop.rs:241](../../src/win32/event_loop.rs#L241) and [event_loop.rs:490](../../src/win32/event_loop.rs#L490) — `commit_composition` callers — **delete**. The fast-path covers them.
- [event_loop.rs:472–477](../../src/win32/event_loop.rs#L472-L477) — the comment block describing the synchronous-publish-before-handler-return mechanism (via `strip.on_resize` / `commit_composition`) is stale. Update: "`set_content_top_offset` and `strip.on_resize` mutate the visual tree; the driver's `CommitNeeded` fast-path publishes inline on the UI thread before this handler returns."

### 6.3 `AngleDevice`

[renderer_angle.rs:40–150](../../src/win32/renderer_angle.rs) — swap `compositor_controller: CompositorController` for `compositor_driver: Arc<CompositorDriver>` (AngleDevice retains the full driver because it needs the pause/resume gate). The two public methods get error-path wrappers so the gate cannot leak in a paused state and any skipped publish is replayed:

```rust
pub fn resize_surface(&self, width: egl::Int, height: egl::Int) -> anyhow::Result<EglSurfaceData> {
    self.compositor_driver.pause_autocommit();
    self.do_resize(width, height).inspect_err(|err| {
        let _ = self
            .compositor_driver
            .publish_and_resume_autocommit()
            .inspect_err(|publish_err| {
                log::warn!("resize_surface error-path publish failed (original error: {err}): {publish_err}");
            });
    })
}

fn do_resize(&self, width: egl::Int, height: egl::Int) -> anyhow::Result<EglSurfaceData> {
    // EGL calls run first (fallible). visual.SetSize is the LAST step so an
    // EGL error returns without queueing an orphan visual-size mutation —
    // otherwise the next drain would publish a partial state (new visual
    // size, old/missing ANGLE backbuffer).
    self.egl_instance.surface_attrib(self.display, self.surface, egl::WIDTH, width)?;
    self.egl_instance.surface_attrib(self.display, self.surface, egl::HEIGHT, height)?;
    self.egl_instance.swap_interval(self.display, 0)?;
    post_sub_buffer(&self.egl_instance, self.display, self.surface, 0, 0, width, height)?;
    let mut framebuffer_binding = 0;
    unsafe { (self.functions.fGetIntegerv)(GR_GL_FRAMEBUFFER_BINDING, &raw mut framebuffer_binding) };
    unsafe { (self.functions.fViewport)(0, 0, width, height) };
    self.visual.SetSize(Vector2 { X: width as f32, Y: height as f32 })?;
    Ok(EglSurfaceData { framebuffer_binding })
}

pub fn swap_buffers(&self) -> anyhow::Result<()> {
    self.do_swap().inspect_err(|err| {
        let _ = self
            .compositor_driver
            .publish_and_resume_autocommit()
            .inspect_err(|publish_err| {
                log::warn!("swap_buffers error-path publish failed (original error: {err}): {publish_err}");
            });
    })
}

fn do_swap(&self) -> anyhow::Result<()> {
    unsafe { (self.functions.fFinish)() };
    self.egl_instance.swap_interval(self.display, 1)?;
    self.egl_instance.swap_buffers(self.display, self.surface)?;
    self.compositor_driver.publish_and_resume_autocommit()?;
    Ok(())
}
```

`pause_autocommit` is a single `Release` store (idempotent — see §7.7). `publish_and_resume_autocommit` does an `AcqRel` swap-and-conditional-Commit: if the gate was paused (prior value `false`), it Commits to publish whatever the resize batched; if the gate was already open (non-resize swap), it skips Commit and the queue continues to drain via `CommitNeeded` as usual. The gate is restored to `true` in both branches because the swap already wrote `true` regardless of the prior value. The `inspect_err` wrappers in `swap_buffers` / `resize_surface` invoke `publish_and_resume_autocommit` on the error path too — so any EGL or `Commit()` failure converges on the same idempotent flush. The Rust-side gate is never left closed across a Rust-side failure boundary.

**Kotlin-side gap.** `resize_surface` and `swap_buffers` are *separate* FFI calls ([renderer_api.rs](../../src/win32/renderer_api.rs)). If Kotlin code between them panics, throws, or skips `swap_buffers` (e.g. abort, JNI exception, logic error in `performDrawing`), the gate stays paused — all subsequent drain Commits skip. The pause heals only on the next successful `swap_buffers` (next paint). For typical bugs this means one frame of caption-animation freeze; for permanent breakage (window stuck mid-resize), it freezes until the next paint or until the window is destroyed. See §7.9.

**ANGLE present-then-resize invariant.** [`eglPostSubBufferNV`](https://chromium.googlesource.com/angle/angle/+/HEAD/extensions/EGL_ANGLE_post_sub_buffer.txt) issues a `Present1` at the OLD swapchain size BEFORE `IDXGISwapChain::ResizeBuffers` runs. Verified in [`SurfaceD3D::swapRect`](https://github.com/google/angle/blob/main/src/libANGLE/renderer/d3d/SurfaceD3D.cpp#L271-L310): the underlying swapchain's `swapRect` (present) is called at line 294 before `checkForOutOfDateSwapChain` at line 307, which in turn dispatches to [`SwapChain11::resize`](https://github.com/google/angle/blob/main/src/libANGLE/renderer/d3d/d3d11/SwapChain11.cpp#L475) and `IDXGISwapChain::ResizeBuffers`. `pause_autocommit` keeps the WUC visual at OLD size for the duration of `do_resize`, so the OLD-size Present1 composes 1:1 into the OLD-size visual rect — no exposed gutter. The matching `swap_buffers` publishes the NEW size on the same DWM tick as the Skia draw that fills it. This is exactly the invariant the controller's atomic batching provided before this spec; the new code preserves it via the gate.

### 6.4 `CaptionButtonStrip`

[caption_buttons.rs:445–560](../../src/win32/caption_buttons.rs#L445) — remove the `compositor_controller: CompositorController` field. `CaptionButtonStrip::new` takes `compositor: &Compositor` — no field stored on the struct. All 13 `.Commit()` sites are removed: drop the `Commit()` line; the mutation stays. Each removal publishes via the driver's `CommitNeeded` fast-path inline on the UI thread (the wndproc handler that triggered the mutation):

- **Interactive** (`on_pointer_update` line 706, `on_pointer_down` 734, `on_pointer_up` 759, `on_pointer_cancel` 777, `cancel_any_press` 794, `on_nc_mouse_leave` 803, `on_activate` 893, `on_appearance_change` 930).
- **Init** (`new` 557).
- **Event-handler boundary** (`on_dpi_change` 916, `on_rendering_device_replaced` 940, `on_max_state_change` 956, `on_resize` 962).

With the `Commit()?` lines gone, most of these methods are now infallible. Their return types drop from `anyhow::Result<()>` to `()` (and `on_pointer_up`'s `anyhow::Result<Option<CaptionButtonAction>>` to `Option<CaptionButtonAction>`). Callers in [event_loop.rs](../../src/win32/event_loop.rs) drop the surrounding `inspect_err(...) | .ok().flatten()` boilerplate. The one method that retains `anyhow::Result<()>` is `on_dpi_change`, because its body still calls `glyph_surface.Resize(...)?` (which is fallible independently of `Commit`).

`CaptionButtonStrip::new` ([caption_buttons.rs:486](../../src/win32/caption_buttons.rs#L486)) takes `compositor: &Compositor` directly. Auto-deref lets the body call `.CreateXxx` / `.clone()` on the reference; no local `let compositor = …clone()` shadow is needed. No field is stored. The init-time `InsertAtTop` + Commit ([caption_buttons.rs:556–557](../../src/win32/caption_buttons.rs#L556)) drops the Commit. The strip becomes visible on the next drain — at most one message-pump iteration later (sub-DWM-frame), in practice before the first `WM_PAINT`. Hit-testing reads `StripGeometry` derived from synchronous struct state, not the WUC tree, so hit-testing is unaffected by the deferred commit.

`relayout` at [caption_buttons.rs:566–611](../../src/win32/caption_buttons.rs#L566), called on construction + DPI change, is unchanged — it already does not call `Commit`. Stale doc comments inside the strip module that reference the removed Commits (e.g. [caption_buttons.rs:954–956](../../src/win32/caption_buttons.rs#L954-L956) which describes `on_max_state_change` "commits the glyph swap") are updated to point at the handler-level publish.

## 7. Risks and known limitations

### 7.1 Multi-window cross-resume race (HIGH, accepted)

One `CompositorController` is shared across every `Window`. Window A's `pause_autocommit` is a global flag; Window B's `swap_buffers.publish_and_resume_autocommit` re-enables the gate. If A's `visual.SetSize` is in the controller's dirty state and B's `Commit()` lands before A's matching `swap_buffers`, A's resize publishes prematurely → one-frame Mica flash on Window A.

Accepted as a known limitation. The shared-controller architecture is the existing model and not being changed by this spec; the race window is single-frame and only triggers when two windows resize concurrently. Promoting `autocommit_enabled` to a pause-counter plus a per-`AngleDevice` "I own a pause" ownership flag would fix it — see §8 alt A — but is deferred.

### 7.2 Tactile press latency (LOW with fast-path; ASSUMPTION)

Today, caption-button press-down mutates the visual and calls `Commit()` inline → publish within the same wndproc invocation. Under this design with the UI-thread fast-path (§5.1), press-down mutates → `CommitNeeded` fires → `on_commit_needed` runs (`GetCurrentThreadId() == ui_thread_id`) → `drain()` → `Commit()` — all inline on the same wndproc invocation. Latency parity with the current code.

ASSUMPTION: `CommitNeeded` is fired synchronously on the UI thread when a UI-thread mutation dirties the queue. Not documented by MS but widely observed across WinRT event sources (sender-thread delivery for synchronous state-change events). If WUC instead defers the fire to a worker thread, the fast-path check sees a different thread id and the dispatcher round-trip costs ≤ 1 message-pump iteration — well below the 6.9 ms 144Hz frame budget. Validated in §9.

### 7.3 Commit cadence under resize (MEDIUM, empirical)

While the gate is paused, `CommitNeeded` continues firing for unrelated mutations (caption hover animation started mid-drag). The drain runs and skips the Commit. The pending animation publishes when the next `swap_buffers` calls `publish_and_resume_autocommit`. Worst case: a one-DWM-tick start lag on an animation that began during a resize tick. Acceptance gated on §9 validation; no documented WUC contract that would force this to fail.

### 7.4 Commit failure inside drain (LOW)

`drain` logs Commit errors and returns. The actual D2D device-loss recovery flow ([composition.rs:123–194](../../src/win32/composition.rs#L123)) is independent of the WUC commit path: D2D loss is detected and rebuilt in the composition module's own callbacks; it doesn't depend on the driver's drain. A purely WUC-side Commit failure with no following paint would log-spin until the next mutation transition (which fires `CommitNeeded` afresh) or until shutdown. Acceptable: WUC controller Commit failures in practice indicate an unrecoverable compositor device.

### 7.5 Subscription teardown vs in-flight drain (LOW)

`Application`'s `Drop` calls `compositor_driver.shutdown()` → `RemoveCommitNeeded(token)` before any field drops. A drain enqueued just before `RemoveCommitNeeded` is still pending in the queue and runs after — upgrades the `Weak<CompositorDriver>` (driver still alive: downstream `Arc` clones in `Window` / `AngleDevice` / `CaptionButtonStrip` keep the strong count > 0 until those drop in turn, so the `Weak` upgrades cleanly), reads `autocommit_enabled = true`, calls `controller.Commit()` once more. Benign no-op. Once the queue enters shutdown state (after `ShutdownQueueAsync` is fired — its returned `IAsyncAction` is discarded by [`Application::shutdown`](../../src/win32/application.rs#L65), with completion arriving via the `ShutdownCompleted` event handler that posts `WM_QUIT`), subsequent `TryEnqueueWithPriority` returns `Ok(false)` and the latch clears.

### 7.6 Subscription-time fire (LOW, defensive — ASSUMPTION)

ASSUMPTION: `CommitNeeded` is not raised at subscription time. MS docs describe the event as occurring "when the framework needs to call Commit" and do not address subscription semantics explicitly. I have not confirmed this against an authoritative source — it is a reasonable inference from typical WinRT event-source behaviour but is not documented.

Worst case if the assumption is wrong: the handler upgrades `Weak<CompositorDriver>` (alive), reads `autocommit_enabled = true` (default), enqueues a drain. The drain runs after construction completes and `commit_needed_token.store(token)` returns, then calls `Commit()` on a clean queue (no-op). The `enqueue_pending` latch correctly coalesces any further fires before `store(token)` completes. No missed signal, no panic. The order "subscribe, then `store(token)`" is therefore safe under both interpretations.

### 7.7 Single-window pause reentrancy (LOW)

`pause_autocommit` is a single bool — two paused callers without an intervening resume coalesce, and the first resume re-opens the gate while the second caller still expects atomicity. Single-window scenario where this matters: nested wndproc dispatch (e.g. `WM_DPICHANGED` arriving mid-resize and recursively calling into `resize_surface`). Unlikely in current code — `resize_surface` is called from `performDrawing` on the Kotlin side, and Win32 wndproc is single-threaded — but not statically prevented. Pause is intentionally idempotent (no `debug_assert!` against re-pause); the §7.9 self-heal path relies on this idempotency, and the same single-thread serialisation that makes the reentrancy unlikely also bounds the blast radius to one publish race that the next `swap_buffers` heals.

### 7.8 Strip init publish lag (LOW)

`CaptionButtonStrip::new`'s `InsertAtTop` no longer Commits inline. The strip becomes visible on the next drain — typically before the first WM_PAINT, but a `WM_NCPAINT` arriving in between would render the non-client area without the strip for one frame. Hit-testing is unaffected (it reads struct geometry, not the WUC tree). Sub-DWM-frame visual artifact; same timing tier the prior auto-commit `Compositor` attempt would have exhibited at strip construction.

### 7.9 Pause-leak across the Rust/Kotlin FFI boundary (HIGH)

`AngleDevice::resize_surface` paused the gate and returned `Ok` over FFI. The matching `swap_buffers` call lives across a separate FFI boundary, driven by Kotlin's `performDrawing` chain. If Kotlin panics, throws an `Error`, hits a JNI exception, or simply skips `swap_buffers` (logic bug), the gate stays paused. The drain runs on every `CommitNeeded` fire but skips the Commit. Effect: caption-button animations and backdrop mutations are stranded until the next successful `swap_buffers` resumes the gate.

Healing paths:
- **Common case** (recoverable Kotlin error): the next `WM_PAINT` triggers `performDrawing` → `resize_surface` (re-pauses idempotently — see §7.7) → draw → `swap_buffers` → `publish_and_resume_autocommit`. Self-heals in one frame.
- **Stuck case** (no further paints): if the window never paints again (orphaned, hidden, JVM-aborted mid-frame), the gate stays paused for the lifetime of the `Application`. The multi-window race (§7.1) generalises: all other windows' drains skip while one window's pause is stuck.

Mitigations considered and deferred (kept simple):
- **RAII guard scoped across resize/swap** — would require storing the guard in a Cell on `AngleDevice` and threading it through the FFI; awkward.
- **Per-`AngleDevice` ownership flag + counter** — fixes both this and §7.1. See §8 alt A.
- **`AngleDevice::Drop` → `publish_and_resume_autocommit`** — heals the orphan-window case. Cheap, low risk; consider for v1.5 if telemetry shows the stuck-pause scenario in practice.

## 8. Alternatives considered

- **A. Pause counter (`AtomicUsize` + per-`AngleDevice` ownership flag).** Eliminates the §7.1 multi-window race. Adds ~30 LoC, an extra atomic per pause/resume, and a three-way branch in `swap_buffers` (own-pause / other-pause / no-pause). Future work when a real multi-window scenario emerges.
- **B. Drop `CompositorController` entirely.** Implemented and failed: auto-commit `Compositor` plus visual-tree reorder introduced whole-window Mica flicker during resize. The DXGI flip-model "resize glitch" (uninitialised back-buffers after `IDXGISwapChain::ResizeBuffers`, amplified by `EGL_EXPERIMENTAL_PRESENT_PATH_FAST_ANGLE`) becomes visible without the controller's atomic batching to align `visual.SetSize` with the matching Present. This spec is the lighter alternative that keeps the controller.
- **C. Keep all 17 explicit Commits and add `CommitNeeded` as a safety net only.** Original "minimal" option rejected — does not introduce auto-commit, just plugs the idle-animation hole.
- **D. `DispatcherQueue::GetForCurrentThread()` instead of plumbing.** `GetForCurrentThread()` returns null on threads without a queue. Plumbing the `Application`-owned queue is explicit and validates ownership at the call site. The natural alternative — `controller.Compositor()?.DispatcherQueue()?` — is unavailable because `Compositor::DispatcherQueue` was introduced in Windows 10 build 20348 and this toolkit's minimum is 17763.
- **E. Per-frame transparent gutter clear** (Electron tech-talk pattern). If residual resize-edge artifacts surface, clear pixels outside the new viewport to transparent so Commit/Present ordering stops mattering. Deferred follow-up.
- **F. `ICompositorInterop::CreateCompositionSurfaceForSwapChain`** — bind an app-owned `IDXGISwapChain` directly to a WUC `CompositionSurfaceBrush` via the in-process pointer overload ([docs](https://learn.microsoft.com/windows/win32/api/windows.ui.composition.interop/nf-windows-ui-composition-interop-icompositorinterop-createcompositionsurfaceforswapchain); pair the swap chain from [`IDXGIFactory2::CreateSwapChainForComposition`](https://learn.microsoft.com/windows/win32/api/dxgi1_2/nf-dxgi1_2-idxgifactory2-createswapchainforcomposition)). Eliminates the manual `SetContent` cycle. Does **not** eliminate `visual.SetSize` on resize — that still maps the swap-chain pixels into the visual rect, controlled by `CompositionSurfaceBrush.Stretch`. Deferred because it requires opening up ANGLE's underlying `IDXGISwapChain` handle, which is currently hidden behind the EGL abstraction; we would need to upstream an ANGLE accessor or replace ANGLE with an app-owned D3D11 path. (Note: Windows Terminal Atlas uses `ISwapChainPanelNative2::SetSwapChainHandle`, but that is the XAML interop bridge — not applicable to a raw `Compositor` / `DesktopWindowTarget` host like this codebase.)

## 9. Validation

- Smoke test: hover a caption button on a window with no active ANGLE rendering (the [TODO.md:85–89](../TODO.md) idle-animation scenario). The 150 ms `ColorKeyFrameAnimation` backplate fade must run at smooth 60 Hz cadence. Pass criterion: visible, no frame-step stutter.
- **Fast-path firing semantics** (validates §7.2 ASSUMPTION): instrument `on_commit_needed` to record whether the agile fire landed on the UI thread vs off-thread. Run interactive drags (caption hover, press, release), resize sequences, and DPI changes; aggregate counts. Expectation: ≥ 95% UI-thread fires for UI-thread-triggered mutations. If off-thread fires dominate, the latency analysis in §7.2 falls back to the dispatcher-round-trip path — still below the 6.9 ms budget, but the fast-path optimisation isn't earning its keep and could be removed.
- Resize regression: slow edge drag, rapid drag, maximize/restore, DPI change mid-drag, Aero Snap, Aero Shake. Pass criterion: no observable backdrop flicker, no anchor drift, no Mica gutter beyond a single frame at the trailing edge.
- Caption-button latency: hover-fade start, press-down feedback, release. Pass criterion: indistinguishable from current branch in subjective response.
- **Induced resize-error gate-leak smoke**: force `egl::Instance::swap_interval` or `eglSwapBuffers` to return Err on one frame (e.g. via a build-flagged fault-injection point); confirm the gate is restored and the very next paint Commits normally. Pass criterion: caption animation does not freeze after the induced failure clears.
- **Shutdown smoke**: open and close a sample window 100 times in a tight loop; confirm no `CommitNeeded` callback fires after `Application` drops and no dangling `RemoveCommitNeeded` panic appears in logs.
- `gradlew lint` clean — `.git-hooks/pre-push` runs `./gradlew lint`.

## 10. References

Microsoft documentation:
- [`CompositorController.CommitNeeded`](https://learn.microsoft.com/uwp/api/windows.ui.composition.core.compositorcontroller.commitneeded)
- [`CompositorController.Commit`](https://learn.microsoft.com/uwp/api/windows.ui.composition.core.compositorcontroller.commit)
- [`DispatcherQueue.TryEnqueueWithPriority`](https://learn.microsoft.com/uwp/api/windows.system.dispatcherqueue.tryenqueuewithpriority)
- [`CreateDispatcherQueueController`](https://learn.microsoft.com/windows/win32/api/dispatcherqueue/nf-dispatcherqueue-createdispatcherqueuecontroller)

Prior art in this repo:
- [`RenderingDeviceReplacedRegistration`](../../src/win32/composition.rs#L229) — RAII registration wrapper this spec follows for `CommitNeeded` token revocation (the spec extends the pattern with an idempotent take so explicit shutdown can revoke before `Drop`).
- [TODO.md:85–89](../TODO.md) — the defect this closes.
- [TODO.md:168](../TODO.md) — `glFinish` perf concern; do not address independently of this spec (§3 non-goal).
