package org.jetbrains.kwm.sample

import org.jetbrains.kwm.PhysicalPoint
import org.jetbrains.skia.Canvas

fun printRuntimeInfo() {
    val javaVersion = System.getProperty("java.runtime.version", System.getProperty("java.version", "unknown"))
    val javaVendor = System.getProperty("java.vendor")
    println("""
        Java vendor: $javaVendor
        Java version: $javaVersion
    """.trimIndent())
}

fun Canvas.withTranslated(point: PhysicalPoint, block: Canvas.() -> Unit) = let { canvas ->
    canvas.save()
    canvas.translate(point.x.toFloat(), point.y.toFloat())
    canvas.block()
    canvas.restore()
}