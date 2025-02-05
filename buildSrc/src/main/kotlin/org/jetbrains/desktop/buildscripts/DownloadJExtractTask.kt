package org.jetbrains.desktop.buildscripts

import org.gradle.api.Action
import org.gradle.api.DefaultTask
import org.gradle.api.file.ArchiveOperations
import org.gradle.api.file.FileVisitDetails
import org.gradle.api.model.ObjectFactory
import org.gradle.api.provider.ProviderFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import java.net.HttpURLConnection
import java.net.URI
import java.nio.file.Files
import java.nio.file.Path
import javax.inject.Inject
import kotlin.io.path.createTempFile

abstract class DownloadJExtractTask @Inject constructor(
    objectFactory: ObjectFactory,
    providerFactory: ProviderFactory,
    private val archiveOperations: ArchiveOperations
) : DefaultTask() {
    @get:Input
    val platform = providerFactory.provider { buildPlatform() }

    @get:Input
    val slug = objectFactory.property<String>()

    @get:OutputDirectory
    val jextractDirectory = objectFactory.directoryProperty()

    @get:OutputFile
    val jextractBinary = providerFactory.provider {
        val dir = jextractDirectory.get().asFile
        when (platform) { // FIXME: implement for each platform if required
            else -> dir.resolve("jextract-22/bin/jextract") // FIXME: path under the directory
        }
    }

    @TaskAction
    fun download() {
        downloadJExtract(
            platform.get(),
            slug.get(),
            jextractDirectory.get().asFile.toPath(),
            archiveOperations,
        )
    }
}

private fun downloadJExtract(
    platform: String,
    slug: String,
    jextractDirectory: Path,
    archiveOperations: ArchiveOperations,
) {
    val url = URI(jextractUrl(platform, slug)).toURL()

    val connection = url.openConnection() as HttpURLConnection
    val tempFile = createTempFile("jextract", ".tar.gz").toFile()

    connection.inputStream.use { input ->
        tempFile.outputStream().use { output -> input.copyTo(output) }
    }

    archiveOperations.tarTree(tempFile).visit {
        val details = this
        if (!details.isDirectory) {
            val targetFile = jextractDirectory.resolve(details.relativePath.pathString)
            Files.createDirectories(targetFile.parent)
            targetFile.toFile().let {
                details.file.copyTo(it, overwrite = true)
                if (details.permissions.user.execute) {
                    it.setExecutable(true)
                }
            }
        }
    }
    tempFile.delete()
}

private fun jextractUrl(
    platform: String,
    slug: String,
): String = "https://download.java.net/java/early_access/jextract/${slug}_${platform}_bin.tar.gz"

private fun buildPlatform(): String = jextractPlatform(buildOs(), buildArch())

private fun jextractPlatform(os: Os, arch: Arch): String {
    val jextractOs = when (os) {
        Os.WINDOWS -> "windows"
        Os.LINUX -> "linux"
        Os.MACOS -> "macos"
    }
    val jextractArch = when (arch) {
        Arch.x86_64 -> "x64"
        Arch.aarch64 -> "aarch64"
    }
    return "${jextractOs}-$jextractArch"
}