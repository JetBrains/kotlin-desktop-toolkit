# Noria → KDT-Compose convergence plan

Consolidated from five parallel backend comparisons. Detailed evidence, code excerpts and
commit archaeology live in the sibling reports:

| Report | Scope | Findings |
|---|---|---|
| [macos.md](macos.md) | `macos/` + text input client | 15 (1 P0, 3 P1, 6 P2, 5 P3) |
| [windows.md](windows.md) | `windows/` | 13 (0 P0, 4 P1, 5 P2, 4 P3) |
| [gtk.md](gtk.md) | `gtk/` + shared Linux bits | 13 (0 P0, 3 P1, 5 P2, 5 P3) |
| [linux.md](linux.md) | `linux/`, `LinuxPlatform`, `MimeTransfer` | 11 (2 P0, 2 P1, 6 P2, 1 P3) |
| [shared.md](shared.md) | root layer, architecture, capability gaps | 12 (0 P0, 5 P1, 6 P2, 1 P3) |

**Total: 64 findings — 3 P0, 17 P1, 28 P2, 16 P3.**

## Preconditions established

- **No KDT version skew.** Noria pins `kotlin-desktop-toolkit-common/0.0.89`; this repo is
  `0.0.89-11-gd496099e` and *is* KDT. Every Noria fix is expressible here as-is.
- Noria head at time of comparison: `a09eb0e22fe01`, 2026-08-28.

## The headline finding

**This is not a one-directional port.** On Windows and GTK the target is already *ahead* of Noria
on most of the tickets that looked relevant, and three seeded tickets would have caused
regressions if ported blindly (see the Do-Not-Port register at the bottom). The genuine gap is
narrow and clusters in **four themes**: application termination, focus/activation, IME teardown,
and object lifetime. Everything else is refactor-grade.

---

# Workstreams, in recommended order

## WS1 — Application termination (P0, spans all four backends)

The single most coherent piece of work. Same defect shape in four places.

1. **`isQuitInProgress` is never reset — verified in all four backends.**
   Set to `true` at `macos/MacOsApplication.kt:205`, `linux/LinuxApplication.kt:365`,
   `gtk/GtkApplication.kt:380`, `windows/WindowsApplication.kt:226`; there is no assignment back
   to `false` anywhere in the module. Any declined window close permanently wedges quit.
   *This was reported as a Windows-local issue; it is universal.*
2. **Linux: hang on exit after any `quit()`** (P0). *Mechanism re-verified against KDT's own
   sources — see the investigation log below; the polarity is fine, the stuck flag is the cause.*
   `linux/LinuxApplication.kt:182-185` returns `Stop` to veto, KDT's `exit` flag stays set
   (`application.rs:237`) and re-sends `ApplicationWantsToTerminate` every 16 ms. Because
   `onQuitRequested()` short-circuits on `isQuitInProgress` from (1) it answers `Stop` forever, the
   loop never returns `Ok(false)`, and `close()`'s `eventLoopThread.join()` blocks permanently.
3. **GTK: answering the terminate callback with `true` hard-hangs.** It calls `gtk_app.quit()`, and
   since `runOnEventLoopAsync` is `glib::idle_add_once`, that drops the whole Compose dispatcher
   queue including `runApplication`'s `runBlocking`.
4. **Quit with zero open windows is a no-op** on Windows (`WindowsApplication.kt:223-225`) and GTK.
   Linux returns `true` correctly. Windows and GTK cannot quit in that state at all.

**Approach:** lift a single structured-quit state machine into the shared layer — consult handlers,
mark in-progress, ask windows to close, **reset the flag on any veto or completion**, and quit
directly when the window list is empty. Then have each backend's terminate callback delegate to it
and always answer the native side with the non-vetoing result. Noria's
`handleApplicationWantsToTerminate` / `requestStructuredQuit` pair is the reference shape.

**Risk:** medium — touches exit paths on every platform. Needs manual verification per OS:
quit with no windows, quit with a vetoing handler, then quit again.

## WS2 — Linux focus and activation (1 P0 + 1 P1)

5. **Activation token requested from the wrong surface** (P0). The target asks the *target* window;
   KDT names the parameter `source_window_id` and builds the request from that surface plus the
   latest seat serial, so compositors refuse it unless it is already focused. **`requestFocus()` on
   any background window is a silent no-op today.** Noria: `7ec5af76e8605`.
6. **`focusedWindow` scans `isFocused`, which has three writers** (configure + keyboard enter/leave),
   so two windows can both claim focus. Compounds (5). Noria indexes `windows[keyboardFocusWindowId]`
   — same commit.

These two share a commit and should land together.

## WS3 — macOS IME teardown (P0)

7. The target calls only native `discardMarkedText()`; it never tells Compose to
   `finishComposingText()` nor toggles `textInputContext.activate()/deactivate()`. The composing
   underline survives focus loss and a CJK IME stays armed with nothing editable.
   Noria: `4b636801ab9ad`, AIR-6619.

Note this is **macOS-only**: the agents confirmed the Linux IME activate/deactivate and
preedit-discard behaviour already matches Noria line for line, and AIR-6619 never touched GTK.

## WS4 — Object lifetime and teardown (2 P1, shared layer)

8. **Native window leaks on abandoned composition.** `Window.kt:286-291` uses
   `remember { application.createWindow(...) }` + `DisposableEffect`; if the composition is
   abandoned, the remembered window is discarded without `onDispose` ever running. Noria uses
   `RememberObserver.onAbandoned`/`onForgotten` (`43a235ad06e6a`, "[fleet] Fix window disposal").
9. **`KdtSnapshotManager` never stops.** `KdtSnapshotManager.kt:26-49` drops the `ObserverHandle`
   from `registerGlobalWriteObserver`, never cancels its scope, never resets `started` — the
   never-finishing loop is deliberate per the class comment. After `close()`, any `mutableStateOf`
   write anywhere in the process resumes the consumer through `KdtDispatcher` →
   `currentApplication()` → `IllegalStateException` on the *writer's* thread. Noria binds this to
   the scene job (AIR-6084, `7e8e349d602e5`).

(9) is the blocker for any second `runApplication` in one process, which is why it also blocks WS10.

## WS5 — Input mode on scroll (2 × P1, trivial)

10. macOS and Windows scroll branches skip `startPointerEvent()`, so focus rings stay drawn and the
    cursor stays hidden after typing until the mouse physically moves. One line per branch.
    **GTK and Linux are already correct** — this is not a four-way fix.

## WS6 — Windows clipboard serialization (P1, trivial)

11. `windows/WindowsClipboard.kt:62-65` suspends between `set` and `flush` with no mutex, so two
    coroutines can interleave and publish the wrong data object. Noria's `clipboardMutex`
    (AIR-5558, `79a633bee12d2`) copies in directly. The retry ladder, the `Cleaner` and AIR-6051
    are already ported here.

## WS7 — GTK input routing and focus (2 × P1)

12. **`ModifiersChanged` must be broadcast to every window and not cleared locally.** It is a
    *seat-wide* event — verified in KDT's Rust, `application_state.rs:161-172` installs one handler
    on `default_seat.keyboard()`. Routing it to only the focused window loses any modifier held
    across a focus change (Alt-Tab with Alt down → window B never learns Alt is pressed).
    Noria: `cc557a8bf5daa`.
13. **Split `isFocused` into `hasActiveAppearance || hasKeyboardFocus`.** One field is written from
    two unrelated GTK signals (`is_active_notify` vs `ToplevelState::FOCUSED`), and
    `update_window_configure` re-sends the cached `active` on every resize — so a resize can clear
    focus on a window that holds the keyboard. Noria: `369319fadacfe`.

## WS8 — Windows caption buttons (P1)

14. Every claimed title-bar rect maps to `HTCLIENT`, so a Compose-drawn maximize button never
    triggers the Win11 Snap Layouts flyout. `NCHitTestResult.MinButton/MaxButton/Close` already
    exist in KDT. AIR-5600, `a644825176abe`.

## WS9 — Linux CSD frame (P1, staged — do not treat as a drop-in)

15. The target requests `padding=0, resizerThickness=0`, so there is no shadow and `tiled` is
    dropped. Confirmed in `window.rs:229-282` that `resizerThickness` is **inert without padding** —
    which is precisely why `LinuxWindowBorders.kt` has to steal 6 dp of content, making edge-flush
    controls unclickable. AIR-4578, `fe5fa7e1834a2`.

**Requires a surface-vs-content coordinate split** before it can land. Staged plan in
[linux.md](linux.md). Highest risk item in this document; sequence it last among the P1s.

## WS10 — Enablers (P1/P2, unblocks everything above)

16. **No tests, no headless path.** The compose module has zero tests. Every P1 above is exactly
    what a headless smoke test would catch. Noria has `headless/HeadlessEventLoopJvm.kt`.
17. **`KdtLoggerAppender`** — bridge KDT's `AppenderInterface` into the host logger; today all four
    apps call `KotlinDesktopToolkit.init()` with defaults and there is no way to supply an appender.
    Also de-duplicates an `initToolkit()` copy-pasted between Linux and GTK.
18. **`InfiniteAnimationPolicy`/`MotionDurationScale` on the app composition.** `YieldFrameClock`
    (`Application.kt:274-279`) will spin the event loop at 100% CPU for an infinite animation
    outside a `Window`. Compose supplies both context elements; Noria installs
    `noWindowAnimationCoroutineContext` into every scene.
19. **`LinuxInputMethodCommon.kt`** — the target duplicates ~110 lines of IME arithmetic between
    `gtk/` and `linux/`, and `GtkKeys.kt` / `LinuxKeys.kt` are byte-identical modulo the package
    token.
20. Build hygiene: the module lacks `maven-publish` and `explicitApi()`, both of which the base
    `kotlin-desktop-toolkit` module has.

---

# Architectural verdict

**The target's shared layer is the better-factored of the two.** Adopt Noria's seams selectively,
not wholesale:

- Target is better: one `KdtDispatcher` where Noria has four duplicated pairs plus a factory;
  `KdtDragAndDropManager` behind a one-method `DragAndDropSourceWindow` where Noria type-switches on
  four window classes; a shared `DragAndDropClipboardEntry`; an opt-in `Dispatchers.Main` artifact;
  a `DesktopBackend` enum making the Wayland/GTK choice explicit.
- Worth adopting from Noria: only `LinuxInputMethodCommon.kt`, `KdtLoggerAppender`, and the headless
  backend.
- **Do not** grow a shared text-input session abstraction. Compose already owns the session via
  `PlatformContext.startInputMethod`; Noria needs `TextInputSessionOwner` only because its scene
  does not, and its `ComposeTextInputSession.jvm.kt` is 74 lines of commented-out dead code.

---

# Do-Not-Port register

Things that look like gaps but are not. Recorded so a later convergence sweep does not regress the
target toward Noria.

| Item | Why not |
|---|---|
| macOS scroll delta scaling (×1/×10) | Verified against released Compose 1.10: `MacOSCocoaConfig` already multiplies by `10.dp.toPx()`. The target's ×0.1/×1 is correct; Noria's would scroll 10× too fast. |
| AIR-5008 `clearPointerButtons()` at drag start (GTK) | **Reverted** by AIR-5571 (`d80de585f4880`) in favour of clearing on `MouseExited` — which is what the target already does. Porting reintroduces a KDE resize bug. |
| AIR-6701 shutdown hook (Linux) | The actual fix was *deleting* the hook. The target correctly has none. |
| AIR-6291 pointer icon (Windows) | The commit is a `HitPathTracker`+macOS fix that explicitly leaves Windows alone. |
| AIR-5280 (Windows) | Reuse-only; the underlying cursor leak is already fixed here via `pointerIcons.onWindowClosing()`. |
| AIR-6619 / AIR-5558 / AIR-6275 / AIR-5776 on the wrong platforms | Confirmed via `git show --stat` to be macOS/Windows-only commits; they never touched the backends they appeared relevant to. |
| Windows: 14 items where Noria is the *worse* side | ANGLE renderer leak on close, `currentSize` committed before a failing resize, `size` set to client rect in `NCCalcSize`, no `WindowResize` handling, `windowRectBeforeMinimization!!` NPE, screens never refreshed, no app-level event `try/catch`, `VirtualKey.Clear -> Key.NumLock`, … — full list in [windows.md](windows.md). |
| macOS: target ahead of Noria | Frame pump, GPU render-thread locking (Noria has a spurious-wakeup NPE, a disabled `assert`, `onComplete` under the lock), resize/density handling, `lastMouseOffset` tracking. |
| GTK: target ahead of Noria | File dialogs (Noria's are `TODO`), drag-icon context caching, IME preedit discard, scroll `isStop`, DnD plumbing. |
| Window reuse / `WindowIdsMapping` / `HeavyweightWindowId` | Out of scope per project decision. |
| `withPreparedThread` main-thread pinning + snapshot read-tracking | Out of scope; the target solves the same problem via `KdtDispatcher` + `KdtSnapshotManager`. |
| Extended `Key` enum / const range | Compose's `Key` is fixed. Genuine keycode *mapping* bugs are reported separately. |

---

# One finding that is not a Noria port

21. Compose's desktop `ScrollConfig` reads `awtEventOrNull`, which is always `null` for KDT events,
    so precise-wheel detection never fires and trackpad scrolling gets Compose's stepping-wheel
    animation stacked on top of macOS inertia. Fix mirrors the target's own `MacOsTypedEvents`
    trick. Fell out of the comparison; see [macos.md](macos.md) finding 15.

---

# Investigation log — WS1 (Linux terminate)

Traced directly in this repo's sources while starting WS1. Records facts that are not obvious from
either Kotlin side alone, and corrects the "inverted handshake" framing in [linux.md](linux.md).

**The FFI polarity, pinned down:**

- `linux/Application.kt:146-148` — Kotlin `Continue` → native `false`, `Stop` → native `true`.
- `application.rs:237` — `if exit.load() && !send_event(ApplicationWantsToTerminate)`, so the loop
  terminates when the handler returns **`Continue`**, and `Stop` **vetoes**.
- The target's `if (onQuitRequested()) Continue else Stop` is therefore **the right way round**.
  `linux.md` finding 1 calls this an inverted handshake; that part of the diagnosis is wrong. The
  hang it describes is real, but the cause is the stuck flag, not the polarity.

**Why it hangs anyway:**

`exit` has exactly one writer — `application_stop_event_loop` (`application_api.rs:119-125`), reached
only from `close()`. So on Linux `ApplicationWantsToTerminate` **never** originates from a user
gesture; it is purely the teardown signal, refiring every 16 ms until the handler concedes.

    quit()  → onQuitRequested() sets isQuitInProgress = true, asks windows to close, returns false
            → windows close, composition stops emitting, runApplication returns
    close() → stopEventLoop() sets exit
            → loop sends WantsToTerminate every 16 ms
            → onQuitRequested() short-circuits on isQuitInProgress, returns false, forever
            → event_loop_iteration never returns Ok(false) → join() hangs

A second hang path needs no `quit()` at all: if `close()` runs while any window is still open,
the first terminate event sets the flag and every later one is short-circuited by it.

**Design that follows from this:** on Linux the two paths should stop sharing a predicate.
`quit()` is the user-initiated request — consult handlers, ask windows to close, and reset the flag
whenever the attempt concludes without terminating, so a declined close does not wedge the next
attempt. The terminate event belongs to unconditional teardown and must never veto. Confirm the
equivalent flag lifetime on macOS, GTK and Windows before mirroring, since only Linux has been
traced end-to-end so far.
