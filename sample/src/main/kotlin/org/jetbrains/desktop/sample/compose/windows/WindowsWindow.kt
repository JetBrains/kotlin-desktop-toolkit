@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.sample.compose.windows

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
import androidx.compose.ui.unit.height
import androidx.compose.ui.unit.isSpecified
import androidx.compose.ui.unit.takeOrElse
import androidx.compose.ui.unit.width
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.jetbrains.desktop.sample.compose.ClipboardItemsEntry
import org.jetbrains.desktop.sample.compose.ComposeWindow
import org.jetbrains.desktop.sample.compose.ComposeWindowParams
import org.jetbrains.desktop.sample.compose.ComposeWindowScope
import org.jetbrains.desktop.sample.compose.DragAndDropImage
import org.jetbrains.desktop.sample.compose.DragAndDropSourceWindow
import org.jetbrains.desktop.sample.compose.KdtDispatcher
import org.jetbrains.desktop.sample.compose.KdtDragAndDropManager
import org.jetbrains.desktop.sample.compose.KdtDragAndDropTransferable
import org.jetbrains.desktop.sample.compose.KdtImmediateDispatcher
import org.jetbrains.desktop.sample.compose.KdtInputModeManager
import org.jetbrains.desktop.sample.compose.LocalComposeWindow
import org.jetbrains.desktop.sample.compose.WindowCloseRequestReason
import org.jetbrains.desktop.sample.compose.WindowDecoration
import org.jetbrains.desktop.sample.compose.WindowPlacement
import org.jetbrains.desktop.win32.DataObject
import org.jetbrains.desktop.win32.DragDropContinueResult
import org.jetbrains.desktop.win32.DragDropEffect
import org.jetbrains.desktop.win32.DragDropManager
import org.jetbrains.desktop.win32.DragDropModifier
import org.jetbrains.desktop.win32.DragDropModifiers
import org.jetbrains.desktop.win32.DragImage
import org.jetbrains.desktop.win32.DragSource
import org.jetbrains.desktop.win32.DropTarget
import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.FileDialog
import org.jetbrains.desktop.win32.Logger
import org.jetbrains.desktop.win32.LogicalPoint
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.NCHitTestResult
import org.jetbrains.desktop.win32.PhysicalPoint
import org.jetbrains.desktop.win32.PhysicalSize
import org.jetbrains.desktop.win32.Screen
import org.jetbrains.desktop.win32.Window
import org.jetbrains.desktop.win32.WindowId
import org.jetbrains.desktop.win32.WindowParams
import org.jetbrains.desktop.win32.WindowStyle
import org.jetbrains.desktop.win32.WindowTitleBarKind
import java.nio.file.Path
import kotlin.math.roundToInt

/**
 * A kotlin-desktop-toolkit window hosting a Compose scene.
 *
 * The frame loop is the window's own, and that is the main difference from the other backends: macOS has
 * a display link and Wayland has the compositor's frame callbacks, while Windows has neither. Frames are
 * driven by [WindowsFramePump] — a paced coroutine loop on the event loop thread — and deliberately not
 * by `WM_PAINT`, which is only a damage *hint* here: the window's content is a composition surface that
 * keeps what was last presented, and the paint message is not delivered at all in several window states.
 *
 * The one exception is resizing. `WM_NCCALCSIZE` is the earliest point at which the new client size is
 * known, and recomposing and presenting inside that very message is what makes an interactive resize
 * track the pointer instead of trailing a frame behind it.
 */
internal class WindowsWindow(
    private val application: WindowsApplication,
    params: ComposeWindowParams,
    private val onCloseRequest: (WindowCloseRequestReason) -> Unit,
) : ComposeWindow,
    DragAndDropSourceWindow {
    override val nativeWindow: Window = application.nativeApplication.newWindow()

    internal val windowId: WindowId = nativeWindow.id

    private var isDisposed = false

    /**
     * The client size the last native size event reported, in physical pixels — what a frame is presented
     * at. Null until the first one arrives, which is before the window is ever shown.
     */
    private var latestPhysicalSize: PhysicalSize? = null

    /** Subpixel layout for text, read once: it is a system-wide setting a user rarely changes mid-run. */
    private val pixelGeometry = currentPixelGeometry()

    private val angleViewContext: AngleViewContext

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
                nativeWindow.setTitle(value)
            }
        }

    private var titleState: String by mutableStateOf(params.title)

    override var position: DpOffset by mutableStateOf(DpOffset.Zero)
        private set

    override var size: DpSize by mutableStateOf(params.size)
        private set

    override var contentSize: DpSize by mutableStateOf(params.size)
        private set

    override var isFocused: Boolean by mutableStateOf(false)
        private set

    override var placement: WindowPlacement by mutableStateOf(WindowPlacement.Floating)
        private set

    override var screen: WindowsScreen by mutableStateOf(application.mainScreen)
        private set

    override var density: Density by mutableStateOf(Density(1f))
        private set

    /**
     * Fixed at creation: `WindowTitleBarKind` is part of the window style Windows resolves when the
     * window is created, and the toolkit offers no way to swap it afterwards. See [requestDecoration].
     */
    override val decoration: WindowDecoration = params.decoration

    private var systemThemeOverride: SystemTheme? by mutableStateOf(null)

    override val systemTheme: SystemTheme get() = systemThemeOverride ?: application.systemTheme

    private var minSizeState: DpSize by mutableStateOf(params.minSize)
    override val minSize: DpSize get() = minSizeState

    /** Tracked here only: win32 has no maximum-size API, so nothing enforces this. */
    private var maxSizeState: DpSize by mutableStateOf(DpSize.Unspecified)
    override val maxSize: DpSize get() = maxSizeState

    override val isResizable: Boolean get() = !isDisposed && nativeWindow.isResizable()

    /** Ltr: win32 exposes no reading order for the user's locale, unlike AppKit and Wayland. */
    override val layoutDirection: LayoutDirection = LayoutDirection.Ltr

    /**
     * Always `null`: Windows draws no buttons of its own into a custom titlebar — a `Custom` titlebar
     * window keeps `WS_CAPTION` for the native minimize and maximize *transitions* but has its system
     * controls suppressed — so the content owes itself a close affordance, and the whole width of the bar
     * is its to use.
     */
    override val customTitleBarInsets: Pair<Dp, Dp>? get() = null

    /**
     * True: a custom titlebar's drag region is reported to Windows as `HTCAPTION` (see [hitTestTitleBar]),
     * which hands the platform the move, the double-click maximize, Aero Snap and the right-click system
     * menu. The content must add no gestures of its own, or both would fire.
     */
    override val handlesCustomTitleBarGestures: Boolean get() = true

    /**
     * Areas of the titlebar the content claimed back, in logical client coordinates.
     *
     * The consequence of the platform owning the bar is that a press anywhere in it goes to Windows and
     * never reaches Compose, which would make a button drawn into the bar — the content's close button —
     * unclickable. An area listed here is reported as client, so its pointer events flow normally.
     */
    private var customTitleBarInteractiveAreas: List<DpRect> = emptyList()

    override fun setCustomTitleBarInteractiveAreas(areas: List<DpRect>) {
        customTitleBarInteractiveAreas = areas
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

    private val pointerIcons = WindowsPointerIcons(nativeWindow)

    /**
     * Per window, unlike macOS's application-wide one: the cursor this hides belongs to a window here,
     * so the window is what has to hide it while the user is navigating by keyboard.
     */
    private val inputModeManager = KdtInputModeManager(InputMode.Touch) { inputMode ->
        pointerIcons.setHiddenUntilPointerMoves(inputMode == InputMode.Keyboard)
    }

    private val textInput = WindowsTextInput(nativeWindow, density = { density })

    private val platformContext = object : PlatformContext by PlatformContext.Empty() {
        override val windowInfo: WindowInfo get() = this@WindowsWindow.windowInfo
        override val viewConfiguration: ViewConfiguration get() = this@WindowsWindow.viewConfiguration
        override val inputModeManager get() = this@WindowsWindow.inputModeManager
        override val dragAndDropManager: PlatformDragAndDropManager = KdtDragAndDropManager(this@WindowsWindow)

        override fun setPointerIcon(pointerIcon: PointerIcon) {
            pointerIcons.apply(pointerIcon)
        }

        override suspend fun startInputMethod(request: PlatformTextInputMethodRequest): Nothing {
            return textInput.startInputMethod(request)
        }

        // The client area is the scene: a custom titlebar lives inside it, and a decorated window's
        // client area starts below the native one. So local, window and client coordinates coincide, and
        // only the screen conversion has an offset to apply.
        override fun convertLocalToWindowPosition(localPosition: Offset): Offset = localPosition

        override fun convertWindowToLocalPosition(positionInWindow: Offset): Offset = positionInWindow

        override fun convertLocalToScreenPosition(localPosition: Offset): Offset = localPosition + clientOriginInScreenPx()

        override fun convertScreenToLocalPosition(positionOnScreen: Offset): Offset = positionOnScreen - clientOriginInScreenPx()
    }

    // ----- Frames -----
    //
    // Declared before the scene, because the scene's `invalidate` callback is this loop.

    private val frameScope = CoroutineScope(KdtDispatcher)

    private val framePump: WindowsFramePump = WindowsFramePump(frameScope, MIN_FRAME_INTERVAL_NANOS) {
        try {
            val didRun = withoutRenderReentrancy { renderCurrentFrame() }
            if (!didRun) {
                // A synchronous resize render is on the stack below us, through re-entered dispatch;
                // this frame is re-run once it unwinds.
                framePump.scheduleFrame()
            }
        } catch (throwable: Throwable) {
            // The frame was presented anyway (see AngleViewContext.renderFrame), and losing the loop
            // would leave the window frozen — so it retries instead.
            Logger.error(throwable) { "Could not render a frame, retrying" }
            framePump.scheduleFrame()
        }
    }

    private var isSceneContentInstalled = false

    private val scene: ComposeScene = CanvasLayersComposeScene(
        density = Density(1f),
        layoutDirection = layoutDirection,
        coroutineContext = KdtDispatcher,
        platformContext = platformContext,
        invalidate = { framePump.scheduleFrame() },
    )

    private val inputStateTracker = WindowsInputStateTracker(
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

    private val dragAndDropManager = WindowsDragAndDropManager(
        nativeWindow = nativeWindow,
        rootDragAndDropNode = { scene.rootDragAndDropNode },
    )

    private val dragDropManager = DragDropManager(nativeWindow).apply {
        registerDropTarget(
            object : DropTarget {
                override fun onDragEnter(
                    dataObject: DataObject,
                    modifiers: DragDropModifiers,
                    point: PhysicalPoint,
                    effect: DragDropEffect,
                ): DragDropEffect = dragAndDropManager.onDragEnter(dataObject, point, effect, modifiers)

                override fun onDragOver(modifiers: DragDropModifiers, point: PhysicalPoint, effect: DragDropEffect): DragDropEffect =
                    dragAndDropManager.onDragOver(point, effect, modifiers)

                override fun onDragLeave() = dragAndDropManager.onDragLeave()

                override fun onDrop(
                    dataObject: DataObject,
                    modifiers: DragDropModifiers,
                    point: PhysicalPoint,
                    effect: DragDropEffect,
                ): DragDropEffect = dragAndDropManager.onDrop(dataObject, point, effect, modifiers)
            },
        )
    }

    private var onPreviewKeyEvent: (KeyEvent) -> Boolean = { false }
    private var onKeyEvent: (KeyEvent) -> Boolean = { false }

    init {
        nativeWindow.create(
            WindowParams(
                origin = (if (params.position.isSpecified) params.position else DpOffset.Zero).toLogicalPoint(),
                size = params.size.toLogicalSize(),
                title = params.title,
                style = WindowStyle(
                    isResizable = params.isResizable,
                    titleBarKind = when (params.decoration) {
                        WindowDecoration.Decorated -> WindowTitleBarKind.System
                        is WindowDecoration.CustomTitleBar -> WindowTitleBarKind.Custom
                    },
                ),
            ),
        )
        nativeWindow.setMinSize(params.minSize.toLogicalSize())
        angleViewContext = AngleViewContext.create(application.nativeApplication, nativeWindow)
        // Read back rather than assumed: Windows resolves the requested bounds against the frame, the
        // work area and the monitor's scale before the window exists.
        val scaleFactor = nativeWindow.getScaleFactor()
        val clientSize = nativeWindow.getClientSize()
        density = Density(scaleFactor)
        position = nativeWindow.getRect().origin.toDpOffset()
        size = nativeWindow.getRect().size.toDpSize()
        contentSize = clientSize.toDpSize()
        // Seeded rather than waited for: the size events Windows sends while a window is being created
        // arrive before it is registered with the event loop, so the first frame after `show()` would
        // otherwise have no size to render at.
        latestPhysicalSize = PhysicalSize(
            (clientSize.width * scaleFactor).roundToInt(),
            (clientSize.height * scaleFactor).roundToInt(),
        )
        screen = WindowsScreen(nativeWindow.getScreen())
        scene.density = density
        scene.size = contentSizeInPx()
        applyImmersiveDarkMode()
        // Registered last: from here on the event loop can deliver events to `handleEvent`, so
        // everything it touches has to be initialized.
        application.attachWindow(this)
    }

    // ----- Requests -----

    override fun requestPosition(position: DpOffset) {
        if (isDisposed) return
        val rect = nativeWindow.getRect()
        nativeWindow.setRect(
            LogicalPoint(
                position.x.takeOrElse { rect.origin.x.dp }.value,
                position.y.takeOrElse { rect.origin.y.dp }.value,
            ),
            rect.size,
        )
    }

    override fun requestSize(size: DpSize) {
        if (isDisposed) return
        val rect = nativeWindow.getRect()
        nativeWindow.setRect(
            rect.origin,
            LogicalSize(
                size.width.takeOrElse { rect.size.width.dp }.value,
                size.height.takeOrElse { rect.size.height.dp }.value,
            ),
        )
    }

    override fun requestBounds(bounds: DpRect) {
        if (isDisposed) return
        val rect = nativeWindow.getRect()
        nativeWindow.setRect(
            LogicalPoint(
                bounds.left.takeOrElse { rect.origin.x.dp }.value,
                bounds.top.takeOrElse { rect.origin.y.dp }.value,
            ),
            LogicalSize(
                bounds.width.takeOrElse { rect.size.width.dp }.value,
                bounds.height.takeOrElse { rect.size.height.dp }.value,
            ),
        )
    }

    override fun requestMinSize(minSize: DpSize) {
        if (isDisposed) return
        val effective = DpSize(
            minSize.width.takeOrElse { minSizeState.width },
            minSize.height.takeOrElse { minSizeState.height },
        )
        nativeWindow.setMinSize(effective.toLogicalSize())
        minSizeState = effective
    }

    /** Recorded and reported back, but not enforced: win32 has no maximum-size API. */
    override fun requestMaxSize(maxSize: DpSize) {
        maxSizeState = maxSize
    }

    override fun requestResizable(resizable: Boolean) {
        if (isDisposed) return
        nativeWindow.setResizable(resizable)
    }

    /**
     * Forces the window to the foreground.
     *
     * Windows normally refuses to let a process that does not own the foreground activate a window,
     * which is why this is a distinct toolkit call rather than an ordinary activation: an application
     * launched from a terminal or an IDE could not come to the front otherwise.
     */
    override fun requestFocus() {
        if (isDisposed) return
        nativeWindow.forceFocus()
    }

    override fun requestBringToFront() {
        if (isDisposed) return
        nativeWindow.forceFocus()
    }

    override fun requestFocusAndBringToFront() {
        if (isDisposed) return
        nativeWindow.forceFocus()
    }

    override fun requestPlacement(placement: WindowPlacement) {
        if (isDisposed) return
        when (placement) {
            WindowPlacement.Floating -> nativeWindow.restore()
            WindowPlacement.Maximized -> nativeWindow.maximize()
            // No fullscreen in the toolkit's win32 backend yet; maximizing is the closest thing that
            // does not lie about what happened.
            WindowPlacement.Fullscreen -> Logger.warn { "Fullscreen is not supported on Windows, ignoring the request" }
        }
    }

    override fun requestMinimized(minimized: Boolean) {
        if (isDisposed) return
        if (minimized) nativeWindow.minimize() else nativeWindow.restore()
    }

    override fun requestSystemTheme(systemTheme: SystemTheme?) {
        systemThemeOverride = systemTheme
        applyImmersiveDarkMode()
    }

    /**
     * Only ever the decoration the window was created with; anything else is refused.
     *
     * The titlebar kind is baked into the window style Windows resolved at creation — and, for a custom
     * one, into how `WM_NCCALCSIZE` lays the client area out — so changing it would mean recreating the
     * window behind the application's back.
     */
    override fun requestDecoration(decoration: WindowDecoration) {
        if (decoration != this.decoration) {
            Logger.warn { "A window's decoration cannot be changed on Windows, keeping ${this.decoration}" }
        }
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

    /**
     * Nothing: the toolkit's win32 backend exposes no way to hand a press to the platform's move loop.
     *
     * Windows does not need one for the case this exists for. A custom titlebar's drag region is reported
     * as `HTCAPTION`, so the platform is already running the move — see [handlesCustomTitleBarGestures].
     */
    override fun startInteractiveMove(pointerEvent: PointerEvent) = Unit

    /**
     * The win32 file dialogs are blocking calls, so — like macOS's modal panels — the blocking call runs
     * on the event loop thread and being `suspend` is what lets the caller's coroutine wait for it.
     *
     * `canChooseFiles` has no win32 counterpart: the picker either chooses files or folders, and asking
     * for neither is not a thing it can express.
     */
    override suspend fun showOpenFileDialog(
        title: String,
        allowsMultipleSelections: Boolean,
        canChooseFiles: Boolean,
        canChooseDirectories: Boolean,
    ): List<Path> = withContext(KdtImmediateDispatcher) {
        FileDialog.showOpenFileDialog(
            owner = nativeWindow,
            options = FileDialog.FileDialogOptions(title = title),
            openDialogOptions = FileDialog.FileOpenDialogOptions(
                chooseDirectories = canChooseDirectories,
                allowsMultipleSelection = allowsMultipleSelections,
            ),
        ).map(Path::of)
    }

    override suspend fun showSaveFileDialog(title: String, nameFieldStringValue: String?): Path? = withContext(KdtImmediateDispatcher) {
        FileDialog.showSaveFileDialog(
            owner = nativeWindow,
            options = FileDialog.FileDialogOptions(title = title, nameFieldStringValue = nameFieldStringValue),
        )?.let(Path::of)
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
            override val window: ComposeWindow get() = this@WindowsWindow
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

            // After a relayout, whatever is now under a stationary pointer has to learn that it is
            // hovered. Queued through the scene's dispatcher rather than run inline, so that pointer
            // input modifiers introduced by this very composition are already subscribed.
            rememberCoroutineScope().launch {
                inputStateTracker.prepareSyntheticMoveAfterRelayout()
                    ?.let(inputStateTracker::sendSyntheticMoveIfCurrent)
            }
        }
        // Shown only once there is something to show: a window mapped before its first frame would flash
        // its background first.
        if (!isSceneContentInstalled && !isDisposed) {
            isSceneContentInstalled = true
            nativeWindow.show()
            application.onWindowFirstShown(this)
        }
        framePump.scheduleFrame()
    }

    // ----- Events -----

    internal fun handleEvent(event: Event): EventHandlerResult {
        if (isDisposed) {
            return EventHandlerResult.Continue
        }
        return try {
            // The active input session gets first refusal on the native text events: a committed
            // character arrives as `CharacterReceived` and is inserted there rather than routed as a key.
            if (textInput.tryHandleTextInputEvent(event)) {
                return EventHandlerResult.Stop
            }
            when (event) {
                is Event.NCCalcSize -> {
                    // The earliest point at which the new client size is known. Windows also sends it
                    // while minimizing, where there is nothing to lay out and nowhere to present.
                    if (!nativeWindow.isMinimized()) {
                        val logicalSize = event.size.toDpSize(event.scale)
                        contentSize = logicalSize
                        size = nativeWindow.getRect().size.toDpSize()
                        placement = currentPlacement()
                        scene.size = contentSizeInPx()
                        latestPhysicalSize = event.size
                        // The one render outside the frame loop: recomposing at the new size within this
                        // message is what makes an interactive resize track the pointer.
                        val didRun = withoutRenderReentrancy { renderCurrentFrame() }
                        if (!didRun) {
                            // Delivered synchronously from a native call made inside a render
                            // (`setRect` → `WM_NCCALCSIZE`). Retrying in place could re-enter the same
                            // way and never return to the message pump, so the loop picks it up.
                            framePump.scheduleFrame()
                        }
                    }
                    EventHandlerResult.Continue
                }

                is Event.WindowDraw -> {
                    // A damage hint, not a frame: the composition surface still holds the last frame, so
                    // the exposed region shows it until the loop presents the next one.
                    latestPhysicalSize = event.size
                    framePump.scheduleFrame()
                    EventHandlerResult.Continue
                }

                is Event.WindowResize -> {
                    contentSize = event.size.toDpSize(event.scale)
                    size = nativeWindow.getRect().size.toDpSize()
                    placement = currentPlacement()
                    scene.size = contentSizeInPx()
                    latestPhysicalSize = event.size
                    framePump.scheduleFrame()
                    EventHandlerResult.Continue
                }

                is Event.WindowMove -> {
                    position = event.origin.toDpOffset(event.scale)
                    EventHandlerResult.Continue
                }

                is Event.WindowScaleChanged -> {
                    density = Density(event.scale)
                    screen = WindowsScreen(nativeWindow.getScreen())
                    position = event.origin.toDpOffset(event.scale)
                    scene.density = density
                    // The accompanying `WM_NCCALCSIZE` carries the new client size and renders at it.
                    EventHandlerResult.Continue
                }

                is Event.WindowTitleChanged -> {
                    titleState = event.title
                    EventHandlerResult.Continue
                }

                is Event.WindowActivated -> {
                    isFocused = event.active
                    if (event.active) {
                        // A window restored from minimized may have parked invalidations, and no paint
                        // message is guaranteed to arrive for them.
                        framePump.scheduleFrame()
                    }
                    inputStateTracker.handleEvent(event, density)
                    EventHandlerResult.Continue
                }

                is Event.WindowCloseRequest -> {
                    onCloseRequest(WindowCloseRequestReason.UserRequest)
                    EventHandlerResult.Stop
                }

                is Event.NCHitTest -> {
                    val result = hitTestTitleBar(event)
                    if (result == null) {
                        EventHandlerResult.Continue
                    } else {
                        event.setHitTestResult(result)
                        EventHandlerResult.Stop
                    }
                }

                is Event.KeyDown -> handleKeyDown(event)

                is Event.KeyUp -> {
                    val result = inputStateTracker.handleEvent(event, density)
                    translateSystemKeyIfUnhandled(event, event.isSystemKey, result)
                }

                else -> inputStateTracker.handleEvent(event, density)
            }
        } catch (throwable: Throwable) {
            // A handler that throws must not take the event loop with it; let the event fall through to
            // the platform's own handling instead.
            Logger.error(throwable) { "Failed to handle event $event" }
            EventHandlerResult.Continue
        }
    }

    /**
     * The order a key press is offered around in, which is what decides who gets to claim it:
     *
     *  1. A text field holding the input session goes first, because on Windows the character a key
     *     produces only exists once the message has been translated — see [WindowsTextInput].
     *  2. Otherwise Compose, so that preview handlers and shortcuts see every key.
     *  3. An unclaimed system key — one held with Alt — is still translated, so that the platform's own
     *     menu accelerators and dead keys keep working.
     */
    private fun handleKeyDown(event: Event.KeyDown): EventHandlerResult {
        if (textInput.offerKeyDownToInputMethod(event)) {
            return EventHandlerResult.Stop
        }
        val result = inputStateTracker.handleEvent(event, density)
        return translateSystemKeyIfUnhandled(event, event.isSystemKey, result)
    }

    private fun translateSystemKeyIfUnhandled(
        event: Event.KeyEvent,
        isSystemKey: Boolean,
        result: EventHandlerResult,
    ): EventHandlerResult {
        if (result == EventHandlerResult.Continue && isSystemKey) {
            event.translate()
        }
        return result
    }

    /**
     * Which non-client region a point in a custom titlebar belongs to, or `null` to leave it to the
     * toolkit's default — the client area, plus the resize borders it resolves before asking.
     *
     * The band is a native drag region, which is what gives the window Windows' own move, double-click
     * maximize, Aero Snap and right-click system menu; the areas the content claimed
     * ([setCustomTitleBarInteractiveAreas]) stay client, so what is drawn there stays clickable.
     */
    private fun hitTestTitleBar(event: Event.NCHitTest): NCHitTestResult? {
        val titleBar = decoration as? WindowDecoration.CustomTitleBar ?: return null
        // `WM_NCHITTEST` coordinates are physical screen pixels; the titlebar's height is in dp, which is
        // logical client space.
        val scale = nativeWindow.getScaleFactor()
        val clientPoint = Screen.mapToClient(nativeWindow, PhysicalPoint(event.mouseX, event.mouseY)).toLogical(scale)
        val clientSize = nativeWindow.getClientSize()
        val isInsideTitleBar = clientPoint.x in 0f..clientSize.width && clientPoint.y in 0f..titleBar.height.value
        if (!isInsideTitleBar) {
            return null
        }
        if (customTitleBarInteractiveAreas.any { area -> clientPoint in area }) {
            return null
        }
        return NCHitTestResult.Caption
    }

    // ----- Drag and drop -----

    /** Escape cancels the drag, releasing the button drops, and anything else keeps it going. */
    private val dragSource = object : DragSource {
        override fun onQueryContinueDrag(escapePressed: Boolean, modifiers: DragDropModifiers): DragDropContinueResult = when {
            escapePressed -> DragDropContinueResult.Cancel
            !modifiers.hasFlag(DragDropModifier.LeftButton) -> DragDropContinueResult.Drop
            else -> DragDropContinueResult.Continue
        }
    }

    /**
     * Starts an OLE drag session, which is a *blocking* modal loop — `DoDragDrop` does not return until
     * the drag ends.
     *
     * Posted rather than run in place, because Compose asks for the session from inside pointer dispatch:
     * running the loop there would block the press's own handler, with the whole event dispatch still on
     * the stack below it. The nested loop keeps pumping posted work, so the window carries on rendering
     * while the drag is in flight.
     */
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
        val allowedEffects = transferData.supportedActions
            .fold(DragDropEffect.None) { effects, action -> effects or action.toDragDropEffect() }
        if (allowedEffects == DragDropEffect.None) {
            return
        }
        val image = DragAndDropImage(decorationSize, density, scene.layoutDirection, drawDragDecoration)
            .encodeToPng()
            ?.takeIf { it.isNotEmpty() }
        val dragImage = image?.let { DragImage(it, transferData.dragDecorationOffset.toPhysicalPoint()) }
        application.invokeOnUiThread {
            if (!isDisposed) {
                runDragSession(transferData, itemsEntry, allowedEffects, dragImage)
            }
        }
    }

    private fun runDragSession(
        transferData: DragAndDropTransferData,
        itemsEntry: ClipboardItemsEntry,
        allowedEffects: DragDropEffect,
        dragImage: DragImage?,
    ) {
        val effect = DataObject.build { addClipboardItems(itemsEntry.items) }.use { dataObject ->
            dragDropManager.doDragDrop(dataObject, allowedEffects, dragSource, dragImage)
        }
        // `doDragDrop` reports what the target did, so — unlike macOS, where the answer arrives in a
        // separate callback — the source can be told right here.
        transferData.onTransferCompleted?.invoke(effect.toDragAndDropTransferAction())
    }

    // ----- Frames -----

    /**
     * Renders the content at the window's latest known size and presents it, returning whether it did.
     *
     * Nothing is presented before there is content to present, or while the window is minimized — where
     * the surface has no size to speak of and the frame would be thrown away.
     */
    private fun renderCurrentFrame(): Boolean {
        if (isDisposed || !isSceneContentInstalled || nativeWindow.isMinimized()) {
            return false
        }
        val physicalSize = latestPhysicalSize ?: return false
        angleViewContext.renderFrame(physicalSize, pixelGeometry) {
            scene.render(asComposeCanvas(), System.nanoTime())
        }
        return true
    }

    // ----- Geometry -----

    private fun contentSizeInPx(): IntSize = with(density) {
        IntSize(contentSize.width.roundToPx(), contentSize.height.roundToPx())
    }

    /** The client area's top-left corner in the global desktop space, in pixels. */
    private fun clientOriginInScreenPx(): Offset {
        val clientOrigin = Screen.mapToClient(nativeWindow, PhysicalPoint(0, 0))
        // `mapToClient` answers in client coordinates, so the screen origin of the client area is the
        // negation of where the screen's own origin landed.
        return Offset(-clientOrigin.x.toFloat(), -clientOrigin.y.toFloat())
    }

    private fun currentPlacement(): WindowPlacement =
        if (nativeWindow.isMaximized()) WindowPlacement.Maximized else WindowPlacement.Floating

    private fun applyImmersiveDarkMode() {
        if (isDisposed) return
        // Colours the native titlebar and frame; a custom titlebar has none, but the window's shadow and
        // border still follow it.
        nativeWindow.setImmersiveDarkMode(systemTheme == SystemTheme.Dark)
    }

    /** Called by the application when its own theme changed, so a window following it can catch up. */
    internal fun onApplicationSystemThemeChanged() {
        applyImmersiveDarkMode()
    }

    override fun dispose() {
        if (isDisposed) return
        isDisposed = true
        // Before anything is torn down: the input session ends asynchronously, and this is what stops it
        // from reaching for a window that no longer exists.
        textInput.onWindowClosing()
        // Balances an outstanding cursor hide, which is process-wide and would otherwise leak.
        pointerIcons.onWindowClosing()
        framePump.cancel()
        frameScope.cancel()
        application.detachWindow(this)
        dragDropManager.revokeDropTarget()
        dragDropManager.close()
        scene.close()
        angleViewContext.close()
        nativeWindow.destroy()
        nativeWindow.close()
    }
}

private operator fun DpRect.contains(point: LogicalPoint): Boolean =
    point.x in left.value..right.value && point.y in top.value..bottom.value

/**
 * Guards the two synchronous render entry points — the paced frame and the resize render — against
 * nested native dispatch, and reports whether [block] ran.
 *
 * Windows delivers some messages synchronously from calls made inside a running render (`setRect` sends
 * `WM_NCCALCSIZE` from where it is called), and the handler of such a message renders again. Dropping the
 * nested render is safe because the caller schedules a follow-up frame through the conflated channel; it
 * must not be retried in place, which could re-enter the same way and never return to the message pump.
 *
 * Shared across every window, and confined to the event loop thread, where every render happens.
 */
private inline fun withoutRenderReentrancy(block: () -> Unit): Boolean {
    if (isRenderInProgress) {
        return false
    }
    isRenderInProgress = true
    try {
        block()
    } finally {
        isRenderInProgress = false
    }
    return true
}

private var isRenderInProgress = false

/** 60 Hz. The toolkit's swap does not block on vsync, so the loop has to pace itself. */
private const val MIN_FRAME_INTERVAL_NANOS = 1_000_000_000L / 60
