package org.jetbrains.kwm.buildscripts

enum class Os {
    LINUX, MACOS, WINDOWS
}

enum class Arch {
    aarch64, x86_64
}

internal fun buildOs(): Os  {
    val os = System.getProperty("os.name").lowercase()
    return when {
        os.contains("win") -> Os.WINDOWS
        os.contains("mac") -> Os.MACOS
        os.contains("nux") || os.contains("nix") || os.contains("aix") -> Os.LINUX
        else -> error("unsupported os '$os'")
    }
}

internal fun buildArch(): Arch = when (val arch = System.getProperty("os.arch").lowercase()) {
    "x86_64", "amd64", "x64" -> Arch.x86_64
    "arm64", "aarch64" -> Arch.aarch64
    else -> error("unsupported arch '$arch'")
}
