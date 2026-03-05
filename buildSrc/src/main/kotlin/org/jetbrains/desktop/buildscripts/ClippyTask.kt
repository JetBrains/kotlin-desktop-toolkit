package org.jetbrains.desktop.buildscripts

import org.gradle.api.tasks.Exec
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.Optional
import org.gradle.kotlin.dsl.listProperty
import org.gradle.kotlin.dsl.property

abstract class ClippyTask : Exec() {
    @get:Input
    @Optional
    val checkOnly = objectFactory.property<Boolean>()

    @get:Input
    val cargoCommand = objectFactory.property<String>()

    @get:Input
    val rustTarget = objectFactory.property<String>()

    @get:Input
    val additionalArgs = objectFactory.listProperty<String>()

    init {
        outputs.upToDateWhen { false }
    }

    override fun exec() {
        val rustTarget = rustTarget.get()
        val additionalArgs = additionalArgs.get();
        executable(cargoCommand.get())
        args(
            buildList {
                add("clippy")
                add("--target=$rustTarget") // required for the cross-compilation support
                add("--all-targets") // also check non-lib targets (e.g. tests)
                addAll(additionalArgs)
                if (checkOnly.getOrElse(false)) {
                    add("--")
                    add("--deny")
                    add("warnings")
                } else {
                    add("--fix")
                    add("--allow-dirty")
                    add("--allow-staged")
                }
            },
        )
        super.exec()
    }
}
