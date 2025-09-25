package org.jetbrains.desktop.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.model.ObjectFactory
import org.gradle.api.provider.ProviderFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import org.gradle.process.ExecOperations
import java.io.File
import java.io.OutputStream
import java.nio.file.Path
import javax.inject.Inject
import kotlin.io.path.isDirectory
import kotlin.text.trim

private fun ProviderFactory.isRustUpToDate(rustcCommand: String, workspaceRoot: File, rustVersion: String): Boolean {
    val result = exec {
        workingDir = workspaceRoot
        executable = rustcCommand
        args = listOf("--version")
        isIgnoreExitValue = true
    }
    if (result.result.get().exitValue != 0) {
        return false
    }
    val out = result.standardOutput.asText.get().trim()
    return out.startsWith("rustc $rustVersion (")
}

private fun ProviderFactory.getRustToolchainDir(rustcCommand: String, workspaceRoot: File, rustTarget: String): String {
    val ret = exec {
        workingDir = workspaceRoot
        executable = rustcCommand
        args = listOf("--target", rustTarget, "--print", "target-libdir")
    }
    return ret.standardOutput.asText.get().trim()
}

private fun ExecOperations.isRustToolchainInstalled(rustupCommand: String, workspaceRoot: File): Boolean {
    val showActiveToolchainArgs = listOf("show", "active-toolchain")
    val result = exec {
        workingDir = workspaceRoot
        executable = rustupCommand
        args = showActiveToolchainArgs
        standardOutput = OutputStream.nullOutputStream()
        errorOutput = OutputStream.nullOutputStream()
        isIgnoreExitValue = true
    }
    return result.exitValue == 0
}

abstract class InstallRust @Inject constructor(
    objectFactory: ObjectFactory,
    providerFactory: ProviderFactory,
    private val execOperations: ExecOperations,
) : DefaultTask() {
    @get:Input
    val rustcCommand = objectFactory.property<String>()

    @get:Input
    val rustupCommand = objectFactory.property<String>()

    @get:Input
    val rustTarget = objectFactory.property<String>()

    @get:InputFile
    val rustToolchainFile = objectFactory.fileProperty()

    @get:Internal
    val rustVersion = providerFactory.fileContents(rustToolchainFile).asText.map { it.trim() }

    init {
        outputs.upToDateWhen {
            val rustcCommand = rustcCommand.get()
            val workspaceRoot = rustToolchainFile.get().asFile.parentFile
            val rustVersion = rustVersion.get()
            val rustTarget = rustTarget.get()
            val rustUpToDate = providerFactory.isRustUpToDate(
                rustcCommand = rustcCommand,
                workspaceRoot = workspaceRoot,
                rustVersion = rustVersion,
            )
            if (rustUpToDate) {
                val toolchainDir = providerFactory.getRustToolchainDir(
                    rustcCommand = rustcCommand,
                    workspaceRoot = workspaceRoot,
                    rustTarget = rustTarget,
                )
                Path.of(toolchainDir).isDirectory()
            } else {
                false
            }
        }
    }

    @TaskAction
    fun compile() {
        val workspaceRoot = rustToolchainFile.get().asFile.parentFile
        val rustupCommand = rustupCommand.get()
        val rustTarget = rustTarget.get()
        if (execOperations.isRustToolchainInstalled(rustupCommand = rustupCommand, workspaceRoot = workspaceRoot)) {
            execOperations.exec {
                workingDir = workspaceRoot
                executable = rustupCommand
                args = listOf("target", "add", rustTarget)
            }
        } else {
            execOperations.exec {
                workingDir = workspaceRoot
                executable = rustupCommand
                args = listOf("toolchain", "install")
            }
        }
    }
}
