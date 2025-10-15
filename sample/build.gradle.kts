import org.jetbrains.desktop.buildscripts.Arch
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitArtifactType
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitAttributes
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitNativeProfile
import org.jetbrains.desktop.buildscripts.Os
import org.jetbrains.desktop.buildscripts.hostArch
import org.jetbrains.desktop.buildscripts.hostOs
import org.jetbrains.desktop.buildscripts.targetArch
import org.panteleyev.jpackage.ImageType

plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.ktlint)
    alias(libs.plugins.jpackage)
}

repositories {
    mavenCentral()
    maven("https://maven.pkg.jetbrains.space/public/p/compose/dev")
}

val skikoTargetOs = hostOs().normalizedName

val skikoTargetArch = when (targetArch(project) ?: hostArch()) {
    Arch.aarch64 -> "arm64"
    Arch.x86_64 -> "x64"
}

val skikoVersion = "0.9.17"
val skikoTarget = "$skikoTargetOs-$skikoTargetArch"
dependencies {
    implementation(project(":kotlin-desktop-toolkit"))
    implementation("org.jetbrains.skiko:skiko-awt-runtime-$skikoTarget:$skikoVersion")
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(21)
    }
}

tasks.compileJava {
    options.compilerArgs = listOf("--enable-preview")
}

val depScope = configurations.dependencyScope("native") {
    withDependencies {
        add(project.dependencies.project(":kotlin-desktop-toolkit"))
    }
}
val nativeLib = configurations.resolvable("nativeParts") {
    extendsFrom(depScope.get())
    attributes {
        attribute(KotlinDesktopToolkitAttributes.TYPE, KotlinDesktopToolkitArtifactType.NATIVE_LIBRARY)
        attribute(KotlinDesktopToolkitAttributes.PROFILE, KotlinDesktopToolkitNativeProfile.DEBUG)
    }
}

fun JavaExec.setUpLoggingAndLibraryPath() {
    val logFilePath = layout.buildDirectory.file("sample-logs/skiko_sample.log").map { it.asFile.absolutePath }
    val nativeLibPath = nativeLib.map { it.singleFile.absolutePath }
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

tasks.register<JavaExec>("runSkikoSampleMac") {
    group = "application"
    description = "Runs example of integration with Skiko on MacOS"
    classpath = sourceSets["main"].runtimeClasspath
    mainClass.set("org.jetbrains.desktop.sample.macos.SkikoSampleMacKt")
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

    environment("MTL_HUD_ENABLED", 1)
//    environment("MallocStackLogging", "1")
}

tasks.register<JavaExec>("runApplicationMenuSampleMac") {
    group = "application"
    description = "Runs example of integration with Application Menu"
    classpath = sourceSets["main"].runtimeClasspath
    mainClass.set("org.jetbrains.desktop.sample.macos.ApplicationMenuSampleMacKt")
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

    environment("MTL_HUD_ENABLED", 1)
//    environment("MallocStackLogging", "1")
}

tasks.register<JavaExec>("runSkikoSampleLinux") {
    group = "application"
    description = "Runs example of integration with Skiko on Linux Wayland"
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

fun JavaExec.setUpCrashDumpPath() {
    val logFilePath = layout.buildDirectory.file("sample-logs/skiko_sample_win32_dump.log").map { it.asFile.absolutePath }
    val crashDumpFilePath = layout.buildDirectory.file("sample-logs/skiko_sample_win32_dump.hprof").map { it.asFile.absolutePath }
    jvmArgumentProviders.add(
        CommandLineArgumentProvider {
            listOf(
                "-XX:+CreateCoredumpOnCrash",
                "-XX:+HeapDumpOnOutOfMemoryError",
                "-XX:ErrorFile=${logFilePath.get()}",
                "-XX:HeapDumpPath=${crashDumpFilePath.get()}",
            )
        },
    )
}

tasks.register<JavaExec>("runSkikoSampleWin32") {
    group = "application"
    description = "Runs example of integration with Skiko on Windows (Win32)"
    classpath = sourceSets["main"].runtimeClasspath
    mainClass.set("org.jetbrains.desktop.sample.win32.SkikoSampleWin32Kt")
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
    setUpCrashDumpPath()
}

tasks.register("lint") {
    dependsOn(tasks.named("ktlintCheck"))
}

tasks.register("autofix") {
    dependsOn(tasks.named("ktlintFormat"))
}

tasks.register<Exec>("runPackagedMac") {
    group = "application"
    description = "Package and run the macOS app bundle"
    dependsOn(tasks.jpackage)
    val appPath = layout.buildDirectory.dir("dist/SkikoSample.app").get().asFile.absolutePath
    commandLine("$appPath/Contents/MacOS/SkikoSample")
}

val prepareJPackageInput by tasks.registering(Copy::class) {
    dependsOn(tasks.jar)
    from(configurations.runtimeClasspath)
    from(tasks.jar)
    into(layout.buildDirectory.dir("jpackage-input"))
}

val prepareJPackageNativeLibs by tasks.registering(Copy::class) {
    from(nativeLib)
    into(layout.buildDirectory.dir("jpackage-input/native"))
}

fun sampleMainClass(): String {
    return when (hostOs()) {
        Os.LINUX -> "org.jetbrains.desktop.sample.linux.SkikoSampleLinuxKt"
        Os.MACOS -> "org.jetbrains.desktop.sample.macos.SkikoSampleMacKt"
        Os.WINDOWS -> "org.jetbrains.desktop.sample.win32.SkikoSampleWin32Kt"
    }
}

tasks.jpackage {
    dependsOn(prepareJPackageInput, prepareJPackageNativeLibs)

    appName = "SkikoSample"
    appVersion = "1.0.0"
    vendor = "JetBrains"
    mainClass = sampleMainClass()
    mainJar = tasks.jar.get().archiveFileName.get()
    type.set(ImageType.APP_IMAGE) // todo replace with something else for other platforms?

    input.set(layout.buildDirectory.dir("jpackage-input"))
    destination.set(layout.buildDirectory.dir("dist"))

    javaOptions = listOf(
        "--enable-preview",
        "--enable-native-access=ALL-UNNAMED",
        "-Djextract.trace.downcalls=false",
        "-Dkdt.library.folder.path=\$APPDIR/native",
        "-Dkdt.debug=true",
        "-Dkdt.native.log.path=\$APPDIR/logs/skiko_sample.log",
    )

    mac {
        macPackageIdentifier = "org.jetbrains.desktop.sample.skiko"
        macAppCategory = "public.app-category.developer-tools"
    }
}
