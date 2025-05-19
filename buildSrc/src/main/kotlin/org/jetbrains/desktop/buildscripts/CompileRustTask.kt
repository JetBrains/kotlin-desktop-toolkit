package org.jetbrains.desktop.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.file.Directory
import org.gradle.api.file.FileTree
import org.gradle.api.file.ProjectLayout
import org.gradle.api.file.RegularFile
import org.gradle.api.model.ObjectFactory
import org.gradle.api.provider.Provider
import org.gradle.api.provider.ProviderFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.Nested
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import org.gradle.process.ExecOperations
import java.io.ByteArrayOutputStream
import java.nio.file.Path
import javax.inject.Inject
import kotlin.io.path.absolutePathString
import kotlin.io.path.copyTo

abstract class CompileRustTask @Inject constructor(
    objectFactory: ObjectFactory,
    providerFactory: ProviderFactory,
    projectLayout: ProjectLayout,
    private val execOperations: ExecOperations,
) : DefaultTask() {
    @Internal
    val workspaceRoot = objectFactory.directoryProperty()

    @Suppress("unused")
    @get:InputFiles
    @get:PathSensitive(org.gradle.api.tasks.PathSensitivity.RELATIVE)
    val workspaceFiles = objectFactory.rustWorkspaceFiles(workspaceRoot)

    @get:Input
    val crateName = objectFactory.property<String>()

    @get:Nested
    val rustTarget = objectFactory.property<Platform>()

    @get:Input
    val rustProfile = objectFactory.property<String>()

    @Internal
    val outputDirectory =
        projectLayout.buildDirectory.dir(providerFactory.provider {
            inCrateArtifactsPath(rustTarget.get(), rustProfile.get())
        })

    @Internal
    val rustOutputLibraryFile = providerFactory.provider {
        val dir = workspaceRoot.get().asFile.resolve(inCrateArtifactsPath(rustTarget.get(), rustProfile.get()))
        val target = rustTarget.get()
        val name = crateName.get().replace('-', '_')
        when (target.os) {
            Os.LINUX -> dir.resolve("lib$name.so")
            Os.MACOS -> dir.resolve("lib$name.dylib")
            Os.WINDOWS -> dir.resolve("$name.dll")
        }
    }

    @get:OutputFile
    val headerFile = outputDirectory.map { outDir -> outDir.file("headers/${crateName.get().replace("-", "_")}.h") }

    @get:OutputFile
    val libraryFile = providerFactory.provider {
        val dir = outputDirectory.get().asFile
        val target = rustTarget.get()
        val rustProfile = rustProfile.get()

        val targetSuffix = when (target.arch) {
            Arch.aarch64 -> "arm64"
            Arch.x86_64 -> "x64"
        }

        val debugSuffix = if (rustProfile == "debug" || rustProfile == "dev") "+debug" else ""

        val crateName = crateName.get().replace('-', '_')
        val libName = "${crateName}_${targetSuffix}${debugSuffix}"


        /**
        * See `KotlinDesktopToolkit.kt` if you would like to change this logic.
        */
        // todo change libname with otool
        when (target.os) {
            Os.LINUX -> dir.resolve("lib$libName.so")
            Os.MACOS -> dir.resolve("lib$libName.dylib")
            Os.WINDOWS -> dir.resolve("$libName.dll")
        }
    }

    @TaskAction
    fun compile() {
        execOperations.compileRust(
            workspaceRoot.get().asFile.toPath(),
            crateName.get(),
            rustTarget.get(),
            rustProfile.get(),
            headerFile.get().asFile.toPath(),
            rustOutputLibraryFile.get().toPath(),
            libraryFile.get().toPath(),
        )
    }
}

internal fun profileFolderName(rustProfile: String) = when (rustProfile) {
    "dev" -> "debug"
    else -> rustProfile
}

internal fun inCrateArtifactsPath(rustTarget: Platform, rustProfile: String): String {
    return "target/${buildPlatformRustTarget(rustTarget)}/${profileFolderName(rustProfile)}/"
}

/**
 * Finds the absolute path to [command]
 */
internal fun ExecOperations.findCommand(command: String, os: Os): Path? {
    val output = ByteArrayOutputStream()
    val result = exec {
        val cmd = when (os) {
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
        else -> Path.of(out)
    }
}

private fun ExecOperations.compileRust(
    nativeDirectory: Path,
    crateName: String,
    rustTarget: Platform,
    rustProfile: String,
    headerFile: Path,
    rustOutputLibraryFile: Path,
    libraryFile: Path,
) {
    exec {
        workingDir = nativeDirectory.toFile()
        if (rustTarget.os == Os.MACOS) {
            environment("MACOSX_DEPLOYMENT_TARGET", "10.12")
        }
        executable = findCommand("cargo", rustTarget.os)?.absolutePathString() ?: error("cannot find cargo path")
        args = listOf(
            "build",
            "--package=$crateName",
            "--profile=$rustProfile",
            "--target=${buildPlatformRustTarget(rustTarget)}",
            "--color=always",
        )
    }

    nativeDirectory
        .resolve(crateName)
        .resolve("headers")
        .resolve(headerFile.fileName).copyTo(headerFile, overwrite = true)

    nativeDirectory
        .resolve(rustOutputLibraryFile)
        .copyTo(libraryFile, overwrite = true)
}

fun buildPlatformRustTarget(platform: Platform): String {
    val osPart = when (platform.os) {
        Os.WINDOWS -> "pc-windows-msvc"
        Os.MACOS -> "apple-darwin"
        Os.LINUX -> "unknown-linux-gnu"
    }
    val archPart = when (platform.arch) {
        Arch.aarch64 -> "aarch64"
        Arch.x86_64 -> "x86_64"
    }
    return "$archPart-$osPart"
}

/**
 * All workspace files under [workspaceRoot] excluding compilation outputs and caches
 */
internal fun ObjectFactory.rustWorkspaceFiles(workspaceRoot: Provider<Directory>): FileTree = fileTree().apply {
  setDir(workspaceRoot)
}.matching {
  exclude("target/**/*")
}


fun getWorkspaceHeaderFile(dir: Directory, crateName: String): RegularFile {
    return dir.dir(crateName).dir("headers").file("${crateName.replace("-", "_")}.h")
}
