@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.compose.gtk

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
import org.jetbrains.desktop.compose.WindowCloseRequestReason
import org.jetbrains.desktop.compose.dataForLinuxMimeType
import org.jetbrains.desktop.compose.forgetApplication
import org.jetbrains.desktop.gtk.Application
import org.jetbrains.desktop.gtk.ApplicationConfig
import org.jetbrains.desktop.gtk.ColorSchemeValue
import org.jetbrains.desktop.gtk.DataSource
import org.jetbrains.desktop.gtk.DesktopSetting
import org.jetbrains.desktop.gtk.DragAndDropQueryData
import org.jetbrains.desktop.gtk.DragAndDropQueryResponse
import org.jetbrains.desktop.gtk.Event
import org.jetbrains.desktop.gtk.EventHandlerResult
import org.jetbrains.desktop.gtk.KotlinDesktopToolkit
import org.jetbrains.desktop.gtk.Logger
import org.jetbrains.desktop.gtk.TextInputSurroundingText
import org.jetbrains.desktop.gtk.WindowId
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.ColorSpace
import org.jetbrains.skia.DirectContext
import org.jetbrains.skia.FramebufferFormat
import org.jetbrains.skia.GLAssembledInterface
import org.jetbrains.skia.Image
import org.jetbrains.skia.Rect
import org.jetbrains.skia.Surface
import org.jetbrains.skia.SurfaceColorFormat
import org.jetbrains.skia.SurfaceOrigin
import org.jetbrains.skia.makeGLWithInterface
import java.nio.file.Files
import java.nio.file.Path
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.atomic.AtomicLong
import kotlin.concurrent.thread
import org.jetbrains.skia.Color as SkiaColor

/**
 * Owns the GTK connection: the event loop, the shared GL context, and every open window.
 *
 * The event loop needs a thread of its own, because [Application.runEventLoop] blocks for as long as the
 * application runs. Everything Compose does is dispatched back onto that thread through
 * [org.jetbrains.desktop.compose.KdtDispatcher].
 *
 * Unlike the Wayland backend, the native [Application] is constructed *on* that thread rather than by the
 * caller: its constructor runs `gtk_init`, and GTK expects that on the thread that will run its main loop.
 * The constructor of this class blocks until the loop has reported the application started, so by the time
 * anybody can reach [nativeApplication] or [glProcFunc] both are set.
 *
 * What lives here rather than in the window, because GTK makes it a property of the application:
 *  - the clipboard and the in-flight drag, both of which are conversations the desktop holds with the
 *    application as a whole and answers by asking for bytes ([ApplicationConfig.getDataTransferData]);
 *  - the drag icon, which is one surface for the whole application no matter which window started the drag;
 *  - the desktop settings — colour scheme, double-click interval — which arrive as a stream of
 *    `DesktopSettingChange` events rather than being there to be read.
 *
 * Two things that are *events* on Wayland are *callbacks* here, and both have to answer synchronously:
 * whether a window may close, and whether the application may terminate.
 */
internal class GtkApplication(private val params: ComposeApplicationParams) : ComposeApplication {
    // Must run before anything else in this class: every other native call needs the library loaded.
    init {
        initToolkit()
    }

    private val windowsById = mutableMapOf<WindowId, GtkWindow>()
    private val windowList = mutableStateListOf<GtkWindow>()
    private val applicationStarted = CountDownLatch(1)
    private val quitHandlers = ConcurrentHashMap<String, () -> Boolean>()

    /** Window ids are ours to hand out, and the toolkit refuses a duplicate, so they only ever go up. */
    private val nextWindowId = AtomicLong(1L)

    @Volatile
    private var isQuitInProgress = false

    /** Set when the event loop never got off the ground — no display, no GL, a missing library. */
    @Volatile
    private var eventLoopFailure: Throwable? = null

    @Volatile
    private var nativeApplicationOrNull: Application? = null

    @Volatile
    private var glProcFuncOrNull: Application.GlProcFunc? = null

    /** Valid once the constructor has returned; see the class comment for why it is not a `val`. */
    override val nativeApplication: Application
        get() = checkNotNull(nativeApplicationOrNull) { "The GTK application is not running yet" }

    /** The GL entry points Skiko renders through, shared by every window and by the drag icon. */
    internal val glProcFunc: Application.GlProcFunc
        get() = checkNotNull(glProcFuncOrNull) { "The GTK application has not initialized OpenGL yet" }

    internal val clipboard = GtkClipboard(this)

    override var systemTheme: SystemTheme by mutableStateOf(SystemTheme.Unknown)
        private set

    override var screens: List<GtkScreen> by mutableStateOf(emptyList())
        private set

    override val windows: List<ComposeWindow> get() = windowList

    override val focusedWindow: ComposeWindow? get() = windowList.firstOrNull { it.isFocused }

    override val mainScreen: GtkScreen
        get() = screens.firstOrNull()
            ?: readScreens().firstOrNull()
            ?: error("GTK has not reported any monitor")

    /**
     * Always `true`: GTK tells a client whether one of its windows is active, which the window reports
     * through `isFocused`, but there is no application-level "frontmost application" to ask about.
     */
    override val isActive: Boolean get() = true

    /** No API for it, and every GTK desktop in practice lays windows out left to right. */
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

    private var keyboardFocusWindowId: WindowId? = null
    private var mouseFocusWindowId: WindowId? = null

    private var activeDragSource: ActiveDragSource? = null

    /**
     * Skia's handle on the drag icon's GL context, created on the first `DragIconDraw` of a session and
     * dropped with the session. The icon is a surface of its own, so it cannot share a window's context,
     * and one per drag is cheap next to one per frame — GTK redraws the icon on every frame tick.
     */
    private var dragIconDirectContext: DirectContext? = null

    private val eventLoopThread: Thread

    // ----- Event loop -----

    private fun startEventLoopThread(): Thread = thread(start = true, name = "KDT GTK Event Loop") {
        try {
            // `gtk_init` runs here, in the constructor, so it has to happen on this thread.
            val application = Application(params.identifier)
            nativeApplicationOrNull = application
            // EGL first, and desktop GL as the fallback, which is what the toolkit's own samples do.
            glProcFuncOrNull = application.getEglProcFunc()
                ?: application.initializeGl("libGL.so")
                ?: error("The GTK backend could not initialize OpenGL, so no window could render")
            application.runEventLoop(
                ApplicationConfig(
                    eventHandler = ::handleEvent,
                    queryDragAndDropTarget = ::queryDragAndDropTarget,
                    getDataTransferData = ::getDataTransferData,
                    windowCloseRequest = ::onWindowCloseRequested,
                    applicationWantsToTerminate = ::onQuitRequested,
                    getSurroundingText = ::getSurroundingText,
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
                is Event.ApplicationStarted -> {
                    screens = readScreens()
                    applicationStarted.countDown()
                    EventHandlerResult.Stop
                }

                is Event.DisplayConfigurationChange -> {
                    screens = event.screens.screens.map(::GtkScreen)
                    EventHandlerResult.Stop
                }

                is Event.DesktopSettingChange -> {
                    handleSettingChange(event.setting)
                    EventHandlerResult.Stop
                }

                is Event.DataTransferAvailable -> {
                    clipboard.onDataTransferAvailable(event)
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

                is Event.DragIconFrameTick -> {
                    if (activeDragSource == null) {
                        EventHandlerResult.Continue
                    } else {
                        nativeApplication.requestRedrawDragIcon()
                        EventHandlerResult.Stop
                    }
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

                is Event.TextInput ->
                    keyboardFocusWindowId?.let { windowsById[it]?.handleEvent(event) } ?: EventHandlerResult.Continue

                is Event.KeyDown, is Event.KeyUp ->
                    keyboardFocusWindowId?.let { windowsById[it]?.handleEvent(event) } ?: EventHandlerResult.Continue

                is Event.ModifiersChanged -> {
                    // The window under the pointer and the one with keyboard focus can differ, and both
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
                // GTK draws the window buttons in a custom titlebar itself, so it has to be told which way
                // the theme went or they end up dark-on-dark.
                nativeApplication.setPreferDarkTheme(systemTheme == SystemTheme.Dark)
            }

            is DesktopSetting.DoubleClickInterval -> {
                doubleClickIntervalMillis = setting.value.inWholeMilliseconds
            }

            else -> {}
        }
    }

    private fun readScreens(): List<GtkScreen> = nativeApplication.allScreens().screens.map(::GtkScreen)

    // ----- Closing and quitting -----
    //
    // Both are callbacks rather than events on GTK, and both are answered synchronously from the event loop
    // thread. Neither closes anything itself: the composition owns the windows, so all these do is ask.

    /**
     * GTK is asking whether one window may close. Always answered with `false` — "not by you" — because a
     * window goes away when the composition stops emitting it, which is what `onCloseRequest` sets in
     * motion.
     */
    private fun onWindowCloseRequested(windowId: WindowId): Boolean {
        windowsById[windowId]?.requestClose()
        return false
    }

    /**
     * Runs when the user chooses Quit (or something calls [quit]).
     *
     * Returns whether the application may terminate right now, which is only the case when nothing vetoed
     * *and* there is no window left. Closing the windows goes through each one's `onCloseRequest`, so the
     * application decides — and the composition, once it stops emitting those windows, is what ends
     * `runApplication` and unwinds the process from `main`.
     */
    private fun onQuitRequested(): Boolean {
        // Every handler is consulted, not just up to the first veto: a handler may be showing a dialog and
        // needs to know a quit was attempted even if another handler already said no.
        val isSafeToQuit = quitHandlers.values.fold(true) { acc, handler -> handler() and acc }
        if (!isSafeToQuit || isQuitInProgress) {
            return false
        }
        val openWindows = windowList.toList()
        if (openWindows.isEmpty()) {
            return true
        }
        isQuitInProgress = true
        Logger.info { "Quit approved, asking ${openWindows.size} window(s) to close" }
        openWindows.forEach { it.requestCloseFromQuit() }
        return false
    }

    // ----- Text input -----

    /**
     * GTK is asking what text surrounds the cursor in [windowId]'s focused field.
     *
     * Pulled rather than published, unlike Wayland — see [GtkTextInput]. An empty answer stands for "no
     * text field here", which is the honest answer for a window whose content is not editing anything.
     */
    private fun getSurroundingText(windowId: WindowId): TextInputSurroundingText = windowsById[windowId]?.currentTextInputSurroundingText()
        ?: TextInputSurroundingText(
            surroundingText = "",
            cursorCodepointOffset = 0u,
            selectionStartCodepointOffset = 0u,
        )

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
        dragIconDirectContext?.close()
        dragIconDirectContext = null
        source.onTransferCompleted(action)
    }

    /**
     * Paints the drag icon into the framebuffer GTK handed us.
     *
     * The icon is a surface of its own that GTK asks to be repainted on every frame of the drag, so the
     * decoration cannot be drawn once and forgotten — the window rasterized it to a PNG when the drag
     * started, and this replays it at whatever size is being asked for now.
     */
    private fun drawDragIcon(event: Event.DragIconDraw): EventHandlerResult {
        val pngBytes = activeDragSource?.dragIconPngBytes ?: return EventHandlerResult.Continue
        val width = event.size.width.rawPhysical
        val height = event.size.height.rawPhysical
        if (width <= 0 || height <= 0) return EventHandlerResult.Continue
        val image = Image.makeFromEncoded(pngBytes)
        val directContext = dragIconDirectContext ?: DirectContext.makeGLWithInterface(
            GLAssembledInterface.createFromNativePointers(ctxPtr = glProcFunc.ctxPtr, fPtr = glProcFunc.fPtr),
        ).also { dragIconDirectContext = it }

        BackendRenderTarget.makeGL(
            width = width,
            height = height,
            sampleCnt = 1,
            stencilBits = 0,
            fbId = event.openGlDrawData.framebuffer,
            fbFormat = FramebufferFormat.GR_GL_RGBA8,
        ).use { renderTarget ->
            Surface.makeFromBackendRenderTarget(
                context = directContext,
                rt = renderTarget,
                origin = SurfaceOrigin.TOP_LEFT,
                colorFormat = SurfaceColorFormat.RGBA_8888,
                colorSpace = ColorSpace.sRGB,
                surfaceProps = null,
            )!!.use { surface ->
                // The icon is a free-floating surface, so everything the decoration did not cover has to
                // stay see-through.
                surface.canvas.clear(SkiaColor.TRANSPARENT)
                surface.canvas.drawImageRect(
                    image,
                    Rect.makeWH(image.width.toFloat(), image.height.toFloat()),
                    Rect.makeWH(width.toFloat(), height.toFloat()),
                )
                surface.flushAndSubmit()
            }
        }
        return EventHandlerResult.Stop
    }

    // ----- Windows -----

    override fun createWindow(params: ComposeWindowParams, onCloseRequest: (WindowCloseRequestReason) -> Unit): ComposeWindow {
        return GtkWindow(this, params, onCloseRequest)
    }

    internal fun allocateWindowId(): WindowId = nextWindowId.getAndIncrement()

    internal fun attachWindow(window: GtkWindow) {
        windowsById[window.windowId] = window
        windowList.add(window)
    }

    internal fun detachWindow(window: GtkWindow) {
        windowsById.remove(window.windowId)
        windowList.remove(window)
    }

    // ----- Application-level API -----

    override fun requestActivation() {
        (focusedWindow ?: windows.firstOrNull())?.requestFocusAndBringToFront()
    }

    override fun quit() {
        onEventLoop { onQuitRequested() }
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
        nativeApplicationOrNull?.stopEventLoop()
        eventLoopThread.join()
        forgetApplication(this)
    }

    init {
        // Started from the last initializer in the class, and not from a property initializer: the thread
        // body runs immediately and reaches back into everything declared above it.
        eventLoopThread = startEventLoopThread()
        // Returns once GTK has reported the application is up, so windows can be created.
        applicationStarted.await()
        eventLoopFailure?.let { throwable ->
            throw IllegalStateException("Could not start the GTK event loop; is a display available?", throwable)
        }
    }
}

/**
 * Loads the native library and points its logger at the configured file.
 *
 * The parent directory is created first: the native logger fails to initialize if it is not there, and a
 * fresh checkout has no build directory yet.
 */
private fun initToolkit() {
    val logFilePath = System.getProperty("kdt.native.log.path")?.let(Path::of)
    logFilePath?.parent?.let(Files::createDirectories)
    if (logFilePath == null) {
        KotlinDesktopToolkit.init()
    } else {
        KotlinDesktopToolkit.init(logFilePath = logFilePath)
    }
}

/**
 * The window an event belongs to, for the events that are routed by nothing more than that.
 *
 * Events with a focus rule of their own (keys, the input method) and events that belong to the application
 * (the clipboard, the drag icon) are handled before this is reached.
 */
private fun Event.windowIdOrNull(): WindowId? = when (this) {
    is Event.WindowConfigure -> windowId
    is Event.WindowDraw -> windowId
    is Event.WindowFrameTick -> windowId
    is Event.WindowScaleChanged -> windowId
    is Event.WindowScreenChange -> windowId
    is Event.MouseDown -> windowId
    is Event.MouseUp -> windowId
    is Event.MouseMoved -> windowId
    is Event.ScrollWheel -> windowId
    is Event.DropPerformed -> windowId
    is Event.DragAndDropLeave -> windowId
    else -> null
}
