# Noria → KDT-Compose: the native Linux (Wayland) backend

Read-only comparison of

- **TARGET** `kotlin-desktop-toolkit-compose/src/main/kotlin/org/jetbrains/desktop/compose/linux/` (+ shared `LinuxMimeTypes.kt`, `Application.kt`, `DesktopPlatform.kt`)
- **SOURCE** `/Users/wojtek/IdeaProjects/ultimate2/fleet/noria/ui/srcJvmMain/androidx/compose/ui/desktop/linux/` (+ shared `LinuxInputMethodCommon.kt`, `LinuxPlatform.kt`, `MimeTransfer.jvm.kt`, `Application.jvm.kt`)

All source paths below are relative to those two roots unless written out in full.

**Headline:** the target is *ahead* of Noria on most of the input-tracking work (Shift+scroll, wheel120 scaling,
modifier tracking across focus/pointer changes, clearing buttons on Leave, the synthetic post-relayout move,
drag-image even-pixel handling, the `WindowDraw` invalidation guard). Those Noria commits have already been
absorbed. What is *not* absorbed clusters in three places: **application lifetime/termination**, **window
activation**, and **client-side decoration (frame/shadow/resizers)**. Two of those are outright bugs that were
verified against the KDT Rust sources in this repo.

Because the target repo *is* kotlin-desktop-toolkit, it builds against KDT `HEAD`; Noria consumes a published
artifact. So there is no case of "Noria uses a newer KDT call" — every API Noria uses exists in the target's
KDT. The divergences are all "the target uses an API less correctly, or not at all".

---

## Ranked findings

### 1. `close()` deadlocks after any user-initiated `quit()` — the terminate handshake is inverted — **P0**

**What the target does.** `TARGET linux/LinuxApplication.kt:182-186`:

```kotlin
Event.ApplicationWantsToTerminate ->
    // Stop means "do not terminate", which is what we want whenever there is still a
    // handler vetoing or a window to close first.
    if (onQuitRequested()) EventHandlerResult.Continue else EventHandlerResult.Stop
```

and `TARGET linux/LinuxApplication.kt:583-588`:

```kotlin
override fun close() {
    nativeApplication.stopEventLoop()
    eventLoopThread.join()
    ...
}
```

**Why it breaks.** Verified against the KDT native loop in this repo,
`native/desktop-linux/src/linux/application.rs:237-243`:

```rust
if self.exit.load(Ordering::Acquire) && !send_event(event_handler, Event::ApplicationWantsToTerminate) {
    send_event(event_handler, Event::ApplicationWillTerminate);
    ...
    Ok(false)   // loop exits
} else {
    Ok(true)    // loop keeps spinning, `exit` STAYS true
}
```

`stopEventLoop()` only sets `exit = true` (`native/desktop-linux/src/linux/application_api.rs:119-126`); it
never clears it. So `ApplicationWantsToTerminate` is re-delivered **every 16 ms forever** until the handler
returns `Continue`. Two concrete failures:

1. `onQuitRequested()` (`TARGET linux/LinuxApplication.kt:354-369`) returns `false` whenever
   `isQuitInProgress` is set, and `isQuitInProgress` is **never reset** (`TARGET
   linux/LinuxApplication.kt:91-92, 365`). Sequence: user picks Quit → `quit()` → `onQuitRequested()` sets
   `isQuitInProgress = true` and asks the windows to close → windows close → composition ends →
   `application { }`'s `use` calls `close()` → `stopEventLoop()` → `ApplicationWantsToTerminate` →
   `isQuitInProgress == true` → `EventHandlerResult.Stop` → the loop never exits → `eventLoopThread.join()`
   blocks forever. **The process hangs on exit after every `application.quit()`.**
2. Even without that, a quit handler that vetoes turns into a 60 Hz busy-loop that re-runs *every* registered
   quit handler and re-calls `requestCloseFromQuit()` bookkeeping until it gives up.

There is a third, smaller bug in the same area: `quit()` (`TARGET linux/LinuxApplication.kt:536-538`) discards
the `Boolean` that `onQuitRequested()` returns, so `application.quit()` with **zero** windows open does
nothing at all — nothing calls `stopEventLoop()`.

**What Noria does.** `SOURCE linux/LinuxApplication.kt:326-331` always returns `Continue` and drives the quit
from Kotlin instead:

```kotlin
Event.ApplicationWantsToTerminate -> {
    if (quitHandlers.values.fold(true) { acc, h -> h() and acc }) {
        requestStructuredQuit()
    }
    EventHandlerResult.Continue
}
```

with `SOURCE linux/LinuxApplication.kt:757-785`:

```kotlin
private fun requestStructuredQuit() {
    if (terminationInProgress || structuredQuitInProgress) return
    val windowsToClose = windows.values.toList()
    if (windowsToClose.isNotEmpty()) {
        structuredQuitInProgress = true
        windowsToClose.forEach { it.requestClose(WindowCloseRequestReason.ApplicationQuit) }
        return
    }
    continueTerminationAsync()
}
internal fun finishStructuredQuitIfNeeded() {
    if (!structuredQuitInProgress || windows.isNotEmpty()) return
    continueTerminationAsync()
}
private fun continueTerminationAsync() {
    if (terminationInProgress) return
    structuredQuitInProgress = false
    terminationInProgress = true
    ...  // off-loop thread -> stopAndJoin() -> nativeApplication.stopEventLoop()
}
```

`finishStructuredQuitIfNeeded()` is called from `Event.WindowClosed` once the map empties
(`SOURCE linux/LinuxApplication.kt:487-497`), and `stopAndJoin()` clears `structuredQuitInProgress` and sets
`terminationInProgress` before calling `stopEventLoop()` (`SOURCE linux/LinuxApplication.kt:662-673`).

**Origin.** Not one commit — this is the shape Noria arrived at; the closest related tickets are
`ec1a8f6c3b69d` *AIR-5478 [air] mitigate the issue but not calling the stopEventLoop for the second time*
(Windows: guard a second `stopEventLoop`) and `e2ba99e694520` *AIR-6701 the order of execution of shutdown
hooks are not specified, so it was a race condition* (removed the JVM shutdown hook entirely — see
"Not worth porting" below). The Linux inversion itself is verifiable directly against
`native/desktop-linux/src/linux/application.rs:237`.

**How to port.**

- Make `Event.ApplicationWantsToTerminate` return `EventHandlerResult.Continue` unconditionally. The event
  only fires because *we* called `stopEventLoop()`, so "veto" at this point is meaningless — the veto has to
  happen before `stopEventLoop()`.
- Move the quit-handler poll and the window-closing loop into `quit()` (the path the veto belongs on), keep
  `isQuitInProgress`, and reset it in `close()` (or add a `terminationInProgress` flag, as Noria does).
- In `close()`, guard `stopEventLoop()` so it is called at most once, and add a bounded `join(timeout)` so a
  wedged loop degrades to a slow exit rather than a hang.
- Make `quit()` act on the `true` result: when there are no windows, terminate (either call `close()`'s path
  or let the composition unwind by flipping the application scope's `isRunning`).

**Confidence: high** (mechanism confirmed in the native sources in this repo). **Risk: low-medium** — touches
shutdown, so worth a manual "Quit with a window open" and "Quit with no window" check on GNOME and Sway. Note
`macos/`, `gtk/` and `windows/` may share the same inverted shape; check them (other agents' scope).

---

### 2. The activation token is requested from the wrong surface, so `requestFocus()` on a background window is a no-op — **P0/P1**

**What the target does.** `TARGET linux/LinuxApplication.kt:521-528`:

```kotlin
internal fun requestWindowActivation(windowId: WindowId) {
    val window = windowsById[windowId] ?: return
    onEventLoop {
        window.nativeWindow.requestActivationToken()?.let { requestId -> pendingActivations[requestId] = windowId }
    }
}
```

i.e. the window *being activated* asks for its own token.

**What Noria does.** `SOURCE linux/LinuxApplication.kt:624-631`:

```kotlin
internal fun requestWindowActivation(windowId: LightweightWindowId) {
    val nativeWindow = windows[keyboardFocusWindowId]?.nativeWindow ?: return
    onEventLoopAsync {
        nativeWindow.requestActivationToken()?.also { requestId -> pendingActivationRequests[requestId] = windowId }
    }
}
```

The **currently focused** window requests the token; the token is then handed to the *target* window.

**Origin.** `7ec5af76e8605` *[air] Fix window activation on Wayland* (Nikola Kocić) — the diff is exactly
`- windows[windowId]?.nativeWindow` / `+ windows[keyboardFocusWindowId]?.nativeWindow`. The same commit also
changed `focusedWindow` (see finding 3) and emptied `requestActivation()`.

**Why it matters.** Confirmed against `native/desktop-linux/src/linux/application.rs:521-547`: the parameter is
literally named `source_window_id`, and the request is built from that surface plus
`get_latest_event_seat_and_serial()`, with a comment citing mutter and kwin:

```rust
pub fn request_activation_token(&mut self, source_window_id: WindowId) -> anyhow::Result<RequestId> {
    let source_w: &SimpleWindow = self.state.get_window(source_window_id)...;
    // Serial should be from the latest keyboard focus or mouse button down event, e.g. <mutter/kwin links>
    xdg_activation.request_token(&self.qh, RequestData { app_id: ..., seat_and_serial, surface: Some(source_w.window.wl_surface().clone()), ... });
}
```

Compositors validate that the requesting surface holds focus (that is the whole point of xdg-activation — it
is the anti-focus-stealing mechanism). A token requested by a background surface is refused or issued
invalid, so `window.requestFocus()` / `requestBringToFront()` / `requestFocusAndBringToFront()` on any window
that is not already focused silently does nothing. That covers "click a notification → raise that window",
"open document → focus its existing window", and multi-window `requestFocus`.

**How to port.** One-line change in `TARGET linux/LinuxApplication.kt:521-528`: source the `nativeWindow` from
`keyboardFocusWindowId`, keep `pendingActivations[requestId] = windowId` pointing at the target. Keep the
early `return` when nothing is focused (the request would be refused anyway). While there, reconsider
`requestActivation()` (`TARGET linux/LinuxApplication.kt:532-534`): with the fix it becomes a no-op when
nothing is focused, which is the honest answer on Wayland — Noria emptied it entirely
(`SOURCE linux/LinuxApplication.kt:636-637`).

**Confidence: high.** **Risk: very low.**

---

### 3. `focusedWindow` is derived from a per-window flag that has two writers instead of from keyboard focus — **P1**

**What the target does.** `TARGET linux/LinuxApplication.kt:108`:

```kotlin
override val focusedWindow: ComposeWindow? get() = windowList.firstOrNull { it.isFocused }
```

and `isFocused` is written from **three** places: `Event.WindowConfigure.active`
(`TARGET linux/LinuxWindow.kt:627`), `Event.WindowKeyboardEnter` (`TARGET linux/LinuxWindow.kt:568-571`) and
`Event.WindowKeyboardLeave` (`TARGET linux/LinuxWindow.kt:573-576`).

**What Noria does.** `SOURCE linux/LinuxApplication.kt:252-253`:

```kotlin
override val focusedWindow: Window?
    get() = windows[keyboardFocusWindowId]
```

and `isFocused` has exactly one writer, `handleConfigure` (`SOURCE linux/LinuxWindow.kt:476`); keyboard
enter/leave only update the tracker's `hasKeyboardFocus` (`SOURCE linux/LinuxWindow.kt:502-508`).

**Origin.** `7ec5af76e8605` *[air] Fix window activation on Wayland*
(`- get() = windows.values.firstOrNull { it.isFocused }` / `+ get() = windows[keyboardFocusWindowId]`).

**Why it matters.** `xdg_toplevel.configure`'s `activated` state and `wl_keyboard.enter/leave` are separate
signals and can disagree — a compositor keeps a toplevel "activated" while a popup or an OSD holds the
keyboard, and during a window switch the new window's configure can arrive before the old window's keyboard
leave. With three writers and a `firstOrNull` scan, two windows can both report `isFocused == true` and
`focusedWindow` then returns whichever was created first. Since finding 2 makes `requestWindowActivation`
depend on the *right* window being identified as focused, and since `TARGET
linux/LinuxApplication.kt:532-534` routes `requestActivation()` through `focusedWindow`, the two bugs
compound.

**How to port.** Change `focusedWindow` to `windowsById[keyboardFocusWindowId]`. Then decide the single
source of truth for `ComposeWindow.isFocused` / `WindowInfo.isWindowFocused`: Noria uses
`WindowConfigure.active`. Compose's `WindowInfo.isWindowFocused` drives caret blinking and focus-ring
rendering, for which "activated" is the right notion, so dropping the keyboard-enter/leave writes matches
both Noria and Compose's semantics. Keep `hasKeyboardFocus` inside `LinuxInputStateTracker` (it already
exists, `TARGET linux/LinuxInputStateTracker.kt:82`) for the modifier-clearing logic.

**Confidence: high** for `focusedWindow`; **medium** for removing the enter/leave writes (test caret blink on
GNOME, where a client-side-decorated window keeps `activated` across menu grabs). **Risk: low.**

---

### 4. Client-side decoration: no shadow, no compositor-owned resizers, `tiled` and the frame ignored (AIR-4578) — **P1**

**What the target does.** `TARGET linux/LinuxWindow.kt:126-134` deliberately asks for a zero frame:

```kotlin
preferClientSideDecoration = params.decoration !is WindowDecoration.Decorated,
// A frame with no padding keeps the surface exactly the size of the window ...
clientSideDecorationFrame = WindowFrame.all(
    WindowFrameSide(padding = LogicalPixelsInt.Zero, resizerThickness = LogicalPixelsInt.Zero),
),
```

`handleConfigure` (`TARGET linux/LinuxWindow.kt:633-638`) then throws the frame away:

```kotlin
decoration = when (event.decorationMode) {
    WindowDecorationMode.Server -> WindowDecoration.Decorated
    is WindowDecorationMode.Client -> WindowDecoration.CustomTitleBar(height = customTitleBarHeight)
}
```

and the resize affordance is reimplemented *inside* the content: a 6 dp strip / 16 dp corners in
`TARGET linux/LinuxWindowBorders.kt:21-53`, driven from `TARGET linux/LinuxWindow.kt:580-598, 690-711`.

**What Noria does.** `SOURCE linux/LinuxWindow.kt:973-984` creates the window with a real frame:

```kotlin
clientSideDecorationFrame = WindowFrame.default().toLinuxWindowFrame(),
```

where `WindowFrame.default()` is `padding = 24.dp, resizerThickness = 12.dp` on all four sides (introduced in
the same commit, `fleet/noria/ui/srcCommonMain/androidx/compose/ui/window/WindowDecoration.kt`), and reports
it straight back to the content (`SOURCE linux/LinuxWindow.kt:482-485`):

```kotlin
decoration = when (val decorationMode = event.decorationMode) {
    WindowDecorationMode.Server -> WindowDecoration.Decorated
    is WindowDecorationMode.Client -> WindowDecoration.Undecorated(decorationMode.frame.toWindowFrame())
}
```

with per-side `tiled` carried through in `SOURCE linux/Converters.kt:108-132`. `requestDecoration` can change
the frame at runtime via `setClientSideDecorationFrame` (`SOURCE linux/LinuxWindow.kt:256-272`), and the
window is cleared to `Color.TRANSPARENT` rather than white (`SOURCE linux/LinuxWindow.kt:844, 867`).

**Origin.** `fe5fa7e1834a2` *[air] AIR-4578 Add a shadow for the Air window on Linux Wayland* — "New KDT also
fixes AIR-5179 and AIR-5907". The same commit brought `LogicalPixels`/`LogicalPixelsInt`, `rawPhysical`,
`LogicalSize.makeWH`, `startMove(serial)` / `startResize(serial, edge)` / `showMenu(serial, position)` and
`requestActivationToken` — **all of which the target already uses**, so only the frame half is missing.

**Why it matters** (verified in `native/desktop-linux/src/linux/window.rs:229-282`):

```rust
let surface_size = LogicalSize { width: content_size.width + frame.left.padding + frame.right.padding, ... };
let geometry = LogicalRect { x: frame.left.padding, y: frame.top.padding, width: content_size.width, ... };
window.xdg_surface().set_window_geometry(...);
let input_region_rect = LogicalRect {
    x: geometry.x - frame.left.resizer_thickness,
    ...
};
window.set_input_region(Some(input_region.wl_region()));
```

- `padding` is the *only* way to get area outside the xdg window geometry to paint a drop shadow into. With
  padding 0 a CSD window on GNOME has no shadow at all, and no room for rounded corners.
- `resizerThickness` extends the **input region** outward from the geometry *into the padding*. With padding 0
  the extended region falls outside the surface and is clipped by the compositor — so **resizerThickness is
  useless without padding**. That is precisely why the target had to invent `LinuxWindowBorders.kt` and steal
  6 dp of content: it is the workaround for having asked for no frame.
- Stealing 6 dp of content is itself a defect: a Compose control flush against the window edge (a scrollbar,
  a full-bleed toolbar button, a split-pane divider) becomes unclickable, and `Modifier.pointerHoverIcon`
  loses to the forced resize cursor (`TARGET linux/LinuxWindow.kt:697-705`).
- `tiled` is dropped entirely. When the compositor half-snaps a window it clears the tiled sides so the client
  stops drawing a shadow/rounded corner there; the target cannot know.
- KDT already resets the frame to all-zero when maximized/fullscreen/server-decorated
  (`native/desktop-linux/src/linux/application_state.rs:536-545`), which lines up with the target's existing
  `placement == WindowPlacement.Floating` guard.

**How to port** — this is a staged change, not a one-liner, because the target's `WindowDecoration` API has no
frame concept:

1. **Enable the frame.** Pass a non-zero `clientSideDecorationFrame` (start with Noria's 24 dp / 12 dp).
2. **Split surface from content.** `Event.WindowConfigure.size` is the **surface** size
   (`application_state.rs:554`), and pointer coordinates are surface-relative. So: keep `size` = surface,
   make `contentSize` = `size - (left+right, top+bottom) padding` (the `ComposeWindow.contentSize` hook
   already exists, `TARGET Window.kt:89`), size the scene to `contentSize`, and subtract the top-left padding
   in `LogicalPoint.toPxOffset` / `PlatformContext.convertLocalToWindowPosition`
   (`TARGET linux/LinuxWindow.kt:270-276`).
3. **Draw into the padding.** Clear to `SkiaColor.TRANSPARENT` instead of `SkiaColor.WHITE`
   (`TARGET linux/LinuxWindow.kt:793, 816`) and render the scene translated by the padding; the padding band
   is where the layer (or the content) paints the shadow.
4. **Move the resizer detection into the padding band.** Replace `resizeEdgeAt(offset, size)` with a test
   against the padding band (`offset` outside the content rect but inside the surface) and delete the 6 dp
   in-content strip. `startResize(serial, edge)` stays exactly as it is.
5. **Surface the frame to content.** Either extend `WindowDecoration.CustomTitleBar` with the frame /
   `tiled` flags, or add a `windowFrameInsets: PaddingValues`-shaped property next to `customTitleBarInsets`
   (`TARGET Window.kt:122`).

If the full change is too big, step 1 + 4 alone (frame enabled, resizers outside the window, no shadow drawn
yet, content offset) already removes the "6 dp of content is unclickable" defect and gives compositor-grade
edge grabbing. Steps 2-3 are what unlock the shadow.

**Confidence: high** on the mechanism (read from the Rust). **Risk: high** — coordinate-space change touching
hit testing, IME caret rectangles (`TARGET linux/LinuxTextInput.kt:222-243`), and drag-and-drop positions
(`TARGET linux/LinuxDragAndDropManager.kt:90, 140`). Do it behind a flag and test on GNOME (always CSD), KDE
and Sway (which does not decorate either).

---

### 5. Backend detection ignores `WAYLAND_SOCKET` (AIR-5601) — **P2**

**What the target does.** `TARGET ../DesktopPlatform.kt:101-112` reads `WAYLAND_DISPLAY`, then
`XDG_SESSION_TYPE`, then `DISPLAY`.

**What Noria does.** `SOURCE ../LinuxPlatform.kt:24-32`:

```kotlin
val waylandDisplay: String? = System.getenv("WAYLAND_DISPLAY")
val waylandSocket: String? = System.getenv("WAYLAND_SOCKET")
return if (!waylandDisplay.isNullOrBlank() || !waylandSocket.isNullOrBlank()) Wayland else Gtk
```

**Origin.** `66b09bbeca1b6` *[air] AIR-5601 Avoid Wayland if required environment variables are missing* —
which deliberately *removed* the `XDG_SESSION_TYPE` and `GDK_BACKEND` heuristics and added `WAYLAND_SOCKET`.

**Why it matters.** A client launched by a compositor with a pre-opened connection gets `WAYLAND_SOCKET` (an
inherited fd) and no `WAYLAND_DISPLAY` — nested compositors, some session managers, and `wl_display_connect`'s
documented second path. The target would then fall through to `XDG_SESSION_TYPE`/`DISPLAY` and can land on
`X11`/`Unknown` → GTK backend in a session where the Wayland backend would have worked. Conversely Noria's
commit shows the *opposite* lesson too: `XDG_SESSION_TYPE == "wayland"` alone is not enough evidence to pick
Wayland — a session can declare wayland while the actual client gets no socket.

**How to port.** Add `WAYLAND_SOCKET` alongside `WAYLAND_DISPLAY` in `currentLinuxWindowSystem()`. Do **not**
adopt Noria's removal of `XDG_SESSION_TYPE`/`DISPLAY`: the target's enum has a real `X11` and `Unknown` case
that `DesktopBackend.default` uses for a better error message, which Noria's two-valued enum cannot express.
The target's `GDK_BACKEND != null → Gtk` rule is a documented deliberate policy divergence; leave it.

**Confidence: high.** **Risk: none.**

---

### 6. `WindowCapabilities` from `WindowConfigure` is ignored — **P2**

**What the target does.** `TARGET linux/LinuxWindow.kt:625-642` reads `size`, `active`, `maximized`,
`fullscreen`, `decorationMode` from `Event.WindowConfigure` and drops `capabilities`.

**What Noria does.** `SOURCE linux/LinuxWindow.kt:481` stores it (`windowCapabilities = event.capabilities`)
and filters the titlebar's minimize/maximize buttons against it
(`SOURCE linux/LinuxWindow.kt:1077-1083`):

```kotlin
private fun WindowDecoration.TitleBarElement.isAvailableIn(capabilities: WindowCapabilities?): Boolean =
    when (this) {
        WindowDecoration.TitleBarElement.MinimizeButton -> capabilities?.minimize ?: true
        WindowDecoration.TitleBarElement.MaximizeButton -> capabilities?.maximize ?: true
        else -> true
    }
```

**Why it matters.** `WindowCapabilities` carries `windowMenu`, `maximize`, `fullscreen`, `minimize`
(`kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/linux/Event.kt:34-48`). Since the target tells
every CSD window's content "you draw the titlebar" (`TARGET linux/LinuxWindow.kt:637`), the content has no way
to know that, say, minimize is unavailable on this compositor — it draws a button that does nothing.
`requestPlacement(Fullscreen)` and `requestMinimized(true)` (`TARGET linux/LinuxWindow.kt:374-394`) likewise
fire into the void.

**How to port.** Add an observable `capabilities` (or four booleans: `canMinimize`, `canMaximize`,
`canFullscreen`, `hasWindowMenu`) to `ComposeWindow`, defaulting to `true` on platforms with no equivalent,
and set it in `handleConfigure`. Content that draws a custom titlebar reads it. Cheap and self-contained.

**Confidence: high.** **Risk: none** (purely additive).

---

### 7. Desktop titlebar settings and the window menu are not consumed — **P2**

**What the target does.** `handleSettingChange` (`TARGET linux/LinuxApplication.kt:309-335`) handles
`ColorScheme`, `DoubleClickInterval`, `CursorTheme`, `CursorSize` and ignores the rest.

**What Noria does.** `SOURCE linux/LinuxApplication.kt:355-383` additionally consumes
`DesktopSetting.TitlebarLayout` (parsed into left/right element lists by
`SOURCE linux/LinuxApplication.kt:1069-1091`), `ActionDoubleClickTitlebar`, `ActionMiddleClickTitlebar`,
`ActionRightClickTitlebar` and `MiddleClickPaste`; the window exposes them as
`requestTitleBarDoubleClickAction` / `TertiaryClickAction` / `SecondaryClickAction`
(`SOURCE linux/LinuxWindow.kt:321-331`) implemented by `performTitleBarAction`
(`SOURCE linux/LinuxWindow.kt:905-928`), which calls `nativeWindow.showMenu(serial, position)` for
`DesktopTitlebarAction.Menu`.

**Why it matters.** All of these exist in the target's KDT
(`.../linux/DesktopSetting.kt:78-136`, `.../linux/Window.kt:92`). Because the target forces custom titlebars
on every CSD window, a Compose app on GNOME gets close-on-the-right, on KDE/Breeze close-on-the-left,
double-click-to-maximize, middle-click-to-lower, right-click-window-menu — and the layer gives the app no way
to honour any of it. Right-click on a CSD titlebar producing no window menu is the most visible.

**How to port.** Compose Desktop has no analogue, so this is a KDT-Compose extension. Minimum viable:
expose `titleBarDoubleClickAction`/`MiddleClick`/`RightClick` on `ComposeApplication` (or a Linux-specific
interface) plus a `ComposeWindow.showWindowMenu(pointerEvent)` that forwards
`showMenu(serial, locationInWindow)` — the target already extracts `Event.MouseDown.serial` for
`startInteractiveMove` (`TARGET linux/LinuxWindow.kt:439-442`), so the plumbing exists. The full
`TitlebarLayout` string parse is nice-to-have; port `toCustomTitleBarLayout()` verbatim if you want it.

**Confidence: high** that the capability is missing; **medium** on how much Compose should expose.
**Risk: low** (additive).

---

### 8. Factor the UTF-8/UTF-16/code-point conversions into a shared file, mirroring `LinuxInputMethodCommon.kt` — **P2**

**What the target does.** `SurroundingText` with `cursorCodePointOffset`, `selectionStartCodePointOffset`,
`codePointsBefore`, `codePointsAfter` and the private `Int.utf8Length()` are duplicated **byte-for-byte**
between `TARGET linux/LinuxTextInput.kt:253-298` and `TARGET ../gtk/GtkTextInput.kt:278-323`.

**What Noria does.** `SOURCE ../LinuxInputMethodCommon.kt` holds the shared arithmetic —
`utf8OffsetToUtf16Offset` (`:18-41`), `LinuxTextInputSurroundingText.convertByteOffsetsFromCursor`
(`:43-63`), `codepointFromOffset` (`:84-86`) — used by both the Wayland and GTK owners.

**Origin.** `369319fadacfe` *[air] AIR-5610 AIR-5611 Enable IME on Linux* created the file.

**Why it matters.** This is pure Kotlin over `String` with no KDT types in it, so unlike `LinuxKeys.kt` /
`LinuxConverters.kt` (which are byte-identical between `linux/` and `gtk/` but reference *different KDT
packages* and therefore genuinely cannot be shared) it is trivially shareable. The target has already done
the equivalent sharing for MIME handling (`LinuxMimeTypes.kt` == Noria's `MimeTransfer.jvm.kt`) and platform
detection (`DesktopPlatform.kt` == Noria's `LinuxPlatform.kt`), so this is the one piece left.

**How to port.** Move `SurroundingText`, `utf8Length` and the code-point helpers into a new
`org/jetbrains/desktop/compose/LinuxInputMethodCommon.kt` next to `LinuxMimeTypes.kt`; both `LinuxTextInput`
and `GtkTextInput` keep their own `TextInputContext`/`TextInputSurroundingText` construction (those *are*
package-specific). Nets ~50 duplicated lines and, more importantly, one place to fix the offset arithmetic.

**Confidence: high.** **Risk: very low.** Coordinate with the GTK agent — it touches `gtk/GtkTextInput.kt`.

---

### 9. The application-level frame clock is unthrottled; a window-less scene can spin — **P2**

**What the target does.** `TARGET ../Application.kt:274-279`:

```kotlin
private object YieldFrameClock : MonotonicFrameClock {
    override suspend fun <R> withFrameNanos(onFrame: (Long) -> R): R {
        yield()
        return onFrame(System.nanoTime())
    }
}
```

Any `withFrameNanos` awaiter in the *application-level* composition therefore re-runs as fast as the event
loop can dispatch.

**What Noria does.** `SOURCE linux/LinuxApplication.kt:820-852` throttles a scene with no attached windows to
60 Hz:

```kotlin
} else if (!noWindowsFrameRequested) {
    noWindowsFrameRequested = true
    val delayMs = ((nextNoWindowsFrameDeadlineNs - System.nanoTime()).coerceAtLeast(0L) + 999_999L) / 1_000_000L
    launch { if (delayMs > 0L) delay(delayMs); ...; nextNoWindowsFrameDeadlineNs = System.nanoTime() + noWindowsMinFrameIntervalNs; ... }
}
```

with `private const val noWindowsMinFrameIntervalNs = 1_000_000_000L / 60` (`SOURCE
linux/LinuxApplication.kt:1143`), plus `noWindowAnimationCoroutineContext`
(`SOURCE ../NoWindowAnimationPolicy.kt`), which zeroes `MotionDurationScale` and suspends
`InfiniteAnimationPolicy` while a scene has no native windows.

**Origin.** `SOURCE ../NoWindowAnimationPolicy.kt`'s own doc comment; the throttle landed with the render-loop
work around `1c212e809ce7a` *AIR-6157 noria: arm delivery frames from pending work on windowed drivers* and
`e95153775d46c` *AIR-6084 Drop frames requested before the scene loop is bound*.

**Why it matters.** In the target the per-window scenes are correctly compositor-paced (`Event.WindowDraw` +
the `isFrameRequested` guard, `TARGET linux/LinuxWindow.kt:552-561` — Noria's own AIR-5322 fix, already
absorbed). The application-level composition is the one that is not paced. A `LaunchedEffect` at
application scope that animates, or any library that awaits frames outside a window, burns the event loop
thread at full speed and starves input. The window is small (the application composition emits no pixels), so
this is latent rather than observed.

**How to port.** Replace `YieldFrameClock` with a clock that yields *and* sleeps to the next 1/60 s deadline,
mirroring `noWindowsMinFrameIntervalNs`. Optionally port `NoWindowAnimationPolicy` verbatim — it is 37 lines,
Compose-only (`MotionDurationScale` + `InfiniteAnimationPolicy` are stock compose-ui), and would also cover
the "window minimized, so no `WindowDraw`" case where a window scene's animations stall indefinitely.

**Confidence: medium** (no reproducer; reasoned from the code). **Risk: low.**

---

### 10. `close()`/`stopEventLoop()` are not idempotent (AIR-5478's lesson) — **P2**

`TARGET linux/LinuxApplication.kt:583-588` calls `stopEventLoop()` then `close()` on the native application
with no guard. Noria's Windows backend hit exactly this: `ec1a8f6c3b69d` *AIR-5478 [air] mitigate the issue by
not calling the stopEventLoop for the second time*, later reinforced by `f73057d4df4dc` *[air, Windows]
AIR-6896 Prevent WindowsApplication from terminating more than once*. On Linux `stopEventLoop` is a benign
`exit.store(true)` (`application_api.rs:119-126`), but `nativeApplication.close()` →
`application_shutdown` → `app_ptr.to_owned::<Application>()` takes ownership, so a second call is a
use-after-free. `AutoCloseable.use` calls `close()` once, but `initApplication()` is public and a caller can
call `close()` directly, and finding 1's fix adds another caller. Add an `AtomicBoolean isClosed` guard, and
skip `stopEventLoop()` when `!eventLoopThread.isAlive`. **Confidence: high. Risk: none.**

---

### 11. The preedit caret range (`cursorBeginBytePos`/`cursorEndBytePos`) is dropped — **P3, probably not portable today**

`Event.TextInput.preeditStringData` carries `cursorBeginBytePos` and `cursorEndBytePos`
(`kotlin-desktop-toolkit/.../linux/TextInput.kt:3-13`). `TARGET linux/LinuxTextInput.kt:139-144` ignores both
and always calls `setComposingText(preeditText, 1)`, i.e. caret pinned after the whole composition. Noria
converts them to a UTF-16 `TextRange` and passes it through
(`SOURCE linux/LinuxTextInputSessionOwner.kt:98-109`):

```kotlin
val start = utf8OffsetToUtf16Offset(preeditString, preeditStringData.cursorBeginBytePos.toUInt())
val end = ...
TextRange(start = start.toInt(), end = end.toInt())
```

**Why the target cannot simply copy it:** I checked `TextEditingScope` in the Compose version the target
builds against (`org.jetbrains.compose.ui:ui-text-desktop:1.11.1`,
`skikoMain/androidx/compose/ui/text/input/PlatformTextInputService2.kt`). It exposes exactly four methods —
`deleteSurroundingTextInCodePoints`, `commitText`, `setComposingText`, `finishComposingText` — with **no**
`setSelection` and no `setComposingRegion`. `setComposingText`'s `newCursorPosition` can only express "at the
start" (`0`) or "just after" (`1`); an interior caret is inexpressible. Noria only manages it because its
`PlatformTextInputMethodRequestLinux` is its own interface (`SOURCE ../LinuxInputMethodCommon.kt:70-82`).

**Recommendation:** keep the `1`, but consider passing `0` when `cursorBeginBytePos == 0`, which is at least
correct for IMEs that park the caret at the start of the composition. Track it as an upstream Compose
request (`TextEditingScope.setSelection`) rather than a port. **Confidence: high** that it is blocked.

---

## Noria files / capabilities with no target counterpart

| Noria file | What it gives | Does Compose need it? |
|---|---|---|
| `linux/LinuxNotificationCenter.kt` (+ `Event.NotificationShown/Closed`, `Application.requestShowNotification/closeNotification`) | Desktop notifications with actions and activation tokens; a notification click raises the right window | **No** for parity with Compose Desktop (which has no notification API), but the KDT API exists and this is the only consumer of `Event.NotificationShown/Closed` — the target currently drops both events into its `else ->` branch. Worth an optional extension module later. |
| Primary selection / middle-click paste — `linux/LinuxClipboard.kt:72-115, 148-166, 207-215`, gated on `DesktopSetting.MiddleClickPaste` | Middle-click paste, `systemSelection()` / `setSystemSelection()`, `primarySelectionPaste/Put` | **No** — `androidx.compose.ui.platform.Clipboard` has no primary-selection concept; `TARGET linux/LinuxClipboard.kt:35-37` documents this decision and it is the right one. Note that the target *does* correctly return `null` for `DataSource.PrimarySelection` in `getDataTransferData` (`TARGET linux/LinuxApplication.kt:445`), so nothing leaks. |
| `linux/Dispatchers.kt` (`LinuxKdtMainDispatcher`) | Per-backend `Dispatchers.Main` | **Already covered** — the target's `KdtDispatcher` (`TARGET ../KdtDispatcher.kt`) is backend-agnostic and strictly better; the separate `kotlin-desktop-toolkit-compose-dispatcher` artifact registers it as `Dispatchers.Main`. |
| `linux/LinuxUriHandler.kt` | `openURL` on the event loop | **Already covered** by the anonymous `uriHandler` in `TARGET linux/LinuxApplication.kt:124-131`, which is identical (both pass `activationToken = null`). |
| `linux/ViewConfiguration.kt` | Standalone `ViewConfiguration` | **No** — dead code in Noria too (`LinuxWindow` defines its own inline). The target's inline version (`TARGET linux/LinuxWindow.kt:234-239`) is better: it reads the real `doubleClickIntervalMillis`. |
| `PointerIconService.pushHide()` / `popHide()` (`SOURCE linux/PointerIcon.kt:36-48`) | Depth-counted cursor hiding, e.g. while a modal drag is in progress | **Maybe.** `TARGET linux/LinuxPointerIcon.kt` has a two-level arbitration (Compose shape / override / hidden-until-move) that already covers the resize-border case; there is no Compose API that would call push/pop. Skip unless a need appears. Related fix `191cb30ea2ecf` *AIR-5280 do not hide pointer forever on window reuse* is window-reuse-only and out of scope. |
| `RenderLoopRegistry` (`bdd1536136965`, AIR-5645) | Synchronised list of render loops | **No** — Noria-specific; the target has no render-loop list. The general lesson (don't mutate application-owned collections off the event loop) already holds: the target's `windowsById`/`windowList` are only touched from the event loop thread, since the composition runs on `KdtDispatcher`. |
| `LinuxDragAndDropClipboardEntry` seeded with `WindowLocalDragMimeType` (`SOURCE ../MimeTransfer.jvm.kt:37-39`) and `ClipboardFormat.WindowLocalDrag` / `CustomSerializable` | Fleet's own window-to-window tab drags | **No** — Fleet-specific; `ClipboardFormat` in the target has no such cases. |
| `AIR-6619` (`4b636801ab9ad`) IME activate/deactivate + drop composed marker on focus loss | — | **macOS only.** The commit touches only `macos/MacOsTextInputSessionOwner.kt`. The Linux equivalent (disable IME and discard the preedit when text input goes away, or when a click lands during composition) is **already in the target**: `TARGET linux/LinuxApplication.kt:377-388` and `:425-431`, matching `SOURCE linux/LinuxApplication.kt:499-511` and `:535-558` line for line, comment for comment. |
| `AIR-5558` (`79a633bee12d2`) new KDT clipboard with async retries | — | **Windows only.** The commit touches only `windows/WindowsClipboard.kt`. The Linux clipboard's async story (suspend + serial-keyed pending reads) is the same in both, and the target's version is slightly safer — see below. |
| `AIR-6275` (`5f9027c853231`), `AIR-5776` (`4fcb3bcfad208`), `AIR-5478` (`ec1a8f6c3b69d`) | — | **macOS / Windows only.** Verified by `git show --stat`. AIR-5478's *principle* is captured as finding 10. |

---

## Divergences I judged NOT worth porting

- **JVM shutdown hook.** Noria used to install one (`JvmApplicationRegistry.installHook`) and **deleted it** in
  `e2ba99e694520` *AIR-6701 the order of execution of shutdown hooks are not specified, so it was a race
  condition*. The target has never had one. Correct as is — do not add one.
- **`Key` mappings.** Noria maps `quotedbl → Key.QuotationMark`, `underscore → Key.Underscore`,
  `colon → Key.Colon`, `bar → Key.VerticalLine`, etc. (`SOURCE linux/Key.kt:144-177`), which only compiles
  against its extended internal `Key` enum. The target's folding onto the physical US-layout key
  (`TARGET linux/LinuxKeys.kt:86-108`) is the right call for the published `Key`, and it is *more* complete
  than Noria in places: Noria has no mapping for `percent`, `question`, `less`/`greater`,
  `asciicircum`-as-Six, `dollar`, `exclam`-as-One — those all fall to `Key.Unknown` there, while the target
  folds them correctly. No genuine mapping bug found in either direction. (Both are equally silent on
  F13-F24, which have no published `Key`.)
- **`InputStateTracker` return values.** Noria returns `Stop`/`Continue` from `ProcessResult`
  (`anyChangeConsumed`, `anyPressOrReleaseConsumed`, `anyScrollingConsumed`); the target returns `Stop`
  unconditionally for pointer events (`TARGET linux/LinuxInputStateTracker.kt:114, 132, 141, 148, 164, 182`).
  `CanvasLayersComposeScene.sendPointerEvent` returns `Unit`, so the target has no consumption signal to
  report. Not portable, and the practical difference is nil: KDT's own fallback handling for pointer events
  is empty.
- **Drag-and-drop `action` during the hover query.** Noria always passes `action = null` into the hover
  `DragAndDropEvent` (`SOURCE linux/LinuxDragAndDropManager.kt:121-129`), which makes its own
  `if (event.action != previousAction) onChanged(event)` dead code. The target computes
  `query.actions.preferredComposeAction()` (`TARGET linux/LinuxDragAndDropManager.kt:86-89, 165-169`) and
  therefore actually fires `onChanged` — **the target is ahead here**; do not regress it.
- **`preferredAction` nullability and `single()` vs `singleOrNull()`.** Noria emits
  `SupportedActionsForMime(preferredAction = linuxActions.firstOrNull())` including empty action sets, and
  calls `mimeTypes.single()` in `getForFormatSync` (`SOURCE ../MimeTransfer.jvm.kt:47`), which throws if the
  compositor ever delivers zero or multiple types. The target filters empty action sets
  (`TARGET linux/LinuxDragAndDropManager.kt:116-127`) and uses `singleOrNull()`
  (`TARGET linux/LinuxDragAndDropManager.kt:43`). **Target is safer**; keep.
- **`handleFileChooserResponse` returning `Stop` unconditionally** (`SOURCE linux/LinuxWindow.kt:590-593`).
  `Event.FileChooserResponse` carries no window id, so Noria's first window swallows every response,
  including other windows'. The target returns `Continue` when the request id is not its own
  (`TARGET linux/LinuxWindow.kt:600-609`). **Target is correct**; keep.
- **Clipboard `check(mimeTypes.contains(content.mimeType))`** (`SOURCE linux/LinuxClipboard.kt:43`). A hard
  assert on the event loop thread against data another application controls. The target does not assert and
  decodes with the format it asked for. **Target is safer**; keep.
- **`LinuxKeys.kt` / `LinuxConverters.kt` / `LinuxInputStateTracker.kt` duplication with `gtk/`.** Byte-identical
  modulo naming, but they reference `org.jetbrains.desktop.linux.*` vs `org.jetbrains.desktop.gtk.*` types
  (`KeySym`, `LogicalPixels`, `MouseButton`, `TextInputContentPurpose` are four distinct pairs of unrelated
  classes). Noria hit the same wall and settled for `@file:Suppress("DuplicatedCode")`. Unifying would need
  either a typealias layer in KDT itself or generics over the KDT packages — out of proportion. The genuinely
  shareable slice is finding 8.
- **Window reuse / pooling, `withPreparedThread`/`withPreparedMainThread` (`34fd99eb6410d`, AIR-6419), the
  Noria scene/frame domain and `DataSource` machinery, `CallbackInterceptor`.** Out of scope per the brief,
  and no independently useful fix found inside them. One note: `resetState()`'s comment
  (`SOURCE linux/LinuxApplication.kt:794-798`) — "Linux window IDs must stay monotonic for the lifetime of the
  process" — is already how the target behaves (`nextWindowId` is a process-wide `AtomicLong`,
  `TARGET linux/LinuxApplication.kt:89`).
- **Noria's `ScrollData.isInverted` handling** — neither side reads it. Not a Noria-derived finding, but flagging
  it: the target's KDT exposes `ScrollData.isInverted` and `isStop`
  (`kotlin-desktop-toolkit/.../linux/Event.kt:130-137`) and `TARGET linux/LinuxInputStateTracker.kt:352-365`
  ignores both, so "natural scrolling" configured at the libinput level may come through un-inverted. Worth
  a separate investigation against a real device; out of scope for a Noria comparison since Noria has the
  same gap.

---

## Appendix: Noria Linux commits already absorbed by the target

For the record, so nobody re-derives them:

| Commit | Ticket / title | Where it already lives in the target |
|---|---|---|
| `a2f0ed4f328a0` | AIR-5322 Fix constant recompositions on Linux Wayland (`isFrameRequested` guard in `WindowDraw`) | `linux/LinuxWindow.kt:552-561` |
| `1ea29f3ceaa6a` | AIR-5233 Fix slow scroll on Linux (wheel120 → 100 dp, smooth delta × 15) | `linux/LinuxInputStateTracker.kt:352-365` |
| `a662747de3f0a` | AIR-5621 Horizontal scrolling with Shift+Scroll | `linux/LinuxInputStateTracker.kt:364` |
| `cc557a8bf5daa` | Improve keyboard modifiers tracking on Linux (`mouseFocusWindowId`, modifiers to both windows, `hasKeyboardFocus`) | `linux/LinuxApplication.kt:145-146, 243-249`; `linux/LinuxInputStateTracker.kt:82, 151-165, 225-238` |
| `d80de585f4880` | AIR-5571 Fix window stuck resizing on KDE Wayland (clear buttons on `MouseExited`, not on drag start) | `linux/LinuxInputStateTracker.kt:151-165` — including the same rationale in the comment |
| `aec57125a9942` | AIR-5008 Clear pressed button on drag start | superseded by AIR-5571 above; the target's comment at `linux/LinuxWindow.kt:753-754` records why |
| `5a0556cd5a949` | AIR-4948 Don't insert characters when modifiers are held (Ctrl→ISO control, `0x40..0x7E`, AltGr exempt) | `linux/LinuxInputStateTracker.kt:340-343`; `linux/LinuxTextInput.kt:166-175` |
| `369319fadacfe` | AIR-5610/5611 Enable IME on Linux (seat-wide arming, `lastTextInputContexts`, discard preedit on click) | `linux/LinuxApplication.kt:371-431`; `linux/LinuxTextInput.kt` |
| `204628d9f04cd` | AIR-5076 Don't prematurely drop the pending clipboard request; tolerate `content == null` | `linux/LinuxClipboard.kt:75-79, 100-109` |
| `556965e4530d7` / `2e62048894166` / `bd7c330b16815` | Linux clipboard + drag-and-drop rework (`getDataForLinuxMimeType`, per-format MIME alias lists, `acceptsFormat`) | `LinuxMimeTypes.kt`; `linux/LinuxDragAndDropManager.kt:31-67` |
| `071d54e4fc489` + `610479a34d711` | AIR-5862 Blurry drag images / even-pixel rounding on macOS only | `DragAndDropImage.kt:22-36` — including the AIR-5862 reference in the doc comment |
| `fe5fa7e1834a2` | AIR-4578 (the KDT-API half: `rawPhysical`, `LogicalPixels`/`LogicalPixelsInt`, `startMove(serial)`, `startResize(serial, edge)`, `requestActivationToken`) | `linux/LinuxConverters.kt`; `linux/LinuxWindow.kt:439-442, 580-587, 779-821`. **The frame/shadow half is finding 4.** |
| `b049c1dd69ba9` | Make loggers lazy | N/A — the target logs through KDT's `Logger` objects; no eager `logger<T>()` fields exist. |
