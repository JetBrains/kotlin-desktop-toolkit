# Windows backend: Noria → KDT-Compose comparison

Read-only comparison of the Windows backends of

- **TARGET** `/Users/wojtek/IdeaProjects/kotlin-desktop-toolkit/kotlin-desktop-toolkit-compose/src/main/kotlin/org/jetbrains/desktop/compose/windows/`
- **SOURCE** `/Users/wojtek/IdeaProjects/ultimate2/fleet/noria/ui/srcJvmMain/androidx/compose/ui/desktop/windows/`

All files on both sides were read in full, plus the shared-layer files they reference
(`Application.kt`, `Window.kt`, `KdtDispatcher.kt`, `KdtDragAndDropManager.kt`,
`KdtInputModeManager.kt`, `DragAndDropImage.kt` on the target;
`KdtDragAndDropManager.kt`, `FrameDispatcher.kt` on the source) and the KDT win32 API
(`kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/win32/`).

## Headline

The target is **already ahead of Noria on most of the listed tickets**. Verified as
*already present in the target*, contrary to the initial hypothesis:

| Ticket | Noria fix | Target status |
|---|---|---|
| AIR-6366 | frames from a paced per-scene loop, `WM_PAINT` demoted to a damage hint | Present — `WindowsFramePump.kt` is a line-for-line equivalent of Noria's `FrameDispatcher` + `FramePacer`, per window instead of per scene |
| AIR-6023 | mouse forward/back → `Key.Button1..Button5` | Present — `WindowsKeys.kt:178-185` |
| AIR-5621 | Shift+Scroll → horizontal | Present — `WindowsInputStateTracker.kt:398-401` |
| AIR-6051 | catch `DataTransferException` in DnD | Present — `WindowsDragAndDropManager.kt:203-215`, and *also* in `WindowsClipboard.readForFormat` (target catches in one more place than Noria) |
| AIR-6083 | drag image via `DragImage(png, hotspot)` | Present — `WindowsWindow.kt:811-814` |
| AIR-4518 | full IMM32 `TextInputClient` | Present — `WindowsTextInput.kt`, and the target additionally calls `notifySelectionChanged()` after *every* edit, which Noria only does on unmark/discard |
| AIR-5776 | translate every unhandled system key (down **and** up) | Present — `WindowsWindow.kt:740-749`, `CharacterReceived` also filters `isSystemKey` (`WindowsTextInput.kt:128`) |
| AIR-5859 | present the frame even when drawing throws | Present — `WindowsGpuContext.kt:76-84` |
| AIR-5128/5316 | skip `NCCalcSize` while minimizing | Present — `WindowsWindow.kt:623` |
| AIR-6291 | pointer icon stuck after cursor leaves | Windows half present — `sendSyntheticPointerMovedOutsideWindow` (`WindowsInputStateTracker.kt:300-316`). The commit itself (8d8e6b32ad55c) is a `HitPathTracker` + macOS fix and explicitly says "Windows and Linux/GTK are left as-is" |
| AIR-5280 | pointer-icon service survives window reuse | N/A (no reuse), and the *underlying* leak is fixed differently and correctly by `pointerIcons.onWindowClosing()` (`WindowsPointerIcon.kt:56-58`, `WindowsWindow.kt:891`) |
| AIR-6111 | window cleanup on the dispatch thread | Structurally guaranteed: `runApplication` runs the whole composition on `KdtDispatcher`, i.e. the event-loop/STA thread, so `Window`'s `onDispose { window.dispose() }` already runs there |

What follows are the divergences that *are* real.

---

## Ranked findings

### 1. Scrolling does not take the UI out of keyboard input mode, so the cursor stays hidden — P1

**Noria.** Both wheel branches end with an explicit input-mode restore, and only then
re-hit-test:

`SOURCE InputStateTracker.kt:251-262` (X) and `:295-306` (Y)

```kotlin
when {
    processResult.anyScrollingConsumed -> {
        sendPointerInputEventWithCurrentStateIfNecessary(PointerEventType.Move)
        Win32EventHandlerResult.Stop
    }
    inputModeManager.inputMode == InputMode.Keyboard -> {
        inputModeManager.requestInputMode(InputMode.Touch)
        sendPointerInputEventWithCurrentStateIfNecessary(PointerEventType.Move)
        Win32EventHandlerResult.Continue
    }
    else -> Win32EventHandlerResult.Continue
}
```

**Target.** `startPointerEvent()` — the only place that requests `InputMode.Touch` —
is called from `PointerDown/Up/Updated/Entered/Exited`
(`TARGET WindowsInputStateTracker.kt:100, 121, 141, 148, 155`) but **not** from the
scroll branches, which set `lastEventTimeMillis` by hand instead:

`TARGET WindowsInputStateTracker.kt:167-182`

```kotlin
is Event.ScrollWheelX -> {
    lastEventTimeMillis = event.timestamp.toDuration().inWholeMilliseconds
    keyboardModifiers = currentKeyboardModifiers()
    movePointerTo(...)
    sendScroll(...)
    EventHandlerResult.Stop
}
```

**Origin.** The Windows scroll input-mode branch predates the package rename
(`62c30770b8e6d [air] Rename androidx.compose.ui.kdt into androidx.compose.ui.desktop`);
it is the same shape the macOS/GTK trackers use. The mechanism it protects is
AIR-5280-adjacent: `KdtInputModeManager` → `pointerIcons.setHiddenUntilPointerMoves(true)`
→ process-wide `Cursor.hide()`.

**Why it matters.** Tab through a dialog (Compose requests `InputMode.Keyboard`, the
cursor is hidden by `WindowsPointerIcon.kt:41-53`), then spin the wheel without moving the
mouse. Noria brings the cursor back; the target leaves it invisible until the pointer
physically moves. Focus rings also stay in keyboard styling while the user is clearly
using the mouse.

**How to port.** Call `startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)`
at the top of both `Event.ScrollWheelX` and `Event.ScrollWheelY` in
`WindowsInputStateTracker.handleEvent`, replacing the bare
`lastEventTimeMillis = …` assignment. That is a one-line change per branch and reuses the
existing helper. (The target already sends the follow-up `Move` unconditionally from
`sendScroll`, `WindowsInputStateTracker.kt:269`, so nothing else is needed.)

**Confidence** high. **Risk** very low.

---

### 2. No caption-button hit testing — Windows 11 Snap Layouts and native min/max/close are lost — P1

**Noria.** A Compose-drawn title bar reports each caption button's rect, and the hit test
returns the matching `HT*` code so Windows takes over:

`SOURCE WindowsWindow.kt:984-1017`

```kotlin
@Volatile
private var captionButtonBounds: Map<CaptionButtonKind, DpRect> = emptyMap()

@Synchronized
override fun setCaptionButtonBounds(kind: CaptionButtonKind, bounds: DpRect?) { … }

// in hitTestTitleBar:
captionButtonBounds.forEach { (kind, r) ->
    if (clientPoint.x in r.left.value..r.right.value && clientPoint.y in r.top.value..r.bottom.value) {
        return when (kind) {
            CaptionButtonKind.Minimize -> NCHitTestResult.MinButton
            CaptionButtonKind.Maximize -> NCHitTestResult.MaxButton
            CaptionButtonKind.Close    -> NCHitTestResult.Close
        }
    }
}
```

**Target.** The only escape hatch is `setCustomTitleBarInteractiveAreas`, which maps every
claimed rect to *client* (return `null` → toolkit default = `HTCLIENT`):

`TARGET WindowsWindow.kt:210-214` and `:759-774`

```kotlin
if (customTitleBarInteractiveAreas.any { area -> clientPoint in area }) {
    return null            // HTCLIENT — the content handles the press itself
}
return NCHitTestResult.Caption
```

**Origin.** `a644825176abe AIR-5600 [air, windows] caption buttons` — the same commit
that first wired `NCHitTest` up at all, and that fixed `requestPlacement` (`restore()` /
`maximize()`, which the target already has) and the bogus `0.dp to 140.dp` custom-title-bar
inset.

**Why it matters.** On Windows 11, hovering the maximize button must return
`HTMAXBUTTON` for the OS to show the Snap Layouts flyout — there is no other way to opt
in. A Compose-drawn maximize button therefore never shows it. Secondarily, returning the
real `HT*` codes gets the system hover highlight, the correct press-and-cancel semantics,
and Close-button behaviour for free instead of reimplementing them in Compose.

**How to port.** `NCHitTestResult.MinButton/MaxButton/Close` all exist in the target's KDT
copy (`kotlin-desktop-toolkit/.../win32/Event.kt:225-242`). Add a Windows-flavoured
sibling to the existing title-bar API on `ComposeWindow`, e.g.

```kotlin
enum class CaptionButtonKind { Minimize, Maximize, Close }
fun setCaptionButtonAreas(areas: Map<CaptionButtonKind, DpRect>) = Unit  // default no-op
```

and check it in `hitTestTitleBar` *before* the `customTitleBarInteractiveAreas` check.
Keep `setCustomTitleBarInteractiveAreas` for non-caption content (a menu button in the
bar), which genuinely wants `HTCLIENT`. Note the threading comment Noria carries: the map
is written from the composition and read from the WndProc — in the target both are the
same thread, so a plain field is enough and the `@Volatile`/`@Synchronized` can be dropped.

**Confidence** high on the mechanism; medium on the exact API shape to expose (this is a
public-API decision for the Compose layer). **Risk** low — additive.

---

### 3. `quit()` with no open windows is a silent no-op — P1

**Noria.** The zero-window case still terminates:

`SOURCE WindowsApplication.kt:370-403`

```kotlin
private fun requestStructuredQuit() {
    if (terminationInProgress || structuredQuitInProgress) return
    val windowsToClose = windows.values.toList()
    if (windowsToClose.isNotEmpty()) {
        structuredQuitInProgress = true
        windowsToClose.forEach { it.requestClose(WindowCloseRequestReason.ApplicationQuit) }
        return
    }
    if (reusableNativeWindowResources.isNotEmpty()) { structuredQuitInProgress = true; return }
    continueTerminationAsync()      // ← nothing left to close: terminate now
}
```

**Target.** `TARGET WindowsApplication.kt:213-229`

```kotlin
val openWindows = windowList.toList()
if (openWindows.isEmpty()) {
    return                          // ← quit() does nothing at all
}
```

**Origin.** The `requestStructuredQuit` / `finishStructuredQuitIfNeeded` /
`continueTerminationAsync` triad predates the rename (`62c30770b8e6d`); the re-entrancy
guards on top of it are `f73057d4df4dc AIR-6896` (see finding 5).

**Why it matters.** Windows is the only target backend where `quit()` can do nothing:
macOS calls `Application.requestTermination()`
(`TARGET macos/MacOsApplication.kt`), GTK posts `onQuitRequested()`
(`TARGET gtk/GtkApplication.kt`). An application whose windows are all closed but whose
composition is still alive (a tray-only or menu-bar-only state, or a transient window-less
moment) cannot be quit at all on Windows.

A second, related sticky-state bug: `isQuitInProgress` (`:71-72`, set at `:226`) is never
cleared. If the application's `onCloseRequest` declines to remove the window from the
composition, every later `quit()` returns at `:214` forever. Noria clears
`structuredQuitInProgress` in both `stopAndJoin` (`:284`) and `continueTerminationAsync`
(`:395`).

**How to port.** Two changes in `WindowsApplication`:

1. Give the application a way to end the composition. The cleanest fit for the target's
   architecture is to route the empty case to the same place `ApplicationScope.exitApplication`
   goes — e.g. hand `WindowsApplication` an `onTerminate: () -> Unit` from
   `initApplication`/`ApplicationScopeImpl`, and call it when `openWindows.isEmpty()`.
   A cheaper stop-gap: `Logger.warn` instead of a silent `return`, so the case is at least
   visible.
2. Clear `isQuitInProgress` once no window is left (a `detachWindow` hook mirroring
   Noria's `finishStructuredQuitIfNeeded`, `SOURCE WindowsApplication.kt:388-391`,
   called from `WindowsWindow.dispose` at `SOURCE WindowsWindow.kt:615-617`).

**Confidence** high on the divergence; medium on the right adaptation, because the target
deliberately has no application-level termination concept on Windows. **Risk** medium —
touches the quit path.

---

### 4. Clipboard operations are not serialized against each other — P1

**Noria.** Every clipboard access holds a `Mutex` across the whole `set` + `flush` pair:

`SOURCE WindowsClipboard.kt:66`, `:78-90`, `:254`

```kotlin
private val clipboardMutex = Mutex()
…
override suspend fun getClipEntry(): ClipEntry = withContext(Dispatchers.Main.immediate) {
    val dataObject = clipboardMutex.withLock { Win32Clipboard.withRetry { get() } }
    ClipEntry(WindowsClipboardEntry(dataObject))
}
…
clipboardMutex.withLock {
    Win32Clipboard.withRetry { set(data) }
    Win32Clipboard.withRetry { flush() }
}
```

**Target.** No lock; `publish` runs the two retries back to back:

`TARGET WindowsClipboard.kt:62-65`

```kotlin
private suspend fun publish(dataObject: DataObject) {
    withBusyRetry { Win32Clipboard.set(dataObject) }
    withBusyRetry { Win32Clipboard.flush() }
}
```

**Origin.** `79a633bee12d2 AIR-5558: Use new KDT clipboard in combination with async retries` —
the commit that introduced both the async retry ladder and the mutex.

**Why it matters.** Both sides are single-threaded (the OLE STA), but
`withBusyRetry` **suspends** (`delay(retryDelay)`, `TARGET WindowsClipboard.kt:176`) and
therefore yields the event-loop thread. Interleaving is real: coroutine A calls
`set(objectA)`, gets `Busy`, delays; coroutine B calls `set(objectB)` and succeeds; A
resumes and calls `flush()`, rendering **B's** data. Every window in the target shares one
`application.clipboard` (`TARGET WindowsApplication.kt:112`, provided as `LocalClipboard`
at `:253` and `WindowsWindow.kt:583`), so two windows racing a copy is enough.

**How to port.** Add `private val clipboardMutex = Mutex()` to `WindowsClipboard` and wrap
the body of `getClipEntry` and of `publish` in `clipboardMutex.withLock { … }`. Straight
copy; `kotlinx.coroutines.sync.Mutex` is already on the classpath.

**Confidence** high. **Risk** very low. (Do *not* copy Noria's `getClipEntrySync` /
`tryLock` path — Compose's `Clipboard` interface is suspend-only, see "Not worth porting".)

---

### 5. Application teardown has no "exactly once" guard — P2

**Noria.** Two independent guards:

`SOURCE WindowsApplication.kt:280-298`

```kotlin
override suspend fun stopAndJoin() {
    if (terminationInProgress) {
        return                                    // AIR-6896
    }
    structuredQuitInProgress = false
    terminationInProgress = true
    try { resetState() } finally {
        // stopAndJoin() is reachable from several paths (structured quit, JVM shutdown hook, dock
        // cleanup); make sure the native event loop is stopped exactly once.
        if (eventLoopStopped.compareAndSet(expectedValue = false, newValue = true)) {
            nativeApplication.stopEventLoop()     // AIR-5478
        }
        eventLoopThread?.join()
        removeApplication(this)
        shutdown = true
    }
}
```

and `current()` was split so the two failure modes report distinctly
(`SOURCE WindowsApplication.kt:139-143`, also AIR-6896).

**Target.** `TARGET WindowsApplication.kt:260-267`

```kotlin
override fun close() {
    nativeApplication.invokeOnDispatcher { nativeApplication.stopEventLoop() }
    eventLoopThread.join()
    forgetApplication(this)
}
```

**Origin.**
- `ec1a8f6c3b69d AIR-5478 [air] mitigate the issue but not calling the stopEventLoop for the second time`
- `f73057d4df4dc AIR-6896 [air, Windows] Prevent WindowsApplication from terminating more than once`

**Why it matters.** `ComposeApplication` is `AutoCloseable` and public: `application { }`
closes it via `use`, but a caller who used `initApplication` directly, or who wraps it in
their own `use` *and* calls `close()`, gets a second `stopEventLoop()` on an already-torn-down
native application. `Application.stopEventLoop()` calls `appPtr`, which is
`checkNotNull`-ed in the KDT layer — the second call runs `invokeOnDispatcher` on a stopped
loop, so the block is enqueued and never runs, and `forgetApplication` is executed twice.
It is not currently reachable through `application { }`, which is why this is P2 rather
than P1.

**How to port.** Add an `AtomicBoolean isClosed` (or reuse the pattern verbatim) and make
`close()` return early on the second call. Note the same shape exists in
`TARGET macos/MacOsApplication.kt`, so this is worth fixing in both if you take it.

**Confidence** high. **Risk** very low.

---

### 6. Clipboard failures crash with `IllegalStateException` instead of a typed, diagnosable exception; shorter retry ladder — P2

**Noria.** `SOURCE WindowsClipboard.kt:260-283`, `:312-327`

```kotlin
private suspend fun <T> Win32Clipboard.withRetry(operation: …): T {
    lateinit var latestFailure: ClipboardResult.Failure
    for (retryDelay in clipboardRetryDelays) {
        when (val result = operation()) {
            is ClipboardResult.Success<T> -> return result.value
            is ClipboardResult.Failure if result.isBusy -> { latestFailure = result; delay(retryDelay) }
            is ClipboardResult.Failure -> throw ClipboardException(result)
        }
    }
    throw ClipboardException(latestFailure)
}

private val clipboardRetryDelays = listOf(10, 25, 50, 100, 200, 400, 800).map { it.milliseconds }

class ClipboardException(failure: ClipboardResult.Failure) : RuntimeException(failure.clipboardExceptionMessage()) {
    val status: DataTransferStatus = failure.status
    val nativeCode: Int = failure.nativeCode
    val nativeMessage: String? = failure.message
}
```

**Target.** `TARGET WindowsClipboard.kt:167-188`

```kotlin
is ClipboardResult.Failure -> {
    check(result.isBusy) { "Clipboard operation failed: ${result.status} (0x${result.nativeCode.toHexString()})" }
    …
}
…
error("Clipboard stayed busy, last failure: $lastFailure")

private val retryDelays = listOf(10, 25, 50, 100, 200).map { it.milliseconds }   // 385 ms total
```

**Origin.** `79a633bee12d2 AIR-5558`, extended in later touches of
`SOURCE WindowsClipboard.kt`.

**Why it matters.** Two things. (a) Using `check()` for a *runtime* clipboard failure
means a genuine OLE error surfaces as `IllegalStateException`, which no caller can catch
selectively and which reads as a programming bug in stack traces; the `HRESULT` is only in
the message string. (b) The target's ladder gives up after 385 ms of contention, Noria's
after 1585 ms. Clipboard monitors (password managers, clipboard history tools) routinely
hold the clipboard for hundreds of milliseconds; 385 ms is realistically hittable, and
the failure mode is a hard throw.

**How to port.** Introduce a `WindowsClipboardException(status, nativeCode, message)`
(or a backend-neutral `ClipboardException` in the shared package, since GTK/Wayland have
the same problem), throw it from `withBusyRetry` for the non-busy case and for exhaustion,
and extend `retryDelays` with `400.milliseconds, 800.milliseconds`. Consider whether the
Compose layer wants to swallow it at the `Clipboard` boundary instead of throwing —
Compose's `Clipboard.getClipEntry()` has no error channel.

**Confidence** high. **Risk** low.

---

### 7. `WindowStyle` creation flags Noria uses and the target ignores — P2

**Noria.** `SOURCE WindowsWindow.kt:203-214`

```kotlin
val windowParams = WindowParams(
    …
    style = WindowStyle(
        isResizable = true,
        isMinimizable = true,
        isMaximizable = true,
        titleBarKind = WindowTitleBarKind.Custom,
        systemBackdropType = WindowSystemBackdropType.Mica,
    ),
)
```

**Target.** `TARGET WindowsWindow.kt:355-368` sets only `isResizable` and `titleBarKind`.

**KDT API check.** All three exist in the target's own KDT copy:
`kotlin-desktop-toolkit/.../win32/Window.kt:29-47` (`isMinimizable`, `isMaximizable`,
`systemBackdropType`) and `:308-321`
(`WindowSystemBackdropType.Auto/None/Mica/DesktopAcrylic/MicaAlt`). There are also
read-backs `Window.isMinimizable()` / `isMaximizable()` at `:201-215`.

**Why it matters.** These are creation-time-only style bits (`WS_MINIMIZEBOX` /
`WS_MAXIMIZEBOX` / `DWMWA_SYSTEMBACKDROP_TYPE`), so they cannot be retro-fitted after
`create()`. `ComposeWindowParams` today only exposes `isResizable`
(`TARGET Window.kt:59-67`); a Compose app that wants a non-maximizable dialog, or the
Mica material behind a custom title bar, has no way to ask.

**How to port.** Add `isMinimizable: Boolean = true` and `isMaximizable: Boolean = true`
to `ComposeWindowParams`, and pass them through in `WindowsWindow.init` (other backends
ignore them). The backdrop is a genuinely Windows-only visual and is best left out of the
cross-platform `ComposeWindowParams`; if wanted, expose it the way
`WindowsPointerIcon` is exposed — a Windows-specific opt-in — rather than in the shared
params. Note `WindowDecoration.CustomTitleBar.roundedCorners` (`TARGET Window.kt:52-55`)
is currently unused by the Windows backend too, which is the natural place a backdrop
option would live.

**Confidence** high. **Risk** low (additive, defaults preserve today's behaviour).

---

### 8. No window icon — P2

**Noria.** `SOURCE WindowsWindow.kt:1116-1120`

```kotlin
override fun setIcon(icon: ByteArray) {
    if (!isDisposed) {
        nativeWindow.setIcon(icon)
    }
}
```
(via the `IconDecoratedWindow` interface it implements, `:163`.)

**Target.** No counterpart anywhere in `ComposeWindow` or `WindowsWindow`.
`Window.setIcon(icon: ByteArray)` exists in the target's KDT at
`kotlin-desktop-toolkit/.../win32/Window.kt:172`.

**Does Compose need it?** Yes — Compose Desktop's `Window(icon: Painter?)` is part of the
API this layer mirrors, and on Windows the icon is what shows in the taskbar and Alt-Tab.
Without it every window gets the generic Java icon.

**How to port.** Add `fun requestIcon(icon: Painter?)` (or an `icon` in
`ComposeWindowParams`) to `ComposeWindow`, render the painter to PNG bytes with the
existing `DragAndDropImage`-style `CanvasDrawScope` + `Image.encodeToData` helper
(`TARGET DragAndDropImage.kt:27-40` is almost exactly the code needed), and call
`nativeWindow.setIcon`. Other backends can no-op or use their own path.

**Confidence** high that the capability is missing; medium on whether the current milestone
wants it. **Risk** low.

---

### 9. IME session teardown does not finish the composition explicitly — P2

**Noria.** `SOURCE WindowsTextInputSessionOwner.kt:54-60`

```kotlin
try {
    awaitCancellation()
} finally {
    currentTextInputClient?.unmarkText()     // finish the composing text first
    nativeWindow.clearTextInputClient()
    currentTextInputClient = null
}
```

**Target.** `TARGET WindowsTextInput.kt:65-79`

```kotlin
withContext(NonCancellable + KdtImmediateDispatcher) {
    activeClient = null
    if (!isWindowGone) {
        // Clearing the client finalizes an open composition, which calls back into it — hence
        // this order, and hence the toolkit swapping the recipient only afterwards.
        nativeWindow.clearTextInputClient()
        nativeWindow.notifySelectionChanged()
    }
}
```

Note the ordering hazard: the target sets `activeClient = null` **before**
`clearTextInputClient()`. If the toolkit's finalize-on-clear does call back into the
installed client, that callback lands on the still-registered *native* client while the
Kotlin-side `activeClient` is already `null` — `tryHandleTextInputEvent` and
`offerKeyDownToInputMethod` would both refuse anything arriving in that window.

**Origin.** `c7df8548603ee [air, Windows] AIR-4518 Support IME on Windows` — the same
commit that introduced the whole `TextInputClient` bridge.

**Why it matters.** Focus leaving a text field mid-composition (CJK candidate open, or a
pending dead key) must commit or discard the composing text. Whether
`clearTextInputClient()` alone does that is a KDT contract question — the target's comment
asserts it does, Noria does not rely on it. Getting it wrong leaves a phantom composition.

**How to port.** Low-cost belt-and-braces: call `activeClient?.unmarkText()` (or
`discardMarkedText()`, depending on the desired semantics — `unmarkText` commits,
`discardMarkedText` abandons) *before* `activeClient = null`, and move
`activeClient = null` to after `clearTextInputClient()`. Verify against the Rust side of
`window_clear_text_input_client` before changing behaviour.

**Confidence** medium — this depends on a KDT contract I did not verify in the Rust layer.
**Risk** low if you keep the existing `clearTextInputClient()` call.

---

### 10. No dispatcher-thread assertions on clipboard / data-object access — P3

**Noria.** `SOURCE WindowsClipboard.kt:41-45`, called from `getClipEntrySync` (`:49`),
`getForFormatSync` (`:154`) and `close` (`:159`):

```kotlin
private fun checkOnDispatcherThread() {
    check(currentWindowsNativeApplication().isDispatcherThread()) {
        "Windows clipboard access must run on the application dispatcher thread (OLE STA)"
    }
}
```

**Target.** `TARGET WindowsClipboard.kt:90` — `getForFormatSync` reaches straight into
`dataObject.readForFormat(format)` with no check, and it is a public-ish entry point on
`ClipboardEntry`.

**Why it matters.** Calling an `IDataObject` method off the STA is undefined behaviour
that usually manifests as `RPC_E_WRONG_THREAD` far from the cause. Cheap assertion,
big diagnostic win — the same reasoning behind `86ce30a1fb8a1 AIR-6111 Execute cleanup of
Windows application on dispatch thread`, whose comment names exactly this HRESULT.

**How to port.** Add a `checkOnDispatcherThread()` private helper in
`WindowsClipboard.kt` reading `application.isUiThread()` and call it from
`WindowsClipboardEntry.getForFormatSync` and `close`. Consider gating it behind an assert
flag if you do not want the cost in production.

**Confidence** high. **Risk** none, unless a caller is currently violating it — which is
precisely what you'd want to find out.

---

### 11. `changed()` does not request a redraw after an IME edit — P3

**Noria.** `SOURCE WindowsTextInputSessionOwner.kt:196-199`

```kotlin
private fun changed() {
    nativeWindow.notifySelectionChanged()
    nativeWindow.requestRedraw()
}
```

**Target.** `TARGET WindowsTextInput.kt:202-204` calls only `notifySelectionChanged()`.

**Why it matters.** In Noria the IME edit path bypasses the normal invalidation, so the
explicit `requestRedraw()` is needed. In the target it very likely is not: `request.editText { … }`
writes Compose state → the scene's `invalidate` → `framePump.scheduleFrame()`
(`TARGET WindowsWindow.kt:298`). Recorded for completeness; **I judge this one not worth
porting** unless composition-while-typing is observed to lag by a frame.

**Confidence** medium. **Risk** n/a.

---

### 12. A pointer press is always *also* emitted as a key event, even when the press was consumed — P3

**Noria.** `SOURCE InputStateTracker.kt:93-107` only sends the synthetic key event if the
pointer press went unconsumed:

```kotlin
when {
    processResult.anyPressOrReleaseConsumed -> Win32EventHandlerResult.Stop
    sendKeyEvent(KeyEvent(key = event.button.toKey(), …)) -> Win32EventHandlerResult.Stop
    else -> Win32EventHandlerResult.Continue
}
```

**Target.** `TARGET WindowsInputStateTracker.kt:105-117` sends both unconditionally.

**Why it matters / why it is P3.** The target *cannot* do better:
`ComposeScene.sendPointerEvent` returns `Unit` in released Compose, and
`PointerEventResult` is `internal` to compose-ui — which the target documents at
`WindowsInputStateTracker.kt:59-65`. Practical impact is small (Compose has no default
`Key.Button1..5` handlers), but an app that binds both a pointer handler and a
`Key.Button4` shortcut will see both fire on one press.

**How to port.** Nothing actionable today. Revisit if Compose exposes the pointer result;
until then, consider only emitting the key events for the *side* buttons
(`XButton1`/`XButton2`), which is the case the feature exists for
(`2be5f2cba08cd AIR-6023 fix: Mouse forward and back buttons do nothing on Windows`), and
skipping `Left`/`Right`/`Middle` where the double dispatch is most likely to surprise.

**Confidence** high on the mechanism. **Risk** low.

---

### 13. Key events are withheld from Compose while a text field holds the input session — P3 (design question)

**Noria.** The IME is offered the key from *inside* the text-input plumbing, after the
focus dispatch has reached the field: `handleEventWithInputSession(keyEvent)`
(`SOURCE WindowsTextInputSessionOwner.kt:108-119`) takes a Compose `KeyEvent` and digs the
native one out of it.

**Target.** `TARGET WindowsWindow.kt:732-738` offers the key to the IME **before** anything
Compose-side sees it:

```kotlin
private fun handleKeyDown(event: Event.KeyDown): EventHandlerResult {
    if (textInput.offerKeyDownToInputMethod(event)) {
        return EventHandlerResult.Stop
    }
    val result = inputStateTracker.handleEvent(event, density)
    …
}
```

**Consequence.** While a `BasicTextField` holds the session, a plain printable key never
reaches `onPreviewKeyEvent` / `scene.sendKeyEvent` / `onKeyEvent`. Window-level preview
handlers (a single-letter accelerator, a "type to search" list) go dark for exactly the
keys they care about.

**Assessment.** This is a *coherent* design, not an accident — routing the key to Compose
*and* letting `WM_CHAR` insert via the IME would double-insert, since the target already
puts `toUnicode()` into `KeyEvent.codePoint` (`WindowsInputStateTracker.kt:191`). Noria's
inversion works only because Noria's editor owns the `handleEventWithInputSession` hook.
Compose has no equivalent hook. **Flagging it, not recommending a port** — if it becomes a
problem, the fix is to run `onPreviewKeyEvent` (only) before
`offerKeyDownToInputMethod`, which is a two-line reorder and preserves the no-double-insert
property.

**Confidence** high on the behaviour, low on whether it is worth changing. **Risk** medium
if changed.

---

## Noria files / capabilities with no target counterpart

| Noria | Target | Needed by Compose? |
|---|---|---|
| `WindowIdsMapping.kt` (`LightweightWindowId` ↔ `WindowId`) | none — target keys `windowsById` on the raw `WindowId` (`WindowsApplication.kt:66`) | **No.** Out of scope (window reuse). |
| `Dispatchers.kt` (`WindowsKdtMainDispatcher`) | `KdtDispatcher` / `KdtImmediateDispatcher` (shared, backend-agnostic) | **No.** Target's is strictly better — one dispatcher for all four backends. |
| `ViewConfiguration.kt` | inlined at `WindowsWindow.kt:218-223`, identical constants | **No.** |
| `WindowsUriHandler.kt` | inlined at `WindowsApplication.kt:106-111`, identical | **No.** |
| `Converters.kt`: `roundToIntSize`, `Size.toLogicalSize`, `LogicalPoint.plus/minus` | absent | **No.** Dead weight in Noria too — nothing in its `windows/` package uses them. |
| `setIcon` / `IconDecoratedWindow` | absent | **Yes** — finding 8. |
| `setCaptionButtonBounds` / `CaptionButtonsHostWindow` | partly (`setCustomTitleBarInteractiveAreas`) | **Yes** — finding 2. |
| `getClipEntrySync` | absent | **No.** Compose's `Clipboard` is suspend-only; the target's `ClipboardEntry.getForFormatSync` covers the per-format sync read. |
| `ClipboardFormat.WindowLocalDrag`, `CustomSerializable` | absent | **No.** Noria-framework formats; Compose's DnD has no analogue. |
| `showEmojiAndSymbolsPopup` | absent (Noria's is a `// TODO` anyway) | **No.** |
| `captureScreenshot` | absent (Noria's is `TODO()`) | **No.** |
| `awaitWhenReady()` / `didFinishLaunchingCompletableJob` | `applicationStarted: CountDownLatch` awaited in `init` (`WindowsApplication.kt:268-272`) | **No.** Equivalent; the target's blocking-in-init form is simpler and matches its `application { }` entry point. |
| `resetForReuse` / `prepareNativeWindowResourcesForReuse` / `reuseWindow` / `disposeReusableNativeWindowResources` | absent | **No.** Out of scope. |
| `withPreparedMainThread` wrappers around every scene touch (AIR-6419) | absent | **No.** Out of scope (Noria snapshot read-tracking). |
| `WindowsScreen.bounds` | `ComposeScreen.bounds` default (`ComposeScreen.kt:20`) | **No.** |

---

## Divergences judged NOT worth porting

**Noria is worse; the target already has the better version.** Recorded so a future
sweep does not "restore" them:

1. **`AngleViewContext.close()` leaks the ANGLE renderer.**
   `SOURCE GpuContext.kt:85-88` closes only the `DirectContext`; target also calls
   `angleRenderer.close()` (`TARGET WindowsGpuContext.kt:89-93`).
2. **`currentSize` is committed before the resize can fail.**
   `SOURCE GpuContext.kt:51-54` assigns `currentSize = physicalSize` *then* calls
   `resizeSurface`; a throw leaves the cache claiming a size the surface does not have, so
   every later frame renders at the wrong size. The target assigns after
   (`TARGET WindowsGpuContext.kt:52-57`) and its comment says exactly why.
   `surfaceParams!!` (`SOURCE GpuContext.kt:60`) is likewise an NPE the target avoids with
   `?: return` (`:58`).
3. **`NCCalcSize` sets `size` (whole window) to the client logical size.**
   `SOURCE WindowsWindow.kt:868-873` writes both `contentSize` and `size` from
   `event.size / event.scale`. The target reads the frame rect back
   (`TARGET WindowsWindow.kt:625-626`).
4. **No `Event.WindowResize` handling at all** in Noria; the target handles it
   (`TARGET WindowsWindow.kt:651-659`).
5. **`requestMinimized(false)` replays a stored rect** with `windowRectBeforeMinimization!!`
   (`SOURCE WindowsWindow.kt:413-426`) — an NPE if restore is called without a prior
   minimize, and it also loses a maximized state. Target uses `nativeWindow.restore()`
   (`TARGET WindowsWindow.kt:489-492`).
6. **Screens are read once at startup and never refreshed**
   (`SOURCE WindowsApplication.kt:174-176`); the target re-reads on
   `WindowScaleChanged` and filters `isAttached`
   (`TARGET WindowsApplication.kt:154-156, 169`).
7. **The application-level event handler has no `try/catch`**
   (`SOURCE WindowsApplication.kt:180-192`); the target wraps it
   (`TARGET WindowsApplication.kt:140-167`), so a throw in `SystemAppearanceChange`
   handling cannot take the loop down.
8. **`nativeApplication.close()` is never called** on the Noria side; the target closes it
   in the loop thread's `finally` (`TARGET WindowsApplication.kt:132-136`).
9. **`dragDropManager.close()` is never called**; Noria only revokes the drop target
   (`SOURCE WindowsWindow.kt:607`). Target does both plus `scene.close()`,
   `framePump.cancel()`, `frameScope.cancel()`, `textInput.onWindowClosing()`,
   `pointerIcons.onWindowClosing()` (`TARGET WindowsWindow.kt:884-901`).
10. **`DataTransferException` is caught in fewer places.** Noria catches in
    `WindowsDragAndDropClipboardEntry.getForFormatSync` (`SOURCE WindowsDragAndDropManager.kt:42-47`)
    but **not** in `WindowsClipboard.readForFormat` (`SOURCE WindowsClipboard.kt:182-219`) —
    a clipboard read of a hostile format still throws. Target catches in the shared
    `DataObject.readForFormat` (`TARGET WindowsClipboard.kt:135-148`), covering both.
    (Historically the target's coverage matches `e3d7419b6ff90 [air] WindowsClipboard:
    catch read failures in getForFormatSync, treat as format-absent`, which Noria later
    lost in the AIR-5558 rewrite.)
11. **`VirtualKey.Clear -> Key.NumLock`** (`SOURCE Key.kt:66`). `VK_CLEAR` (0x0C) is
    Numpad-5-with-NumLock-off, not NumLock (which is `VK_NUMLOCK`, already mapped at
    `:139`). The target's `Key.Unknown` is closer to correct. Also note
    `VirtualKey.Oem1` and `VirtualKey.OemSemicolon` are the *same value* (186) in the
    target's KDT (`win32/Keyboard.kt:206-207`), so the target's single `Oem1` branch
    (`TARGET WindowsKeys.kt:65`) already covers both — Noria's extra alternative is
    redundant.
12. **`IME insertText`/`setMarkedText` do not notify the IME of the caret move.**
    Noria only calls `changed()` from `unmarkText`/`discardMarkedText`
    (`SOURCE WindowsTextInputSessionOwner.kt:164-194`); the target calls it after every
    edit (`TARGET WindowsTextInput.kt:164-194`), which keeps the candidate window
    following the caret.
13. **Application-wide `isActive` hard-coded to `true`** (`SOURCE WindowsApplication.kt:245`);
    the target derives it from window focus (`TARGET WindowsApplication.kt:102`).
14. **Noria has no `onWindowFirstShown` / `forceFocus` on the first window**
    (`TARGET WindowsApplication.kt:196-201`) — a real target-side improvement for
    terminal- and IDE-launched processes.

**Noria-specific, no Compose analogue.** `withPreparedMainThread` / `withPreparedMainThread`-wrapped
scene calls, `DrainableUpdateQueue`, `FrameInvalidationCallbacks`/`FrameCompletionCallbacks`,
`RenderPerfMetrics`, `noria.reconcile()`, `LayoutOwner`/`LayoutNode` plumbing,
`NoriaPointerInputEventProcessor`/`PositionCalculator`, `SnapshotMutableStateImpl` +
`setValueAndScheduleDependantsRightAway`, `Resource`/`withSupervisor`, `fleet.reporting`
spans, `DefaultTextToolbar`/`FocusOwnerImpl` wiring, the extended `Key` const range.
Deliberately excluded per the brief.

---

## Quick reference: where each finding lands

| # | Priority | File to change |
|---|---|---|
| 1 | P1 | `windows/WindowsInputStateTracker.kt` |
| 2 | P1 | `windows/WindowsWindow.kt` + `Window.kt` (new API) |
| 3 | P1 | `windows/WindowsApplication.kt` (+ `Application.kt`) |
| 4 | P1 | `windows/WindowsClipboard.kt` |
| 5 | P2 | `windows/WindowsApplication.kt` (and `macos/MacOsApplication.kt`) |
| 6 | P2 | `windows/WindowsClipboard.kt` |
| 7 | P2 | `Window.kt` (`ComposeWindowParams`) + `windows/WindowsWindow.kt` |
| 8 | P2 | `Window.kt` (`ComposeWindow`) + `windows/WindowsWindow.kt` |
| 9 | P2 | `windows/WindowsTextInput.kt` |
| 10 | P3 | `windows/WindowsClipboard.kt` |
| 11 | P3 | — (not recommended) |
| 12 | P3 | `windows/WindowsInputStateTracker.kt` |
| 13 | P3 | — (design question) |
