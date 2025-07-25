package org.jetbrains.desktop.sample.common

fun runtimeInfo(): String {
    val javaVersion = System.getProperty("java.runtime.version", System.getProperty("java.version", "unknown"))
    val javaVendor = System.getProperty("java.vendor")
    return """
        Java vendor: $javaVendor
        Java version: $javaVersion
    """.trimIndent()
}
