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

## Porting principle

**Resemble Noria as closely as the difference between the two runtimes allows.** The same people
maintain both, so a fix that reads the same on both sides is worth more than a tidier one that does
not. In practice:

- Keep Noria's **names** — `requestStructuredQuit`, `structuredQuitInProgress`, `quitHandlers`,
  `finishStructuredQuitIfNeeded` — even where this module arrives at them differently.
- Keep Noria's **structure**, including duplication. Noria carries the quit state on each of its four
  `*Application` classes rather than in a shared type; so do we. Lifting something into the shared
  layer that Noria has not lifted needs a reason beyond "it is the same four times".
- **Do not add commentary Noria does not have.** Match the density of the file being edited.
  Reasoning that belongs to the convergence rather than to the code goes in this document.
- Diverge only where copying would carry a bug across, or where this module's architecture makes the
  Noria shape meaningless — and **say so here**, per workstream, with the reason.

This supersedes the selective-adoption advice in *Architectural verdict* below wherever the two
conflict: that section still records which side is better factored, but it is no longer the tie-breaker.

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

## WS1 — Application termination (P0, spans all four backends) — **done**

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

**Approach:** mirror Noria's `requestStructuredQuit` on each of the four `*Application` classes —
consult handlers, mark in-progress, ask windows to close, **reset the flag on any veto or
completion**, and quit directly when the window list is empty. Then have each backend's terminate
callback delegate to it and always answer the native side with the non-vetoing result.

*(An earlier revision of this item lifted the state machine into a shared `ApplicationQuit` class.
Reverted: Noria keeps the state per backend, and the Porting principle above makes that the tie-breaker.)*

**Risk:** medium — touches exit paths on every platform. Needs manual verification per OS:
quit with no windows, quit with a vetoing handler, then quit again.

**Landed** as a private `requestStructuredQuit()` plus `quitHandlers` and `structuredQuitInProgress`
on each of the four `*Application` classes, as in Noria. Two deliberate departures:

- **The in-progress flag became a scoped re-entrancy guard**, set and cleared inside one round of
  `requestStructuredQuit()` under a `try`/`finally`, rather than a latch cleared when termination proceeds.
  Noria's `structuredQuitInProgress` is *also* never cleared on a veto (`GtkApplication.kt:565`,
  `LinuxApplication.kt:757` — only `continueTerminationAsync` clears it), so porting its lifetime
  verbatim would have carried finding 1 across rather than fixing it. The guard now only does the
  job it is needed for: a window's own `onCloseRequest` runs inline in the loop below and may call
  `quit()` again.
- **No backend tells its native side "yes, terminate" any more.** AppKit's `TerminateNow` halts the
  process where it stands and GTK's `gtk_app.quit()` drops the GLib queue the Compose dispatcher
  runs on, so both are answered with the cancel result unconditionally and the exit is driven
  through the composition on all four platforms. Linux's terminate event, being teardown-only,
  answers `Continue` unconditionally for the same reason read from the other side.

Noria's `terminationInProgress`, `continueTerminationAsync()` and `finishStructuredQuitIfNeeded()`
have **no counterpart here** and were not ported: they exist because Noria terminates itself from an
off-loop thread once its window map empties, whereas here the composition ends `runApplication` and
`application()`'s `use` calls `close()` exactly once. Their one job that does survive — deciding to
terminate when there is no window to negotiate with — is finding 4, and it needed a new seam:
`application()` installs its `exitApplication` in a process-wide slot (`Application.kt`,
`requestApplicationExit`) that `requestStructuredQuit` reaches for in that case. A composition driven
by a bare `runApplication` has no such lever and gets a warning instead.

## WS2 — Linux focus and activation (1 P0 + 1 P1) — **done**

5. **Activation token requested from the wrong surface** (P0). The target asks the *target* window;
   KDT names the parameter `source_window_id` and builds the request from that surface plus the
   latest seat serial, so compositors refuse it unless it is already focused. **`requestFocus()` on
   any background window is a silent no-op today.** Noria: `7ec5af76e8605`.
6. **`focusedWindow` scans `isFocused`, which has three writers** (configure + keyboard enter/leave),
   so two windows can both claim focus. Compounds (5). Noria indexes `windows[keyboardFocusWindowId]`
   — same commit.

These two share a commit and should land together.

**Landed** as Noria has them, including the third change in `7ec5af76e8605`: `requestActivation()` is
now empty. Keyboard enter/leave no longer write `isFocused`, so `xdg_toplevel.configure`'s `activated`
is its only writer — needs a caret-blink check on GNOME, where a CSD window keeps `activated` across a
menu grab.

## WS3 — macOS IME teardown (P0) — **done**

7. The target calls only native `discardMarkedText()`; it never tells Compose to
   `finishComposingText()` nor toggles `textInputContext.activate()/deactivate()`. The composing
   underline survives focus loss and a CJK IME stays armed with nothing editable.
   Noria: `4b636801ab9ad`, AIR-6619.

Note this is **macOS-only**: the agents confirmed the Linux IME activate/deactivate and
preedit-discard behaviour already matches Noria line for line, and AIR-6619 never touched GTK.

**Landed** as Noria's four lines in Noria's order, with the target's existing
`invalidateCharacterCoordinates()` kept after them and the whole block still behind `isWindowGone`
(`Window.close` already discards and deactivates). Noria's comment weighing `insertText` against
`unmarkText` was **not** copied: it records that its action context is stale at teardown, which is a
Noria problem — for Compose `finishComposingText()`, which is what `unmarkText()` calls here, is the
right call.

## WS4 — Object lifetime and teardown (2 P1, shared layer) — **done**

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

**Landed.** (8) is Noria's `RememberObserver` block verbatim. (9) has no literal Noria shape to copy —
its `activateSnapshots` is built on `snapshotHolder`, `DrainableCoroutineDispatcher` and the data-source
context, none of which exist here — so only the lesson was taken: keep the `ObserverHandle`, and dispose
it last of all, on the event loop thread. The name is Noria's; the handle is disposed in
`runApplication`'s `finally`, which is this module's equivalent of the scene job's.

The report's suggested belt-and-braces — make `KdtDispatcherBase.dispatch` tolerate a missing
application — was **not** added. Noria's four dispatchers call `GrandCentralDispatch` and friends
directly and cannot have the problem, so there is nothing to mirror, and disposing the handle removes
the cause rather than swallowing the symptom.

A bug found while writing (9), not in either codebase: the two `launch`es in `runApplication` return
immediately, so a `try`/`finally` wrapped straight around them would have torn the flush down before
the first composition. They are inside a `coroutineScope` now.

## WS5 — Input mode on scroll (2 × P1, trivial) — **done**

10. macOS and Windows scroll branches skip `startPointerEvent()`, so focus rings stay drawn and the
    cursor stays hidden after typing until the mouse physically moves. One line per branch.
    **GTK and Linux are already correct** — this is not a four-way fix.

**Landed**: three branches (macOS `ScrollWheel`, Windows `ScrollWheelX`/`ScrollWheelY`) set
`lastEventTimeMillis` by hand where GTK and Linux call `startPointerEvent`, which does that *and*
requests `InputMode.Touch`. Not a Noria port — an internal inconsistency the comparison surfaced.

## WS6 — Windows clipboard serialization (P1, trivial) — **done**

11. `windows/WindowsClipboard.kt:62-65` suspends between `set` and `flush` with no mutex, so two
    coroutines can interleave and publish the wrong data object. Noria's `clipboardMutex`
    (AIR-5558, `79a633bee12d2`) copies in directly. The retry ladder, the `Cleaner` and AIR-6051
    are already ported here.

**Landed** as Noria has it: a file-level `private val clipboardMutex = Mutex()` held across `get`, and
across `set`+`flush` together. Noria's `getClipEntrySync` `tryLock` check has no counterpart — this
module's `Clipboard` has no synchronous read to guard.

## WS7 — GTK input routing and focus (2 × P1) — **done**

12. **`ModifiersChanged` must be broadcast to every window and not cleared locally.** It is a
    *seat-wide* event — verified in KDT's Rust, `application_state.rs:161-172` installs one handler
    on `default_seat.keyboard()`. Routing it to only the focused window loses any modifier held
    across a focus change (Alt-Tab with Alt down → window B never learns Alt is pressed).
    Noria: `cc557a8bf5daa`.
13. **Split `isFocused` into `hasActiveAppearance || hasKeyboardFocus`.** One field is written from
    two unrelated GTK signals (`is_active_notify` vs `ToplevelState::FOCUSED`), and
    `update_window_configure` re-sends the cached `active` on every resize — so a resize can clear
    focus on a window that holds the keyboard. Noria: `369319fadacfe`.

**Landed**, both as Noria has them. The broadcast made the tracker's `hasKeyboardFocus` and the
application's `mouseFocusWindowId` dead — Noria's GTK has neither — so both are gone, along with the two
local `keyboardModifiers` clears they guarded. `pointerButtons` still clears on `MouseExited`, as it does
in Noria: that one is about the pointer grab.

**GTK's `focusedWindow` was deliberately left as a `firstOrNull { it.isFocused }` scan.** Noria changed
only the Wayland one (`7ec5af76e8605`, WS2 finding 6) and left GTK's alone, and this is Wayland-specific
reasoning: on GTK the OR above means the scan can still match two windows during a switch. Worth
revisiting on its own evidence, not as part of this port.

## WS8 — Windows caption buttons (P1) — **done**

14. Every claimed title-bar rect maps to `HTCLIENT`, so a Compose-drawn maximize button never
    triggers the Win11 Snap Layouts flyout. `NCHitTestResult.MinButton/MaxButton/Close` already
    exist in KDT. AIR-5600, `a644825176abe`.

**Landed** with Noria's names — `CaptionButtonKind`, `setCaptionButtonBounds(kind, bounds)`, checked
first in `hitTestTitleBar` — but as a defaulted member on `ComposeWindow` rather than Noria's
`CaptionButtonsHostWindow` capability interface, which is this module's existing choice for
`setCustomTitleBarInteractiveAreas` and everything else in that seam. `@Volatile`/`@Synchronized`
dropped: Noria needs them because it reads the map from the WndProc thread, and here that is the same
thread the composition runs on.

## WS11 — macOS startup failure surfaces as a hang (P1) — **done**

Not in the original workstream list: [macos.md](macos.md) findings 2 and 3 are P1 and were never
promoted into one. This is finding 3.

22. `MacOsApplication`'s constructor blocks on `applicationStarted.await()`, and nothing caught a
    throw from `GrandCentralDispatch.startOnMainThread { … }` or counted the latch down in a
    `finally`. Any failure before `ApplicationDidFinishLaunching` — the native library, `Application.init`,
    `Screen.allScreens()`, a bad `kdt.library.folder.path`, or running headless — hung `main` with no
    diagnostic. Noria: `5c3e7e685eedf`.

**Landed** as the `eventLoopFailure` slot the Linux, GTK and Windows backends in this module already
had; macOS was the only one without it. Noria uses a `CompletableJob` and
`completeExceptionally`, but its `awaitWhenReady()` is `suspend` — here the constructor blocks, so the
latch is the right primitive and the house pattern is the closer match.

**Still open from that report: [macos.md](macos.md) finding 2** (P1) — a window that is not visible
stops recomposing entirely, because its display link is the only thing pumping its scene. Needs a
decision about how a scene with no display link is paced; Noria falls back to a rate-limited windowless
reconcile (AIR-6275, AIR-5036, AIR-5022).

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
    Also de-duplicates an `initToolkit()` copy-pasted between Linux and GTK. — **done**. Noria's
    `KdtLoggerAppender` adapts one Fleet `KLogger` onto the macOS `AppenderInterface`; a library cannot
    name a host logger, so a `ComposeLogAppender` interface of the same shape is the public seam and the
    four adapters onto the per-backend `AppenderInterface` are internal. The appender is assigned to
    `Logger.appender` after `init` rather than passed to it: `init` only takes effect once per process,
    and KDT's `DefaultConsoleAppender` is internal, so it cannot be named as a fallback.
18. **`InfiniteAnimationPolicy`/`MotionDurationScale` on the app composition.** `YieldFrameClock`
    (`Application.kt:274-279`) will spin the event loop at 100% CPU for an infinite animation
    outside a `Window`. Compose supplies both context elements; Noria installs
    `noWindowAnimationCoroutineContext` into every scene. — **done**: Noria's
    `NoWindowAnimationPolicy.kt` copied as-is, installed on the application composition rather than
    per scene, since here it is `YieldFrameClock` that has no display to pace against; each window's
    scene already runs off its own frame pump.
19. **`LinuxInputMethodCommon.kt`** — the target duplicates ~110 lines of IME arithmetic between
    `gtk/` and `linux/`, and `GtkKeys.kt` / `LinuxKeys.kt` are byte-identical modulo the package
    token. — **done, in part**: the arithmetic is hoisted under Noria's class name,
    `LinuxTextInputSurroundingText`, in a file with Noria's name. Verified that Noria's
    `LinuxInputMethodCommon.kt` really is shared and not a Wayland file that GTK ignores: both
    `linux/LinuxTextInputSessionOwner.kt` and `gtk/GtkTextInputSessionOwner.kt` import from it.
    A file holding one class trips ktlint's `standard:filename` rule, so the file carries a one-line
    `@file:Suppress` rather than being renamed away from Noria's. `toTextInputContentPurpose` stays
    duplicated, as it is in Noria: the `TextInputContentPurpose` enum it maps onto is per-backend.
    **`GtkKeys.kt` / `LinuxKeys.kt` are deliberately left duplicated** — Noria duplicates them too
    (`gtk/Key.kt`, `linux/Key.kt`), so merging them would diverge rather than converge.
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

# Findings that are not Noria ports

21. Compose's desktop `ScrollConfig` reads `awtEventOrNull`, which is always `null` for KDT events,
    so precise-wheel detection never fires and trackpad scrolling gets Compose's stepping-wheel
    animation stacked on top of macOS inertia. Fix mirrors the target's own `MacOsTypedEvents`
    trick. Fell out of the comparison; see [macos.md](macos.md) finding 15.

22. **macOS: a modal file dialog crashed the scene's dispatcher** — **fixed**. Found in use, not by the
    comparison. `NSOpenPanel`/`NSSavePanel`'s `runModal` spins a *nested* run loop, so the display link
    keeps ticking and Grand Central keeps delivering the frame block — inside the coroutine that opened
    the dialog, which the scene's `FlushCoroutineDispatcher` is still resuming after
    `sendPointerEvent`. The nested `scene.render` re-entered that dispatcher and resumed an
    already-completed continuation: `CompletedContinuation cannot be cast to DispatchedContinuation`,
    followed by the calling coroutine getting a coroutines-internal `Symbol` as its result.

    The fix is one word: `withContext(KdtDispatcher)` rather than `KdtImmediateDispatcher`. The
    immediate dispatcher does not dispatch when it is already on the event loop thread — that is its
    purpose — so the dialog ran inline on the caller's stack. Dispatching lets the flush unwind first.

    Two false starts, both reverted, both worth not repeating: suppressing frames during the modal
    treated the symptom (the frame pump was merely the first thing to walk into the nested loop), and it
    froze a window that stays visible next to the panel rather than under it. And the frame it requested
    on the way out was redundant — `invalidate` is already wired to the frame request, and resuming the
    caller onto the scene's dispatcher invalidates.

    **Windows had the same bug**, same construct around the same `open_file_dialog_run_modal`, and got
    the same fix. Its `withoutRenderReentrancy` guard does not catch it: that blocks a render inside a
    render, and here there is only a flush on the stack below. Unverified — no Windows machine here.
    Wayland and GTK are unaffected: their dialogs are `suspendCancellableCoroutine` over the portal's
    async response, so nothing blocks the loop.

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
