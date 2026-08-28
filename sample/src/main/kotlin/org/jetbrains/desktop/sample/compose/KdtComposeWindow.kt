@file:OptIn(ExperimentalComposeUiApi::class)

package org.jetbrains.desktop.sample.compose

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.draganddrop.DragAndDropTransferData
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asComposeCanvas
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.platform.PlatformContext
import androidx.compose.ui.platform.PlatformDragAndDropManager
import androidx.compose.ui.platform.WindowInfo
import androidx.compose.ui.scene.CanvasLayersComposeScene
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import org.jetbrains.desktop.macos.DisplayLink
import org.jetbrains.desktop.macos.DraggingItem
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.Image
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalRect
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.Window
import org.jetbrains.desktop.macos.WindowEvent
import org.jetbrains.skia.PictureRecorder
import org.jetbrains.skia.Rect
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.ceil

/**
 * A kotlin-desktop-toolkit window that hosts a Compose scene.
 *
 * The frame loop has three participants:
 *  - the scene's `invalidate` callback marks that Compose wants a new frame,
 *  - a [DisplayLink] tied to the window's screen decides when that frame may be produced,
 *  - and [MetalViewContext] submits it to the GPU off the main thread.
 *
 * A frame is only started when the previous one has finished presenting, so a slow GPU throttles the
 * frame rate instead of queueing frames up without bound.
 */
@OptIn(InternalComposeUiApi::class)
class KdtComposeWindow(
    private val application: ComposeApplication,
    private val onCloseRequested: () -> Unit,
    title: String,
    origin: LogicalPoint,
    size: LogicalSize,
) : ComposeWindow {
    val window: Window = Window.create(Window.WindowParams(origin = origin, size = size, title = title)).apply {
        registerForDraggedTypes(
            listOf(
                UniformTypeIdentifiers.utf8PlainText,
                UniformTypeIdentifiers.html,
                UniformTypeIdentifiers.file,
                UniformTypeIdentifiers.png,
            ),
        )
    }

    private val viewContext = application.gpuContext.createMetalViewContext()
    private val pictureRecorder = PictureRecorder()

    @Volatile
    private var isFrameScheduled = false
    private val isFrameInProgress = AtomicBoolean(false)
    private var mouseOffset = DpOffset.Zero

    private var density: Density by mutableStateOf(Density(window.scaleFactor().toFloat()))
    private val positionCalculator = object {
        fun screenToLocal(positionOnScreen: Offset): Offset {
            return positionOnScreen - density.run {
                window.origin.run {
                    Offset(
                        x.dp.toPx(),
                        y.dp.toPx(),
                    )
                }
            }
        }

        fun localToScreen(localPosition: Offset): Offset {
            return localPosition + density.run {
                window.origin.run {
                    Offset(
                        x.dp.toPx(),
                        y.dp.toPx(),
                    )
                }
            }
        }
    }

    private val scene = CanvasLayersComposeScene(
        density = density,
        layoutDirection = application.globalLayoutDirection(),
        size = window.contentSize.toIntSize(window.scaleFactor()),
        coroutineContext = KdtDispatcher,
        invalidate = {
            isFrameScheduled = true
        },
        platformContext = object : PlatformContext by PlatformContext.Empty()  {
            override val dragAndDropManager: PlatformDragAndDropManager =
                KdtDragAndDropManager(this@KdtComposeWindow)

            override fun convertLocalToWindowPosition(localPosition: Offset): Offset =
                calculatePositionInWindow(localPosition)
            override fun convertWindowToLocalPosition(positionInWindow: Offset): Offset =
                calculateLocalPosition(positionInWindow)
            override fun convertLocalToScreenPosition(localPosition: Offset): Offset =
                positionCalculator.localToScreen(calculatePositionInWindow(localPosition))
            override fun convertScreenToLocalPosition(positionOnScreen: Offset): Offset =
                calculateLocalPosition(positionCalculator.screenToLocal(positionOnScreen))
        }
    )
    internal var macOsDragAndDropManager = MacOsDragAndDropManager(
        rootDragAndDropNode = { scene.rootDragAndDropNode },
        density = { scene.density },
    )
    internal var activeDragAndDropTransferData: DragAndDropTransferData? = null

    private var displayLink: DisplayLink? = null

    init {
        window.minSize = LogicalSize(320.0, 240.0)
        window.attachView(viewContext.view)
        // CoreAnimation asks the layer to redraw synchronously, most importantly while resizing, where a
        // frame recorded for the old size would be dropped by the render thread.
        viewContext.onDisplayLayer = { repaintSynchronously() }
        setupDisplayLink()
    }

    private fun setupDisplayLink() {
        displayLink?.setRunning(false)
        displayLink?.close()
        displayLink = DisplayLink.create(window.screenId()) {
            // Runs on the display link's own thread, so hop to the main thread to touch the scene.
            if (isFrameScheduled && isFrameInProgress.compareAndSet(false, true)) {
                GrandCentralDispatch.dispatchOnMain(highPriority = true) {
                    isFrameScheduled = false
                    val picture = recordFrame()
                    viewContext.presentAsync(picture, waitForCATransaction = false) {
                        picture.close()
                        isFrameInProgress.set(false)
                    }
                }
            }
        }
        displayLink?.setRunning(true)
    }

    private fun destroyDisplayLink() {
        // Blocks until the display link has left its callback.
        displayLink?.setRunning(false)
        displayLink?.close()
        displayLink = null
    }

    private fun recordFrame(): PresentablePicture {
        val size = viewContext.view.size()
        val canvas = pictureRecorder.beginRecording(Rect.makeWH(size.width.toFloat(), size.height.toFloat()))
        canvas.clear(Color.White.toArgb())
        scene.render(canvas.asComposeCanvas(), System.nanoTime())
        return PresentablePicture(pictureRecorder.finishRecordingAsPicture(), size)
    }

    private fun repaintSynchronously() {
        displayLink?.setRunning(false)
        isFrameScheduled = false
        recordFrame().use { picture ->
            viewContext.presentSync(picture, waitForCATransaction = true)
        }
        displayLink?.setRunning(true)
    }

    fun handleEvent(event: WindowEvent) {
        val scale = window.scaleFactor()
        when (event) {
            is Event.WindowScreenChange -> {
                // The new screen may run at a different refresh rate or scale.
                density = Density(window.scaleFactor().toFloat())
                scene.density = density
                setupDisplayLink()
            }

            is Event.WindowResize -> {
                scene.density = Density(scale.toFloat())
                scene.size = window.contentSize.toIntSize(scale)
            }

            is Event.WindowChangedOcclusionState -> {
                displayLink?.setRunning(event.isVisible)
            }

            is Event.WindowCloseRequest -> {
                onCloseRequested()
            }

            is Event.MouseDown -> sendPointerEvent(PointerEventType.Press, event, event.locationInWindow.toOffset(scale))
            is Event.MouseUp -> sendPointerEvent(PointerEventType.Release, event, event.locationInWindow.toOffset(scale))
            is Event.MouseMoved -> sendPointerEvent(PointerEventType.Move, event, event.locationInWindow.toOffset(scale))
            is Event.MouseDragged -> sendPointerEvent(PointerEventType.Move, event, event.locationInWindow.toOffset(scale))
            is Event.MouseEntered -> sendPointerEvent(PointerEventType.Enter, event, event.locationInWindow.toOffset(scale))
            is Event.MouseExited -> sendPointerEvent(PointerEventType.Exit, event, event.locationInWindow.toOffset(scale))

            is Event.ScrollWheel -> {
                sendPointerEvent(
                    eventType = PointerEventType.Scroll,
                    event = event,
                    position = event.locationInWindow.toOffset(scale),
                    // Compose scrolls content in the opposite direction to the wheel delta.
                    scrollDelta = Offset(-event.scrollingDeltaX.toFloat(), -event.scrollingDeltaY.toFloat()),
                )
            }

            is Event.KeyDown -> {
                scene.sendKeyEvent(event.toComposeKeyEvent(KeyEventType.KeyDown))
                // Sent as a second event, the way AWT delivers KEY_PRESSED and KEY_TYPED separately, so
                // shortcuts and navigation still see an ordinary KeyDown. See [typedKeyEventOrNull].
                event.typedKeyEventOrNull()?.let { scene.sendKeyEvent(it) }
            }
            is Event.KeyUp -> scene.sendKeyEvent(event.toComposeKeyEvent(KeyEventType.KeyUp))

            else -> {}
        }
    }

    fun startDragSession(
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

        // odd-sized drag images look blurry, so we pad the canvas up to even pixels.
        val canvasSize = decorationSize.roundUpToEvenPixels()
        val density = scene.density
        val image = DragAndDropImage(
            decorationSize,
            density,
            scene.layoutDirection,
            drawDragDecoration,
        ).encodeToPng(canvasSize = canvasSize)

        val imageOffset = mouseOffset.toLogicalPoint() -
                offset.toLogicalPoint(density) +
                transferData.dragDecorationOffset.toLogicalPoint(density)

        // we want to have only 1 image, even when we have multiple items,
        // so we pass an empty image for other than the first item
        val emptyImageSize = Size(1f, 1f)
        val emptyImage = DragAndDropImage(
            emptyImageSize, density, scene.layoutDirection,
        ) {}.encodeToPng()!!
        val emptyImageRect = LogicalRect(
            imageOffset,
            emptyImageSize.toLogicalSize(density),
        )

        val draggingItems = pasteboardItems.mapIndexed { index, item ->
            DraggingItem(
                item,
                if (index == 0)
                    LogicalRect(
                        imageOffset,
                        canvasSize.toLogicalSize(density),
                    ) else emptyImageRect,
                Image(if (index == 0 && image != null) image else emptyImage),
            )
        }

        activeDragAndDropTransferData = transferData
        window.startDragSession(mouseOffset.toLogicalPoint(), draggingItems)
    }


    private fun sendPointerEvent(eventType: PointerEventType, event: Event, position: Offset, scrollDelta: Offset = Offset.Zero) {
        scene.sendPointerEvent(
            eventType = eventType,
            position = position,
            scrollDelta = scrollDelta,
            timeMillis = event.timeMillis(),
            buttons = currentPointerButtons(),
            keyboardModifiers = Event.pressedModifiers().toPointerKeyboardModifiers(),
            nativeEvent = event,
            button = (event as? Event.MouseDown)?.button?.toComposePointerButton()
                ?: (event as? Event.MouseUp)?.button?.toComposePointerButton(),
        )
    }

    override fun setContent(content: @Composable () -> Unit) {
        scene.setContent(content)
    }

    override val size: DpSize get() = window.size.toDpSize()
    override val contentSize: DpSize get() = window.contentSize.toDpSize()
    override val isActive: Boolean get() = window.isMain
    override val isKey: Boolean get() = window.isKey



    /**
     * Relative to window
     */
    private fun contentPosition(): Offset {
        val d = Density(window.scaleFactor().toFloat())
        val windowOrigin = window.origin.toDpOffset()
        val contentOrigin = window.contentOrigin.toDpOffset()
        val deltaX = contentOrigin.x - windowOrigin.x
        val deltaY = contentOrigin.y - windowOrigin.y
        return with(d) { Offset(deltaX.toPx(), deltaY.toPx()) }
    }

    fun calculatePositionInWindow(localPosition: Offset): Offset {
        return contentPosition() + localPosition
    }

    fun calculateLocalPosition(positionInWindow: Offset): Offset {
        return positionInWindow - contentPosition()
    }

    override fun close() {
        application.forgetWindow(window.windowId())
        destroyDisplayLink()
        scene.close()
        pictureRecorder.close()
        application.gpuContext.destroyMetalViewContext(viewContext)
        window.close()
    }
}

private fun Event.timeMillis(): Long {
    return when (this) {
        is Event.MouseDown -> timestamp
        is Event.MouseUp -> timestamp
        is Event.MouseMoved -> timestamp
        is Event.MouseDragged -> timestamp
        is Event.MouseEntered -> timestamp
        is Event.MouseExited -> timestamp
        is Event.ScrollWheel -> timestamp
        else -> null
    }?.toDuration()?.inWholeMilliseconds ?: 0L
}

// The `KeyEvent(key = ..., type = ...)` factory is the only way to build a Compose key event from
// scratch: the `InternalKeyEvent` it wraps is `internal` to compose-ui, so it cannot be used here.
@OptIn(InternalComposeUiApi::class)
private fun Event.KeyDown.toComposeKeyEvent(type: KeyEventType): KeyEvent {
    return KeyEvent(
        key = keyCode.toComposeKey(),
        type = type,
        codePoint = keyWithModifiers.text.firstOrNull()?.code ?: 0,
        isCtrlPressed = modifiers.control,
        isMetaPressed = modifiers.command,
        isAltPressed = modifiers.option,
        isShiftPressed = modifiers.shift,
        nativeEvent = this,
    )
}

@OptIn(InternalComposeUiApi::class)
private fun Event.KeyUp.toComposeKeyEvent(type: KeyEventType): KeyEvent {
    return KeyEvent(
        key = keyCode.toComposeKey(),
        type = type,
        codePoint = keyWithModifiers.text.firstOrNull()?.code ?: 0,
        isCtrlPressed = modifiers.control,
        isMetaPressed = modifiers.command,
        isAltPressed = modifiers.option,
        isShiftPressed = modifiers.shift,
        nativeEvent = this,
    )
}

private fun Size.roundUpToEvenPixels(): Size {
    fun evenCeil(value: Float): Float {
        val ceiled = ceil(value).toInt()
        return (if (ceiled % 2 == 0) ceiled else ceiled + 1).toFloat()
    }
    return Size(evenCeil(width), evenCeil(height))
}