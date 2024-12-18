package org.jetbrains.kwm

typealias PhysicalPixels = Double
typealias LogicalPixels = Double

data class PhysicalSize(val width: PhysicalPixels, val height: PhysicalPixels) {
    companion object
}

data class PhysicalPoint(val x: PhysicalPixels, val y: PhysicalPixels) {
    companion object
}

data class LogicalSize(val width: LogicalPixels, val height: LogicalPixels) {
    companion object
}

data class LogicalPoint(val x: LogicalPixels, val y: LogicalPixels) {
    companion object
}