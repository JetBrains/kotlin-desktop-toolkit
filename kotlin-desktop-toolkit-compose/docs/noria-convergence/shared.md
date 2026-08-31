# Shared layer & architecture: `kotlin-desktop-toolkit-compose` vs Noria's KDT integration

Read-only comparison, 2026-08-31.

- **TARGET** = `/Users/wojtek/IdeaProjects/kotlin-desktop-toolkit/kotlin-desktop-toolkit-compose/src/main/kotlin/org/jetbrains/desktop/compose/`
  (paths below are relative to that directory unless prefixed).
- **SOURCE** = `/Users/wojtek/IdeaProjects/ultimate2/fleet/noria/ui/`
  — `srcJvmMain/androidx/compose/ui/desktop/` (`SOURCE:jvm/…`) and
  `srcCommonMain/androidx/compose/ui/desktop/` (`SOURCE:common/…`).
  The brief listed only the JVM root files; the real shared layer of Noria lives in
  `srcCommonMain`, so both are covered.

Headline: **the target's shared layer is, on balance, the better-factored of the two.** Its
platform seam (`ComposeApplication` / `ComposeWindow` / `KdtDispatcher` /
`DragAndDropSourceWindow`) is genuinely narrower than Noria's, and several things Noria
duplicates four times the target already has as one file. What the target is missing is not
structure but **hardening** — six concrete correctness/robustness items Noria absorbed as bug
fixes — and **two capabilities** (headless backend, log appender) that matter specifically
because the target is a *library* and Noria is not.

---

## 1. Architecture comparison

### 1.1 Side by side

| Concern | TARGET | SOURCE (Noria) | Verdict |
|---|---|---|---|
| Platform selection | `DesktopBackend` enum (`MacOs`/`Wayland`/`Gtk`/`Windows`) + `DesktopBackend.default`, one `when` in `initApplication` (`Application.kt:209-221`). GTK vs Wayland is an explicit caller-visible choice; `GDK_BACKEND` respected. | `DesktopPlatform.Current` × `currentLinuxWindowSystem()` (`SOURCE:jvm/LinuxPlatform.kt:24-31`), repeated verbatim in `initializeJvmApplication`, `defaultUriHandlerJvm`, and `ComposeDispatcher.jvm.kt:34-42`. No `GDK_BACKEND` handling; anything without `WAYLAND_DISPLAY` is GTK. | **Target better.** One switch instead of three; the enum makes the Linux ambiguity a first-class API decision. |
| Main dispatcher | **One** `KdtDispatcher` / `KdtImmediateDispatcher` resolving the thread through `currentApplication().invokeOnUiThread` / `isUiThread()` (`KdtDispatcher.kt:19-48`). | **Four** near-identical dispatcher pairs (`SOURCE:jvm/{macos,linux,gtk,windows}/Dispatchers.kt`, ~50 lines each) plus a platform-selecting `ComposeDispatcher.jvm.kt` and `SOURCE:common/KdtMainDispatcherFactory.kt` delegating to it. | **Target much better.** ~200 lines of duplication avoided. Noria's copies also carry a latent bug — see §2.13. |
| `Dispatchers.Main` takeover | Opt-in via a separate artifact `kotlin-desktop-toolkit-compose-dispatcher` (`KdtMainDispatcherFactory.kt`), documented as process-wide. | Always on: `SOURCE:common/KdtMainDispatcherFactory.kt` is on Noria's classpath unconditionally, with a `@Volatile overridingMainDispatcher` escape hatch. | **Target better** for a library. |
| Drag-and-drop start | `KdtDragAndDropManager(window: DragAndDropSourceWindow)` — one shared class behind a 1-method interface (`KdtDragAndDropManager.kt:18`, `Window.kt:236-243`). | `KdtDragAndDropManager` type-switches on `is GtkWindow / is LinuxWindow / is MacOsWindow / is WindowsWindow` and casts `DragAndDropTransferData as DragAndDropTransferDataJvm` (`SOURCE:jvm/KdtDragAndDropManager.kt:56-82`). | **Target better.** |
| Drop-target capability negotiation | Shared `DragAndDropClipboardEntry` interface with `containsFormat`/`acceptsFormat` + `DragAndDropEvent` extensions (`DragAndDrop.kt:16-35`). | Per-platform `internal fun containsFormat/acceptsFormat` on four unrelated entry classes, no common type (`SOURCE:jvm/MimeTransfer.jvm.kt:56-68` for Linux, per-platform elsewhere). | **Target better.** |
| Linux MIME / file-URI code | Shared `LinuxMimeTypes.kt` (97 lines) + `FileChooserPaths.kt` — used by both Wayland and GTK. | Shared `SOURCE:jvm/MimeTransfer.jvm.kt` (160 lines). | **Parity.** Target's `decodeFileChooserPath` percent-decoding is a genuine extra (Noria has none). |
| Linux IME arithmetic | Duplicated: `SurroundingText`, `codePointsBefore/After`, `utf8Length`, `KeyboardType→ContentPurpose` exist twice, in `gtk/GtkTextInput.kt:278-334` and `linux/LinuxTextInput.kt:253-309`. | Hoisted into `SOURCE:jvm/LinuxInputMethodCommon.kt` (`utf8OffsetToUtf16Offset`, `LinuxTextInputSurroundingText`, `PlatformTextInputSessionLinux`). | **Noria better.** See finding 7. |
| Keysym→`Key` table | `gtk/GtkKeys.kt` and `linux/LinuxKeys.kt` are **byte-identical modulo the package token**, 187 lines each. | Same duplication (`SOURCE:jvm/gtk/Key.kt` 280 lines, `linux/Key.kt` 297). | **Both bad**; target's are exactly identical so the win is easier. |
| Text input session | Uses Compose's own machinery: `PlatformContext.startInputMethod` + Compose's internal `SessionMutex`; per-platform `*TextInput.kt` is only the protocol bridge. | Needs its own `TextInputSessionOwner` interface (`SOURCE:common/TextInputSessionOwner.kt:26-39`) and `PlatformTextInputSessionLinux` because Noria's scene has no equivalent. `SOURCE:jvm/ComposeTextInputSession.jvm.kt` is **74 lines of fully commented-out dead code**. | **Target better.** Do not port the session-owner shape. |
| Render-loop / frame pacing state | Per-window frame pumps; `macos/DisplayLinkFramePump.kt` already uses an `AtomicReference` latch. No application-level list. | Application-level `RenderLoopRegistry` (`SOURCE:common/RenderLoopRegistry.kt`) guarding a shared `mutableListOf<RenderLoop>`. | **Target better** (no shared list to race on). AIR-5645 has no analogue to port. |
| Shutdown | No JVM shutdown hook; `application()` uses `use {}` and each `close()` stops the loop and joins. | Also no hook — one was **removed** in AIR-6701. | **Parity**; target should keep it that way. |
| `Clipboard` shape | `ClipboardEntry`/`ClipboardItem`/`ClipboardElement`/builder — a near-verbatim match of Noria's, minus `WindowLocalDrag` and `CustomSerializable` (`Clipboard.kt` vs `SOURCE:common/Clipboard.kt`). | Same, plus those two formats. | **Parity**; `CustomSerializable` worth adopting (§3). |
| Window/Application interfaces | Single flat `ComposeWindow` with defaulted members (`Window.kt:79-219`). | Capability interfaces: `PositionAwareWindow`, `InteractiveMoveInitiator`, `InteractiveResizeInitiator`, `IconDecoratedWindow`, `CaptionButtonsHostWindow` (`SOURCE:common/Window.desktop.kt:217-276`). | **Judgement call.** Noria's split exists because a GTK window genuinely has no global position; the target papers over that with a `DpOffset` that is meaningless on Wayland. Not worth churning now, but it is the one place Noria's seam is arguably sharper. |

### 1.2 Verdict on seams worth adopting

Only three of Noria's seams are worth copying, and none of them are the big ones:

1. **`LinuxInputMethodCommon.kt`** — a shared Wayland/GTK IME file. Finding 7.
2. **`KdtLoggerAppender`** — a shared `AppenderInterface` bridge, so the host application's
   logger receives KDT's native and JVM logs. Finding 6.
3. **The headless backend** (`SOURCE:jvm/headless/HeadlessEventLoopJvm.kt` +
   `SOURCE:common/headless/*`) — a fifth `ComposeApplication` implementation that runs on a
   plain `LinkedBlockingQueue` thread. Finding 10.

Everything else in Noria's shared layer is either already better in the target, or is
Noria-runtime-specific (scene/frame-domain, `DrainableCoroutineDispatcher`, `DataSourceContext`,
`withPreparedMainThread`, window reuse).

---

## 2. Ranked findings

### 1. Native window leaks when the composition is abandoned — P1

**Noria** (`SOURCE:common/Window.desktop.kt:291-308`):

```kotlin
val window = remember {
    object : RememberObserver {
        val window = application.createWindow(scene, { reason -> currentOnCloseRequest(reason) })
        override fun onRemembered() {}
        override fun onAbandoned() { window.dispose() }
        override fun onForgotten() { window.dispose() }
    }
}.window
```

**Target** (`Window.kt:274-291`):

```kotlin
val window = remember { application.createWindow(params = …, onCloseRequest = …) }
DisposableEffect(Unit) { onDispose { window.dispose() } }
```

`DisposableEffect`'s `onDispose` only runs for a composition that was **applied**. When Compose
*abandons* a composition — the composition throws before `applyChanges`, a `movableContentOf`
subtree is created and then discarded, a cancelled recomposition — the value created inside
`remember` is dropped with only `RememberObserver.onAbandoned` as notification. The target's
`createWindow` has already created a real native window by that point: it stays mapped on
screen, keeps its GPU surface, and is never removed from `windowList`/`windowsById`, so it also
keeps the application alive from `onQuitRequested`'s point of view.

**Origin**: `43a235ad06e6a` "[fleet] Fix window disposal" (Manuel Unterhofer) — the commit that
replaced exactly this `remember` + effect pair with a `RememberObserver`. Not ticketed; the diff
is unambiguous about what it fixes.

**Port**: mechanical — wrap the `createWindow` call in an anonymous `RememberObserver`, move
`window.dispose()` into both `onAbandoned()` and `onForgotten()`, and delete the
`DisposableEffect(Unit)`. Keep the `SideEffect` and the `LaunchedEffect(window)` as they are.

**Confidence**: high. **Risk**: low — `onForgotten` fires in the same situations `onDispose` did.

---

### 2. `KdtSnapshotManager` outlives the application and throws on the writer's thread — P1

**Target** (`KdtSnapshotManager.kt:26-49`):

```kotlin
internal object KdtSnapshotManager {
    private val started = AtomicBoolean(false)
    fun ensureStarted() {
        if (!started.compareAndSet(false, true)) return
        val channel = Channel<Unit>(capacity = 1)
        CoroutineScope(KdtDispatcher).launch { channel.consumeEach { … } }
        Snapshot.registerGlobalWriteObserver { if (pending.compareAndSet(false, true)) channel.trySend(Unit) }
    }
}
```

Three problems, all downstream of the same thing — nothing ever undoes this:

- `registerGlobalWriteObserver` returns an `ObserverHandle` that is dropped, so the observer is
  installed **for the life of the process**.
- The consumer coroutine's scope is never cancelled, and `started` is never reset, so a second
  `application()` call in the same process silently reuses the first run's machinery.
- The dangerous one: after `close()`, `ApplicationRegistry` is empty
  (`Application.kt:317-319`). Any `mutableStateOf` write anywhere in the process — including from
  a plain background thread, including from library code that has nothing to do with this
  toolkit — hits the still-registered observer, `trySend` resumes the suspended consumer, the
  resumption is dispatched through `KdtDispatcher.dispatch` → `currentApplication()` →
  `IllegalStateException("No ComposeApplication has been initialized…")`. That throw surfaces
  **on the writing thread**, at a `mutableStateOf` assignment that has no plausible connection to
  windowing.

**Noria** binds snapshot observation to the scene's lifetime instead: the handle is kept
(`sceneSnapshots = scene.activateSnapshots(::requestFrame, drainableDispatcher)`,
`SOURCE:jvm/macos/MacOsApplication.kt:632`) and disposed in the scene job's `finally`, which the
comment above it says is "the only place guaranteed to run however the scene ends — last of all
and on the main thread".

**Origin**: `7e8e349d602e5` AIR-6084 "Close the frame domain last, and on the main thread"; the
surrounding "drop frames requested before the scene loop is bound" work (`e95153775d46c`) is the
same lesson from the other end.

**Port**: give `KdtSnapshotManager` a `stop()`: keep the `ObserverHandle` and the `CoroutineScope`
in fields, `handle.dispose()` + `scope.cancel()` + `started.set(false)` from `forgetApplication`
(`Application.kt:327-329`), and make `dispatch` tolerate a missing application. Cheapest belt-
and-braces: also guard `KdtDispatcherBase.dispatch` so a dispatch with no live application drops
the block with a log line rather than throwing.

**Confidence**: high on the leak and the second-run bug; medium-high on the exact throw site
(depends on `Channel` resuming the receiver from the sender's thread, which is what
`trySend` on a buffered channel with a suspended receiver does).
**Risk**: low.

---

### 3. No `InfiniteAnimationPolicy` / `MotionDurationScale` on the application composition — P1

**Noria** (`SOURCE:jvm/NoWindowAnimationPolicy.kt:17-36`):

```kotlin
internal fun noWindowAnimationCoroutineContext(hasNativeWindows: () -> Boolean): CoroutineContext =
    NoWindowMotionDurationScale(hasNativeWindows) + NoWindowInfiniteAnimationPolicy(hasNativeWindows)
```

installed into every scene's context at `SOURCE:jvm/macos/MacOsApplication.kt:620` (and the same
line in the linux/gtk/windows applications). Its doc: *"Desktop application-level content is not
synchronized to a display. When a scene temporarily has no native windows, finite animations
should complete in a single frame and infinite animations should suspend until a window is
attached again."*

**Target** (`Application.kt:233-279`): the application composition runs under

```kotlin
private object YieldFrameClock : MonotonicFrameClock {
    override suspend fun <R> withFrameNanos(onFrame: (Long) -> R): R { yield(); return onFrame(System.nanoTime()) }
}
```

which is precisely a clock "not synchronized to a display" — and there is no policy element in
the context. Idle is fine (nothing awaits a frame when nothing is invalid), but any *infinite*
animation reached from the application composition — `rememberInfiniteTransition`, a spinner in
an app-level overlay, an `animate*AsState` that never settles — will request a frame, get one
immediately after a `yield()`, request another, and pin the event-loop thread at 100 % while
starving input. Every window's own content is safe (each window has a display-link clock), so
this only bites app-level content, which is exactly the case Noria's comment describes.

**Origin**: no single ticket; the file predates the AIR ticket range in this checkout.

**Port**: Compose already ships both abstractions — `androidx.compose.ui.MotionDurationScale` and
`androidx.compose.ui.platform.InfiniteAnimationPolicy` are `CoroutineContext.Element`s the
animation runtime reads. Copy `NoWindowAnimationPolicy.kt` almost verbatim into the target's root
package and change `Application.kt:235` to

```kotlin
withContext(YieldFrameClock + noWindowAnimationCoroutineContext { application.windows.isNotEmpty() }) { … }
```

Note that `application.windows` is already Compose state (`mutableStateListOf`), so Noria's
`snapshotFlow { hasNativeWindows() }.first { it }` works unchanged.

**Confidence**: high on the mechanism; medium on how often it bites in practice (app-level
animation is unusual). **Risk**: low — the policy is a no-op while any window is open.

---

### 4. Windows clipboard: `set` + `flush` is not serialized against concurrent callers — P1

**Noria** (`SOURCE:jvm/windows/WindowsClipboard.kt:79-89, 254`):

```kotlin
private val clipboardMutex = Mutex()
…
clipboardMutex.withLock {
    Win32Clipboard.withRetry { set(data) }
    Win32Clipboard.withRetry { flush() }
}
```

**Target** (`windows/WindowsClipboard.kt:62-65`):

```kotlin
private suspend fun publish(dataObject: DataObject) {
    withBusyRetry { Win32Clipboard.set(dataObject) }
    withBusyRetry { Win32Clipboard.flush() }
}
```

`withBusyRetry` `delay()`s between attempts (`windows/WindowsClipboard.kt:167-180`), which is a
suspension point on the OLE STA. Two coroutines both doing a copy — a paste racing a copy, a
retry storm while the user hits Ctrl+C twice — can therefore interleave as
`A.set → B.set → A.flush → B.flush`, publishing B's data object and flushing it twice while A's
`set` is lost. The target already ported the interesting half of AIR-5558 (the busy-retry ladder,
the `Cleaner`-released `DataObject`, `readForFormat`'s `DataTransferException` guard from
AIR-6051); it just dropped the mutex.

Two smaller deltas in the same neighbourhood:
- Non-busy failures: Noria throws a typed `ClipboardException(status, nativeCode, nativeMessage)`
  (`SOURCE:jvm/windows/WindowsClipboard.kt:312-317`); the target uses
  `check(result.isBusy) { … }`, i.e. an `IllegalStateException` whose message is the only
  machine-readable part. For a library, the typed exception is worth having.
- Exhaustion: Noria throws `ClipboardException(latestFailure)`, the target `error("Clipboard
  stayed busy…")`. Same point.

**Origin**: `79a633bee12d2` / `9d61f959d21cd` AIR-5558 "Use new KDT clipboard in combination with
async retries" (Manuel Unterhofer) — the commit that introduced `clipboardMutex`, `withRetry`,
`withBusyRetry`, the `Cleaner`, and `ClipboardException` together. `238d2c0dfea1c` AIR-6051
"Handle DataTransferException in Windows drag-and-drop" (Kevin Gosse) — external drag sources
such as Firefox fail OLE queries; already ported.

**Port**: add a `private val clipboardMutex = Mutex()` to `WindowsClipboard` and wrap `getClipEntry`'s
`Win32Clipboard.get()` and `publish`'s set/flush pair in `withLock`. Introduce
`ClipboardException` and replace the two `check`/`error` calls. ~20 lines.

**Confidence**: high. **Risk**: low; the mutex is only ever taken on the STA, and every holder
suspends rather than blocks.

---

### 5. Toolkit init and log routing are not configurable, and the init path is duplicated — P1 (library-facing)

**Noria** (`SOURCE:jvm/macos/MacOsApplication.kt:194-201`, `SOURCE:jvm/KdtLoggerAppender.kt`):

```kotlin
System.setProperty("skiko.library.path", libraryFolderPath.toString())
val logFilePath = logFolderPath.resolve("MacOsApplication").resolve("MacOsApplication.log")
KotlinDesktopToolkit.init(libraryFolderPath, logFilePath, appenderInterface = KdtLoggerAppender(kdtLogger))
```

`KdtLoggerAppender` (57 lines) is a straight adapter from KDT's `AppenderInterface` onto Fleet's
`KLogger`, so **everything the native library logs lands in the host application's log**.
The GTK and Windows applications pass `consoleLogLevel = Info, fileLogLevel = Debug`
(`SOURCE:jvm/gtk/GtkApplication.kt:889-894`, `SOURCE:jvm/windows/WindowsApplication.kt:160-165`).

**Target**:
- `macos/MacOsApplication.kt:63` — `KotlinDesktopToolkit.init()`, all defaults.
- `windows/WindowsApplication.kt:55` — same.
- `linux/LinuxApplication.kt:600-616` and `gtk/GtkApplication.kt:573-589` — an identical private
  `initToolkit()` **duplicated in both files**, which reads `kdt.native.log.path` and creates the
  parent directory.

Consequences for a consuming application:
- `kdt.library.folder.path` and `kdt.native.log.path` must be set as **system properties** before
  `application()`; there is no programmatic path (`KotlinDesktopToolkit.init` throws from
  `requiredProperty` otherwise, see `kotlin-desktop-toolkit/src/main/kotlin/org/jetbrains/desktop/macos/KotlinDesktopToolkit.kt:35-37`).
- No log level control, and **no way at all to capture KDT's logs** — `Logger.appender` is
  reachable only as four unrelated `org.jetbrains.desktop.<backend>.Logger` objects, which is
  exactly the problem `BackendLogger.kt` exists to solve for `info` and then stops solving.
- The macOS and Windows paths do not create the log directory, so they behave differently from
  Linux/GTK on a fresh checkout.

**Origin**: no single ticket; `KdtLoggerAppender` is load-bearing infrastructure in Noria from the
start. The related `b049c1dd69ba9` "[air] Make loggers lazy" is a Fleet-wide startup-time change
(`val logger` → `val logger by lazy`) that does **not** apply here — the target's loggers are KDT
objects with no expensive init, and `BackendLogger.kt:18-24`'s `when` already defers each
`Logger` class's `<clinit>` to the branch actually taken. (Its doc comment's claim that "each
`Logger` pulls in its own native library on first touch" is inaccurate — `Logger` is a pure-JVM
object with a `DefaultConsoleAppender` — but the laziness conclusion is right for the wrong
reason.)

**Port**:
1. Extend `ComposeApplicationParams` (`Application.kt:100-111`) with
   `libraryFolderPath: Path? = null`, `logFilePath: Path? = null`,
   `consoleLogLevel`/`fileLogLevel`, and `logAppender: ComposeLogAppender? = null`.
2. Add a shared `ComposeLogAppender` interface in `BackendLogger.kt` mirroring KDT's
   `AppenderInterface` (identical in all four backends), plus four thin per-backend adapters in
   the same `when` style `logInfo` already uses.
3. Add `internal fun initToolkit(backend: DesktopBackend, params: ComposeApplicationParams)` next
   to `logInfo`, doing the directory creation once, and call it from all four applications in
   place of their current `KotlinDesktopToolkit.init()` / private `initToolkit()`.

**Confidence**: high. **Risk**: low; the defaults keep today's behaviour.

---

### 6. `close()` is not idempotent — `stopEventLoop` can be called twice — P2

**Noria** (`SOURCE:jvm/windows/WindowsApplication.kt`, post-AIR-5478):

```kotlin
// stopAndJoin() is reachable from several paths (structured quit, JVM shutdown hook, dock
// cleanup); make sure the native event loop is stopped exactly once.
if (eventLoopStopped.compareAndSet(expectedValue = false, newValue = true)) {
    nativeApplication.stopEventLoop()
}
```

plus, later, an early return: `override suspend fun stopAndJoin() { if (terminationInProgress) return; … }`.

**Target**: none of the four `close()` implementations guard —
`macos/MacOsApplication.kt:318-324`, `windows/WindowsApplication.kt:260-266`,
`linux/LinuxApplication.kt:583-588`, `gtk/GtkApplication.kt:557-561`. A second `close()` posts a
second `stopEventLoop()` to a loop that has already exited (on Windows, `invokeOnDispatcher` to a
closed dispatcher), then `join()`s an already-dead thread, then no-ops the `forgetApplication`
`compareAndSet`.

In today's call graph `close()` is only ever reached once, from `use {}` in `application()`
(`Application.kt:166`). But `ComposeApplication : AutoCloseable` is public API, `initApplication`
is public and hands the caller an application it is expected to close itself, and
`AutoCloseable.close()` is contractually idempotent. Noria hit this for real.

**Origin**: `ec1a8f6c3b69d` / `676afa56a77b4` AIR-5478 "[air] mitigate the issue but not calling
the stopEventLoop for the second time" (Pavel Sergeev); `f73057d4df4dc` AIR-6896 "[air, Windows]
Prevent WindowsApplication from terminating more than once" (Alexander Ovchinnikov), which also
split the `check(initialized && !shutdown)` into two checks so the error message says which one
failed.

**Port**: one `AtomicBoolean closed` per application (or, better, hoist it: a small
`abstract class` / shared `closeOnce { … }` helper used by all four), early-return from `close()`.
Consider also splitting `ApplicationRegistry.current()`'s error message the way AIR-6896 did:
"not initialized" and "already closed" are different bugs for a caller to chase.

**Confidence**: high. **Risk**: none.

---

### 7. Wayland/GTK IME arithmetic duplicated instead of hoisted — P2

**Noria**: `SOURCE:jvm/LinuxInputMethodCommon.kt` (128 lines) holds `utf8OffsetToUtf16Offset`,
`LinuxTextInputSurroundingText` (with `convertByteOffsetsFromCursor`), `LinuxTextInputContext`,
`codepointFromOffset`, and the shared `PlatformTextInputSessionLinux` scope. Both
`SOURCE:jvm/linux/LinuxTextInputSessionOwner.kt` and `SOURCE:jvm/gtk/GtkTextInputSessionOwner.kt`
build on it.

**Target**: `linux/LinuxTextInput.kt:253-309` and `gtk/GtkTextInput.kt:278-334` each carry their
own private copy of `SurroundingText` (with `cursorCodePointOffset`, `selectionStartCodePointOffset`,
`codePointsBefore`, `codePointsAfter`), `Int.utf8Length()`, and
`KeyboardType.toTextInputContentPurpose()`. A normalised diff of the two files
(`s/Gtk/X/;s/Linux/X/`) is ~110 lines, most of it comments; the UTF-8↔UTF-16↔code-point
conversions are character-for-character identical. This is the code most likely to carry a subtle
off-by-one, and fixing it once currently means fixing it twice.

The differences that *are* real and must stay per-file: GTK **pulls** surrounding text through
`getSurroundingText` (hence `publishedSurroundingText` and `surroundingTextForInputMethod()`),
Wayland **pushes** it inside `TextInputContext`; GTK arms the IME per window, Wayland per seat;
the two `TextInputContentHint`/`TextInputContextHint` enums differ.

**Port**: create `LinuxTextInputCommon.kt` next to `LinuxMimeTypes.kt` holding
`internal data class SurroundingText` + its four methods, `Int.utf8Length()`, and a
`KeyboardType.toLinuxContentPurposeName()`-style shared classification that each backend maps onto
its own enum. ~90 lines removed. While there, do the same for the keysym table: `gtk/GtkKeys.kt`
and `linux/LinuxKeys.kt` are byte-identical modulo the package token — a shared
`internal fun keySymToComposeKey(value: <raw>): Key` over the raw XKB keysym value (both backends'
`KeySym` are value classes over the same keysym numbering) collapses another 187 duplicated lines.
`GtkConverters`/`LinuxConverters` (20 diff lines) and the two DnD managers (48) are the next
candidates.

**Confidence**: high. **Risk**: low-medium — the keysym unification needs care that the two
`KeySym` constants really are the same numbering; verify a handful before committing.

---

### 8. `ViewConfiguration` / `WindowInfo` boilerplate repeated in all four windows — P2

**Target**: `macos/MacOsWindow.kt:214-225`, `gtk/GtkWindow.kt:220-232`, and the matching blocks in
`linux/LinuxWindow.kt:234-252` and `windows/WindowsWindow.kt` each declare the same anonymous
`ViewConfiguration` (500/300/40/18.dp) and the same anonymous `WindowInfo`
(`isWindowFocused`/`keyboardModifiers`/`containerSize`). The only real variation is GTK's
`doubleTapTimeoutMillis get() = application.doubleClickIntervalMillis`.

**Noria** is no better — it has four *files* with the same constants
(`SOURCE:jvm/{macos,linux,gtk,windows}/ViewConfiguration.kt`, 12 lines each, textually identical
apart from the package). So there is no seam to copy; the target can simply do better than both.

**Port**: one `internal class KdtViewConfiguration(density: () -> Density, doubleTapTimeoutMillis: () -> Long = { 300 })`
and one `internal class KdtWindowInfo(isFocused: () -> Boolean, keyboardModifiers: () -> PointerKeyboardModifiers, containerSize: () -> IntSize)`
in the root package. ~60 lines removed, and the "why 18.dp" comment gets one home.

**Confidence**: high. **Risk**: none.

---

### 9. `ClipboardFormat.CustomSerializable` — P2 (capability)

**Noria** (`SOURCE:common/Clipboard.kt:42-48`):

```kotlin
data class CustomSerializable<T : Any>(val mimeType: String, val serializer: KSerializer<T>) : ClipboardFormat<T> {
    fun decode(serializedForm: String): T = json.decodeFromString(serializer, serializedForm)
    fun encode(value: T): String = json.encodeToString(serializer, value)
}
```

handled in `SOURCE:jvm/MimeTransfer.jvm.kt:93-98,129,143` and in the Windows clipboard via
`Win32ClipboardFormat.register(mimeType)`.

**Target** (`Clipboard.kt:29-34`): the sealed hierarchy is closed at four built-in formats, so an
application cannot put a typed payload of its own on the clipboard or in a drag at all — which is
the single most common thing an application-level drag needs. Noria's other extra format,
`WindowLocalDrag`, is Fleet-specific (a `LightweightWindowId` marker) and should **not** be ported.

**Port**: add the `CustomSerializable` case and extend the four `when`s that switch on
`ClipboardFormat` (`LinuxMimeTypes.kt:25-30,51-72,74-83`, `windows/WindowsClipboard.kt:121-126,
135-161`, `macos/MacOsClipboard.kt:38-60` + the macOS reader, `gtk`/`linux` clipboard entries).
The target has no `kotlinx.serialization` dependency, so either take one or make the format carry
`encode: (T) -> ByteArray` / `decode: (ByteArray) -> T` lambdas instead — the latter is a better
fit for a dependency-light library and also handles binary payloads, which Noria's JSON-only
version cannot.

**Confidence**: high on the gap; medium on the preferred shape. **Risk**: low, but it widens a
public sealed interface — decide the encoding story before shipping.

---

### 10. No headless backend — the compose module has no tests at all — P2

**Noria**: `SOURCE:jvm/headless/HeadlessEventLoopJvm.kt` (95 lines) — a `LinkedBlockingQueue`
pumped by one named thread, with `pendingTasksCount`, `isCurrentThread()`, and a `close(dropPendingTasks)`
that drains, warns about dropped tasks, and joins — behind `SOURCE:common/headless/HeadlessEventLoop.kt`.
On top of it sit a full `HeadlessApplication` (788 lines), `HeadlessWindow`, `HeadlessScreen`,
`HeadlessUriHandler`, `HeadlessMainDispatcher` and an input-state tracker: a complete fifth
backend used to run Noria's UI tests without a display.

**Target**: no headless path, and `kotlin-desktop-toolkit-compose/build.gradle.kts` has no test
source set — **the module has zero tests** (only `kotlin-desktop-toolkit-compose-dispatcher` has
one). Every one of the shared-layer bugs above (findings 1, 2, 4, 6) is exactly the kind that a
headless `application { Window { … } }` smoke test would have caught: open a window, drop it from
the composition, assert `application.windows` is empty; close twice; write snapshot state after
shutdown.

**Port**: this is the largest item and the one with the best long-term payoff for a library.
A minimal version is much smaller than Noria's: `DesktopBackend.Headless`, a `HeadlessApplication`
implementing `ComposeApplication` over a queue-pumped thread (port `HeadlessEventLoopJvm.kt`
close to verbatim — its `dropPendingTasks` drain and its `if (Thread.currentThread() !== loopThread)`
join guard are both worth keeping), a `HeadlessWindow` whose `setContent` drives an
`ImageComposeScene` or just a `CanvasLayersComposeScene` with no presentation, and a
`HeadlessScreen`. Everything else in the shared layer — dispatcher, snapshot manager,
drag-and-drop manager, clipboard entry model — then becomes testable on CI without a display.

**Confidence**: high on the value. **Risk**: medium — it is a real chunk of work and it adds a
fifth `when` branch everywhere `DesktopBackend` is switched on (which is, to the target's credit,
only three places).

---

### 11. `maven-publish` and `explicitApi()` missing from the compose module — P2 (build)

`kotlin-desktop-toolkit/build.gradle.kts:50` applies `` `maven-publish` `` and line 89 turns on
`explicitApi()`. `kotlin-desktop-toolkit-compose/build.gradle.kts` (36 lines) applies neither:

- **Not publishable.** The module produces a `sourcesJar` but has no `publishing {}` block and no
  plugin, so the whole point of the module — being consumed as
  `org.jetbrains.kotlin-desktop-toolkit:kotlin-desktop-toolkit-compose` — is not wired up.
- **No explicit API mode.** Everything in `Application.kt`, `Window.kt`, `Clipboard.kt`,
  `DragAndDrop.kt` and `ComposeScreen.kt` is implicitly `public` with no explicit visibility or
  return types, in a module whose entire content is API. The base module holds itself to a
  stricter standard than the facade does.
- `kotlin-desktop-toolkit-compose-dispatcher/build.gradle.kts:2` has a stray
  `import org.gradle.kotlin.dsl.\`maven-publish\`` with **no matching plugin application** — a
  leftover that will mislead the next reader.

Other build notes, none of them problems: opt-ins are per-file `@file:OptIn` rather than
`compilerOptions.optIn` (fine, but a new file can silently miss one); Skiko is taken transitively
from `compose.ui` even though `DragAndDropImage.kt` imports `org.jetbrains.skia` directly, while
`sample/build.gradle.kts` pins `skikoVersion = "0.9.37.3"` by hand — worth a comment tying the two
together. Compose is `1.11.1`, Kotlin `2.3.21`, toolchain Java 25. Noria pins KDT at `0.0.83`
through jps library XML; the target consumes `project(":kotlin-desktop-toolkit")` so has no skew
risk, but also documents no compatibility floor for standalone consumers.

**Confidence**: high. **Risk**: none for `maven-publish`; turning on `explicitApi()` is a
mechanical but wide diff.

---

### 12. `NotificationCenter` — P3 (capability)

**Noria**: `SOURCE:jvm/macos/MacOsNotificationCenter.kt` (243 lines, authorization flow +
per-action callbacks + "bring this window to the front" on action),
`SOURCE:jvm/gtk/GtkNotificationCenter.kt` and `SOURCE:jvm/linux/LinuxNotificationCenter.kt`
(120 lines each, `@file:Suppress("DuplicatedCode")` — they are copies of each other). No Windows
implementation.

**Target**: none. KDT does expose the primitives —
`org.jetbrains.desktop.macos.NotificationCenter`, and `requestShowNotification`/`closeNotification`
on the GTK and Wayland `Application` — but win32 has nothing, so a facade API would be 3/4
platforms. Compose has **no** abstraction to plug into; this would be a new
`ComposeApplication.notifications: ComposeNotificationCenter?` surface of the target's own design.

Worth doing eventually for a desktop library; not urgent, and if done, do it as one shared
`ComposeNotificationCenter` interface with a shared Linux implementation (the GTK and Wayland
sides really are the same code) rather than repeating Noria's three-way copy.

---

### 13. Two Noria bugs the target already avoids — informational

Worth recording so nobody "aligns" the target back onto them:

- **`immediate.immediate` is a different object in Noria.** `SOURCE:jvm/macos/Dispatchers.kt:26`
  puts `override val immediate: ImmediateMacOsKdtMainDispatcher by lazy { ImmediateMacOsKdtMainDispatcher() }`
  on the **base** class, which `ImmediateMacOsKdtMainDispatcher` also extends — so
  `dispatcher.immediate.immediate !== dispatcher.immediate`, and each level allocates another
  dispatcher. Same in the linux, gtk, windows and headless copies
  (`SOURCE:common/headless/HeadlessMainDispatcher.kt:8`). The target gets it right:
  `KdtImmediateDispatcher.immediate get() = this` (`KdtDispatcher.kt:31`).
- **A JVM shutdown hook stopping the application is a race.** Noria had one in
  `JvmApplicationRegistry` and removed it: `e2ba99e694520` (and `aa04c23d0026f`, `6537facdbe1af`,
  `8e324a7a7ad9d`) AIR-6701 *"the order of execution of shutdown hooks are not specified, so it
  was a race condition"* (Pavel Sergeev). The target has never had one — do not add one.

---

## 3. Capability gap table

| Noria capability | Does Compose Desktop need it? | Compose abstraction to plug into | Effort |
|---|---|---|---|
| Per-platform `UriHandler` | Yes — **already done** | `LocalUriHandler`, provided in all four `*Application`/`*Window` | — |
| Per-platform `ViewConfiguration` | Yes — **already done** (hoist it, finding 8) | `PlatformContext.viewConfiguration` | XS |
| `WindowInfo` | Yes — **already done** (hoist it) | `PlatformContext.windowInfo` | XS |
| `InputModeManager` | Yes — **already done** | `KdtInputModeManager` behind `PlatformContext.inputModeManager` | — |
| `TextInputSessionOwner` / `ComposeTextInputSession` | **No** — Compose owns the session (`PlatformContext.startInputMethod` + internal `SessionMutex`); Noria's copy is commented-out dead code | `PlatformTextInputMethodRequest` | — |
| `LinuxInputMethodCommon` (utf8↔utf16, shared session scope) | Yes, as a *shared file* (finding 7) | n/a — internal helper | S |
| `NoWindowAnimationPolicy` | **Yes** (finding 3) | `InfiniteAnimationPolicy` + `MotionDurationScale` in the application composition's `CoroutineContext` | S |
| `KdtLoggerAppender` | **Yes** (finding 6) | none — new `ComposeApplicationParams.logAppender` over KDT `AppenderInterface` | S |
| Headless event loop + headless backend | **Yes**, for testability (finding 10) | new `DesktopBackend.Headless` + `ComposeApplication`/`ComposeWindow`/`ComposeScreen` impls | L |
| `ClipboardFormat.CustomSerializable` | **Yes** (finding 9) | target's own `ClipboardFormat` sealed interface | S |
| `ClipboardFormat.WindowLocalDrag` | No — Fleet-specific window-id marker | — | — |
| `RenderLoopRegistry` (AIR-5645) | No — the target has no application-level render-loop list | — | — |
| `DrainableCoroutineDispatcher` | No — Noria frame-domain machinery | — | — |
| `NotificationCenter` (macOS/Wayland/GTK; no win32) | Optional (finding 12) | none in Compose; new facade surface | M |
| `Window.captureScreenshot(): ImageBitmap` | Optional, but pairs with the headless backend for screenshot tests | none in Compose | M |
| `IconDecoratedWindow.setIcon` / `IconDecoratedApplication.setIcon` | Yes for a general-purpose library (Linux/Windows care) | none in Compose | S |
| `showEmojiAndSymbolsPopup()` | Optional, macOS-only | none | XS |
| `InteractiveResizeInitiator` + `WindowResizeHandle` | Maybe — the target has `startInteractiveMove` only; a custom-decorated window needs resize too. Note the target has a `linux/LinuxWindowBorders.kt` doing this in-layer instead. | none | M |
| `CaptionButtonsHostWindow.setCaptionButtonBounds` | Solved differently and arguably better by `ComposeWindow.setCustomTitleBarInteractiveAreas` (`Window.kt:152`) | — | — |
| `Application.awaitWhenReady()` (suspend) | No — the target blocks in the constructor via `CountDownLatch`, which is right for a `main()`-style entry point | — | — |
| Window reuse / pooling | Explicitly out of scope | — | — |

---

## 4. Divergences judged NOT worth porting

- **Window reuse / `reusableNativeWindowResources` / `resetForReuse` / `prepareNativeWindowResourcesForReuse`.**
  Out of scope by instruction, and the machinery only pays off for Fleet's dock. The one fix
  *inside* it that generalises — AIR-5280's "reuse the pointer-icon service, otherwise the cursor
  hides forever" — has no analogue in the target, which creates a fresh `*PointerIcons` per window.
- **Main-thread pinning** (`withPreparedThread` / `withPreparedMainThread`, AIR-6419) and Noria's
  snapshot read-tracking around it. The target solves the same problem differently and more
  simply: everything runs on the event-loop thread under `KdtDispatcher`, and
  `KdtSnapshotManager` flushes writes on that same thread in the same turn (see finding 2 for the
  one thing that needs fixing about it).
- **The extended `Key` enum / const range.** Out of scope; the target's `GtkKeys.kt` comment
  already documents the fold-onto-the-published-enum decision it makes instead.
- **`TextInputSessionOwner` / `PlatformTextInputSessionLinux` as a session abstraction.**
  The target should **not** grow a shared session abstraction of this shape. Noria needs one
  because its scene is not a `ComposeScene` and has no `PlatformContext.startInputMethod`;
  Compose already supplies exactly this contract, and the target's four `*TextInput.kt` files
  correctly implement it (`suspend fun startInputMethod(request): Nothing`, ended by cancellation,
  with a `NonCancellable` teardown — `linux/LinuxTextInput.kt:103-111` — which is the subtle part
  and is already right). What *should* be shared is the arithmetic, not the session (finding 7).
  Noria's own `ComposeTextInputSession.jvm.kt` is 74 lines of commented-out code, which is a
  fair signal about how settled that abstraction is.
- **`Application : Clipboard, UriHandler`** (`SOURCE:common/Application.desktop.kt:79`). Noria
  makes the application *be* the clipboard and the URI handler. The target keeps them as separate
  objects provided through `LocalClipboard`/`LocalUriHandler`, which is the Compose-idiomatic
  shape and testable in isolation. Keep it.
- **Noria's `Screen`/`GloballyPositionedScreen` split** (`SOURCE:common/Screen.desktop.kt`). The
  target's flat `ComposeScreen` with a `position` is simpler; the split exists for the same reason
  as `PositionAwareWindow`, and if the target ever tightens Wayland positioning it should do both
  or neither.
- **`Dispatchers.Main` always-on registration.** Noria's `KdtMainDispatcherFactory` is on its
  classpath unconditionally; the target's opt-in artifact is the right call for a library.
- **Fleet-wide "make loggers lazy"** (`b049c1dd69ba9`). Not applicable: the target's loggers are
  KDT objects with trivial `<clinit>`, and `BackendLogger.kt`'s `when` already defers class
  initialization to the taken branch.
- **`skiko.library.path` juggling** (`SOURCE:jvm/macos/MacOsApplication.kt:195`,
  `SOURCE:jvm/headless/HeadlessEventLoopJvm.kt:93`). Noria ships Skiko natives beside KDT's;
  the target takes Skiko from the Compose artifacts, which load their own. Leave it.
