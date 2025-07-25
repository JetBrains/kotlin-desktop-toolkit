package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.PhysicalPoint
import org.jetbrains.skia.Canvas

fun Canvas.withTranslated(point: PhysicalPoint, block: Canvas.() -> Unit) = let { canvas ->
    canvas.save()
    canvas.translate(point.x.toFloat(), point.y.toFloat())
    canvas.block()
    canvas.restore()
}
