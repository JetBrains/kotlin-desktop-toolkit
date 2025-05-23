package org.jetbrains.desktop.common

internal class Platform private constructor() {
    enum class Type {
        Windows,
        Linux,
        MacOS,
        Unknown,
    }

    val name: String
        get() = System.getProperty("os.name")

    val version: String
        get() = System.getProperty("os.version").lowercase()

    val arch: String
        get() = System.getProperty("os.arch")

    val type: Type
        get() {
            val normalizedName = this.name.lowercase()
            return when {
                normalizedName.startsWith("mac") -> Type.MacOS
                normalizedName.startsWith("win") -> Type.Windows
                normalizedName.contains("nix") || normalizedName.contains("nux") -> Type.Linux
                else -> Type.Unknown
            }
        }

    val isAarch64: Boolean
        get() {
            val arch = this.arch
            return "aarch64" == arch || "arm64" == arch
        }

    val isMac: Boolean
        get() = this.type == Type.MacOS

    val isWindows: Boolean
        get() = this.type == Type.Windows

    val isLinux: Boolean
        get() = this.type == Type.Linux

    val isUnix: Boolean
        get() {
            val type = this.type
            return type == Type.Linux || type == Type.MacOS
        }

    companion object {
        val INSTANCE: Platform = Platform()
    }
}
