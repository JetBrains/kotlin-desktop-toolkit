package org.jetbrains.desktop.buildscripts

import org.gradle.api.DefaultTask
import org.gradle.api.file.FileSystemOperations
import org.gradle.api.model.ObjectFactory
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import javax.inject.Inject

abstract class CollecNativeArtifactsTask @Inject constructor(
    objectFactory: ObjectFactory,
    private val fs: FileSystemOperations,
) : DefaultTask() {
    @get:InputFiles
    val angleBinaries = objectFactory.fileCollection()

    @get:InputFile
    val nativeLibrary = objectFactory.fileProperty()

    @get:OutputDirectory
    val targetDirectory = objectFactory.directoryProperty()

    @TaskAction
    fun copyBinaries() {
        fs.copy {
            from(angleBinaries.files)
            into(targetDirectory)
        }
        fs.copy {
            from(nativeLibrary)
            into(targetDirectory)
        }
    }
}
