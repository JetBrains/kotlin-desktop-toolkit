@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.sample.compose.gtk

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
import androidx.compose.ui.unit.takeOrElse
import kotlinx.coroutines.CancellableContinuation
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import org.jetbrains.desktop.gtk.DragAndDropAction
import org.jetbrains.desktop.gtk.DragAndDropQueryData
import org.jetbrains.desktop.gtk.DragAndDropQueryResponse
import org.jetbrains.desktop.gtk.DragIconParams
import org.jetbrains.desktop.gtk.Event
import org.jetbrains.desktop.gtk.EventHandlerResult
import org.jetbrains.desktop.gtk.FileDialog
import org.jetbrains.desktop.gtk.RenderingMode
import org.jetbrains.desktop.gtk.RequestId
import org.jetbrains.desktop.gtk.StartDragAndDropParams
import org.jetbrains.desktop.gtk.TextInputSurroundingText
import org.jetbrains.desktop.gtk.Window
import org.jetbrains.desktop.gtk.WindowDecorationMode
import org.jetbrains.desktop.gtk.WindowId
import org.jetbrains.desktop.gtk.WindowParams
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
import org.jetbrains.skia.Surface
import org.jetbrains.skia.SurfaceColorFormat
import org.jetbrains.skia.SurfaceOrigin
import org.jetbrains.skia.makeGLWithInterface
import java.nio.file.Path
import java.util.concurrent.ConcurrentHashMap
import kotlin.coroutines.resume
import org.jetbrains.skia.Color as SkiaColor

/**
 * A kotlin-desktop-toolkit GTK window hosting a Compose scene.
 *
 * The frame loop is GTK's, and unlike the Wayland backend it has two halves: `Event.WindowFrameTick` marks
 * every beat of the frame clock and only asks for a frame when something invalidated the scene, and the
 * `Event.WindowDraw` that follows a [Window.requestRedraw] is where the pixels are produced. Rendering
 * happens inline, on the event loop thread, into the framebuffer the draw event names — GTK always renders
 * through OpenGL here, so there is no software path.
 *
 * What a GTK window does not offer, and what that costs the code below:
 *  - it cannot be positioned or resized by the application, so [position] is `Unspecified` and
 *    [requestPosition] / [requestSize] are no-ops;
 *  - its decoration is fixed when it is created. [requestDecoration] therefore cannot change it, and the
 *    authority on what the window ended up with is `Event.WindowConfigure`;
 *  - dragging and resizing a client-side-decorated window is GTK's own business, which is why — unlike the
 *    Wayland backend — there is no resize border here and [startInteractiveMove] has nothing to do.
 *
 * In exchange, a custom titlebar keeps GTK's own window buttons, and [customTitleBarInsets] reports the
 * room they need at each edge.
 */
internal class GtkWindow(
    private val application: GtkApplication,
    params: ComposeWindowParams,
    private val onCloseRequest: (WindowCloseRequestReason) -> Unit,
) : ComposeWindow,
    DragAndDropSourceWindow {
    override val nativeWindow: Window = application.nativeApplication.createWindow(
        WindowParams(
            windowId = application.allocateWindowId(),
            title = params.title,
            size = params.size.roundToLogicalSize(),
            minSize = params.minSize.roundToLogicalSize(),
            decorationMode = params.decoration.toDecorationMode(),
            // Let the toolkit pick between desktop GL and GL ES; it knows which one it managed to set up.
            renderingMode = RenderingMode.Auto,
        ),
    )

    internal val windowId: WindowId = nativeWindow.windowId

    private var isDisposed = false

    @Volatile
    private var isFrameRequested = false

    private val fileDialogResponses = ConcurrentHashMap<RequestId, CancellableContinuation<List<Path>?>>()

    // ----- Observable window state -----
    //
    // Every one of these mirrors something GTK owns, and is only written from `handleEvent`, when GTK
    // reports that it changed. The `request*` methods ask; they never assign.

    override var title: String
        get() = titleState
        set(value) {
            if (titleState == value) return
            titleState = value
            onNativeWindow { setTitle(value) }
        }

    private var titleState: String by mutableStateOf(params.title)

    /**
     * GTK does not tell a window where it is, and on Wayland it could not know; [requestPosition] and
     * [requestBounds] are no-ops to match.
     */
    override val position: DpOffset = DpOffset.Unspecified

    override var size: DpSize by mutableStateOf(params.size)
        private set

    /** Equal to [size]: the scene is drawn over the whole window, titlebar area included. */
    override val contentSize: DpSize get() = size

    override var isFocused: Boolean by mutableStateOf(false)
        private set

    override var placement: WindowPlacement by mutableStateOf(WindowPlacement.Floating)
        private set

    override var screen: GtkScreen by mutableStateOf(application.mainScreen)
        private set

    override var density: Density by mutableStateOf(application.mainScreen.density)
        private set

    override var decoration: WindowDecoration by mutableStateOf(params.decoration)
        private set

    override var minSize: DpSize by mutableStateOf(params.minSize)
        private set

    /** Only ever what was asked for: GTK has no API to cap a window's size. */
    override var maxSize: DpSize by mutableStateOf(DpSize(Dp.Infinity, Dp.Infinity))
        private set

    /** Likewise only ever what was asked for; GTK offers no way to forbid a resize. */
    override var isResizable: Boolean by mutableStateOf(params.isResizable)
        private set

    private var systemThemeOverride: SystemTheme? by mutableStateOf(null)

    override val systemTheme: SystemTheme get() = systemThemeOverride ?: application.systemTheme

    /** No API for it, and every GTK desktop in practice lays windows out left to right. */
    override val layoutDirection: LayoutDirection = LayoutDirection.Ltr

    /**
     * How much room GTK's own window buttons need on each edge of a custom titlebar.
     *
     * Unlike the Wayland backend this is a real value: GTK keeps drawing its buttons over a
     * [WindowDecoration.CustomTitleBar], so the content has to stay out from under them. `null` while the
     * window is server-decorated, or before the first configure.
     */
    override var customTitleBarInsets: Pair<Dp, Dp>? by mutableStateOf(null)
        private set

    /** GTK's, not ours: see [startInteractiveMove] and [GtkInputStateTracker]. */
    override val handlesCustomTitleBarGestures: Boolean = true

    // ----- Compose scene wiring -----

    private val pointerIcons = GtkPointerIcons { shape -> onNativeWindow { setPointerShape(shape) } }

    /** Per-window, like the rest of GTK input: hiding the pointer while typing is this window's business. */
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

    /**
     * Posted straight to this window: GTK arms the input method per window, so unlike Wayland there is no
     * seat-wide arbitration for the application to do.
     */
    private val textInput = GtkTextInput(
        onStartInputMethod = { context -> onNativeWindow { textInputEnable(context) } },
        onUpdateInputMethod = { context -> onNativeWindow { textInputUpdate(context) } },
        onStopInputMethod = { onNativeWindow { textInputDisable() } },
        density = { density },
    )

    private val platformContext = object : PlatformContext by PlatformContext.Empty() {
        override val windowInfo: WindowInfo get() = this@GtkWindow.windowInfo
        override val viewConfiguration: ViewConfiguration get() = this@GtkWindow.viewConfiguration
        override val inputModeManager get() = this@GtkWindow.inputModeManager
        override val dragAndDropManager: PlatformDragAndDropManager = KdtDragAndDropManager(this@GtkWindow)

        override fun setPointerIcon(pointerIcon: PointerIcon) {
            pointerIcons.apply(pointerIcon)
        }

        override suspend fun startInputMethod(request: PlatformTextInputMethodRequest): Nothing {
            return textInput.startInputMethod(request)
        }

        // The scene covers the whole window, and the window's own position is unknowable, so window and
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

    private val inputStateTracker = GtkInputStateTracker(
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

    internal val dragAndDropManager = GtkDragAndDropManager(
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

    /** GTK windows cannot position themselves; see [position]. */
    override fun requestPosition(position: DpOffset) = Unit

    /** GTK owns the window geometry; a client can only ever set a lower bound with [requestMinSize]. */
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

    /** Recorded for the content to read back; GTK has no maximum-size API to forward it to. */
    override fun requestMaxSize(maxSize: DpSize) {
        this.maxSize = DpSize(
            maxSize.width.takeOrElse { this.maxSize.width },
            maxSize.height.takeOrElse { this.maxSize.height },
        )
    }

    override fun requestResizable(resizable: Boolean) {
        isResizable = resizable
    }

    /**
     * Unlike Wayland, GTK lets a window activate itself without an activation token: it holds the X11 or
     * Wayland plumbing and passes `null` to mean "no token, do what you can".
     */
    override fun requestFocus() {
        onNativeWindow { activate(null) }
    }

    override fun requestBringToFront() {
        onNativeWindow { activate(null) }
    }

    override fun requestFocusAndBringToFront() {
        onNativeWindow { activate(null) }
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
            // There is no un-minimize: a minimized window comes back by being activated.
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

    /**
     * A no-op: GTK fixes a window's decoration when it is created, and the toolkit exposes no way to
     * change it afterwards. The [Window] composable calls this on every recomposition, so it has to be
     * harmless rather than an error; [decoration] keeps reporting what the window actually has.
     */
    override fun requestDecoration(decoration: WindowDecoration) = Unit

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
     * A no-op: GTK runs the move itself.
     *
     * A [WindowDecoration.CustomTitleBar] is still a real GTK titlebar — the application draws into it, but
     * GTK keeps the drag and the double-click-to-maximize gestures — so there is nothing for the content to
     * hand over. Content that calls this from a pointer handler simply gets GTK's behaviour instead.
     */
    override fun startInteractiveMove(pointerEvent: PointerEvent) = Unit

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
                    // The chooser picks files or directories, not both; files win when both were asked for.
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
     * The toolkit's file chooser is asynchronous — the call only starts the conversation and the selection
     * comes back later as an `Event.FileChooserResponse` — so this cannot be a blocking call, and being
     * `suspend` is also what makes it safe to call from the event loop thread itself.
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
            override val window: ComposeWindow get() = this@GtkWindow
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
                density = Density(event.newScale.rawValue.toFloat())
                scene.density = density
                scene.size = contentSizeInPx()
                isFrameRequested = true
                EventHandlerResult.Stop
            }

            is Event.WindowScreenChange -> {
                screen = application.screens.firstOrNull { it.nativeScreen.screenId == event.newScreenId } ?: screen
                EventHandlerResult.Stop
            }

            // Every beat of GTK's frame clock. Asking for a redraw is what produces the `WindowDraw`
            // below, so this is where a frame is spent or skipped depending on whether anything changed.
            is Event.WindowFrameTick -> {
                if (!isDisposed && isFrameRequested) {
                    onNativeWindow { requestRedraw() }
                }
                EventHandlerResult.Stop
            }

            is Event.WindowDraw -> {
                if (isDisposed) {
                    return EventHandlerResult.Continue
                }
                isFrameRequested = false
                draw(event)
                EventHandlerResult.Stop
            }

            is Event.WindowKeyboardEnter -> {
                // GTK arms the input method per window, and a window that lost focus had it disarmed
                // below; a text field that is still focused has to get it back.
                if (textInput.isSessionActive) {
                    textInput.currentContext?.let { context -> onNativeWindow { textInputEnable(context) } }
                }
                isFocused = true
                inputStateTracker.handleEvent(event, density)
            }

            is Event.WindowKeyboardLeave -> {
                // A composition left open across a focus change would be committed into whatever gets
                // focus next; disarming the input method is what tells it to abandon the composition.
                if (textInput.isSessionActive && textInput.hasPreeditString) {
                    onNativeWindow { textInputDisable() }
                    textInput.discardActivePreedit()
                }
                isFocused = false
                inputStateTracker.handleEvent(event, density)
            }

            is Event.KeyDown -> handleKeyDown(event)

            is Event.MouseDown -> {
                cancelCompositionOnPress()
                inputStateTracker.handleEvent(event, density)
            }

            is Event.TextInput -> {
                textInput.handleTextInputEvent(event)
                EventHandlerResult.Stop
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
        decoration = when (val decorationMode = event.decorationMode) {
            WindowDecorationMode.Server -> WindowDecoration.Decorated
            is WindowDecorationMode.CustomTitlebar -> WindowDecoration.CustomTitleBar(height = decorationMode.height.toDp())
        }
        // Both zero means GTK put no buttons of its own in the titlebar, which is also what a
        // server-decorated window reports; either way the content has nothing to keep clear.
        customTitleBarInsets = if (event.insetStart.width.rawLogical == 0 && event.insetEnd.width.rawLogical == 0) {
            null
        } else {
            event.insetStart.width.toDp() to event.insetEnd.width.toDp()
        }
        scene.size = contentSizeInPx()
        isFrameRequested = true
        return EventHandlerResult.Stop
    }

    /**
     * The order a key press is offered around in:
     *
     *  1. Compose goes first, so that preview handlers and shortcuts see every key. While an input method
     *     holds a composition the question does not arise — GTK routes the keys into the IME and they are
     *     never delivered to us at all, which is the part that makes dead keys and CJK input work.
     *  2. A key nobody claimed, that produced characters, and that has a text field to go into becomes
     *     text; see [GtkTextInput.commitCharacters] for why that has to be explicit.
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

    /**
     * Cancels an open composition when the user clicks somewhere.
     *
     * A composition left open across a click ends up committed into whatever the click moved the caret to.
     * Disarming and re-arming the input method is what tells it to start over.
     */
    private fun cancelCompositionOnPress() {
        if (!textInput.isSessionActive || !textInput.hasPreeditString) return
        val context = textInput.currentContext ?: return
        textInput.discardActivePreedit()
        onNativeWindow {
            textInputDisable()
            textInputEnable(context)
        }
    }

    /** Answers GTK's `getSurroundingText` callback for this window; see [GtkTextInput]. */
    internal fun currentTextInputSurroundingText(): TextInputSurroundingText? = textInput.surroundingTextForInputMethod()

    internal fun onClosed() {
        isDisposed = true
        cancelPendingFileDialogs()
    }

    internal fun queryDragAndDropTarget(query: DragAndDropQueryData): DragAndDropQueryResponse = dragAndDropManager.onQuery(query)

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
        // The drag icon is a surface of its own that GTK asks us to paint whenever it moves, so the
        // decoration is rasterized once here and replayed from `Event.DragIconDraw`.
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
            GtkApplication.ActiveDragSource(
                windowId = windowId,
                itemsEntry = itemsEntry,
                onTransferCompleted = { action -> transferData.onTransferCompleted?.invoke(action) },
                dragIconPngBytes = dragIconPngBytes,
            ),
        )
        onNativeWindow {
            // GTK takes over the pointer for the drag and sends MouseExited, where the tracker clears the
            // pressed buttons; nothing has to be cleared here.
            startDragAndDrop(
                StartDragAndDropParams(
                    mimeTypes = mimeTypes,
                    actions = actions,
                    dragIconParams = DragIconParams(
                        renderingMode = RenderingMode.Auto,
                        size = decorationSize.roundToNonEmptyLogicalSize(currentDensity),
                    ),
                ),
            )
        }
    }

    // ----- Frames -----

    private val lazyDirectContext = lazy {
        DirectContext.makeGLWithInterface(
            GLAssembledInterface.createFromNativePointers(
                ctxPtr = application.glProcFunc.ctxPtr,
                fPtr = application.glProcFunc.fPtr,
            ),
        )
    }

    /**
     * Renders one frame into the framebuffer GTK named.
     *
     * Three things differ from the Wayland backend's GL path and all three come from GTK owning the
     * surface: the framebuffer is GTK's rather than the default one, it is laid out top-down, and it
     * carries no stencil bits. The canvas is cleared to transparent so that the rounded corners and shadow
     * GTK composites around a client-side-decorated window are not painted over.
     */
    private fun draw(event: Event.WindowDraw) {
        val now = System.nanoTime()
        val width = event.size.width.rawPhysical
        val height = event.size.height.rawPhysical
        if (width <= 0 || height <= 0) return

        BackendRenderTarget.makeGL(
            width = width,
            height = height,
            sampleCnt = 1,
            stencilBits = 0,
            fbId = event.openGlDrawData.framebuffer,
            fbFormat = FramebufferFormat.GR_GL_RGBA8,
        ).use { renderTarget ->
            Surface.makeFromBackendRenderTarget(
                context = lazyDirectContext.value,
                rt = renderTarget,
                origin = SurfaceOrigin.TOP_LEFT,
                colorFormat = SurfaceColorFormat.RGBA_8888,
                colorSpace = ColorSpace.sRGB,
                surfaceProps = null,
            )!!.use { surface ->
                surface.canvas.clear(SkiaColor.TRANSPARENT)
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
        // `dispose` takes this window out of the application's map before GTK's WindowClosed arrives, so
        // `onClosed` will not run for it and anybody waiting on a dialog has to be released here.
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
}

/**
 * The decoration a window is created with. GTK cannot be told to change it later, so this is read once —
 * see [GtkWindow.requestDecoration].
 */
private fun WindowDecoration.toDecorationMode(): WindowDecorationMode = when (this) {
    WindowDecoration.Decorated -> WindowDecorationMode.Server
    is WindowDecoration.CustomTitleBar -> WindowDecorationMode.CustomTitlebar(height.roundToLogicalPixelsInt())
}
