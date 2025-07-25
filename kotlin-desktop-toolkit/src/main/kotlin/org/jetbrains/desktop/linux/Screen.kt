package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeScreenInfo
import java.lang.foreign.MemorySegment

public typealias ScreenId = Int

public data class Screen(
    val screenId: ScreenId,
    val isPrimary: Boolean,
    val name: String?,
    val origin: LogicalPoint,
    val size: LogicalSize,
    val scale: Double,
    val maximumFramesPerSecond: Int,
) {
    public companion object {
        internal fun fromNative(s: MemorySegment): Screen {
            val nativeName = NativeScreenInfo.name(s)
            return Screen(
                screenId = NativeScreenInfo.screen_id(s),
                isPrimary = NativeScreenInfo.is_primary(s),
                name = if (nativeName == MemorySegment.NULL) {
                    null
                } else {
                    nativeName.getUtf8String(0)
                },
                origin = LogicalPoint.fromNative(NativeScreenInfo.origin(s)),
                size = LogicalSize.fromNative(NativeScreenInfo.size(s)),
                scale = NativeScreenInfo.scale(s),
                maximumFramesPerSecond = NativeScreenInfo.maximum_frames_per_second(s),
            )
        }
    }
}

public data class AllScreens(val screens: List<Screen>) {
    public fun findById(screenId: ScreenId): Screen? {
        return screens.firstOrNull { it.screenId == screenId }
    }
}
