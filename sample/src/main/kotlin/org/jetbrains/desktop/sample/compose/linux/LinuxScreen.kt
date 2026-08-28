package org.jetbrains.desktop.sample.compose.linux

import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpSize
import org.jetbrains.desktop.linux.Screen
import org.jetbrains.desktop.sample.compose.ComposeScreen

/**
 * Snapshot of one Wayland output.
 *
 * A data class on purpose: the application replaces every instance when the output configuration
 * changes, and structural equality is what keeps that from invalidating readers whose screen did not
 * actually change.
 *
 * [density] is always 1: Wayland scales per surface, not per output, so the real density is the one the
 * window reports through `WindowScaleChanged` — see [org.jetbrains.desktop.sample.compose.ComposeWindow.density].
 */
internal data class LinuxScreen(override val nativeScreen: Screen) : ComposeScreen {
    override val name: String = nativeScreen.name ?: "output ${nativeScreen.screenId}"
    override val position: DpOffset = nativeScreen.origin.toDpOffset()
    override val size: DpSize = nativeScreen.size.toDpSize()
    override val density: Density = Density(1f)
    override val maximumFramesPerSecond: Int = nativeScreen.maximumFramesPerSecond
}
