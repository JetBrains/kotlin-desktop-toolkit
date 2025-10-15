package org.jetbrains.desktop.buildscripts

fun List<String>.asCmdArgs(): String = joinToString("\" \"", prefix = "\"", postfix = "\"")
