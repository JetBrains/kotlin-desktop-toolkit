plugins {
    // Apply the org.jetbrains.kotlin.jvm Plugin to add support for Kotlin.
    alias(libs.plugins.kotlin.jvm)

    // Apply the application plugin to add support for building a CLI application in Java.
    application
}

repositories {
    // Use Maven Central for resolving dependencies.
    mavenCentral()
}

dependencies {
    // Use the Kotlin JUnit 5 integration.
    testImplementation("org.jetbrains.kotlin:kotlin-test-junit5")

    // Use the JUnit 5 integration.
    testImplementation(libs.junit.jupiter.engine)

    testRuntimeOnly("org.junit.platform:junit-platform-launcher")

    // This dependency is used by the application.
    implementation(libs.guava)
    implementation(project(":lib"))
}

// Apply a specific Java toolchain to ease working on different environments.
java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(21)
    }
}

tasks.compileJava {
    options.compilerArgs = listOf("--enable-preview")
}

application {
    // Define the main class for the application.
    mainClass = "org.jetbrains.kwm.sample.AppKt"
    applicationDefaultJvmArgs = listOf("--enable-preview",
                                       "--enable-native-access=ALL-UNNAMED",
                                       "-Djextract.trace.downcalls=false")
}

tasks.named<JavaExec>("run") {
    environment("DYLD_LIBRARY_PATH", "/Users/pavel/work/KWM/native/target/debug")
}

tasks.named<Test>("test") {
    // Use JUnit Platform for unit tests.
    useJUnitPlatform()
}
