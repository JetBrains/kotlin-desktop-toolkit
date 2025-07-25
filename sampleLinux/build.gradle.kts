import org.jetbrains.desktop.buildscripts.Arch
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitAttributes
import org.jetbrains.desktop.buildscripts.KotlingDesktopToolkitArtifactType
import org.jetbrains.desktop.buildscripts.KotlingDesktopToolkitNativeProfile
import org.jetbrains.desktop.buildscripts.hostArch
import org.jetbrains.desktop.buildscripts.targetArch

plugins {
    // Apply the org.jetbrains.kotlin.jvm Plugin to add support for Kotlin.
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.ktlint)

    // Apply the application plugin to add support for building a CLI application in Java.
    application
}

repositories {
    // Use Maven Central for resolving dependencies.
    mavenCentral()
    maven("https://maven.pkg.jetbrains.space/public/p/compose/dev")
}

val skikoTargetOs = "linux"

val skikoTargetArch = when (targetArch(project) ?: hostArch()) {
    Arch.aarch64 -> "arm64"
    Arch.x86_64 -> "x64"
}

val skikoVersion = "0.9.17"
val skikoTarget = "$skikoTargetOs-$skikoTargetArch"
dependencies {
    // Use the Kotlin JUnit 5 integration.
    testImplementation("org.jetbrains.kotlin:kotlin-test-junit5")

    // Use the JUnit 5 integration.
    testImplementation(libs.junit.jupiter.engine)

    testRuntimeOnly("org.junit.platform:junit-platform-launcher")

    // This dependency is used by the application.
    implementation(libs.guava)
    implementation(project(":kotlin-desktop-toolkit-linux"))
    implementation("org.jetbrains.skiko:skiko-awt-runtime-$skikoTarget:$skikoVersion")
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
    mainClass = "org.jetbrains.desktop.sample.ApplicationSampleKt"
    applicationDefaultJvmArgs = listOf(
        "--enable-preview",
        "--enable-native-access=ALL-UNNAMED",
        "-Djextract.trace.downcalls=false",
    )
}

val depScope = configurations.dependencyScope("linuxNative") {
    withDependencies {
        add(project.dependencies.project(":kotlin-desktop-toolkit-linux"))
    }
}
val nativeLib = configurations.resolvable("linuxNativeParts") {
    extendsFrom(depScope.get())
    attributes {
        attribute(KotlinDesktopToolkitAttributes.TYPE, KotlingDesktopToolkitArtifactType.NATIVE_LIBRARY)
        attribute(KotlinDesktopToolkitAttributes.PROFILE, KotlingDesktopToolkitNativeProfile.DEBUG)
    }
}

fun JavaExec.setUpLoggingAndLibraryPath() {
    val logFilePath = layout.buildDirectory.file("sample-logs/skiko_sample.log").map { it.asFile.absolutePath }
    val nativeLibPath = nativeLib.map { it.singleFile.parentFile.absolutePath }
    jvmArgumentProviders.add(
        CommandLineArgumentProvider {
            listOf(
                "-Dkdt.library.folder.path=${nativeLibPath.get()}",
                "-Dkdt.debug=true",
                "-Dkdt.native.log.path=${logFilePath.get()}",
            )
        },
    )
}

tasks.named<JavaExec>("run") {
    jvmArgs("--enable-preview")
    setUpLoggingAndLibraryPath()
}

tasks.register<JavaExec>("runSkikoSampleLinux") {
    group = "application"
    description = "Runs example of integration with Skiko"
    classpath = sourceSets["main"].runtimeClasspath
    mainClass.set("org.jetbrains.desktop.sample.linux.SkikoSampleLinuxKt")
    javaLauncher.set(
        javaToolchains.launcherFor {
            languageVersion.set(JavaLanguageVersion.of(21))
        },
    )
    jvmArgs = listOf(
        "--enable-preview",
        "--enable-native-access=ALL-UNNAMED",
        "-Djextract.trace.downcalls=false",
    )
    setUpLoggingAndLibraryPath()
}

tasks.named<Test>("test") {
    // Use JUnit Platform for unit tests.
    useJUnitPlatform()
}

task("lint") {
    dependsOn(tasks.named("ktlintCheck"))
}

task("autofix") {
    dependsOn(tasks.named("ktlintFormat"))
}
