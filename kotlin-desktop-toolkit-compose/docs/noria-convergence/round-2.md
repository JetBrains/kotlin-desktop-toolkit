# Noria → KDT-Compose convergence, round 2

A second five-way comparison, run against `compose@0bba6184` and Noria at
`ultimate2@b009147fffe74` (2026-08-28). The five agents worked without sight of the round-1
reports, so where they land on the same finding it is an independent confirmation, and where
they contradict round 1 the reason is recorded below.

Round 1 closed all three P0s and most P1s (WS1–WS8, WS11). What follows is what round 2 found
*on top of* that: items round 1 never recorded, items it under-rated, and two defects in code
round 1 itself landed.

## The headline

**On three of four backends this module is now ahead of Noria**, and round 2 says so more
strongly than round 1 did. macOS: the frame pump, display-link handling, GPU context, IME client
and input tracker are all ahead. Windows: ahead on ANGLE lifetime, resize bookkeeping, pointer-icon
restore, IMM32 candidate-window refresh, and the `DataTransferException` path. Linux: ahead on
pointer-icon arbitration, drag guards, and `currentLinuxWindowSystem()`. GTK: ahead on the cached
drag-icon `DirectContext`, zero-size draw guards, and file dialogs (Noria's are `TODO`).

Where Noria is genuinely ahead it is now in **surface, not structure**: notifications, a custom
clipboard format, screenshots, window/dock icons, a headless backend, and richer file dialogs.
The shared layer's architecture needs no correction — see *Architecture, re-affirmed*.

## Tier 1 — verified, mechanical, no API decision

Each was confirmed by reading the code, not just by an agent's report.

1. **Stack traces are discarded at 9 log sites.** `Logger.error { "…: $throwable" }` prints the
   exception's type and message and drops every frame. Sites: `macos/MacOsGpuContext.kt:83`,
   `macos/MacOsApplication.kt:158,203`, `macos/DisplayLinkFramePump.kt:52,57`,
   `macos/MacOsWindow.kt:786,831`, `linux/LinuxApplication.kt:187,326`. These are the catch sites
   around Skia, Metal and the event loop, where the stack is the only diagnostic.
   `Logger.error(t: Throwable) { }` already exists. GTK and Windows were fixed in round 1
   (`gtk.md` 6); macOS and Wayland were never given the same treatment.

2. **The canvas is cleared to opaque white on macOS and Wayland.** `macos/MacOsWindow.kt:809`
   (`Color.White`), `linux/LinuxWindow.kt:820,843` (`SkiaColor.WHITE`). GTK
   (`GtkWindow.kt:781`, `GtkApplication.kt:477`), Windows (`WindowsGpuContext.kt:75`) and
   Wayland's *own* drag-icon painter (`LinuxApplication.kt:497`) all clear `TRANSPARENT`, as does
   Noria. Costs a white flash on the first frame and in dark mode, and makes a translucent or
   round-cornered window impossible. It is also a hard prerequisite for WS9's shadow padding.
   Round 1 filed this as macOS-only P3; it is cross-backend and blocks other work.

3. **Windows never received WS11.** `eventLoopFailure` is present on three backends
   (`macos:133,157,368`, `linux:105,186,636`, `gtk:110,198,598`) and absent on Windows, which only
   logs and then counts down `applicationStarted` in its `finally`
   (`windows/WindowsApplication.kt:139-145`). The constructor therefore returns a live-looking
   application whose loop thread has already run `nativeApplication.close()`; the first
   `createWindow` dies with "App has not been initialized yet" and the real cause is gone. The GTK
   sibling is a verbatim template.

4. **The startup log line bypasses the caller's appender.** `Application.kt:175` runs
   `logInfo(backend) { startupInfo(backend) }` *before* `initApplication` at `:176` installs the
   appender, so a host passing `consoleLogLevel = Off` or its own `logAppender` still gets
   "Starting Compose application on …" on stderr and never sees it in its own log — the one line
   most worth having, and the only one that cannot reach it. Emit it from the end of
   `initToolkit` (`BackendLogger.kt:83`) so `initApplication`-only callers get it too.

5. **A throwing state observer permanently silences `KdtSnapshotManager`.**
   `KdtSnapshotManager.kt:41-47` calls `Snapshot.sendApplyNotifications()` inside `consumeEach` on
   a standalone `CoroutineScope(KdtDispatcher)`. If any global write observer throws, that
   coroutine dies and every subsequent out-of-composition state write stops invalidating anything,
   silently. Note the fix is a try/catch around the call — **not** a `SupervisorJob`, which does
   nothing for a scope with one child.

6. **`MouseUp` does not restore `isPointerInWindow`** — `linux/LinuxInputStateTracker.kt:117-133`
   and `gtk/GtkInputStateTracker.kt:119-136`, while `MouseDown` and `MouseMoved` both do
   (`linux:99,139`, `gtk:102,142`). After a compositor grab leaves the flag false via
   `MouseExited`, a release returning to the surface without a fresh `MouseEntered` leaves hover
   state stale until the pointer moves. Noria sets it (`linux/InputStateTracker.kt:120`).

7. **GTK drops `Exit` when it already believes the pointer is outside.**
   `gtk/GtkInputStateTracker.kt:158` sends Exit through a path that early-returns at `:321` on
   `!isPointerInWindow`. Noria sets `pointerInWindow = true` immediately before sending
   (`gtk/InputStateTracker.kt:180`) precisely so the Exit always gets out. Same bug as 6, other half.

8. **`asterisk` and `numbersign` break the US-layout folding rule the file documents.**
   `gtk/GtkKeys.kt:97,99` map them to `Key.Multiply` (the *numpad* multiply) and `Key.Pound` (£),
   against the rule stated at `:19-31` and applied two lines later (`exclam -> Key.One`,
   `dollar -> Key.Four`, …). `*` is Shift+8 and `#` is Shift+3, so they should fold to `Key.Eight`
   and `Key.Three` — which is what `windows/WindowsKeys.kt:24,29` reports for the same chords, so a
   Ctrl+Shift+8 binding works on Windows and not on GTK. `linux/LinuxKeys.kt` is byte-identical:
   fix both. This is a target-only bug, not a port — Noria uses its extended enum here.

9. **GTK clipboard reads are not checked against the MIME type that came back.**
   `gtk/GtkClipboard.kt:110` decodes whatever arrives as the requested format. Noria guards
   (`gtk/GtkClipboard.kt:43-45`). Return `emptyList()` rather than Noria's `check` — an exception
   here lands inside a Compose paste handler.

10. **`WindowsTextInput.isSessionActive` is dead** (`windows/WindowsTextInput.kt:53`);
    `offerKeyDownToInputMethod` tests `activeClient == null` directly at `:106`. macOS, Linux and
    GTK all use theirs. Wire it up or delete it.

11. **`maven-publish` and `explicitApi()` are still missing** from
    `kotlin-desktop-toolkit-compose/build.gradle.kts`; the base module has both
    (`kotlin-desktop-toolkit/build.gradle.kts:50,89`). Round 1 filed this as WS10 #20 and it never
    landed.

    **Deferred on request, 2026-09-01** — these modules are not to be published yet, so both were
    implemented and then dropped rather than kept on the branch. Two notes for whoever picks this up:
    `explicitApi()` wants 189 declarations annotated (183 visibilities, six return types), all
    mechanical and none of them an API change, since every one was already public by default. Doing
    it does surface one thing worth deciding first: `GpuContext`, `MetalViewContext`,
    `PresentablePicture` and `CallbackReference` in `macos/MacOsGpuContext.kt` are public API, though
    nothing outside `macos/` refers to them and they are Skia and Metal plumbing no consumer can use.
    Narrow them before writing `public` on them, not after.

## Tier 2 — correctness, needs a little judgement

12. **The Linux/GTK IME violates two steps of KDT's own six-step protocol**
    (`kotlin-desktop-toolkit/.../gtk/Event.kt:250-257`).
    - *Step 1 skipped:* the existing preedit is not cleared before `delete_surrounding`
      (`gtk/GtkTextInput.kt:152-176` goes delete → commit → setComposing, and its KDoc at
      `:145-151` says so). An IME sending `delete_surrounding` while a preedit is on screen measures
      byte offsets against a buffer that still contains it, and deletes the wrong characters.
      Fix: capture `hasPreeditString` before `:157`, prepend `setComposingText("", 1)` in the same
      `editText { }` block — the same call `discardActivePreedit()` already makes at `:184-187`.
    - *Step 6 dropped:* `cursorBytePos` is never read, so `setComposingText(preeditText, 1)` always
      parks the caret after the whole composition. CJK and Korean two-set IMEs put it mid-preedit
      to mark the segment under conversion, so the caret visibly jumps to the end.
      Noria's helper is fork-internal; on released Compose the equivalent is
      `setComposingText(text, newCursorPosition = -(text.length - caretUtf16))`. Needs a
      UTF-8-byte → UTF-16 converter next to `codePointsBefore` in `LinuxInputMethodCommon.kt`, so
      `linux/LinuxTextInput.kt:122-147` — the same gap — gets it too.
    **Unverified without a CJK IME**, like the round-1 `setMarkedText` fix.

13. **`close()` leaves windows undisposed on Linux, GTK and macOS.** Their `ComposeScene`s and
    Skia `DirectContext`s are dropped on the floor and the registries keep the entries
    (`linux/LinuxApplication.kt:619-628`, `gtk/GtkApplication.kt:582-590`,
    `macos/MacOsApplication.kt:349-359`). Noria disposes first, and GTK additionally *awaits*
    each window's native close before stopping the loop (`gtk/GtkApplication.kt:595-606`, backed by
    a per-window `CompletableDeferred`) — without that, `dispose()`'s posted `nativeWindow.close()`
    is a `glib::idle_add_once` the stop can drop. Harmless at process exit; a per-window GL context
    leak per cycle for a host that starts and stops an application inside a longer-lived JVM, which
    is exactly what the headless tests in 18 would do.

14. **The `terminationInProgress` guard is a non-atomic check-then-set on all four backends**
    (`macos:81,350,353`, `linux:96,620,623`, `gtk:103,583,586`, `windows:80,280,283`). Round 1's
    `adb5183d` fixed the sequential double-`close()` it was aimed at, but two concurrent calls can
    both pass. Noria uses `AtomicBoolean.compareAndSet` — the AIR-5478 / AIR-6896 lesson. Also on
    Windows: if the loop thread already exited on its own, `invokeOnDispatcher` throws out of
    `close()`, i.e. out of `use { }`, masking whatever actually killed the application.

15. **The synthetic Move before a Press carries stale modifiers** (Windows).
    `windows/WindowsInputStateTracker.kt:101-104` calls `movePointerTo` — which reads the
    `keyboardModifiers` field at `:285` — *before* refreshing it. On a Ctrl+click the Move that
    Compose sees immediately before the Press reports no Ctrl. The `PointerUpdated` branch at
    `:142-143` already refreshes first, so Down/Up are inconsistent with their own sibling.
    Latent in Noria too, which refreshes in the right order at `windows/InputStateTracker.kt:71-80`.

16. **A failed constructor leaves the toolkit initialized and unreachable.**
    `Application.kt:251-263` registers *after* construction, but the backend constructors call
    `initToolkit` and start the loop in their `init` blocks, so a constructor that throws (e.g.
    `linux/LinuxApplication.kt:637`) leaves the native library initialized and the loop thread
    possibly alive, with nothing registered and no `close()` reachable. A retry then meets
    `KotlinDesktopToolkit.init`'s once-per-process behaviour with no way to observe it. Wrap the
    construction in `runCatching` and close on failure, or register before starting the loop.

17. **The Wayland resize border eats the outer 6 dp of content.**
    `linux/LinuxWindowBorders.kt:21-53` hit-tests *inside* the surface and
    `linux/LinuxWindow.kt:607-615` consumes the press before the tracker sees it. With
    `clientSideDecorationFrame` zeroed, that strip is content: on GNOME, which is always CSD, a
    click on an edge-flush scrollbar, a bottom list row or a top tab strip starts a window resize,
    and `updateResizeBorderPointerShape` (`:724-732`) overrides the content's hover cursor there.
    This falls out of WS9 — once the frame has a real `resizerThickness`, KDT extends the input
    region *outside* the geometry and `LinuxWindowBorders.kt` can be deleted entirely. If WS9 stays
    deferred, the cheap partial fix is to arm the border only when the *requested* decoration was
    undecorated, rather than on every `WindowDecorationMode.Client`.

## Tier 3 — new public API, each a design decision

Grouped because they share one question: what shape should an optional platform capability take on
a library that mirrors Compose Desktop? Round 1 settled the precedent —
`handlesCustomTitleBarGestures` (`Window.kt:140`) pairs an optional member with a capability flag,
and `ComposeLogAppender` puts one interface in the shared package with four internal adapters.
These should follow it rather than Noria's capability sub-interfaces.

18. **Headless backend and a test source set.** The module has zero tests and nothing in it is
    testable without a display and a real GPU context. Every P1 round 1 shipped is what a headless
    smoke test would have caught. Noria's `headless/HeadlessEventLoopJvm.kt` is self-contained
    (a `LinkedBlockingQueue` on a dedicated thread) and ports close to verbatim. **The target is
    better positioned than Noria here:** because `ComposeApplication` already exposes
    `invokeOnUiThread`/`isUiThread` (`Application.kt:83-86`), a `HeadlessApplication` works with the
    existing `KdtDispatcher` unchanged — Noria needs its `overridingMainDispatcher` hook
    (`KdtMainDispatcherFactory.kt:14-19`) only because its four dispatchers each bind a concrete
    native API. **Do not port that hook.** Scope to a `HeadlessApplication` + queue loop first;
    add `HeadlessWindow` only when a scene test needs one, and build it on
    `CanvasLayersComposeScene`/`ImageComposeScene`, not Noria's `Scene<*>`.
    First tests to write: `decodeFileChooserPath`, `LinuxMimeTypes` round-trips,
    `LinuxTextInputSurroundingText` byte↔codepoint arithmetic, `ApplicationExit` reentrancy, and
    the `Window` composable's abandon/forget disposal path.

19. **Notifications.** `macos/MacOsNotificationCenter.kt` (243 lines),
    `linux/LinuxNotificationCenter.kt` and `gtk/GtkNotificationCenter.kt` (120 each). The target
    ignores `NotificationShown`/`NotificationClosed` entirely
    (`linux/LinuxApplication.kt:648-660`). The four native APIs diverge — macOS has authorization
    and categories, Linux/GTK a request/response id protocol, Windows nothing — so this wants a
    `ComposeNotificationCenter` interface plus per-backend impls with `isSupported` false on
    Windows. Keep Noria's rule that macOS returns `null` when
    `NotificationCenter.isSupportedByApplication` is false (running unbundled): AIR-5002 exists
    because an earlier version threw there.

20. **A custom clipboard format.** `ClipboardFormat` is closed to
    `Utf8PlainText`/`Html`/`Png`/`File` (`Clipboard.kt:29-34`), so any app-specific drag payload —
    a tree node, an editor selection, a tab — round-trips through `Utf8PlainText` and becomes
    indistinguishable from a plain text drag out of another app, so drop targets cannot filter on
    it. Noria added `CustomSerializable`. Prefer `Custom(mimeType)` over `ByteArray` to avoid
    pulling in kotlinx.serialization. Source-breaking for exhaustive `when`s in consumer code,
    which is acceptable pre-1.0.

21. **macOS main menu.** `AppMenuManager` is already imported and called on every keypress
    (`macos/MacOsWindow.kt:65,657` — `offerCurrentEvent()` exists to blink the matching item), so
    the module pays for the integration and no consumer can install a menu for it to find. Without
    one there are no Cmd-Q/Cmd-H/Services entries beyond AppKit's bare default. The largest
    functional hole on macOS.

22. **Window and dock icons.** `setIcon` exists in KDT (`win32/Window.kt:172`, macOS
    `setDockIcon`) and is unreachable. The window shows the default Windows icon in its titlebar,
    taskbar button and Alt+Tab. Per-window on Windows, per-application on macOS.

23. **Richer macOS file dialogs.** The target exposes `title`, `allowsMultipleSelections`,
    `canChooseFiles`, `canChooseDirectories`, `nameFieldStringValue`
    (`macos/MacOsWindow.kt:479-513`); Noria also passes `prompt`, `message`, `nameFieldLabel`,
    `directoryPath`, `canCreateDirectories`, `canSelectHiddenExtensions`, `showsHiddenFiles`,
    `isExtensionHidden`, `resolvesAliases` (`:478-574`). No initial directory is the painful one.
    Carry over the trap from Noria's `b2a4d7b9115a4`: pass `directoryPath?.toString()`, never
    `"$directoryPath"`. Keep the target's `KdtDispatcher` hop at `:485-489`, which Noria lacks.

24. **`captureScreenshot()`.** Absent from `ComposeWindow`. Each backend owns its Skia surface, so
    it is a `Surface.makeImageSnapshot()` per backend, but it must be sequenced against the frame
    pump rather than read at an arbitrary moment. The enabling primitive for visual regression
    tests, so it pairs with 18.

25. **`suspend fun awaitApplication`.** Compose Desktop ships both `application {}` and
    `awaitApplication {}`; this module has only the blocking one (`Application.kt:170-188`, with
    `runBlocking` at `:276`). A caller who already has a scope must burn a thread and cannot cancel
    by cancelling a job. The wrinkle Noria does not have: the backend constructors block on a
    `CountDownLatch`, so boot stays blocking unless that changes too.

26. **Wayland titlebar button layout and `WindowCapabilities`.** Round 1 landed the titlebar
    *actions* (`fd4d0c35`) but not the *layout*: `DesktopSetting.TitlebarLayout` falls into
    `else -> {}` (`linux/LinuxApplication.kt:331-369`), so on a desktop configured with the close
    button on the left every window drawn on this library gets it wrong. `event.capabilities` is
    likewise discarded at `:652-669`, so on a tiling compositor a custom titlebar draws
    maximize/minimize buttons that silently do nothing. Both need the same new public surface,
    which is why round 1 left `WindowCapabilities` undone; `CaptionButtonKind` (`Window.kt:61`) is
    the natural seed but lacks AppMenu/Icon/Spacer.

27. **Primary selection (middle-click paste).** Deliberately absent and documented as such
    (`linux/LinuxClipboard.kt:35-36`), because released `Clipboard` has no member for it. Middle-click
    paste is a baseline Linux expectation and a text editor built on this library will not have it.
    Worth deciding explicitly — as a Linux-specific API — rather than by omission.

28. **`startInteractiveResize`.** `Window.kt:231` declares only `startInteractiveMove`. Content
    drawing its own frame (a corner-only grip, a horizontally-resizable dialog) cannot ask.
    Note round 1's shared report called the *absence* the better library design; round 2's Linux
    report calls it a gap. Both are right about different things: hiding the resize border is good,
    but it should not also hide the gesture. The mapping already exists at
    `linux/LinuxWindowBorders.kt:55-63`.

## Blocked upstream — do not attempt

Recorded so they are not re-diagnosed. All three were reached independently by two or more agents.

- **The synthetic `Move` after every scroll** (`macos:196`, `windows:268`, `gtk:188`) is
  unconditional; Noria gates it on `processResult.anyScrollingConsumed`. Released
  `ComposeScene.sendPointerEvent` returns `Unit` and `PointerEventResult` is internal to compose-ui.
  Cost only — a second hit test per wheel notch over non-scrollable content — not a correctness bug.
  The same limitation is already documented at `MacOsInputStateTracker.kt:66-74`.
- **Phantom `Key.Button1/2/3` on consumed clicks** (`windows:116,136`, `gtk:115,133`). Noria gates
  on `anyPressOrReleaseConsumed`, unreadable here for the same reason. A click a widget already
  handled — or one in the titlebar — injects a keystroke into the focused key chain, which any
  catch-all `onPreviewKeyEvent` or shortcut table sees. **A real bug with no clean fix**; the
  tractable half is narrowing the synthetic key to `XButton1`/`XButton2`, which is all the comment
  at `WindowsInputStateTracker.kt:115` claims to want.
- **AIR-6291's second half.** The `InputStateTracker` half is ported; the other half is a change
  inside `androidx.compose.ui.input.pointer.HitPathTracker`, where an OS Exit is re-classified as a
  Move whenever the exit pixel still falls geometrically inside a node. A node spanning the window
  edge can therefore still keep a stale I-beam after the cursor leaves. Worth a comment beside the
  ported half, and an upstream ticket.
- **Drag decoration custom size (AIR-5140).** Noria added `dragDecorationSize` to its own fork of
  `DragAndDropTransferData`; released Compose has no such field. Same class of blocker as the
  drag-threshold note in `146a375b`.

## Architecture, re-affirmed

Round 1's verdict stands and round 2 strengthens it. Twelve Noria files
(`Dispatchers.kt`, `ViewConfiguration.kt`, `UriHandler.kt` × 4) exist only because Noria's
`Application` interface stops short of an `invokeOnUiThread`/`isUiThread` abstraction; this module
added those two members and collapsed all twelve into one `KdtDispatcher` plus inlined members,
with no capability lost and one gained — it reads the desktop's real double-click interval
(`linux/LinuxApplication.kt:142`) where Noria hard-codes a constant. The same pattern holds in
drag-and-drop (a `DragAndDropSourceWindow` interface against Noria's exhaustive `when (window)`
with an `error()` fallback) and in logging (`ComposeLogAppender` + four adapters against Noria's
single macOS-only adapter, still sitting in a stale `androidx.compose.ui.kdt` package).

Three small correctness fixes flow the *other* way, worth offering upstream to Noria:
`decodeFileUriList` filters `#` comment lines (`LinuxMimeTypes.kt:90`) where Noria's does not;
`LinuxTextInputSurroundingText` (`LinuxInputMethodCommon.kt:29-54`) walks code points where Noria's
`convertByteOffsetsFromCursor` does `max(0U, cursorOffset - beforeLengthInBytes)` on a `UInt` and
wraps on underflow; and Noria's `updateModifierState` builds synthetic modifier `KeyEvent`s from
the *previous* mask where this module uses the new one, which is what matches AWT.

Two Noria changes would regress this module if ported, in addition to round 1's three: its
`GpuContext.close()` never closes the ANGLE renderer (a device leak per window), and its
`VirtualKey.Clear -> Key.NumLock` mapping is wrong (`VK_CLEAR` is numpad-5).

**Do not reintroduce a shutdown hook.** Noria deleted `installHook()` in `e2ba99e6` (AIR-6701)
because shutdown-hook ordering is unspecified and it was a race. This module is already at the
endpoint Noria reached the hard way; if "the app doesn't clean up on Ctrl-C" comes up, that is the
evidence.

## One open question on the porting principle

Round 1's stated principle is *"resemble Noria as closely as the difference between the two
runtimes allows"* — keep its names and structure, including its duplication. Round 2's evidence
sits awkwardly with it: on the shared layer this module is consistently the better-factored of the
two, and three of round 2's tier-3 items (notifications, icons, custom clipboard formats) are
places where copying Noria's shape — capability sub-interfaces, `LightweightWindowId` payloads —
would be actively wrong. The principle earned its keep on the quit sequence, where matching names
made two implementations reviewable side by side. It should probably be narrowed to *behaviour and
naming where the concept is shared*, rather than structure, before the tier-3 work starts.
