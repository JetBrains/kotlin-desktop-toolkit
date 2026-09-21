package org.jetbrains.desktop.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.model.ObjectFactory
import org.gradle.api.provider.ProviderFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import org.gradle.process.ExecOperations
import javax.inject.Inject

abstract class InstallCargoProgram @Inject constructor(
    objectFactory: ObjectFactory,
    providerFactory: ProviderFactory,
    private val execOperations: ExecOperations,
) : DefaultTask() {
    private val targetDirectory = temporaryDir

    @get:InputFile
    val rustToolchainFile = objectFactory.fileProperty()

    @get:Input
    val cargoCommand = objectFactory.property<String>()

    @get:Input
    val crate = objectFactory.property<String>()

    @get:Input
    val version = objectFactory.property<String>()

    @get:OutputFile
    val targetBinPath = providerFactory.provider {
        val crate = crate.get()
        val targetBinDir = targetDirectory.resolve("bin")
        targetBinDir.resolve(crate).path
    }

    @TaskAction
    fun compile() {
        val cargoCommand = cargoCommand.get()
        val crate = crate.get()
        val version = version.get()
        val workspaceRoot = rustToolchainFile.get().asFile
        workspaceRoot.copyTo(targetDirectory.resolve(workspaceRoot.name), overwrite = true)

        val cargoArgs = listOf(
            "install",
            crate,
            "--version",
            version,
            "--locked",
            "--color=always",
            "--root=$targetDirectory",
        )
        logger.info("Installing Cargo program '$crate' to '$targetDirectory' using:\n $cargoCommand ${cargoArgs.asCmdArgs()}")

        val targetBinDir = temporaryDir.resolve("bin")
        execOperations.exec {
            workingDir = targetDirectory
            environment["PATH"] = "${environment["PATH"]}:$targetBinDir"
            executable = cargoCommand
            args = cargoArgs
        }
    }
}
