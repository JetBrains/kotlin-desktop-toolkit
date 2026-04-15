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
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.attribute.PosixFilePermissions
import kotlin.io.path.absolutePathString
import kotlin.io.path.copyTo
import kotlin.io.path.createDirectories
import kotlin.io.path.createDirectory
import kotlin.io.path.createFile
import kotlin.io.path.deleteIfExists
import kotlin.io.path.exists
import kotlin.io.path.name
import kotlin.io.path.readLines
import kotlin.io.path.readText
import kotlin.io.path.writeLines
import kotlin.io.path.writeText
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
version = (project.properties["version"] as? String)?.takeIf { it.isNotBlank() && it != "unspecified" } ?: "SNAPSHOT"

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
    runtimeOnly(gradleApi())

    // Use the Kotlin JUnit 5 integration.
    testImplementation("org.jetbrains.kotlin:kotlin-test-junit5")

    // Use the JUnit 5 integration.
    testImplementation(libs.junit.jupiter.engine)

    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
    implementation(kotlin("stdlib"))
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

@Serializable
data class RustTarget(
    @get:Input val platform: Platform,
    @get:Input val profile: String,
    @get:Input val backend: Backend,
)

val enabledPlatforms = crossCompilationSettings.enabled()

val profiles = listOf("dev", "release")

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
        dependsOn(buildNativeTask)
        downloadAngleTask?.let {
            dependsOn(downloadAngleTask)
            angleBinaries.setFrom(downloadAngleTask.map { it.binaries })
        }
        nativeLibrary = buildNativeTask.flatMap { it.libraryFile }
        targetDirectory = layout.buildDirectory.dir("native-${jarSuffixForPlatform(target.platform, target.backend)}-${target.profile}")
    }
}

val installCbindgenTask = tasks.register<InstallCargoProgram>("installCbindgen") {
    dependsOn(installRustTaskByPlatform[hostPlatform()]!!)
    cargoCommand = providers.cargoCommand().get()
    crate = "cbindgen"
    version = "0.29.0"
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

    private val homeTempDir by lazy { Files.createTempDirectory("test_home") }
    private val xdgDataHome by lazy { homeTempDir.resolve(".local/share").createDirectories() }

    private val ibusTempDir by lazy { Files.createTempDirectory("test_ibus") }
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
            "GTK_A11Y" to "none",
            "IBUS_ADDRESS_FILE" to ibusAddressFile.absolutePathString(),
            "IBUS_COMPONENT_PATH" to "${ibusComponentPath.absolutePathString()}:/usr/share/ibus/component",
            "TEST_IBUS_ENGINE_CAPS_OUT_FILE" to ibusEngineTmpCapsOutputFile.absolutePathString(),
            "TEST_IBUS_ENGINE_CONTENT_TYPE_OUT_FILE" to ibusEngineTmpContentTypeOutputFile.absolutePathString(),
            "TEST_IBUS_ENGINE_CURSOR_LOCATION_OUT_FILE" to ibusEngineTmpCursorLocationOutputFile.absolutePathString(),
            "LANG" to "en_US.UTF-8",
            "HOME" to homeTempDir.absolutePathString(),
            "XDG_DATA_HOME" to xdgDataHome.absolutePathString(),
            "XDG_RUNTIME_DIR" to homeTempDir.resolve("xdg_runtime_dir").createDirectory(
                PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")),
            ).absolutePathString(),
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

    private fun newProcess(vararg args: String, afterStart: (Process, Path) -> Boolean = { _, _ -> true }): Boolean {
        println("Running ${args.toList()}")
        val exeName = args.first().let {
            val p = Path.of(it)
            if (p.isAbsolute) {
                p.name
            } else {
                it
            }
        }
        val logFileStderr = Path.of(newEnv["HOME"]!!).resolve("$exeName-stderr.log")
        return ProcessBuilder(*args).also { pb ->
            val env = pb.environment()
            env.clear()
            env.putAll(newEnv)
            println(logFileStderr)
            pb.redirectError(ProcessBuilder.Redirect.to(logFileStderr.toFile()))
        }.start().let {
            check(it.isAlive)
            val ret = afterStart(it, logFileStderr)
            if (ret) {
                logFiles.add(logFileStderr)
                startedProcesses.add(Pair(it, args.first()))
            }
            ret
        }
    }

    fun run(
        test: Test,
        i3config: RegularFile,
        dbusConfigFile: RegularFile,
        dunstConfigFile: RegularFile,
        baseXSettingsDConfigFile: RegularFile,
        ibusTestEngineFile: RegularFile,
        testResourcesDir: Directory,
        headless: Boolean,
    ): Map<String, String> {
        this.test = test

        xdgDataHome
            .resolve("dbus-1/services")
            .createDirectories(PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
            .resolve("org.freedesktop.Notifications.service")
            .writeText(
                """[D-BUS Service]
Name=org.freedesktop.Notifications
Exec=/bin/true
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
            newProcess("Xephyr", testDisplay, "-screen", "3000x1500x24", "-dpi", "192", "-sw-cursor")
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
            newEnv["DBUS_SESSION_BUS_ADDRESS"] = p.inputReader().readLine()
            true
        }

        ProcessBuilder(
            "xdotool",
            "getmouselocation",
        ).also { pb ->
            val env = pb.environment()
            env.clear()
            env.putAll(newEnv)
            val startTime = TimeSource.Monotonic.markNow()
            while (true) {
                val p = pb.start()
                if (p.waitFor() == 0) {
                    break
                }
                if (startTime.elapsedNow() > 3.seconds) {
                    throw Error("Could not run xdotool: ${p.errorReader().readText()}")
                }
                Thread.sleep(10)
            }
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
            val startTime = TimeSource.Monotonic.markNow()
            while (true) {
                val p = pb.start()
                if (p.waitFor() == 0) {
                    break
                }
                if (startTime.elapsedNow() > 3.seconds) {
                    throw Error("Could not run setxkbmap: ${p.errorReader().readText()}")
                }
                Thread.sleep(10)
            }
        }

        // `gsettings` works with dconf and XDG Desktop Portal, and GTK uses them only on Wayland.
        // On X11, we need to have some xsettings daemon running.
        // We're using `xsettingsd` as a desktop-agnostic xsettings daemon.
        // E.g., Gnome used `gsd-xsettings` (gnome-settings-daemon) for this.
        var xSettingsDPid: String? = null
        val startTime = TimeSource.Monotonic.markNow()
        while (startTime.elapsedNow() < 10.seconds &&
            !newProcess("xsettingsd", "--config=$xSettingsDConfigFilePathString") { p, log ->
                xSettingsDPid = p.pid().toString()
                var ret = false
                while (p.isAlive) {
                    val lines = log.readLines()
                    if (lines.any { it.contains("Took ownership of selection") }) {
                        ret = true
                        newEnv["TEST_XSETTINGSD_LOG_FILE"] = log.absolutePathString()
                        break
                    }
                    if (startTime.elapsedNow() > 10.seconds) {
                        throw Error("Could not run xsettingsd:\n${lines.joinToString("\n")}")
                    }
                    Thread.sleep(10)
                }
                ret
            }
        ) {
            continue
        }

        ibusComponentFile.writeText(generateIBusXmlFileContent(ibusTestEngineFile.asFile))
        newProcess(
            "ibus-daemon",
            "-a",
            "unix:path=${ibusSocketFile.absolutePathString()}",
            "--verbose",
            "--panel",
            "disable",
            "--xim",
            "--cache=none",
        ) { _, _ ->
            val aliveCheckIntervalMs = 10L
            var aliveCheckTimeoutMs = 1000L
            while (!ibusSocketFile.exists() && aliveCheckTimeoutMs > 0) {
                Thread.sleep(aliveCheckIntervalMs)
                aliveCheckTimeoutMs -= aliveCheckIntervalMs
            }
            check(ibusSocketFile.exists()) { "${ibusSocketFile.absolutePathString()} does not exist" }
            true
        }

        newProcess(ibusTestEngineFile.asFile.absolutePath)

        newProcess("i3", "--shmlog-size=26214400", "-c", i3config.asFile.absolutePath)

        newEnv["GSK_RENDERER"] = "vulkan"
        newEnv["TEST_DUNST_CONFIG_FILE"] = dunstConfigFile.asFile.absolutePath
        newEnv["TEST_RESOURCES_DIR"] = testResourcesDir.asFile.absolutePath
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

val buildWaylandVirtualDevicesTestApp = tasks.register<Exec>(
    "buildWaylandVirtualDevicesTestApp-${buildPlatformRustTarget(runTestsWithPlatform)}",
) {
    val rustTarget = buildPlatformRustTarget(runTestsWithPlatform)
    dependsOn(installRustTaskByPlatform[runTestsWithPlatform]!!)
    inputs.files(nativeDir.rustWorkspaceFiles())
    workingDir = nativeDir.asFile
    executable = providers.cargoCommand().get()
    args = listOf(
        "build",
        "--example=wayland_virtual_devices",
        "--color=always",
        "--target=$rustTarget",
    )
    outputs.file(nativeDir.file("target/$rustTarget/debug/examples/wayland_virtual_devices"))
}

sourceSets {
    create("testGtk") {
        compileClasspath += sourceSets.main.get().output
        runtimeClasspath += sourceSets.main.get().output
    }
}

val testGtkImplementation by configurations.getting {
    extendsFrom(configurations.implementation.get())
}
val testGtkRuntimeOnly by configurations.getting

configurations["testGtkRuntimeOnly"].extendsFrom(configurations.runtimeOnly.get())

dependencies {
    testGtkImplementation("org.jetbrains.kotlin:kotlin-test-junit5")
    testGtkImplementation(libs.junit.jupiter.engine)
    testGtkImplementation("org.jetbrains.skiko:skiko-awt-runtime-$skikoTargetOs-$skikoTargetArch:$skikoVersion")
    testGtkImplementation("net.java.dev.jna:jna-platform:5.18.1")
    testGtkRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

val testGtk = tasks.register<Test>("testGtk") {
    description = "Test GTK implementation."
    group = "verification"

    testClassesDirs = sourceSets["testGtk"].output.classesDirs
    classpath = sourceSets["testGtk"].runtimeClasspath

    configureTestTask(this, listOf(Backend.GTK))

    if (backendsForOS(runTestsWithPlatform.os).contains(Backend.GTK)) {
        val x11TestEnvProvider = gradle.sharedServices.registerIfAbsent("x11TestEnv", X11TestEnv::class)
        usesService(x11TestEnvProvider)
        val testResourcesDir = layout.projectDirectory.dir("src/test/resources/linux")
        doFirst {
            val x11TestEnv = x11TestEnvProvider.get()
            try {
                val newEnv = x11TestEnv.run(
                    this@register,
                    i3config = testResourcesDir.file("i3_test_config"),
                    dbusConfigFile = testResourcesDir.file("dbus-session-conf.xml"),
                    dunstConfigFile = testResourcesDir.file("dunstrc.conf"),
                    baseXSettingsDConfigFile = testResourcesDir.file("xsettingsd.conf"),
                    ibusTestEngineFile = testResourcesDir.file("ibus_test_engine.py"),
                    testResourcesDir = testResourcesDir,
                    headless = true,
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

val testWaylandImplementation by configurations.getting {
    extendsFrom(configurations.implementation.get())
}
val testWaylandRuntimeOnly by configurations.getting

configurations["testWaylandRuntimeOnly"].extendsFrom(configurations.runtimeOnly.get())

dependencies {
    testWaylandImplementation("org.jetbrains.kotlin:kotlin-test-junit5")
    testWaylandImplementation(libs.junit.jupiter.engine)
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
        dependsOn(buildWaylandVirtualDevicesTestApp)
        val waylandTestEnvProvider = gradle.sharedServices.registerIfAbsent("waylandTestEnv", WaylandTestEnv::class)
        usesService(waylandTestEnvProvider)
        val testResourcesDir = layout.projectDirectory.dir("src/test/resources/linux")
        val runVirtualDevicesCmd = buildWaylandVirtualDevicesTestApp.map { it.outputs.files.first() }.get().absolutePath
        doFirst {
            val waylandTestEnv = waylandTestEnvProvider.get()
            try {
                val newEnv = waylandTestEnv.run(
                    this@register,
                    swayConfig = testResourcesDir.file("sway_test_config"),
                    dbusConfigFile = testResourcesDir.file("dbus-session-conf.xml"),
                    testResourcesDir = testResourcesDir,
                    runVirtualDevicesCmd = listOf(runVirtualDevicesCmd),
                    headless = true,
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
        jvmArgs("--enable-preview", "--enable-native-access=ALL-UNNAMED")
        useJUnitPlatform()

        val getLibFolderForBackend: Map<Backend, Provider<Directory>> = buildMap {
            for (backend in backends) {
                val collectNativeArtifactsTask = collectNativeArtifactsTaskByTarget[RustTarget(runTestsWithPlatform, "dev", backend)]
                if (collectNativeArtifactsTask != null) {
                    dependsOn(collectNativeArtifactsTask)
                    put(backend, collectNativeArtifactsTask.flatMap { it.targetDirectory })
                }
            }
        }

        val logFile = layout.buildDirectory.file("test-logs/desktop_native.log")
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

    private val homeTempDir by lazy { Files.createTempDirectory("test_home") }
    private val xdgDataHome by lazy { homeTempDir.resolve(".local/share").createDirectories() }

    private val newEnv by lazy {
        mutableMapOf(
            "XDG_CURRENT_DESKTOP" to "GNOME",
            "SWAYSOCK" to homeTempDir.resolve("sway-socket").absolutePathString(),
            "LANG" to "en_US.UTF-8",
            "HOME" to homeTempDir.absolutePathString(),
            "XDG_DATA_HOME" to xdgDataHome.absolutePathString(),
            "XDG_RUNTIME_DIR" to homeTempDir.resolve("xdg_runtime_dir").createDirectory(
                PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")),
            ).absolutePathString(),
            "XDG_SESSION_TYPE" to "wayland",
//            "WAYLAND_DEBUG" to "1",
        )
    }

    private fun newProcess(
        vararg args: String,
        getAdditionalEnvs: ((Map<String, String>) -> Map<String, String>)? = null,
        afterStart: ((Process) -> Unit)? = null,
    ) {
        println("Running ${args.toList()}")
        val exeName = args.first().let {
            val p = Path.of(it)
            if (p.isAbsolute) {
                p.name
            } else {
                it
            }
        }
        ProcessBuilder(*args).also { pb ->
            val env = pb.environment()
            val additionalEnvs = getAdditionalEnvs?.invoke(env)
            env.clear()
            env.putAll(newEnv)
            additionalEnvs?.let { env.putAll(it) }

            val logFileStderr = Path.of(newEnv["HOME"]!!).resolve("$exeName-stderr.log").also {
                println(it)
                logFiles.add(it)
            }
            pb.redirectError(ProcessBuilder.Redirect.to(logFileStderr.toFile()))
        }.start().let {
            check(it.isAlive)
            afterStart?.invoke(it)
            startedProcesses.add(Pair(it, exeName))
        }
    }

    fun run(
        test: Test,
        swayConfig: RegularFile,
        dbusConfigFile: RegularFile,
        testResourcesDir: Directory,
        runVirtualDevicesCmd: List<String>,
        headless: Boolean,
    ): Map<String, String> {
        println("WaylandTestEnv run")
        this.test = test

        xdgDataHome
            .resolve("dbus-1/services")
            .createDirectories(PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
            .resolve("org.freedesktop.Notifications.service")
            .writeText(
                """[D-BUS Service]
Name=org.freedesktop.Notifications
Exec=/bin/true
""",
            )

        val testSwayOut = Files.createTempDirectory("test_sway_out")
        val testSwayDisplayName = testSwayOut.resolve("display_name")
        val testSwayConfig = Files.createTempFile("test_sway_config", "")
        testSwayConfig.writeLines(
            swayConfig.asFile.readLines() + listOf(
                $$"""exec echo -n "$WAYLAND_DISPLAY" > $${testSwayDisplayName}.tmp && mv $${testSwayDisplayName}.tmp $$testSwayDisplayName""",
            ),
        )

        newProcess(
            "sway",
//            "--debug",
//            "--verbose",
            "--config",
            testSwayConfig.absolutePathString(),
            getAdditionalEnvs = { env ->
                buildMap {
                    if (headless) {
                        put("WLR_BACKENDS", "headless")
                    } else {
                        val orgXdgRuntimeDir = env["XDG_RUNTIME_DIR"]!!
                        val orgWaylandDisplay = env["WAYLAND_DISPLAY"]!!
                        put("WAYLAND_DISPLAY", Path.of(orgXdgRuntimeDir).resolve(orgWaylandDisplay).absolutePathString())
                    }
                }
            },
            afterStart = { p ->
                val startTime = TimeSource.Monotonic.markNow()
                while (true) {
                    if (testSwayDisplayName.exists()) {
                        break
                    }
                    if (startTime.elapsedNow() > 3.seconds) {
                        throw Error("Could not run sway: ${p.errorReader().readText()}")
                    }
                    Thread.sleep(10)
                }
            },
        )

        val testDisplay = testSwayDisplayName.readText().trim()
        testSwayDisplayName.deleteIfExists()
        testSwayOut.deleteIfExists()
        newEnv["WAYLAND_DISPLAY"] = testDisplay

        newProcess(
            "dbus-daemon",
            "--config-file=${dbusConfigFile.asFile.absolutePath}",
            "--nofork",
            "--nopidfile",
            "--nosyslog",
            "--print-address",
        ) {
            newEnv["DBUS_SESSION_BUS_ADDRESS"] = it.inputReader().readLine()
        }

        newProcess(*runVirtualDevicesCmd.toTypedArray())

//        ProcessBuilder("foot").also { pb ->
//            val env = pb.environment()
//            env.clear()
//            env.putAll(newEnv)
//            pb.start().waitFor()
//        }

        newEnv["TEST_RESOURCES_DIR"] = testResourcesDir.asFile.absolutePath

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

        startedProcesses.reverse()
        for ((p, name) in startedProcesses) {
            val wasAlive = p.isAlive
            if (!wasAlive) {
                println("ERROR: $name is not alive")
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
