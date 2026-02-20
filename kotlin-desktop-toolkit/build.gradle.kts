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
import org.jetbrains.desktop.buildscripts.getBooleanProperty
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

    fun additionalCargoArgs(): List<String> = when (this) {
        GTK -> listOf("--features", "desktop-gtk/enabled")
        else -> emptyList()
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
        Os.LINUX -> mutableListOf(Backend.WAYLAND)
    }
    if (getBooleanProperty(project, "enableGtkBackend")) {
        backends.add(Backend.GTK)
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
                    additionalCargoArgs = backend.additionalCargoArgs()
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
        targetDirectory =
            layout.buildDirectory.dir(
                "native-${target.backend.normalizedName()}-${buildPlatformRustTarget(target.platform)}-${target.profile}",
            )
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
    val osName = platform.os.normalizedName
    val archName = when (platform.arch) {
        Arch.aarch64 -> "arm64"
        Arch.x86_64 -> "x64"
    }
    val backend = backend.normalizedName()
    return "$backend-$osName-$archName"
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

val collectNativeArtifactsTaskByTargetForTests = collectNativeArtifactsTaskByTarget.filterKeys { rustTarget ->
    if (rustTarget.profile == "dev" && rustTarget.platform == runTestsWithPlatform) {
        if (getBooleanProperty(project, "enableGtkBackend")) {
            rustTarget.backend == Backend.GTK
        } else {
            true
        }
    } else {
        false
    }
}

collectNativeArtifactsTaskByTargetForTests.forEach { (rustTarget, task) ->
    artifacts.add(nativeConsumable.name, task.flatMap { it.targetDirectory }) {
        builtBy(task) // redundant because of the flatMap usage above, but if you want to be sure, you can specify that
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
    val allAdditionalArgs = backendsForOS(platform.os).flatMap { it.additionalCargoArgs() }
    tasks.register<ClippyTask>("clippyCheckFor${platform.name()}") {
        checkOnly = true
        workingDir = nativeDir.asFile
        cargoCommand = providers.cargoCommand()
        rustTarget = buildPlatformRustTarget(platform)
        additionalArgs = allAdditionalArgs
        dependsOn(installRustTask)
    }
}

val clippyFixTasks = installRustTaskByPlatform.map { (platform, installRustTask) ->
    val allAdditionalArgs = backendsForOS(platform.os).flatMap { it.additionalCargoArgs() }
    tasks.register<ClippyTask>("clippyFixFor${platform.name()}") {
        workingDir = nativeDir.asFile
        cargoCommand = providers.cargoCommand()
        rustTarget = buildPlatformRustTarget(platform)
        additionalArgs = allAdditionalArgs
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

abstract class X11TestEnv :
    BuildService<BuildServiceParameters.None>,
    AutoCloseable {
    private val testDisplay = ":65"
    private var xServerProcess: Process? = null
    private var windowManagerProcess: Process? = null
    private var dbusProcess: Process? = null
    private var i3configFile: File? = null

    fun run(i3config: String, headless: Boolean): Map<String, String> {
        val i3configFile = File.createTempFile("test_i3config", "").also {
            it.writeText(i3config)
            this.i3configFile = it
        }
        dbusProcess = ProcessBuilder("dbus-daemon", "--session", "--nofork", "--nopidfile", "--nosyslog", "--print-address").start().also {
            assert(it.isAlive)
        }
        val dbusAddress = dbusProcess!!.inputReader().readLine()

        xServerProcess = if (headless) {
            ProcessBuilder("Xvfb", "-ac", "-screen", "0", "2000x1000x24", testDisplay)
        } else {
            ProcessBuilder("Xephyr", testDisplay, "-screen", "2000x1000x24", "-sw-cursor")
        }.start().also {
            assert(it.isAlive)
        }

        val newEnv = mapOf(
            "DBUS_SESSION_BUS_ADDRESS" to dbusAddress,
            "DISPLAY" to testDisplay,
            "GDK_BACKEND" to "x11",
            "LANG" to "en_US.UTF-8",
            "XDG_SESSION_TYPE" to "x11",
        )

        windowManagerProcess = ProcessBuilder("i3", "-c", i3configFile.absolutePath).also {
            val env = it.environment()
            env.clear()
            env.putAll(newEnv)
        }.start().also {
            assert(it.isAlive)
        }

        return newEnv
    }

    override fun close() {
        dbusProcess?.let {
            it.destroy()
            it.waitFor()
        }
        dbusProcess = null

        windowManagerProcess?.let {
            it.destroy()
            it.waitFor()
        }
        windowManagerProcess = null

        i3configFile?.delete()

        xServerProcess?.let {
            it.destroy()
            it.waitFor()
        }
        xServerProcess = null
    }
}

tasks.test {
    if (hostOs() == Os.LINUX) {
        val x11TestEnvProvider = gradle.sharedServices.registerIfAbsent("x11TestEnv", X11TestEnv::class)
        usesService(x11TestEnvProvider)
        doFirst {
            val x11TestEnv = x11TestEnvProvider.get()
            val i3config = """# i3 config file (v4)
font pango:monospace 8
focus_follows_mouse no
new_window none
new_float none
default_border none
for_window [class="^.*"] border pixel 0, floating disable
exec setxkbmap -layout us -variant intl
#bindsym Mod1+Return exec xterm
            """
            val newEnv = x11TestEnv.run(i3config, headless = true)
            println(newEnv)
            setEnvironment(newEnv)
        }
    }
    systemProperty("junit.jupiter.testmethod.order.default", "org.junit.jupiter.api.MethodOrderer\$Random")
    systemProperty("junit.jupiter.testclass.order.default", "org.junit.jupiter.api.ClassOrderer\$Random")

    jvmArgs(
        "--enable-preview",
        "--enable-native-access=ALL-UNNAMED",
    )
    useJUnitPlatform()
    testLogging {
        showStandardStreams = true
    }

    if (collectNativeArtifactsTaskByTargetForTests.isEmpty()) {
        enabled = false
    } else {
        dependsOn(collectNativeArtifactsTaskByTargetForTests.values)
        val logFile = layout.buildDirectory.file("test-logs/desktop_native.log")

        val targetDirectories = collectNativeArtifactsTaskByTargetForTests.map { e ->
            e.value.flatMap { task -> task.targetDirectory }.get()
        }
        // All the directories should be the same in this case
        val libFolder = targetDirectories.map { it.asFile.absolutePath }.toSet().single()

        jvmArgumentProviders.add(
            CommandLineArgumentProvider {
                listOf(
                    "-Dkdt.library.folder.path=$libFolder",
                    "-Dkdt.debug=true",
                    "-Dkdt.native.log.path=${logFile.get().asFile.absolutePath}",
                )
            },
        )
    }

    testLogging {
        exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
        events("failed")
        events("passed")
        events("skipped")
    }

    // We run every test class in a separate JVM
    forkEvery = 1
    maxParallelForks = 1
}
