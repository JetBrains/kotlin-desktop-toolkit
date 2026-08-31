# macOS backend: Noria (SOURCE) → KDT-Compose (TARGET) comparison

- **TARGET**: `/Users/wojtek/IdeaProjects/kotlin-desktop-toolkit/kotlin-desktop-toolkit-compose/src/main/kotlin/org/jetbrains/desktop/compose/`
  (`macos/` plus the shared layer next to it)
- **SOURCE**: `/Users/wojtek/IdeaProjects/ultimate2/fleet/noria/ui/srcJvmMain/androidx/compose/ui/desktop/`
  (`macos/` plus the shared layer next to it)
- Git archaeology run in `/Users/wojtek/IdeaProjects/ultimate2`. Note: history before
  `62c30770b8e6d` lives under the old path `fleet/noria/ui/srcJvmMain/androidx/compose/ui/kdt/macos/`.

Read in full on both sides: `MacOsApplication`, `MacOsWindow`, `(MacOs)InputStateTracker`,
`MacOsKeys`/`Key`, `MacOsGpuContext`/`GpuContext`, `MacOsPointerIcon`/`PointerIcon`, `MacOsScreen`,
`MacOsConverters`/`Converters`, `MacOsClipboard`, `MacOsDragAndDropManager`,
`MacOsTextInput`/`MacOsTextInputSessionOwner` + `ComposeTextInputClient`, `MacOsTypedEvents`,
`DisplayLinkFramePump`, `Dispatchers`, `MacOsNotificationCenter`, `MacOsUriHandler`,
`ViewConfiguration`, `WindowIdsMapping`, `HeavyweightWindowId`. Cross-checked against this repo's KDT
bindings (`kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/macos/*.kt`) and against the
released Compose 1.10 desktop sources in the Gradle cache.

**Headline**: the target is genuinely ahead of Noria in several places (frame pump, GPU render-thread
locking, key-code mapping, resize/density handling, structure and docs). The gaps that matter are
concentrated in **IME session lifecycle**, **frame delivery when a window is not visible**, **startup
failure handling**, and **input-mode/scroll plumbing**.

---

## 1. IME session teardown leaves the composition marked and the input context active — P0

**Noria** ends a text-input session by explicitly deactivating the input context *and* telling the
client to drop the composition, and activates the context when the session starts:

`SOURCE macos/MacOsTextInputSessionOwner.kt:65-90`
```kotlin
override suspend fun startInputMethod(request: PlatformTextInputMethodRequestMacOs): Nothing {
    withContext(Dispatchers.Main.immediate) {
        val textInputClient = request.toTextInputClient(scene, nativeWindow)
        nativeWindow.setTextInputClient(textInputClient)
        nativeWindow.textInputContext.activate()          // <-- (a)
        currentTextInputClient = textInputClient
    }
    try { awaitCancellation() } finally {
        withContext(Dispatchers.Main.immediate + NonCancellable) {
            nativeWindow.textInputContext.discardMarkedText()
            currentTextInputClient?.unmarkText()          // <-- (b) drops Compose's composition
            nativeWindow.textInputContext.deactivate()    // <-- (c)
            nativeWindow.setTextInputClient(TextInputClient.Noop)
            currentTextInputClient = null
        }
    }
}
```
`unmarkText()` on the client routes into `request.editText { finishComposingText() }`
(`SOURCE macos/ComposeTextInputClient.kt:133-155`, and the live path
`SOURCE macos/MacOsTextInputSessionOwner.kt:195-198`).

**Target** does (b') `discardMarkedText()` + `invalidateCharacterCoordinates()` on the *native* side
only, and never touches `activate()`/`deactivate()`:

`TARGET macos/MacOsTextInput.kt:60-79`
```kotlin
    finally {
        withContext(NonCancellable + KdtImmediateDispatcher) {
            activeClient = null
            if (!isWindowGone) {
                nativeWindow.setTextInputClient(TextInputClient.Noop)
                nativeWindow.textInputContext.discardMarkedText()
                nativeWindow.textInputContext.invalidateCharacterCoordinates()
            }
        }
    }
```

**Origin**: `4b636801ab9adb0980c43489214d7b1bf2cb82c7` — *AIR-6619 [air]: activate and deactivate IME,
drop composed text makrer when text field lost focus* (this commit is exactly the three lines above).
Precursors in the same area: `f2937473ad550` *AIR-5137 … stop IME session when editor isn't active,
don't pass events to the session when editor isn't active*.

**Why it matters**
- `discardMarkedText()` tells *AppKit* to abandon the composition. It does **not** tell Compose. The
  Compose text field keeps `state.composition != null`, so the composing underline survives the loss
  of focus and the next edit operates on a stale composition range. KDT's own docs say so explicitly:
  `TextInputContext.deactivate` "does *not* end an in-flight composition, so pair it with
  `discardMarkedText`" (`kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/macos/TextInputClient.kt:54-70`).
- Without `deactivate()`, the window's input context stays the application's active one after the text
  field loses focus. With a CJK/Japanese IME that leaves the IME armed (candidate window / conversion
  state) while nothing is editable.
- `activate()` is needed when focus moves between fields inside one window without a first-responder
  change, and to restore a context this code previously deactivated
  (`TextInputClient.kt:37-52`).

**How to port (concrete, Compose)**
In `TARGET macos/MacOsTextInput.kt`:
1. In `startInputMethod`, after `nativeWindow.setTextInputClient(client)`, add
   `nativeWindow.textInputContext.activate()`.
2. In the `finally`, before dropping the client, do — in this order —
   `nativeWindow.textInputContext.discardMarkedText()`, then finish the Compose composition, then
   `nativeWindow.textInputContext.deactivate()`, then `setTextInputClient(TextInputClient.Noop)`.
   Keep `invalidateCharacterCoordinates()`.
3. For "finish the Compose composition", add a method to `MacOsTextInputClient` (the target's private
   class) that calls `request.editText { finishComposingText() }` — the same body as its existing
   `unmarkText()` override (`TARGET macos/MacOsTextInput.kt:159-162`) — and call it on the captured
   client. Do **not** call the `TextInputClient` override through the KDT interface; call it directly,
   because at this point the native client has (or is about to be) replaced by `Noop`.
4. Guard all of it with the existing `isWindowGone` flag — `Window.close()` in KDT already does
   `discardMarkedText()` + `deactivate()` (`Window.kt:397-404`), so the closing path must stay a no-op.

Compose-specific caveat: Noria's comment on (b) says calling `insertText`/commit instead of
`finishComposingText` would be more correct (typing handlers, undo) but broke their action context.
For Compose, `finishComposingText()` is the right call — it commits the composing text as final and
clears the range.

**Confidence**: high on (a)/(c) — the KDT API docs corroborate the intent. High-medium on (b): I did
not run a CJK typing session, but the target has no other path that clears the Compose composition
when the session ends.
**Risk**: low. All three calls are idempotent-ish and already guarded by `isWindowGone`.

---

## 2. A window that is not visible stops recomposing entirely — P1

**Noria** treats "no display link" as "this window cannot deliver frames" and falls back to a
windowless, rate-limited reconcile so the scene keeps progressing:

`SOURCE macos/MacOsApplication.kt:577-606`
```kotlin
fun requestFrame() {
    val windowsInScene = windows.values.filter { it.scene == scene && it.nativeWindow.isVisible }
    if (windowsInScene.isNotEmpty()) {
        if (!frameRequested) { frameRequested = true; windowsInScene.forEach { it.isFrameRequested = true } }
    } else if (!noWindowsFrameRequested) {
        noWindowsFrameRequested = true
        val delayMs = ((nextNoWindowsFrameDeadlineNs - System.nanoTime()).coerceAtLeast(0L) + 999_999L) / 1_000_000L
        launch { /* delay, then dispatchOnMain { scene.reconcile(true) } */ }
    }
}
```
plus `SOURCE macos/MacOsWindow.kt:641-656`, which schedules one last reconcile when the last window of
a scene is disposed with a frame still pending.

**Origin**:
- `5f9027c8532317cf250d0d74f294523cc7e7a551` — *AIR-6275 [air]: process system notifications even when
  window is not visible*: "When a window isn't visible, it doesn't have a display link, so the
  requested frame won't be rendered until the window is shown. To mitigate that, we filter out
  occluded windows and do windowless reconcile." The commit is literally
  `+ && it.nativeWindow.isVisible`.
- `cd71aa3a89dc5` — *AIR-5036 Limit recompositions without windows to at most 60/s* (where the
  `noWindowsMinFrameIntervalNs` throttle comes from, `SOURCE macos/MacOsApplication.kt:827`).
- `c232eb44416ed` — *AIR-5022 don't try to create display link if the window isn't visible*.

**Target**: the *only* thing that pumps a window's scene is its display link, and the link is stopped
whenever the window goes invisible:

`TARGET macos/MacOsWindow.kt:564-571`
```kotlin
is Event.WindowChangedOcclusionState -> {
    when {
        event.isVisible && displayLink == null -> setUpDisplayLink()
        else -> displayLink?.setRunning(event.isVisible)
    }
    EventHandlerResult.Stop
}
```
`TARGET macos/MacOsWindow.kt:692-710` returns early from `setUpDisplayLink()` when
`!nativeWindow.isVisible`, and logs "repainting is stalled until the window becomes visible again".

**Why it matters (Compose specifically)**
`CanvasLayersComposeScene` owns its own `BroadcastFrameClock`; the clock only ticks inside
`scene.render(...)` (`TARGET macos/MacOsWindow.kt:712-719`). No display link ⇒ no `render` ⇒ no frame
clock tick ⇒ **the Recomposer for that window never recomposes**. Consequences, all real for Compose:
- Any `LaunchedEffect`/animation that awaits `withFrameNanos` stalls for as long as the window is
  occluded or minimized.
- Snapshot writes made from event handlers, timers or background coroutines never reach composition,
  so the window's state machine (not just its pixels) freezes — the AIR-6275 symptom.
- `DisplayLink.create` returning `null` (machine asleep, all displays gone;
  `TARGET macos/MacOsWindow.kt:698-707`) is the same failure with no recovery until the next occlusion
  or screen-change event.

**How to port (concrete, Compose)**
The target has no scene-level `reconcile`, so the Noria fallback maps onto "render into a throwaway
target at a low rate":
1. In `MacOsWindow`, add a fallback pump used only while `displayLink == null || !nativeWindow.isVisible`
   *and* `isFrameRequested` is set: a coroutine on `KdtDispatcher` that, at most ~60/s (reuse Noria's
   `nextNoWindowsFrameDeadlineNs` idea, `SOURCE macos/MacOsApplication.kt:586,597`), calls
   `recordFrame()` and immediately closes the returned picture without presenting it. Recording is the
   part that ticks the frame clock; presenting is what you want to skip.
   A cheaper variant: give the scene an explicit tick by calling `scene.render` into a 1×1 recorder.
2. Arm/disarm it from the two places that today just stop the link:
   `Event.WindowChangedOcclusionState` (`:564`) and the `link == null` branch of `setUpDisplayLink`
   (`:698`).
3. Mirror Noria's dispose case (`SOURCE macos/MacOsWindow.kt:641-656`): if `isFrameRequested` is still
   set when `dispose()` runs, do one last tick before `scene.close()` — otherwise work queued for that
   frame is silently dropped.

Adaptation note: Noria's fallback is *per scene* (many windows share one scene). Compose here is
one-scene-per-window, so it is per window, which is simpler.

**Confidence**: high that the mechanism is missing and that recomposition is frame-clock-gated.
Medium on which exact fallback shape is best for Compose — recording a frame you throw away is
wasteful, and if `CanvasLayersComposeScene` ever exposes a "tick the clock" entry point that is
strictly better.
**Risk**: medium — a fallback pump that fails to disarm burns CPU in the background; make disarming
unconditional in `dispose()` and on the visible transition.

---

## 3. Startup deadlocks instead of failing when the event loop thread throws — P1

**Noria** wraps the whole main-thread handover and completes the readiness signal exceptionally:

`SOURCE macos/MacOsApplication.kt:223-388`
```kotlin
private fun startEventLoopThread(): Thread = thread(start = true, name = "EventLoopWatcher") {
    try {
        GrandCentralDispatch.startOnMainThread { /* init, run event loop */ }
    } catch (throwable: Throwable) {
        didFinishLaunchingCompletableJob.completeExceptionally(throwable)
    }
}
```
with `awaitWhenReady()` = `didFinishLaunchingCompletableJob.join()`
(`SOURCE macos/MacOsApplication.kt:443-445`).

**Target** uses a `CountDownLatch` with no failure path:

`TARGET macos/MacOsApplication.kt:68` `private val applicationStarted = CountDownLatch(1)`
`TARGET macos/MacOsApplication.kt:126-142` — nothing catches a throw from `startOnMainThread { … }`
`TARGET macos/MacOsApplication.kt:326-333`
```kotlin
init {
    eventLoopThread = startEventLoopThread()
    applicationStarted.await()   // <-- forever, if the thread above died
}
```

**Origin**: `5c3e7e685eedf42d7cebc4846d03d1dfcb38696c` — *[fleet, air] application was hanging when
started in headless mode on macos; it was observed in Generate webhelp [Mac] [x64]*. That commit is
the one that unified the readiness signal onto `awaitWhenReady()`; the `try/catch` +
`completeExceptionally` shape is what makes a failed boot surface as an exception instead of a hang.

**Why it matters**: any failure before `Event.ApplicationDidFinishLaunching` — native library load,
`Application.init`, `Screen.allScreens()`, `DragAndDropHandler.init`, a bad `kdt.library.folder.path`
system property (`KotlinDesktopToolkit.kt:35-37` throws `Error`), or running headless — hangs `main`
with no diagnostic. Anything worse than a stack trace for a library entry point.

**How to port**: keep the latch (it is the right primitive for a blocking constructor) and add a
failure slot:
```kotlin
private val applicationStarted = CountDownLatch(1)
@Volatile private var startupFailure: Throwable? = null
// in startEventLoopThread:
try { GrandCentralDispatch.startOnMainThread { … } }
catch (t: Throwable) { startupFailure = t; }
finally { applicationStarted.countDown() }
// in init:
applicationStarted.await()
startupFailure?.let { throw IllegalStateException("KDT application failed to start", it) }
```
Also count down in the `finally` of the normal path, so a `runEventLoop` that returns before
`ApplicationDidFinishLaunching` cannot hang either. Consider a bounded `await(timeout)` with a clear
error as a second belt.

**Confidence**: high — this is a pure control-flow gap, verifiable by reading the two files.
**Risk**: very low.

---

## 4. Scrolling never switches the input mode back to pointer — P1

**Noria** requests `InputMode.Touch` from the scroll handler:

`SOURCE macos/InputStateTracker.kt:235-249`
```kotlin
when {
    processResult.anyScrollingConsumed -> { sendPointerInputEventWithCurrentStateIfNecessary(PointerEventType.Move); EventHandlerResult.Stop }
    inputModeManager.inputMode == InputMode.Keyboard -> {
        // todo[unterhofer] This won't work with the hide stack in macOS
        inputModeManager.requestInputMode(InputMode.Touch)
        sendPointerInputEventWithCurrentStateIfNecessary(PointerEventType.Move)
        EventHandlerResult.Continue
    }
    else -> EventHandlerResult.Continue
}
```
Every other pointer branch in Noria does the same (`:75-77`, `:103-105`, `:138-140`, `:159-161`,
`:179-181`, `:199-201`).

**Target** funnels the mouse branches through `startPointerEvent()`
(`TARGET macos/MacOsInputStateTracker.kt:248-254`) which does request `InputMode.Touch` — but the
`ScrollWheel` branch deliberately bypasses it:

`TARGET macos/MacOsInputStateTracker.kt:183-199`
```kotlin
is Event.ScrollWheel -> {
    lastEventTimeMillis = event.timestamp.toDuration().inWholeMilliseconds   // no startPointerEvent()
    movePointerTo(...)
    sendPointerEvent(PointerEventType.Scroll, ...)
    sendPointerEventWithCurrentState(PointerEventType.Move)
    EventHandlerResult.Stop
}
```
`Event.Swipe` (`:235-242`) has the same gap.

**Why it matters**: the target's `inputModeManager` also drives the cursor
(`TARGET macos/MacOsApplication.kt:118-120`: `Cursor.setHiddenUntilMouseMoves(inputMode == Keyboard)`),
and Compose keys focus rings off `LocalInputModeManager`. After the user types (mode → `Keyboard`),
scrolling with a trackpad or wheel is pointer input, but the mode stays `Keyboard`: focus indicators
stay drawn and — this is the visible half — the target's own post-relayout synthetic pointer event
picks `PointerEventType.Exit` instead of `Move`
(`TARGET macos/MacOsInputStateTracker.kt:351-356`), so hover state under a stationary pointer is
dropped after every relayout that follows a keystroke.

**How to port**: replace `lastEventTimeMillis = …` with `startPointerEvent(…)` in both the
`ScrollWheel` and `Swipe` branches. That is a strict superset of Noria's behaviour and matches the
rest of the target's own file. Note the target's `MacOsApplication.handleEvent` already un-hides the
cursor for movement events (`TARGET macos/MacOsApplication.kt:148-152`,
`isPointerMovement()` at `:336-337`) but `ScrollWheel`/`Swipe` are **not** in that set either — worth
adding there too, so scrolling reveals a cursor hidden by typing.

**Confidence**: high.
**Risk**: very low. Noria's `todo[unterhofer]` about "the hide stack in macOS" only concerns
`pushHide`/`popHide`, which the target does not use.

---

## 5. Window position is stale after the window has been hidden — P2

**Noria** re-reads the origin when a window becomes visible again:

`SOURCE macos/MacOsWindow.kt:1037-1041`
```kotlin
is Event.WindowChangedOcclusionState -> {
    if (event.isVisible) {
        position = nativeWindow.origin.toDpOffset()
    }
    …
```

**Origin**: `e3926148c0078ea89147bea234e8714472384209` — *[fleet,air] window position is reliable only
when the window is visible*. (The same commit also fixed `contentPosition()` to be
`contentOrigin - windowOrigin` rather than raw `contentOrigin` — the target already has the fixed
form at `TARGET macos/MacOsWindow.kt:750-754`.)

**Target**: `TARGET macos/MacOsWindow.kt:564-571` handles occlusion for the display link only; the
observable `position` (`:163`) is only written from `Event.WindowMove`/`WindowResize`.

**Why it matters**: AppKit's reported origin is unreliable while a window is off-screen (minimized,
on another Space, restored by the window server). Anything reading `ComposeWindow.position` to persist
or restore bounds — Noria hit this in `d9aba8972bdf3` *AIR-4945/4937/4980/4982 Fix persistence and
restoration of window bounds* — sees a stale value until the next user-initiated move.

**How to port**: one-line addition to the existing occlusion branch. Consider also refreshing `size`,
`contentSize`, `screen` and `density` there, since the window may have been moved to a different
display while hidden and `WindowScreenChange` is not guaranteed for a hidden window.

**Confidence**: medium-high (the commit message is unambiguous about the underlying platform
behaviour; I have not reproduced it).
**Risk**: very low.

---

## 6. No custom-title-bar drag / double-click-to-zoom fallback — P2

**Noria** intercepts an unconsumed left `MouseDown` inside the custom title bar and gives it the
system behaviour:

`SOURCE macos/MacOsWindow.kt:1068-1072` and `:1100-1113`
```kotlin
is Event.MouseDown if result == EventHandlerResult.Continue &&
    decoration is WindowDecoration.CustomTitleBar -> sendEventIntoTitleBar(event)
…
private fun sendEventIntoTitleBar(mouseDownEvent: Event.MouseDown): EventHandlerResult {
    return if (mouseDownEvent.button == MouseButton.LEFT &&
        mouseDownEvent.locationInWindow.y < (decoration as WindowDecoration.CustomTitleBar).height.value) {
        when (mouseDownEvent.clickCount) {
            1L -> nativeWindow.startDragWindow()
            else -> nativeWindow.toggleMaximize()
        }
        EventHandlerResult.Stop
    } else EventHandlerResult.Continue
}
```

**Target** exposes `startInteractiveMove(pointerEvent)` (`TARGET macos/MacOsWindow.kt:453-456`,
API at `TARGET Window.kt:192`) and expects application content to call it. There is no fallback, and
crucially the tracker makes one impossible today: `Event.MouseDown` always returns
`EventHandlerResult.Stop` (`TARGET macos/MacOsInputStateTracker.kt:128`), so `handleEvent` can never
see `Continue` for a press.

**Origin**: predates the path rename (`62c30770b8e6d`); no dedicated ticket found.

**Why it matters**: with `WindowDecoration.CustomTitleBar`, an app that forgets to wire a draggable
area gets a window that cannot be moved, and double-click-to-zoom (a macOS system preference:
"Double-click a window's title bar to zoom") never works regardless of what the app does.

**How to port**: this is a *policy* question for a Compose library, so I would not port it verbatim.
Two options, in order of preference for Compose:
1. Provide the missing composable instead: a `WindowDraggableArea`-equivalent in the shared layer that
   calls `window.startInteractiveMove(...)` on drag and `window.requestPlacement(...)` on double
   click. That mirrors Compose Desktop's own API and keeps the platform layer dumb.
2. If a fallback is wanted, it needs the tracker change too: return `Continue` from `MouseDown` when
   nothing in the scene consumed it. That is currently impossible from outside compose-ui — the target
   documents exactly why at `TARGET macos/MacOsInputStateTracker.kt:66-74` (`PointerEventResult`
   members are internal). So option 2 is effectively blocked; do option 1.

**Confidence**: high on the gap, low on "port Noria's shape".
**Risk**: n/a for option 1.

---

## 7. `isQuitInProgress` is never reset: a cancelled quit disables Quit forever — P2

**Target** `macos/MacOsApplication.kt:194-209`
```kotlin
private fun onQuitRequested(): Boolean {
    val isSafeToQuit = quitHandlers.values.fold(true) { acc, handler -> handler() and acc }
    if (!isSafeToQuit || isQuitInProgress) return false
    val openWindows = windowList.toList()
    if (openWindows.isEmpty()) return true
    isQuitInProgress = true                       // <-- set, never cleared
    openWindows.forEach { it.requestCloseFromQuit() }
    return false
}
```
If the application's `onCloseRequest` decides *not* to close (an unsaved-changes dialog the user
cancels), `isQuitInProgress` stays `true` and every later ⌘Q returns `false` at the second line — Quit
is dead for the rest of the process's life.

**Noria** has the same flag but clears it on the way through shutdown
(`SOURCE macos/MacOsApplication.kt:455-457`: `structuredQuitInProgress = false` at the top of
`stopAndJoin()`) and drives completion explicitly from window disposal
(`SOURCE macos/MacOsWindow.kt:663-667` → `finishStructuredQuitIfNeeded()`,
`SOURCE macos/MacOsApplication.kt:530-538`). Noria does not clear it on a *veto* either, so this is
not strictly a Noria fix — it is a bug the comparison surfaced, and Noria's structure suggests the
shape of the fix.

**How to port / fix**: the target's model is "the composition ending is what terminates", so the flag
only needs to mean "a structured close is already in flight". Reset it when the wave finishes without
the application exiting — simplest robust version: in `detachWindow`, if `isQuitInProgress` and
`windowList.isEmpty()`, clear it; and clear it in `onQuitRequested` if the previous wave left windows
open and none of them is closing (or just drop the flag and make `requestCloseFromQuit` idempotent per
window). Add a `Logger.info` when a quit is refused because one is already in progress.

**Confidence**: high on the code path; medium on "user-visible" (depends on the host app vetoing).
**Risk**: low.

---

## 8. KDT native logs are not routed into the host's logger — P2

**Noria** `SOURCE macos/MacOsApplication.kt:191-204`
```kotlin
val logFilePath = logFolderPath.resolve("MacOsApplication").resolve("MacOsApplication.log")
KotlinDesktopToolkit.init(libraryFolderPath, logFilePath, appenderInterface = KdtLoggerAppender(kdtLogger))
```
with `SOURCE ../KdtLoggerAppender.kt` bridging KDT's Rust-side log records into `fleet.util.logging`,
under a dedicated logger name (`SOURCE macos/MacOsApplication.kt:124`).

**Target** `macos/MacOsApplication.kt:62-64`
```kotlin
init { KotlinDesktopToolkit.init() }
```
which falls back to `System.getProperty("kdt.library.folder.path")` /
`kdt.native.log.path` and throws `Error` if unset
(`kotlin-desktop-toolkit/.../KotlinDesktopToolkit.kt:11-37`), and to
`DefaultConsoleAppender.fromLevel(Info)`.

**Origin**: `fd3aa5d8f51d1` *do not evaluate logging messages before actual logging* and
`b049c1dd69ba9` *[air] Make loggers lazy* are the related hygiene commits (the target already uses
lambda-form `Logger.info { }` everywhere, so those are done).

**Why it matters**: for a library, "you must set two JVM system properties or we throw an `Error`
during construction" is a rough contract, and native log output that cannot be redirected into the
embedder's logging stack is a support problem.

**How to port**: surface the four knobs on `ComposeApplicationParams` — library folder, log file,
log level, and an appender — and pass them through each backend's `init`. Default to today's
behaviour so nothing changes for existing callers. The target's `BackendLogger.kt` is already the right
home for the platform-agnostic half.

**Confidence**: high. **Risk**: low (API addition).

---

## 9. `setMarkedText` ignores the IME's `selectedRange` — P2

**Noria** passes the IME's selection inside the composition through to the client
(`SOURCE macos/MacOsTextInputSessionOwner.kt:200-211`), and its (currently commented-out) reference
implementation applies it:

`SOURCE macos/ComposeTextInputClient.kt:205-220`
```kotlin
selectedRange?.let {
    val effectiveSelectedRangeStart = effectiveReplacementRangeStart + it.location
    val effectiveSelectedRangeEnd = effectiveSelectedRangeStart + it.length
    setSelection(TextRange(effectiveSelectedRangeStart.toInt(), effectiveSelectedRangeEnd.toInt()))
}
```

**Target** drops both range arguments:

`TARGET macos/MacOsTextInput.kt:164-169`
```kotlin
override fun setMarkedText(text: String, selectedRange: KdtTextRange?, replacementRange: KdtTextRange?) {
    didEditText = true
    request.editText { setComposingText(text, 1) }
}
```

**Why it matters**: `selectedRange` is how an IME says "the caret/active clause is *here* inside the
composition" — multi-clause Japanese conversion, and Korean/Vietnamese input, rely on it. Without it
the caret always sits at the end of the composing text, and the active clause is invisible.

**How to port**: inside `request.editText { }`, after `setComposingText(text, 1)`, apply
`setSelection(compositionStart + selectedRange.location, … + length)`. The composition start is
readable from `request.state.composition?.start` before the edit. Verify against the actual
`EditingBuffer`-style scope the released Compose `PlatformTextInputMethodRequest.editText` exposes —
the target's own comment at `:147-149` claims the released scope "can only edit around the selection
and the composition", so `setSelection` may or may not be available. Check before committing to this.

**Confidence**: medium-high on the gap; **low** on the exact API being available in released Compose
(Noria's version is commented out and targets their own fork).
**Risk**: medium — a wrong selection index is worse than no selection. Gate on a null check and clamp.

---

## 10. Nothing surfaces frame-pacing diagnostics — P3

**Noria** times each display-link frame from tick to present-complete and logs slow ones:

`SOURCE macos/MacOsWindow.kt:889` and `:906-915`
```kotlin
val frameStartTimeMarkWrapper = TimeMarkWrapper(TimeSource.Monotonic.markNow())
…
onComplete = {
    presentablePicture.close()
    val elapsedTime = displayLinkFrameStartTimeMark.exchange(null)!!.timeMark.elapsedNow()
    if (elapsedTime.inWholeMilliseconds > 10) { logger.debug { "Long frame: $elapsedTime" } }
}
```

**Target**'s `DisplayLinkFramePump` (`TARGET macos/DisplayLinkFramePump.kt`) uses an opaque token as
the latch and measures nothing.

**Why it matters**: purely diagnostic, but frame pacing is exactly the thing you cannot debug after
the fact. Noria's own frame-pump commits (`e95153775d46c`, `06e9266c2f07d`, `1c212e809ce7a`) are all
in this area.

**How to port**: make the token a small `data class Frame(val startedAt: TimeSource.Monotonic.ValueTimeMark)`
instead of `Any()` (`DisplayLinkFramePump.kt:32-34`), and log in the `presentAsync` completion
callback. Structurally the target's pump is *better* than Noria's — keep the CAS-token design and the
`retry(token)` invariant; only add the measurement.

**Confidence**: high. **Risk**: none.

---

## 11. `insertText` ignores `replacementRange` — P3

**Noria** (commented reference, `SOURCE macos/ComposeTextInputClient.kt:96-116`) does
`setSelection(replacementRange)` before `commitText`. **Target** documents the omission and skips it
(`TARGET macos/MacOsTextInput.kt:146-151`): *"macOS only sends a range for corrections we do not
offer"*.

That reasoning holds for the current feature set (no dictation/autocorrect/Touch Bar replacement
surfaces), so this is fine as-is. Worth revisiting if the target ever wires up `doCommand` or
dictation. **Confidence**: medium — I could not verify which AppKit callers pass a non-null
`replacementRange` in practice.

---

## 12. Frame canvas is cleared to opaque white — P3

**Noria** `SOURCE macos/MacOsWindow.kt:871`: `canvas.clear(org.jetbrains.skia.Color.TRANSPARENT)`.
**Target** `TARGET macos/MacOsWindow.kt:716`: `canvas.clear(Color.White.toArgb())`.

For Compose this is defensible (it matches Compose Desktop's default opaque white window background),
but it forecloses transparent / vibrancy windows, and any first frame before content draws flashes
white — noticeable in dark mode. KDT exposes both `MetalView.isOpaque`
(`kotlin-desktop-toolkit/.../Metal.kt:67-69`) and `Window.setBackground(WindowBackground)`
(`Window.kt:319-325`), neither of which either side uses.

**Suggestion (not a straight port)**: clear to transparent and let `Window.setBackground` /
`MetalView.isOpaque` decide what shows through, exposing a background/transparency parameter on
`ComposeWindowParams`. **Confidence**: low on which default is right for a Compose library. **Risk**:
medium — changing the clear colour changes every window's appearance.

---

## 13. Dispose order: native window is closed *after* its Metal view is destroyed — P3

**Noria** `SOURCE macos/MacOsWindow.kt:657-661`: `nativeWindow.close()` → then
`destroyMetalViewContext(viewContext)`.
**Target** `TARGET macos/MacOsWindow.kt:768-771`: `scene.close()` → `pictureRecorder.close()` →
`destroyMetalViewContext(viewContext)` → `nativeWindow.close()`.

The target destroys (and `close()`s) the `MetalView` while it is still attached to a live `NSWindow`.
`MetalView.close()` defers its arena close to the main queue specifically to dodge a display-link
re-entrancy SIGSEGV (`Metal.kt:71-78`), which suggests this teardown is order-sensitive. Noria's order
— detach by closing the window, then the view — is the safer one.

Separately, the target closes `pictureRecorder` before `stopRenderThread()` runs (inside
`destroyMetalViewContext`), so an in-flight `Picture` recorded by that recorder can still be on the
render thread when its recorder is freed. Swapping the two lines removes the window.

**Suggested order**: `textInput.onWindowClosing()` → stop/close display link → `scene.close()` →
`nativeWindow.close()` → `destroyMetalViewContext(viewContext)` (which joins the render thread) →
`pictureRecorder.close()` → `application.detachWindow(this)`.

**Confidence**: low-medium — I have not observed a crash, and Skia `Picture` lifetime relative to
`PictureRecorder` may be independent. Worth a look with a stress test (open/close windows in a loop).
**Risk**: low (pure reordering), but touch it deliberately.

---

## 14. `minSize` / `maxSize` / `isResizable` are not observable — P3

**Noria** mirrors each into snapshot state so a composition that reads them recomposes:
`SOURCE macos/MacOsWindow.kt:231-275` (`minSizeState`, `maxSizeState`, `isUserResizableState`).
**Target** `TARGET macos/MacOsWindow.kt:191-193` reads straight through to the native window:
```kotlin
override val minSize: DpSize get() = nativeWindow.minSize.toDpSize()
override val maxSize: DpSize get() = nativeWindow.maxSize.toDpSize()
override val isResizable: Boolean get() = nativeWindow.isResizable
```
Reading these inside a composable never subscribes, so a `requestMinSize` from elsewhere does not
recompose readers. Note Noria's implementation is ugly (`get() = native.also { state.value = it }`
writes state from a getter) — port the *intent*, not the shape: back each with a `mutableStateOf`
written from `request*`, exactly like the target already does for `title`
(`TARGET macos/MacOsWindow.kt:151-161`).

The same applies to `layoutDirection` (`TARGET macos/MacOsWindow.kt:195-199`) — a plain getter, and
`scene.layoutDirection` is only set at construction, so a system reading-order change never reaches
the scene. Noria has the identical gap (`SOURCE macos/MacOsWindow.kt:443-444` is never reassigned), so
there is nothing to port — but it is a real bug on both sides.

**Confidence**: high. **Risk**: very low.

---

## 15. Compose's macOS `ScrollConfig` cannot detect a precise wheel — P2 (not a Noria port)

This is not a Noria finding — Noria bypasses Compose's `ScrollConfig` entirely — but it falls out of
the scroll comparison and is actionable.

Released Compose 1.10 (`foundation-desktop-1.10.0-sources.jar`,
`desktopMain/androidx/compose/foundation/gestures/DesktopScrollable.desktop.kt`) decides smooth-scroll
behaviour from the **AWT** event:
```kotlin
private val PointerEvent.isPreciseWheelRotation
    get() = (awtEventOrNull as? MouseWheelEvent)?.isPreciseWheelRotation ?: false
private val PointerEvent.scrollAmount
    get() = (awtEventOrNull as? MouseWheelEvent)?.scrollAmount?.toFloat() ?: 1f
private val PointerEvent.shouldScrollByPage
    get() = (awtEventOrNull as? MouseWheelEvent)?.scrollType == MouseWheelEvent.WHEEL_BLOCK_SCROLL
```
The target passes the KDT `Event.ScrollWheel` as `nativeEvent`
(`TARGET macos/MacOsInputStateTracker.kt:186-195`), so `awtEventOrNull` is `null` and
`isPreciseWheelScroll` is **always false**. Compose therefore applies its stepping-wheel smooth-scroll
animation to trackpad input, on top of macOS's own inertia — trackpad scrolling feels laggy/rubbery
rather than 1:1.

**How to fix**: the target already has the exact pattern for this — `MacOsTypedEvents.kt` synthesizes a
`java.awt.event.KeyEvent` purely so Compose's internal `isTypedEvent` check passes. Do the same for
scroll: synthesize a `java.awt.event.MouseWheelEvent` with a `preciseWheelRotation` that differs from
`wheelRotation` when `hasPreciseScrillingDeltas` is true (and equals it otherwise), `scrollAmount = 1`,
`scrollType = WHEEL_UNIT_SCROLL`, and pass it as `nativeEvent`. Reuse
`MacOsTypedEvents.SyntheticEventSource` as the component.

**Confidence**: high on the mechanism (read from the actual released Compose source); medium on how
much the user notices.
**Risk**: low-medium — `MouseWheelEvent`'s constructor is fussy; the target's `MacOsTypedEvents`
already shows how to satisfy such a constructor.

---

# Present in Noria, absent in the target

| Noria capability | File | Does Compose need it? |
|---|---|---|
| User notifications (`UNUserNotificationCenter`): authorization, categories, actions, action→window focus | `macos/MacOsNotificationCenter.kt` (243 lines) | **Yes, eventually** — it is a real desktop-app need and KDT already exposes `NotificationCenter`. Note the null-when-unsupported pattern (`:27-35`, from `8425dae7e3aa1` *AIR-5002 do not throw when notification center is not initialiazed*) and the init-on-`ApplicationDidFinishLaunching` placement (`SOURCE macos/MacOsApplication.kt:357-361`, race fixed in `7bbb13bf0a716`). Not a window-management concern, so a separate module would be fine. |
| Application menu bar: `setMainMenu(AppMenuStructure?)` / `setMainMenuToNone()` | `SOURCE macos/MacOsApplication.kt:393-399` | **Yes.** The target calls `AppMenuManager.offerCurrentEvent()` (`TARGET macos/MacOsWindow.kt:619`) to blink menu items but offers no way to *install* a menu — so nothing can blink. A macOS app without a menu bar is not shippable. |
| Dock icon (`setDockIcon`), emoji & symbols palette (`orderFrontCharactersPalette`), `Sound` | `SOURCE macos/MacOsApplication.kt:401-407, 418` | Low priority; cheap one-liners when needed. |
| `awaitWhenReady()` suspend readiness signal | `SOURCE macos/MacOsApplication.kt:443-445` | Useful — see finding 3. The target's blocking constructor covers the common case. |
| Full file-dialog parameter surface (`prompt`, `message`, `nameFieldLabel`, `directoryPath`, `showsHiddenFiles`, `canSelectHiddenExtensions`, `isExtensionHidden`, `resolvesAliases`; separate single/multiple open) | `SOURCE macos/MacOsWindow.kt:478-574` | **Yes** — the target exposes only `title` + three booleans (`TARGET macos/MacOsWindow.kt:458-486`). KDT's `FileDialog.CommonDialogParams` already has the rest. Related fix worth knowing: `b2a4d7b9115a4` *when a directory path is not provided, we should pass null but not "null"* — the target is not exposed to it because it never passes a directory, but do not reintroduce it. |
| `PointerIconService`: `getIcon()`, `pushHide()`/`popHide()`, `setHiddenUntilPointerMoves()` | `SOURCE macos/PointerIcon.kt:9-26` (simplified in `1b0ec65043ca9`) | **No** for the push/pop stack — Compose's `PlatformContext` has only `setPointerIcon`. The target already covers `setHiddenUntilMouseMoves` via `KdtInputModeManager`. |
| `captureScreenshot()` | `SOURCE macos/MacOsWindow.kt:628-630` — `TODO()` | No (unimplemented there too). |
| `Dispatchers.kt` — a macOS-specific main dispatcher | `SOURCE macos/Dispatchers.kt` | Already covered, and better: the target's `KdtDispatcher` is platform-agnostic and resolves the UI thread via `currentApplication()`. |
| `ViewConfiguration.kt` as a named class | `SOURCE macos/ViewConfiguration.kt` | No — the target inlines an equivalent anonymous object (`TARGET macos/MacOsWindow.kt:215-220`). Values are identical (500/300/40/18.dp). |
| `MacOsUriHandler.kt` | `SOURCE macos/MacOsUriHandler.kt` | Already covered inline (`TARGET macos/MacOsApplication.kt:108-112`). |
| `WindowIdsMapping.kt`, `HeavyweightWindowId.kt` | — | Out of scope (window reuse). Nothing independently useful found inside them. |
| `ComposeTextInputClient.kt` | — | Entirely commented out in Noria; mined for findings 9 and 11 only. It also contains an unused `silentlyConsumedEvent` heuristic for the diacritics/accent popup (`:29-49`) — the target solves the same problem differently and, I think, better, via `didEditText` (`TARGET macos/MacOsTextInput.kt:128-138, 102-113`), which came out of `15d575e601c34` *Fix arrow keys and esc in diacritics popup on macOS*. |

---

# Divergences deliberately judged NOT worth porting

1. **Scroll delta scaling** — Noria uses `×1` for precise and `×10` for stepped deltas
   (`SOURCE macos/InputStateTracker.kt:667-686`); the target uses `×0.1` / `×1`
   (`TARGET macos/MacOsInputStateTracker.kt:409-415`). **The target is right for Compose.** Compose's
   `MacOSCocoaConfig.calculateMouseWheelScroll` multiplies the reported delta by `10.dp.toPx()`
   (verified in `foundation-desktop-1.10.0-sources.jar`,
   `DesktopScrollable.desktop.kt`), so Compose expects deltas in wheel *notches*, not points. Noria
   has no such multiplier because it consumes scroll itself. Porting Noria's constants would make
   scrolling 10× too fast. High confidence.
2. **`withPreparedMainThread` / `withoutReentrancy` / snapshot pinning** — explicitly out of scope, and
   Noria-runtime specific.
3. **Window reuse & pooling**, `reusableNativeWindowResources`, `WindowIdsMapping`,
   `HeavyweightWindowId`, `prepareNativeWindowResourcesForReuse` / `reuseWindow` — out of scope.
4. **The extended `Key` enum and `keyFromInternationalSymbols`** (`SOURCE macos/Key.kt:283-401`) — out
   of scope; Compose's `Key` has no `AAcute`/`CCaron`/etc. Note the *keycode mapping* itself is in good
   shape on the target: `b239c921ba6df` *AIR-6023 fix: Mouse forward button triggers back, back button
   does nothing on MacOS* changed `MouseButton.toKey()` to a 0-based `value` switch, and the target
   already has exactly that (`TARGET macos/MacOsKeys.kt:89-100`, with the reason in a comment). Same
   for `Key.isModifier` (`SOURCE macos/Key.kt:466-483`), which the target does not need because it
   never has to distinguish synthetic modifier events.
5. **Modifier-transition synthetic key events use the *previous* mask in Noria**
   (`SOURCE macos/InputStateTracker.kt:406-419`: `isCtrlPressed = previousKeyboardModifiers.isCtrlPressed`)
   vs the *new* mask in the target (`TARGET macos/MacOsInputStateTracker.kt:324-336`). **The target is
   right**: AWT's `KEY_PRESSED` for Shift already reports `isShiftPressed = true`, and `KEY_RELEASED`
   reports it false, which is what the new mask gives. Do not port.
6. **GPU render thread** — Noria's version has three defects the target has already fixed:
   `if (pendingTask == null) hasPendingTask.await()` (single `if`, so a spurious wakeup NPEs on
   `pendingTask!!`) vs the target's `while`; `assert(pendingTask == null)` (disabled by default) vs the
   target's `check`; and `onComplete` invoked *inside* `renderTaskLock` and *before* `pendingTask` is
   cleared (`SOURCE macos/GpuContext.kt:63-85, 92-106`), which the target inverts
   (`TARGET macos/MacOsGpuContext.kt:68-104`). Keep the target's version. Origin of Noria's shape:
   `d062add14f12a` *AIR-4952 Make MacOsWindow more resilient against exceptions during frame preparation*.
7. **`MouseDown`/`MouseUp` returning `Continue` when unconsumed** — Noria threads
   `ProcessResult.anyPressOrReleaseConsumed` back to AppKit; the target always returns `Stop` and
   documents why at length (`TARGET macos/MacOsInputStateTracker.kt:66-74`): Compose's
   `PointerEventResult` members are internal to compose-ui, so a host outside those modules cannot
   read them. Not portable until Compose opens that up. (This is what blocks finding 6, option 2.)
8. **`setupDisplayLink` early-return placement** — Noria returns *before* closing the old link when the
   window is invisible (`SOURCE macos/MacOsWindow.kt:882-887`), leaving a link bound to the old screen
   alive; the target closes and nulls first (`TARGET macos/MacOsWindow.kt:692-697`). Target is better.
   `AIR-5022` (`25bd01df893bf`, *after the display link is closed, it must be nulled*) and
   `c232eb44416ed` are both already satisfied by the target.
9. **`requestDecoration(vararg decorations)` with a preference list** including
   `WindowDecoration.Undecorated` (`SOURCE macos/MacOsWindow.kt:361-378`) — Noria-framework API shape,
   and `Undecorated` is `continue`d there anyway (unsupported).
10. **Clipboard `WindowLocalDrag` and `CustomSerializable` formats**
    (`SOURCE macos/MacOsClipboard.kt:59-70, 107-111, 119, 129-134`) — Fleet-internal transfer types.
    The target's four-format set is the right Compose surface. Both sides already have
    `4d91d0379958c` *Fix KDT macOS clipboard suspend functions* (the `withContext(main.immediate)`
    wrapping).
11. **Narrower `registerForDraggedTypes`** — Noria registers only `FILE_URL_TYPE` +
    `windowLocalDrag` (`SOURCE macos/MacOsWindow.kt:181-186`); the target registers text/HTML/file/PNG
    (`TARGET macos/MacOsWindow.kt:115-122`), which is the correct superset for a general-purpose
    library.
12. **`MacOsScreen` as a data class with declared `bounds`** — equivalent; the target's `ComposeScreen`
    interface supplies `bounds` as a default (`TARGET ComposeScreen.kt:182`). The target's doc comment
    on structural equality also answers Noria's own `//todo[unterhofer] Make these reactive`.
13. **`AIR-6291` pointer-icon-stuck fix** (`8d8e6b32ad55c`) — the `InputStateTracker` half (drop
    `MouseMoved` until a real `MouseEntered`) is **already in the target**
    (`TARGET macos/MacOsInputStateTracker.kt:149-155`, with the same explanation). The other half was a
    `HitPathTracker` change inside Noria's fork of compose-ui (an OS-reported Exit now forces every
    tracked hit path to Exit, regardless of geometry) — that cannot be ported and is presumably still a
    live bug in upstream Compose. Worth filing upstream rather than working around.
14. **Frame-domain / DataSource / `reconcile` machinery**, `RenderLoopRegistry`, `resetForReuse`,
    `noWindowAnimationCoroutineContext`, tracing spans — Noria rendering architecture.
    (`AIR-6084`, `AIR-6157`, `AIR-5450`, `AIR-5645` all live here.) The one idea worth extracting is
    finding 2.
15. **`AIR-2231` rounded window corners on Tahoe** (`ec324383ec584`) — the target already passes
    `largeCornerRadius = decoration.roundedCorners` (`TARGET macos/MacOsWindow.kt:425-428`).
16. **`AIR-6611` activate-ignoring-others on bring-to-front** (`faa6b768c998f`) and
    **`AIR-5862` drag-image even-pixel rounding** (`071d54e4fc489`, `610479a34d711`) — both already in
    the target (`TARGET macos/MacOsWindow.kt:383-387` and `:652, 783-790`).

---

# Where the target is already ahead (do not regress these)

- `DisplayLinkFramePump` as a separately testable state machine with a documented single invariant, and
  a CAS-based `retry(token)` that cannot steal a concurrent repaint's latch
  (`TARGET macos/DisplayLinkFramePump.kt`) — Noria inlines the same logic into `setupDisplayLink` with
  an `AtomicReference<TimeMarkWrapper?>` doing double duty as latch and clock.
- `Event.WindowResize` also refreshes `density`, `scene.density` and `scene.size`
  (`TARGET macos/MacOsWindow.kt:541-551`); Noria only refreshes those on `WindowScreenChange`.
- `lastMouseOffset` updated from `MouseDown`/`MouseDragged` as well as `MouseMoved`
  (`TARGET macos/MacOsWindow.kt:580-593`) — Noria only from `MouseMoved`
  (`SOURCE macos/MacOsWindow.kt:1073-1076`), so a drag session started without a preceding move
  anchors at a stale point.
- GPU render thread correctness (see "not worth porting" #6).
- Key-event routing order around the IME is explicit and documented
  (`TARGET macos/MacOsWindow.kt:599-635`), and the `didEditText` interpretation-tracking approach is
  cleaner than Noria's `isIMENavigationEvent` keycode allow-list
  (`SOURCE macos/MacOsTextInputSessionOwner.kt:131-160`).
- `Window.close()`-race guard via `textInput.onWindowClosing()` (`TARGET macos/MacOsTextInput.kt:41-45, 81-84`)
  — Noria has no equivalent.
