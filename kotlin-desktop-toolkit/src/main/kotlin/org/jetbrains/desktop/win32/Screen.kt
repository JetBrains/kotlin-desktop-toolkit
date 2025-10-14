package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeScreenInfo
import org.jetbrains.desktop.win32.generated.NativeScreenInfoArray
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public data class Screen(
    val isPrimary: Boolean,
    val name: String?,
    val origin: LogicalPoint,
    val size: LogicalSize,
    val scale: Float,
    val maximumFramesPerSecond: Int,
) {
    public companion object {
        internal fun fromNative(s: MemorySegment): Screen {
            return Screen(
                isPrimary = NativeScreenInfo.is_primary(s),
                name = NativeScreenInfo.name(s).getUtf8String(0),
                origin = LogicalPoint.fromNative(NativeScreenInfo.origin(s)),
                size = LogicalSize.fromNative(NativeScreenInfo.size(s)),
                scale = NativeScreenInfo.scale(s),
                maximumFramesPerSecond = NativeScreenInfo.maximum_frames_per_second(s),
            )
        }

        public fun allScreens(): List<Screen> {
            return Arena.ofConfined().use { arena ->
                val screenInfoArray = ffiDownCall { desktop_win32_h.screen_list(arena) }
                val screens = mutableListOf<Screen>()
                try {
                    val ptr = NativeScreenInfoArray.ptr(screenInfoArray)
                    val len = NativeScreenInfoArray.len(screenInfoArray)

                    for (i in 0 until len) {
                        screens.add(fromNative(NativeScreenInfo.asSlice(ptr, i)))
                    }
                } finally {
                    ffiDownCall { desktop_win32_h.screen_list_drop(screenInfoArray) }
                }
                screens.toList()
            }
        }
    }
}
