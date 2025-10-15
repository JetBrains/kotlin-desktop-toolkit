package org.jetbrains.desktop.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.file.FileTree
import org.gradle.api.model.ObjectFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import org.gradle.process.ExecOperations
import java.nio.file.Path
import javax.inject.Inject
import kotlin.io.path.ExperimentalPathApi
import kotlin.io.path.absolutePathString
import kotlin.io.path.createDirectories
import kotlin.io.path.createTempFile
import kotlin.io.path.deleteIfExists
import kotlin.io.path.deleteRecursively
import kotlin.io.path.name
import kotlin.io.path.pathString
import kotlin.io.path.readLines
import kotlin.io.path.writeLines

abstract class GenerateJavaBindingsTask @Inject constructor(
    objectFactory: ObjectFactory,
    private val execOperations: ExecOperations,
) : DefaultTask() {
    @get:Input
    val cbindgenBinary = objectFactory.property<String>()

    @get:InputFile
    val jextractBinary = objectFactory.fileProperty()

    @get:Input
    val packageName = objectFactory.property<String>()

    @get:InputFiles
    @get:PathSensitive(org.gradle.api.tasks.PathSensitivity.RELATIVE)
    val workspaceFiles = objectFactory.property<FileTree>()

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
        val headerFile = execOperations.generateOsHeader(
            cbindgenBinary = cbindgenBinary.get(),
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
private fun ExecOperations.generateOsHeader(cbindgenBinary: String, crateDirectory: Path, headerDirectory: Path): Path {
    val headerFile = headerDirectory.resolve("${crateDirectory.name.replace("-", "_")}.h")
    headerDirectory.deleteRecursively()
    headerDirectory.createDirectories()
    val args = buildList {
        add(crateDirectory.absolutePathString())
        add("--output=${headerFile.absolutePathString()}")
    }.toTypedArray()
    exec {
        workingDir = crateDirectory.toFile()
        commandLine(cbindgenBinary, *args)
    }
    return headerFile
}

private fun ExecOperations.listHeaderSymbols(jextractBinary: Path, headerFile: Path): Path {
    val symbolsFile = createTempFile("headerSymbols.txt")
    val filteredSymbolsFile = createTempFile("filteredSymbols.txt")
    return try {
        val args = buildList {
            add("--dump-includes")
            add(symbolsFile.pathString)
            add(headerFile.pathString)
        }.toTypedArray()
        exec {
            commandLine(jextractBinary.pathString, *args)
        }
        val filteredSymbols = symbolsFile.readLines()
            .filter { it.endsWith(headerFile.name) && it.startsWith("--") }
            .map { it.split("\\s+".toRegex()).take(2).joinToString(separator = " ") }

        filteredSymbolsFile.writeLines(filteredSymbols)
        filteredSymbolsFile
    } finally {
        symbolsFile.deleteIfExists()
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
    try {
        val args = buildList {
            add("--target-package")
            add(packageName)
            add("--output")
            add(generatedSourcesDirectory.pathString)
            add("@${filteredSymbols.absolutePathString()}")
            add(headerFile.pathString)
        }.toTypedArray()

        exec {
            commandLine(jextractBinary.pathString, *args)
        }
    } finally {
        filteredSymbols.deleteIfExists()
    }
}
