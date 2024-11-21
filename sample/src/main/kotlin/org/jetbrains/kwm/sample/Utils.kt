package org.jetbrains.kwm.sample

fun printRuntimeInfo() {
    val javaVersion = System.getProperty("java.runtime.version", System.getProperty("java.version", "unknown"))
    val javaVendor = System.getProperty("java.vendor")
    println("""
        Java vendor: $javaVendor
        Java version: $javaVersion
    """.trimIndent())
}