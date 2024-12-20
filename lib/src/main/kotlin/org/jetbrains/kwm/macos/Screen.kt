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
            val callback = DisplayLinkCallback.allocate(onNextFrame, arena)
            return DisplayLink(kwm_macos_h.display_link_create(screenId, callback), arena)
        }
    }

    fun setRunning(value: Boolean) {
        kwm_macos_h.display_link_set_running(pointer, value);
    }

    fun isRunning(): Boolean {
        return kwm_macos_h.display_link_is_running(pointer)
    }

    override fun close() {
        super.close()
        arena.close()
    }
}

data class Screen(
    val screenId: ScreenId,
    val isMain: Boolean,
    val name: String,
    val origin: LogicalPoint,
    val size: LogicalSize,
    val scale: Double) {
    companion object {
        internal fun fromNative(s: MemorySegment): Screen {
            return Screen(
                screenId = ScreenInfo.screen_id(s),
                isMain = ScreenInfo.is_main(s),
                name = ScreenInfo.name(s).getUtf8String(0),
                origin = LogicalPoint.fromNative(ScreenInfo.origin(s)),
                size = LogicalSize.fromNative(ScreenInfo.size(s)),
                scale = ScreenInfo.scale(s)
            )
        }

        fun allScreens(): List<Screen> {
            return Arena.ofConfined().use { arena ->
                val screenInfoArray = kwm_macos_h.screen_list(arena)
                val screens = mutableListOf<Screen>()
                try {
                    val ptr = ScreenInfoArray.ptr(screenInfoArray)
                    val len = ScreenInfoArray.len(screenInfoArray)

                    for (i in 0 until len) {
                        screens.add(fromNative(ScreenInfo.asSlice(ptr, i)))
                    }
                } finally {
                    kwm_macos_h.screen_list_drop(screenInfoArray)
                }
                screens
            }
        }
    }
}