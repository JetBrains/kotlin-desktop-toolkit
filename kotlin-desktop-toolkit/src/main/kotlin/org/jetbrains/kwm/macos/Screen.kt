package org.jetbrains.kwm.macos

import org.jetbrains.kwm.macos.generated.DisplayLinkCallback
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import org.jetbrains.kwm.LogicalPoint
import org.jetbrains.kwm.LogicalSize
import org.jetbrains.kwm.macos.generated.ScreenInfo
import org.jetbrains.kwm.macos.generated.ScreenInfoArray
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

typealias ScreenId = Int

class DisplayLink internal constructor(ptr: MemorySegment, val arena: Arena): Managed(ptr,
    kwm_macos_h::display_link_drop
) {
    companion object {
        fun create(screenId: ScreenId, onNextFrame: () -> Unit): DisplayLink {
            val arena = Arena.ofConfined()
            val callback = DisplayLinkCallback.allocate({
                ffiUpCall {
                    onNextFrame()
                }
            }, arena)
            try {
                val ptr = ffiDownCall { kwm_macos_h.display_link_create(screenId, callback) }
                return DisplayLink(ptr, arena)
            } catch (e: Throwable) {
                arena.close()
                throw e
            }
        }
    }

    fun setRunning(value: Boolean) {
        ffiDownCall {
            kwm_macos_h.display_link_set_running(pointer, value)
        }
    }

    fun isRunning(): Boolean {
        return ffiDownCall {
            kwm_macos_h.display_link_is_running(pointer)
        }
    }

    override fun close() {
        super.close()
        arena.close()
    }
}

data class Screen(
    val screenId: ScreenId,
    val isPrimary: Boolean,
    val name: String,
    val origin: LogicalPoint,
    val size: LogicalSize,
    val scale: Double) {
    companion object {
        private fun fromNative(s: MemorySegment): Screen {
            return Screen(
                screenId = ScreenInfo.screen_id(s),
                isPrimary = ScreenInfo.is_primary(s),
                name = ScreenInfo.name(s).getUtf8String(0),
                origin = LogicalPoint.fromNative(ScreenInfo.origin(s)),
                size = LogicalSize.fromNative(ScreenInfo.size(s)),
                scale = ScreenInfo.scale(s)
            )
        }

        fun allScreens(): AllScreens {
            return Arena.ofConfined().use { arena ->
                    val screenInfoArray = ffiDownCall { kwm_macos_h.screen_list(arena) }
                    val screens = mutableListOf<Screen>()
                    try {
                        val ptr = ScreenInfoArray.ptr(screenInfoArray)
                        val len = ScreenInfoArray.len(screenInfoArray)

                        for (i in 0 until len) {
                            screens.add(fromNative(ScreenInfo.asSlice(ptr, i)))
                        }
                    } finally {
                        ffiDownCall { kwm_macos_h.screen_list_drop(screenInfoArray) }
                    }
                    AllScreens(screens)
            }
        }
    }
}

data class AllScreens(val screens: List<Screen>) {
    fun mainScreen(): Screen {
        val screenId = ffiDownCall {
            kwm_macos_h.screen_get_main_screen_id()
        }
        return screens.first { it.screenId == screenId }
    }
}