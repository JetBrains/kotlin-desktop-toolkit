import org.gradle.internal.impldep.kotlinx.serialization.Serializable
import org.jetbrains.desktop.buildscripts.Arch
import org.jetbrains.desktop.buildscripts.CargoFmtTask
import org.jetbrains.desktop.buildscripts.ClippyTask
import org.jetbrains.desktop.buildscripts.CollecNativeArtifactsTask
import org.jetbrains.desktop.buildscripts.CompileRustTask
import org.jetbrains.desktop.buildscripts.CrossCompilationSettings
import org.jetbrains.desktop.buildscripts.DownloadAngleTask
import org.jetbrains.desktop.buildscripts.DownloadJExtractTask
import org.jetbrains.desktop.buildscripts.GenerateJavaBindingsTask
import org.jetbrains.desktop.buildscripts.InstallCargoProgram
import org.jetbrains.desktop.buildscripts.InstallRust
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitArtifactType
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitAttributes
import org.jetbrains.desktop.buildscripts.KotlinDesktopToolkitNativeProfile
import org.jetbrains.desktop.buildscripts.Os
import org.jetbrains.desktop.buildscripts.Platform
import org.jetbrains.desktop.buildscripts.angleArch
import org.jetbrains.desktop.buildscripts.hostArch
import org.jetbrains.desktop.buildscripts.hostOs
import org.jetbrains.desktop.buildscripts.hostPlatform
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

fun mainCrateForOS(os: Os): String {
    return when (os) {
        Os.MACOS -> "desktop-macos"
        Os.WINDOWS -> "desktop-win32"
        Os.LINUX -> "desktop-linux"
    }
}

fun List<Platform>.allOSes(): List<Os> {
    return this.map { it.os }.distinct()
}

private fun ProviderFactory.cargoCommand(): Provider<String> {
    val cargoCommand = gradleProperty("rust.cargoCommand").map {
        it.ifEmpty {
            val homePath = systemProperty("user.home").get()
            val defaultRustupCargoPath = File("$homePath/.cargo/bin/cargo")
            if (defaultRustupCargoPath.canExecute()) {
                defaultRustupCargoPath.path
            } else {
                "cargo"
            }
        }
    }
    return cargoCommand
}

private fun ProviderFactory.rustupCommand(): Provider<String> {
    val rustupCommand = gradleProperty("rust.rustupCommand").map {
        it.ifEmpty {
            val homePath = systemProperty("user.home").get()
            val defaultRustupPath = File("$homePath/.cargo/bin/rustup")
            if (defaultRustupPath.canExecute()) {
                defaultRustupPath.path
            } else {
                "rustup"
            }
        }
    }
    return rustupCommand
}

private fun ProviderFactory.rustcCommand(): Provider<String> {
    val rustcCommand = gradleProperty("rust.rustcCommand").map {
        it.ifEmpty {
            val homePath = systemProperty("user.home").get()
            val defaultRustupPath = File("$homePath/.cargo/bin/rustc")
            if (defaultRustupPath.canExecute()) {
                defaultRustupPath.path
            } else {
                "rustc"
            }
        }
    }
    return rustcCommand
}

/**
 * All workspace files under the directory, excluding compilation outputs and caches
 */
internal fun Directory.rustWorkspaceFiles(): FileTree = this.asFileTree.matching {
    exclude("target/**/*")
}

fun buildPlatformRustTarget(platform: Platform): String {
    return when (platform.os) {
        Os.WINDOWS -> when (platform.arch) {
            Arch.aarch64 -> "aarch64-pc-windows-msvc"
            Arch.x86_64 -> "x86_64-pc-windows-msvc"
        }
        Os.MACOS -> when (platform.arch) {
            Arch.aarch64 -> "aarch64-apple-darwin"
            Arch.x86_64 -> "x86_64-apple-darwin"
        }
        Os.LINUX -> when (platform.arch) {
            Arch.aarch64 -> "aarch64-unknown-linux-gnu"
            Arch.x86_64 -> "x86_64-unknown-linux-gnu"
        }
    }
}

val installRustTaskByPlatform = buildMap {
    for (platform in enabledPlatforms) {
        val target = buildPlatformRustTarget(platform)
        val taskName = "installRust-$target"
        val otherInstallRustTasks = this.values.toList()
        val installRustTask = tasks.register<InstallRust>(taskName) {
            rustupCommand = providers.rustupCommand().get()
            rustcCommand = providers.rustcCommand().get()
            rustToolchainFile = nativeDir.file("rust-toolchain")
            rustTarget = target
            // Using rustup concurrently is not supported: https://github.com/rust-lang/rustup/issues/988
            mustRunAfter(otherInstallRustTasks)
        }

        put(platform, installRustTask)
    }
}

val compileNativeTaskByTarget = buildMap {
    for ((platform, installRustTask) in installRustTaskByPlatform) {
        for (profile in profiles) {
            val buildNativeTask = tasks.register<CompileRustTask>("compileNative-${buildPlatformRustTarget(platform)}-$profile") {
                dependsOn(installRustTask)
                cargoCommand = providers.cargoCommand().get()
                crateName = mainCrateForOS(platform.os)
                rustProfile = profile
                rustTarget = buildPlatformRustTarget(platform)
                targetPlatform = platform
                workspaceRoot = nativeDir.asFile.path
                workspaceFiles = nativeDir.rustWorkspaceFiles()
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
    tasks.register<DownloadAngleTask>("downloadAngleFor${platform.name()}") {
        this.platform = platform
        version = providers.gradleProperty("kdt.win32.angle-version")
        outputDirectory = layout.buildDirectory.dir("angle-${angleArch(platform.arch)}")
    }
}

val collectNativeArtifactsTaskByTarget = compileNativeTaskByTarget.mapValues { (target, buildNativeTask) ->
    val downloadAngleTask = downloadAngleTaskByPlatform[target.platform]
    tasks.register<CollecNativeArtifactsTask>(
        "collectNativeArtifactsFor${target.platform.name()}-${target.profile}",
    ) {
        dependsOn(buildNativeTask)
        downloadAngleTask?.let {
            dependsOn(downloadAngleTask)
            angleBinaries.setFrom(downloadAngleTask.map { it.binaries })
        }
        nativeLibrary = buildNativeTask.flatMap { it.libraryFile }
        targetDirectory = layout.buildDirectory.dir("native-${buildPlatformRustTarget(target.platform)}-${target.profile}")
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

val installCbindgenTask = tasks.register<InstallCargoProgram>("installCbindgen") {
    dependsOn(installRustTaskByPlatform[hostPlatform()]!!)
    cargoCommand = providers.cargoCommand().get()
    crate = "cbindgen"
    version = "0.29.0"
}

val generateBindingsTaskByOS = allPlatforms.allOSes().associateWith { os ->
    tasks.register<GenerateJavaBindingsTask>("generateBindingsFor${os.titlecase()}") {
        dependsOn(downloadJExtractTask)
        dependsOn(installCbindgenTask)
        cbindgenBinary = installCbindgenTask.flatMap { it.targetBinPath }
        jextractBinary = downloadJExtractTask.flatMap { it.jextractBinary }
        packageName = packageNameForOS(os)
        workspaceFiles = nativeDir.rustWorkspaceFiles()
        crateDirectory = nativeDir.dir(mainCrateForOS(os))
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
    cargoCommand = providers.cargoCommand()
    dependsOn(installRustTaskByPlatform[hostPlatform()]!!)
}

val cargoFmtTask = tasks.register<CargoFmtTask>("cargoFmt") {
    workingDir = nativeDir.asFile
    cargoCommand = providers.cargoCommand()
    clippyFixTasks.forEach { mustRunAfter(it) }
    dependsOn(installRustTaskByPlatform[hostPlatform()]!!)
}

val clippyCheckTasks = installRustTaskByPlatform.map { (platform, installRustTask) ->
    tasks.register<ClippyTask>("clippyCheckFor${platform.name()}") {
        checkOnly = true
        workingDir = nativeDir.asFile
        cargoCommand = providers.cargoCommand()
        rustTarget = buildPlatformRustTarget(platform)
        dependsOn(installRustTask)
    }
}

val clippyFixTasks = installRustTaskByPlatform.map { (platform, installRustTask) ->
    tasks.register<ClippyTask>("clippyFixFor${platform.name()}") {
        workingDir = nativeDir.asFile
        cargoCommand = providers.cargoCommand()
        rustTarget = buildPlatformRustTarget(platform)
        dependsOn(installRustTask)
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

    // We run every test class in a separate JVM
    forkEvery = 1
    maxParallelForks = 1
}
