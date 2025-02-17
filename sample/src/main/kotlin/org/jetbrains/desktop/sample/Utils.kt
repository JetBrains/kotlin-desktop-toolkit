package org.jetbrains.desktop.sample

import org.jetbrains.desktop.PhysicalPoint
import org.jetbrains.skia.Canvas

fun runtimeInfo(): String {
    val javaVersion = System.getProperty("java.runtime.version", System.getProperty("java.version", "unknown"))
    val javaVendor = System.getProperty("java.vendor")
    return """
        Java vendor: $javaVendor
        Java version: $javaVersion
    """.trimIndent()
}

fun Canvas.withTranslated(point: PhysicalPoint, block: Canvas.() -> Unit) = let { canvas ->
    canvas.save()
    canvas.translate(point.x.toFloat(), point.y.toFloat())
    canvas.block()
    canvas.restore()
}
