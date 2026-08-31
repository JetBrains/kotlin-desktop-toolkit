package org.jetbrains.desktop.compose.macos

import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpSize
import org.jetbrains.desktop.compose.ComposeScreen
import org.jetbrains.desktop.macos.Screen

/**
 * Snapshot of one `NSScreen`.
 *
 * A data class on purpose: the application replaces every instance when the display configuration
 * changes, and structural equality is what keeps that from invalidating readers whose screen did not
 * actually change.
 */
internal data class MacOsScreen(override val nativeScreen: Screen) : ComposeScreen {
    override val name: String = nativeScreen.name
    override val position: DpOffset = nativeScreen.origin.toDpOffset()
    override val size: DpSize = nativeScreen.size.toDpSize()
    override val density: Density = Density(nativeScreen.scale.toFloat())
    override val maximumFramesPerSecond: Int = nativeScreen.maximumFramesPerSecond
}
