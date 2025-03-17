plugins {
    alias(libs.plugins.ktlint)
    `kotlin-dsl`
    kotlin("jvm") version "2.1.10"
}

repositories {
    mavenCentral()
}

task("lint") {
    dependsOn(tasks.named("ktlintCheck"))
}

task("autofix") {
    dependsOn(tasks.named("ktlintFormat"))
}
