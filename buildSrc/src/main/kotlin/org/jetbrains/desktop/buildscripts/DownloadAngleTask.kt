package org.jetbrains.desktop.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.file.ArchiveOperations
import org.gradle.api.file.FileSystemOperations
import org.gradle.api.model.ObjectFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.OutputFiles
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import java.net.URI
import java.nio.file.Path
import javax.inject.Inject
import kotlin.io.path.createTempFile

abstract class DownloadAngleTask @Inject constructor(
    objectFactory: ObjectFactory,
    private val archiveOperations: ArchiveOperations,
    private val fsOperations: FileSystemOperations,
) : DefaultTask() {
    @get:Input
    val platform = objectFactory.property<Platform>()

    @get:Input
    val version = objectFactory.property<String>()

    @get:OutputDirectory
    val outputDirectory = objectFactory.directoryProperty()

    @get:OutputFiles
    val binaries = outputDirectory.map { dir ->
        dir.asFileTree
            .matching { include("**/libEGL.dll", "**/libGLESv2.dll") }
            .files
    }

    @TaskAction
    fun download() {
        downloadAngle(
            platform.get(),
            version.get(),
            outputDirectory.get().asFile.toPath(),
            fsOperations,
            archiveOperations,
        )
    }
}

private fun downloadAngle(
    platform: Platform,
    version: String,
    targetDir: Path,
    fs: FileSystemOperations,
    archiveOperations: ArchiveOperations
) {
    val tempFile = createTempFile("angle", ".zip").toFile()
    val url = URI(angleUrl(platform, version)).toURL()

    url.openStream().use { input ->
        tempFile.outputStream().use { output -> input.copyTo(output) }
    }

    fs.sync {
        from(archiveOperations.zipTree(tempFile))
        into(targetDir)
    }

    tempFile.delete()
}

private fun angleUrl(platform: Platform, version: String): String {
    val angleOs = when (platform.os) {
        Os.WINDOWS -> "windows"
        else -> error("ANGLE is currently only supported on Windows. Current OS: ${platform.os}")
    }
    val angleArch = angleArch(platform.arch)
    return "https://github.com/JetBrains/angle-pack/releases/download/${version}/Angle-${version}-${angleOs}-Release-${angleArch}.zip"
}

fun angleArch(arch: Arch): String = when (arch) {
    Arch.x86_64 -> "x64"
    Arch.aarch64 -> "arm64"
}
