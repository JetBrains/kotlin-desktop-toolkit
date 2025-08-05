plugins {
    alias(libs.plugins.ktlint)
    `kotlin-dsl`
    kotlin("jvm") version "2.1.10"
}

repositories {
    mavenCentral()
}

tasks.register("lint") {
    dependsOn(tasks.named("ktlintCheck"))
}

tasks.register("autofix") {
    dependsOn(tasks.named("ktlintFormat"))
}
