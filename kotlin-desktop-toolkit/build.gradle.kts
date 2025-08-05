import org.gradle.internal.impldep.kotlinx.serialization.Serializable
import org.jetbrains.desktop.buildscripts.Arch
import org.jetbrains.desktop.buildscripts.CargoFmtTask
import org.jetbrains.desktop.buildscripts.ClippyTask
import org.jetbrains.desktop.buildscripts.CompileRustTask
import org.jetbrains.desktop.buildscripts.CrossCompilationSettings
import org.jetbrains.desktop.buildscripts.DownloadJExtractTask
import org.jetbrains.desktop.buildscripts.GenerateJavaBindingsTask
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitArtifactType
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitAttributes
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitNativeProfile
import org.jetbrains.desktop.buildscripts.Os
import org.jetbrains.desktop.buildscripts.Platform
import org.jetbrains.desktop.buildscripts.buildPlatformRustTarget
import org.jetbrains.desktop.buildscripts.hostArch
import org.jetbrains.desktop.buildscripts.hostOs
import org.jetbrains.desktop.buildscripts.targetArch

private val crossCompilationSettings = CrossCompilationSettings.create(project)
private val nativeDir = layout.projectDirectory.dir("../native")

private val runTestsWithPlatform = Platform(hostOs(), targetArch(project) ?: hostArch())

plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.ktlint)

    `maven-publish`
}
group = "org.jetbrains.kotlin-desktop-toolkit"
version = (project.properties["version"] as? String)?.takeIf { it.isNotBlank() && it != "unspecified" } ?: "SNAPSHOT"

repositories {
    mavenCentral()
}

dependencies {
    // Use the Kotlin JUnit 5 integration.
    testImplementation("org.jetbrains.kotlin:kotlin-test-junit5")

    // Use the JUnit 5 integration.
    testImplementation(libs.junit.jupiter.engine)

    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.compileJava {
    options.compilerArgs = listOf("--enable-preview")
}

// Apply a specific Java toolchain to ease working on different environments.
java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(21)
    }
    withSourcesJar()
}

kotlin {
    explicitApi()
}

@Serializable
data class RustTarget(
    @get:Input val platform: Platform,
    @get:Input val profile: String,
)

val allPlatforms = listOf(
    Platform(Os.MACOS, Arch.x86_64),
    Platform(Os.MACOS, Arch.aarch64),
    Platform(Os.LINUX, Arch.x86_64),
    Platform(Os.LINUX, Arch.aarch64),
)

val enabledPlatforms = allPlatforms.filter { crossCompilationSettings.enabled(it) }

val profiles = listOf("dev", "release")

fun crateNameForOS(os: Os): String {
    return when (os) {
        Os.MACOS -> "desktop-macos"
        Os.WINDOWS -> "desktop-windows"
        Os.LINUX -> "desktop-linux"
    }
}

fun List<Platform>.allOSes(): List<Os> {
    return this.map { it.os }.distinct()
}

val compileNativeTaskByTarget = buildMap {
    for (platform in enabledPlatforms) {
        for (profile in profiles) {
            val buildNativeTask = tasks.register<CompileRustTask>("compileNative-${buildPlatformRustTarget(platform)}-$profile") {
                crateName = crateNameForOS(platform.os)
                rustProfile = profile
                rustTarget = platform
                workspaceRoot = nativeDir
            }
            put(RustTarget(platform, profile), buildNativeTask)
        }
    }
}

val downloadJExtractTask = tasks.register<DownloadJExtractTask>("downloadJExtract") {
    slug = "22/6/openjdk-22-jextract+6-47"
    jextractDirectory = layout.buildDirectory.dir("jextract")
}

fun packageNameForOS(os: Os): String {
    return "org.jetbrains.desktop.${os.normalizedName}.generated"
}

val generateBindingsTaskByOS = allPlatforms.allOSes().associateWith { os ->
    tasks.register<GenerateJavaBindingsTask>("generateBindingsFor${os.normalizedName}") {
        dependsOn(downloadJExtractTask)
        jextractBinary = downloadJExtractTask.flatMap { it.jextractBinary }
        packageName = packageNameForOS(os)
        workspaceRoot = nativeDir
        crateDirectory = nativeDir.dir(crateNameForOS(os))
        generatedSourcesDirectory = layout.buildDirectory.dir("generated/sources/jextract/${os.normalizedName}/main/java/")
    }
}

tasks.compileJava {
    generateBindingsTaskByOS.values.forEach { dependsOn(it) }
}

tasks.compileKotlin {
    generateBindingsTaskByOS.values.forEach { dependsOn(it) }
}

sourceSets.main {
    generateBindingsTaskByOS.values.forEach { task ->
        java.srcDir(task.flatMap { it.generatedSourcesDirectory })
    }
}

// Publishing

fun shouldPublishCommon(): Boolean {
    return (project.property("publishCommon") as String).toBooleanStrict()
}

tasks.withType<Jar>().configureEach {
    isPreserveFileTimestamps = false
    isReproducibleFileOrder = true
}

// Same as in skiko, the Fleet build system breaks on _ in jar name
fun jarSuffixForPlatform(platform: Platform): String {
    val osName = platform.os.normalizedName
    val archName = when (platform.arch) {
        Arch.aarch64 -> "arm64"
        Arch.x86_64 -> "x64"
    }
    return "$osName-$archName"
}

val nativeJarTasksByPlatform = enabledPlatforms.associateWith { platform ->
    val jarSuffix = jarSuffixForPlatform(platform)
    tasks.register<Jar>("package-jar-$jarSuffix") {
        archiveBaseName = "kotlin-desktop-toolkit-$jarSuffix"
        for (profile in profiles) {
            val compileTask = compileNativeTaskByTarget[RustTarget(platform, profile)]!!
            dependsOn(compileTask)
            from(compileTask.flatMap { it.libraryFile })
        }
    }
}

val spaceUsername: String? by project
val spacePassword: String? by project
publishing {
    publications {
        configureEach {
            this as MavenPublication
            pom {
                description.set("Kotlin Desktop Toolkit")
                licenses {
                    license {
                        name.set("The Apache License, Version 2.0")
                        url.set("https://www.apache.org/licenses/LICENSE-2.0.txt")
                    }
                }
                val repoUrl = "https://www.github.com/JetBrains/kotlin-desktop-toolkit"
                url.set(repoUrl)
                scm {
                    url.set(repoUrl)
                    val repoConnection = "scm:git:$repoUrl.git"
                    connection.set(repoConnection)
                    developerConnection.set(repoConnection)
                }
                developers {
                    developer {
                        organization.set("JetBrains")
                        organizationUrl.set("https://www.jetbrains.com")
                    }
                }
            }
        }

        if (shouldPublishCommon()) {
            create<MavenPublication>("Common") {
                from(components["java"])
                artifactId = "kotlin-desktop-toolkit-common"
                pom {
                    licenses {
                        license {
                            name = "The Apache License, Version 2.0"
                            url = "https://www.apache.org/licenses/LICENSE-2.0.txt"
                        }
                    }
                }
            }
        }

        nativeJarTasksByPlatform.forEach { (platform, jarTask) ->
            val suffix = jarSuffixForPlatform(platform)
            create<MavenPublication>("Native-$suffix") {
                artifactId = "kotlin-desktop-toolkit-$suffix"
                artifact(jarTask)
            }
        }
    }
    repositories {
        maven {
            name = "IntellijDependencies"
            url = uri("https://packages.jetbrains.team/maven/p/ij/intellij-dependencies")
            credentials {
                username = spaceUsername
                password = spacePassword
            }
        }
    }
}

// Share artifacts

val nativeConsumable = configurations.consumable("nativeParts") {
    attributes {
        attribute(KotlinDesktopToolkitAttributes.TYPE, KotlinDesktopToolkitArtifactType.NATIVE_LIBRARY)
        attribute(KotlinDesktopToolkitAttributes.PROFILE, KotlinDesktopToolkitNativeProfile.DEBUG)
    }
}

compileNativeTaskByTarget[RustTarget(runTestsWithPlatform, "dev")]?.let { buildNativeTask ->
    artifacts.add(nativeConsumable.name, buildNativeTask.flatMap { it.libraryFile }) {
        builtBy(buildNativeTask) // redundant because of the flatMap usage above, but if you want to be sure, you can specify that
    }
}

// Linting

val cargoFmtCheckTask = tasks.register<CargoFmtTask>("cargoFmtCheck") {
    checkOnly = true
    workingDir = nativeDir.asFile
}

val cargoFmtTask = tasks.register<CargoFmtTask>("cargoFmt") {
    workingDir = nativeDir.asFile
    clippyFixTasks.forEach { mustRunAfter(it) }
}

val clippyCheckTasks = enabledPlatforms.map { target ->
    tasks.register<ClippyTask>("clippyCheck-${buildPlatformRustTarget(target)}") {
        checkOnly = true
        workingDir = nativeDir.asFile
        targetPlatform = target
        crateName = crateNameForOS(target.os)
    }
}

val clippyFixTasks = enabledPlatforms.map { target ->
    tasks.register<ClippyTask>("clippyFix-${buildPlatformRustTarget(target)}") {
        workingDir = nativeDir.asFile
        targetPlatform = target
        crateName = crateNameForOS(target.os)
    }
}

tasks.register("lint") {
    dependsOn(tasks.named("ktlintCheck"))
    clippyCheckTasks.forEach { dependsOn(it) }
    dependsOn(cargoFmtCheckTask)
}

tasks.register("autofix") {
    dependsOn(tasks.named("ktlintFormat"))
    clippyFixTasks.forEach { dependsOn(it) }
    dependsOn(cargoFmtTask)
}

// Junit tests

tasks.test {
    jvmArgs("--enable-preview", "--enable-native-access=ALL-UNNAMED")
    useJUnitPlatform()

    val buildNativeTask = compileNativeTaskByTarget[RustTarget(runTestsWithPlatform, "dev")]
    if (buildNativeTask == null) {
        enabled = false
    } else {
        dependsOn(buildNativeTask)
        val logFile = layout.buildDirectory.file("test-logs/desktop_native.log")
        val libFolder = buildNativeTask.flatMap { it.libraryFile }.map { it.parent }
        jvmArgumentProviders.add(
            CommandLineArgumentProvider {
                listOf(
                    "-Dkdt.library.folder.path=${libFolder.get()}",
                    "-Dkdt.debug=true",
                    "-Dkdt.native.log.path=${logFile.get().asFile.absolutePath}",
                )
            },
        )
    }

    testLogging {
        events("failed")
        events("passed")
        events("skipped")
    }

    // We run every test class in separate JVM
    forkEvery = 1
    maxParallelForks = 1
}
