package org.jetbrains.kwm.sample

import org.jetbrains.kwm.Point
import org.jetbrains.kwm.Size
import org.jetbrains.kwm.macos.*
import org.jetbrains.skia.*
import kotlin.time.TimeSource

abstract class SkikoWindow(device: MetalDevice,
                           val queue: MetalCommandQueue,
                           title: String,
                           position: Point): AutoCloseable {

    val window = Window.create(title,
                               position.x.toFloat(),
                               position.y.toFloat(),
                               onResize = {
                                   performDrawing()
                               })
    val displayLink = DisplayLink.createForWindow(window, onNextFrame = {
        performDrawing()
    })
    val directContext = DirectContext.makeMetal(device.pointer.address(), queue.pointer.address())
    var view: MetalView = MetalView.create(device)
    val creationTime = TimeSource.Monotonic.markNow()

    init {
        window.attachView(view)
        performDrawing()
    }

    private fun performDrawing() {
        val size = view.size()
        view.nextTexture().use { texture ->
//             sleep(100) // uncomment this to check window resize quality
            BackendRenderTarget.makeMetal(size.width.toInt(), size.height.toInt(), texture.pointer.address()).use { renderTarget ->
                Surface.makeFromBackendRenderTarget(
                    context = directContext,
                    origin = SurfaceOrigin.TOP_LEFT,
                    colorFormat = SurfaceColorFormat.BGRA_8888,
                    colorSpace = null,
                    surfaceProps = null,
                    rt = renderTarget
                )!!.use { surface ->
                    val time = creationTime.elapsedNow().inWholeMilliseconds
                    surface.canvas.draw(Size(surface.width.toDouble(), surface.height.toDouble()), time)
                    surface.flushAndSubmit()
                }
            }
            queue.commit()
            view.present()
        }
    }

    abstract fun Canvas.draw(size: Size, time: Long)

    override fun close() {
        displayLink.close()
        directContext.close()
        view.close()
        window.close()
    }

}