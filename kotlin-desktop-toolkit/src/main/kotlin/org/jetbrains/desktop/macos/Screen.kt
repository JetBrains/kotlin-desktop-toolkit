package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeDisplayLinkCallback
import org.jetbrains.desktop.macos.generated.NativeScreenInfo
import org.jetbrains.desktop.macos.generated.NativeScreenInfoArray
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public typealias ScreenId = Int

public class DisplayLink internal constructor(
    ptr: MemorySegment,
    private val arena: Arena,
) : Managed(
    ptr,
    desktop_macos_h::display_link_drop,
) {
    public companion object {
        public fun create(screenId: ScreenId, onNextFrame: () -> Unit): DisplayLink {
            val arena = Arena.ofConfined()
            val callback = NativeDisplayLinkCallback.allocate({
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

    public fun setRunning(value: Boolean) {
        ffiDownCall {
            desktop_macos_h.display_link_set_running(pointer, value)
        }
    }

    public fun isRunning(): Boolean {
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

public data class Screen(
    val screenId: ScreenId,
    val isPrimary: Boolean,
    val name: String,
    val origin: LogicalPoint,
    val size: LogicalSize,
    val scale: Double,
    val maximumFramesPerSecond: Int,
) {
    public companion object {
        private fun fromNative(s: MemorySegment): Screen {
            return Screen(
                screenId = NativeScreenInfo.screen_id(s),
                isPrimary = NativeScreenInfo.is_primary(s),
                name = NativeScreenInfo.name(s).getUtf8String(0),
                origin = LogicalPoint.fromNative(NativeScreenInfo.origin(s)),
                size = LogicalSize.fromNative(NativeScreenInfo.size(s)),
                scale = NativeScreenInfo.scale(s),
                maximumFramesPerSecond = NativeScreenInfo.maximum_frames_per_second(s),
            )
        }

        public fun allScreens(): AllScreens {
            return Arena.ofConfined().use { arena ->
                val screenInfoArray = ffiDownCall { desktop_macos_h.screen_list(arena) }
                val screens = mutableListOf<Screen>()
                try {
                    val ptr = NativeScreenInfoArray.ptr(screenInfoArray)
                    val len = NativeScreenInfoArray.len(screenInfoArray)

                    for (i in 0 until len) {
                        screens.add(fromNative(NativeScreenInfo.asSlice(ptr, i)))
                    }
                } finally {
                    ffiDownCall { desktop_macos_h.screen_list_drop(screenInfoArray) }
                }
                AllScreens(screens)
            }
        }
    }
}

public data class AllScreens(val screens: List<Screen>) {
    public fun mainScreen(): Screen {
        val screenId = ffiDownCall {
            desktop_macos_h.screen_get_main_screen_id()
        }
        return screens.first { it.screenId == screenId }
    }

    public fun findById(screenId: ScreenId): Screen? {
        return screens.firstOrNull { it.screenId == screenId }
    }
}
