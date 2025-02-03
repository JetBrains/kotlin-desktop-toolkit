package org.jetbrains.kwm.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.model.ObjectFactory
import org.gradle.api.provider.ProviderFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import java.nio.file.Path
import javax.inject.Inject
import kotlin.io.path.createDirectories

abstract class DownloadJExtractTask @Inject constructor(
    objectFactory: ObjectFactory,
    providerFactory: ProviderFactory,
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
                else -> dir.resolve("jextract") // FIXME: path under the directory
        }
    }

    @TaskAction
    fun generate() {
        downloadJExtract(
            platform.get(),
            slug.get(),
            jextractDirectory.get().asFile.toPath(),
        )
    }
}

private fun downloadJExtract(
    platform: String,
    slug: String,
    jextractDirectory: Path,
) {
    val url = jextractUrl(platform, slug)
    // download URL
    // extract stuff
    // fill jextractDirectory with jextract
    jextractDirectory.createDirectories()
    jextractDirectory.resolve("jextract").toFile().writeText("fsdfs")
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
    return "${jextractOs}_$jextractArch"
}