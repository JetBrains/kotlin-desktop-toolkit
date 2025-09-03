import org.gradle.internal.impldep.kotlinx.serialization.Serializable
import org.jetbrains.desktop.buildscripts.Arch
import org.jetbrains.desktop.buildscripts.CargoFmtTask
import org.jetbrains.desktop.buildscripts.ClippyTask
import org.jetbrains.desktop.buildscripts.CollectWindowsArtifactsTask
import org.jetbrains.desktop.buildscripts.CompileRustTask
import org.jetbrains.desktop.buildscripts.CrossCompilationSettings
import org.jetbrains.desktop.buildscripts.DownloadAngleTask
import org.jetbrains.desktop.buildscripts.DownloadJExtractTask
import org.jetbrains.desktop.buildscripts.GenerateJavaBindingsTask
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitArtifactType
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitAttributes
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitNativeProfile
import org.jetbrains.desktop.buildscripts.Os
import org.jetbrains.desktop.buildscripts.Platform
import org.jetbrains.desktop.buildscripts.angleArch
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
    Platform(Os.WINDOWS, Arch.x86_64),
    Platform(Os.WINDOWS, Arch.aarch64),
)

val enabledPlatforms = allPlatforms.filter { crossCompilationSettings.enabled(it) }

val profiles = listOf("dev", "release")

// The first element of the returned list is the main crate
fun crateNamesForOS(os: Os): List<String> {
    return when (os) {
        Os.MACOS -> listOf("desktop-macos")
        Os.WINDOWS -> listOf("desktop-win32")
        Os.LINUX -> listOf("desktop-linux", "desktop-linux-sample", "desktop-linux-test-helper", "desktop-linux-test-helper-http")
    }
}

fun List<Platform>.allOSes(): List<Os> {
    return this.map { it.os }.distinct()
}

val compileNativeTaskByTarget = buildMap {
    for (platform in enabledPlatforms) {
        for (profile in profiles) {
            val buildNativeTask = tasks.register<CompileRustTask>("compileNative-${buildPlatformRustTarget(platform)}-$profile") {
                crateName = crateNamesForOS(platform.os).first()
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

val downloadAngleTaskByPlatform = enabledPlatforms.filter { it.os == Os.WINDOWS }.associateWith { platform ->
    tasks.register<DownloadAngleTask>("downloadAngle-${buildPlatformRustTarget(platform)}") {
        this.platform = platform
        version = providers.gradleProperty("kdt.win32.angle-version")
        outputDirectory = layout.buildDirectory.dir("angle-${angleArch(platform.arch)}")
    }
}

val collectNativeArtifactsTaskByTarget = buildMap {
    for (platform in enabledPlatforms) {
        val downloadAngleTask = downloadAngleTaskByPlatform[platform]
        for (profile in profiles) {
            val buildNativeTask = compileNativeTaskByTarget[RustTarget(platform, profile)]!!
            val collectWindowsArtifactsTask = tasks.register<CollectWindowsArtifactsTask>(
                "collectWindowsArtifacts-${buildPlatformRustTarget(platform)}-$profile",
            ) {
                dependsOn(buildNativeTask)
                downloadAngleTask?.let {
                    dependsOn(downloadAngleTask)
                    angleBinaries.setFrom(downloadAngleTask.map { it.binaries })
                }
                nativeLibrary = buildNativeTask.flatMap { it.libraryFile }
                targetDirectory = layout.buildDirectory.dir("native-${buildPlatformRustTarget(platform)}")
            }
            put(RustTarget(platform, profile), collectWindowsArtifactsTask)
        }
    }
}

fun Os.getKdtName(): String {
    return when (this) {
        Os.WINDOWS -> "win32"
        else -> this.normalizedName
    }
}

fun packageNameForOS(os: Os): String {
    return "org.jetbrains.desktop.${os.getKdtName()}.generated"
}

val generateBindingsTaskByOS = allPlatforms.allOSes().associateWith { os ->
    tasks.register<GenerateJavaBindingsTask>("generateBindingsFor${os.normalizedName}") {
        dependsOn(downloadJExtractTask)
        jextractBinary = downloadJExtractTask.flatMap { it.jextractBinary }
        packageName = packageNameForOS(os)
        workspaceRoot = nativeDir
        crateDirectory = nativeDir.dir(crateNamesForOS(os).first())
        generatedSourcesDirectory = layout.buildDirectory.dir("generated/sources/jextract/${os.normalizedName}/main/java/")
    }
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
            val collectArtifactsTasks = collectNativeArtifactsTaskByTarget[RustTarget(platform, profile)]!!
            dependsOn(collectArtifactsTasks)
            from(collectArtifactsTasks.flatMap { it.targetDirectory })
        }
    }
}

tasks.compileJava {
    dependsOn(nativeJarTasksByPlatform.values)
    dependsOn(generateBindingsTaskByOS.values)
}

tasks.compileKotlin {
    dependsOn(nativeJarTasksByPlatform.values)
    dependsOn(generateBindingsTaskByOS.values)
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

collectNativeArtifactsTaskByTarget[RustTarget(runTestsWithPlatform, "dev")]?.let { collectArtifactsTask ->
    artifacts.add(nativeConsumable.name, collectArtifactsTask.flatMap { it.targetDirectory }) {
        builtBy(collectArtifactsTask) // redundant because of the flatMap usage above, but if you want to be sure, you can specify that
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

val clippyCheckTasks = enabledPlatforms.flatMap { target ->
    crateNamesForOS(target.os).map { targetCrateName ->
        tasks.register<ClippyTask>("clippyCheck-$targetCrateName") {
            checkOnly = true
            workingDir = nativeDir.asFile
            targetPlatform = target
            crateName = targetCrateName
        }
    }
}

val clippyFixTasks = enabledPlatforms.flatMap { target ->
    crateNamesForOS(target.os).map { targetCrateName ->
        tasks.register<ClippyTask>("clippyFix-$targetCrateName") {
            workingDir = nativeDir.asFile
            targetPlatform = target
            crateName = targetCrateName
        }
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

    val collectNativeArtifactsTask = collectNativeArtifactsTaskByTarget[RustTarget(runTestsWithPlatform, "dev")]
    if (collectNativeArtifactsTask == null) {
        enabled = false
    } else {
        dependsOn(collectNativeArtifactsTask)
        val logFile = layout.buildDirectory.file("test-logs/desktop_native.log")
        val libFolder = collectNativeArtifactsTask.flatMap { it.targetDirectory }
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
