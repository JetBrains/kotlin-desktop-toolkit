package org.jetbrains.desktop.gtk

import org.jetbrains.desktop.gtk.generated.NativeScreenInfo
import java.lang.foreign.MemorySegment

public typealias ScreenId = ULong

public data class Screen(
    val screenId: ScreenId,
    val name: String?,
    val origin: LogicalPoint,
    val size: LogicalSize,
    val scale: Double,
    val millihertz: UInt,
) {
    public companion object {
        internal fun fromNative(s: MemorySegment): Screen {
            val nativeName = NativeScreenInfo.name(s)
            return Screen(
                screenId = NativeScreenInfo.screen_id(s).toULong(),
                name = if (nativeName == MemorySegment.NULL) {
                    null
                } else {
                    nativeName.getUtf8String(0)
                },
                origin = LogicalPoint.fromNative(NativeScreenInfo.origin(s)),
                size = LogicalSize.fromNative(NativeScreenInfo.size(s)),
                scale = NativeScreenInfo.scale(s),
                millihertz = NativeScreenInfo.millihertz(s).toUInt(),
            )
        }
    }
}

public data class AllScreens(val screens: List<Screen>) {
    public fun findById(screenId: ScreenId): Screen? {
        return screens.firstOrNull { it.screenId == screenId }
    }
}
