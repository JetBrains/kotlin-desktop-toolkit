package org.jetbrains.desktop.buildscripts

import org.gradle.api.Project

 // `rustup target add --toolchain 1.90.0 x86_64-apple-darwin`

data class CrossCompilationSettings(private val platforms: List<Platform>) {
    companion object {
        private fun getBooleanProperty(project: Project, name: String): Boolean {
            return (project.property(name) as String).toBooleanStrict()
        }

        private fun enabled(targetPlatform: Platform, host: Platform, project: Project): Boolean {
            if (targetPlatform == host) {
                return true
            }

            return when (targetPlatform.os) {
                Os.LINUX -> when (targetPlatform.arch) {
                    Arch.aarch64 -> getBooleanProperty(project, "enableCrossCompileToLinuxAarch64")
                    Arch.x86_64 -> getBooleanProperty(project, "enableCrossCompileToLinuxX86_64")
                }
                Os.MACOS -> when (targetPlatform.arch) {
                    Arch.aarch64 -> getBooleanProperty(project, "enableCrossCompileToMacosAarch64")
                    Arch.x86_64 -> getBooleanProperty(project, "enableCrossCompileToMacosX86_64")
                }
                Os.WINDOWS -> when (targetPlatform.arch) {
                    Arch.aarch64 -> getBooleanProperty(project, "enableCrossCompileToWindowsAarch64")
                    Arch.x86_64 -> getBooleanProperty(project, "enableCrossCompileToWindowsX86_64")
                }
            }
        }

        fun create(project: Project): CrossCompilationSettings {
            val host = hostPlatform()
            val targetArch = targetArch(project)
            return CrossCompilationSettings(buildList {
                for (os in Os.entries) {
                    for (arch in Arch.entries) {
                        if (targetArch == null || arch == targetArch) {
                            val platform = Platform(os, arch)
                            if (enabled(platform, host, project)) {
                                add(platform)
                            }
                        }
                    }
                }
            })
        }
    }

    fun enabled(targetPlatform: Platform): Boolean = platforms.contains(targetPlatform)
}
