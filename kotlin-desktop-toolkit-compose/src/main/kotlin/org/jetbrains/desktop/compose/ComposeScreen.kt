package org.jetbrains.desktop.compose

import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpRect
import androidx.compose.ui.unit.DpSize

/**
 * A display, as reported by the platform.
 *
 * Instances are snapshots: they are replaced wholesale when the display configuration changes, so
 * reading them off [ComposeApplication.screens] or [ComposeWindow.screen] inside a composition is
 * enough to stay up to date.
 */
interface ComposeScreen {
    val name: String

    /** Position of the screen's top-left corner in the global desktop coordinate space. */
    val position: DpOffset
    val size: DpSize
    val bounds: DpRect get() = DpRect(position, size)

    val density: Density

    /** Highest frame rate this screen can present at, which bounds the window's frame pacing. */
    val maximumFramesPerSecond: Int

    /** The platform object behind this screen, for code that needs to reach past this abstraction. */
    val nativeScreen: Any
}
