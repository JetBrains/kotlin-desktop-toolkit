import org.jetbrains.desktop.buildscripts.Arch
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitAttributes
import org.jetbrains.desktop.buildscripts.KotlingDesktopToolkitArtifactType
import org.jetbrains.desktop.buildscripts.KotlingDesktopToolkitNativeProfile
import org.jetbrains.desktop.buildscripts.hostArch
import org.jetbrains.desktop.buildscripts.hostOs
import org.jetbrains.desktop.buildscripts.targetArch

plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.ktlint)
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
    implementation(project(":kotlin-desktop-toolkit-win32"))
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
        add(project.dependencies.project(":kotlin-desktop-toolkit-win32"))
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
    val logFilePath = layout.buildDirectory.file("sample-logs/skiko_sample_win32.log").map { it.asFile.absolutePath }
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

tasks.register<JavaExec>("runSkikoSampleWindows") {
    group = "application"
    description = "Runs example of integration with Skiko on Windows"
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

task("lint") {
    dependsOn(tasks.named("ktlintCheck"))
}

task("autofix") {
    dependsOn(tasks.named("ktlintFormat"))
}
