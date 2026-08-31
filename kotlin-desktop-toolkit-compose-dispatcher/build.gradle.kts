import org.gradle.kotlin.dsl.kotlin
import org.gradle.kotlin.dsl.`maven-publish`

plugins {
    kotlin("jvm")
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
    // `KdtDispatcher` is what this module hands to kotlinx.coroutines, so it is part of its API.
    api(project(":kotlin-desktop-toolkit-compose"))
    // `MainDispatcherFactory` lives in coroutines' internal package: declared directly rather than taken
    // transitively, since an implementation against an internal API should name the version it was built on.
    implementation(libs.kotlinx.coroutines.core)
    implementation(kotlin("stdlib"))

    testImplementation("org.jetbrains.kotlin:kotlin-test-junit5")
    testImplementation(libs.junit.jupiter.engine)
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(25)
    }
    withSourcesJar()
}

tasks.test {
    useJUnitPlatform()
}

tasks.register("lint") {
    dependsOn(tasks.named("ktlintCheck"))
}

tasks.register("autofix") {
    dependsOn(tasks.named("ktlintFormat"))
}
