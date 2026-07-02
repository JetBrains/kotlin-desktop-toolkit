package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h

/**
 * Phase of a continuous gesture (pinch, rotate, swipe).
 * Apple doc: https://developer.apple.com/documentation/appkit/nsevent/phase?language=objc
 */
public enum class EventPhase {
    None,
    Began,
    Stationary,
    Changed,
    Ended,
    Cancelled,
    MayBegin,
    ;

    internal companion object {
        internal fun fromNative(value: Int): EventPhase {
            return when (value) {
                desktop_macos_h.NativeEventPhase_None() -> None
                desktop_macos_h.NativeEventPhase_Began() -> Began
                desktop_macos_h.NativeEventPhase_Stationary() -> Stationary
                desktop_macos_h.NativeEventPhase_Changed() -> Changed
                desktop_macos_h.NativeEventPhase_Ended() -> Ended
                desktop_macos_h.NativeEventPhase_Cancelled() -> Cancelled
                desktop_macos_h.NativeEventPhase_MayBegin() -> MayBegin
                else -> throw Error("Unexpected variant $value")
            }
        }
    }
}
