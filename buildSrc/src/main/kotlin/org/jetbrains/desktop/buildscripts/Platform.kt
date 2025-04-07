package org.jetbrains.desktop.buildscripts

import org.gradle.api.GradleException
import org.gradle.api.Project
import org.gradle.api.tasks.Input
import org.gradle.internal.impldep.kotlinx.serialization.Serializable


@Serializable
data class Platform(
    @get:Input val os: Os,
    @get:Input val arch: Arch,
)

enum class Os(val normalizedName: String) {
    LINUX("linux"), MACOS("macos"), WINDOWS("windows");
}

enum class Arch {
    aarch64, x86_64
}

fun hostOs(): Os  {
    val os = System.getProperty("os.name").lowercase()
    return when {
        os.contains("win") -> Os.WINDOWS
        os.contains("mac") -> Os.MACOS
        os.contains("nux") || os.contains("nix") || os.contains("aix") -> Os.LINUX
        else -> error("unsupported os '$os'")
    }
}

fun targetOs(project: Project): Os  {
    val projectTargetOsName = project.properties["targetOs"]
    return when(projectTargetOsName) {
        "macos" -> Os.MACOS
        "linux" -> Os.LINUX
        "windows" -> Os.WINDOWS
        null -> hostOs()
        else -> throw GradleException("Unsupported target os: $projectTargetOsName")
    }
}

fun targetArch(project: Project): Arch {
    val projectTargetArchName = project.properties["targetArch"]
    return when (projectTargetArchName) {
        "x86_64" -> Arch.x86_64
        "aarch64" -> Arch.aarch64
        null -> hostArch()
        else -> throw GradleException("Unsupported target arch: $projectTargetArchName")
    }
}

fun hostArch(): Arch = when (val arch = System.getProperty("os.arch").lowercase()) {
    "x86_64", "amd64", "x64" -> Arch.x86_64
    "arm64", "aarch64" -> Arch.aarch64
    else -> error("unsupported arch '$arch'")
}
