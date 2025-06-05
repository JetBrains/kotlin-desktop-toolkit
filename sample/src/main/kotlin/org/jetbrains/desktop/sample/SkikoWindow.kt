package org.jetbrains.desktop.sample

import org.jetbrains.desktop.macos.DisplayLink
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalRect
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.MetalCommandQueue
import org.jetbrains.desktop.macos.MetalDevice
import org.jetbrains.desktop.macos.MetalView
import org.jetbrains.desktop.macos.PhysicalSize
import org.jetbrains.desktop.macos.ScreenId
import org.jetbrains.desktop.macos.TextInputClient
import org.jetbrains.desktop.macos.TextRange
import org.jetbrains.desktop.macos.Window
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.ColorSpace
import org.jetbrains.skia.DirectContext
import org.jetbrains.skia.Surface
import org.jetbrains.skia.SurfaceColorFormat
import org.jetbrains.skia.SurfaceOrigin
import kotlin.time.TimeSource

abstract class SkikoWindow(
    device: MetalDevice,
    private val queue: MetalCommandQueue,
    windowParams: Window.WindowParams,
) : AutoCloseable {

    val window = Window.create(windowParams)
    var displayLink = DisplayLink.create(window.screenId(), onNextFrame = {
        performDrawing(syncWithCA = false)
    })

    private val directContext = DirectContext.makeMetal(device.pointerAddress, queue.pointerAddress)
    var view: MetalView = MetalView.create(device)
    private val creationTime = TimeSource.Monotonic.markNow()

    init {
        window.setTextInputClient(object : TextInputClient {
            override fun hasMarkedText(): Boolean {
                Logger.info { "hasMarkedText" }
                return false
            }

            override fun markedRange(): TextRange? {
                Logger.info { "markedRange" }
                return null
            }

            override fun selectedRange(): TextRange? {
                Logger.info { "selectedRange" }
                return null
            }

            override fun insertText(text: String, replacementRange: TextRange?) {
                Logger.info { "insertText: $text, $replacementRange" }
            }

            override fun doCommand(command: String): Boolean {
                Logger.info { "doCommand: $command" }
                return false
            }

            override fun unmarkText() {
                Logger.info { "unmarkText" }
            }

            override fun setMarkedText(text: String, selectedRange: TextRange?, replacementRange: TextRange?) {
                Logger.info { "setMarkedText: $text, $selectedRange, $replacementRange" }
            }

            override fun attributedStringForRange(range: TextRange): TextInputClient.StringAndRange {
                Logger.info { "attributedStringForRange: $range" }
                return TextInputClient.StringAndRange("", TextRange(0, 0))
            }

            override fun firstRectForCharacterRange(range: TextRange): TextInputClient.RectAndRange {
                Logger.info { "firstRectForCharacterRange: $range" }
                return TextInputClient.RectAndRange(LogicalRect.Zero, TextRange(0, 0))
            }

            override fun characterIndexForPoint(point: LogicalPoint): Long {
                Logger.info { "characterIndexForPoint: $point" }
                return 0L
            }
        })
        window.minSize = LogicalSize(320.0, 240.0)
        view.isOpaque = false
        window.attachView(view)
    }

    private fun updateDisplayLink(screenId: ScreenId) {
        val isRunning = displayLink.isRunning()
        displayLink.close()
        displayLink = DisplayLink.create(screenId, onNextFrame = {
            performDrawing(syncWithCA = false)
        })
        displayLink.setRunning(isRunning)
    }

    open fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.WindowScreenChange -> {
                updateDisplayLink(screenId = event.newScreenId)
                EventHandlerResult.Continue
            }
            else -> EventHandlerResult.Continue
        }
    }

    fun performDrawing(syncWithCA: Boolean) {
        val size = view.size()
        view.nextTexture().use { texture ->
//             sleep(100) // uncomment this to check window resize quality
            BackendRenderTarget.makeMetal(size.width.toInt(), size.height.toInt(), texture.pointerAddress).use { renderTarget ->
                Surface.makeFromBackendRenderTarget(
                    context = directContext,
                    origin = SurfaceOrigin.TOP_LEFT,
                    colorFormat = SurfaceColorFormat.BGRA_8888,
                    colorSpace = ColorSpace.sRGB,
                    surfaceProps = null,
                    rt = renderTarget,
                )!!.use { surface ->
                    val time = creationTime.elapsedNow().inWholeMilliseconds
                    surface.canvas.clear(Color.TRANSPARENT)
                    surface.canvas.draw(PhysicalSize(surface.width.toDouble(), surface.height.toDouble()), time)
                    surface.flushAndSubmit()
                }
            }
            view.present(queue, waitForCATransaction = syncWithCA)
        }
    }

    abstract fun Canvas.draw(size: PhysicalSize, time: Long)

    override fun close() {
        displayLink.close()
        directContext.close()
        view.close()
        window.close()
    }
}
