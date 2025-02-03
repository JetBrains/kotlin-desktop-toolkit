package org.jetbrains.kwm.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.model.ObjectFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import org.gradle.process.ExecOperations
import java.nio.file.Path
import javax.inject.Inject
import kotlin.io.path.createDirectories
import kotlin.io.path.writeText

abstract class CompileRustTask @Inject constructor(
    objectFactory: ObjectFactory,
): DefaultTask() {
    @get:InputDirectory
    val nativeDirectory = objectFactory.directoryProperty()

    @get:Input
    val crateName = objectFactory.property<String>()

    @get:Input
    val rustTarget = objectFactory.property<String>().convention(buildPlatformRustTarget())

    @get:Input
    val rustProfile = objectFactory.property<String>()

    @get:OutputFile
    val headerFile = objectFactory.fileProperty()

    @get:OutputDirectory
    val libraryDirectory = objectFactory.directoryProperty()

//    @get:OutputFile
//    val libraryFile = libraryDirectory.map { libDirectory ->
//        val target = rustTarget.get()
//        when {
//            target.contains("linux") -> libDirectory.asFile.resolve("${crateName}.so") // FIXME: verify
//            target.contains("macos") -> libDirectory.asFile.resolve("${crateName}.dylib") // FIXME: verify
//            target.contains("windows") -> libDirectory.asFile.resolve("lib_${crateName}.dll") // FIXME: verify
//            else -> error("unsupported target '$target'")
//        }
//    }

    @TaskAction
    fun compile() {
        compileRust(
            nativeDirectory.get().asFile.toPath(),
            crateName.get(),
            rustTarget.get(),
            rustProfile.get(),
            headerFile.get().asFile.toPath(),
            libraryDirectory.get().asFile.toPath(),
        )
    }
}

private fun compileRust(
    nativeDirectory: Path,
    crateName: String,
    rustTarget: String,
    rustProfile: String,
    headerFile: Path,
    libraryDirectory: Path,
) {
//    ProcessBuilder("cargo", "build")
//        .workingDir(nativeDirectory.toFile())
//        .start().waitFor()
//    exec {
//        workingDir = nativeDirectory.toFile()
//        commandLine("cargo", "build")
//    }

//    myHeaderGEneratedFile.copyTo(headerFile)
    // TODO: remember, you need to copy the output of cargo to the `libraryDirectory` location, and the `headerFile` location

    libraryDirectory.createDirectories()
    headerFile.parent.createDirectories()
    headerFile.writeText("fsdfsfs")
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