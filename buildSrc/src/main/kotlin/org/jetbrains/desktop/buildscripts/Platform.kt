package org.jetbrains.desktop.buildscripts

enum class Os {
    LINUX,
    MACOS,
    WINDOWS,
}

enum class Arch {
    AARCH64,
    X86_64,
}

fun buildOs(): Os {
    val os = System.getProperty("os.name").lowercase()
    return when {
        os.contains("win") -> Os.WINDOWS
        os.contains("mac") -> Os.MACOS
        os.contains("nux") || os.contains("nix") || os.contains("aix") -> Os.LINUX
        else -> error("unsupported os '$os'")
    }
}

fun buildArch(): Arch = when (val arch = System.getProperty("os.arch").lowercase()) {
    "x86_64", "amd64", "x64" -> Arch.X86_64
    "arm64", "aarch64" -> Arch.AARCH64
    else -> error("unsupported arch '$arch'")
}
