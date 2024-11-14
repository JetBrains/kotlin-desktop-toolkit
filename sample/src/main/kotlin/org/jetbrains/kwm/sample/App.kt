package org.jetbrains.kwm.sample

import org.jetbrains.kwm.Library


fun main() {
    val a = 4
    val b = 2
    val result = Library().someLibraryMethod(a, b)
    println("Hello from app!! result from library: $a + $b = $result")
}