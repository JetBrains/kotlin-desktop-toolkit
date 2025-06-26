package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeEglGetProcFuncData
import org.jetbrains.desktop.win32.generated.desktop_windows_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public class AngleRenderer internal constructor(
    private val angle_device_ptr: MemorySegment,
) : AutoCloseable {
    public companion object {
        public fun create(window: Window): AngleRenderer {
            return AngleRenderer(
                ffiDownCall {
                    window.withPointer { windowPtr ->
                        desktop_windows_h.renderer_angle_device_create(windowPtr)
                    }
                },
            )
        }
    }

    public data class EglGetProcFunc(val fPtr: Long, val ctxPtr: Long)

    public fun getEglGetProcFunc(): EglGetProcFunc {
        return Arena.ofConfined().use { arena ->
            val native = desktop_windows_h.renderer_angle_get_egl_get_proc_func(arena, angle_device_ptr)
            val f = NativeEglGetProcFuncData.f(native)
            val ctx = NativeEglGetProcFuncData.ctx(native)
            EglGetProcFunc(f.address(), ctx.address())
        }
    }

    override fun close() {
        ffiDownCall {
            // TODO
        }
    }
}
