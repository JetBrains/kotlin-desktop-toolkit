package org.jetbrains.desktop.sample.compose.windows

import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpSize
import org.jetbrains.desktop.sample.compose.ComposeScreen
import org.jetbrains.desktop.win32.Screen

/**
 * Snapshot of one display, as `EnumDisplayMonitors` reported it.
 *
 * A data class on purpose: the application replaces every instance when the display configuration
 * changes, and structural equality is what keeps that from invalidating readers whose screen did not
 * actually change.
 *
 * The origin arrives in physical pixels — the virtual desktop is laid out in them — so it is the one
 * value here that has to be divided by the screen's own scale.
 */
internal data class WindowsScreen(override val nativeScreen: Screen) : ComposeScreen {
    override val name: String = nativeScreen.name ?: ""
    override val position: DpOffset = nativeScreen.origin.toDpOffset(nativeScreen.scale)
    override val size: DpSize = nativeScreen.size.toDpSize()
    override val density: Density = Density(nativeScreen.scale)
    override val maximumFramesPerSecond: Int = nativeScreen.maximumFramesPerSecond
}
