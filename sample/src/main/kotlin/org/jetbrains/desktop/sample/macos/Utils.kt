package org.jetbrains.desktop.sample.macos

import org.jetbrains.desktop.macos.PhysicalPoint
import org.jetbrains.skia.Canvas

fun Canvas.withTranslated(point: PhysicalPoint, block: Canvas.() -> Unit) = let { canvas ->
    canvas.save()
    canvas.translate(point.x.toFloat(), point.y.toFloat())
    canvas.block()
    canvas.restore()
}
