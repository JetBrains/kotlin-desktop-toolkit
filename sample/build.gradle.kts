import org.jetbrains.desktop.buildscripts.Arch
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitAttributes
import org.jetbrains.desktop.buildscripts.KotlingDesktopToolkitArtifactType
import org.jetbrains.desktop.buildscripts.KotlingDesktopToolkitNativeProfile
import org.jetbrains.desktop.buildscripts.Os
import org.jetbrains.desktop.buildscripts.buildArch
import org.jetbrains.desktop.buildscripts.buildOs

plugins {
    // Apply the org.jetbrains.kotlin.jvm Plugin to add support for Kotlin.
    alias(libs.plugins.kotlin.jvm)

    // Apply the application plugin to add support for building a CLI application in Java.
    application
}

repositories {
    // Use Maven Central for resolving dependencies.
    mavenCentral()
    maven("https://maven.pkg.jetbrains.space/public/p/compose/dev")
}

val targetOs = when (buildOs()) {
    Os.LINUX -> "linux"
    Os.MACOS -> "macos"
    Os.WINDOWS -> "windows"
}

val targetArch = when (buildArch()) {
    Arch.aarch64 -> "arm64"
    Arch.x86_64 -> "x64"
}

val skikoVersion = "0.8.18"
val skikoTarget = "${targetOs}-${targetArch}"
dependencies {
    // Use the Kotlin JUnit 5 integration.
    testImplementation("org.jetbrains.kotlin:kotlin-test-junit5")

    // Use the JUnit 5 integration.
    testImplementation(libs.junit.jupiter.engine)

    testRuntimeOnly("org.junit.platform:junit-platform-launcher")

    // This dependency is used by the application.
    implementation(libs.guava)
    implementation(project(":kotlin-desktop-toolkit"))
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
    applicationDefaultJvmArgs = listOf("--enable-preview",
                                       "-XstartOnFirstThread",
                                       "--enable-native-access=ALL-UNNAMED",
                                       "-Djextract.trace.downcalls=false")
}

val depScope = configurations.dependencyScope("native") {
    withDependencies {
        add(project.dependencies.project(":kotlin-desktop-toolkit"))
    }
}
val nativeLib = configurations.resolvable("nativeParts") {
    extendsFrom(depScope.get())
    attributes {
        attribute(KotlinDesktopToolkitAttributes.TYPE, KotlingDesktopToolkitArtifactType.NATIVE_LIBRARY)
        attribute(KotlinDesktopToolkitAttributes.PROFILE, KotlingDesktopToolkitNativeProfile.DEBUG)
    }
}

fun JavaExec.setUpLoggingAndLibraryPath() {
    val logFilePath = layout.buildDirectory.file("sample-logs/skiko_sample.log").map { it.asFile.absolutePath }
    val nativeLibPath = nativeLib.map { it.singleFile.absolutePath }
    jvmArgumentProviders.add(CommandLineArgumentProvider {
        listOf(
            "-Dkdt.library.path=${nativeLibPath.get()}",
            "-Dkdt.native.log.path=${logFilePath.get()}",
        )
    })
}

tasks.named<JavaExec>("run") {
    jvmArgs("--enable-preview")
    setUpLoggingAndLibraryPath()
}

tasks.register<JavaExec>("runAppMenuAwtSample") {
    group = "application"
    description = "Runs sample app based on AWT"
    classpath = sourceSets["main"].runtimeClasspath
    mainClass.set("org.jetbrains.desktop.sample.AppMenuAwtSampleKt")
    javaLauncher.set(javaToolchains.launcherFor {
        languageVersion.set(JavaLanguageVersion.of(21))
    })
    jvmArgs = listOf(
        "--enable-preview",
        "--enable-native-access=ALL-UNNAMED",
        "-Djextract.trace.downcalls=false"
    )
    setUpLoggingAndLibraryPath()
}

tasks.register<JavaExec>("runSkikoSample") {
    group = "application"
    description = "Runs example of integration with Skiko"
    classpath = sourceSets["main"].runtimeClasspath
    mainClass.set("org.jetbrains.desktop.sample.SkikoSampleKt")
    javaLauncher.set(javaToolchains.launcherFor {
        languageVersion.set(JavaLanguageVersion.of(21))
    })
    jvmArgs = listOf(
        "--enable-preview",
        "-XstartOnFirstThread",
        "--enable-native-access=ALL-UNNAMED",
        "-Djextract.trace.downcalls=false"
    )
    setUpLoggingAndLibraryPath()
    
    environment("MTL_HUD_ENABLED", 1)
//    environment("MallocStackLogging", "1")
}

tasks.named<Test>("test") {
    // Use JUnit Platform for unit tests.
    useJUnitPlatform()
}
