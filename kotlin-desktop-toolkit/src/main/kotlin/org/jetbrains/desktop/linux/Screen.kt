package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeScreenInfo
import java.lang.foreign.MemorySegment

public typealias ScreenId = Int

@ConsistentCopyVisibility
public data class Screen internal constructor(
    val screenId: ScreenId,
    val name: String?,
    val origin: LogicalPoint,
    val size: LogicalSize,
    val maximumFramesPerSecond: Int,
) {
    public companion object {
        internal fun fromNative(s: MemorySegment): Screen {
            val nativeName = NativeScreenInfo.name(s)
            return Screen(
                screenId = NativeScreenInfo.screen_id(s),
                name = readNativeAutoDropU8Array(nativeName)?.decodeToString(),
                origin = LogicalPoint.fromNative(NativeScreenInfo.origin(s)),
                size = LogicalSize.fromNative(NativeScreenInfo.size(s)),
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
