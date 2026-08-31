@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.compose.linux

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.LocalSystemTheme
import androidx.compose.ui.SystemTheme
import androidx.compose.ui.draganddrop.DragAndDropTransferAction
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.UriHandler
import androidx.compose.ui.unit.LayoutDirection
import org.jetbrains.desktop.compose.ClipboardItemsEntry
import org.jetbrains.desktop.compose.ComposeApplication
import org.jetbrains.desktop.compose.ComposeApplicationParams
import org.jetbrains.desktop.compose.ComposeWindow
import org.jetbrains.desktop.compose.ComposeWindowParams
import org.jetbrains.desktop.compose.DesktopBackend
import org.jetbrains.desktop.compose.WindowCloseRequestReason
import org.jetbrains.desktop.compose.dataForLinuxMimeType
import org.jetbrains.desktop.compose.forgetApplication
import org.jetbrains.desktop.compose.initToolkit
import org.jetbrains.desktop.compose.requestApplicationExit
import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.ApplicationConfig
import org.jetbrains.desktop.linux.ColorSchemeValue
import org.jetbrains.desktop.linux.DataSource
import org.jetbrains.desktop.linux.DesktopSetting
import org.jetbrains.desktop.linux.DragAndDropQueryData
import org.jetbrains.desktop.linux.DragAndDropQueryResponse
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.Logger
import org.jetbrains.desktop.linux.RequestId
import org.jetbrains.desktop.linux.TextInputContext
import org.jetbrains.desktop.linux.WindowId
import org.jetbrains.skia.ColorSpace
import org.jetbrains.skia.Image
import org.jetbrains.skia.ImageInfo
import org.jetbrains.skia.Rect
import org.jetbrains.skia.Surface
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.atomic.AtomicLong
import kotlin.concurrent.thread
import org.jetbrains.skia.Color as SkiaColor

/**
 * Owns the Wayland connection: the event loop, the shared EGL context, and every open surface.
 *
 * Unlike macOS the event loop does not need any particular thread, but it does need one of its own,
 * because [Application.runEventLoop] blocks for as long as the application runs. Everything Compose does is
 * dispatched back onto that thread through [org.jetbrains.desktop.compose.KdtDispatcher].
 *
 * Three things live here rather than in the window because on Wayland they are properties of the *seat* or
 * of the connection, not of a surface:
 *  - which surface has keyboard focus, which pointer focus, and which one the input method is armed for —
 *    all three are separate, and only the keyboard-focus surface may use the input method at all;
 *  - the clipboard and the in-flight drag, both of which are conversations the compositor holds with the
 *    application as a whole and answers by asking for bytes ([ApplicationConfig.getDataTransferData]);
 *  - the desktop settings — colour scheme, double-click interval, cursor theme — which arrive as a stream
 *    of `DesktopSettingChange` events rather than being there to be read.
 */
internal class LinuxApplication(private val params: ComposeApplicationParams) : ComposeApplication {
    // Must run before anything else in this class: every other native call needs the library loaded.
    init {
        initToolkit(DesktopBackend.Wayland, params)
    }

    override val nativeApplication: Application = Application()

    /** The Wayland app id, which is what the desktop matches a window against its `.desktop` file with. */
    internal val identifier: String get() = params.identifier

    private val windowsById = mutableMapOf<WindowId, LinuxWindow>()
    private val windowList = mutableStateListOf<LinuxWindow>()
    private val applicationStarted = CountDownLatch(1)
    private val quitHandlers = ConcurrentHashMap<String, () -> Boolean>()

    private var structuredQuitInProgress = false

    /**
     * Guards teardown, which `AutoCloseable` invites a caller to run twice: `application` closes through
     * `use`, but nothing stops a caller who drove [initApplication] itself from calling [close] again.
     * A second `stopEventLoop` reaches an already torn-down native application.
     */
    @Volatile
    private var terminationInProgress = false

    private val pendingActivations = ConcurrentHashMap<RequestId, WindowId>()

    /** Window ids are ours to hand out, and the toolkit refuses a duplicate, so they only ever go up. */
    private val nextWindowId = AtomicLong(1L)

    /** Set when the event loop never got off the ground — no compositor, no EGL, a missing library. */
    @Volatile
    private var eventLoopFailure: Throwable? = null

    internal val clipboard = LinuxClipboard(this)

    override var systemTheme: SystemTheme by mutableStateOf(SystemTheme.Unknown)
        private set

    override var screens: List<LinuxScreen> by mutableStateOf(emptyList())
        private set

    override val windows: List<ComposeWindow> get() = windowList

    override val focusedWindow: ComposeWindow? get() = keyboardFocusWindowId?.let { windowsById[it] }

    override val mainScreen: LinuxScreen
        get() = screens.firstOrNull()
            ?: readScreens().firstOrNull()
            ?: error("The compositor has not reported any output")

    /**
     * Always `true`: Wayland tells a client whether one of its surfaces is activated, which the window
     * reports through `isFocused`, but there is no application-level "frontmost application" to ask about.
     */
    override val isActive: Boolean get() = true

    /** No protocol for it, and every compositor in practice lays windows out left to right. */
    override val layoutDirection: LayoutDirection = LayoutDirection.Ltr

    internal val uriHandler: UriHandler = object : UriHandler {
        override fun openUri(uri: String) {
            onEventLoop {
                // The activation token would let the browser raise itself; this layer has none to give.
                nativeApplication.openURL(uri, null)
            }
        }
    }

    /** Reported to Compose through each window's `ViewConfiguration`, hence observable. */
    internal var doubleClickIntervalMillis: Long by mutableStateOf(500L)
        private set

    private var cursorTheme: String? = null
    private var cursorSize: UInt? = null

    // Wayland text input is seat-wide: only one surface at a time may use it, and the last context per
    // window is kept so the seat can be re-armed when focus comes back.
    private var textInputWindowId: WindowId? = null
    private val lastTextInputContexts = mutableMapOf<WindowId, TextInputContext>()

    private var keyboardFocusWindowId: WindowId? = null
    private var mouseFocusWindowId: WindowId? = null

    private var activeDragSource: ActiveDragSource? = null

    private val eventLoopThread: Thread

    // ----- Event loop -----

    private fun startEventLoopThread(): Thread = thread(start = true, name = "KDT Event Loop") {
        try {
            nativeApplication.runEventLoop(
                ApplicationConfig(
                    eventHandler = ::handleEvent,
                    queryDragAndDropTarget = ::queryDragAndDropTarget,
                    getDataTransferData = ::getDataTransferData,
                ),
            )
        } catch (throwable: Throwable) {
            eventLoopFailure = throwable
            Logger.error { "The event loop stopped with an error: $throwable" }
        } finally {
            // Releases a constructor still waiting to be told the application launched, so a loop that
            // never started fails the caller instead of hanging it.
            applicationStarted.countDown()
        }
    }

    private fun handleEvent(event: Event): EventHandlerResult {
        return try {
            when (event) {
                Event.ApplicationStarted -> {
                    screens = readScreens()
                    applicationStarted.countDown()
                    EventHandlerResult.Stop
                }

                // Not a user gesture, despite the name: the toolkit raises this only once `stopEventLoop`
                // has set its exit flag, and re-raises it every frame until the handler concedes, so Stop
                // — the veto — hangs [close] forever. The user's Quit arrives through [quit] instead.
                Event.ApplicationWantsToTerminate -> EventHandlerResult.Continue

                Event.ApplicationWillTerminate -> EventHandlerResult.Continue

                is Event.DisplayConfigurationChange -> {
                    screens = event.screens.screens.map(::LinuxScreen)
                    EventHandlerResult.Stop
                }

                is Event.DesktopSettingChange -> {
                    handleSettingChange(event.setting)
                    EventHandlerResult.Stop
                }

                is Event.DataTransferAvailable -> {
                    if (event.dataSource == DataSource.Clipboard) {
                        clipboard.onDataTransferAvailable(event)
                    }
                    EventHandlerResult.Stop
                }

                is Event.DataTransfer ->
                    if (clipboard.onDataTransfer(event)) EventHandlerResult.Stop else EventHandlerResult.Continue

                is Event.DataTransferCancelled -> {
                    when (event.dataSource) {
                        DataSource.Clipboard -> clipboard.onOwnershipLost()
                        DataSource.DragAndDrop -> finishDragSession(action = null)
                        DataSource.PrimarySelection -> {}
                    }
                    EventHandlerResult.Stop
                }

                is Event.ActivationTokenResponse -> {
                    pendingActivations.remove(event.requestId)
                        ?.let { windowId -> windowsById[windowId]?.activate(event.token) }
                    EventHandlerResult.Stop
                }

                is Event.DragIconDraw -> drawDragIcon(event)

                is Event.DragAndDropFinished -> {
                    if (activeDragSource?.windowId == event.windowId) {
                        finishDragSession(event.action?.toDragAndDropTransferAction())
                        EventHandlerResult.Stop
                    } else {
                        EventHandlerResult.Continue
                    }
                }

                is Event.TextInputAvailability -> handleTextInputAvailability(event)

                is Event.TextInput ->
                    textInputWindowId?.let { windowsById[it]?.handleTextInput(event) } ?: EventHandlerResult.Continue

                is Event.KeyDown, is Event.KeyUp ->
                    keyboardFocusWindowId?.let { windowsById[it]?.handleEvent(event) } ?: EventHandlerResult.Continue

                is Event.ModifiersChanged -> {
                    // The surface under the pointer and the one with keyboard focus can differ, and both
                    // consume the modifier state — hover cursors on one side, shortcuts on the other.
                    setOfNotNull(mouseFocusWindowId, keyboardFocusWindowId)
                        .forEach { windowId -> windowsById[windowId]?.handleEvent(event) }
                    EventHandlerResult.Stop
                }

                is Event.WindowKeyboardEnter -> {
                    keyboardFocusWindowId = event.windowId
                    windowsById[event.windowId]?.handleEvent(event) ?: EventHandlerResult.Continue
                }

                is Event.WindowKeyboardLeave -> {
                    if (keyboardFocusWindowId == event.windowId) {
                        keyboardFocusWindowId = null
                    }
                    windowsById[event.windowId]?.handleEvent(event) ?: EventHandlerResult.Continue
                }

                is Event.MouseEntered -> {
                    mouseFocusWindowId = event.windowId
                    windowsById[event.windowId]?.handleEvent(event) ?: EventHandlerResult.Continue
                }

                is Event.MouseExited -> {
                    if (mouseFocusWindowId == event.windowId) {
                        mouseFocusWindowId = null
                    }
                    windowsById[event.windowId]?.handleEvent(event) ?: EventHandlerResult.Continue
                }

                is Event.MouseDown -> {
                    val window = windowsById[event.windowId] ?: return EventHandlerResult.Continue
                    cancelCompositionOnPress(event.windowId, window)
                    window.handleEvent(event)
                }

                is Event.WindowClosed -> {
                    windowsById.remove(event.windowId)?.let { window ->
                        windowList.remove(window)
                        window.onClosed()
                    }
                    EventHandlerResult.Stop
                }

                is Event.FileChooserResponse ->
                    // The event carries no window id, so it has to be offered around until it is claimed.
                    if (windowList.toList().any { it.handleEvent(event) == EventHandlerResult.Stop }) {
                        EventHandlerResult.Stop
                    } else {
                        EventHandlerResult.Continue
                    }

                else -> event.windowIdOrNull()
                    ?.let { windowId -> windowsById[windowId]?.handleEvent(event) }
                    ?: EventHandlerResult.Continue
            }
        } catch (throwable: Throwable) {
            // A handler that throws must not take the event loop with it; let the event fall through to the
            // toolkit's own handling instead.
            Logger.error { "Failed to handle event $event: $throwable" }
            EventHandlerResult.Continue
        }
    }

    private fun handleSettingChange(setting: DesktopSetting) {
        when (setting) {
            is DesktopSetting.ColorScheme -> {
                systemTheme = when (setting.value) {
                    ColorSchemeValue.PreferDark -> SystemTheme.Dark
                    ColorSchemeValue.PreferLight -> SystemTheme.Light
                    ColorSchemeValue.NoPreference -> SystemTheme.Unknown
                }
            }

            is DesktopSetting.DoubleClickInterval -> {
                doubleClickIntervalMillis = setting.value.inWholeMilliseconds
            }

            is DesktopSetting.CursorTheme -> {
                cursorTheme = setting.value
                applyCursorThemeIfReady()
            }

            is DesktopSetting.CursorSize -> {
                cursorSize = setting.value
                applyCursorThemeIfReady()
            }

            else -> {}
        }
    }

    /** Both halves arrive as separate settings, and the toolkit wants them together. */
    private fun applyCursorThemeIfReady() {
        val theme = cursorTheme ?: return
        val size = cursorSize ?: return
        nativeApplication.setCursorTheme(theme, size)
    }

    private fun readScreens(): List<LinuxScreen> = nativeApplication.allScreens().screens.map(::LinuxScreen)

    // ----- Text input -----
    //
    // The input method is enabled on the seat rather than on a surface, so arming it is the application's
    // job: a window only ever says what its focused text field looks like, and whether that reaches the
    // input method depends on whether the window still holds the seat.

    private fun handleTextInputAvailability(event: Event.TextInputAvailability): EventHandlerResult {
        textInputWindowId = event.windowId.takeIf { event.available }
        if (event.available) {
            lastTextInputContexts[event.windowId]?.let { context ->
                nativeApplication.textInputEnable(context.copy(changeCausedByInputMethod = false))
            }
        } else {
            nativeApplication.textInputDisable()
            windowsById[event.windowId]?.discardActivePreedit()
        }
        return EventHandlerResult.Stop
    }

    internal fun startTextInput(windowId: WindowId, context: TextInputContext) {
        onEventLoop {
            lastTextInputContexts[windowId] = context
            if (textInputWindowId == windowId) {
                nativeApplication.textInputEnable(context)
            }
        }
    }

    internal fun updateTextInput(windowId: WindowId, context: TextInputContext) {
        onEventLoop {
            lastTextInputContexts[windowId] = context
            if (textInputWindowId == windowId) {
                nativeApplication.textInputUpdate(context)
            }
        }
    }

    internal fun stopTextInput(windowId: WindowId) {
        onEventLoop {
            lastTextInputContexts.remove(windowId)
            if (textInputWindowId == windowId) {
                nativeApplication.textInputDisable()
                windowsById[windowId]?.discardActivePreedit()
            }
        }
    }

    /**
     * Cancels an open composition when the user clicks somewhere.
     *
     * Some compositors (GNOME, KDE) do this themselves, others (Sway) do not, and a composition left open
     * across a click ends up committed into whatever the click moved the caret to. Disabling and
     * re-enabling the input method is what tells it to start over.
     */
    private fun cancelCompositionOnPress(windowId: WindowId, window: LinuxWindow) {
        if (!window.hasPreeditString()) return
        val lastContext = lastTextInputContexts[windowId] ?: return
        nativeApplication.textInputDisable()
        window.discardActivePreedit()
        nativeApplication.textInputEnable(lastContext.copy(changeCausedByInputMethod = false))
    }

    // ----- Data transfer -----

    /**
     * The bytes another application asked for, for a copy or a drag this application is the source of.
     *
     * Called from the event loop thread while the other side waits, which is why both answers have to come
     * out of data already in hand rather than out of a round trip through the composition.
     */
    private fun getDataTransferData(dataSource: DataSource, mimeType: String): ByteArray? = when (dataSource) {
        DataSource.Clipboard -> clipboard.dataForMimeType(mimeType)
        DataSource.DragAndDrop -> activeDragSource?.itemsEntry?.dataForLinuxMimeType(mimeType)
        // Not offered: Compose's `Clipboard` has no concept of the primary selection.
        DataSource.PrimarySelection -> null
    }

    private fun queryDragAndDropTarget(query: DragAndDropQueryData): DragAndDropQueryResponse =
        windowsById[query.windowId]?.queryDragAndDropTarget(query) ?: DragAndDropQueryResponse(emptyList())

    /** One in-flight drag this application started. There can only be one, because there is one pointer. */
    internal class ActiveDragSource(
        val windowId: WindowId,
        val itemsEntry: ClipboardItemsEntry,
        val onTransferCompleted: (DragAndDropTransferAction?) -> Unit,
        val dragIconPngBytes: ByteArray?,
    )

    internal fun beginDragSession(source: ActiveDragSource) {
        activeDragSource = source
    }

    private fun finishDragSession(action: DragAndDropTransferAction?) {
        val source = activeDragSource ?: return
        activeDragSource = null
        source.onTransferCompleted(action)
    }

    /**
     * Paints the drag icon into the buffer the compositor handed us.
     *
     * The icon is a surface of its own that the compositor asks to be repainted whenever it moves between
     * outputs, so the decoration cannot be drawn once and forgotten — the window rasterized it to a PNG
     * when the drag started, and this replays it at whatever size is being asked for now.
     */
    private fun drawDragIcon(event: Event.DragIconDraw): EventHandlerResult {
        val pngBytes = activeDragSource?.dragIconPngBytes ?: return EventHandlerResult.Continue
        val softwareDrawData = event.softwareDrawData ?: return EventHandlerResult.Continue
        val width = event.size.width.rawPhysical
        val height = event.size.height.rawPhysical
        if (width <= 0 || height <= 0) return EventHandlerResult.Continue
        val image = Image.makeFromEncoded(pngBytes)
        Surface.makeRasterDirect(
            imageInfo = ImageInfo.makeN32Premul(width, height, ColorSpace.sRGB),
            pixelsPtr = softwareDrawData.canvas,
            rowBytes = softwareDrawData.stride,
        ).use { surface ->
            surface.canvas.clear(SkiaColor.TRANSPARENT)
            surface.canvas.drawImageRect(
                image,
                Rect.makeWH(image.width.toFloat(), image.height.toFloat()),
                Rect.makeWH(width.toFloat(), height.toFloat()),
            )
            surface.flushAndSubmit()
        }
        return EventHandlerResult.Stop
    }

    // ----- Windows -----

    override fun createWindow(params: ComposeWindowParams, onCloseRequest: (WindowCloseRequestReason) -> Unit): ComposeWindow {
        return LinuxWindow(this, params, onCloseRequest)
    }

    internal fun allocateWindowId(): WindowId = nextWindowId.getAndIncrement()

    internal fun attachWindow(window: LinuxWindow) {
        windowsById[window.windowId] = window
        windowList.add(window)
    }

    internal fun detachWindow(window: LinuxWindow) {
        windowsById.remove(window.windowId)
        windowList.remove(window)
    }

    /**
     * Asks the compositor for an activation token for [windowId], and hands it straight back once it
     * arrives — see [LinuxWindow.requestFocus] for why focus needs the round trip.
     */
    internal fun requestWindowActivation(windowId: WindowId) {
        // Asked for by the window that holds the keyboard, not by the one being raised: the compositor
        // builds the token from the requesting surface plus the seat's latest serial, and refuses one asked
        // for by a surface that is not focused.
        val source = keyboardFocusWindowId?.let { windowsById[it] } ?: return
        onEventLoop {
            source.nativeWindow.requestActivationToken()?.let { requestId ->
                pendingActivations[requestId] = windowId
            }
        }
    }

    // ----- Application-level API -----

    /** Nothing to do: a Wayland client cannot raise itself, only spend a focus it already has. */
    override fun requestActivation() = Unit

    override fun quit() {
        onEventLoop { requestStructuredQuit() }
    }

    private fun requestStructuredQuit() {
        // Every handler is consulted, not just up to the first veto: a handler may be showing a dialog and
        // needs to know a quit was attempted even if another handler already said no.
        if (!quitHandlers.values.fold(true) { acc, quitHandler -> quitHandler() and acc }) {
            return
        }
        // Re-entered from a window's own `onCloseRequest`, which runs inline below.
        if (structuredQuitInProgress) {
            return
        }
        val windowsToClose = windowList.toList()
        if (windowsToClose.isEmpty()) {
            requestApplicationExit(DesktopBackend.Wayland)
            return
        }
        Logger.info { "Quit approved, asking ${windowsToClose.size} window(s) to close" }
        // Cleared again whatever the windows do, so a quit the application declined leaves nothing behind.
        structuredQuitInProgress = true
        try {
            windowsToClose.forEach { it.requestCloseFromQuit() }
        } finally {
            structuredQuitInProgress = false
        }
    }

    override fun putQuitHandler(id: String, isSafeToQuit: () -> Boolean) {
        quitHandlers[id] = isSafeToQuit
    }

    override fun removeQuitHandler(id: String) {
        quitHandlers.remove(id)
    }

    override fun openUri(uri: String) {
        uriHandler.openUri(uri)
    }

    override fun invokeOnUiThread(block: () -> Unit) {
        nativeApplication.runOnEventLoopAsync(block)
    }

    override fun isUiThread(): Boolean = nativeApplication.isEventLoopThread()

    /** Runs [block] on the event loop thread, inline when that is already where we are. */
    internal fun onEventLoop(block: () -> Unit) {
        if (nativeApplication.isEventLoopThread()) {
            block()
        } else {
            nativeApplication.runOnEventLoopAsync(block)
        }
    }

    /** [onEventLoop] with the native application as the receiver, for the many one-line native calls. */
    internal fun onEventLoopAsync(block: Application.() -> Unit) {
        onEventLoop { nativeApplication.block() }
    }

    @Composable
    override fun ProvideCompositionLocals(content: @Composable () -> Unit) {
        CompositionLocalProvider(
            LocalSystemTheme provides systemTheme,
            LocalClipboard provides clipboard,
            LocalUriHandler provides uriHandler,
        ) {
            content()
        }
    }

    override fun close() {
        if (terminationInProgress) {
            return
        }
        terminationInProgress = true
        nativeApplication.stopEventLoop()
        eventLoopThread.join()
        nativeApplication.close()
        forgetApplication(this)
    }

    init {
        // Started from the last initializer in the class, and not from a property initializer: the thread
        // body runs immediately and reaches back into everything declared above it.
        eventLoopThread = startEventLoopThread()
        // Returns once the compositor has reported the application is up, so windows can be created.
        applicationStarted.await()
        eventLoopFailure?.let { throwable ->
            throw IllegalStateException("Could not start the Wayland event loop; is this a Wayland session?", throwable)
        }
    }
}

/**
 * The window an event belongs to, for the events that are routed by nothing more than that.
 *
 * Events with a focus rule of their own (keys, the input method) and events that belong to the application
 * (the clipboard, the drag icon) are handled before this is reached.
 */
private fun Event.windowIdOrNull(): WindowId? = when (this) {
    is Event.WindowCloseRequest -> windowId
    is Event.WindowConfigure -> windowId
    is Event.WindowDraw -> windowId
    is Event.WindowScaleChanged -> windowId
    is Event.WindowScreenChange -> windowId
    is Event.MouseUp -> windowId
    is Event.MouseMoved -> windowId
    is Event.ScrollWheel -> windowId
    is Event.DropPerformed -> windowId
    is Event.DragAndDropLeave -> windowId
    else -> null
}
