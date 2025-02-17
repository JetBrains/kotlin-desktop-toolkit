package org.jetbrains.desktop.macos

import org.jetbrains.desktop.LogicalPoint
import org.jetbrains.desktop.LogicalSize
import org.jetbrains.desktop.macos.generated.DisplayLinkCallback
import org.jetbrains.desktop.macos.generated.ScreenInfo
import org.jetbrains.desktop.macos.generated.ScreenInfoArray
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

typealias ScreenId = Int

class DisplayLink internal constructor(
    ptr: MemorySegment,
    val arena: Arena,
) : Managed(
    ptr,
    desktop_macos_h::display_link_drop,
) {
    companion object {
        fun create(screenId: ScreenId, onNextFrame: () -> Unit): DisplayLink {
            val arena = Arena.ofConfined()
            val callback = DisplayLinkCallback.allocate({
                ffiUpCall {
                    // todo don't execute this callback after the link was closed
                    onNextFrame()
                }
            }, arena)
            try {
                val ptr = ffiDownCall { desktop_macos_h.display_link_create(screenId, callback) }
                return DisplayLink(ptr, arena)
            } catch (e: Throwable) {
                arena.close()
                throw e
            }
        }
    }

    fun setRunning(value: Boolean) {
        ffiDownCall {
            desktop_macos_h.display_link_set_running(pointer, value)
        }
    }

    fun isRunning(): Boolean {
        return ffiDownCall {
            desktop_macos_h.display_link_is_running(pointer)
        }
    }

    override fun close() {
        super.close()
        // We need this dispatchOnMain to make sure that currently we don't have this display link callback in stacktrace
        // otherwise we will get cryptic sigsegv during the next garbage collection
        GrandCentralDispatch.dispatchOnMain {
            arena.close()
        }
    }
}

data class Screen(
    val screenId: ScreenId,
    val isPrimary: Boolean,
    val name: String,
    val origin: LogicalPoint,
    val size: LogicalSize,
    val scale: Double,
    val maximumFramesPerSecond: Int,
) {
    companion object {
        private fun fromNative(s: MemorySegment): Screen {
            return Screen(
                screenId = ScreenInfo.screen_id(s),
                isPrimary = ScreenInfo.is_primary(s),
                name = ScreenInfo.name(s).getUtf8String(0),
                origin = LogicalPoint.fromNative(ScreenInfo.origin(s)),
                size = LogicalSize.fromNative(ScreenInfo.size(s)),
                scale = ScreenInfo.scale(s),
                maximumFramesPerSecond = ScreenInfo.maximum_frames_per_second(s),
            )
        }

        fun allScreens(): AllScreens {
            return Arena.ofConfined().use { arena ->
                val screenInfoArray = ffiDownCall { desktop_macos_h.screen_list(arena) }
                val screens = mutableListOf<Screen>()
                try {
                    val ptr = ScreenInfoArray.ptr(screenInfoArray)
                    val len = ScreenInfoArray.len(screenInfoArray)

                    for (i in 0 until len) {
                        screens.add(fromNative(ScreenInfo.asSlice(ptr, i)))
                    }
                } finally {
                    ffiDownCall { desktop_macos_h.screen_list_drop(screenInfoArray) }
                }
                AllScreens(screens)
            }
        }
    }
}

data class AllScreens(val screens: List<Screen>) {
    fun mainScreen(): Screen {
        val screenId = ffiDownCall {
            desktop_macos_h.screen_get_main_screen_id()
        }
        return screens.first { it.screenId == screenId }
    }

    fun findById(screenId: ScreenId): Screen? {
        return screens.firstOrNull { it.screenId == screenId }
    }
}
