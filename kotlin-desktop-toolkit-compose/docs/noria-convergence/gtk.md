# GTK backend: Noria → KDT-Compose comparison

Read-only comparison of

- **TARGET** `kotlin-desktop-toolkit-compose/src/main/kotlin/org/jetbrains/desktop/compose/` (`gtk/`, plus the
  shared `LinuxMimeTypes.kt`, `DesktopPlatform.kt`, `FileChooserPaths.kt`, `linux/LinuxWindowBorders.kt`)
- **SOURCE** `/Users/wojtek/IdeaProjects/ultimate2/fleet/noria/ui/srcJvmMain/androidx/compose/ui/desktop/`
  (`gtk/`, plus `LinuxInputMethodCommon.kt`, `LinuxPlatform.kt`, `MimeTransfer.jvm.kt`,
  `KdtDragAndDropManager.kt`, `Application.jvm.kt`)

Both were read in full. Where a claim depends on what the toolkit actually emits, it was checked against the
Rust backend in `native/desktop-gtk/src/gtk/`.

**Headline:** the target is in many places *ahead* of Noria (scroll `isStop` handling, cached drag-icon
`DirectContext`, zero-size draw guard, `discardActivePreedit`, non-empty drag actions, refresh-rate coercion,
generational synthetic-move). The findings below are the places where Noria is ahead, ranked.

Four of the six tickets suggested in the brief turned out **not** to touch GTK at all — see
[Tickets that do not apply](#tickets-that-do-not-apply-verified).

---

## Findings

### 1. `ModifiersChanged` must reach every window, and must never be cleared locally — P1

**What Noria does.** `Event.ModifiersChanged` is broadcast to *all* windows, and it is the only writer of
`keyboardModifiers`:

SOURCE `gtk/GtkApplication.kt:345-350`
```kotlin
is Event.ModifiersChanged -> {
    for (window in windows.values) {
        window.handleEvent(event)
    }
    EventHandlerResult.Stop
}
```

SOURCE `gtk/InputStateTracker.kt:280-283` — the tracker only assigns from the event; it has no
`WindowKeyboardEnter`/`WindowKeyboardLeave` branches at all (they were deliberately deleted, see the commit
below), and `Event.KeyDown` no longer re-derives modifiers either.

**What the target does.** It routes the event to at most two windows and clears the state on focus/hover loss:

TARGET `gtk/GtkApplication.kt:263-269`
```kotlin
is Event.ModifiersChanged -> {
    setOfNotNull(mouseFocusWindowId, keyboardFocusWindowId)
        .forEach { windowId -> windowsById[windowId]?.handleEvent(event) }
    EventHandlerResult.Stop
}
```
TARGET `gtk/GtkInputStateTracker.kt:167-169` (clear on `MouseExited` when unfocused) and
`gtk/GtkInputStateTracker.kt:243-251` (clear on `WindowKeyboardLeave` when the pointer is outside).

**Origin.** `cc557a8bf5daa` — *"[air] Improve keyboard modifiers tracking on Linux"*. The commit does exactly
three things: broadcast `ModifiersChanged` to every window, drop `ModifiersChanged` from `windowIdOrNull()`,
and delete the `WindowKeyboardEnter`/`WindowKeyboardLeave` branches that reset `keyboardModifiers` and
`pointerButtons`.

**Why it matters.** In this toolkit `ModifiersChanged` is a **seat-wide** event, not a window event — see
`native/desktop-gtk/src/gtk/application_state.rs:161-172`, where the handler is installed once on
`default_seat.keyboard()`'s `modifier_state_notify` and carries no window id (`Event.ModifiersChanged` in
`kotlin-desktop-toolkit/.../gtk/Event.kt:187` has only `modifiers`). Consequences of the target's approach:

- **Modifiers held across a focus change are lost.** Press and hold Alt while window A has focus, then
  Alt-Tab to window B. The `modifier_state_notify` for "Alt down" fired while A was focused, so B never saw
  it; the state has not *changed* since, so no further event arrives. B's tracker reports no Alt while Alt is
  physically down — Alt-click, Alt-drag and Alt-scroll gestures silently do the unmodified thing.
- **Symmetrically sticky.** A window that is not the mouse/keyboard focus target when a modifier is *released*
  keeps that modifier set until the next change it happens to receive.
- The two local clears are a patch over the routing gap, and they introduce their own hole: `WindowKeyboardLeave`
  clears only `if (!isPointerInWindow)`, and `MouseExited` clears only `if (!hasKeyboardFocus)`, so a window
  that loses both one after the other keeps whatever was held at the first of the two.

**How to port.**
1. In `GtkApplication.handleEvent`, replace the two-window fan-out with
   `windowList.toList().forEach { it.handleEvent(event) }` and return `Stop`. (Iterate a copy: a handler can
   dispose a window.)
2. In `GtkInputStateTracker`, delete the `keyboardModifiers = PointerKeyboardModifiers()` assignments at
   `:167-169` and `:243-251`. Keep the `hasKeyboardFocus` flag only if something else needs it — after this
   change nothing does, so both `WindowKeyboardEnter`/`WindowKeyboardLeave` branches can collapse to
   `EventHandlerResult.Continue` (or be removed, letting the `else` branch handle them).
3. Keep `pointerButtons` clearing on `MouseExited` — that one is about the pointer grab and is correct
   (Noria's final state does the same, `gtk/InputStateTracker.kt:186-187`).

**Confidence / risk.** High confidence on the mechanism (verified in the Rust source). Low risk: the change
removes state rather than adding it. Worth also doing step 4 below (seeding new windows) — see finding 12.

---

### 2. "Active window" and "window with keyboard focus" are two different GTK signals — P1

**What Noria does.** It keeps them apart and ORs them:

SOURCE `gtk/GtkWindow.kt:223-227`
```kotlin
private var hasActiveAppearance: Boolean by mutableStateOf(false)
private var hasKeyboardFocus: Boolean by mutableStateOf(false)

override val isFocused: Boolean
    get() = hasActiveAppearance || hasKeyboardFocus
```
`hasActiveAppearance` is written only from `Event.WindowConfigure` (SOURCE `gtk/GtkWindow.kt:466`),
`hasKeyboardFocus` only from `WindowKeyboardEnter`/`WindowKeyboardLeave` (SOURCE `gtk/GtkWindow.kt:501, 511`).

**What the target does.** One field, written from all three events:

- TARGET `gtk/GtkWindow.kt:165` — `override var isFocused: Boolean by mutableStateOf(false)`
- TARGET `gtk/GtkWindow.kt:607` — `isFocused = event.active` (from `WindowConfigure`)
- TARGET `gtk/GtkWindow.kt:553` — `isFocused = true` (from `WindowKeyboardEnter`)
- TARGET `gtk/GtkWindow.kt:564` — `isFocused = false` (from `WindowKeyboardLeave`)

**Origin.** `369319fadacfe` — *"[air] AIR-5610 AIR-5611 Enable IME on Linux"* introduced the split (it is the
commit that first adds both `hasActiveAppearance` and `hasKeyboardFocus`).

**Why it matters.** The two events come from unrelated GTK signals and are free to disagree, in value and in
order:

- `WindowKeyboardEnter/Leave` ← `window.connect_is_active_notify` (`native/desktop-gtk/src/gtk/window.rs:373-379`)
- `WindowConfigure.active` ← `gdk4::ToplevelState::FOCUSED` on the toplevel
  (`native/desktop-gtk/src/gtk/window.rs:211, 225-231`)

and — this is the important part — `update_window_configure`
(`native/desktop-gtk/src/gtk/window.rs:146-158`) always re-sends the **whole cached** configure struct. So a
configure emitted for an unrelated reason (a resize through `on_allocate`, a maximize, a decoration change)
carries whatever `active` was last cached. If that cached value lags `is_active_notify` — which it does at
window creation, where `WindowConfigureEvent::new` hardcodes `active: false`
(`native/desktop-gtk/src/gtk/window.rs:107`) — a resize will flip `isFocused` to `false` on a window that
does have the keyboard. Compose then drops the caret, the selection highlight and the focus ring mid-drag,
and `ComposeApplication.focusedWindow` reports the wrong window.

**How to port.** Mechanical, and Compose needs no adaptation:

```kotlin
private var hasActiveAppearance: Boolean by mutableStateOf(false)
private var hasKeyboardFocus: Boolean by mutableStateOf(false)
override val isFocused: Boolean get() = hasActiveAppearance || hasKeyboardFocus
```
then write `hasActiveAppearance = event.active` in `handleConfigure` and `hasKeyboardFocus = true/false` in
the two keyboard branches. `windowInfo.isWindowFocused` and `focusOwner` already read through `isFocused`, so
nothing else changes.

**Confidence / risk.** High confidence that the target's single field is over-written from disagreeing
sources; medium confidence on how often it bites in practice (it needs a configure to arrive while the
cached `active` is stale). Very low risk.

---

### 3. Answering GTK's terminate callback with `true` kills the loop the dispatcher runs on — P1

**What Noria does.** It *never* lets GTK terminate: the callback always returns `false`, and shutdown is
driven from Kotlin on a thread of its own.

SOURCE `gtk/GtkApplication.kt:465-469`
```kotlin
internal fun handleApplicationWantsToTerminate(): Boolean {
    if (!quitHandlers.values.fold(true) { accumulator, quitHandler -> quitHandler() and accumulator }) return false
    requestStructuredQuit()
    return false
}
```
SOURCE `gtk/GtkApplication.kt:565-596` — `requestStructuredQuit()` closes every window and, once
`Event.WindowClosed` has emptied the map (`gtk/GtkApplication.kt:388-397` calls
`finishStructuredQuitIfNeeded()`), `continueTerminationAsync()` runs the real teardown off the event loop
thread. `quit()` (SOURCE `gtk/GtkApplication.kt:499-503`) goes through the same path.

**What the target does.**

TARGET `gtk/GtkApplication.kt:369-384`
```kotlin
private fun onQuitRequested(): Boolean {
    val isSafeToQuit = quitHandlers.values.fold(true) { acc, handler -> handler() and acc }
    if (!isSafeToQuit || isQuitInProgress) return false
    val openWindows = windowList.toList()
    if (openWindows.isEmpty()) {
        return true          // <-- GTK will call gtk_app.quit()
    }
    isQuitInProgress = true
    openWindows.forEach { it.requestCloseFromQuit() }
    return false
}
```
TARGET `gtk/GtkApplication.kt:510-512` — `override fun quit() { onEventLoop { onQuitRequested() } }`, the
return value discarded.

**Why it matters.** Two distinct bugs:

- **Returning `true` hangs the process.** `application_wants_to_terminate` is wired to the GTK `quit` action
  and to SIGTERM, and a `true` answer calls `gtk_app.quit()`
  (`native/desktop-gtk/src/gtk/application_state.rs:140-146` and `:191-197`). That stops the GLib main loop.
  `Application.runOnEventLoopAsync` is `glib::source::idle_add_once`
  (`native/desktop-gtk/src/gtk/application.rs:121-126`), so *every* task the whole Compose layer has queued —
  including the `runBlocking(KdtDispatcher)` that `runApplication` (TARGET `Application.kt:234`) is sitting
  in — is dropped on the floor and never runs. The composition can no longer make progress and
  `application { }` never returns. The window is narrow (it needs a SIGTERM or a `quit` action while no window
  is open, e.g. between two windows of a multi-window app, or a tray-only phase) but the failure is a hard hang.
- **`quit()` with no windows open silently does nothing.** The `true` is discarded, so nothing terminates.
  Noria's `quit()` reaches `continueTerminationAsync()` in exactly that case.

**How to port.** Adopt Noria's shape, adapted to Compose's composition-owned lifetime:

1. Make `onQuitRequested` always `return false`.
2. Give `GtkApplication` a "nothing left to close" continuation. Compose's equivalent of Noria's
   `continueTerminationAsync()` is ending the composition, i.e. `ApplicationScope.exitApplication()`. Add an
   optional `onTerminate: () -> Unit` to `ComposeApplicationParams` (or a settable
   `ComposeApplication.onTerminateRequested`) that `application()` (TARGET `Application.kt:160-176`) wires to
   `scope.exitApplication()`, and call it from `onQuitRequested` when `openWindows.isEmpty()`.
3. Mirror Noria's `finishStructuredQuitIfNeeded()`: in the `Event.WindowClosed` branch
   (TARGET `gtk/GtkApplication.kt:295-301`), when `isQuitInProgress && windowList.isEmpty()`, invoke the same
   continuation. Today the target relies on the composition noticing by itself, which works for the
   `Window {}` path but not for a quit that started with zero windows.
4. Reset `isQuitInProgress = false` once the quit is over or refused. *(Not from Noria — Noria has the same
   latch bug: if a window's `onCloseRequest` vetoes, `isQuitInProgress` / `structuredQuitInProgress` stays set
   and every later Quit is silently ignored. Worth fixing here rather than copying.)*

**Confidence / risk.** High confidence on the `true` → `gtk_app.quit()` → dead dispatcher chain (traced in
the Rust). Medium confidence on how reachable it is. Step 2 touches public API, so it is the one to design
carefully; steps 1, 3, 4 are safe.

---

### 4. Route key and text-input events by the event's own window id — P2

**What Noria does.** SOURCE `gtk/GtkApplication.kt:338-343` dispatches `KeyDown`/`KeyUp`/`TextInput` through
`event.windowIdOrNull()`, which reads the id off the event itself (SOURCE `gtk/GtkApplication.kt:952-955`).

**What the target does.** It keeps a `keyboardFocusWindowId` mirror and routes through it:

TARGET `gtk/GtkApplication.kt:156-157, 257-261, 271-281`
```kotlin
is Event.TextInput ->
    keyboardFocusWindowId?.let { windowsById[it]?.handleEvent(event) } ?: EventHandlerResult.Continue

is Event.KeyDown, is Event.KeyUp ->
    keyboardFocusWindowId?.let { windowsById[it]?.handleEvent(event) } ?: EventHandlerResult.Continue
```

**Origin.** Not a single fix commit — it is the shape `cc557a8bf5daa` left behind after `ModifiersChanged`
(the one event that genuinely has no window id) was split out of `windowIdOrNull()`.

**Why it matters.** `Event.KeyDown`, `Event.KeyUp` and `Event.TextInput` all carry a `windowId`
(`kotlin-desktop-toolkit/.../gtk/Event.kt:172, 180, 259`), and the key controller is attached per window
(`native/desktop-gtk/src/gtk/window.rs:368-370`), so the event's own id is authoritative. The mirror can only
ever be equal to it or wrong: it is fed from `is_active_notify`, a *different* signal on a *different*
schedule, and a key delivered before the first `WindowKeyboardEnter` (window just mapped and given focus) or
after a `WindowKeyboardLeave` that raced ahead is dropped entirely rather than misrouted. The target already
guards the Leave against reordering (`:277-279`), which is evidence the ordering is not trustworthy.

**How to port.** Add a `windowIdOrNull()` case for `KeyDown`/`KeyUp`/`TextInput` (TARGET
`gtk/GtkApplication.kt:597-610`) and let them fall through to the generic `else` branch, deleting the two
explicit branches. `keyboardFocusWindowId` can go away entirely once finding 1 removes the other use of it;
`mouseFocusWindowId` likewise.

**Confidence / risk.** High confidence it is strictly better. Very low risk — it removes a cache.

---

### 5. Three desktop settings are read by Noria and ignored by the target — P2

**What Noria does.** SOURCE `gtk/GtkApplication.kt:261-292` handles five settings:

```kotlin
is DesktopSetting.ColorScheme -> ...
is DesktopSetting.DoubleClickInterval  -> doubleClickIntervalMillis = setting.value.inWholeMilliseconds
is DesktopSetting.DoubleClickDistancePixels     -> doubleClickDistance = setting.value.dp
is DesktopSetting.DragAndDropDragThresholdPixels -> dragThreshold = setting.value.dp
is DesktopSetting.MiddleClickPaste -> middleClickPasteEnabled = setting.value
```

**What the target does.** TARGET `gtk/GtkApplication.kt:323-342` handles `ColorScheme` and
`DoubleClickInterval`; everything else falls into `else -> {}`.

**Origin.** `2e62048894166` (*"[air] Fix Linux clipboard and drag&drop"*) brought `MiddleClickPaste`; the
distance/threshold pair came with the initial GTK port `c7cc34d3ebe0f` (*"AIR-5137: [fleet, air] basic GTK
implementation"*).

**Why it matters / how to port.** All three exist in this repo's KDT
(`kotlin-desktop-toolkit/.../gtk/DesktopSetting.kt:75, 77, 88`), and each maps to something Compose has —
though not one-to-one:

- `DragAndDropDragThresholdPixels` → `ViewConfiguration.touchSlop` (TARGET `gtk/GtkWindow.kt:224`, currently a
  hardcoded `18.dp`). Note `touchSlop` is in **px**, so it is `setting.value.toFloat()` directly, not via dp.
  This is the one with real user-visible payoff: on a desktop configured for a large drag threshold, Compose
  currently starts drags too eagerly. *Caveat:* Noria keeps `dragThreshold` separate from its
  `ViewConfiguration` and feeds it to its own gesture code, so the mapping onto `touchSlop` is my suggestion,
  not a port — and `touchSlop` is also used for scroll-start slop, where GTK's DnD threshold may be too
  large. Medium confidence; worth an experiment before committing.
- `DoubleClickDistancePixels` → Compose's `ViewConfiguration` has no counterpart. Skip, or expose it on
  `GtkApplication` for content that wants it.
- `MiddleClickPaste` → only meaningful together with finding 10.

The other four backends hardcode the same `18.dp` (`macos/MacOsWindow.kt:219`, `windows/WindowsWindow.kt:222`,
`linux/LinuxWindow.kt:238`), so changing GTK alone introduces an inconsistency; decide that deliberately.

**Confidence / risk.** High confidence the settings are being dropped; medium confidence on the best Compose
mapping. Low risk.

---

### 6. The event-handler catch throws away the stack trace — P2

**What Noria does.** SOURCE `gtk/GtkApplication.kt:429-432`
```kotlin
} catch (throwable: Throwable) {
    logger.error(throwable) { "Failed to handle GTK KDT event $event" }
    EventHandlerResult.Continue
}
```

**What the target does.** TARGET `gtk/GtkApplication.kt:315-320`
```kotlin
} catch (throwable: Throwable) {
    Logger.error { "Failed to handle event $event: $throwable" }
    EventHandlerResult.Continue
}
```
`"$throwable"` is `Throwable.toString()` — class name and message, no frames, no cause chain.

**Origin.** Not a fix commit; but `b049c1dd69ba9` (*"[air] Make loggers lazy"*) is the commit that shows Noria
treats this logger as a first-class diagnostic surface.

**Why it matters.** This catch is the only thing standing between a Compose exception and a dead event loop,
so it is where every non-fatal input/render/DnD bug surfaces. Without frames, a `NullPointerException` from
somewhere inside a composition is unattributable.

**How to port.** One-liner: KDT's `Logger` already has the overload —
`kotlin-desktop-toolkit/.../gtk/Logger.kt:175-179` declares
`public inline fun error(t: Throwable, msg: () -> String = { "" })`. Use
`Logger.error(throwable) { "Failed to handle event $event" }`. Same in the event-loop-thread catch at TARGET
`gtk/GtkApplication.kt:191-193`, and in the equivalent spots in the other three backends.

**Confidence / risk.** Certain. Zero risk.

---

### 7. `currentLinuxWindowSystem()` should require a Wayland socket that actually exists — P2

**What Noria does.** SOURCE `LinuxPlatform.kt:24-32`
```kotlin
internal fun currentLinuxWindowSystem(): LinuxWindowSystem {
    val waylandDisplay: String? = System.getenv("WAYLAND_DISPLAY")
    val waylandSocket: String? = System.getenv("WAYLAND_SOCKET")
    return if (!waylandDisplay.isNullOrBlank() || !waylandSocket.isNullOrBlank()) {
        LinuxWindowSystem.Wayland
    } else {
        LinuxWindowSystem.Gtk
    }
}
```

**What the target does.** TARGET `DesktopPlatform.kt:101-112` also accepts `XDG_SESSION_TYPE == "wayland"` as
proof of a Wayland session, and never looks at `WAYLAND_SOCKET`. `DesktopBackend.default`
(TARGET `DesktopPlatform.kt:62-75`) then picks the Wayland backend for it.

**Origin.** `66b09bbeca1b6` — *"[air] AIR-5601 Avoid Wayland if required environment variables are missing"*.
The diff is precisely the removal of the `XDG_SESSION_TYPE`/`GDK_BACKEND` heuristics in favour of "is there a
socket to connect to".

**Why it matters.** `XDG_SESSION_TYPE=wayland` is what the *login session* declares; it survives into
environments where the compositor socket is not reachable (a `sudo`/`su` shell, a service unit, a container,
an ssh session into a Wayland desktop). The target then routes to the Wayland backend, which has nothing to
connect to, and the failure is a native connect error rather than the perfectly serviceable X11/GTK fallback
that AIR-5601 restored. Conversely, `WAYLAND_SOCKET` (an inherited socket fd, how compositors launch some
clients) is a genuine Wayland session the target classifies as `Unknown` → GTK.

**How to port.** Keep the target's richer three-valued `LinuxWindowSystem` — it is genuinely better than
Noria's two-valued one and `startupInfo` (TARGET `Application.kt:195`) reports it — but:
1. Add `WAYLAND_SOCKET` alongside `WAYLAND_DISPLAY` in the first branch.
2. Demote `xdgSessionType == "wayland"` so that it no longer *forces* the Wayland backend on its own. Simplest
   shape: keep `currentLinuxWindowSystem()` as the honest description of the session, and change
   `DesktopBackend.default` to require a socket:
   ```kotlin
   DesktopPlatform.Linux -> when {
       System.getenv("GDK_BACKEND") != null -> Gtk
       !System.getenv("WAYLAND_DISPLAY").isNullOrBlank() ||
           !System.getenv("WAYLAND_SOCKET").isNullOrBlank() -> Wayland
       else -> Gtk
   }
   ```
   The `GDK_BACKEND` precedence the target added is a real improvement over Noria and should stay.

**Confidence / risk.** High confidence on the reasoning behind AIR-5601. Low risk; it only ever moves a
would-have-failed case onto GTK.

---

### 8. No synchronous "run this on the event loop and give me the answer" helper — P2

**What Noria does.** SOURCE `gtk/GtkApplication.kt:853-869` provides `onEventLoopSync`, which runs inline
when already on the loop thread and otherwise round-trips through a `CompletableDeferred`. Every native call
that has to return something goes through it — window creation (SOURCE `gtk/GtkWindow.kt:890-903`) and the
initial screen read (SOURCE `gtk/GtkWindow.kt:288-291`,
`GtkScreen(application.onEventLoopSync { allScreens().screens.first() })`).

**What the target does.** It has `onEventLoop` (fire-and-forget, inline when already there) and
`onEventLoopAsync` (TARGET `gtk/GtkApplication.kt:533-544`), and makes the value-returning native calls
directly on whatever thread the caller is on:

- TARGET `gtk/GtkApplication.kt:344` — `readScreens()` calls `nativeApplication.allScreens()`, reached from
  the public `mainScreen` (`:129-132`).
- TARGET `gtk/GtkWindow.kt:117-127` — the property initializer calls
  `application.nativeApplication.createWindow(...)`.

**Why it matters.** Today both are in fact reached on the event loop thread — `runApplication` wraps the whole
composition in `runBlocking(KdtDispatcher)` (TARGET `Application.kt:234`), and `createWindow` is documented as
"called by the `Window` composable" — so this is not a live bug. It is a live *hazard*: `mainScreen` is public
API on `ComposeApplication`, and GTK calls from a foreign thread do not fail loudly, they corrupt.

**How to port.** Add Noria's helper next to `onEventLoop`:
```kotlin
internal fun <T> onEventLoopSync(block: Application.() -> T): T {
    if (nativeApplication.isEventLoopThread()) return nativeApplication.block()
    val result = CompletableDeferred<T>()
    nativeApplication.runOnEventLoopAsync {
        runCatching { nativeApplication.block() }.fold(result::complete, result::completeExceptionally)
    }
    return runBlocking { result.await() }
}
```
and route `readScreens()` and the `GtkWindow` native-window creation through it. (Do **not** use it from
inside an event handler — it is inline there, which is the point.)

**Confidence / risk.** High confidence it is the safer shape; low confidence that it fixes anything today.
Low risk.

---

### 9. `MouseUp` should re-establish that the pointer is in the window — P3

**What Noria does.** SOURCE `gtk/InputStateTracker.kt:105-111` — the `MouseUp` branch sets
`pointerInWindow = true` before doing anything else, exactly as `MouseDown` and `MouseMoved` do.

**What the target does.** TARGET `gtk/GtkInputStateTracker.kt:120-136` — `MouseUp` does not touch
`isPointerInWindow` (`MouseDown` at `:102` and `MouseMoved` at `:142` both do).

**Origin.** Present since the initial port `c7cc34d3ebe0f`; not a dedicated fix.

**Why it matters.** GTK holds an implicit grab between press and release, so a release can be delivered after
a `MouseExited` cleared `isPointerInWindow` (TARGET `:160`). The `Release` itself still goes out — it is sent
through `sendPointerEvent` directly — but everything that goes through `sendPointerEventWithCurrentState`
(the post-scroll hover refresh at `:194`, the post-relayout synthetic move at `:300-313`) stays suppressed
until the next `MouseEntered`. Narrow, but free to fix.

**How to port.** Add `isPointerInWindow = true` to the `MouseUp` branch, next to the existing
`startPointerEvent(...)` call.

**Confidence / risk.** Medium confidence it is reachable in practice; certain that it is harmless.

---

### 10. Primary selection (middle-click paste) is not supported — P3

**What Noria does.** A full second clipboard: `GtkSystemSelectionEntry` (SOURCE `gtk/GtkClipboard.kt:72-115`),
`systemSelection()`/`setSystemSelection()` gated on the `MiddleClickPaste` desktop setting
(SOURCE `gtk/GtkApplication.kt:234-245`), `DataSource.PrimarySelection` answered in `getDataTransferData`
(SOURCE `gtk/GtkApplication.kt:452-458`), and `DataTransferCancelled` clearing it
(SOURCE `gtk/GtkApplication.kt:301-311`).

**What the target does.** TARGET `gtk/GtkApplication.kt:412-413` — `DataSource.PrimarySelection -> null`, with
the comment "Compose's `Clipboard` has no concept of the primary selection"; the same at
`gtk/GtkApplication.kt:232` and `gtk/GtkClipboard.kt:69`.

**Does Compose need it?** Not for API parity — `androidx.compose.ui.platform.Clipboard` really has no
primary-selection member, and the target's reasoning is sound. But Compose Desktop on AWT/Swing *does* give
Linux users middle-click paste for free through the AWT text components, so replacing AWT with this layer is
a user-visible regression on Linux. The toolkit already exposes everything needed
(`Application.primarySelectionPaste`/`primarySelectionPut`,
`kotlin-desktop-toolkit/.../gtk/Application.kt:339, 352`).

**How to port.** As an extension rather than a Compose API: add
`suspend fun systemSelection(): String?` / `suspend fun setSystemSelection(text: String?)` to `GtkClipboard`
(and its Wayland twin), gate them on `DesktopSetting.MiddleClickPaste` from finding 5, and answer
`DataSource.PrimarySelection` in `getDataTransferData`. Wiring it into text fields needs a middle-click
handler that content supplies, which is out of this layer's reach — so ship the capability, not the
behaviour.

**Confidence / risk.** High confidence it is missing; genuinely uncertain whether it belongs in a
Compose-mirroring library. Low risk if kept off the `Clipboard` interface.

---

### 11. No notification centre — P3

**What Noria does.** SOURCE `gtk/GtkNotificationCenter.kt` (120 lines): a `showNotification` that suspends on
`Event.NotificationShown`, `removeNotification`, and per-notification action callbacks that can raise a window
using the activation token from `Event.NotificationClosed`. Wired at SOURCE `gtk/GtkApplication.kt:326-336`:
```kotlin
is Event.NotificationClosed -> {
    notificationCenter.onNotificationClosed(event) { windowId, activationToken ->
        windows[windowId]?.activate(activationToken)
    }
    EventHandlerResult.Stop
}
```

**What the target does.** Nothing: `Event.NotificationShown` / `Event.NotificationClosed` fall into the final
`else` of `handleEvent` (TARGET `gtk/GtkApplication.kt:311-313`), `windowIdOrNull()` returns `null` for them,
and the event is answered `Continue`.

**Does Compose need it?** No — Compose Desktop has no notification API either. Note it costs nothing today
(the events are simply ignored) and both KDT calls exist
(`kotlin-desktop-toolkit/.../gtk/Application.kt:369, 380`).

The one detail worth stealing if this is ever built: Noria threads the `activationToken` from
`NotificationClosed` into `Window.activate(token)`. Without that token, raising a window from a notification
click is refused by strict Wayland compositors. The target's `requestFocus()` hardcodes `activate(null)`
(TARGET `gtk/GtkWindow.kt:347-357`), which is correct for user-initiated focus but would not do for this.

**Confidence / risk.** Certain it is absent. Recommend: do not port unless a consumer asks.

---

### 12. Seed a newly created window with the last known modifier state — P3

Follow-on to finding 1, and a gap Noria has too. `ModifiersChanged` only fires on *change*, so a window
created while Ctrl is held starts with an empty `PointerKeyboardModifiers` and stays wrong until the user
lets go. Cheap fix once the broadcast is in place: keep the last `Set<KeyModifiers>` on `GtkApplication`
(the toolkit even emits an initial one at startup — `native/desktop-gtk/src/gtk/application_state.rs:167-172`
sends one if the mask is non-empty, which the target currently drops because no window exists yet) and pass it
to `GtkInputStateTracker`'s constructor in `attachWindow`.

Marked P3 because the window has to be *created* mid-chord for it to matter. Not from Noria — flagged as an
improvement on both.

---

### 13. `gtk/GtkKeys.kt` and `linux/LinuxKeys.kt` are byte-identical — P3

Diffed with `gtk`/`Gtk` and `linux`/`Linux` normalised away: the only differences are the import block and
`0x7eu` vs `0x7eU`. Same for `toPointerKeyboardModifiers`. Noria has the same duplication and marks it
`@file:Suppress("DuplicatedCode")` — it did *not* factor these out, because `org.jetbrains.desktop.gtk.KeySym`
and `org.jetbrains.desktop.linux.KeySym` are unrelated types.

Noria *did* factor out the IME session plumbing (`LinuxInputMethodCommon.kt`: UTF-8↔UTF-16 offset conversion,
`LinuxTextInputContext`, `PlatformTextInputSessionLinux`) shared by both Linux backends. The target's
equivalents — `SurroundingText` with its `codePointsBefore`/`codePointsAfter`/`utf8Length` helpers — are
duplicated between `gtk/GtkTextInput.kt:271-323` and `linux/LinuxTextInput.kt`, and *those* are genuinely
platform-independent (plain `String` arithmetic, no toolkit types). Extracting them into a shared
`LinuxTextOffsets.kt` next to `LinuxMimeTypes.kt` is the one piece of Noria's shared-Linux factoring the
target has not done and could do cleanly. Everything else the target already shares (`LinuxMimeTypes.kt`,
`FileChooserPaths.kt`, `KdtDragAndDropManager.kt`) — and shares *better* than Noria, whose
`KdtDragAndDropManager` is a four-way `when` over window classes (SOURCE `KdtDragAndDropManager.kt:56-82`)
where the target uses a `DragAndDropSourceWindow` interface.

Low value, low risk.

---

## Noria files / capabilities with no target counterpart

| Noria | Target | Verdict |
| --- | --- | --- |
| `gtk/GtkNotificationCenter.kt` + `Event.NotificationShown`/`NotificationClosed` handling | — | Finding 11. Compose has no notification API; skip unless asked. |
| `GtkSystemSelectionEntry`, `systemSelection()`, `DataSource.PrimarySelection` | returns `null` | Finding 10. Not Compose API; a Linux UX regression vs AWT. |
| `gtk/Dispatchers.kt` (`GtkKdtMainDispatcher`) | `KdtDispatcher` (shared, backend-agnostic) | Target is better — one dispatcher for all backends, resolved through `currentApplication()`. |
| `gtk/GtkUriHandler.kt` | inline `uriHandler` in `GtkApplication` (`:143-150`) | Equivalent. Both pass `null` for the activation token; Noria has a `// TODO` on it. |
| `gtk/ViewConfiguration.kt` | inline in `GtkWindow` (`:220-225`) | Target's is better — it reads `doubleClickIntervalMillis` from the desktop setting; Noria's standalone class hardcodes 300 ms and is unused by `GtkWindow`, which has its own inline one. |
| `GtkWindow.captureScreenshot()` | — | Noria test-harness support. Not Compose API. |
| `GtkWindow.reuse()` / `isMarkedForReuse` / `prepareNativeWindowResourcesForReuse` | — | Explicitly out of scope. |
| `Application.awaitWhenReady()` / `didFinishLaunchingCompletableJob` | `applicationStarted` `CountDownLatch` in the constructor | Equivalent; the target's blocking constructor is simpler and reports the failure (`:569-571`). |
| `resetForReuse()` / `stopAndJoin()` / `RenderLoopRegistry` | `close()` | Noria-lifecycle specific. The one idea worth taking is `resetState()`'s `windowsToClose.forEach { it.awaitNativeClosed() }` before `stopEventLoop()` — the target posts `nativeWindow.close()` from `dispose()` (`gtk/GtkWindow.kt:795`) and can stop the loop before that idle runs. At process exit it does not matter. |
| `showOpenSingleDialog` / `showOpenMultipleDialog` / `showSaveDialog` | fully implemented (`gtk/GtkWindow.kt:421-474`) | **Target is ahead** — Noria's are all `// TODO return null`. |
| `LinuxInputMethodCommon.kt` (shared UTF-8 offset maths) | duplicated in `GtkTextInput`/`LinuxTextInput` | Finding 13. |
| `Event.DragAndDropFeedbackFinished` handling | unhandled (`Continue`) | Noria removed its handling in `fe5fa7e1834a2`. Nothing to port. |

---

## Divergences judged NOT worth porting

- **`clearPointerButtons()` at drag-session start (`aec57125a9942`, AIR-5008).** Tempting, because Noria's
  comment describes exactly the "stale tooltip / press hit path keeps receiving events" symptom. But
  `d80de585f4880` (AIR-5571, *"Fix window stuck resizing on KDE Wayland with custom titlebar"*) **reverted**
  it and moved the clearing into the `MouseExited` branch instead — which is precisely what the target already
  does (`gtk/GtkInputStateTracker.kt:159-164`, and the note at `gtk/GtkWindow.kt:716-717`). The target matches
  Noria's *final* state; porting the intermediate one would reintroduce AIR-5571.

- **Returning `Stop` from consumed pointer events** (SOURCE `gtk/InputStateTracker.kt:88-102, 127-141`). The
  target deliberately always returns `Continue`, documented at `gtk/GtkInputStateTracker.kt:323-337`: the
  toolkit attaches pointer controllers in GTK's *capture* phase, so `Stop` also blocks GTK's own titlebar
  buttons and drag handle. That reasoning is sound and is coupled to
  `ComposeWindow.handlesCustomTitleBarGestures`. Do not port — but note that in exchange the target cannot
  stop a Compose-consumed press from reaching GTK, so a `CustomTitleBar` where content draws its own buttons
  will double-handle. That is a design consequence, not a bug to fix by copying Noria.

- **Extended `Key` enum mappings** (SOURCE `gtk/Key.kt:130-160`: `Key.QuotationMark`, `Key.VerticalLine`,
  `Key.LessSign`, `Key.Underscore`, `Key.Colon`, `Key.Ampersand`, …). Out of scope by instruction, and the
  target's folding onto the US-layout base key (`gtk/GtkKeys.kt:85-108`) is the right answer for the published
  `Key`. Checked for genuine gaps: the target additionally maps `question`, `percent` and `asterisk`, which
  Noria lacks; nothing Compose can express is missing from the target. `KeySym.F13`–`F24` are Unknown on both.

- **Per-frame `DirectContext` for the drag icon** (SOURCE `gtk/GtkWindow.kt:931-974`). The target caches it
  for the life of the drag session and closes it in `finishDragSession` (`gtk/GtkApplication.kt:434-435`).
  Target is ahead; GTK asks for a redraw on every frame tick of the drag.

- **`Application.requestActivation()` as a no-op** (SOURCE `gtk/GtkApplication.kt:483-484`, emptied by
  `7ec5af76e8605` *"Fix window activation on Wayland"*). The target activates the focused-or-first window
  (`gtk/GtkApplication.kt:506-508`). Noria's removal was about Wayland activation tokens in the Fleet shell;
  the target's behaviour is the more useful default and there is no evidence it breaks anything on GTK.

- **`withPreparedMainThread` / snapshot read-tracking** (SOURCE, everywhere in `GtkWindow`/`GtkApplication`).
  Out of scope by instruction; Compose's `ComposeScene` + `KdtSnapshotManager` cover the same ground.

- **Noria's `launchScene` frame loop** — `BroadcastFrameClock`, `DrainableUpdateQueue`, the windowless-frame
  fallback at 60 Hz, `withoutReentrancy`, `AIR-6084`/`AIR-6157`/`AIR-5450`. Noria's rendering architecture;
  `CanvasLayersComposeScene` owns all of this for Compose. Noted only because the *windowless* frame
  fallback has a Compose analogue: a GTK window that is unmapped stops receiving `WindowFrameTick`
  (`native/desktop-gtk/src/gtk/window.rs:74-84` — `add_tick_callback` ticks only while the widget is mapped),
  so that scene's animations and `withFrameNanos` awaiters stall. Nothing visible is affected; if a consumer
  ever needs off-screen animation progress, this is where it would go.

---

## Tickets that do not apply (verified)

Four of the six tickets named in the brief were checked with `git log --all --grep` and `git show --stat`;
none touches the GTK backend.

| Ticket | Commit | What it actually changed |
| --- | --- | --- |
| AIR-6619 — activate/deactivate IME, drop composed-text marker on focus loss | `4b636801ab9ad` | `macos/MacOsTextInputSessionOwner.kt` only (9 insertions). The target's GTK backend **already** does the GTK equivalent and does it better than Noria's GTK: `gtk/GtkWindow.kt:557-566` disarms the IME *and* calls `textInput.discardActivePreedit()`, plus `cancelCompositionOnPress()` at `:656-664`; Noria's GTK (`gtk/GtkWindow.kt:510-524`) only calls `textInputDisable()` and leaves the marker in the field. |
| AIR-5558 — new KDT clipboard with async retries | `9d61f959d21cd` | `windows/WindowsClipboard.kt` + KDT version bump. No Linux/GTK change. |
| AIR-6275 — process system notifications when the window is not visible | `5f9027c853231` | One line in `macos/MacOsApplication.kt`. |
| AIR-5776 — translate all system key down/up events | `b7a01f0ac4de0` | `windows/WindowsWindow.kt` only. |
| AIR-5621 — Shift+Scroll horizontal scrolling | `a662747de3f0a` | Does touch `gtk/InputStateTracker.kt` — and the target **already has it**: `gtk/GtkInputStateTracker.kt:382-389` swaps the axes on Shift, with the same 15×/100× smooth-vs-notch split Noria arrived at in `1ea29f3ceaa6a` (AIR-5233). |
| AIR-4578 — shadow for the window on Linux Wayland | `fe5fa7e1834a2` | A KDT version bump. Its GTK-side diff is purely the `LogicalPixels`/`LogicalPixelsInt`/`Scale.rawValue` type migration the target was written against from the start. The actual shadow work is Wayland-side (`linux/LinuxWindow.kt`, `WindowDecoration.kt`) and has no GTK counterpart — GTK composites the shadow itself, which is why the target clears the canvas to transparent (`gtk/GtkWindow.kt:772`). |

One more from the brief's themes, checked and clean: **clipboard async retries.** Noria's GTK clipboard has
none either; the retry logic AIR-5558 added lives in `WindowsClipboard`. The one GTK clipboard fix worth
knowing about is `204628d9f04cd` (AIR-5076, *"Fix clipboard paste by not prematurely removing the pending
clipboard request from the map"*): its two lessons were "remove the pending request even when
`event.content` is null" and "resume the continuation with null rather than dropping it". The target's
`onDataTransfer` (`gtk/GtkClipboard.kt:81-85`) plus `RemoteClipboardEntry.getForFormat`
(`gtk/GtkClipboard.kt:109-111`, `content?.let { … }.orEmpty()`) already handle both. The remaining hole —
shared with Noria — is that a pending read is never resumed if the `DataTransfer` event simply never arrives
(the source application died mid-transfer), leaving the coroutine suspended forever. Neither side has a
timeout; worth considering independently.
