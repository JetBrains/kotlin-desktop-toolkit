package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeEglGetProcFuncData
import org.jetbrains.desktop.win32.generated.NativeEglSurfaceData
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public class AngleRenderer internal constructor(private val angleDevicePtr: MemorySegment) : AutoCloseable {
    public companion object {
        public fun create(window: Window): AngleRenderer {
            return AngleRenderer(
                ffiDownCall {
                    window.withPointer { windowPtr ->
                        desktop_win32_h.renderer_angle_device_create(windowPtr)
                    }
                },
            )
        }
    }

    public data class EglGetProcFunc(
        val fPtr: Long,
        val ctxPtr: Long,
    )

    public fun getEglGetProcFunc(): EglGetProcFunc {
        return Arena.ofConfined().use { arena ->
            val native = ffiDownCall {
                desktop_win32_h.renderer_angle_get_egl_get_proc_func(arena, angleDevicePtr)
            }
            val f = NativeEglGetProcFuncData.f(native)
            val ctx = NativeEglGetProcFuncData.ctx(native)
            EglGetProcFunc(f.address(), ctx.address())
        }
    }

    public fun resizeSurface(width: Int, height: Int): SurfaceParams {
        return Arena.ofConfined().use { arena ->
            val native = ffiDownCall {
                desktop_win32_h.renderer_angle_resize_surface(arena, angleDevicePtr, width, height)
            }
            val framebufferBinding = NativeEglSurfaceData.framebuffer_binding(native)
            SurfaceParams(framebufferBinding)
        }
    }

    public fun makeCurrent() {
        ffiDownCall {
            desktop_win32_h.renderer_angle_make_current(angleDevicePtr)
        }
    }

    public fun swapBuffers() {
        ffiDownCall {
            desktop_win32_h.renderer_angle_swap_buffers(angleDevicePtr)
        }
    }

    public fun draw(drawFun: () -> Unit) {
        makeCurrent()
        drawFun()
        swapBuffers()
    }

    override fun close() {
        ffiDownCall {
            desktop_win32_h.renderer_angle_drop(angleDevicePtr)
        }
    }
}

public data class SurfaceParams(val framebufferBinding: Int)
