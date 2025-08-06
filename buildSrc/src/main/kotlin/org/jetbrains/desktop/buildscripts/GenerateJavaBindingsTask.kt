package org.jetbrains.desktop.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.model.ObjectFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import org.gradle.process.ExecOperations
import org.slf4j.Logger
import java.nio.file.Path
import javax.inject.Inject
import kotlin.io.path.ExperimentalPathApi
import kotlin.io.path.absolutePathString
import kotlin.io.path.createDirectories
import kotlin.io.path.createTempFile
import kotlin.io.path.deleteIfExists
import kotlin.io.path.deleteRecursively
import kotlin.io.path.exists
import kotlin.io.path.name
import kotlin.io.path.pathString

abstract class GenerateJavaBindingsTask @Inject constructor(
    objectFactory: ObjectFactory,
    private val execOperations: ExecOperations,
) : DefaultTask() {
    @get:InputFile
    val jextractBinary = objectFactory.fileProperty()

    @get:Input
    val packageName = objectFactory.property<String>()

    @Internal
    val workspaceRoot = objectFactory.directoryProperty()

    @Suppress("unused")
    @get:InputFiles
    @get:PathSensitive(org.gradle.api.tasks.PathSensitivity.RELATIVE)
    val workspaceFiles = objectFactory.rustWorkspaceFiles(workspaceRoot)

    @get:InputDirectory
    val crateDirectory = objectFactory.directoryProperty()

    @get:OutputDirectory
    val generatedSourcesDirectory = objectFactory.directoryProperty()

    @TaskAction
    fun generate() {
        outputs.previousOutputFiles.forEach {
            if (it.isFile) {
                it.delete()
            }
        }

        val crateDir = crateDirectory.get().asFile.toPath()
        val cbindgenBinary = execOperations.installCargoProgram(
            moduleDirectory = crateDir,
            crate = "cbindgen",
            version = "0.29.0",
            targetDirectory = temporaryDir.resolve("cargoInstallation").toPath(),
            logger = logger,
        )
        val headerFile = execOperations.generateOsHeader(
            cbindgenBinary = cbindgenBinary,
            crateDirectory = crateDir,
            headerDirectory = temporaryDir.resolve("headers").toPath(),
        )

        execOperations.generateJavaBindings(
            jextractBinary.get().asFile.toPath(),
            headerFile,
            packageName.get(),
            generatedSourcesDirectory.get().asFile.toPath(),
        )
    }
}

@OptIn(ExperimentalPathApi::class)
private fun ExecOperations.installCargoProgram(moduleDirectory: Path, crate: String, version: String, targetDirectory: Path, logger: Logger): Path {
    val targetBinDir = targetDirectory.resolve("bin")
    val targetBinPath = targetBinDir.resolve(crate)
    if (!targetBinPath.exists()) {
        targetDirectory.createDirectories()
        val cmd = listOf(
            findCommand("cargo", hostOs())?.absolutePathString() ?: error("cannot find cargo path"),
            "install",
            crate,
            "--version",
            version,
            "--locked",
            "--color=always",
            "--root=${targetDirectory.absolutePathString()}"
        )
        logger.info("Installing Cargo program '$crate' in module '$moduleDirectory' using:\n  ${cmd.joinToString(" ")}")

        exec {
            workingDir = moduleDirectory.toFile()
            environment["PATH"] = "${environment["PATH"]}:$targetBinDir"
            commandLine(*cmd.toTypedArray())
        }
    }

    return targetBinPath
}

@OptIn(ExperimentalPathApi::class)
private fun ExecOperations.generateOsHeader(cbindgenBinary: Path, crateDirectory: Path, headerDirectory: Path): Path {
    val headerFile = headerDirectory.resolve("${crateDirectory.name.replace("-", "_")}.h")
    headerDirectory.deleteRecursively()
    headerDirectory.createDirectories()
    val args = buildList {
        add(crateDirectory.absolutePathString())
        add("--output=${headerFile.absolutePathString()}")
    }.toTypedArray()
    exec {
        workingDir = crateDirectory.toFile()
        commandLine(cbindgenBinary.pathString, *args)
    }
    return headerFile
}

private fun ExecOperations.listHeaderSymbols(jextractBinary: Path, headerFile: Path): List<List<String>> {
    val symbols = createTempFile("headerSymbols.txt")
    try {
        val args = buildList {
            add("--dump-includes")
            add(symbols.pathString)
            add(headerFile.pathString)
        }.toTypedArray()
        exec {
            commandLine(jextractBinary.pathString, *args)
        }

        return symbols.toFile().readLines()
            .filter { it.endsWith(headerFile.name) && it.startsWith("--") }
            .map { it.split("\\s+".toRegex()).take(2) }
    } finally {
        symbols.deleteIfExists()
    }
}

private fun ExecOperations.generateJavaBindings(
    jextractBinary: Path,
    headerFile: Path,
    packageName: String,
    generatedSourcesDirectory: Path,
) {
    generatedSourcesDirectory.createDirectories()
    val filteredSymbols = listHeaderSymbols(jextractBinary, headerFile)
    val args = buildList {
        add("--target-package")
        add(packageName)
        add("--output")
        add(generatedSourcesDirectory.pathString)
        filteredSymbols.map {
            addAll(it)
        }
        add(headerFile.pathString)
    }.toTypedArray()

    exec {
        commandLine(jextractBinary.pathString, *args)
    }
}
