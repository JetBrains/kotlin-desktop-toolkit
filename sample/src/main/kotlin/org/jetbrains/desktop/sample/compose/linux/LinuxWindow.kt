@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.sample.compose.linux

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.LocalSystemTheme
import androidx.compose.ui.SystemTheme
import androidx.compose.ui.draganddrop.DragAndDropTransferData
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.asComposeCanvas
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.input.InputMode
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerIcon
import androidx.compose.ui.input.pointer.PointerType
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.PlatformContext
import androidx.compose.ui.platform.PlatformDragAndDropManager
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.ViewConfiguration
import androidx.compose.ui.platform.WindowInfo
import androidx.compose.ui.scene.CanvasLayersComposeScene
import androidx.compose.ui.scene.ComposeScene
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpRect
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.isFinite
import androidx.compose.ui.unit.takeOrElse
import kotlinx.coroutines.CancellableContinuation
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import org.jetbrains.desktop.linux.DragAndDropAction
import org.jetbrains.desktop.linux.DragAndDropQueryData
import org.jetbrains.desktop.linux.DragAndDropQueryResponse
import org.jetbrains.desktop.linux.DragIconParams
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.FileDialog
import org.jetbrains.desktop.linux.LogicalPixelsInt
import org.jetbrains.desktop.linux.RenderingMode
import org.jetbrains.desktop.linux.RequestId
import org.jetbrains.desktop.linux.StartDragAndDropParams
import org.jetbrains.desktop.linux.Window
import org.jetbrains.desktop.linux.WindowDecorationMode
import org.jetbrains.desktop.linux.WindowFrame
import org.jetbrains.desktop.linux.WindowFrameSide
import org.jetbrains.desktop.linux.WindowId
import org.jetbrains.desktop.linux.WindowParams
import org.jetbrains.desktop.sample.compose.ClipboardItemsEntry
import org.jetbrains.desktop.sample.compose.ComposeWindow
import org.jetbrains.desktop.sample.compose.ComposeWindowParams
import org.jetbrains.desktop.sample.compose.ComposeWindowScope
import org.jetbrains.desktop.sample.compose.DragAndDropImage
import org.jetbrains.desktop.sample.compose.DragAndDropSourceWindow
import org.jetbrains.desktop.sample.compose.KdtDispatcher
import org.jetbrains.desktop.sample.compose.KdtDragAndDropManager
import org.jetbrains.desktop.sample.compose.KdtDragAndDropTransferable
import org.jetbrains.desktop.sample.compose.KdtInputModeManager
import org.jetbrains.desktop.sample.compose.LocalComposeWindow
import org.jetbrains.desktop.sample.compose.WindowCloseRequestReason
import org.jetbrains.desktop.sample.compose.WindowDecoration
import org.jetbrains.desktop.sample.compose.WindowPlacement
import org.jetbrains.desktop.sample.compose.decodeFileChooserPath
import org.jetbrains.desktop.sample.compose.linuxMimeTypes
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.ColorSpace
import org.jetbrains.skia.DirectContext
import org.jetbrains.skia.FramebufferFormat
import org.jetbrains.skia.GLAssembledInterface
import org.jetbrains.skia.ImageInfo
import org.jetbrains.skia.Surface
import org.jetbrains.skia.SurfaceColorFormat
import org.jetbrains.skia.SurfaceOrigin
import org.jetbrains.skia.makeGLWithInterface
import java.nio.file.Path
import java.util.concurrent.ConcurrentHashMap
import kotlin.coroutines.resume
import org.jetbrains.skia.Color as SkiaColor

/**
 * A kotlin-desktop-toolkit Wayland surface hosting a Compose scene.
 *
 * The frame loop is the compositor's, not ours: it hands out an `Event.WindowDraw` for every frame the
 * surface is allowed to present, and the only decision left is whether there is anything new to draw —
 * hence the `isFrameRequested` flag the scene's `invalidate` sets. Rendering happens inline, on the event
 * loop thread, straight into the buffer the event carries (a shared-memory one when the compositor asked
 * for software rendering, the surface's EGL framebuffer otherwise). There is no display link and no render
 * thread: presenting late means the compositor skips the frame, not that frames queue up.
 *
 * Two things about a Wayland window have no counterpart on the other platforms and shape most of what
 * looks unusual below:
 *  - a surface never learns where it is on screen, so [position] is `Unspecified` and cannot be requested;
 *  - the compositor, not the application, decides who draws the window frame. When it decides the
 *    application does — which GNOME always does — [decoration] reports a [WindowDecoration.CustomTitleBar]
 *    even if a decorated window was asked for, so the content knows it has to draw one, and this class
 *    takes over the window's own edges for resizing (see `resizeEdgeAt`).
 */
internal class LinuxWindow(
    private val application: LinuxApplication,
    params: ComposeWindowParams,
    private val onCloseRequest: (WindowCloseRequestReason) -> Unit,
) : ComposeWindow,
    DragAndDropSourceWindow {
    override val nativeWindow: Window = application.nativeApplication.createWindow(
        WindowParams(
            windowId = application.allocateWindowId(),
            appId = application.identifier,
            title = params.title,
            size = params.size.roundToLogicalSize(),
            minSize = params.minSize.roundToLogicalSize(),
            preferClientSideDecoration = params.decoration !is WindowDecoration.Decorated,
            renderingMode = RenderingMode.Auto,
            // A frame with no padding keeps the surface exactly the size of the window, so the scene can
            // cover all of it: the shadow margin a client-side-decorated window would normally reserve is
            // the application's to draw, and this sample does not draw one.
            clientSideDecorationFrame = WindowFrame.all(
                WindowFrameSide(padding = LogicalPixelsInt.Zero, resizerThickness = LogicalPixelsInt.Zero),
            ),
        ),
    )

    internal val windowId: WindowId = nativeWindow.windowId

    private var isDisposed = false

    @Volatile
    private var isFrameRequested = false

    private val fileDialogResponses = ConcurrentHashMap<RequestId, CancellableContinuation<List<Path>?>>()

    // ----- Observable window state -----
    //
    // Every one of these mirrors something the compositor owns, and is only written from `handleEvent`,
    // when the compositor reports that it changed. The `request*` methods ask; they never assign.

    override var title: String
        get() = titleState
        set(value) {
            if (titleState == value) return
            titleState = value
            onNativeWindow { setTitle(value) }
        }

    private var titleState: String by mutableStateOf(params.title)

    /**
     * Wayland deliberately hides the global desktop layout from clients, so there is no position to report
     * and none to ask for. `Unspecified` is what says so; [requestPosition] and [requestBounds] are no-ops.
     */
    override val position: DpOffset = DpOffset.Unspecified

    override var size: DpSize by mutableStateOf(params.size)
        private set

    /** Equal to [size]: the surface has no frame padding, so all of it is content. */
    override val contentSize: DpSize get() = size

    override var isFocused: Boolean by mutableStateOf(false)
        private set

    override var placement: WindowPlacement by mutableStateOf(WindowPlacement.Floating)
        private set

    override var screen: LinuxScreen by mutableStateOf(application.mainScreen)
        private set

    override var density: Density by mutableStateOf(Density(1f))
        private set

    override var decoration: WindowDecoration by mutableStateOf(params.decoration)
        private set

    override var minSize: DpSize by mutableStateOf(params.minSize)
        private set

    override var maxSize: DpSize by mutableStateOf(DpSize(Dp.Infinity, Dp.Infinity))
        private set

    /**
     * Only ever what was asked for: Wayland has no way to tell the compositor a window may not be resized,
     * and the compositor's own answer (`WindowCapabilities`) is about maximize and fullscreen, not about
     * dragging an edge.
     */
    override var isResizable: Boolean by mutableStateOf(params.isResizable)
        private set

    private var systemThemeOverride: SystemTheme? by mutableStateOf(null)

    override val systemTheme: SystemTheme get() = systemThemeOverride ?: application.systemTheme

    /** No API for it, and every compositor in practice lays windows out left to right. */
    override val layoutDirection: LayoutDirection = LayoutDirection.Ltr

    /**
     * `null` on Wayland: when the application draws the titlebar it draws all of it, so there are no
     * platform-owned buttons to leave room for. That also tells the content it has to offer a close
     * affordance of its own.
     */
    override val customTitleBarInsets: Pair<Dp, Dp>? = null

    /** The height to report once the compositor tells us we are the one drawing the titlebar. */
    private var customTitleBarHeight: Dp = (params.decoration as? WindowDecoration.CustomTitleBar)?.height ?: DefaultTitleBarHeight

    /** Set while the pointer sits on the resize border, so the shape can be handed back to Compose on exit. */
    private var isPointerOnResizeBorder = false

    // ----- Compose scene wiring -----

    private val pointerIcons = LinuxPointerIcons { shape -> onNativeWindow { setPointerShape(shape) } }

    /**
     * Per-window, unlike macOS: hiding the pointer while the user types is a property of one surface here,
     * not of the whole application.
     */
    private val inputModeManager = KdtInputModeManager(InputMode.Touch) { inputMode ->
        pointerIcons.setHiddenUntilPointerMoves(inputMode == InputMode.Keyboard)
    }

    private val viewConfiguration = object : ViewConfiguration {
        override val longPressTimeoutMillis: Long = 500
        override val doubleTapTimeoutMillis: Long get() = application.doubleClickIntervalMillis
        override val doubleTapMinTimeMillis: Long = 40
        override val touchSlop: Float get() = with(density) { 18.dp.toPx() }
    }

    private val windowInfo = object : WindowInfo {
        override val isWindowFocused: Boolean get() = isFocused
        override val keyboardModifiers get() = inputStateTracker.keyboardModifiers
        override val containerSize: IntSize get() = contentSizeInPx()
    }

    private val textInput = LinuxTextInput(
        onStartInputMethod = { context -> application.startTextInput(windowId, context) },
        onUpdateInputMethod = { context -> application.updateTextInput(windowId, context) },
        onStopInputMethod = { application.stopTextInput(windowId) },
        density = { density },
    )

    private val platformContext = object : PlatformContext by PlatformContext.Empty() {
        override val windowInfo: WindowInfo get() = this@LinuxWindow.windowInfo
        override val viewConfiguration: ViewConfiguration get() = this@LinuxWindow.viewConfiguration
        override val inputModeManager get() = this@LinuxWindow.inputModeManager
        override val dragAndDropManager: PlatformDragAndDropManager = KdtDragAndDropManager(this@LinuxWindow)

        override fun setPointerIcon(pointerIcon: PointerIcon) {
            pointerIcons.apply(pointerIcon)
        }

        override suspend fun startInputMethod(request: PlatformTextInputMethodRequest): Nothing {
            return textInput.startInputMethod(request)
        }

        // The scene covers the whole surface, and the surface's own position is unknowable, so window and
        // screen coordinates are both just the local ones.
        override fun convertLocalToWindowPosition(localPosition: Offset): Offset = localPosition

        override fun convertWindowToLocalPosition(positionInWindow: Offset): Offset = positionInWindow

        override fun convertLocalToScreenPosition(localPosition: Offset): Offset = localPosition

        override fun convertScreenToLocalPosition(positionOnScreen: Offset): Offset = positionOnScreen
    }

    private val scene: ComposeScene = CanvasLayersComposeScene(
        density = density,
        layoutDirection = layoutDirection,
        size = contentSizeInPx(),
        coroutineContext = KdtDispatcher,
        platformContext = platformContext,
        invalidate = { isFrameRequested = true },
    )

    private val inputStateTracker = LinuxInputStateTracker(
        inputModeManager = inputModeManager,
        sendPointerEvent = { eventType, position, scrollDelta, timeMillis, buttons, modifiers, nativeEvent, button ->
            scene.sendPointerEvent(
                eventType = eventType,
                position = position,
                scrollDelta = scrollDelta,
                timeMillis = timeMillis,
                type = PointerType.Mouse,
                buttons = buttons,
                keyboardModifiers = modifiers,
                nativeEvent = nativeEvent,
                button = button,
            )
        },
        sendKeyEvent = { keyEvent ->
            onPreviewKeyEvent(keyEvent) || scene.sendKeyEvent(keyEvent) || onKeyEvent(keyEvent)
        },
    )

    internal val dragAndDropManager = LinuxDragAndDropManager(
        rootDragAndDropNode = { scene.rootDragAndDropNode },
        density = { density },
    )

    private var onPreviewKeyEvent: (KeyEvent) -> Boolean = { false }
    private var onKeyEvent: (KeyEvent) -> Boolean = { false }

    init {
        // Registered last: from here on the event loop can deliver events to `handleEvent`, so everything
        // it touches has to be initialized.
        application.attachWindow(this)
    }

    // ----- Requests -----

    /** Wayland clients cannot position themselves; see [position]. */
    override fun requestPosition(position: DpOffset) = Unit

    /** A Wayland client can only ever ask for a size *range*; the compositor picks the size. */
    override fun requestSize(size: DpSize) = Unit

    override fun requestBounds(bounds: DpRect) = Unit

    override fun requestMinSize(minSize: DpSize) {
        val resolved = DpSize(
            minSize.width.takeOrElse { this.minSize.width },
            minSize.height.takeOrElse { this.minSize.height },
        )
        this.minSize = resolved
        onNativeWindow { setMinSize(resolved.roundToLogicalSize()) }
    }

    override fun requestMaxSize(maxSize: DpSize) {
        val resolved = DpSize(
            maxSize.width.takeOrElse { this.maxSize.width },
            maxSize.height.takeOrElse { this.maxSize.height },
        )
        this.maxSize = resolved
        if (resolved.width.isFinite && resolved.height.isFinite) {
            onNativeWindow { setMaxSize(resolved.roundToLogicalSize()) }
        }
    }

    override fun requestResizable(resizable: Boolean) {
        isResizable = resizable
    }

    /**
     * Focus is not the application's to take on Wayland — a compositor that let any client steal it would
     * be a security problem. What a client can do is ask for an *activation token* and hand it back, which
     * the compositor honours when it believes the request came from user intent; that round trip is what
     * [LinuxApplication.requestWindowActivation] runs.
     */
    override fun requestFocus() {
        application.requestWindowActivation(windowId)
    }

    override fun requestBringToFront() {
        application.requestWindowActivation(windowId)
    }

    override fun requestFocusAndBringToFront() {
        application.requestWindowActivation(windowId)
    }

    override fun requestPlacement(placement: WindowPlacement) {
        onNativeWindow {
            when (placement) {
                WindowPlacement.Floating -> {
                    unmaximize()
                    unsetFullScreen()
                }
                WindowPlacement.Maximized -> maximize()
                WindowPlacement.Fullscreen -> setFullScreen()
            }
        }
    }

    override fun requestMinimized(minimized: Boolean) {
        if (minimized) {
            onNativeWindow { minimize() }
        } else {
            // There is no un-minimize: a minimized surface comes back by being activated.
            requestFocusAndBringToFront()
        }
    }

    /**
     * Purely local. The desktop-wide colour scheme is a setting rather than something a window carries, so
     * overriding it only changes what this window's content is told.
     */
    override fun requestSystemTheme(systemTheme: SystemTheme?) {
        systemThemeOverride = systemTheme
    }

    override fun requestDecoration(decoration: WindowDecoration) {
        if (decoration is WindowDecoration.CustomTitleBar) {
            customTitleBarHeight = decoration.height
        }
        if (decoration == requestedDecoration) return
        requestedDecoration = decoration
        onNativeWindow { setPreferClientSideDecoration(decoration !is WindowDecoration.Decorated) }
    }

    private var requestedDecoration: WindowDecoration = params.decoration

    override fun requestClose() {
        if (!isDisposed) {
            onCloseRequest(WindowCloseRequestReason.UserRequest)
        }
    }

    /**
     * Asks the application's owner to close this window because the application is quitting. Separate from
     * the user-driven path only in the reason it reports.
     */
    internal fun requestCloseFromQuit() {
        if (!isDisposed) {
            onCloseRequest(WindowCloseRequestReason.ApplicationQuit)
        }
    }

    /**
     * Hands the window to the compositor's move loop.
     *
     * The compositor will only do it for a press it can identify, so the serial of the originating
     * `MouseDown` has to be handed back to it — which is why this takes the pointer event rather than
     * reading "the current event" the way macOS can. A move asked for from anything but a press is ignored,
     * and that is what lets content call this from a pointer handler that sees every event.
     */
    override fun startInteractiveMove(pointerEvent: PointerEvent) {
        val serial = (pointerEvent.nativeEvent as? Event.MouseDown)?.serial ?: return
        onNativeWindow { startMove(serial) }
    }

    override suspend fun showOpenFileDialog(
        title: String,
        allowsMultipleSelections: Boolean,
        canChooseFiles: Boolean,
        canChooseDirectories: Boolean,
    ): List<Path> {
        return showFileDialog {
            nativeWindow.showOpenFileDialog(
                FileDialog.CommonDialogParams(modal = true, title = title, acceptLabel = null, currentFolder = null),
                FileDialog.OpenDialogParams(
                    // The portal picks files or directories, not both; files win when both were asked for.
                    selectDirectories = canChooseDirectories && !canChooseFiles,
                    allowsMultipleSelections = allowsMultipleSelections,
                ),
            )
        }.orEmpty()
    }

    override suspend fun showSaveFileDialog(title: String, nameFieldStringValue: String?): Path? {
        return showFileDialog {
            nativeWindow.showSaveFileDialog(
                FileDialog.CommonDialogParams(modal = true, title = title, acceptLabel = null, currentFolder = null),
                FileDialog.SaveDialogParams(nameFieldStringValue = nameFieldStringValue),
            )
        }?.firstOrNull()
    }

    /**
     * Issues one file-chooser request and suspends until its response event arrives.
     *
     * The toolkit's file chooser is asynchronous — the call only starts an XDG portal conversation and the
     * selection comes back later as an `Event.FileChooserResponse` — so this cannot be a blocking call, and
     * being `suspend` is also what makes it safe to call from the event loop thread itself.
     *
     * `null` means the request was refused outright; an empty list means the user cancelled.
     */
    private suspend fun showFileDialog(request: () -> RequestId?): List<Path>? = suspendCancellableCoroutine { continuation ->
        application.onEventLoop {
            // The caller may have been cancelled between suspending and this block running; don't open
            // a chooser nobody is listening to.
            if (!continuation.isActive) return@onEventLoop
            if (isDisposed) {
                continuation.resume(null)
                return@onEventLoop
            }
            val requestId = request()
            if (requestId == null) {
                continuation.resume(null)
            } else {
                fileDialogResponses[requestId] = continuation
                continuation.invokeOnCancellation { fileDialogResponses.remove(requestId) }
            }
        }
    }

    // ----- Content -----

    override fun setContent(
        onPreviewKeyEvent: (KeyEvent) -> Boolean,
        onKeyEvent: (KeyEvent) -> Boolean,
        content: @Composable ComposeWindowScope.() -> Unit,
    ) {
        this.onPreviewKeyEvent = onPreviewKeyEvent
        this.onKeyEvent = onKeyEvent
        val windowScope = object : ComposeWindowScope {
            override val window: ComposeWindow get() = this@LinuxWindow
        }
        scene.setContent {
            CompositionLocalProvider(
                LocalComposeWindow provides this,
                LocalSystemTheme provides systemTheme,
                // The scene defaults these to AWT-backed implementations; the toolkit's own have to win.
                LocalClipboard provides application.clipboard,
                LocalUriHandler provides application.uriHandler,
            ) {
                windowScope.content()
            }

            // After a relayout, whatever is now under a stationary pointer has to learn that it is hovered.
            // Queued through the scene's dispatcher rather than run inline, so that pointer input modifiers
            // introduced by this very composition are already subscribed.
            rememberCoroutineScope().launch {
                inputStateTracker.prepareSyntheticMoveAfterRelayout()
                    ?.let(inputStateTracker::sendSyntheticMoveIfCurrent)
            }
        }
        isFrameRequested = true
    }

    // ----- Events -----

    internal fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.WindowConfigure -> handleConfigure(event)

            is Event.WindowScaleChanged -> {
                density = Density(event.newScale.rawScale.toFloat())
                scene.density = density
                scene.size = contentSizeInPx()
                isFrameRequested = true
                EventHandlerResult.Stop
            }

            is Event.WindowScreenChange -> {
                screen = application.screens.firstOrNull { it.nativeScreen.screenId == event.newScreenId } ?: screen
                EventHandlerResult.Stop
            }

            is Event.WindowDraw -> {
                // WindowDraw arrives on every compositor frame, so render only when something invalidated.
                // The toolkit re-arms the frame callback unconditionally, so skipping cannot stall the loop.
                if (isDisposed || !isFrameRequested) {
                    return EventHandlerResult.Continue
                }
                isFrameRequested = false
                draw(event)
                EventHandlerResult.Stop
            }

            is Event.WindowCloseRequest -> {
                onCloseRequest(WindowCloseRequestReason.UserRequest)
                EventHandlerResult.Stop
            }

            is Event.WindowKeyboardEnter -> {
                isFocused = true
                inputStateTracker.handleEvent(event, density)
            }

            is Event.WindowKeyboardLeave -> {
                isFocused = false
                inputStateTracker.handleEvent(event, density)
            }

            is Event.KeyDown -> handleKeyDown(event)

            is Event.MouseDown -> {
                val edge = resizeEdgeIfClientSideDecorated(event.locationInWindow.toDpOffset())
                if (edge != null) {
                    onNativeWindow { startResize(event.serial, edge) }
                    EventHandlerResult.Stop
                } else {
                    inputStateTracker.handleEvent(event, density)
                }
            }

            is Event.MouseMoved -> {
                updateResizeBorderPointerShape(event.locationInWindow.toDpOffset())
                inputStateTracker.handleEvent(event, density)
            }

            is Event.MouseExited -> {
                releaseResizeBorderPointerShape()
                inputStateTracker.handleEvent(event, density)
            }

            is Event.FileChooserResponse -> {
                val continuation = fileDialogResponses.remove(event.requestId)
                if (continuation == null) {
                    // Somebody else's request: the event carries no window id, so every window is offered it.
                    EventHandlerResult.Continue
                } else {
                    continuation.resume(event.files.map(::decodeFileChooserPath))
                    EventHandlerResult.Stop
                }
            }

            is Event.DropPerformed -> {
                dragAndDropManager.onDrop(event)
                EventHandlerResult.Stop
            }

            is Event.DragAndDropLeave -> {
                dragAndDropManager.onLeave()
                EventHandlerResult.Stop
            }

            else -> inputStateTracker.handleEvent(event, density)
        }
    }

    private fun handleConfigure(event: Event.WindowConfigure): EventHandlerResult {
        size = event.size.toDpSize()
        isFocused = event.active
        placement = when {
            event.fullscreen -> WindowPlacement.Fullscreen
            event.maximized -> WindowPlacement.Maximized
            else -> WindowPlacement.Floating
        }
        decoration = when (event.decorationMode) {
            WindowDecorationMode.Server -> WindowDecoration.Decorated
            // The compositor will not decorate this window, so the content has to. Reported even when a
            // decorated window was asked for — on GNOME that is the only answer there is.
            is WindowDecorationMode.Client -> WindowDecoration.CustomTitleBar(height = customTitleBarHeight)
        }
        scene.size = contentSizeInPx()
        isFrameRequested = true
        return EventHandlerResult.Stop
    }

    /**
     * The order a key press is offered around in:
     *
     *  1. Compose goes first, so that preview handlers and shortcuts see every key. While an input method
     *     holds a composition the question does not arise — the compositor grabs the keyboard for the IME
     *     and the keys are never delivered to us at all, which is the part of the protocol that makes dead
     *     keys and CJK input work.
     *  2. A key nobody claimed, that produced characters, and that has a text field to go into becomes
     *     text; see [LinuxTextInput.commitCharacters] for why that has to be explicit.
     */
    private fun handleKeyDown(event: Event.KeyDown): EventHandlerResult {
        val result = inputStateTracker.handleEvent(event, density)
        if (result == EventHandlerResult.Stop) {
            return result
        }
        return if (textInput.commitCharacters(event, inputStateTracker.keyboardModifiers)) {
            EventHandlerResult.Stop
        } else {
            result
        }
    }

    internal fun handleTextInput(event: Event.TextInput): EventHandlerResult {
        textInput.handleTextInputEvent(event)
        return EventHandlerResult.Stop
    }

    internal fun hasPreeditString(): Boolean = textInput.isSessionActive && textInput.hasPreeditString

    internal fun discardActivePreedit() {
        textInput.discardActivePreedit()
    }

    internal fun onClosed() {
        isDisposed = true
        cancelPendingFileDialogs()
    }

    internal fun activate(activationToken: String) {
        onNativeWindow { activate(activationToken) }
    }

    internal fun queryDragAndDropTarget(query: DragAndDropQueryData): DragAndDropQueryResponse = dragAndDropManager.onQuery(query)

    // ----- Resize border -----

    private fun resizeEdgeIfClientSideDecorated(offset: DpOffset) =
        if (decoration is WindowDecoration.CustomTitleBar && isResizable && placement == WindowPlacement.Floating) {
            resizeEdgeAt(offset, size)
        } else {
            null
        }

    private fun updateResizeBorderPointerShape(offset: DpOffset) {
        val edge = resizeEdgeIfClientSideDecorated(offset)
        if (edge == null) {
            releaseResizeBorderPointerShape()
        } else {
            isPointerOnResizeBorder = true
            pointerIcons.override(edge.toPointerShape())
        }
    }

    private fun releaseResizeBorderPointerShape() {
        if (!isPointerOnResizeBorder) return
        isPointerOnResizeBorder = false
        pointerIcons.override(null)
    }

    // ----- Drag and drop -----

    override fun startDragSession(
        offset: Offset,
        transferData: DragAndDropTransferData,
        decorationSize: Size,
        drawDragDecoration: DrawScope.() -> Unit,
    ) {
        val itemsEntry = (transferData.transferable as? KdtDragAndDropTransferable)
            ?.clipboardEntry
            ?.nativeClipEntry as? ClipboardItemsEntry
            ?: return
        val mimeTypes = itemsEntry.linuxMimeTypes()
        if (mimeTypes.isEmpty()) return

        val currentDensity = density
        // The drag icon is a separate surface the compositor asks us to paint, so the decoration is
        // rasterized once here and replayed from `Event.DragIconDraw`.
        val dragIconPngBytes = DragAndDropImage(
            decorationSize,
            currentDensity,
            scene.layoutDirection,
            drawDragDecoration,
        ).encodeToPng()

        val actions = transferData.supportedActions
            .mapNotNull { it.toDragAndDropAction() }
            .toSet()
            // A drag that allows nothing would be refused by every target.
            .ifEmpty { setOf(DragAndDropAction.Copy) }

        application.beginDragSession(
            LinuxApplication.ActiveDragSource(
                windowId = windowId,
                itemsEntry = itemsEntry,
                onTransferCompleted = { action -> transferData.onTransferCompleted?.invoke(action) },
                dragIconPngBytes = dragIconPngBytes,
            ),
        )
        onNativeWindow {
            // The compositor takes over the pointer for the drag and sends MouseExited, where the tracker
            // clears the pressed buttons; nothing has to be cleared here.
            startDragAndDrop(
                StartDragAndDropParams(
                    mimeTypes = mimeTypes,
                    actions = actions,
                    dragIconParams = DragIconParams(
                        renderingMode = RenderingMode.Software,
                        size = decorationSize.roundToNonEmptyLogicalSize(currentDensity),
                    ),
                ),
            )
        }
    }

    // ----- Frames -----

    private val lazyDirectContext = lazy {
        val eglProcFunc = checkNotNull(application.nativeApplication.getEglProcFunc()) {
            "The Linux backend did not provide an EGL function table, so the window cannot render with OpenGL"
        }
        DirectContext.makeGLWithInterface(
            GLAssembledInterface.createFromNativePointers(ctxPtr = eglProcFunc.ctxPtr, fPtr = eglProcFunc.fPtr),
        )
    }

    private fun draw(event: Event.WindowDraw) {
        val now = System.nanoTime()
        val width = event.size.width.rawPhysical
        val height = event.size.height.rawPhysical
        if (width <= 0 || height <= 0) return

        val softwareDrawData = event.softwareDrawData
        if (softwareDrawData != null) {
            // The compositor gave us a shared-memory buffer to paint into directly.
            Surface.makeRasterDirect(
                imageInfo = ImageInfo.makeN32Premul(width, height, ColorSpace.sRGB),
                pixelsPtr = softwareDrawData.canvas,
                rowBytes = softwareDrawData.stride,
            ).use { surface ->
                surface.canvas.clear(SkiaColor.WHITE)
                scene.render(surface.canvas.asComposeCanvas(), now)
                surface.flushAndSubmit()
            }
            return
        }

        BackendRenderTarget.makeGL(
            width = width,
            height = height,
            sampleCnt = 1,
            stencilBits = 8,
            fbId = 0,
            fbFormat = FramebufferFormat.GR_GL_RGBA8,
        ).use { renderTarget ->
            Surface.makeFromBackendRenderTarget(
                context = lazyDirectContext.value,
                rt = renderTarget,
                origin = SurfaceOrigin.BOTTOM_LEFT,
                colorFormat = SurfaceColorFormat.RGBA_8888,
                colorSpace = ColorSpace.sRGB,
                surfaceProps = null,
            )!!.use { surface ->
                surface.canvas.clear(SkiaColor.WHITE)
                scene.render(surface.canvas.asComposeCanvas(), now)
                surface.flushAndSubmit()
            }
        }
    }

    // ----- Geometry -----

    private fun contentSizeInPx(): IntSize = contentSize.roundToIntSize(density)

    override fun dispose() {
        if (isDisposed) return
        isDisposed = true
        application.detachWindow(this)
        // `dispose` takes this window out of the application's map before the compositor's WindowClosed
        // arrives, so `onClosed` will not run for it and anybody waiting on a dialog has to be released here.
        cancelPendingFileDialogs()
        scene.close()
        if (lazyDirectContext.isInitialized()) {
            lazyDirectContext.value.close()
        }
        // Posted rather than run through `onNativeWindow`, which drops work once `isDisposed` is set.
        application.onEventLoop { nativeWindow.close() }
    }

    private fun cancelPendingFileDialogs() {
        fileDialogResponses.values.forEach { it.cancel() }
        fileDialogResponses.clear()
    }

    private inline fun onNativeWindow(crossinline block: Window.() -> Unit) {
        if (isDisposed) return
        application.onEventLoop {
            if (!isDisposed) {
                nativeWindow.block()
            }
        }
    }

    private companion object {
        /** What a titlebar the sample has to draw itself is worth, when nobody asked for a specific height. */
        val DefaultTitleBarHeight = 38.dp
    }
}
