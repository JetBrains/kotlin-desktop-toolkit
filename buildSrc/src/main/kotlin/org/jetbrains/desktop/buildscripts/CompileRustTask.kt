package org.jetbrains.desktop.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.file.DuplicatesStrategy
import org.gradle.api.file.FileSystemOperations
import org.gradle.api.file.FileTree
import org.gradle.api.file.ProjectLayout
import org.gradle.api.model.ObjectFactory
import org.gradle.api.provider.ProviderFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import org.gradle.process.ExecOperations
import java.nio.file.Path
import javax.inject.Inject

abstract class CompileRustTask @Inject constructor(
    objectFactory: ObjectFactory,
    providerFactory: ProviderFactory,
    projectLayout: ProjectLayout,
    private val execOperations: ExecOperations,
    private val fileSystemOperations: FileSystemOperations,
) : DefaultTask() {
    @get:Input
    val workspaceRoot = objectFactory.property<String>()

    @get:InputFiles
    @get:PathSensitive(org.gradle.api.tasks.PathSensitivity.RELATIVE)
    val workspaceFiles = objectFactory.property<FileTree>()

    @get:Input
    val cargoCommand = objectFactory.property<String>()

    @get:Input
    val crateName = objectFactory.property<String>()

    @get:Input
    val targetPlatform = objectFactory.property<Platform>()

    @get:Input
    val rustTarget = objectFactory.property<String>()

    @get:Input
    val rustProfile = objectFactory.property<String>()

    @Internal
    val outputDirectory =
        projectLayout.buildDirectory.dir(
            rustTarget.map { rustTarget ->
                inCrateArtifactsPath(rustTarget, rustProfile.get())
            },
        )

    @Internal
    val rustOutputLibraryFile = providerFactory.provider {
        val dir = Path.of(workspaceRoot.get()).resolve(inCrateArtifactsPath(rustTarget.get(), rustProfile.get()))
        val targetPlatform = targetPlatform.get()
        val name = crateName.get().replace('-', '_')
        when (targetPlatform.os) {
            Os.LINUX -> dir.resolve("lib$name.so")
            Os.MACOS -> dir.resolve("lib$name.dylib")
            Os.WINDOWS -> dir.resolve("$name.dll")
        }
    }

    @get:OutputFile
    val libraryFile = providerFactory.provider {
        val dir = outputDirectory.get().asFile
        val targetPlatform = targetPlatform.get()
        val rustProfile = rustProfile.get()

        val targetSuffix = when (targetPlatform.arch) {
            Arch.aarch64 -> "arm64"
            Arch.x86_64 -> "x64"
        }

        val debugSuffix = if (rustProfile == "debug" || rustProfile == "dev") "+debug" else ""

        val crateName = crateName.get().replace('-', '_')
        val libName = "${crateName}_${targetSuffix}$debugSuffix"

        /**
         * See `KotlinDesktopToolkit.kt` if you would like to change this logic.
         */
        // todo macOS change libname with otool
        when (targetPlatform.os) {
            Os.LINUX -> dir.resolve("lib$libName.so")
            Os.MACOS -> dir.resolve("lib$libName.dylib")
            Os.WINDOWS -> dir.resolve("$libName.dll")
        }
    }

    @TaskAction
    fun compile() {
        execOperations.compileRust(
            cargoCommand = cargoCommand.get(),
            nativeDirectory = Path.of(workspaceRoot.get()),
            crateName = crateName.get(),
            rustTarget = rustTarget.get(),
            targetPlatform = targetPlatform.get(),
            rustProfile = rustProfile.get(),
        )

        val libraryFile = libraryFile.get()
        fileSystemOperations.copy {
            from(rustOutputLibraryFile)
            into(libraryFile.parent)
            rename { libraryFile.name }
            duplicatesStrategy = DuplicatesStrategy.INCLUDE
        }
    }
}

internal fun profileFolderName(rustProfile: String) = when (rustProfile) {
    "dev" -> "debug"
    else -> rustProfile
}

internal fun inCrateArtifactsPath(rustTarget: String, rustProfile: String): String {
    return "target/$rustTarget/${profileFolderName(rustProfile)}/"
}

private fun ExecOperations.compileRust(
    cargoCommand: String,
    nativeDirectory: Path,
    crateName: String,
    rustTarget: String,
    targetPlatform: Platform,
    rustProfile: String,
) {
    exec {
        workingDir = nativeDirectory.toFile()
        if (targetPlatform.os == Os.MACOS) {
            environment("MACOSX_DEPLOYMENT_TARGET", "10.12")
        }
        executable = cargoCommand
        args = listOf(
            "build",
            "--package=$crateName",
            "--profile=$rustProfile",
            "--target=$rustTarget",
            "--color=always",
        )
    }
}
