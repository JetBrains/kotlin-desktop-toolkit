package org.jetbrains.desktop.sample.compose

import androidx.compose.runtime.Composable
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asComposeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.scene.CanvasLayersComposeScene
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpSize
import org.jetbrains.desktop.macos.DisplayLink
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.Window
import org.jetbrains.desktop.macos.WindowEvent
import org.jetbrains.skia.PictureRecorder
import org.jetbrains.skia.Rect
import java.util.concurrent.atomic.AtomicBoolean

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
    val window: Window = Window.create(Window.WindowParams(origin = origin, size = size, title = title))
    private val viewContext = application.gpuContext.createMetalViewContext()
    private val pictureRecorder = PictureRecorder()

    @Volatile
    private var isFrameScheduled = false
    private val isFrameInProgress = AtomicBoolean(false)

    private val scene = CanvasLayersComposeScene(
        density = Density(window.scaleFactor().toFloat()),
        layoutDirection = application.globalLayoutDirection(),
        size = window.contentSize.toIntSize(window.scaleFactor()),
        coroutineContext = KdtDispatcher,
        invalidate = {
            isFrameScheduled = true
        },
    )

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
                scene.density = Density(scale.toFloat())
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
