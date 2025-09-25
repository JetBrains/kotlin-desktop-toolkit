package org.jetbrains.desktop.buildscripts

import org.gradle.api.tasks.Exec
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.Optional
import org.gradle.kotlin.dsl.property

abstract class CargoFmtTask : Exec() {
    @get:Input
    @Optional
    val checkOnly = objectFactory.property<Boolean>()

    @get:Input
    val cargoCommand = objectFactory.property<String>()

    init {
        outputs.upToDateWhen { false }
    }

    override fun exec() {
        executable(cargoCommand.get())
        args(
            buildList {
                add("fmt")
                if (checkOnly.getOrElse(false)) {
                    add("--check")
                }
            },
        )
        super.exec()
    }
}
