@file:OptIn(ExperimentalPathApi::class)

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
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.attribute.PosixFilePermission
import java.nio.file.attribute.PosixFilePermissions
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import java.time.format.DateTimeFormatterBuilder
import kotlin.io.path.ExperimentalPathApi
import kotlin.io.path.absolutePathString
import kotlin.io.path.bufferedWriter
import kotlin.io.path.copyTo
import kotlin.io.path.copyToRecursively
import kotlin.io.path.createDirectories
import kotlin.io.path.createDirectory
import kotlin.io.path.createFile
import kotlin.io.path.deleteIfExists
import kotlin.io.path.exists
import kotlin.io.path.getPosixFilePermissions
import kotlin.io.path.listDirectoryEntries
import kotlin.io.path.moveTo
import kotlin.io.path.name
import kotlin.io.path.readLines
import kotlin.io.path.readText
import kotlin.io.path.setPosixFilePermissions
import kotlin.io.path.writeText
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Duration.Companion.seconds
import kotlin.time.TimeSource
import java.time.Duration as JavaDuration

private val crossCompilationSettings = CrossCompilationSettings.create(project)
private val nativeDir = layout.projectDirectory.dir("../native")

private val runTestsWithPlatform = Platform(hostOs(), targetArch(project) ?: hostArch())

plugins {
    kotlin("jvm")
    id("org.jlleitschuh.gradle.ktlint")

    `maven-publish`
}
group = "org.jetbrains.kotlin-desktop-toolkit"
version = (project.findProperty("version") as? String)?.takeIf { it.isNotBlank() && it != "unspecified" } ?: "SNAPSHOT"

repositories {
    mavenCentral()
}

val skikoVersion = "0.9.37.3"
val skikoTargetOs = runTestsWithPlatform.os.normalizedName
val skikoTargetArch = when (runTestsWithPlatform.arch) {
    Arch.aarch64 -> "arm64"
    Arch.x86_64 -> "x64"
}

dependencies {
    // To be able to inspect gradle source code
    compileOnly(gradleApi())

    testImplementation(libs.junit.jupiter)

    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
    testImplementation(libs.assertj.core)
    implementation(kotlin("stdlib"))
}

// Apply a specific Java toolchain to ease working on different environments.
java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(25)
    }
    withSourcesJar()
}

kotlin {
    explicitApi()
}

enum class Backend {
    GTK,
    MACOS,
    WAYLAND,
    WIN32,
    ;

    fun crateDirName(): String = when (this) {
        GTK -> "desktop-gtk"
        MACOS -> "desktop-macos"
        WAYLAND -> "desktop-linux"
        WIN32 -> "desktop-win32"
    }

    fun normalizedName(): String = when (this) {
        GTK -> "gtk"
        MACOS -> "macos"
        WAYLAND -> "linux"
        WIN32 -> "win32"
    }

    fun packageName(): String {
        return "org.jetbrains.desktop.${normalizedName()}.generated"
    }

    fun taskName(): String = when (this) {
        GTK -> "Gtk"
        MACOS -> "MacOs"
        WAYLAND -> "Wayland"
        WIN32 -> "Win32"
    }
}

data class RustTarget(
    @get:Input val platform: Platform,
    @get:Input val profile: String,
    @get:Input val backend: Backend,
)

val enabledPlatforms = crossCompilationSettings.enabled()

val profiles = listOf("dev") // , "release")

private fun backendsForOS(os: Os): List<Backend> {
    val backends = when (os) {
        Os.MACOS -> mutableListOf(Backend.MACOS)
        Os.WINDOWS -> mutableListOf(Backend.WIN32)
        Os.LINUX -> mutableListOf(Backend.WAYLAND, Backend.GTK)
    }
    return backends
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
            for (backend in backendsForOS(platform.os)) {
                val buildNativeTask = tasks.register<CompileRustTask>(
                    "compileNative${backend.taskName()}-${buildPlatformRustTarget(platform)}-$profile",
                ) {
                    dependsOn(installRustTask)
                    cargoCommand = providers.cargoCommand().get()
                    crateName = backend.crateDirName()
                    rustProfile = profile
                    rustTarget = buildPlatformRustTarget(platform)
                    targetPlatform = platform
                    workspaceRoot = nativeDir.asFile.path
                    workspaceFiles = nativeDir.rustWorkspaceFiles()
                }
                put(RustTarget(platform, profile, backend), buildNativeTask)
            }
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
        "collectNativeArtifactsFor${target.backend.taskName()}-${target.platform.name()}-${target.profile}",
    ) {
        downloadAngleTask?.let {
            dependsOn(downloadAngleTask)
            angleBinaries.setFrom(downloadAngleTask.map { it.binaries })
        }

        val prebuiltNativeLibraryPath = (project.property("kdt.prebuiltNativeLibrariesPath") as String).trim().ifEmpty { null }?.let {
            val expectedLibraryFileName = CompileRustTask.libraryFileName(
                targetPlatform = target.platform,
                rustProfile = target.profile,
                crateName = target.backend.crateDirName(),
            )
            val expectedLibraryPath = Path.of(it).resolve(expectedLibraryFileName)
            if (expectedLibraryPath.exists()) expectedLibraryPath else null
        }

        if (prebuiltNativeLibraryPath != null) {
            nativeLibrary = project.objects.fileProperty().fileValue(prebuiltNativeLibraryPath.toFile())
        } else {
            dependsOn(buildNativeTask)
            nativeLibrary = buildNativeTask.flatMap { it.libraryFile }
        }
        targetDirectory = layout.buildDirectory.dir("native-${jarSuffixForPlatform(target.platform, target.backend)}-${target.profile}")
    }
}

val installCbindgenTask = tasks.register<InstallCargoProgram>("installCbindgen") {
    dependsOn(installRustTaskByPlatform[hostPlatform()]!!)
    rustToolchainFile = nativeDir.file("rust-toolchain")
    cargoCommand = providers.cargoCommand().get()
    crate = "cbindgen"
    version = "0.29.4"
}

val installKdotoolTask = tasks.register<InstallCargoProgram>("installKdotool") {
    dependsOn(installRustTaskByPlatform[hostPlatform()]!!)
    rustToolchainFile = nativeDir.file("rust-toolchain")
    cargoCommand = providers.cargoCommand().get()
    crate = "kdotool"
    version = "0.3.0"
}

val generateBindingsTasks = Backend.entries.map { backend ->
    tasks.register<GenerateJavaBindingsTask>("generateBindingsFor${backend.taskName()}") {
        dependsOn(downloadJExtractTask)
        dependsOn(installCbindgenTask)
        cbindgenBinary = installCbindgenTask.flatMap { it.targetBinPath }
        jextractBinary = downloadJExtractTask.flatMap { it.jextractBinary }
        packageName = backend.packageName()
        workspaceFiles = nativeDir.rustWorkspaceFiles()
        crateDirectory = nativeDir.dir(backend.crateDirName())
        generatedSourcesDirectory = layout.buildDirectory.dir("generated/sources/jextract/${backend.crateDirName()}/main/java/")
    }
}

sourceSets.main {
    generateBindingsTasks.forEach { task ->
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
fun jarSuffixForPlatform(platform: Platform, backend: Backend): String {
    val osName = platform.os.normalizedName.let {
        if (backend == Backend.GTK) "gtk-$it" else it
    }
    val archName = when (platform.arch) {
        Arch.aarch64 -> "arm64"
        Arch.x86_64 -> "x64"
    }
    return "$osName-$archName"
}

val nativeJarTasksByTarget = enabledPlatforms
    .flatMap { platform -> backendsForOS(platform.os).map { backend -> Pair(platform, backend) } }
    .associateWith { (platform, backend) ->
        val jarSuffix = jarSuffixForPlatform(platform, backend)
        tasks.register<Jar>("package-jar-$jarSuffix") {
            // every profile like dev and debug contains an identical copy of angle
            // so we take only one of them
            duplicatesStrategy = DuplicatesStrategy.EXCLUDE
            archiveBaseName = "kotlin-desktop-toolkit-$jarSuffix"
            for ((rustTarget, collectArtifactsTasks) in collectNativeArtifactsTaskByTarget) {
                if (rustTarget.platform == platform && rustTarget.backend == backend) {
                    dependsOn(collectArtifactsTasks)
                    from(collectArtifactsTasks.flatMap { it.targetDirectory })
                }
            }
        }
    }

tasks.compileJava {
    dependsOn(nativeJarTasksByTarget.values)
    dependsOn(generateBindingsTasks)
}

tasks.compileKotlin {
    dependsOn(nativeJarTasksByTarget.values)
    dependsOn(generateBindingsTasks)
}

val spaceUsername = project.findProperty("spaceUsername") as String?
val spacePassword = project.findProperty("spacePassword") as String?
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

        nativeJarTasksByTarget.forEach { (target, jarTask) ->
            val suffix = jarSuffixForPlatform(target.first, target.second)
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

for (backend in backendsForOS(runTestsWithPlatform.os)) {
    collectNativeArtifactsTaskByTarget[RustTarget(runTestsWithPlatform, "dev", backend)]?.let { collectArtifactsTask ->
        artifacts.add(nativeConsumable.name, collectArtifactsTask.flatMap { it.targetDirectory }) {
            builtBy(collectArtifactsTask) // redundant because of the flatMap usage above, but if you want to be sure, you can specify that
        }
    }
}

// Linting

val cargoFmtCheckTask = tasks.register<CargoFmtTask>("cargoFmtCheck") {
    checkOnly = true
    group = "verification"
    workingDir = nativeDir.asFile
    cargoCommand = providers.cargoCommand()
    dependsOn(installRustTaskByPlatform[hostPlatform()]!!)
}

val cargoFmtTask = tasks.register<CargoFmtTask>("cargoFmt") {
    group = "formatting"
    workingDir = nativeDir.asFile
    cargoCommand = providers.cargoCommand()
    clippyFixTasksByPlatform.forEach { mustRunAfter(it.value) }
    dependsOn(installRustTaskByPlatform[hostPlatform()]!!)
}

val clippyCheckTasksByPlatform = installRustTaskByPlatform.mapValues { (platform, installRustTask) ->
    tasks.register<ClippyTask>("clippyCheckFor${platform.name()}") {
        checkOnly = true
        group = "verification"
        workingDir = nativeDir.asFile
        cargoCommand = providers.cargoCommand()
        rustTarget = buildPlatformRustTarget(platform)
        dependsOn(installRustTask)
    }
}

val clippyFixTasksByPlatform = installRustTaskByPlatform.mapValues { (platform, installRustTask) ->
    tasks.register<ClippyTask>("clippyFixFor${platform.name()}") {
        workingDir = nativeDir.asFile
        cargoCommand = providers.cargoCommand()
        rustTarget = buildPlatformRustTarget(platform)
        dependsOn(installRustTask)
    }
}

fun createLintTask(name: String, platforms: List<Platform>) {
    tasks.register(name) {
        group = "verification"
        dependsOn(tasks.named("ktlintCheck"))
        for (platform in platforms) {
            dependsOn(clippyCheckTasksByPlatform[platform]!!)
        }
        dependsOn(cargoFmtCheckTask)
    }
}

enabledPlatforms.forEach {
    createLintTask("lintFor${it.name()}", listOf(it))
}
createLintTask("lint", enabledPlatforms)

fun createAutofixTask(name: String, platforms: List<Platform>) {
    tasks.register(name) {
        dependsOn(tasks.named("ktlintFormat"))
        for (platform in platforms) {
            dependsOn(clippyFixTasksByPlatform[platform]!!)
        }
        dependsOn(cargoFmtTask)
    }
}

enabledPlatforms.forEach {
    createAutofixTask("autofixFor${it.name()}", listOf(it))
}
createAutofixTask("autofix", enabledPlatforms)

// Junit tests

abstract class X11TestEnv :
    BuildService<BuildServiceParameters.None>,
    AutoCloseable {
    private var test: Test? = null
    private var successfulRun = false

    private var startedProcesses = mutableListOf<Pair<Process, String>>()
    private var logFiles = mutableListOf<Path>()

    private val homeTempDir by lazy { Files.createTempDirectory("test_home_x11") }
    private val xdgConfigHome by lazy { homeTempDir.resolve(".config").createDirectories() }
    private val xdgDataHome by lazy { homeTempDir.resolve(".local/share").createDirectories() }
    private val xdgRuntimeDir by lazy {
        homeTempDir.resolve("xdg_runtime_dir").createDirectory(
            PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")),
        )
    }

    private val ibusTempDir by lazy { Files.createTempDirectory(xdgRuntimeDir, "test_ibus") }
    private val ibusAddressFile by lazy {
        ibusTempDir.resolve("ibus-addr").createFile() // suppress the IBus warning about the non-existing file
    }
    private val ibusSocketFile by lazy { ibusTempDir.resolve("ibus-socket") }
    private val ibusComponentPath by lazy { ibusTempDir.resolve("component").createDirectory() }
    private val ibusComponentFile by lazy { ibusComponentPath.resolve("ibus_test_engine.xml") }
    private val ibusEngineTmpCapsOutputFile by lazy { ibusTempDir.resolve("test-engine-caps-out.txt") }
    private val ibusEngineTmpContentTypeOutputFile by lazy { ibusTempDir.resolve("test-engine-content-type-out.txt") }
    private val ibusEngineTmpCursorLocationOutputFile by lazy { ibusTempDir.resolve("test-engine-cursor-location-out.txt") }

    private var xSettingsDConfigFile: Path? = null

    private val newEnv by lazy {
        mutableMapOf(
            "GDK_BACKEND" to "x11",
            "GDK_DEBUG" to "no-portals",
            "GTK_USE_PORTAL" to "0",
            "IBUS_ADDRESS_FILE" to ibusAddressFile.absolutePathString(),
            "IBUS_COMPONENT_PATH" to "${ibusComponentPath.absolutePathString()}:/usr/share/ibus/component",
            "TEST_IBUS_ENGINE_CAPS_OUT_FILE" to ibusEngineTmpCapsOutputFile.absolutePathString(),
            "TEST_IBUS_ENGINE_CONTENT_TYPE_OUT_FILE" to ibusEngineTmpContentTypeOutputFile.absolutePathString(),
            "TEST_IBUS_ENGINE_CURSOR_LOCATION_OUT_FILE" to ibusEngineTmpCursorLocationOutputFile.absolutePathString(),
            "LANG" to "en_US.UTF-8",
            "HOME" to homeTempDir.absolutePathString(),
            "XAUTHORITY" to homeTempDir.resolve(".Xauthority").also { it.createFile() }.absolutePathString(),
            "XDG_DATA_HOME" to xdgDataHome.absolutePathString(),
            "XDG_RUNTIME_DIR" to xdgRuntimeDir.absolutePathString(),
            "XDG_SESSION_TYPE" to "x11",
        )
    }

    private fun findFirstAvailableDisplayNumber(): Int {
        var displayNum = 0
        val socketDir = Path.of("/tmp/.X11-unix")
        while (socketDir.resolve("X$displayNum").exists()) {
            displayNum += 1
        }
        return displayNum
    }

    private fun generateIBusXmlFileContent(ibusTestEngineFile: File): String {
        return """<?xml version="1.0" encoding="utf-8"?>
<component>
    <name>com.jetbrains.kdt.IBusTestEngine</name>
    <description>An IBus engine for KDT testing</description>
    <version>0.1.0</version>
    <license>Proprietary</license>
    <author>JetBrains</author>
    <homepage>https://www.jetbrains.com/</homepage>
    <exec>/usr/bin/python3 ${ibusTestEngineFile.absolutePath}</exec>
    <textdomain>jb-kdt-ibus-test-engine</textdomain>
    <engines>
        <engine>
            <name>jb_kdt_ibus_test_engine</name>
            <longname>JetBrains KDT IBus test engine</longname>
            <description>An IBus engine for KDT testing</description>
            <language>en</language>
            <license>Proprietary</license>
            <author>JetBrains</author>
            <layout>us</layout>
            <layout_variant/>
            <layout_option/>
            <hotkeys/>
            <symbol/>
            <setup/>
            <version/>
            <textdomain/>
            <rank>0</rank>
        </engine>
    </engines>
</component>
"""
    }

    private fun waitUntil(msg: String, sleepInterval: Duration = 10.milliseconds, predicate: () -> Boolean) {
        val startTime = TimeSource.Monotonic.markNow()
        while (!predicate()) {
            if (startTime.elapsedNow() > 10.seconds) {
                throw Error("Timed out waiting for $msg")
            }
            Thread.sleep(sleepInterval.inWholeMilliseconds)
        }
    }

    private fun runCommand(vararg args: String): Pair<Int, List<String>> {
        val p = ProcessBuilder(*args).also { pb ->
            val env = pb.environment()
            env.clear()
            env.putAll(newEnv)
        }.start()
        println("Running as PID ${p.pid()}: ${args.joinToString(" ") { "\"$it\"" }}")
        val stdOut = p.inputReader().readLines()
        return Pair(p.waitFor(), stdOut)
    }

    private fun newProcess(
        vararg args: String,
        logStdOut: Boolean = false,
        getAdditionalEnvs: ((Map<String, String>) -> Map<String, String>)? = null,
        afterStart: (Process, Path) -> Boolean = { _, _ -> true },
    ): Boolean {
        println("New process: ${args.joinToString(" ") { "\"$it\"" }}")
        val exeName = args.first().let {
            val p = Path.of(it)
            if (p.isAbsolute) {
                p.name
            } else {
                it
            }
        }
        val logFile = Path.of(newEnv["HOME"]!!).resolve("$exeName-output.log")
        return ProcessBuilder(*args).also { pb ->
            val env = pb.environment()
            val additionalEnvs = getAdditionalEnvs?.invoke(env)
            env.clear()
            env.putAll(newEnv)
            additionalEnvs?.let { env.putAll(it) }
            println(logFile)
            val logRedirect = ProcessBuilder.Redirect.to(logFile.toFile())
            if (logStdOut) {
                pb.redirectOutput(logRedirect)
            } else {
                pb.redirectError(logRedirect)
            }
        }.start().let {
            check(it.isAlive)
            val ret = afterStart(it, logFile)
            if (ret) {
                logFiles.add(logFile)
                startedProcesses.add(Pair(it, args.first()))
            }
            ret
        }
    }

    fun run(
        test: Test,
        wm: String,
        i3config: RegularFile,
        dbusConfigFile: RegularFile,
        dunstConfigFile: RegularFile,
        baseXSettingsDConfigFile: RegularFile,
        ibusTestEngineFile: RegularFile,
        testAppBrowser: RegularFile,
        testAppFileManager: RegularFile,
        testResourcesDir: Directory,
        testAppDataSourceCmd: String,
        headless: Boolean,
    ): Map<String, String> {
        this.test = test

        val dbusServicesDir = xdgDataHome.resolve(
            "dbus-1/services",
        ).createDirectories(PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
        dbusServicesDir.resolve("org.freedesktop.Notifications.service").writeText(
            """[D-BUS Service]
Name=org.freedesktop.Notifications
Exec=/bin/true
""",
        )

        dbusServicesDir.resolve("org.freedesktop.FileManager1.service").writeText(
            """[D-BUS Service]
Name=org.freedesktop.FileManager1
Exec=${testAppFileManager.asFile.absolutePath}
""",
        )

        val userApplicationsDir = xdgDataHome.resolve("applications").createDirectory()

        userApplicationsDir.resolve("test_app_browser.desktop").writeText(
            """[Desktop Entry]
Type=Application
Exec=${testAppBrowser.asFile.absolutePath} %u
Terminal=false
Categories=GNOME;GTK;Network;WebBrowser;
MimeType=x-scheme-handler/chrome;x-scheme-handler/http;x-scheme-handler/https;x-scheme-handler/mailto
StartupNotify=true
Name=KDT Test Browser
""",
        )

        userApplicationsDir.resolve("test_app_file_manager.desktop").writeText(
            """[Desktop Entry]
Type=Application
Exec=${testAppFileManager.asFile.absolutePath} %u
Terminal=false
Categories=GNOME;GTK;FileManager;
MimeType=inode/directory
StartupNotify=true
Name=KDT Test File Manager
""",
        )

        xdgConfigHome.resolve("mimeapps.list").writeText(
            """[Default Applications]
x-scheme-handler/https=test_app_browser.desktop
inode/directory=test_app_file_manager.desktop;
""",
        )

        val xSettingsDConfigFilePathString = homeTempDir.resolve(".xsettingsd").also {
            Path.of(baseXSettingsDConfigFile.asFile.absolutePath).copyTo(it)
            xSettingsDConfigFile = it
        }.absolutePathString()

        val testDisplayNumber = findFirstAvailableDisplayNumber()
        val testDisplay = ":$testDisplayNumber"
        if (headless) {
            newProcess("Xvfb", testDisplay, "-ac", "-screen", "0", "3000x1500x24", "-dpi", "192")
        } else {
            newProcess("Xephyr", testDisplay, "-screen", "3000x1500x24", "-dpi", "192", "-sw-cursor", getAdditionalEnvs = { env ->
                mapOf("DISPLAY" to env["DISPLAY"]!!, "XAUTHORITY" to env["XAUTHORITY"]!!)
            })
        }

        newEnv["DISPLAY"] = testDisplay

        newProcess(
            "dbus-daemon",
            "--config-file=${dbusConfigFile.asFile.absolutePath}",
            "--nofork",
            "--nopidfile",
            "--nosyslog",
            "--print-address",
        ) { p, _ ->
            val dbusSessionBusAddress = p.inputReader().readLine()
            println("dbusSessionBusAddress=$dbusSessionBusAddress")
            newEnv["DBUS_SESSION_BUS_ADDRESS"] = dbusSessionBusAddress
            true
        }

        ProcessBuilder(
            "setxkbmap",
            "-layout",
            "us",
            // "-variant",
            // "intl",
        ).also { pb ->
            val env = pb.environment()
            env.clear()
            env.putAll(newEnv)
            waitUntil("Run setxkbmap", sleepInterval = 100.milliseconds) {
                pb.start().waitFor() == 0
            }
        }

        // `gsettings` works with dconf and XDG Desktop Portal, and GTK uses them only on Wayland.
        // On X11, we need to have some xsettings daemon running.
        // We're using `xsettingsd` as a desktop-agnostic xsettings daemon.
        // E.g., Gnome used `gsd-xsettings` (gnome-settings-daemon) for this.
        var xSettingsDPid: String? = null

        waitUntil("Run xsettingsd", sleepInterval = 100.milliseconds) {
            newProcess("xsettingsd", "--config=$xSettingsDConfigFilePathString") { p, log ->
                xSettingsDPid = p.pid().toString()
                waitUntil("xsettingsd propertly started") {
                    p.isAlive && log.readLines().any { it.contains("Took ownership of selection") }
                }
                newEnv["TEST_XSETTINGSD_LOG_FILE"] = log.absolutePathString()
                true
            }
        }

        runCommand("xauth", "generate", testDisplay, ".", "trusted")

        runCommand(
            "gsettings",
            "set",
            "org.gnome.system.wsdd",
            "display-mode",
            "'disabled'",
        )

        ibusComponentFile.writeText(generateIBusXmlFileContent(ibusTestEngineFile.asFile))
        newProcess(
            "ibus-daemon",
            "-a",
            "unix:path=${ibusSocketFile.absolutePathString()}",
            "--verbose",
            "--panel",
            "disable",
            "--emoji-extension",
            "disable",
            "--xim",
            "--cache=none",
        ) { _, _ ->
            waitUntil("IBus socket file exists") { ibusSocketFile.exists() }
            true
        }

        newProcess(ibusTestEngineFile.asFile.absolutePath)

        when (wm) {
            "i3" -> {
                newProcess("i3", "--shmlog-size=26214400", "-c", i3config.asFile.absolutePath)
            }

            "picom" -> {
                newProcess("i3", "--shmlog-size=26214400", "-c", i3config.asFile.absolutePath)

                newProcess("picom", "--backend", "xrender", "--log-level", "DEBUG") { p, errorLog ->
                    waitUntil("Run picom") {
                        p.isAlive && errorLog.readLines().any { it.contains("Screen redirected.") }
                    }
                    true
                }
            }

            else -> newProcess(*wm.split(' ').toTypedArray())
        }

        waitUntil("Get _NET_SUPPORTED", sleepInterval = 100.milliseconds) {
            val (exitCode, stdOut) = runCommand("xprop", "-root", "_NET_SUPPORTED")
            (exitCode == 0 && !stdOut.any { it.contains("no such atom on any window.") || it.contains("not found.") }).also {
                if (it) {
                    println("_NET_SUPPORTED: $stdOut")
                }
            }
        }
//        val p = ProcessBuilder("ps", "ux").start()
//        println("ps ux -> ${p.inputReader().readLines().joinToString("\n")}")

        newEnv["GSK_RENDERER"] = "vulkan"
        newEnv["TEST_DUNST_CONFIG_FILE"] = dunstConfigFile.asFile.absolutePath
        newEnv["TEST_RESOURCES_DIR"] = testResourcesDir.asFile.absolutePath
        newEnv["TEST_APP_DATA_SOURCE_CMD"] = testAppDataSourceCmd
        newEnv["TEST_XSETTINGSD_PID"] = xSettingsDPid!!
        newEnv["TEST_XSETTINGSD_CONFIG_FILE"] = xSettingsDConfigFilePathString
        successfulRun = true
        return newEnv
    }

    override fun close() {
        val testsFailed = try {
            test?.state?.rethrowFailure()
            false
        } catch (_: Throwable) {
            true
        }
        test = null

        startedProcesses.reverse()
        for ((p, name) in startedProcesses) {
            val wasAlive = p.isAlive
            if (!wasAlive) {
                println("ERROR: $name is not alive")
            }
            if (name == "i3" && (!wasAlive || testsFailed)) {
                val f = File.createTempFile("i3-out", ".log")
                val p = ProcessBuilder("i3-dump-log").also { pb ->
                    pb.redirectOutput(f)
                    val env = pb.environment()
                    env.clear()
                    env.putAll(newEnv)
                }.start()
                p.waitFor()
                println("i3-dump-log output: ${f.absolutePath}")
            }

            p.toHandle().destroy()
            if (!wasAlive || testsFailed || name == "dbus-daemon") {
                val stderr = p.errorReader().readText()
                if (stderr.isNotBlank()) {
                    println("\n$name stderr:\n$stderr")
                }
            }
            p.destroy()
            p.waitFor()
        }
        startedProcesses.clear()

        ibusAddressFile.deleteIfExists()
        ibusSocketFile.deleteIfExists()
        ibusEngineTmpCapsOutputFile.deleteIfExists()
        ibusEngineTmpContentTypeOutputFile.deleteIfExists()
        ibusEngineTmpCursorLocationOutputFile.deleteIfExists()
        ibusComponentFile.deleteIfExists()
        ibusComponentPath.deleteIfExists()
        ibusTempDir.deleteIfExists()

        if (successfulRun && !testsFailed) {
            for (logFile in logFiles) {
                logFile.deleteIfExists()
            }
            homeTempDir.toFile().deleteRecursively()
        }
    }
}

fun registerTaskForRustBinary(name: String): TaskProvider<Exec> {
    return tasks.register<Exec>(
        "buildRustBinary-$name-${buildPlatformRustTarget(runTestsWithPlatform)}",
    ) {
        val rustTarget = buildPlatformRustTarget(runTestsWithPlatform)
        dependsOn(installRustTaskByPlatform[runTestsWithPlatform]!!)
        inputs.files(nativeDir.rustWorkspaceFiles())
        workingDir = nativeDir.asFile.resolve(name)
        executable = providers.cargoCommand().get()
        args = listOf(
            "build",
            "--color=always",
            "--target=$rustTarget",
        )
        outputs.file(nativeDir.file("target/$rustTarget/debug/$name"))
    }
}

val buildTestAppWaylandVirtualDevices = registerTaskForRustBinary("desktop-linux-test-app-wlr-virtual-input")
val buildTestAppUInputVirtualDevices = registerTaskForRustBinary("desktop-linux-test-app-uinput-virtual-input")
val buildTestAppDataSource = registerTaskForRustBinary("desktop-linux-test-app-data-source")

sourceSets {
    create("testGtk") {
        compileClasspath += sourceSets.main.get().output
        runtimeClasspath += sourceSets.main.get().output
    }
}

val testGtkImplementation = configurations.getByName("testGtkImplementation") {
    extendsFrom(configurations.implementation.get())
}
val testGtkRuntimeOnly = configurations.getByName("testGtkRuntimeOnly")

configurations["testGtkRuntimeOnly"].extendsFrom(configurations.runtimeOnly.get())

dependencies {
    testGtkImplementation(libs.junit.jupiter)
    testGtkImplementation(libs.assertj.core)
    testGtkImplementation("org.jetbrains.skiko:skiko-awt-runtime-$skikoTargetOs-$skikoTargetArch:$skikoVersion")
    testGtkImplementation("com.github.moaxcp.x11:x11-client:0.22.0")
    testGtkImplementation("com.squareup.moshi:moshi-kotlin:1.15.2")
    testGtkRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

val testGtk = tasks.register<Test>("testGtk") {
    description = "Test GTK implementation."
    group = "verification"

    testClassesDirs = sourceSets["testGtk"].output.classesDirs
    classpath = sourceSets["testGtk"].runtimeClasspath

    configureTestTask(this, listOf(Backend.GTK))

    if (backendsForOS(runTestsWithPlatform.os).contains(Backend.GTK)) {
        dependsOn(buildTestAppDataSource)
        val x11TestEnvProvider = gradle.sharedServices.registerIfAbsent("x11TestEnv", X11TestEnv::class)
        usesService(x11TestEnvProvider)
        val testResourcesDir = layout.projectDirectory.dir("src/test/resources/linux")
        val testAppDataSourceCmd = buildTestAppDataSource.map { it.outputs.files.first() }.get().absolutePath
        val headless = (project.property("tests.x11.headless") as String).toBooleanStrict()
        val wm = project.property("tests.x11.wm") as String
        doFirst {
            val x11TestEnv = x11TestEnvProvider.get()
            try {
                val newEnv = x11TestEnv.run(
                    this@register,
                    wm,
                    i3config = testResourcesDir.file("i3_test_config"),
                    dbusConfigFile = testResourcesDir.file("dbus-session-conf.xml"),
                    dunstConfigFile = testResourcesDir.file("dunstrc.conf"),
                    baseXSettingsDConfigFile = testResourcesDir.file("xsettingsd.conf"),
                    ibusTestEngineFile = testResourcesDir.file("ibus_test_engine.py"),
                    testAppBrowser = testResourcesDir.file("test_app_browser"),
                    testAppFileManager = testResourcesDir.file("test_app_file_manager"),
                    testResourcesDir = testResourcesDir,
                    testAppDataSourceCmd = testAppDataSourceCmd,
                    headless = headless,
                )
                println(newEnv)
                setEnvironment(newEnv)
            } catch (e: Throwable) {
                x11TestEnv.close()
                throw e
            }
        }
    }
}

sourceSets {
    create("testWayland") {
        compileClasspath += sourceSets.main.get().output
        runtimeClasspath += sourceSets.main.get().output
    }
}

val testWaylandImplementation = configurations.getByName("testWaylandImplementation") {
    extendsFrom(configurations.implementation.get())
}
val testWaylandRuntimeOnly = configurations.getByName("testWaylandRuntimeOnly")

configurations["testWaylandRuntimeOnly"].extendsFrom(configurations.runtimeOnly.get())

dependencies {
    testWaylandImplementation(libs.junit.jupiter)
    testWaylandImplementation(libs.assertj.core)
    testWaylandImplementation("org.jetbrains.skiko:skiko-awt-runtime-$skikoTargetOs-$skikoTargetArch:$skikoVersion")
    testWaylandImplementation("com.squareup.moshi:moshi-kotlin:1.15.2")
    testWaylandRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

val testWayland = tasks.register<Test>("testWayland") {
    description = "Test Wayland implementation."
    group = "verification"

    testClassesDirs = sourceSets["testWayland"].output.classesDirs
    classpath = sourceSets["testWayland"].runtimeClasspath

    configureTestTask(this, listOf(Backend.WAYLAND))

    if (backendsForOS(runTestsWithPlatform.os).contains(Backend.WAYLAND)) {
        dependsOn(buildTestAppWaylandVirtualDevices)
        dependsOn(buildTestAppUInputVirtualDevices)
        dependsOn(buildTestAppDataSource)
        dependsOn(installKdotoolTask)
        val waylandTestEnvProvider = gradle.sharedServices.registerIfAbsent("waylandTestEnv", WaylandTestEnv::class)
        usesService(waylandTestEnvProvider)
        val testResourcesDir = layout.projectDirectory.dir("src/test/resources/linux")
        val runVirtualDevicesCmd = buildTestAppWaylandVirtualDevices.map { it.outputs.files.first() }.get().absolutePath
        val runUInputVirtualDevicesCmd = buildTestAppUInputVirtualDevices.map { it.outputs.files.first() }.get().absolutePath
        val testAppDataSourceCmd = buildTestAppDataSource.map { it.outputs.files.first() }.get().absolutePath
        val kdotoolCmd = installKdotoolTask.flatMap { it.targetBinPath }.get()
        val ime = project.property("tests.wayland.ime") as String
        val wm = project.property("tests.wayland.wm") as String
        val currentEnv = environment
        doFirst {
            val waylandTestEnv = waylandTestEnvProvider.get()
            try {
                val newEnv = waylandTestEnv.run(
                    this@register,
                    ime = ime,
                    wm = wm,
                    currentEnv = currentEnv,
                    testResourcesDir = testResourcesDir,
                    kdotoolCmd = kdotoolCmd,
                    runVirtualDevicesCmd = runVirtualDevicesCmd,
                    testAppDataSourceCmd = testAppDataSourceCmd,
                    runUInputVirtualDevicesCmd = runUInputVirtualDevicesCmd,
                    headless = false,
                )
                println(newEnv)
                setEnvironment(newEnv)
            } catch (e: Throwable) {
                waylandTestEnv.close()
                throw e
            }
        }
    }
}

fun configureTestTask(test: Test, backends: List<Backend>) {
    test.apply {
        jvmArgs("--enable-native-access=ALL-UNNAMED")
        useJUnitPlatform()
        addTestListener(object : TestListener {
            private fun withTimestamp(message: String): String {
                val formatter = DateTimeFormatterBuilder()
                    .append(DateTimeFormatter.ISO_LOCAL_DATE).appendLiteral(' ')
                    .append(DateTimeFormatter.ISO_LOCAL_TIME)
                    .toFormatter()
                val time = LocalDateTime.now().format(formatter)
                return "$time: $message"
            }

            private fun getPrintableTestName(descriptor: TestDescriptor?): String {
                val names = mutableListOf<String>()
                var current = descriptor
                while (current != null) {
                    names.add(current.displayName)
                    current = current.parent
                }
                names.reverse()
                return names.joinToString(" > ")
            }

            private fun log(msg: String) {
                logger.warn(withTimestamp(msg))
            }

            override fun beforeSuite(suite: TestDescriptor?) {
                log("beforeSuite: ${getPrintableTestName(suite)}")
            }

            override fun beforeTest(testDescriptor: TestDescriptor?) {
                log("beforeTest: ${getPrintableTestName(testDescriptor)}")
            }

            override fun afterSuite(suite: TestDescriptor?, result: TestResult?) {
                log("afterSuite: ${getPrintableTestName(suite)}")
            }

            override fun afterTest(testDescriptor: TestDescriptor?, result: TestResult?) {
                log("afterTest: ${getPrintableTestName(testDescriptor)}")
            }
        })

        val getLibFolderForBackend: Map<Backend, Provider<Directory>> = buildMap {
            for (backend in backends) {
                val collectNativeArtifactsTask = collectNativeArtifactsTaskByTarget[RustTarget(runTestsWithPlatform, "dev", backend)]
                if (collectNativeArtifactsTask != null) {
                    dependsOn(collectNativeArtifactsTask)
                    put(backend, collectNativeArtifactsTask.flatMap { it.targetDirectory })
                }
            }
        }

        val logFile = layout.buildDirectory.file("test-logs/$name-desktop_native.log")
        jvmArgumentProviders.add(
            CommandLineArgumentProvider {
                listOf(
                    "-Dkdt.debug=true",
                    "-Dkdt.native.log.path=${logFile.get().asFile.absolutePath}",
                ) + getLibFolderForBackend.map { (backend, libFolder) ->
                    "-Dkdt.${backend.normalizedName()}.library.folder.path=${libFolder.get()}"
                }
            },
        )

        systemProperty(
            "java.util.logging.config.file",
            layout.projectDirectory.file("src/test/resources/logging.properties").asFile.absolutePath,
        )

        systemProperty("junit.jupiter.testmethod.order.default", $$"org.junit.jupiter.api.MethodOrderer$Random")
        systemProperty("junit.jupiter.testclass.order.default", $$"org.junit.jupiter.api.ClassOrderer$Random")
//        systemProperty("junit.jupiter.execution.order.random.seed", 55999234918088)
//        systemProperty("junit.jupiter.execution.class.order.random.seed", 55999234918088)

        testLogging {
            showStandardStreams = true
            exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
            events("failed")
            events("passed")
            events("skipped")
        }

        timeout = JavaDuration.ofMinutes(10)

        // We run every test class in a separate JVM
        forkEvery = 1
        maxParallelForks = 1
    }
}

abstract class WaylandTestEnv :
    BuildService<BuildServiceParameters.None>,
    AutoCloseable {
    private var test: Test? = null

    private var startedProcesses = mutableListOf<Pair<Process, String>>()
    private var logFiles = mutableListOf<Path>()
    private var tempFiles = mutableListOf<Path>()
    private var wmPid: Int? = null

    private val homeTempDir by lazy { Files.createTempDirectory("test_home_wayland") }
    private val xdgConfigHome by lazy { homeTempDir.resolve(".config").createDirectories() }
    private val xdgDataHome by lazy { homeTempDir.resolve(".local/share").createDirectories() }
    private val xdgRuntimeDir by lazy {
        homeTempDir.resolve("xdg_runtime_dir").createDirectory(
            PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")),
        )
    }

    private val newEnv by lazy {
        mutableMapOf(
//            "XDG_CURRENT_DESKTOP" to "GNOME",
            "LANG" to "en_US.UTF-8",
            "HOME" to homeTempDir.absolutePathString(),
            "XDG_DATA_HOME" to xdgDataHome.absolutePathString(),
            "XDG_RUNTIME_DIR" to xdgRuntimeDir.absolutePathString(),
            "XDG_SESSION_TYPE" to "wayland",
            "XDG_DATA_DIRS" to "/usr/share/",
            "PATH" to "/usr/sbin:/usr/bin:/sbin:/bin",
//            "WAYLAND_DEBUG" to "1",
        )
    }

    private fun generateIBusXmlFileContent(ibusTestEngineFile: Path): String {
        return """<?xml version="1.0" encoding="utf-8"?>
<component>
    <name>com.jetbrains.kdt.IBusTestEngine</name>
    <description>An IBus engine for KDT testing</description>
    <version>0.1.0</version>
    <license>Proprietary</license>
    <author>JetBrains</author>
    <homepage>https://www.jetbrains.com/</homepage>
    <exec>/usr/bin/python3 ${ibusTestEngineFile.absolutePathString()}</exec>
    <textdomain>jb-kdt-ibus-test-engine</textdomain>
    <engines>
        <engine>
            <name>jb_kdt_ibus_test_engine</name>
            <longname>JetBrains KDT IBus test engine</longname>
            <description>An IBus engine for KDT testing</description>
            <language>en</language>
            <license>Proprietary</license>
            <author>JetBrains</author>
            <layout>us</layout>
            <layout_variant/>
            <layout_option/>
            <hotkeys/>
            <symbol/>
            <setup/>
            <version/>
            <textdomain/>
            <rank>0</rank>
        </engine>
    </engines>
</component>
"""
    }

    private fun waitUntil(msg: String, sleepInterval: Duration = 10.milliseconds, predicate: () -> Boolean) {
        val startTime = TimeSource.Monotonic.markNow()
        while (!predicate()) {
            if (startTime.elapsedNow() > 10.seconds) {
                throw Error("Timed out waiting for $msg")
            }
            Thread.sleep(sleepInterval.inWholeMilliseconds)
        }
    }

    private fun runCommand(vararg args: String): Pair<Int, Process> {
        val p = ProcessBuilder(*args).also { pb ->
            val env = pb.environment()
            env.clear()
            env.putAll(newEnv)
        }.start()
        println("Running as PID ${p.pid()}: ${args.joinToString(" ") { "\"$it\"" }}")
        val exitCode = p.waitFor()
        return Pair(exitCode, p)
    }

    private fun newProcess(
        vararg args: String,
        logStdOut: Boolean = false,
        getAdditionalEnvs: ((Map<String, String>) -> Map<String, String>)? = null,
    ): Pair<Process, Path> {
        println("Running: ${args.joinToString(" ") { "\"$it\"" }}")
        val exeName = args.first().let {
            val p = Path.of(it)
            if (p.isAbsolute) {
                p.name
            } else {
                it
            }
        }
        val logFileStderr = Path.of(newEnv["HOME"]!!).resolve("$exeName-stderr.log")
        val p = ProcessBuilder(*args).also { pb ->
            val env = pb.environment()
            val additionalEnvs = getAdditionalEnvs?.invoke(env)
            env.clear()
            env.putAll(newEnv)
            additionalEnvs?.let { env.putAll(it) }

            println(logFileStderr)
            logFiles.add(logFileStderr)
            val logRedirect = ProcessBuilder.Redirect.to(logFileStderr.toFile())
            if (logStdOut) {
                pb.redirectOutput(logRedirect)
            } else {
                pb.redirectError(logRedirect)
            }
        }.start()

        println("$exeName started: ${p.pid()}")
        check(p.isAlive)
        startedProcesses.add(Pair(p, exeName))
        return Pair(p, logFileStderr)
    }

    fun run(
        test: Test,
        ime: String,
        wm: String,
        currentEnv: Map<String, Any>,
        testResourcesDir: Directory,
        kdotoolCmd: String,
        runVirtualDevicesCmd: String,
        runUInputVirtualDevicesCmd: String,
        testAppDataSourceCmd: String,
        headless: Boolean,
    ): Map<String, String> {
        println("WaylandTestEnv run")
        this.test = test
        val testResourcesDir = testResourcesDir.asFile.toPath()

        val swayConfig = testResourcesDir.resolve("sway_test_config")
        val dbusConfigFile = testResourcesDir.resolve("dbus-session-conf.xml")
        val ibusTestEngineFile = testResourcesDir.resolve("ibus_test_engine.py")
        val testAppBrowser = testResourcesDir.resolve("test_app_browser")
        val testAppFileManager = testResourcesDir.resolve("test_app_file_manager")

        val dbusServicesDir = xdgDataHome.resolve(
            "dbus-1/services",
        ).createDirectories(PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
        dbusServicesDir.resolve("org.freedesktop.Notifications.service").writeText(
            """[D-BUS Service]
Name=org.freedesktop.Notifications
Exec=/bin/true
""",
        )

        dbusServicesDir.resolve("org.freedesktop.FileManager1.service").writeText(
            """[D-BUS Service]
Name=org.freedesktop.FileManager1
Exec=${testAppFileManager.absolutePathString()}
""",
        )

        val userApplicationsDir = xdgDataHome.resolve("applications").createDirectory()

        userApplicationsDir.resolve("test_app_browser.desktop").writeText(
            """[Desktop Entry]
Type=Application
Exec=${testAppBrowser.absolutePathString()} %u
Terminal=false
Categories=GNOME;GTK;Network;WebBrowser;
MimeType=x-scheme-handler/chrome;x-scheme-handler/http;x-scheme-handler/https;x-scheme-handler/mailto
StartupNotify=true
Name=KDT Test Browser
""",
        )

        xdgConfigHome.resolve("mimeapps.list").writeText(
            """[Default Applications]
x-scheme-handler/https=test_app_browser.desktop;
""",
        )

        xdgConfigHome.resolve("xdg-desktop-portal").createDirectories().resolve("portals.conf").writeText(
            """[preferred]
default=kde
org.freedesktop.impl.portal.Settings=gtk
org.freedesktop.impl.portal.Screenshot=kde
org.freedesktop.impl.portal.Secret=kwallet
""",
        )

        homeTempDir.resolve("Pictures").createDirectory()

        val waylandDisplay = "wayland-0"
        newEnv["WAYLAND_DISPLAY"] = waylandDisplay

        when (wm) {
            "kwin" -> {
                newEnv["KDE_SESSION_VERSION"] = "6"
                newEnv["XDG_CURRENT_DESKTOP"] = "KDE"
                newEnv["KWIN_SCREENSHOT_NO_PERMISSION_CHECKS"] = "1"
//                newEnv["KWIN_WAYLAND_NO_PERMISSION_CHECKS"] = "1"
//                newEnv["KWIN_FORCE_SW_CURSOR"] = "1"
            }
        }

        val dbusSessionBusAddress = newProcess(
            "dbus-daemon",
            "--config-file=${dbusConfigFile.absolutePathString()}",
            "--nofork",
            "--nopidfile",
            "--nosyslog",
            "--print-address",
        ).let { (p, _) ->
            p.inputReader().readLine()
        }
        newEnv["DBUS_SESSION_BUS_ADDRESS"] = dbusSessionBusAddress

        newProcess(runUInputVirtualDevicesCmd)

        var mouseEventDevice: String? = null
        waitUntil("Fake mouse device created") {
            mouseEventDevice = Path.of("/sys/class/input/").listDirectoryEntries("event*").firstNotNullOfOrNull {
                if (it.resolve("device/name").readText().trim() == "kdt_test_mouse") {
                    it.name
                } else {
                    null
                }
            }
            mouseEventDevice != null
        }

        newProcess("busctl", "--user", "monitor", logStdOut = true)

        val kdtTestWmRunScript = Path.of((currentEnv["HOME"] as String)).resolve("kdt-test-wm-run")
        kdtTestWmRunScript.parent.resolve("kdt-test-wm-run.tmp").also { tmp ->
            tmp.bufferedWriter().use { writer ->
                for ((k, v) in newEnv) {
                    if (k != "WAYLAND_DISPLAY") {
                        writer.write("$k=$v ")
                    }
                }
                when (wm) {
                    "kwin" -> {
                        writer.write("exec kwin_wayland --no-lockscreen --no-global-shortcuts --no-kactivities")
                    }
                    else -> {
                        writer.write("exec ${wm} -c /home/ubuntu/.config/sway/config")
                    }
                }
            }
            val permissions = tmp.getPosixFilePermissions().toMutableSet()
            permissions.add(PosixFilePermission.OWNER_EXECUTE)
            tmp.setPosixFilePermissions(permissions)
            tmp.moveTo(kdtTestWmRunScript)
        }
        tempFiles.add(kdtTestWmRunScript)

        when (wm) {
            "kwin" -> {
                waitUntil("Compositor started") {
                    xdgRuntimeDir.resolve(waylandDisplay).exists()
                }

                waitUntil("KWin started") {
                    wmPid = runCommand(
                        "busctl",
                        "--user",
                        "call",
                        "org.freedesktop.DBus",
                        "/org/freedesktop/DBus",
                        "org.freedesktop.DBus",
                        "GetConnectionUnixProcessID",
                        "s",
                        "org.kde.KWin",
                    ).let { (exitCode, p) ->
                        if (exitCode == 0) {
                            val out = p.inputReader().readText().trim()
                            out.split(' ').last().toInt()
                        } else {
                            null
                        }
                    }
                    wmPid != null
                }
            }
            "sway" -> {
                val wmEnv = Path.of("/tmp/kdt-test-wm-env").let { testWmEnvFile ->
                    waitUntil("WM started") { testWmEnvFile.exists() }
                    val testWmEnvFileContent = testWmEnvFile.readLines()
                    testWmEnvFile.deleteIfExists()
                    val env = mutableMapOf<String, String>()
                    for (l in testWmEnvFileContent) {
                        val (k, v) = l.split('=', limit = 2)
                        env[k] = v
                    }
                    env
                }

                wmEnv["SWAYSOCK"]?.let {
                    newEnv["SWAYSOCK"] = it
                }
                newEnv["WAYLAND_DISPLAY"] = wmEnv["WAYLAND_DISPLAY"]!!
            }
        }

//        if (headless) {
//
//            newProcess("kwin_wayland", "--virtual", "--socket", waylandDisplay)
//        } else {
//            val wmEnv = Path.of("/tmp/kdt-test-wm-env").let { testWmEnvFile ->
//                waitUntil("WM started") { testWmEnvFile.exists() }
//                val testWmEnvFileContent = testWmEnvFile.readLines()
//                val env = mutableMapOf<String, String>()
//                for (l in testWmEnvFileContent) {
//                    val (k, v) = l.split('=', limit = 2)
//                    env[k] = v
//                }
//                env
//            }
//
//            wmEnv["SWAYSOCK"]?.let {
//                newEnv["SWAYSOCK"] = it
//            }
//            val orgXdgRuntimeDir = currentEnv["XDG_RUNTIME_DIR"] as String
//            val orgWaylandDisplay = wmEnv["WAYLAND_DISPLAY"] ?: "wayland-0"
//            waitUntil("Compositor started") {
//                Path.of(orgXdgRuntimeDir).resolve(orgWaylandDisplay).exists()
//            }
//            xdgRuntimeDir.resolve(orgWaylandDisplay).createSymbolicLinkPointingTo(Path.of(orgXdgRuntimeDir).resolve(orgWaylandDisplay))
//            newEnv["WAYLAND_DISPLAY"] = orgWaylandDisplay
//        }

        runCommand("dbus-update-activation-environment", "WAYLAND_DISPLAY=${newEnv["WAYLAND_DISPLAY"]}")

        if (wm == "kwin") {
            runCommand(
                "busctl",
                "--user",
                "get-property",
                "org.kde.KWin",
                "/org/kde/KWin/InputDevice/$mouseEventDevice",
                "org.kde.KWin.InputDevice",
                "supportsPointerAccelerationProfileFlat",
            ).let { (exitCode, p) ->
                if (exitCode == 0) {
                    println("supportsPointerAccelerationProfileFlat: ${p.inputReader().readText()}")
                } else {
                    println("supportsPointerAccelerationProfileFlat error: ${p.errorReader().readText()}")
                }
            }

            runCommand(
                "busctl",
                "--user",
                "set-property",
                "org.kde.KWin",
                "/org/kde/KWin/InputDevice/$mouseEventDevice",
                "org.kde.KWin.InputDevice",
                "pointerAccelerationProfileFlat",
                "b",
                "true",
            ).let { (exitCode, p) ->
                if (exitCode == 0) {
                    println("set pointerAccelerationProfileFlat: ${p.inputReader().readText()}")
                } else {
                    println("set pointerAccelerationProfileFlat error: ${p.errorReader().readText()}")
                }
            }
            runCommand(
                "busctl",
                "--user",
                "get-property",
                "org.kde.KWin",
                "/org/kde/KWin/InputDevice/$mouseEventDevice",
                "org.kde.KWin.InputDevice",
                "pointerAccelerationProfileFlat",
            ).let { (exitCode, p) ->
                if (exitCode == 0) {
                    println("pointerAccelerationProfileFlat: ${p.inputReader().readText()}")
                } else {
                    println("pointerAccelerationProfileFlat error: ${p.errorReader().readText()}")
                }
            }
            runCommand(
                "busctl",
                "--user",
                "get-property",
                "org.kde.KWin",
                "/org/kde/KWin/InputDevice/$mouseEventDevice",
                "org.kde.KWin.InputDevice",
                "pointerAccelerationProfileAdaptive",
            ).let { (exitCode, p) ->
                if (exitCode == 0) {
                    println("pointerAccelerationProfileAdaptive: ${p.inputReader().readText()}")
                } else {
                    println("pointerAccelerationProfileAdaptive error: ${p.errorReader().readText()}")
                }
            }
        }

        ProcessBuilder(
            "gsettings",
            "set",
            "org.gnome.system.wsdd",
            "display-mode",
            "'disabled'",
        ).also { pb ->
            val env = pb.environment()
            env.clear()
            env.putAll(newEnv)
        }.start().waitFor()

//        newProcess("/usr/libexec/xdg-desktop-portal", "--replace", "--verbose")

        runCommand("busctl", "--user", "call",
            "org.freedesktop.impl.portal.PermissionStore",
            "/org/freedesktop/impl/portal/PermissionStore",
            "org.freedesktop.impl.portal.PermissionStore",
            "SetPermission",
            "sbssas",
            "screenshot",
            "true",
            "screenshot",
            "",
            "1",
            "yes"
        ).let { (exitCode, p) ->
            if (exitCode == 0) {
                println("PermissionStore: ${p.inputReader().readText()}")
            } else {
                println("PermissionStore error: ${p.errorReader().readText()}")
            }
        }

//        runCommand("kwriteconfig6", "--file", "kscreensaverrc", "--group", "Daemon", "--key", "Autolock", "false")
//        runCommand("dbus-send", "--dest=org.freedesktop.ScreenSaver", "/MainApplication", "org.freedesktop.ScreenSaver.reparseConfiguration")
        runCommand("busctl", "--user", "tree").let { (exitCode, p) ->
            if (exitCode == 0) {
                println("busctl tree: ${p.inputReader().readText()}")
            } else {
                println("busctl tree error: ${p.errorReader().readText()}")
            }
        }

        runCommand("busctl", "--user", "introspect", "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop").let { (exitCode, p) ->
            if (exitCode == 0) {
                println("org.freedesktop.portal.Desktop: ${p.inputReader().readText()}")
            } else {
                println("org.freedesktop.portal.Desktop error: ${p.errorReader().readText()}")
            }
        }

        when (ime) {
            "fcitx5" -> {
                testResourcesDir.resolve(
                    "fcitx5",
                ).copyToRecursively(xdgConfigHome.resolve("fcitx5"), overwrite = false, followLinks = false)
                val fcitx5Log = newProcess("fcitx5", "--disable", "clipboard", "--verbose", "*=5").let { (p, log) -> log }
                newEnv["FCITX5_LOG_FILE"] = fcitx5Log.absolutePathString()
            }
            // Requires at least version 1.5.32 for proper Wayland integration
            "ibus" -> {
                val ibusTempDir = xdgRuntimeDir.resolve("test_ibus").createDirectory()
                val ibusAddressFile = ibusTempDir.resolve("ibus-addr").createFile() // suppress the IBus warning about the non-existing file
                val ibusSocketFile = ibusTempDir.resolve("ibus-socket")
                val ibusComponentPath = ibusTempDir.resolve("component").createDirectory()
                val ibusComponentFile = ibusComponentPath.resolve("ibus_test_engine.xml")
                val ibusEngineTmpCapsOutputFile = ibusTempDir.resolve("test-engine-caps-out.txt")
                val ibusEngineTmpContentTypeOutputFile = ibusTempDir.resolve("test-engine-content-type-out.txt")
                val ibusEngineTmpCursorLocationOutputFile = ibusTempDir.resolve("test-engine-cursor-location-out.txt")

                ibusComponentFile.writeText(generateIBusXmlFileContent(ibusTestEngineFile))

                newEnv["IBUS_ADDRESS_FILE"] = ibusAddressFile.absolutePathString()
                newEnv["IBUS_COMPONENT_PATH"] = "${ibusComponentPath.absolutePathString()}:/usr/share/ibus/component"

                runCommand("gsettings", "set", "org.freedesktop.ibus.general", "use-system-keyboard-layout", "true").let { (exitCode, p) ->
                    check(exitCode == 0) { "exit code $exitCode" }
                }

                newProcess(
                    "ibus",
                    "start",
                    "--verbose",
                    "--type=wayland",
                    "-a",
                    "unix:path=${ibusSocketFile.absolutePathString()}",
                    "--xim",
                    "--panel",
                    "disable",
                    "--emoji-extension",
                    "disable",
                    "--cache=none",
                    "--verbose",
                )
                waitUntil("IBus socket file exists") { ibusSocketFile.exists() }

                newProcess(ibusTestEngineFile.absolutePathString())

                runCommand("ibus", "address").let { (exitCode, p) ->
                    check(exitCode == 0) { "exit code $exitCode" }
                    val ibusAddress = p.inputReader().readText().trim()
                    newProcess(
                        "dbus-monitor",
                        "--address",
                        ibusAddress,
                        logStdOut = true,
                    )
                }

                newEnv["TEST_IBUS_ENGINE_CAPS_OUT_FILE"] = ibusEngineTmpCapsOutputFile.absolutePathString()
                newEnv["TEST_IBUS_ENGINE_CONTENT_TYPE_OUT_FILE"] = ibusEngineTmpContentTypeOutputFile.absolutePathString()
                newEnv["TEST_IBUS_ENGINE_CURSOR_LOCATION_OUT_FILE"] = ibusEngineTmpCursorLocationOutputFile.absolutePathString()

                tempFiles.add(ibusAddressFile)
                tempFiles.add(ibusSocketFile)
                tempFiles.add(ibusEngineTmpCapsOutputFile)
                tempFiles.add(ibusEngineTmpContentTypeOutputFile)
                tempFiles.add(ibusEngineTmpCursorLocationOutputFile)
                tempFiles.add(ibusComponentFile)
                tempFiles.add(ibusComponentPath)
                tempFiles.add(ibusTempDir)
            }
        }
//        ProcessBuilder("foot").also { pb ->
//            val env = pb.environment()
//            env.clear()
//            env.putAll(newEnv)
//            pb.start().waitFor()
//        }

        newEnv["TEST_IME"] = ime
        newEnv["TEST_RESOURCES_DIR"] = testResourcesDir.absolutePathString()
        newEnv["TEST_APP_DATA_SOURCE_CMD"] = testAppDataSourceCmd
        newEnv["KDOTOOL_CMD"] = kdotoolCmd

        // Work around the Ubuntu 22.04 apparmor issues: https://github.com/emersion/mako/issues/257
        val testMakoPath = homeTempDir.resolve("mako")
        Path.of("/usr/bin/mako").copyTo(testMakoPath)
        newEnv["TEST_MAKO_PATH"] = testMakoPath.absolutePathString()

        return newEnv
    }

    override fun close() {
        println("WaylandTestEnv close")
        val testsFailed = try {
            test?.state?.rethrowFailure()
            false
        } catch (_: Throwable) {
            true
        }
        test = null

        if (wmPid != null) {
            ProcessBuilder("kill", wmPid.toString()).start().waitFor()
        } else if (newEnv.containsKey("SWAYSOCK")) {
            runCommand("swaymsg", "exit").let { (exitCode, p) ->
                if (exitCode == 0) {
                    println("swaymsg exit: ${p.inputReader().readText()}")
                } else {
                    println("swaymsg exit error: ${p.errorReader().readText()}")
                }
            }
        }

        startedProcesses.reverse()
        for ((p, name) in startedProcesses) {
            val wasAlive = p.isAlive
            if (!wasAlive) {
                println("ERROR: $name is not alive")
            }
            if (name == "fcitx5") {
                ProcessBuilder("fcitx5-remote", "-e").start().waitFor()
            }

            p.toHandle().destroy()
            if (!wasAlive || testsFailed || name == "dbus-daemon") {
                val stderr = p.errorReader().readText()
                if (stderr.isNotBlank()) {
                    println("\n$name stderr:\n$stderr")
                }
            }
            p.destroy()
            p.waitFor()
        }
        startedProcesses.clear()

        for (f in tempFiles) {
            f.deleteIfExists()
        }
        if (!testsFailed) {
            for (logFile in logFiles) {
                logFile.deleteIfExists()
            }
            homeTempDir.toFile().deleteRecursively()
        }
    }
}

tasks.test {
    val otherBackends = backendsForOS(runTestsWithPlatform.os).filter { it != Backend.GTK && it != Backend.WAYLAND }
    configureTestTask(this, otherBackends)
}
