package org.jetbrains.desktop.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.file.ProjectLayout
import org.gradle.api.model.ObjectFactory
import org.gradle.api.provider.ProviderFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import org.gradle.process.ExecOperations
import java.io.ByteArrayOutputStream
import java.nio.file.Path
import javax.inject.Inject
import kotlin.io.path.copyTo
import kotlin.io.path.nameWithoutExtension

abstract class CompileRustTask @Inject constructor(
    objectFactory: ObjectFactory,
    providerFactory: ProviderFactory,
    projectLayout: ProjectLayout,
    private val execOperations: ExecOperations,
): DefaultTask() {
    @get:InputDirectory
    val nativeDirectory = objectFactory.directoryProperty()

    @get:Input
    val crateName = objectFactory.property<String>()

    @get:Input
    val rustTarget = objectFactory.property<String>().convention(buildPlatformRustTarget())

    @get:Input
    val rustProfile = objectFactory.property<String>()

    @Internal
    val outputDirectory =
        projectLayout.buildDirectory.dir(providerFactory.provider { "target/${rustTarget.get()}/${rustProfile.get()}" })

    @get:OutputFile
    val headerFile = outputDirectory.map { outDir -> outDir.file("headers/${crateName.get().replace("-", "_")}.h") }

    @get:OutputDirectory
    val libraryDirectory = outputDirectory.map { outDir -> outDir.dir("deps") }

    @get:OutputFile
    val libraryFile = providerFactory.provider {
        val dir = libraryDirectory.get().asFile
        val target = rustTarget.get()
        val name = headerFile.get().asFile.toPath().nameWithoutExtension
        when {
            target.contains("apple") -> dir.resolve("lib$name.dylib")
            target.contains("linux") -> dir.resolve("$name.so") // FIXME: verify
            target.contains("windows") -> dir.resolve("lib_$name.dll") // FIXME: verify
            else -> error("unsupported target '$target'")
        }
    }

    @TaskAction
    fun compile() {
        execOperations.compileRust(
            nativeDirectory.get().asFile.toPath(),
            crateName.get(),
            rustTarget.get(),
            rustProfile.get(),
            headerFile.get().asFile.toPath(),
            libraryFile.get().toPath(),
        )
    }
}

/**
 * Finds the absolute path to [command]
 */
internal fun ExecOperations.findCommand(command: String): Path? {
    val output = ByteArrayOutputStream()
    val result = exec {
        val cmd = when (buildOs()) {
            Os.MACOS, Os.LINUX -> listOf("/bin/sh", "-c", "command -v $command")
            Os.WINDOWS -> listOf("cmd.exe", "/c", "where", command)
        }

        commandLine(*cmd.toTypedArray())
        standardOutput = output
        isIgnoreExitValue = true
    }
    val out = output.toString().trim().takeIf { it.isNotBlank() }
    return when {
        result.exitValue != 0 -> null
        out == null -> error("failed to resolve absolute path of command '$command'")
        else ->  Path.of(out)
    }
}


private fun ExecOperations.compileRust(
    nativeDirectory: Path,
    crateName: String,
    rustTarget: String,
    rustProfile: String,
    headerFile: Path,
    libraryFile: Path,
) {
    exec {
        workingDir = nativeDirectory.toFile()
        commandLine(findCommand("cargo"), "build",
            "--package=$crateName",
            "--profile=$rustProfile",
            "--target=$rustTarget",
            "--color=always")
    }

    nativeDirectory
        .resolve(crateName)
        .resolve("headers")
        .resolve(headerFile.fileName).copyTo(headerFile, overwrite = true)

    val folderName = when (rustProfile) {
        "dev" -> "debug"
        else -> rustProfile
    }

    nativeDirectory
        .resolve("target")
        .resolve(rustTarget)
        .resolve(folderName)
        .resolve(libraryFile.fileName)
        .copyTo(libraryFile, overwrite = true)
}

private fun buildPlatformRustTarget(): String {
    val osPart = when (buildOs()) {
        Os.WINDOWS -> "windows-msvc"
        Os.MACOS -> "apple-darwin"
        Os.LINUX -> "unknown-linux-gnu"
    }
    val archPart = when (buildArch()) {
        Arch.aarch64 -> "aarch64"
        Arch.x86_64 -> "x86_64"
    }
    return "$archPart-$osPart"
}