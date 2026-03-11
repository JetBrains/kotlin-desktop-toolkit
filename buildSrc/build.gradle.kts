plugins {
    `kotlin-dsl`
    id("org.jlleitschuh.gradle.ktlint")
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
