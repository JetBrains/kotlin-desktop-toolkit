plugins {
    kotlin("jvm")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jlleitschuh.gradle.ktlint")
}

group = "org.jetbrains.kotlin-desktop-toolkit"
version = (project.findProperty("version") as? String)?.takeIf { it.isNotBlank() && it != "unspecified" } ?: "SNAPSHOT"

repositories {
    google()
    mavenCentral()
    maven("https://maven.pkg.jetbrains.space/public/p/compose/dev")
}

dependencies {
    api(project(":kotlin-desktop-toolkit"))
    api(libs.compose.runtime)
    api(libs.compose.ui)
    implementation(kotlin("stdlib"))
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(25)
    }
    withSourcesJar()
}

tasks.register("lint") {
    dependsOn(tasks.named("ktlintCheck"))
}

tasks.register("autofix") {
    dependsOn(tasks.named("ktlintFormat"))
}
