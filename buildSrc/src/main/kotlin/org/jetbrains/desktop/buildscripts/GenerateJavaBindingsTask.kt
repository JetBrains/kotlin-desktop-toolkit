package org.jetbrains.desktop.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.model.ObjectFactory
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import org.gradle.kotlin.dsl.property
import org.gradle.process.ExecOperations
import java.nio.file.Path
import java.nio.file.Paths
import javax.inject.Inject
import kotlin.io.path.createDirectories
import kotlin.io.path.createTempFile
import kotlin.io.path.deleteIfExists
import kotlin.io.path.name
import kotlin.io.path.pathString

abstract class GenerateJavaBindingsTask @Inject constructor(
    objectFactory: ObjectFactory,
    private val execOperations: ExecOperations
) : DefaultTask() {
    @get:InputFile
    val jextractBinary = objectFactory.fileProperty()

    @get:InputFile
    val headerFile = objectFactory.fileProperty()

    @get:Input
    val packageName = objectFactory.property<String>()

    @get:OutputDirectory
    val generatedSourcesDirectory = objectFactory.directoryProperty()

    @TaskAction
    fun generate() {
        execOperations.generateJavaBindings(
            jextractBinary.get().asFile.toPath(),
            headerFile.get().asFile.toPath(),
            packageName.get(),
            generatedSourcesDirectory.get().asFile.toPath()
        )
    }
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
