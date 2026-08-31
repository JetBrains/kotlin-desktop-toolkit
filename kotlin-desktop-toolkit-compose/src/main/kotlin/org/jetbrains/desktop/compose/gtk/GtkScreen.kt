package org.jetbrains.desktop.compose.gtk

import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpSize
import org.jetbrains.desktop.compose.ComposeScreen
import org.jetbrains.desktop.gtk.Screen

/**
 * Snapshot of one GTK monitor.
 *
 * A data class on purpose: the application replaces every instance when the monitor configuration changes,
 * and structural equality is what keeps that from invalidating readers whose screen did not actually
 * change.
 *
 * Unlike the Wayland backend, GTK reports a scale per monitor, so [density] is a real value here rather
 * than a placeholder of 1 — though a window still follows its own `WindowScaleChanged`, which is what
 * counts while a window straddles two monitors.
 */
internal data class GtkScreen(override val nativeScreen: Screen) : ComposeScreen {
    override val name: String = nativeScreen.name ?: "monitor ${nativeScreen.screenId}"
    override val position: DpOffset = nativeScreen.origin.toDpOffset()
    override val size: DpSize = nativeScreen.size.toDpSize()
    override val density: Density = Density(nativeScreen.scale.rawValue.toFloat())

    /**
     * GTK reports the refresh rate in millihertz, and 0 when it does not know it. Coerced to a positive
     * value so that content dividing by it cannot trip over a zero; nothing in this backend paces frames
     * off it, because GTK drives the frame clock itself (see `Event.WindowFrameTick`).
     */
    override val maximumFramesPerSecond: Int = (nativeScreen.millihertz.toInt() / 1000).coerceAtLeast(1)
}
