package org.jetbrains.kwm.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.model.ObjectFactory
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import java.nio.file.Path
import javax.inject.Inject
import kotlin.io.path.createDirectories

abstract class GenerateJavaBindingsTask @Inject constructor(
    objectFactory: ObjectFactory,
) : DefaultTask() {
    @get:InputFile
    val jextractBinary = objectFactory.fileProperty()

    @get:InputFile
    val headerFile = objectFactory.fileProperty()

    @get:OutputDirectory
    val generatedSourcesDirectory = objectFactory.directoryProperty()

    @TaskAction
    fun generate() {
        generateJavaBindings(
            jextractBinary.get().asFile.toPath(),
            headerFile.get().asFile.toPath(),
            generatedSourcesDirectory.get().asFile.toPath()
        )
    }
}

private fun generateJavaBindings(
    jextractBinary: Path,
    headerFile: Path,
    generatedSourcesDirectory: Path,
) {
    generatedSourcesDirectory.createDirectories()
 // fill up `generatedSourcesDirectory` with the classes
}
