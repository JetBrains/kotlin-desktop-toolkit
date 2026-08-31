@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.compose.macos

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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asComposeCanvas
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.toArgb
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
import androidx.compose.ui.unit.height
import androidx.compose.ui.unit.isSpecified
import androidx.compose.ui.unit.takeOrElse
import androidx.compose.ui.unit.width
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.jetbrains.desktop.compose.ClipboardItemsEntry
import org.jetbrains.desktop.compose.ComposeWindow
import org.jetbrains.desktop.compose.ComposeWindowParams
import org.jetbrains.desktop.compose.ComposeWindowScope
import org.jetbrains.desktop.compose.DragAndDropImage
import org.jetbrains.desktop.compose.DragAndDropSourceWindow
import org.jetbrains.desktop.compose.KdtDispatcher
import org.jetbrains.desktop.compose.KdtDragAndDropManager
import org.jetbrains.desktop.compose.KdtDragAndDropTransferable
import org.jetbrains.desktop.compose.KdtImmediateDispatcher
import org.jetbrains.desktop.compose.LocalComposeWindow
import org.jetbrains.desktop.compose.WindowCloseRequestReason
import org.jetbrains.desktop.compose.WindowDecoration
import org.jetbrains.desktop.compose.WindowPlacement
import org.jetbrains.desktop.macos.AppMenuManager
import org.jetbrains.desktop.macos.Appearance
import org.jetbrains.desktop.macos.DisplayLink
import org.jetbrains.desktop.macos.DraggingItem
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.FileDialog
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.Image
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalRect
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.Screen
import org.jetbrains.desktop.macos.TextDirection
import org.jetbrains.desktop.macos.TitlebarConfiguration
import org.jetbrains.desktop.macos.Window
import org.jetbrains.desktop.macos.WindowEvent
import org.jetbrains.desktop.macos.WindowId
import org.jetbrains.skia.PictureRecorder
import org.jetbrains.skia.Rect
import java.nio.file.Path
import kotlin.math.ceil

/**
 * A kotlin-desktop-toolkit window hosting a Compose scene.
 *
 * The frame loop has three participants:
 *  - the scene's `invalidate` callback records that Compose wants a new frame,
 *  - a [DisplayLink] tied to the window's screen decides when that frame may be produced, and
 *  - [MetalViewContext] submits it to the GPU off the event loop thread.
 *
 * [DisplayLinkFramePump] keeps at most one frame in flight, so a slow GPU throttles the frame rate
 * instead of queueing frames without bound. CoreAnimation can also ask the layer to redraw
 * synchronously — most importantly while resizing, where a frame recorded for the old size would be
 * dropped by the render thread — which is what [repaintSynchronously] is for.
 */
internal class MacOsWindow(
    private val application: MacOsApplication,
    params: ComposeWindowParams,
    private val onCloseRequest: (WindowCloseRequestReason) -> Unit,
) : ComposeWindow,
    DragAndDropSourceWindow {
    override val nativeWindow: Window = Window.create(
        Window.WindowParams(
            origin = (if (params.position.isSpecified) params.position else DpOffset.Zero).toLogicalPoint(),
            size = params.size.toLogicalSize(),
            title = params.title,
            isResizable = params.isResizable,
        ),
    ).apply {
        minSize = params.minSize.toLogicalSize()
        registerForDraggedTypes(
            listOf(
                UniformTypeIdentifiers.UTF8_PLAIN_TEXT,
                UniformTypeIdentifiers.HTML,
                UniformTypeIdentifiers.FILE,
                UniformTypeIdentifiers.PNG,
            ),
        )
    }

    internal val windowId: WindowId = nativeWindow.windowId()

    private var isDisposed = false

    @Volatile
    private var isFrameRequested = false

    private val viewContext = application.gpuContext.createMetalViewContext()
    private val pictureRecorder = PictureRecorder()
    private var displayLink: DisplayLink? = null
    private val framePump = DisplayLinkFramePump<PresentablePicture>(
        isDisposed = { isDisposed },
        isFrameRequested = { isFrameRequested },
        setFrameRequested = { isFrameRequested = it },
        dispatchOnMain = { block -> GrandCentralDispatch.dispatchOnMain(highPriority = true, f = block) },
        recordFrame = ::recordFrame,
        presentAsync = { picture, onComplete ->
            viewContext.presentAsync(picture, waitForCATransaction = false, onComplete = onComplete)
        },
    )

    // ----- Observable window state -----
    //
    // Every one of these mirrors something the platform owns, and is only written from `handleEvent`,
    // when the platform reports that it changed. The `request*` methods ask; they never assign.

    override var title: String
        get() = titleState
        set(value) {
            if (titleState == value) return
            titleState = value
            if (!isDisposed) {
                nativeWindow.title = value
            }
        }

    private var titleState: String by mutableStateOf(params.title)

    override var position: DpOffset by mutableStateOf(nativeWindow.origin.toDpOffset())
        private set

    override var size: DpSize by mutableStateOf(nativeWindow.size.toDpSize())
        private set

    override var contentSize: DpSize by mutableStateOf(nativeWindow.contentSize.toDpSize())
        private set

    override var isFocused: Boolean by mutableStateOf(nativeWindow.isKey)
        private set

    override var placement: WindowPlacement by mutableStateOf(nativeWindow.currentPlacement())
        private set

    override var screen: MacOsScreen by mutableStateOf(MacOsScreen(Screen.allScreens().findById(nativeWindow.screenId())))
        private set

    override var density: Density by mutableStateOf(Density(nativeWindow.scaleFactor().toFloat()))
        private set

    override var decoration: WindowDecoration by mutableStateOf(WindowDecoration.Decorated)
        private set

    private var systemThemeOverride: SystemTheme? by mutableStateOf(nativeWindow.overriddenAppearance?.toComposeSystemTheme())

    override val systemTheme: SystemTheme get() = systemThemeOverride ?: application.systemTheme

    override val minSize: DpSize get() = nativeWindow.minSize.toDpSize()
    override val maxSize: DpSize get() = nativeWindow.maxSize.toDpSize()
    override val isResizable: Boolean get() = nativeWindow.isResizable

    override val layoutDirection: LayoutDirection
        get() = when (nativeWindow.textDirection) {
            TextDirection.LeftToRight -> LayoutDirection.Ltr
            TextDirection.RightToLeft -> LayoutDirection.Rtl
        }

    override val customTitleBarInsets: Pair<Dp, Dp>?
        get() {
            val decoration = decoration
            // In full screen macOS hides the traffic lights, so the content gets the whole width.
            if (decoration !is WindowDecoration.CustomTitleBar || placement == WindowPlacement.Fullscreen) {
                return null
            }
            // The traffic-light buttons sit on the leading edge, laid out relative to the titlebar
            // height, so the space they need grows with it.
            return (40.dp + decoration.height) to 0.dp
        }

    // ----- Compose scene wiring -----

    private val viewConfiguration = object : ViewConfiguration {
        override val longPressTimeoutMillis: Long = 500
        override val doubleTapTimeoutMillis: Long = 300
        override val doubleTapMinTimeMillis: Long = 40
        override val touchSlop: Float get() = with(density) { 18.dp.toPx() }
    }

    private val windowInfo = object : WindowInfo {
        override val isWindowFocused: Boolean get() = isFocused
        override val keyboardModifiers get() = inputStateTracker.keyboardModifiers
        override val containerSize: IntSize get() = contentSizeInPx()
    }

    private val textInput = MacOsTextInput(nativeWindow, density = { density })

    private val platformContext = object : PlatformContext by PlatformContext.Empty() {
        override val windowInfo: WindowInfo get() = this@MacOsWindow.windowInfo
        override val viewConfiguration: ViewConfiguration get() = this@MacOsWindow.viewConfiguration
        override val inputModeManager get() = application.inputModeManager
        override val dragAndDropManager: PlatformDragAndDropManager = KdtDragAndDropManager(this@MacOsWindow)

        override fun setPointerIcon(pointerIcon: PointerIcon) {
            MacOsPointerIcons.apply(pointerIcon)
        }

        override suspend fun startInputMethod(request: PlatformTextInputMethodRequest): Nothing {
            return textInput.startInputMethod(request)
        }

        override fun convertLocalToWindowPosition(localPosition: Offset): Offset = contentOffsetInWindow() + localPosition

        override fun convertWindowToLocalPosition(positionInWindow: Offset): Offset = positionInWindow - contentOffsetInWindow()

        override fun convertLocalToScreenPosition(localPosition: Offset): Offset =
            localPosition + with(density) { nativeWindow.contentOrigin.toDpOffset().toPxOffset(this) }

        override fun convertScreenToLocalPosition(positionOnScreen: Offset): Offset =
            positionOnScreen - with(density) { nativeWindow.contentOrigin.toDpOffset().toPxOffset(this) }
    }

    private val scene: ComposeScene = CanvasLayersComposeScene(
        density = density,
        layoutDirection = layoutDirection,
        size = contentSizeInPx(),
        coroutineContext = KdtDispatcher,
        platformContext = platformContext,
        invalidate = { isFrameRequested = true },
    )

    private val inputStateTracker = MacOsInputStateTracker(
        inputModeManager = application.inputModeManager,
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

    internal val dragAndDropManager = MacOsDragAndDropManager(
        rootDragAndDropNode = { scene.rootDragAndDropNode },
        density = { density },
    )

    internal var activeDragAndDropTransferData: DragAndDropTransferData? = null

    private var onPreviewKeyEvent: (KeyEvent) -> Boolean = { false }
    private var onKeyEvent: (KeyEvent) -> Boolean = { false }

    init {
        requestDecoration(params.decoration)
        nativeWindow.attachView(viewContext.view)
        viewContext.onDisplayLayer = { repaintSynchronously() }
        setUpDisplayLink()
        // Registered last: from here on the event loop can deliver events to `handleEvent`, so
        // everything it touches has to be initialized.
        application.attachWindow(this)
    }

    // ----- Requests -----

    override fun requestPosition(position: DpOffset) {
        if (isDisposed) return
        nativeWindow.setRect(
            LogicalPoint(
                position.x.takeOrElse { nativeWindow.origin.x.dp }.value.toDouble(),
                position.y.takeOrElse { nativeWindow.origin.y.dp }.value.toDouble(),
            ),
            nativeWindow.size,
            animateTransition = false,
        )
    }

    override fun requestSize(size: DpSize) {
        if (isDisposed) return
        nativeWindow.setRect(
            nativeWindow.origin,
            LogicalSize(
                size.width.takeOrElse { nativeWindow.size.width.dp }.value.toDouble(),
                size.height.takeOrElse { nativeWindow.size.height.dp }.value.toDouble(),
            ),
            animateTransition = false,
        )
    }

    override fun requestBounds(bounds: DpRect) {
        if (isDisposed) return
        nativeWindow.setRect(
            LogicalPoint(
                bounds.left.takeOrElse { nativeWindow.origin.x.dp }.value.toDouble(),
                bounds.top.takeOrElse { nativeWindow.origin.y.dp }.value.toDouble(),
            ),
            LogicalSize(
                bounds.width.takeOrElse { nativeWindow.size.width.dp }.value.toDouble(),
                bounds.height.takeOrElse { nativeWindow.size.height.dp }.value.toDouble(),
            ),
            animateTransition = false,
        )
    }

    override fun requestMinSize(minSize: DpSize) {
        if (isDisposed) return
        nativeWindow.minSize = LogicalSize(
            minSize.width.takeOrElse { nativeWindow.minSize.width.dp }.value.toDouble(),
            minSize.height.takeOrElse { nativeWindow.minSize.height.dp }.value.toDouble(),
        )
    }

    override fun requestMaxSize(maxSize: DpSize) {
        if (isDisposed) return
        nativeWindow.maxSize = LogicalSize(
            maxSize.width.takeOrElse { nativeWindow.maxSize.width.dp }.value.toDouble(),
            maxSize.height.takeOrElse { nativeWindow.maxSize.height.dp }.value.toDouble(),
        )
    }

    override fun requestResizable(resizable: Boolean) {
        if (isDisposed) return
        nativeWindow.isResizable = resizable
    }

    override fun requestFocus() {
        if (isDisposed) return
        nativeWindow.makeKeyAndOrderFront()
    }

    override fun requestBringToFront() {
        if (isDisposed) return
        activateApplicationIfNeeded()
        nativeWindow.orderFront()
    }

    override fun requestFocusAndBringToFront() {
        if (isDisposed) return
        activateApplicationIfNeeded()
        nativeWindow.makeKeyAndOrderFront()
    }

    private fun activateApplicationIfNeeded() {
        if (!application.isActive) {
            application.requestActivation()
        }
    }

    override fun requestPlacement(placement: WindowPlacement) {
        if (isDisposed) return
        val isFullScreen = nativeWindow.isFullScreen
        val isMaximized = nativeWindow.isMaximizedButNotFullScreen()
        when (placement) {
            WindowPlacement.Floating -> when {
                isMaximized -> nativeWindow.toggleMaximize()
                isFullScreen -> nativeWindow.toggleFullScreen()
                else -> {}
            }
            WindowPlacement.Maximized -> if (!isMaximized) nativeWindow.toggleMaximize()
            WindowPlacement.Fullscreen -> if (!isFullScreen) nativeWindow.toggleFullScreen()
        }
    }

    override fun requestMinimized(minimized: Boolean) {
        if (isDisposed) return
        if (minimized) nativeWindow.miniaturize() else nativeWindow.deminiaturize()
    }

    override fun requestSystemTheme(systemTheme: SystemTheme?) {
        if (!isDisposed) {
            nativeWindow.overriddenAppearance = when (systemTheme) {
                null -> null
                SystemTheme.Light -> Appearance.Light
                SystemTheme.Dark -> Appearance.Dark
                else -> throw IllegalArgumentException("Unsupported system theme: $systemTheme")
            }
        }
        systemThemeOverride = systemTheme
    }

    override fun requestDecoration(decoration: WindowDecoration) {
        if (decoration == this.decoration) return
        val configuration = when (decoration) {
            WindowDecoration.Decorated -> TitlebarConfiguration.Regular
            is WindowDecoration.CustomTitleBar -> TitlebarConfiguration.Custom(
                titlebarHeight = decoration.height.value.toDouble(),
                largeCornerRadius = decoration.roundedCorners,
            )
        }
        if (!isDisposed) {
            nativeWindow.setTitlebarConfiguration(configuration)
        }
        this.decoration = decoration
    }

    /**
     * Asks the application's owner to close this window because the application is quitting. Separate
     * from the user-driven path only in the reason it reports.
     */
    internal fun requestCloseFromQuit() {
        if (!isDisposed) {
            onCloseRequest(WindowCloseRequestReason.ApplicationQuit)
        }
    }

    override fun requestClose() {
        if (!isDisposed) {
            onCloseRequest(WindowCloseRequestReason.UserRequest)
        }
    }

    // AppKit starts the drag from the event it is currently handling, so `pointerEvent` is not needed here.
    override fun startInteractiveMove(pointerEvent: PointerEvent) {
        if (isDisposed) return
        nativeWindow.startDragWindow()
    }

    override suspend fun showOpenFileDialog(
        title: String,
        allowsMultipleSelections: Boolean,
        canChooseFiles: Boolean,
        canChooseDirectories: Boolean,
    ): List<Path> {
        return withContext(KdtImmediateDispatcher) {
            runModal {
                FileDialog.showOpenFileDialog(
                    FileDialog.CommonDialogParams(title = title),
                    FileDialog.OpenDialogParams(
                        canChooseFiles = canChooseFiles,
                        canChooseDirectories = canChooseDirectories,
                        allowsMultipleSelections = allowsMultipleSelections,
                    ),
                ).map(Path::of)
            }
        }
    }

    override suspend fun showSaveFileDialog(title: String, nameFieldStringValue: String?): Path? {
        return withContext(KdtImmediateDispatcher) {
            runModal {
                FileDialog.showSaveFileDialog(
                    FileDialog.CommonDialogParams(
                        title = title,
                        nameFieldStringValue = nameFieldStringValue,
                        canCreateDirectories = true,
                    ),
                )?.let(Path::of)
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
            override val window: ComposeWindow get() = this@MacOsWindow
        }
        scene.setContent {
            CompositionLocalProvider(
                LocalComposeWindow provides this,
                LocalSystemTheme provides systemTheme,
                // The scene defaults these to AWT-backed implementations; the toolkit's own have to win.
                LocalClipboard provides MacOsClipboard,
                LocalUriHandler provides application.uriHandler,
            ) {
                windowScope.content()
            }

            // After a relayout, whatever is now under a stationary pointer has to learn that it is
            // hovered. Queued through the scene's dispatcher rather than run inline, so that pointer
            // input modifiers introduced by this very composition are already subscribed.
            rememberCoroutineScope().launch {
                inputStateTracker.prepareSyntheticMoveAfterRelayout()
                    ?.let(inputStateTracker::sendSyntheticMoveIfCurrent)
            }
        }
        isFrameRequested = true
    }

    // ----- Events -----

    internal fun handleEvent(event: WindowEvent): EventHandlerResult {
        return when (event) {
            is Event.WindowScreenChange -> {
                // The new screen may have a different scale and refresh rate.
                screen = MacOsScreen(Screen.allScreens().findById(nativeWindow.screenId()))
                density = Density(nativeWindow.scaleFactor().toFloat())
                scene.density = density
                scene.size = contentSizeInPx()
                setUpDisplayLink()
                EventHandlerResult.Stop
            }

            is Event.WindowMove -> {
                position = nativeWindow.origin.toDpOffset()
                EventHandlerResult.Stop
            }

            is Event.WindowResize -> {
                // Resizing from the top or left edge moves the origin too.
                position = nativeWindow.origin.toDpOffset()
                size = nativeWindow.size.toDpSize()
                contentSize = nativeWindow.contentSize.toDpSize()
                density = Density(nativeWindow.scaleFactor().toFloat())
                scene.density = density
                scene.size = contentSizeInPx()
                isFrameRequested = true
                EventHandlerResult.Stop
            }

            is Event.WindowFocusChange -> {
                isFocused = event.isKeyWindow
                inputStateTracker.handleEvent(event, density)
                EventHandlerResult.Stop
            }

            is Event.WindowFullScreenToggle -> {
                placement = nativeWindow.currentPlacement()
                EventHandlerResult.Stop
            }

            is Event.WindowChangedOcclusionState -> {
                when {
                    // A window that was hidden while its screen went away has no display link to resume.
                    event.isVisible && displayLink == null -> setUpDisplayLink()
                    else -> displayLink?.setRunning(event.isVisible)
                }
                EventHandlerResult.Stop
            }

            is Event.WindowCloseRequest -> {
                onCloseRequest(WindowCloseRequestReason.UserRequest)
                EventHandlerResult.Stop
            }

            is Event.KeyDown -> handleKeyDown(event)

            is Event.MouseMoved -> {
                lastMouseOffset = event.locationInWindow.toDpOffset()
                inputStateTracker.handleEvent(event, density)
            }

            is Event.MouseDown -> {
                lastMouseOffset = event.locationInWindow.toDpOffset()
                inputStateTracker.handleEvent(event, density)
            }

            is Event.MouseDragged -> {
                lastMouseOffset = event.locationInWindow.toDpOffset()
                inputStateTracker.handleEvent(event, density)
            }

            else -> inputStateTracker.handleEvent(event, density)
        }
    }

    /**
     * The order a key press is offered around in, which is what decides who gets to claim it:
     *
     *  1. While a composition is open the input method owns the keyboard — arrows walk the candidate
     *     list, Return and Escape end the composition — so it goes first.
     *  2. Otherwise Compose goes first, so that preview handlers and shortcuts see every key.
     *  3. A key nobody claimed becomes text: through the input method when a text field holds a session
     *     (which is what makes dead keys and CJK input work), and otherwise through a synthetic "typed"
     *     event, because nothing else would insert the character. See [MacOsTypedEvents].
     */
    private fun handleKeyDown(event: Event.KeyDown): EventHandlerResult {
        val hasMarkedText = textInput.hasMarkedText()
        if (hasMarkedText && textInput.offerKeyDownToInputMethod(event)) {
            return EventHandlerResult.Stop
        }

        val result = inputStateTracker.handleEvent(event, density)
        // Highlights the matching menu item, which AppKit only does for keys it routes itself. Purely
        // visual: menu actions run on activation, not on this call. The result handed back to native is
        // still the Compose one, so a key Compose did not claim still reaches the menu bar.
        AppMenuManager.offerCurrentEvent()
        if (result == EventHandlerResult.Stop) {
            return result
        }

        when {
            // Already offered above, and refused; re-offering would run the interpretation twice.
            hasMarkedText -> {}
            textInput.isSessionActive -> {
                if (textInput.offerKeyDownToInputMethod(event)) {
                    return EventHandlerResult.Stop
                }
            }
            else -> event.typedKeyEventOrNull()?.let { scene.sendKeyEvent(it) }
        }
        return result
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
        val pasteboardItems = itemsEntry.items.toPasteboardItems()

        // Odd-sized drag images look blurry, so the canvas is padded up to even pixels.
        val canvasSize = decorationSize.roundUpToEvenPixels()
        val currentDensity = density
        val image = DragAndDropImage(
            decorationSize,
            currentDensity,
            scene.layoutDirection,
            drawDragDecoration,
        ).encodeToPng(canvasSize = canvasSize)

        val pointerPosition = lastMouseOffset
        val imageOffset = pointerPosition.toLogicalPoint() -
            offset.toLogicalPoint(currentDensity) +
            transferData.dragDecorationOffset.toLogicalPoint(currentDensity)

        // macOS draws one image per dragged item; we want a single decoration, so every item after the
        // first gets a 1x1 placeholder.
        val emptyImageSize = Size(1f, 1f)
        val emptyImage = DragAndDropImage(emptyImageSize, currentDensity, scene.layoutDirection) {}.encodeToPng()!!
        val emptyImageRect = LogicalRect(imageOffset, emptyImageSize.toLogicalSize(currentDensity))

        val draggingItems = pasteboardItems.mapIndexed { index, item ->
            DraggingItem(
                item,
                if (index == 0) LogicalRect(imageOffset, canvasSize.toLogicalSize(currentDensity)) else emptyImageRect,
                Image(if (index == 0 && image != null) image else emptyImage),
            )
        }

        activeDragAndDropTransferData = transferData
        nativeWindow.startDragSession(pointerPosition.toLogicalPoint(), draggingItems)
    }

    /**
     * Where the pointer last was, in window coordinates. macOS wants the drag session anchored at the
     * press that started it, and by the time Compose asks for one the event itself is gone.
     */
    private var lastMouseOffset: DpOffset = DpOffset.Zero

    // ----- Frames -----

    private fun setUpDisplayLink() {
        displayLink?.close()
        displayLink = null
        if (!nativeWindow.isVisible) {
            return
        }
        val link = DisplayLink.create(nativeWindow.screenId()) { framePump.onDisplayLinkTick() }
        if (link == null) {
            // `create` returns null when there is no active display — the machine is asleep, or every
            // screen was disconnected. The retry path is the next occlusion or screen change event.
            Logger.warn {
                "Could not create a display link for screen ${nativeWindow.screenId()}; " +
                    "repainting is stalled until the window becomes visible again"
            }
            return
        }
        displayLink = link
        link.setRunning(true)
    }

    /**
     * Set while a modal file dialog owns the event loop thread.
     *
     * `runModal` spins a *nested* run loop, so the display link keeps ticking and Grand Central keeps
     * delivering the frame block — inside the coroutine that opened the dialog, which the scene's own
     * `FlushCoroutineDispatcher` is still in the middle of resuming. Rendering there re-enters that
     * dispatcher and resumes an already-completed continuation, which fails as
     * `CompletedContinuation cannot be cast to DispatchedContinuation`.
     */
    private var isModalDialogRunning = false

    /**
     * Runs a modal dialog with frame production suspended; see [isModalDialogRunning].
     *
     * Nothing is lost by skipping those frames: the dialog is covering the window, and the pump treats a
     * null frame as "nothing to show yet" and asks again.
     */
    private fun <T> runModal(block: () -> T): T {
        isModalDialogRunning = true
        try {
            return block()
        } finally {
            isModalDialogRunning = false
            isFrameRequested = true
        }
    }

    private fun recordFrame(): PresentablePicture? {
        if (isDisposed || isModalDialogRunning) return null
        val size = viewContext.view.size()
        val canvas = pictureRecorder.beginRecording(Rect.makeWH(size.width.toFloat(), size.height.toFloat()))
        canvas.clear(Color.White.toArgb())
        scene.render(canvas.asComposeCanvas(), System.nanoTime())
        return PresentablePicture(pictureRecorder.finishRecordingAsPicture(), size)
    }

    /**
     * Records and presents a frame right now, on the event loop thread.
     *
     * The display link is paused around it: a frame already in flight was recorded for the old size and
     * would be dropped anyway, and letting the pump start another one would race with this frame.
     */
    private fun repaintSynchronously() {
        displayLink?.setRunning(false)
        isFrameRequested = false
        try {
            val picture = recordFrame()
            if (picture == null) {
                isFrameRequested = true
            } else {
                picture.use { viewContext.presentSync(it, waitForCATransaction = true) }
            }
        } catch (throwable: Throwable) {
            Logger.error { "Could not present frame synchronously: $throwable" }
            isFrameRequested = true
        } finally {
            displayLink?.setRunning(true)
        }
    }

    // ----- Geometry -----

    private fun contentSizeInPx(): IntSize = nativeWindow.contentSize.roundToIntSize(density)

    /** The content area's top-left corner relative to the window's, which the titlebar shifts down. */
    private fun contentOffsetInWindow(): Offset {
        val windowOrigin = nativeWindow.origin.toDpOffset()
        val contentOrigin = nativeWindow.contentOrigin.toDpOffset()
        return DpOffset(contentOrigin.x - windowOrigin.x, contentOrigin.y - windowOrigin.y).toPxOffset(density)
    }

    override fun dispose() {
        if (isDisposed) return
        isDisposed = true
        // Before anything is torn down: the input session ends asynchronously, and this is what stops it
        // from reaching for a native window that no longer exists.
        textInput.onWindowClosing()
        viewContext.onDisplayLayer = {}
        // Blocks until the display link has left its callback, so no frame can start after this.
        displayLink?.setRunning(false)
        displayLink?.close()
        displayLink = null
        application.detachWindow(this)
        scene.close()
        pictureRecorder.close()
        application.gpuContext.destroyMetalViewContext(viewContext)
        nativeWindow.close()
    }
}

private fun Window.currentPlacement(): WindowPlacement = when {
    isFullScreen -> WindowPlacement.Fullscreen
    isMaximizedButNotFullScreen() -> WindowPlacement.Maximized
    else -> WindowPlacement.Floating
}

private fun Window.isMaximizedButNotFullScreen(): Boolean = !isFullScreen && isMaximized

/** Rounds a size in pixels up so that both dimensions are even integers. */
private fun Size.roundUpToEvenPixels(): Size {
    fun evenCeil(value: Float): Float {
        val ceiled = ceil(value).toInt()
        return (if (ceiled % 2 == 0) ceiled else ceiled + 1).toFloat()
    }
    return Size(evenCeil(width), evenCeil(height))
}
