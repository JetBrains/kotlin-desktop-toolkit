package org.jetbrains.desktop.buildscripts

import org.gradle.api.tasks.Exec
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.Nested
import org.gradle.api.tasks.Optional
import org.gradle.kotlin.dsl.property

abstract class ClippyTask: Exec() {
    @get:Input
    @Optional
    val checkOnly = objectFactory.property<Boolean>()

    @get:Input
    val crateName = objectFactory.property<String>()

    @get:Nested
    val targetPlatform = objectFactory.property<Platform>()

    init {
        outputs.upToDateWhen { false }
        executable("cargo")
    }

    override fun exec() {
        val rustTarget = buildPlatformRustTarget(targetPlatform.get())
        val crateName = crateName.get()
        args(buildList {
            add("clippy")
            add("--target=$rustTarget")
            add("--package=$crateName")
            add("--all-targets")
            add("--all-features")
            if (checkOnly.getOrElse(false)) {
                add("--")
                add("--deny")
                add("warnings")
            } else {
                add("--fix")
                add("--allow-dirty")
                add("--allow-staged")
            }
        })
        super.exec()
    }
}
