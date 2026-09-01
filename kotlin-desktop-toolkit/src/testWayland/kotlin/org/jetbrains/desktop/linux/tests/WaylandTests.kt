package org.jetbrains.desktop.linux.tests

import com.squareup.moshi.JsonClass
import com.squareup.moshi.Moshi
import com.squareup.moshi.kotlin.reflect.KotlinJsonAdapterFactory
import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.ApplicationConfig
import org.jetbrains.desktop.linux.Color
import org.jetbrains.desktop.linux.ColorSchemeValue
import org.jetbrains.desktop.linux.DataSource
import org.jetbrains.desktop.linux.DesktopSetting
import org.jetbrains.desktop.linux.DesktopTitlebarAction
import org.jetbrains.desktop.linux.DragAndDropAction
import org.jetbrains.desktop.linux.DragAndDropQueryData
import org.jetbrains.desktop.linux.DragAndDropQueryResponse
import org.jetbrains.desktop.linux.DragIconParams
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.EventSerial
import org.jetbrains.desktop.linux.FileDialog
import org.jetbrains.desktop.linux.FontAntialiasingValue
import org.jetbrains.desktop.linux.FontHintingValue
import org.jetbrains.desktop.linux.FontRgbaOrderValue
import org.jetbrains.desktop.linux.KeyCode
import org.jetbrains.desktop.linux.KeyModifiers
import org.jetbrains.desktop.linux.KeySym
import org.jetbrains.desktop.linux.KotlinDesktopToolkit
import org.jetbrains.desktop.linux.LogLevel
import org.jetbrains.desktop.linux.LogicalPixelsInt
import org.jetbrains.desktop.linux.LogicalPoint
import org.jetbrains.desktop.linux.LogicalRect
import org.jetbrains.desktop.linux.LogicalSize
import org.jetbrains.desktop.linux.MouseButton
import org.jetbrains.desktop.linux.PhysicalSize
import org.jetbrains.desktop.linux.PointerShape
import org.jetbrains.desktop.linux.RenderingMode
import org.jetbrains.desktop.linux.Scale
import org.jetbrains.desktop.linux.ShowNotificationParams
import org.jetbrains.desktop.linux.SoftwareDrawData
import org.jetbrains.desktop.linux.StartDragAndDropParams
import org.jetbrains.desktop.linux.SupportedActionsForMime
import org.jetbrains.desktop.linux.TextInputContentHint
import org.jetbrains.desktop.linux.TextInputContentPurpose
import org.jetbrains.desktop.linux.TextInputContext
import org.jetbrains.desktop.linux.Window
import org.jetbrains.desktop.linux.WindowCapabilities
import org.jetbrains.desktop.linux.WindowDecorationMode
import org.jetbrains.desktop.linux.WindowFrame
import org.jetbrains.desktop.linux.WindowFrameTiling
import org.jetbrains.desktop.linux.WindowId
import org.jetbrains.desktop.linux.WindowParams
import org.jetbrains.desktop.linux.WindowResizeEdge
import org.jetbrains.desktop.linux.XdgDesktopContrast
import org.jetbrains.desktop.linux.XdgDesktopReducedMotion
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertIterableEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Disabled
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Timeout
import org.junit.jupiter.api.assertInstanceOf
import org.junit.jupiter.api.assertNotNull
import org.junit.jupiter.api.assertNull
import org.junit.jupiter.api.assertThrows
import org.junit.jupiter.api.condition.EnabledIfEnvironmentVariable
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import org.junit.jupiter.api.fail
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import java.net.URI
import java.net.URLEncoder
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.nio.file.Files
import java.nio.file.Path
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ExecutionException
import java.util.concurrent.Future
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread
import kotlin.io.path.absolutePathString
import kotlin.io.path.createDirectories
import kotlin.io.path.deleteIfExists
import kotlin.io.path.exists
import kotlin.io.path.fileSize
import kotlin.io.path.outputStream
import kotlin.io.path.readBytes
import kotlin.io.path.readLines
import kotlin.io.path.readText
import kotlin.io.path.writeText
import kotlin.math.round
import kotlin.math.roundToInt
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Duration.Companion.seconds
import kotlin.time.TimeSource
import kotlin.time.measureTime
import kotlin.use
import org.jetbrains.skia.Bitmap as SkBitmap
import org.jetbrains.skia.Canvas as SkCanvas
import org.jetbrains.skia.Color as SkColor
import org.jetbrains.skia.ColorAlphaType as SkColorAlphaType
import org.jetbrains.skia.ColorSpace as SkColorSpace
import org.jetbrains.skia.ColorType as SkColorType
import org.jetbrains.skia.Image as SkImage
import org.jetbrains.skia.ImageInfo as SkImageInfo
import org.jetbrains.skia.Paint as SkPaint
import org.jetbrains.skia.Rect as SkRect
import org.jetbrains.skia.Surface as SkSurface

internal data class GlobalPosition(
    val x: LogicalPixelsInt,
    val y: LogicalPixelsInt,
) {
    companion object {
        val Zero: GlobalPosition = GlobalPosition(LogicalPixelsInt.Zero, LogicalPixelsInt.Zero)
    }
}

internal data class GlobalRect(
    val pos: GlobalPosition,
    val size: LogicalSize,
)

internal data class WmVersion(
    val name: String,
    val major: Int,
    val minor: Int,
    val patch: Int,
    val checkWindowCapabilities: (WindowCapabilities) -> Unit,
    val checkWindowScale: (WmOutputState, Scale) -> Unit,
)

internal data class WmOutputState(val scale: Float)

internal data class WmWindowState(
    val output: WmOutputState,
    val title: String,
    val clientArea: GlobalRect,
    val padding: WindowFrame.Padding,
)

internal class SwayWm {
    @Suppress("DestructuringDeclaration")
    private fun getWindowStateImpl(predicate: (SwayIpcWindow) -> Boolean): WmWindowState? {
        for (output in getSwayTree().nodes) {
            if (output.current_workspace == null) {
                continue
            }
            for (workspace in output.nodes) {
                for (window in workspace.nodes + workspace.floating_nodes) {
                    if (predicate(window)) {
                        return swayWindowState(output, window)
                    }
                }
            }
        }
        return null
    }

    private fun getWindowStateHelper(predicate: (SwayIpcWindow) -> Boolean): WmWindowState? {
        return when (val state = getWindowStateImpl(predicate)) {
            null -> {
                Thread.sleep(100)
                getWindowStateImpl(predicate)
            }
            else -> state
        }
    }

    fun getWindowState(title: String): WmWindowState {
        return getWindowStateHelper { it.name == title } ?: fail("Cannot find window with title $title")
    }

    fun getVersion(): WmVersion {
        val swayVersion = getSwayVersion()

        val checkWindowCapabilities: (WindowCapabilities) -> Unit =
            if (swayVersion.major > 1 || swayVersion.minor > 11 || (swayVersion.minor == 11 && swayVersion.patch > 0)) {
                { capabilities ->
                    assertFalse(capabilities.windowMenu)
                    assertFalse(capabilities.maximize)
                    assertTrue(capabilities.fullscreen)
                    assertFalse(capabilities.minimize)
                }
            } else if (swayVersion.major == 1 && (swayVersion.minor == 10 || (swayVersion.minor == 11 && swayVersion.patch == 0))) {
                // See https://github.com/swaywm/sway/commit/516a3de4ca6c2378b875f62ffa6008d1cfa0cba9
                // and https://github.com/swaywm/sway/commit/c5456be7506adece2cdf922ed6d919db597944ab
                { capabilities ->
                    assertTrue(capabilities.windowMenu)
                    assertTrue(capabilities.maximize)
                    assertFalse(capabilities.fullscreen)
                    assertFalse(capabilities.minimize)
                }
            } else {
                { capabilities ->
                    assertTrue(capabilities.windowMenu)
                    assertTrue(capabilities.maximize)
                    assertTrue(capabilities.fullscreen)
                    assertTrue(capabilities.minimize)
                }
            }

        // https://github.com/swaywm/sway/commit/9162b536f69cb69466fb4fcfa24d282fa54b122b
        val checkWindowScale: (WmOutputState, Scale) -> Unit = if (swayVersion.major > 1 || swayVersion.minor > 8) {
            { output, scale ->
                assertEquals(output.scale, scale.rawScale.toFloat())
            }
        } else {
            { output, scale ->
                assertEquals(round(output.scale), scale.rawScale.toFloat())
            }
        }

        return WmVersion(
            name = swayVersion.variant,
            major = swayVersion.major,
            minor = swayVersion.minor,
            patch = swayVersion.patch,
            checkWindowCapabilities = checkWindowCapabilities,
            checkWindowScale = checkWindowScale,
        )
    }

    private fun runSwayMsg(title: String, cmd: String) {
        val command = listOf("swaymsg", "[title=\"$title\"] $cmd")
        waitUntilEq(true, waitStepMs = 100) {
            val output = runCommandWithOutput(command)?.decodeToString()
            output?.contains("No matching node") == false
        }
    }

    fun tileWindows(windowTitles: List<String>) {
        for (title in windowTitles) {
            runSwayMsg(title, "floating disable")
        }
    }

    fun moveWindow(title: String, pos: GlobalPosition) {
        runSwayMsg(title, "move position ${pos.x.rawLogical} ${pos.y.rawLogical}")
    }

    fun fullScreenWindow(title: String) {
        runSwayMsg(title, "fullscreen enable")
    }

    fun unsetFullScreenWindow(title: String) {
        runSwayMsg(title, "fullscreen disable")
    }

    fun closeWindow(title: String) {
        runSwayMsg(title, "kill")
    }

    fun focusWindow(title: String) {
        runSwayMsg(title, "focus")
    }

    fun closeFocusedWindow() {
        runCommand(listOf("swaymsg", "kill"))
    }

    fun focusWindowWithTitle(title: String) {
        runSwayMsg(title, "focus")
    }

    companion object {
        @Suppress("PropertyName")
        @JsonClass(generateAdapter = true)
        private data class SwayVersionModel(
            val human_readable: String,
            val variant: String,
            val major: Int,
            val minor: Int,
            val patch: Int,
            val loaded_config_file_name: String,
        )

        private fun getSwayVersion(): SwayVersionModel {
//            log("getSwayVersion")
            val json = runCommandWithOutput(listOf("swaymsg", "--raw", "-t", "get_version"))!!.decodeToString()
//            log(json)
            val moshi: Moshi = Moshi.Builder().addLast(KotlinJsonAdapterFactory()).build()
            val jsonAdapter = moshi.adapter(SwayVersionModel::class.java)
            return jsonAdapter.fromJson(json)!!
        }

        private fun getSwayTree(): SwayIpcRoot {
            val outputsJson = runCommandWithOutput(listOf("swaymsg", "--raw", "-t", "get_outputs"))!!.decodeToString()
            log(outputsJson)
//            log("getSwayTree")
            val json = runCommandWithOutput(listOf("swaymsg", "--raw", "-t", "get_tree"))!!.decodeToString()
            log(json)
            val moshi: Moshi = Moshi.Builder().addLast(KotlinJsonAdapterFactory()).build()
            val jsonAdapter = moshi.adapter(SwayIpcRoot::class.java).lenient()
            return jsonAdapter.fromJson(json)!!
        }

        private data class IdleInhibitors(
            /** Can be "enabled" or "none" */
            val application: String,

            /** Can be "focus", "fullscreen", "open", "visible" or "none" */
            val user: String,
        )

        private data class Rect(
            val x: Int,
            val y: Int,
            val width: Int,
            val height: Int,
        ) {
            fun toLogicalSize(): LogicalSize {
                return LogicalSize(width = LogicalPixelsInt(width), height = LogicalPixelsInt(height))
            }
        }

        private data class SwayIpcRoot(val nodes: List<SwayIpcOutput>)

        // From https://raw.githubusercontent.com/swaywm/sway/refs/tags/1.7/sway/sway-ipc.7.scd
        @Suppress("PropertyName")
        private data class SwayIpcOutput(
            /** The internal unique ID for this node */
            val id: Int,

            /** The name of the node such as the output name or window title. */
            val name: String,

            /** The node type. It can be "root", "output", "workspace", "con", or "floating_con" */
            val type: String,

            /** Whether this output is active/enabled */
            val active: Boolean = false,

            /** The scale currently in use on the output or -1 for disabled outputs */
            val scale: Float = -1f,

            /** The workspace currently visible on the output or null for disabled outputs */
            val current_workspace: String?,

            /** The bounds for the output consisting of x, y, width, and height */
            val rect: Rect,

            val nodes: List<SwayIpcWorkspace>,
        )

        // From https://raw.githubusercontent.com/swaywm/sway/refs/tags/1.7/sway/sway-ipc.7.scd
        @Suppress("PropertyName", "SpellCheckingInspection")
        private data class SwayIpcWorkspace(
            /** The internal unique ID for this node */
            val id: Int,

            /** The name of the node such as the output name or window title. */
            val name: String,

            /** The node type. It can be "root", "output", "workspace", "con", or "floating_con" */
            val type: String,

            /** The node's layout. It can either be "splith", "splitv", "stacked", "tabbed", or "output" */
            val layout: String,

            /** The node's orientation. It can be "vertical", "horizontal", or "none" */
            val orientation: String,

            /** The absolute geometry of the node. The window decorations are excluded from this, but borders are included. */
            val rect: Rect,

            /** Whether the node is currently focused by the default seat (seat0) */
            val focused: Boolean,

            /** Array of child node IDs in the current focus order */
            val focus: List<Int>,

            /** The tiling children nodes for the node */
            val nodes: List<SwayIpcWindow>,

            /** The floating children nodes for the node */
            val floating_nodes: List<SwayIpcWindow>,
        )

        // From https://raw.githubusercontent.com/swaywm/sway/refs/tags/1.7/sway/sway-ipc.7.scd
        @Suppress("PropertyName", "SpellCheckingInspection", "GrazieInspection")
        private data class SwayIpcWindow(
            /** The internal unique ID for this node */
            val id: Int,

            /** The name of the node such as the output name or window title. */
            val name: String,

            /** The node type. It can be "root", "output", "workspace", "con", or "floating_con" */
            val type: String,

            /** The border style for the node. It can be "normal", "none", "pixel", or "csd" */
            val border: String,

            /** Number of pixels used for the border width */
            val current_border_width: Int,

            /** The node's layout. It can either be "splith", "splitv", "stacked", "tabbed", or "output" */
            val layout: String,

            /** The node's orientation. It can be "vertical", "horizontal", or "none" */
            val orientation: String,

            /** The percentage of the node's parent that it takes up or null for the root and other special nodes such as the scratchpad */
            val percent: Float?,

            /** The absolute geometry of the node. The window decorations are excluded from this, but borders are included. */
            val rect: Rect,

            /** The geometry of the content inside the node. These coordinates are relative to the node itself.
             * Window decorations and borders are outside the window_rect
             */
            val window_rect: Rect,

            /** The geometry of the decorations for the node relative to the parent node */
            val deco_rect: Rect,

            /** The natural geometry of the contents if it were to size itself */
            val geometry: Rect,

            /** Whether the node or any of its descendants has the urgent hint set.
             * Note: This may not exist when compiled without xwayland support
             */
            val urgent: Boolean?,

            /** Whether the node is sticky (shows on all workspaces) */
            val sticky: Boolean,

            /** List of marks assigned to the node */
            val marks: List<String>,

            /** Whether the node is currently focused by the default seat (seat0) */
            val focused: Boolean,

            /** Floating state of container. Can be either "auto_off" or "user_on" */
            val floating: String?,

            /** (Only windows) For an xdg-shell window, the name of the application, if set. Otherwise, null */
            val app_id: String?,

            /** (Only windows) The PID of the application that owns the window */
            val pid: Int?,

            /** (Only windows) Whether the node is visible */
            val visible: Boolean?,

            /** (Only windows) The shell of the window, such as xdg_shell or xwayland */
            val shell: String?,

            /** (Only windows) Whether the window is inhibiting the idle state */
            val inhibit_idle: Boolean?,

            /** (Only windows) An object containing the state of the application and user idle inhibitors. */
            val idle_inhibitors: IdleInhibitors?,
        )

        private fun swayWindowState(output: SwayIpcOutput, window: SwayIpcWindow): WmWindowState {
            val clientAreaTopLeftGlobalPosition = GlobalPosition(
                LogicalPixelsInt((window.rect.x + window.window_rect.x)),
                LogicalPixelsInt((window.rect.y + window.window_rect.y)),
            )
            val padding = WindowFrame.Padding(
                left = LogicalPixelsInt(window.window_rect.x),
                top = LogicalPixelsInt(if (window.window_rect.y == 0) window.deco_rect.height else window.window_rect.y),
                right = LogicalPixelsInt(window.rect.width - window.window_rect.width - window.window_rect.x),
                bottom = LogicalPixelsInt(window.rect.height - window.window_rect.height - window.window_rect.y),
            )

            return WmWindowState(
                output = WmOutputState(output.scale),
                title = window.name,
                clientArea = GlobalRect(clientAreaTopLeftGlobalPosition, window.window_rect.toLogicalSize()),
                padding = padding,
            )
        }
    }
}

private val testStart by lazy { TimeSource.Monotonic.markNow() }
private val dateTimeFormatter = DateTimeFormatter.ofPattern("yyyyMMdd HH:mm:ss.SSS")

private fun withTimestamp(message: String): String {
    val time = LocalDateTime.now().format(dateTimeFormatter)
    val elapsedTime = testStart.elapsedNow().toString()
    return "$time ($elapsedTime): $message"
}

private fun log(message: String) {
    println(withTimestamp(message))
}

private fun runCommandImpl(command: List<String>, timeout: Duration = 5.seconds): Result<Path> {
    log("runCommand: $command")
    val outputFile = Files.createTempFile("${command.first()}-output", ".log")
    val proc = ProcessBuilder(command).redirectOutput(ProcessBuilder.Redirect.to(outputFile.toFile())).start()
    if (!proc.waitFor(timeout.inWholeMilliseconds, TimeUnit.MILLISECONDS)) {
        fail(withTimestamp("Timed out waiting for $command to finish"))
    }

    val exitValue = proc.exitValue()
    if (exitValue != 0) {
        val stderr = proc.errorStream.readAllBytes().decodeToString()
        val stdout = outputFile.readBytes().decodeToString()
        outputFile.deleteIfExists()
        return Result.failure(Error("$command failed ($exitValue), stderr=$stderr, stdout=$stdout"))
    }
    return Result.success(outputFile)
}

private fun runCommandWithOutput(command: List<String>, timeout: Duration = 5.seconds): ByteArray? {
    val result = runCommandImpl(command, timeout)
    result.exceptionOrNull()?.also {
        log(it.toString())
    }
    return result.getOrNull()?.let {
        val data = it.readBytes()
        it.deleteIfExists()
        data
    }
}

private fun runCommand(command: List<String>, timeout: Duration = 5.seconds) {
    runCommandImpl(command, timeout).getOrThrow().deleteIfExists()
}

private fun <T> waitUntilEq(
    expectedValue: T,
    timeout: Duration = 5.seconds,
    waitStepMs: Long = 10,
    msg: String? = null,
    actualValueGetter: () -> T,
) {
    val startTime = TimeSource.Monotonic.markNow()
    var actualValue = actualValueGetter()
    while (startTime.elapsedNow() < timeout) {
        if (actualValue == expectedValue) {
            return
        }
        Thread.sleep(waitStepMs)
        actualValue = actualValueGetter()
    }
    val msg = msg?.let { ", message: $it" } ?: ""
    fail(withTimestamp("waitUntilEq timed out: Expected: $expectedValue, actual: $actualValue$msg"))
}

internal data class TestAppData(
    val readLastLine: (Duration) -> String?,
    val windowTitle: String,
)

private class TestApp(private val cmd: List<String>) {
    private fun createProcessBuilder(): ProcessBuilder {
        log("Running test app")
        return ProcessBuilder(cmd).also {
            val env = it.environment()
            env.remove("GTK_DEBUG")
            env.remove("GDK_DEBUG")
            env.remove("LD_LIBRARY_PATH")
            env.remove("WAYLAND_DEBUG")
            env["GSK_RENDERER"] = "cairo"
            env["GTK_A11Y"] = "none"
        }
    }

    fun run(windowTitle: String, block: (TestAppData) -> Unit) {
        val outputFile = Files.createTempFile("linux_test_app_output", "log")
        val process = createProcessBuilder()
            .redirectOutput(ProcessBuilder.Redirect.to(outputFile.toFile()))
            .start()
        AutoCloseable {
            if (process.isAlive) {
                process.toHandle().destroy()
                process.waitFor(5, TimeUnit.SECONDS)
            } else {
                log("Test app process stopped by itself")
            }
            process.errorStream.readAllBytes().decodeToString().also {
                if (it.isNotEmpty()) {
                    log("Test app error output: $it")
                }
            }
            process.destroy()
            log("Test app full output:\n${outputFile.readText()}")
            outputFile.deleteIfExists()
            log("Test app closed")
        }.use {
            waitUntilEq(true) { !process.isAlive || outputFile.fileSize() > 0 }
            assertTrue(process.isAlive)

            val lines = outputFile.readLines()
            assertArrayEquals(arrayOf("window created"), lines.toTypedArray())
            log("Test app ready")

            val lastLinesCount = mutableListOf(1)
            val readLastLine = { timeout: Duration ->
                var ret: String? = null
                val retryIntervalMs = 10L
                var retryTimes = timeout.inWholeMilliseconds / retryIntervalMs
                while (retryTimes > 0) {
                    val lines = outputFile.readLines()
                    if (lines.size != lastLinesCount.last()) {
                        log("Test app output: $lines")
                        lastLinesCount.clear()
                        lastLinesCount.add(lines.size)
                        ret = lines.last()
                        break
                    }
                    retryTimes -= 1
                    Thread.sleep(retryIntervalMs)
                }
                ret
            }
            block(TestAppData(readLastLine, windowTitle = windowTitle))
        }
    }
}

val TEST_RESOURCES_DIR = System.getenv("TEST_RESOURCES_DIR")!!
val TEST_APP_DATA_SOURCE_CMD = System.getenv("TEST_APP_DATA_SOURCE_CMD")!!

private fun getResourcePath(path: String): String {
    return Path.of(TEST_RESOURCES_DIR).resolve(path).absolutePathString()
}

internal fun withBlankWindowTestApp(block: (TestAppData) -> Unit) {
    TestApp(listOf("python3", getResourcePath("test_app_blank_window.py"))).run("Blank Window", block)
}

internal fun withDropTargetTestApp(block: (TestAppData) -> Unit) {
    TestApp(listOf("python3", getResourcePath("test_app_drop_target.py"))).run("Drop Target", block)
}

private enum class TestAppOperationMode(val value: String) {
    Normal("normal"),

    Slow("slow"),

    @Suppress("unused")
    ExitImmediately("exit-immediately"),

    ExitAfterStartWriting("exit-after-start-writing"),
}

private fun withDataSourceTestAppImpl(
    dataSource: String,
    dataSourceArg: String,
    operationMode: TestAppOperationMode,
    data: List<Pair<String, Sequence<ByteArray>>>,
    block: (TestAppData) -> Unit,
) {
    val tempFiles = mutableListOf<Path>()
    val args = data.flatMap { (mimeType, inputStream) ->
        val tempFile = Files.createTempFile("linux_test_app_clipboard_data", ".bin")
        tempFile.outputStream().use { outStream ->
            for (e in inputStream) {
                outStream.write(e)
            }
        }
        tempFiles.add(tempFile)
        listOf("--data", mimeType, tempFile.absolutePathString())
    }
    TestApp(listOf(TEST_APP_DATA_SOURCE_CMD, dataSource, dataSourceArg, operationMode.value) + args).run("Test Data Source", block)
    tempFiles.forEach { it.deleteIfExists() }
}

private fun withDragSourceTestApp(
    data: List<Pair<String, Sequence<ByteArray>>>,
    operationMode: TestAppOperationMode = TestAppOperationMode.Normal,
    block: (TestAppData) -> Unit,
) {
    withDataSourceTestAppImpl("--drag", "copy,move", operationMode, data, block)
}

private fun withClipboardSourceTestApp(
    data: List<Pair<String, Sequence<ByteArray>>>,
    operationMode: TestAppOperationMode = TestAppOperationMode.Normal,
    block: (TestAppData) -> Unit,
) {
    withDataSourceTestAppImpl("--clipboard", "clipboard", operationMode, data, block)
}

private fun withPrimarySelectionSourceTestApp(
    data: List<Pair<String, Sequence<ByteArray>>>,
    operationMode: TestAppOperationMode = TestAppOperationMode.Normal,
    block: (TestAppData) -> Unit,
) {
    withDataSourceTestAppImpl("--clipboard", "primary", operationMode, data, block)
}

private fun assertDataMatches(expectedDataSequence: Sequence<ByteArray>, actual: ByteArray) {
    val actualDataStream = actual.inputStream()
    for (e in expectedDataSequence) {
        assertArrayEquals(e, actualDataStream.readNBytes(e.size))
    }
}

private class Dconf private constructor() {
    companion object {
        private const val GNOME_DESKTOP_A11Y_NAMESPACE = "org.gnome.desktop.a11y.interface"
        private const val GNOME_DESKTOP_INTERFACE_NAMESPACE = "org.gnome.desktop.interface"
        private const val GNOME_DESKTOP_PERIPHERALS_MOUSE_NAMESPACE = "org.gnome.desktop.peripherals.mouse"
        private const val GNOME_DESKTOP_PRIVACY_NAMESPACE = "org.gnome.desktop.privacy"
        private const val GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE = "org.gnome.desktop.wm.preferences"

        private data class Setting(
            val schema: String,
            val name: String,
        ) {
            fun read(): String? {
                return runCommandWithOutput(listOf("gsettings", "get", schema, name))?.decodeToString()?.trim()
            }

            fun changeTo(value: String) {
                runCommand(listOf("gsettings", "set", schema, name, value))
            }

            override fun toString(): String {
                return "$schema $name"
            }
        }

        private fun desktopTitlebarActionToString(v: DesktopTitlebarAction): String {
            return when (v) {
                DesktopTitlebarAction.Minimize -> "minimize"
                DesktopTitlebarAction.ToggleMaximize -> "toggle-maximize"
                DesktopTitlebarAction.Menu -> "menu"
                DesktopTitlebarAction.None -> "none"
            }
        }

        private fun withChangedSetting(setting: Setting, tempValue: String, block: () -> Unit, returnOnSame: Boolean = false): Boolean {
            val origValue = setting.read()
            assertNotNull(origValue, "Failed to read $setting")
            log("Original value of setting $setting = $origValue")
            if (returnOnSame && origValue == tempValue) {
                return false
            }

            log("Changing $setting to $tempValue")
            setting.changeTo(tempValue)

            val changedValue = setting.read()
            assertNotNull(changedValue, "Failed to read changed setting $setting")

            assertNotEquals(origValue, changedValue)
            log("Applied $setting $tempValue")

            block()

            log("Reverting $setting to $origValue")
            setting.changeTo(origValue)
            return true
        }

        fun withChangedAudibleBell(value: Boolean, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE, "audible-bell")
            withChangedSetting(setting, value.toString(), block)
        }

        fun withChangedActionDoubleClickTitlebar(value: DesktopTitlebarAction, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE, "action-double-click-titlebar")
            withChangedSetting(setting, desktopTitlebarActionToString(value), block)
        }

        fun withChangedActionMiddleClickTitlebar(value: DesktopTitlebarAction, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE, "action-middle-click-titlebar")
            withChangedSetting(setting, desktopTitlebarActionToString(value), block)
        }

        fun withChangedActionRightClickTitlebar(value: DesktopTitlebarAction, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE, "action-right-click-titlebar")
            withChangedSetting(setting, desktopTitlebarActionToString(value), block)
        }

        fun withChangedAccentColor(value: String, block: () -> Unit): Boolean {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "accent-color")
            return withChangedSetting(setting, "'$value'", block, returnOnSame = true)
        }

        fun withChangedColorScheme(value: ColorSchemeValue, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "color-scheme")
            val tempValue = when (value) {
                ColorSchemeValue.NoPreference -> "default"
                ColorSchemeValue.PreferDark -> "prefer-dark"
                ColorSchemeValue.PreferLight -> "prefer-light"
            }
            withChangedSetting(setting, tempValue, block)
        }

        fun withChangedContrast(value: XdgDesktopContrast, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_A11Y_NAMESPACE, "high-contrast")
            val tempValue = when (value) {
                XdgDesktopContrast.NoPreference -> false
                XdgDesktopContrast.High -> true
            }
            withChangedSetting(setting, tempValue.toString(), block)
        }

        fun withChangedDoubleClick(value: Duration, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_PERIPHERALS_MOUSE_NAMESPACE, "double-click")
            withChangedSetting(setting, value.inWholeMilliseconds.toString(), block)
        }

        fun withChangedMiddleClickPaste(value: Boolean, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "gtk-enable-primary-paste")
            withChangedSetting(setting, value.toString(), block)
        }

        fun withChangedCursorBlink(value: Boolean, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "cursor-blink")
            withChangedSetting(setting, value.toString(), block)
        }

        fun withChangedCursorBlinkTime(value: Duration, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "cursor-blink-time")
            withChangedSetting(setting, value.inWholeMilliseconds.toString(), block)
        }

        fun withChangedCursorBlinkTimeout(value: Duration, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "cursor-blink-timeout")
            withChangedSetting(setting, value.inWholeSeconds.toString(), block)
        }

        fun withChangedCursorTheme(value: String, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "cursor-theme")
            withChangedSetting(setting, "'$value'", block)
        }

        fun withChangedCursorSize(value: UInt, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "cursor-size")
            withChangedSetting(setting, value.toString(), block)
        }

        fun withChangedOverlayScrolling(value: Boolean, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "overlay-scrolling")
            withChangedSetting(setting, value.toString(), block)
        }

        fun withChangedFontAntialiasing(value: FontAntialiasingValue, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "font-antialiasing")
            val strVal = when (value) {
                FontAntialiasingValue.None -> "none"
                FontAntialiasingValue.Grayscale -> "grayscale"
                FontAntialiasingValue.Rgba -> "rgba"
            }
            withChangedSetting(setting, strVal, block)
        }

        fun withChangedFontHinting(value: FontHintingValue, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "font-hinting")
            val strVal = when (value) {
                FontHintingValue.None -> "none"
                FontHintingValue.Slight -> "slight"
                FontHintingValue.Medium -> "medium"
                FontHintingValue.Full -> "full"
            }
            withChangedSetting(setting, strVal, block)
        }

        fun withChangedFontRgbaOrder(value: FontRgbaOrderValue, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "font-rgba-order")
            val strVal = when (value) {
                FontRgbaOrderValue.Rgb -> "rgb"
                FontRgbaOrderValue.Bgr -> "bgr"
                FontRgbaOrderValue.Vrgb -> "vrgb"
                FontRgbaOrderValue.Vbgr -> "vbgr"
            }
            withChangedSetting(setting, strVal, block)
        }

        fun withChangedEnableAnimations(value: Boolean, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_INTERFACE_NAMESPACE, "enable-animations")
            withChangedSetting(setting, value.toString(), block)
        }

        fun withChangedRecentFilesEnabled(value: Boolean, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_PRIVACY_NAMESPACE, "remember-recent-files")
            withChangedSetting(setting, value.toString(), block)
        }

        fun withChangedRecentFilesMaxAgeDays(value: Int, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_PRIVACY_NAMESPACE, "recent-files-max-age")
            withChangedSetting(setting, value.toString(), block)
        }

        fun withChangedReducedMotion(value: XdgDesktopReducedMotion, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_A11Y_NAMESPACE, "reduced-motion")
            val tempValue = when (value) {
                XdgDesktopReducedMotion.NoPreference -> "'no-preference'"
                XdgDesktopReducedMotion.ReducedMotion -> "'reduce'"
            }
            withChangedSetting(setting, tempValue, block)
        }

        fun withChangedDragThresholdPixelsChanged(value: Int, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_PERIPHERALS_MOUSE_NAMESPACE, "drag-threshold")
            withChangedSetting(setting, value.toString(), block)
        }

        fun withChangedTitlebarLayout(value: String, block: () -> Unit) {
            val setting = Setting(GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE, "button-layout")
            withChangedSetting(setting, "'$value'", block)
        }
    }
}

private fun isNotificationServiceRunning(): Boolean {
    return ProcessBuilder("busctl", "--user", "status", "org.freedesktop.Notifications").start().waitFor() == 0
}

@JsonClass(generateAdapter = true)
private data class MakoList(val data: List<List<Entry>>) {
    data class StringData(val data: String)
    data class IntData(val data: Int)

    @Suppress("PropertyName")
    data class Entry(
        val `app-name`: StringData,
        val `app-icon`: StringData,
        val category: StringData,
        val summary: StringData,
        val body: StringData,
        val id: IntData,
        // val actions
    )
}

private fun getMakoList(): MakoList? {
    val json = runCommandWithOutput(listOf("makoctl", "list"))?.decodeToString() ?: return null
    val moshi: Moshi = Moshi.Builder().addLast(KotlinJsonAdapterFactory()).build()
    val jsonAdapter = moshi.adapter(MakoList::class.java)
    return jsonAdapter.fromJson(json)
}

private fun withMako(block: () -> Unit) {
    val makoPath = System.getenv("TEST_MAKO_PATH")!!
    val process = ProcessBuilder(makoPath).start()
    AutoCloseable {
        if (process.isAlive) {
            process.toHandle().destroy()
            process.waitFor()
        } else {
            log("mako process stopped by itself")
        }
        process.errorStream.readAllBytes().decodeToString().also {
            if (it.isNotEmpty()) {
                log("mako error output: $it")
            }
        }
        process.inputStream.readAllBytes().decodeToString().also {
            if (it.isNotEmpty()) {
                log("mako output: $it")
            }
        }
        process.destroy()
    }.use {
        assertTrue(process.isAlive)
        waitUntilEq(true) { isNotificationServiceRunning() }
        block()
    }
}

private fun listClipboardFormats(): ByteArray? {
    return runCommandWithOutput(listOf("wl-paste", "--list-types"))
}

private fun getClipboardContent(format: String): ByteArray? {
    return runCommandWithOutput(listOf("wl-paste", "--no-newline", "-t", format))
}

private fun listPrimarySelectionFormats(): ByteArray? {
    return runCommandWithOutput(listOf("wl-paste", "--primary", "--list-types"))
}

private fun getPrimarySelectionContent(format: String): ByteArray? {
    return runCommandWithOutput(listOf("wl-paste", "--no-newline", "--primary", "-t", format))
}

private fun screenshot(outPath: Path, rect: LogicalRect? = null, hideCursor: Boolean = true) {
    val cmd = buildList {
        add("grim")
        if (rect != null) {
            add("-g")
            add("${rect.x.rawLogical},${rect.y.rawLogical} ${rect.width.rawLogical}x${rect.height.rawLogical}")
        }
        if (!hideCursor) {
            add("-c")
        }
        add(outPath.absolutePathString())
    }
    runCommand(cmd)
}

private fun <T : Any> LinkedBlockingQueue<T>.drainAll(): List<T> {
    val ret = mutableListOf<T>()
    drainTo(ret)
    return ret
}

private class SingleThreadTaskQueue {
    private val taskQueue = LinkedBlockingQueue<Pair<() -> Unit, CompletableFuture<Error?>>>()

    private val thread = thread {
        while (true) {
            val (task, future) = taskQueue.take()
            try {
                task()
                future.complete(null)
            } catch (_: InterruptedException) {
                future.complete(null)
                break
            } catch (e: Error) {
                future.complete(e)
            }
        }
    }

    fun add(task: () -> Unit): Future<Error?> {
        val future = CompletableFuture<Error?>()
        taskQueue.add(task to future)
        return future
    }

    fun exit() {
        add { throw InterruptedException() }.get()
        thread.join()
    }
}

private class Checklist(entries: List<String>) {
    private val state: MutableMap<String, Boolean> = entries.associateWith { _ -> false }.toMutableMap()

    fun uncheckedEntries(): Set<String> {
        return state.filter { !it.value }.keys
    }

    fun checkEntry(entry: String): Boolean {
        return if (state[entry]!!) {
            false
        } else {
            state[entry] = true
            true
        }
    }

    fun uncheckEntry(entry: String) {
        state[entry] = false
    }
}

internal data class TestMousePosition(
    val base: GlobalPosition,
    val offsetX: LogicalPixelsInt = LogicalPixelsInt.Zero,
    val offsetY: LogicalPixelsInt = LogicalPixelsInt.Zero,
) {
    fun assertEquals(actual: LogicalPoint) {
        val expected = Pair(offsetX.toLogicalPixels(), offsetY.toLogicalPixels())
        val actualPair = Pair(actual.x, actual.y)
        assertEquals(expected, actualPair)
    }

    fun shifted(x: LogicalPixelsInt = LogicalPixelsInt.Zero, y: LogicalPixelsInt = LogicalPixelsInt.Zero): TestMousePosition {
        return TestMousePosition(this.base, this.offsetX + x, this.offsetY + y)
    }
}

private fun performSoftwareDrawing(size: PhysicalSize, softwareDrawData: SoftwareDrawData, draw: (SkCanvas) -> Unit) {
    return SkSurface.makeRasterDirect(
        imageInfo = SkImageInfo(
            width = size.width.rawPhysical,
            height = size.height.rawPhysical,
            colorType = SkColorType.BGRA_8888,
            alphaType = SkColorAlphaType.OPAQUE,
            colorSpace = SkColorSpace.sRGB,
        ),
        pixelsPtr = softwareDrawData.canvas,
        rowBytes = softwareDrawData.stride,
        surfaceProps = null,
    ).use { surface ->
        draw(surface.canvas)
        surface.flushAndSubmit()
    }
}

internal data class InitialSettings(
    var accentColor: DesktopSetting.AccentColor? = null,
    var actionDoubleClickTitlebar: DesktopSetting.ActionDoubleClickTitlebar? = null,
    var actionMiddleClickTitlebar: DesktopSetting.ActionMiddleClickTitlebar? = null,
    var actionRightClickTitlebar: DesktopSetting.ActionRightClickTitlebar? = null,
    var audibleBell: DesktopSetting.AudibleBell? = null,
    var colorScheme: DesktopSetting.ColorScheme? = null,
    var contrast: DesktopSetting.Contrast? = null,
    var cursorBlink: DesktopSetting.CursorBlink? = null,
    var cursorBlinkTime: DesktopSetting.CursorBlinkTime? = null,
    var cursorBlinkTimeout: DesktopSetting.CursorBlinkTimeout? = null,
    var cursorSize: DesktopSetting.CursorSize? = null,
    var cursorTheme: DesktopSetting.CursorTheme? = null,
    var doubleClickInterval: DesktopSetting.DoubleClickInterval? = null,
    var dragThresholdPixels: DesktopSetting.DragThresholdPixels? = null,
    var enableAnimations: DesktopSetting.EnableAnimations? = null,
    var fontAntialiasing: DesktopSetting.FontAntialiasing? = null,
    var fontHinting: DesktopSetting.FontHinting? = null,
    var fontRgbaOrder: DesktopSetting.FontRgbaOrder? = null,
    var middleClickPaste: DesktopSetting.MiddleClickPaste? = null,
    var overlayScrolling: DesktopSetting.OverlayScrolling? = null,
    var recentFilesEnabled: DesktopSetting.RecentFilesEnabled? = null,
    var recentFilesMaxAgeDays: DesktopSetting.RecentFilesMaxAgeDays? = null,
    var reducedMotion: DesktopSetting.ReducedMotion? = null,
    var titlebarLayout: DesktopSetting.TitlebarLayout? = null,
)

internal data class ExpectedWindowConfigure(
    val checkWindowCapabilities: (WindowCapabilities) -> Unit,
    val windowId: WindowId,
    val size: LogicalSize,
    val active: Boolean,
    val maximized: Boolean,
    val fullscreen: Boolean,
    val decorationMode: WindowDecorationMode,
) {
    fun assertEquals(event: Event?, msg: String? = null) {
        val msg = msg?.let { ": $it" } ?: ""
        assertInstanceOf<Event.WindowConfigure>(event, msg)
        val failures = mutableListOf<String>()
        if (windowId != event.windowId) {
            failures.add("\nWindowConfigure.windowId(expected=$windowId, actual=${event.windowId})$msg")
        }
        if (size != event.size) {
            failures.add("\nWindowConfigure.size(expected=$size, actual=${event.size})$msg")
        }
        if (active != event.active) {
            failures.add("\nWindowConfigure.active(expected=$active, actual=${event.active})$msg")
        }
        if (maximized != event.maximized) {
            failures.add("\nWindowConfigure.maximized(expected=$maximized, actual=${event.maximized})$msg")
        }
        if (fullscreen != event.fullscreen) {
            failures.add("\nWindowConfigure.fullscreen(expected=$fullscreen, actual=${event.fullscreen})$msg")
        }
        if (decorationMode != event.decorationMode) {
            failures.add("\nWindowConfigure.decorationMode(expected=$decorationMode, actual=${event.decorationMode})$msg")
        }
        val failuresString = failures.joinToString()
        assertEquals("", failuresString)
        checkWindowCapabilities(event.capabilities)
    }
}

internal fun checkPhysicalSize(expectedLogical: LogicalSize, scale: Scale, actual: PhysicalSize) {
    val expected = Pair(
        expectedLogical.width.toRawPhysical(scale).roundToInt(),
        expectedLogical.height.toRawPhysical(scale).roundToInt(),
    )
    val actualPair = Pair(actual.width.rawPhysical, actual.height.rawPhysical)
    assertEquals(expected, actualPair)
}

abstract class WaylandTestsBase {
    companion object {
        internal const val APP_ID = "org.jetbrains.desktop.linux.tests"
        internal const val TEXT_UTF8_MIME_TYPE = "text/plain;charset=utf-8"
        internal const val URI_LIST_MIME_TYPE = "text/uri-list"
        internal const val HTML_TEXT_MIME_TYPE = "text/html"
        internal const val PNG_MIME_TYPE = "image/png"

        internal val TEST_APP_BROWSER_LOG_PATH: Path = Path.of(System.getenv("HOME")!!, "test_app_browser.log")

        internal val wm by lazy { SwayWm() }

        private val appExecutor = SingleThreadTaskQueue()

        @BeforeAll
        @JvmStatic
        fun initAll() {
            KotlinDesktopToolkit.init(
                libraryFolderPath = Path.of(System.getProperty("kdt.linux.library.folder.path")!!),
                consoleLogLevel = LogLevel.Debug,
                useDebugBuild = true,
            )
        }

        @AfterAll
        @JvmStatic
        fun tearDownAll() {
            log("tearDownAll start")
            appExecutor.exit()

            log("tearDownAll end")
        }

        internal fun defaultWindowParams(): WindowParams {
            return WindowParams(
                windowId = 0,
                title = "Test Window 1",
                size = LogicalSize(width = LogicalPixelsInt(200), height = LogicalPixelsInt(300)),
                minSize = null,
                appId = APP_ID,
                preferClientSideDecoration = false,
                renderingMode = RenderingMode.Software,
                clientSideDecorationFrame = WindowFrame(
                    padding = WindowFrame.Padding.withAll(LogicalPixelsInt(25)),
                    resizerThickness = WindowFrame.ResizerThickness.withAll(LogicalPixelsInt(12)),
                ),
            )
        }

        internal fun checkEvents(events: List<Event>, checks: Map<String, (Event, Set<String>) -> Boolean>): Set<String> {
            val remainingChecks = checks.toMutableMap()
            log("checkEvents: ${remainingChecks.keys}")
            for (event in events) {
                val matchedName = remainingChecks.firstNotNullOfOrNull { (name, predicate) ->
                    if (predicate(event, remainingChecks.keys)) name else null
                }
                if (matchedName != null) {
                    remainingChecks.remove(matchedName)
                }
            }
            return remainingChecks.keys
        }
    }

    val app by lazy { Application() }
    private var appExecutingResult: Future<Error?>? = null
    val eventQueue = LinkedBlockingQueue<Event>()
    var lastDrawEvents = mutableMapOf<WindowId, Event.WindowDraw>()
    var lastConfigureEvents = mutableMapOf<WindowId, Event.WindowConfigure>()
    var lastScales = mutableMapOf<WindowId, Scale>()
    internal var lastScreenSize: LogicalSize? = null
    var testSuccessful = false

    internal fun defaultApplicationConfig(
        queryDragAndDropTarget: (DragAndDropQueryData) -> DragAndDropQueryResponse = { _ -> DragAndDropQueryResponse(emptyList()) },
        getDataTransferData: (DataSource, String) -> ByteArray? = { _, _ -> null },
    ): ApplicationConfig {
        return ApplicationConfig(
            eventHandler = { event ->
                when (event) {
                    is Event.WindowScaleChanged -> {
                        lastScales[event.windowId] = event.newScale
                    }
                    is Event.WindowDraw -> {
                        val configure = lastConfigureEvents[event.windowId]!!
                        val scale = lastScales[event.windowId] ?: Scale.NO_SCALE
                        checkPhysicalSize(configure.size, scale, event.size)
                    }
                    is Event.WindowConfigure -> {
                        lastConfigureEvents[event.windowId] = event
                    }
                    else -> {}
                }
                val shouldDraw = event is Event.WindowDraw && lastDrawEvents[event.windowId]?.size != event.size
                if (shouldDraw) {
                    lastDrawEvents[event.windowId] = event
                }
                val res = eventHandler?.invoke(event) ?: run {
                    // Need to draw window content in order for Sway to activate it
                    if (shouldDraw) {
                        event.softwareDrawData?.let { softwareDrawData ->
                            performSoftwareDrawing(event.size, softwareDrawData) { canvas ->
                                canvas.clear(SkColor.WHITE)
                                SkPaint().use { paint ->
                                    paint.color = SkColor.RED
                                    canvas.drawRect(
                                        SkRect.makeXYWH(
                                            event.size.width.rawPhysical.toFloat() - 10f,
                                            event.size.height.rawPhysical.toFloat() - 10f,
                                            10f,
                                            10f,
                                        ),
                                        paint,
                                    )
                                }
                            }
                            EventHandlerResult.Stop
                        } ?: EventHandlerResult.Continue
                    } else {
                        EventHandlerResult.Continue
                    }
                }
                if (event !is Event.WindowDraw || shouldDraw) {
                    log("Event: $event")
                    assert(eventQueue.offer(event)) { "Event queue overflow" }
                }
                res
            },
            queryDragAndDropTarget = queryDragAndDropTarget,
            getDataTransferData = getDataTransferData,
        )
    }

    internal fun run(applicationConfig: ApplicationConfig): InitialSettings {
        appExecutingResult = appExecutor.add {
            try {
                app.runEventLoop(applicationConfig)
            } catch (t: Throwable) {
                app.stopEventLoop()
                fail(withTimestamp("Application event loop finished exceptionally"), t)
            }
        }
        assertInstanceOf<Event.ApplicationStarted>(getNextEvent())

        var displayConfigurationChangeEvent: Event.DisplayConfigurationChange? = null
        val initialSettings = InitialSettings()

        // Initial settings are always sent in the same order, and this is the last one.
        while (initialSettings.titlebarLayout == null) {
            val event = getNextEvent()
            if (event is Event.DisplayConfigurationChange) {
                displayConfigurationChangeEvent = event
            } else {
                assertInstanceOf<Event.DesktopSettingChange>(event, "initialSettings=$initialSettings")
                when (val setting = event.setting) {
                    is DesktopSetting.AccentColor -> initialSettings.accentColor = setting
                    is DesktopSetting.ActionDoubleClickTitlebar -> initialSettings.actionDoubleClickTitlebar = setting
                    is DesktopSetting.ActionMiddleClickTitlebar -> initialSettings.actionMiddleClickTitlebar = setting
                    is DesktopSetting.ActionRightClickTitlebar -> initialSettings.actionRightClickTitlebar = setting
                    is DesktopSetting.AudibleBell -> initialSettings.audibleBell = setting
                    is DesktopSetting.ColorScheme -> initialSettings.colorScheme = setting
                    is DesktopSetting.Contrast -> initialSettings.contrast = setting
                    is DesktopSetting.CursorBlink -> initialSettings.cursorBlink = setting
                    is DesktopSetting.CursorBlinkTime -> initialSettings.cursorBlinkTime = setting
                    is DesktopSetting.CursorBlinkTimeout -> initialSettings.cursorBlinkTimeout = setting
                    is DesktopSetting.CursorSize -> initialSettings.cursorSize = setting
                    is DesktopSetting.CursorTheme -> initialSettings.cursorTheme = setting
                    is DesktopSetting.DoubleClickInterval -> initialSettings.doubleClickInterval = setting
                    is DesktopSetting.DragThresholdPixels -> initialSettings.dragThresholdPixels = setting
                    is DesktopSetting.EnableAnimations -> initialSettings.enableAnimations = setting
                    is DesktopSetting.FontAntialiasing -> initialSettings.fontAntialiasing = setting
                    is DesktopSetting.FontHinting -> initialSettings.fontHinting = setting
                    is DesktopSetting.FontRgbaOrder -> initialSettings.fontRgbaOrder = setting
                    is DesktopSetting.MiddleClickPaste -> initialSettings.middleClickPaste = setting
                    is DesktopSetting.OverlayScrolling -> initialSettings.overlayScrolling = setting
                    is DesktopSetting.RecentFilesEnabled -> initialSettings.recentFilesEnabled = setting
                    is DesktopSetting.RecentFilesMaxAgeDays -> initialSettings.recentFilesMaxAgeDays = setting
                    is DesktopSetting.ReducedMotion -> initialSettings.reducedMotion = setting
                    is DesktopSetting.TitlebarLayout -> initialSettings.titlebarLayout = setting
                }
            }
        }
        log("Received all settings")

        if (displayConfigurationChangeEvent == null) {
            displayConfigurationChangeEvent = assertInstanceOf<Event.DisplayConfigurationChange>(getNextEvent())
        }
        assertNotNull(displayConfigurationChangeEvent)
        lastScreenSize = getScreenSize(displayConfigurationChangeEvent)

        // Reset the mouse position
        moveMouseTo(TestMousePosition(GlobalPosition.Zero))

        assertTrue(eventQueue.isEmpty(), "Event queue empty, but contained ${eventQueue.firstOrNull()}. InitialSettings=$initialSettings")
        return initialSettings
    }

    internal fun getNextEvent(timeout: Duration = 5.seconds): Event? {
        val event = eventQueue.poll(timeout.inWholeMilliseconds, TimeUnit.MILLISECONDS)
        if (event == null) {
            log("Timed out waiting for event")
            return null
        }
        return event
    }

    internal fun <T> withNextEvent(timeout: Duration = 5.seconds, block: (Event?) -> T): T {
        return block(getNextEvent(timeout))
    }

    private fun <T : Event> awaitEventWithHistory(
        timeout: Duration = 5.seconds,
        convert: (Event) -> T?,
        predicate: (T) -> Boolean,
    ): Pair<T?, List<Event>> {
        val otherEvents = mutableListOf<Event>()
        while (true) {
            val event: Event = eventQueue.poll(timeout.inWholeMilliseconds, TimeUnit.MILLISECONDS) ?: return Pair(null, otherEvents)
            val converted = convert(event)
            if (converted != null && predicate(converted)) {
                return Pair(converted, otherEvents)
            } else {
                otherEvents.add(event)
            }
        }
    }

    internal fun <T : Event> awaitEvent(
        convert: (Event?) -> T?,
        timeout: Duration = 5.seconds,
        msg: String? = null,
        predicate: (T) -> Boolean,
    ): T {
        val (event, otherEvents) = awaitEventWithHistory(timeout, convert = convert, predicate = predicate)
        if (event == null) {
            val additionalMsg = if (msg != null) ": $msg" else ""
            val otherEventsMsg = if (otherEvents.isEmpty()) "" else ". Other events:\n${otherEvents.joinToString("\n")}"
            fail(withTimestamp("Timed out waiting for event$additionalMsg$otherEventsMsg"))
        } else {
            return event
        }
    }

    internal inline fun <reified T : Event> awaitEventOfType(
        timeout: Duration = 5.seconds,
        msg: String? = null,
        noinline predicate: (T) -> Boolean,
    ): T {
        return awaitEvent(convert = { it as? T }, timeout = timeout, msg = msg, predicate = predicate)
    }

    internal fun awaitAnyEvent(timeout: Duration = 5.seconds, msg: String? = null, predicate: (Event?) -> Boolean): Event {
        return awaitEvent(convert = { it }, timeout = timeout, msg = msg, predicate = predicate)
    }

    internal fun <T> ui(timeout: Duration = 5.seconds, body: () -> T): T {
        val future = CompletableFuture<T>()
        app.runOnEventLoopAsync {
            try {
                val ret = body()
                future.complete(ret)
            } catch (e: Throwable) {
                future.completeExceptionally(e)
            }
        }
        try {
            return future.get(timeout.inWholeMilliseconds, TimeUnit.MILLISECONDS)
        } catch (e: ExecutionException) {
            throw e.cause!!
        }
    }

    internal fun doVirtualDeviceEvent(uriPath: String, expectedStatusCode: Int = 200): String {
        val httpClient = HttpClient.newBuilder().connectTimeout(java.time.Duration.ofSeconds(2)).build()

        val httpRequest = HttpRequest.newBuilder(URI("http://127.0.0.1:8000/$uriPath")).build()
        val response = httpClient.send(httpRequest, HttpResponse.BodyHandlers.ofString())
        assertEquals(expectedStatusCode, response.statusCode())
        return response.body()
    }

    internal fun withSyncVirtualDeviceCommand(uriPath: String, expectedStatusCode: Int = 200, block: () -> Unit): String {
        doVirtualDeviceEvent("wait_for_im_update")
        block()
        return doVirtualDeviceEvent(uriPath, expectedStatusCode)
    }

    internal fun withSyncGetInputMethodState(block: () -> Unit): String {
        return withSyncVirtualDeviceCommand("get_input_state", block = block)
    }

    internal fun <T> withVirtualDeviceEvent(perform: () -> String, revert: () -> String, block: () -> T): T {
        doVirtualDeviceEvent(perform())
        AutoCloseable {
            doVirtualDeviceEvent(revert())
        }.use {
            return block()
        }
    }

    internal fun <T> withKeyPress(key: UInt, block: () -> T): T {
        return withVirtualDeviceEvent(
            { "raw_key?keycode=$key&direction=1" },
            { "raw_key?keycode=$key&direction=0" },
            block,
        )
    }

    internal fun <T> withMouseButtonDown(button: MouseButton, block: () -> T): T {
        val buttonVal = button.value
        return withVirtualDeviceEvent(
            { "mousebutton?button=$buttonVal&direction=1" },
            { "mousebutton?button=$buttonVal&direction=0" },
            block,
        )
    }

    internal fun moveMouseTo(pos: TestMousePosition) {
        val screenSize = checkNotNull(lastScreenSize)
        log("moveMouseTo: $pos")
        val x: Int = (pos.base.x + pos.offsetX).rawLogical
        val y: Int = (pos.base.y + pos.offsetY).rawLogical
        val xExtent: Int = screenSize.width.rawLogical
        val yExtent: Int = screenSize.height.rawLogical
        doVirtualDeviceEvent("mousemove?x=$x&y=$y&x_extent=$xExtent&y_extent=$yExtent")
    }

    internal fun wiggleMouseUntil(pos: TestMousePosition, timeout: Duration = 5.seconds, predicate: () -> Boolean): Boolean {
        val startTime = TimeSource.Monotonic.markNow()
        var moveToRight = false
        while (startTime.elapsedNow() < timeout) {
            if (predicate()) {
                return true
            }
            val offset = if (moveToRight) LogicalPixelsInt(5) else LogicalPixelsInt(0)
            moveMouseTo(TestMousePosition(pos.base, pos.offsetX + offset, pos.offsetY + offset))
            ui {}
            Thread.sleep(1)
            moveToRight = !moveToRight
        }
        return false
    }

    internal fun scrollMouseUp() {
        doVirtualDeviceEvent("mousescroll?axis_source=0&vertical_scroll_120=-120&horizontal_scroll_120=0")
    }

    internal fun scrollMouseDown() {
        doVirtualDeviceEvent("mousescroll?axis_source=0&vertical_scroll_120=120&horizontal_scroll_120=0")
    }

    internal fun withSetClipboardContent(@Suppress("unused") eventSerial: EventSerial, mimeTypes: List<String>, block: () -> Unit) {
        ui { app.clipboardPut(mimeTypes) }
        awaitEventOfType<Event.DataTransferAvailable>(msg = "withSetClipboardContent set") {
            it.dataSource == DataSource.Clipboard &&
                it.mimeTypes == mimeTypes
        }
        AutoCloseable {
            ui { app.clipboardPut(emptyList()) }
            // Ensure that `DataTransferAvailable` events are sent.
            ui {}
        }.use {
            block()
        }
    }

    internal fun withSetPrimarySelectionContent(@Suppress("unused") eventSerial: EventSerial, mimeTypes: List<String>, block: () -> Unit) {
        ui { app.primarySelectionPut(mimeTypes) }
        awaitEventOfType<Event.DataTransferAvailable>(msg = "withSetPrimarySelectionContent set") {
            it.dataSource ==
                DataSource.PrimarySelection &&
                it.mimeTypes == mimeTypes
        }
        AutoCloseable {
            ui { app.primarySelectionPut(emptyList()) }
            // Ensure that `DataTransferAvailable` events are sent.
            ui {}
        }.use {
            block()
        }
    }

    @Suppress("unused")
    internal class InitialWindowData(
        val window: Window,
        val scale: Event.WindowScaleChanged,
        val configure: Event.WindowConfigure,
        val draw: Event.WindowDraw,
        val keyboardEnter: Event.WindowKeyboardEnter,
        val screen: Event.WindowScreenChange,
        val otherEvents: List<Event>,
    )

    internal fun createWindowAndWaitForFocus(windowParams: WindowParams): InitialWindowData {
        val window = ui { app.createWindow(windowParams) }
        lateinit var scale: Event.WindowScaleChanged
        lateinit var configure: Event.WindowConfigure
        lateinit var draw: Event.WindowDraw
        lateinit var keyboardEnter: Event.WindowKeyboardEnter
        lateinit var screen: Event.WindowScreenChange
        val otherEvents = mutableListOf<Event>()
        val checklist = Checklist(listOf("configure", "draw", "keyboardEnter", "scale", "screen", "textInput"))
        var drawEventCount = 0
        waitUntilEq(emptySet()) {
            when (val event: Event? = eventQueue.poll(3000, TimeUnit.MILLISECONDS)) {
                null -> {
                    fail("Timed out waiting for event, unchecked events: ${checklist.uncheckedEntries()}")
                }

                is Event.WindowConfigure -> {
                    if (windowParams.windowId == event.windowId) {
                        if (event.active) {
                            configure = event
                            checklist.checkEntry("configure")
                        } else {
                            wm.focusWindow(windowParams.title)
                        }
                    }
                }

                is Event.WindowScreenChange -> {
                    if (windowParams.windowId == event.windowId) {
                        screen = event
                        checklist.checkEntry("screen")
                    }
                }

                is Event.WindowDraw -> {
                    if (windowParams.windowId == event.windowId) {
                        draw = event
                        drawEventCount += 1
                        checklist.checkEntry("draw")
                    }
                }

                is Event.TextInputAvailability -> {
                    if (windowParams.windowId == event.windowId && event.available) {
                        checklist.checkEntry("textInput")
                    }
                }

                is Event.WindowKeyboardEnter -> {
                    if (windowParams.windowId == event.windowId) {
                        keyboardEnter = event
                        checklist.checkEntry("keyboardEnter")
                    }
                }

                is Event.WindowScaleChanged -> {
                    if (windowParams.windowId == event.windowId) {
                        scale = event
                        checklist.checkEntry("scale")
                        checklist.uncheckEntry("draw")
                    }
                }

                else -> {
                    otherEvents.add(event)
                }
            }
            checklist.uncheckedEntries()
        }
        return InitialWindowData(
            window = window,
            scale = scale,
            configure = configure,
            draw = draw,
            keyboardEnter = keyboardEnter,
            screen = screen,
            otherEvents = otherEvents,
        )
    }

    internal fun moveMouseInsideWindow(windowTitle: String): TestMousePosition {
        val pos = TestMousePosition(
            wm.getWindowState(windowTitle).clientArea.pos,
            LogicalPixelsInt(50),
            LogicalPixelsInt(50),
        )
        moveMouseTo(pos)
        withNextEvent { event ->
            assertInstanceOf<Event.MouseEntered>(event)
            pos.assertEquals(event.locationInWindow)
        }
        return pos
    }

    internal fun waitForWindowFocusAfterMouseDown(windowId: WindowId): Event.MouseDown {
        val checklist = Checklist(listOf("configure", "keyboardEnter", "mouseDown"))
        var mouseDown: Event.MouseDown? = null
        waitUntilEq(emptySet()) {
            when (val event: Event? = eventQueue.poll()) {
                is Event.WindowConfigure -> {
                    assertEquals(windowId, event.windowId)
                    if (event.active) {
                        checklist.checkEntry("configure")
                    }
                }

                is Event.WindowKeyboardEnter -> {
                    assertEquals(windowId, event.windowId)
                    checklist.checkEntry("keyboardEnter")
                }

                is Event.MouseDown -> {
                    assertEquals(windowId, event.windowId)
                    checklist.checkEntry("mouseDown")
                    mouseDown = event
                }

                else -> {}
            }
            checklist.uncheckedEntries()
        }
        return mouseDown!!
    }

    internal fun moveWindowTo(windowTitle: String, pos: GlobalPosition): GlobalPosition {
        wm.moveWindow(windowTitle, pos)
        return wm.getWindowState(windowTitle).clientArea.pos
    }

    internal fun waitForWindowFocus(windowId: WindowId, msg: String? = null) {
        val checklist = Checklist(listOf("configure", "keyboardEnter"))
        waitUntilEq(emptySet(), msg = msg) {
            when (val event: Event? = eventQueue.poll()) {
                is Event.WindowConfigure -> {
                    assertEquals(windowId, event.windowId)
                    if (event.active) {
                        checklist.checkEntry("configure")
                    }
                }

                is Event.WindowKeyboardEnter -> {
                    assertEquals(windowId, event.windowId)
                    checklist.checkEntry("keyboardEnter")
                }

                else -> {}
            }
            checklist.uncheckedEntries()
        }
    }

    internal fun waitForTestAppFocus(windowId: WindowId, msg: String? = null) {
        val testAppFocusedChecklist = Checklist(listOf("configure", "keyboardLeave"))
        waitUntilEq(emptySet(), msg = msg) {
            when (val event: Event? = eventQueue.poll()) {
                is Event.WindowConfigure -> {
                    assertEquals(windowId, event.windowId)
                    if (!event.active) {
                        testAppFocusedChecklist.checkEntry("configure")
                    }
                }

                is Event.WindowKeyboardLeave -> {
                    assertEquals(windowId, event.windowId)
                    testAppFocusedChecklist.checkEntry("keyboardLeave")
                }

                else -> {}
            }
            testAppFocusedChecklist.uncheckedEntries()
        }
    }

    internal fun waitForWindowClosed(windowId: WindowId) {
        withNextEvent { event ->
            assertInstanceOf<Event.WindowClosed>(event)
            assertEquals(windowId, event.windowId)
        }
    }

    internal fun getScreenSize(event: Event.DisplayConfigurationChange): LogicalSize {
        val screens = event.screens.screens
        val screen = screens.firstOrNull()
        assertNotNull(screen)
        return screen.size
    }

    @Volatile
    internal var eventHandler: ((Event) -> EventHandlerResult)? = null

    @BeforeEach
    fun setUp() {
        testStart.elapsedNow() // trigger lazy evaluation
    }

    @AfterEach
    @Timeout(value = 20, unit = TimeUnit.SECONDS)
    fun tearDown() {
        log("tearDown start")
//        if (!testSuccessful) {
//            Thread.sleep(60 * 1000)
//        }
        appExecutingResult?.let {
            if (!it.isDone) {
                app.stopEventLoop()
            }
            assertNull(it.get())
            appExecutor.add { app.close() }.get()
            appExecutingResult = null
            awaitEventOfType<Event.ApplicationWantsToTerminate> { true }
            awaitEventOfType<Event.ApplicationWillTerminate> { true }
        }

        log("tearDown end")
    }
}

@EnabledOnOs(OS.LINUX)
@EnabledIfEnvironmentVariable(named = "WAYLAND_DISPLAY", matches = ".*")
@Execution(ExecutionMode.SAME_THREAD)
class WaylandTests : WaylandTestsBase() {
    @Test
    fun testIsEventLoopThread() {
        run(defaultApplicationConfig())
        assertFalse(app.isEventLoopThread())
        ui { assertTrue(app.isEventLoopThread()) }
        testSuccessful = true
    }

    /**
     * Implementation tries to fix the flakiness, e.g.:
     * Original value of setting org.gnome.desktop.interface cursor-theme = 'Adwaita'
     * Changing org.gnome.desktop.interface cursor-theme to 'AdwaitaTest'
     * Event: DesktopSettingChange(setting=CursorTheme(value=AdwaitaTest))
     * Applied org.gnome.desktop.interface cursor-theme 'AdwaitaTest'
     * Reverting org.gnome.desktop.interface cursor-theme to 'Adwaita'
     * Event: DesktopSettingChange(setting=CursorTheme(value=Adwaita))
     * Event: DesktopSettingChange(setting=CursorTheme(value=AdwaitaTest))
     * Event: DesktopSettingChange(setting=CursorTheme(value=Adwaita))
     */
    @Test
    fun testSettings() {
        val initialSettings = run(defaultApplicationConfig())

        val initialActionDoubleClickTitlebar =
            checkNotNull(initialSettings.actionDoubleClickTitlebar) { "Initial ActionDoubleClickTitlebar setting" }
        val initialActionMiddleClickTitlebar =
            checkNotNull(initialSettings.actionMiddleClickTitlebar) { "Initial ActionMiddleClickTitlebar setting" }
        val initialActionRightClickTitlebar =
            checkNotNull(initialSettings.actionRightClickTitlebar) { "Initial ActionRightClickTitlebar setting" }
        val initialAudibleBell = checkNotNull(initialSettings.audibleBell) { "Initial AudibleBell setting" }
        val initialColorScheme = checkNotNull(initialSettings.colorScheme) { "Initial ColorScheme setting" }
        val initialCursorBlink = checkNotNull(initialSettings.cursorBlink) { "Initial CursorBlink setting" }
        val initialCursorBlinkTime = checkNotNull(initialSettings.cursorBlinkTime) { "Initial CursorBlinkTime setting" }
        val initialCursorBlinkTimeout = checkNotNull(initialSettings.cursorBlinkTimeout) { "Initial CursorBlinkTimeout setting" }
        val initialCursorSize = checkNotNull(initialSettings.cursorSize) { "Initial CursorSize setting" }
        val initialCursorTheme = checkNotNull(initialSettings.cursorTheme) { "Initial CursorTheme setting" }
        val initialEnableAnimations = checkNotNull(initialSettings.enableAnimations) { "Initial EnableAnimations setting" }
        val initialFontAntialiasing = checkNotNull(initialSettings.fontAntialiasing) { "Initial FontAntialiasing setting" }
        val initialFontHinting = checkNotNull(initialSettings.fontHinting) { "Initial FontHinting setting" }
        val initialFontRgbaOrder = checkNotNull(initialSettings.fontRgbaOrder) { "Initial FontRgbaOrder setting" }
        val initialMiddleClickPaste = checkNotNull(initialSettings.middleClickPaste) { "Initial MiddleClickPaste setting" }
        val initialOverlayScrolling = checkNotNull(initialSettings.overlayScrolling) { "Initial OverlayScrolling setting" }
        val initialRecentFilesEnabled = checkNotNull(initialSettings.recentFilesEnabled) { "Initial RecentFilesEnabled setting" }
        val initialRecentFilesMaxAgeDays = checkNotNull(initialSettings.recentFilesMaxAgeDays) { "Initial RecentFilesMaxAgeDays setting" }
        val initialTitlebarLayout = checkNotNull(initialSettings.titlebarLayout) { "Initial TitlebarLayout setting" }

        initialSettings.accentColor?.let { initialAccentColor ->
            val newColors = mutableSetOf<Color>()
            var matchedOriginal = false
            for (newColorName in listOf("blue", "teal", "green", "yellow", "orange", "red", "pink", "purple", "slate")) {
                val isChanged = Dconf.withChangedAccentColor(newColorName) {
                    val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange>(msg = newColorName) { true }
                    val setting = settingChangeEvent.setting
                    assertInstanceOf<DesktopSetting.AccentColor>(setting, newColorName)
                    assertEquals(1.0, setting.value.alpha, newColorName)
                    assertTrue(newColors.add(setting.value), newColorName)
                }
                if (isChanged) {
                    awaitEventOfType<Event.DesktopSettingChange> {
                        assertEquals(initialAccentColor, it.setting)
                        true
                    }
                } else {
                    assertFalse(matchedOriginal)
                    matchedOriginal = true
                }
            }
        }

        val initialAudibleBellChanged = !initialAudibleBell.value
        Dconf.withChangedAudibleBell(initialAudibleBellChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.AudibleBell }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.AudibleBell>(setting)
            assertEquals(initialAudibleBellChanged, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialAudibleBell == it.setting
        }

        for (changed in DesktopTitlebarAction.entries.filter { it != initialActionDoubleClickTitlebar.value }) {
            Dconf.withChangedActionDoubleClickTitlebar(changed) {
                awaitEventOfType<Event.DesktopSettingChange> { (it.setting as? DesktopSetting.ActionDoubleClickTitlebar)?.value == changed }
            }
            awaitEventOfType<Event.DesktopSettingChange> {
                initialActionDoubleClickTitlebar == it.setting
            }
        }

        for (changed in DesktopTitlebarAction.entries.filter { it != initialActionMiddleClickTitlebar.value }) {
            Dconf.withChangedActionMiddleClickTitlebar(changed) {
                awaitEventOfType<Event.DesktopSettingChange> { (it.setting as? DesktopSetting.ActionMiddleClickTitlebar)?.value == changed }
            }
            awaitEventOfType<Event.DesktopSettingChange> {
                initialActionMiddleClickTitlebar == it.setting
            }
        }

        for (changed in DesktopTitlebarAction.entries.filter { it != initialActionRightClickTitlebar.value }) {
            Dconf.withChangedActionRightClickTitlebar(changed) {
                awaitEventOfType<Event.DesktopSettingChange> { (it.setting as? DesktopSetting.ActionRightClickTitlebar)?.value == changed }
            }
            awaitEventOfType<Event.DesktopSettingChange> {
                initialActionRightClickTitlebar == it.setting
            }
        }

        val changedColorScheme = when (initialColorScheme.value) {
            ColorSchemeValue.NoPreference -> ColorSchemeValue.PreferDark
            ColorSchemeValue.PreferDark -> ColorSchemeValue.PreferLight
            ColorSchemeValue.PreferLight -> ColorSchemeValue.NoPreference
        }
        Dconf.withChangedColorScheme(changedColorScheme) {
            awaitEventOfType<Event.DesktopSettingChange> {
                DesktopSetting.ColorScheme(changedColorScheme) == it.setting
            }
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialColorScheme == it.setting
        }

        initialSettings.contrast?.let { initialContrast ->
            val changed = when (initialContrast.value) {
                XdgDesktopContrast.NoPreference -> XdgDesktopContrast.High
                XdgDesktopContrast.High -> XdgDesktopContrast.NoPreference
            }
            Dconf.withChangedContrast(changed) {
                awaitEventOfType<Event.DesktopSettingChange> {
                    DesktopSetting.Contrast(changed) == it.setting
                }
            }
            awaitEventOfType<Event.DesktopSettingChange> {
                initialContrast == it.setting
            }
        }

        val initialCursorBlinkChanged = !initialCursorBlink.value
        Dconf.withChangedCursorBlink(initialCursorBlinkChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.CursorBlink }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.CursorBlink>(setting)
            assertEquals(initialCursorBlinkChanged, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialCursorBlink == it.setting
        }

        val initialCursorBlinkTimeChanged = initialCursorBlinkTime.value + 100.milliseconds
        Dconf.withChangedCursorBlinkTime(initialCursorBlinkTimeChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.CursorBlinkTime }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.CursorBlinkTime>(setting)
            assertEquals(initialCursorBlinkTimeChanged, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialCursorBlinkTime == it.setting
        }

        val initialCursorBlinkTimeoutChanged = initialCursorBlinkTimeout.value + 2.seconds
        Dconf.withChangedCursorBlinkTimeout(initialCursorBlinkTimeoutChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.CursorBlinkTimeout }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.CursorBlinkTimeout>(setting)
            assertEquals(initialCursorBlinkTimeoutChanged, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialCursorBlinkTimeout == it.setting
        }

        val initialCursorThemeChanged = initialCursorTheme.value + "Test"
        Dconf.withChangedCursorTheme(initialCursorThemeChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.CursorTheme }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.CursorTheme>(setting)
            assertEquals(initialCursorThemeChanged, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialCursorTheme == it.setting
        }

        val initialCursorSizeChanged = initialCursorSize.value + 24U
        Dconf.withChangedCursorSize(initialCursorSizeChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.CursorSize }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.CursorSize>(setting)
            assertEquals(initialCursorSizeChanged, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialCursorSize == it.setting
        }

        initialSettings.doubleClickInterval?.let { initialDoubleClickInterval ->
            val initialDoubleClickIntervalChanged = initialDoubleClickInterval.value + 100.milliseconds
            Dconf.withChangedDoubleClick(initialDoubleClickIntervalChanged) {
                val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.DoubleClickInterval }
                val setting = settingChangeEvent.setting
                assertInstanceOf<DesktopSetting.DoubleClickInterval>(setting)
                assertEquals(initialDoubleClickIntervalChanged, setting.value)
            }
            awaitEventOfType<Event.DesktopSettingChange> {
                initialDoubleClickInterval == it.setting
            }
        }

        initialSettings.dragThresholdPixels?.let { initialDragThresholdPixels ->
            val initialDragThresholdPixelsChanged = initialDragThresholdPixels.value + 5
            Dconf.withChangedDragThresholdPixelsChanged(initialDragThresholdPixelsChanged) {
                val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.DragThresholdPixels }
                val setting = settingChangeEvent.setting
                assertInstanceOf<DesktopSetting.DragThresholdPixels>(setting)
                assertEquals(initialDragThresholdPixelsChanged, setting.value)
            }
            awaitEventOfType<Event.DesktopSettingChange> {
                initialDragThresholdPixels == it.setting
            }
        }

        for (changed in FontRgbaOrderValue.entries.filter { it != initialFontRgbaOrder.value }) {
            Dconf.withChangedFontRgbaOrder(changed) {
                awaitEventOfType<Event.DesktopSettingChange> { (it.setting as? DesktopSetting.FontRgbaOrder)?.value == changed }
            }
            awaitEventOfType<Event.DesktopSettingChange> {
                initialFontRgbaOrder == it.setting
            }
        }

        val initialMiddleClickPasteChanged = !initialMiddleClickPaste.value
        Dconf.withChangedMiddleClickPaste(initialMiddleClickPasteChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.MiddleClickPaste }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.MiddleClickPaste>(setting)
            assertEquals(initialMiddleClickPasteChanged, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialMiddleClickPaste == it.setting
        }

        for (changed in FontAntialiasingValue.entries.filter { it != initialFontAntialiasing.value }) {
            Dconf.withChangedFontAntialiasing(changed) {
                awaitEventOfType<Event.DesktopSettingChange> { (it.setting as? DesktopSetting.FontAntialiasing)?.value == changed }
            }
            awaitEventOfType<Event.DesktopSettingChange> {
                initialFontAntialiasing == it.setting
            }
        }

        for (changed in FontHintingValue.entries.filter { it != initialFontHinting.value }) {
            Dconf.withChangedFontHinting(changed) {
                awaitEventOfType<Event.DesktopSettingChange> { (it.setting as? DesktopSetting.FontHinting)?.value == changed }
            }
            awaitEventOfType<Event.DesktopSettingChange> {
                initialFontHinting == it.setting
            }
        }

        val initialOverlayScrollingChanged = !initialOverlayScrolling.value
        Dconf.withChangedOverlayScrolling(initialOverlayScrollingChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.OverlayScrolling }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.OverlayScrolling>(setting)
            assertEquals(initialOverlayScrollingChanged, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialOverlayScrolling == it.setting
        }

        val changedEnableAnimations = !initialEnableAnimations.value
        Dconf.withChangedEnableAnimations(changedEnableAnimations) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.EnableAnimations }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.EnableAnimations>(setting)
            assertEquals(changedEnableAnimations, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialEnableAnimations == it.setting
        }

        val changedRecentFilesEnabled = !initialRecentFilesEnabled.value
        Dconf.withChangedRecentFilesEnabled(changedRecentFilesEnabled) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.RecentFilesEnabled }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.RecentFilesEnabled>(setting)
            assertEquals(changedRecentFilesEnabled, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialRecentFilesEnabled == it.setting
        }

        val changedRecentFilesMaxAgeDays = initialRecentFilesMaxAgeDays.value + 2
        Dconf.withChangedRecentFilesMaxAgeDays(changedRecentFilesMaxAgeDays) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.RecentFilesMaxAgeDays }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.RecentFilesMaxAgeDays>(setting)
            assertEquals(changedRecentFilesMaxAgeDays, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialRecentFilesMaxAgeDays == it.setting
        }

        initialSettings.reducedMotion?.let { initial ->
            val changed = when (initial.value) {
                XdgDesktopReducedMotion.NoPreference -> XdgDesktopReducedMotion.ReducedMotion
                XdgDesktopReducedMotion.ReducedMotion -> XdgDesktopReducedMotion.NoPreference
            }
            Dconf.withChangedReducedMotion(changed) {
                awaitEventOfType<Event.DesktopSettingChange> {
                    DesktopSetting.ReducedMotion(changed) == it.setting
                }
            }
            awaitEventOfType<Event.DesktopSettingChange> {
                initial == it.setting
            }
        }

        assertEquals("appmenu:close", initialTitlebarLayout.value)
        val changedTitlebarLayout = "close,spacer,maximize:minimize"
        Dconf.withChangedTitlebarLayout(changedTitlebarLayout) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { it.setting is DesktopSetting.TitlebarLayout }
            val setting = settingChangeEvent.setting
            assertInstanceOf<DesktopSetting.TitlebarLayout>(setting)
            assertEquals(changedTitlebarLayout, setting.value)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            initialTitlebarLayout == it.setting
        }

        // TODO: test scale change
    }

    @Test
    fun exitWithoutClosingWindow() {
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams()
        ui { app.createWindow(windowParams) }

        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }
        testSuccessful = true
    }

    internal fun checkNextEvents(timeout: Duration = 5.seconds, checks: Map<String, (Event, Set<String>) -> Boolean>): List<Event> {
        val remainingChecks = checks.toMutableMap()
        log("checkNextEvents: ${remainingChecks.keys}")
        val otherEvents = mutableListOf<Event>()
        waitUntilEq(emptySet()) {
            val event = checkNotNull(eventQueue.poll(timeout.inWholeMilliseconds, TimeUnit.MILLISECONDS)) {
                withTimestamp("Timed out waiting for event, remaining: ${remainingChecks.keys}")
            }
            val matchedName = remainingChecks.firstNotNullOfOrNull { (name, predicate) ->
                if (predicate(event, remainingChecks.keys)) name else null
            }
            if (matchedName == null) {
                otherEvents.add(event)
            } else {
                remainingChecks.remove(matchedName)
            }
            remainingChecks.keys
        }
        return otherEvents
    }

    @Test
    fun testWindowCloseRequest() {
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams()
        createWindowAndWaitForFocus(windowParams)

        wm.closeWindow(windowParams.title)
        withNextEvent { event ->
            assertInstanceOf<Event.WindowCloseRequest>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }
    }

    @Test
    fun testSingleWindowCreation() {
        run(defaultApplicationConfig())

        val screens = ui { app.allScreens() }.screens
//        log("$screens")
        val screen = screens.firstOrNull()
        assertNotNull(screen)
        val fullscreenWindowSize = screen.size
        assertEquals(0.0, screen.origin.x.rawLogical)
        assertEquals(0.0, screen.origin.y.rawLogical)
        if (screen.name != "WL-1") {
            assertEquals("HEADLESS-1", screen.name)
        }
        assertNotEquals(0, screen.screenId)

        val windowParams = defaultWindowParams().copy(
            minSize = LogicalSize(width = LogicalPixelsInt(100), height = LogicalPixelsInt(70)),
        )
        val requestedSize = checkNotNull(windowParams.size)
        var windowTitle = windowParams.title
        val window = ui { app.createWindow(windowParams) }
        val wmVersion = wm.getVersion()

        var expectedConfigureEvent = ExpectedWindowConfigure(
            wmVersion.checkWindowCapabilities,
            windowId = windowParams.windowId,
            size = requestedSize,
            active = false,
            maximized = false,
            fullscreen = false,
            decorationMode = WindowDecorationMode.Server,
        )

        withNextEvent { event ->
            expectedConfigureEvent.assertEquals(event)
        }

        var scale = Scale.NO_SCALE

        withNextEvent { event ->
            assertNotNull(event)
            assertInstanceOf<Event.WindowDraw>(event)
            assertEquals(windowParams.windowId, event.windowId)
            // https://github.com/swaywm/sway/issues/7668
            checkPhysicalSize(requestedSize, Scale.NO_SCALE, event.size)
        }

        withNextEvent { event ->
            val keyboardEnteredEvent = if (event is Event.WindowScaleChanged) {
                assertEquals(windowParams.windowId, event.windowId)
                scale = event.newScale
                getNextEvent()
            } else {
                event
            }
            assertInstanceOf<Event.WindowKeyboardEnter>(keyboardEnteredEvent)
            assertEquals(windowParams.windowId, keyboardEnteredEvent.windowId)
        }

        withNextEvent { event ->
            assertInstanceOf<Event.ModifiersChanged>(event)
            assertEquals(emptySet<KeyModifiers>(), event.modifiers)
        }

        withNextEvent { event ->
            assertInstanceOf<Event.TextInputAvailability>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertTrue(event.available)
        }

        expectedConfigureEvent = expectedConfigureEvent.copy(active = true)

        withNextEvent { event ->
            val configureEvent = if (event is Event.WindowDraw) {
                assertEquals(windowParams.windowId, event.windowId)
                checkPhysicalSize(requestedSize, scale, event.size)
                getNextEvent()
            } else {
                event
            }
            expectedConfigureEvent.assertEquals(configureEvent)
        }

        val otherEvents = checkNextEvents(
            checks = mapOf(
                "WindowScreenChange" to { event, _ ->
                    val matchesType = event is Event.WindowScreenChange
                    if (matchesType) {
                        assertEquals(windowParams.windowId, event.windowId)
                        assertEquals(screen.screenId, event.newScreenId)
                    }
                    matchesType
                },
            ),
        )

        checkEvents(
            otherEvents,
            checks = mapOf(
                "WindowScaleChanged" to { event, _ ->
                    val matchesType = event is Event.WindowScaleChanged
                    if (matchesType) {
                        assertEquals(windowParams.windowId, event.windowId)
                        scale = event.newScale
                    }
                    matchesType
                },
                "WindowConfigure" to { event, _ ->
                    val matchesType = event is Event.WindowConfigure
                    if (matchesType) {
                        expectedConfigureEvent.assertEquals(event)
                    }
                    matchesType
                },
            ),
        )

        wmVersion.checkWindowScale(wm.getWindowState(windowTitle).output, scale)

        var mousePos = TestMousePosition(wm.getWindowState(windowTitle).clientArea.pos, LogicalPixelsInt(10), LogicalPixelsInt(10))
        moveMouseTo(mousePos)

        withNextEvent { event ->
            val mouseEnteredEvent = if (event is Event.WindowDraw) {
                assertEquals(windowParams.windowId, event.windowId)
                checkPhysicalSize(requestedSize, scale, event.size)

                getNextEvent()
            } else {
                event
            }
            assertInstanceOf<Event.MouseEntered>(mouseEnteredEvent)
            assertEquals(windowParams.windowId, mouseEnteredEvent.windowId)
            mousePos.assertEquals(mouseEnteredEvent.locationInWindow)
        }

        withMouseButtonDown(MouseButton.LEFT) {
            withNextEvent { event ->
                assertInstanceOf<Event.MouseDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(MouseButton.LEFT, event.button)
                mousePos.assertEquals(event.locationInWindow)
                assertNotEquals(Duration.ZERO, event.timestamp.toDuration())
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.MouseUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(MouseButton.LEFT, event.button)
        }

        mousePos = TestMousePosition(
            GlobalPosition.Zero,
            lastScreenSize!!.width - LogicalPixelsInt(50),
            lastScreenSize!!.height - LogicalPixelsInt(50),
        )
        moveMouseTo(mousePos)
        withNextEvent { event ->
            val mouseExitedEvent = if (event is Event.WindowDraw) {
                assertEquals(windowParams.windowId, event.windowId)
                checkPhysicalSize(requestedSize, scale, event.size)
                getNextEvent()
            } else {
                event
            }
            assertInstanceOf<Event.MouseExited>(mouseExitedEvent)
            assertEquals(windowParams.windowId, mouseExitedEvent.windowId)
            // TODO? locationInWindow
        }

        assertEquals(windowTitle, wm.getWindowState(windowTitle).title)
        "New title 🙂".also {
            ui { window.setTitle(it) }
            windowTitle = it
            waitUntilEq(it) { wm.getWindowState(windowTitle).title }
        }

        for (useWm in listOf(true, false)) {
            log("Fullscreen enable (useWm=$useWm)")
            if (useWm) {
                wm.fullScreenWindow(windowTitle)
            } else {
                ui { window.setFullScreen() }
            }

            expectedConfigureEvent = expectedConfigureEvent.copy(fullscreen = true, size = fullscreenWindowSize)
            withNextEvent { event ->
                expectedConfigureEvent.assertEquals(event, "fullscreen configure, useWm=$useWm")
            }

            withNextEvent { event ->
                assertNotNull(event)
                assertInstanceOf<Event.WindowDraw>(event)
                assertEquals(windowParams.windowId, event.windowId)
                checkPhysicalSize(lastScreenSize!!, scale, event.size)
            }

            withNextEvent { event ->
                val mouseEnteredEvent = if (event is Event.WindowScreenChange) {
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(screen.screenId, event.newScreenId)
                    getNextEvent()
                } else {
                    event
                }
                assertInstanceOf<Event.MouseEntered>(mouseEnteredEvent)
                assertEquals(windowParams.windowId, mouseEnteredEvent.windowId)
                mousePos.assertEquals(mouseEnteredEvent.locationInWindow)
            }

            ui {
                eventQueue.poll()?.let { event ->
                    expectedConfigureEvent.assertEquals(event, "fullscreen configure, useWm=$useWm")
                }
            }

            log("Fullscreen disable (useWm=$useWm)")
            if (useWm) {
                wm.unsetFullScreenWindow(windowTitle)
            } else {
                ui { window.unsetFullScreen() }
            }

            expectedConfigureEvent = expectedConfigureEvent.copy(fullscreen = false, size = requestedSize)

            withNextEvent { event ->
                expectedConfigureEvent.assertEquals(event, "fullscreen exit configure, useWm=$useWm")
            }

            withNextEvent { event ->
                assertNotNull(event)
                assertInstanceOf<Event.WindowDraw>(event)
                assertEquals(windowParams.windowId, event.windowId)
                checkPhysicalSize(requestedSize, scale, event.size)
            }

            withNextEvent { event ->
                val mouseExitedEvent = if (event is Event.WindowScreenChange) {
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(screen.screenId, event.newScreenId)
                    getNextEvent()
                } else {
                    event
                }
                assertInstanceOf<Event.MouseExited>(mouseExitedEvent)
                assertEquals(windowParams.windowId, mouseExitedEvent.windowId)
            }

            ui {
                eventQueue.poll()?.let { event ->
                    expectedConfigureEvent.assertEquals(event, "fullscreen exit configure, useWm=$useWm")
                }
            }
        }

        ui { window.close() }
        waitForWindowClosed(windowParams.windowId)
        withKeyPress(KeyCode.A) {}
        assertTrue(eventQueue.isEmpty(), "Event queue empty, but contained ${eventQueue.firstOrNull()}")
    }

    @Test
    fun testWindowSetPointerShape() {
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams()
        val initialWindowData = createWindowAndWaitForFocus(windowParams)
        val window = initialWindowData.window

        val screenshots = mutableListOf<Path>()
        val tempDir = Files.createTempDirectory("test_linux_screenshots")
        val clientAreaPos = wm.getWindowState(windowParams.title).clientArea.pos

        val screenshotRect =
            LogicalRect(x = clientAreaPos.x, y = clientAreaPos.y, width = LogicalPixelsInt(150), height = LogicalPixelsInt(150))

        ui { app.setCursorTheme("phinger-cursors-light", 48U) }
        ui {}

        val screenSize = checkNotNull(lastScreenSize)
        moveMouseTo(TestMousePosition(GlobalPosition.Zero, screenSize.width, screenSize.height))
        val blank = tempDir.resolve("_blank.png").also {
            screenshot(it, screenshotRect, hideCursor = false)
            screenshots.add(it)
        }

        val mouseLoc = TestMousePosition(clientAreaPos, LogicalPixelsInt(100), LogicalPixelsInt(100))
        moveMouseTo(mouseLoc)
        awaitEventOfType<Event.MouseEntered> { true }

        val errors = mutableListOf<String>()
        for (shape in PointerShape.entries) {
            ui { window.setPointerShape(shape) }
            lastDrawEvents.clear()
            awaitEventOfType<Event.WindowDraw> { true }

            tempDir.resolve("$shape.png").also { shapeScreenshotPath ->
                var retryCount = 20
                while (retryCount > 0) {
                    screenshot(shapeScreenshotPath, screenshotRect, hideCursor = false)
                    val same = screenshots.find {
                        val isIdentical = Files.mismatch(shapeScreenshotPath, it) == -1L
                        if (shape == PointerShape.Hidden && it == blank) {
                            !isIdentical
                        } else {
                            isIdentical
                        }
                    }
                    if (same == null) {
                        break
                    }
                    retryCount -= 1
                    if (retryCount == 0) {
                        errors.add("$shapeScreenshotPath and $same are identical")
                        break
                    }
                    log("Retrying $shape (same as $same)")
                    // For animated cursors, such as Progress and Wait, for some reason we need to wait a bit before they can be screenshotted
                    Thread.sleep(100)
                }
                screenshots.add(shapeScreenshotPath)
            }
        }

        assertTrue(errors.isEmpty(), errors.joinToString("\n"))
//        screenshots.forEach { it.deleteIfExists() }
//        tempDir.deleteIfExists()
        testSuccessful = true
    }

    @Test
    fun testMultipleMouseButtonsAtOnce() {
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams()
        createWindowAndWaitForFocus(windowParams)
        moveMouseInsideWindow(windowParams.title)

        withMouseButtonDown(MouseButton.LEFT) {
            withNextEvent { event ->
                assertInstanceOf<Event.MouseDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(MouseButton.LEFT, event.button)
            }
            withMouseButtonDown(MouseButton.RIGHT) {
                withNextEvent { event ->
                    assertInstanceOf<Event.MouseDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(MouseButton.RIGHT, event.button)
                }
                withMouseButtonDown(MouseButton.MIDDLE) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.MouseDown>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertEquals(MouseButton.MIDDLE, event.button)
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.MouseUp>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(MouseButton.MIDDLE, event.button)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.MouseUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(MouseButton.RIGHT, event.button)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.MouseUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(MouseButton.LEFT, event.button)
        }
        testSuccessful = true
    }

    @Test
    fun testWindowCreationLargeWindowId() {
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams().copy(windowId = WindowId.MAX_VALUE)
        val w = createWindowAndWaitForFocus(windowParams).window

        ui { w.close() }

        awaitEventOfType<Event.WindowClosed> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }

        testSuccessful = true
    }

    @Test
    fun testWindowCreationTitleTooLong() {
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams().copy(
            title = "t".repeat(Application.MAX_STRING_SIZE_BYTES + 1),
        )
        assertThrows<IllegalArgumentException> { ui { app.createWindow(windowParams) } }

        testSuccessful = true
    }

    @Test
    fun testWindowCreationAppIdTooLong() {
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams().copy(
            appId = "a".repeat(Application.MAX_STRING_SIZE_BYTES + 1),
        )
        assertThrows<IllegalArgumentException> { ui { app.createWindow(windowParams) } }

        testSuccessful = true
    }

    @Test
    fun testWindowCreationLargeSizeValues() {
        run(defaultApplicationConfig())

        val maxVal = LogicalPixelsInt(Int.MAX_VALUE)

        val windowParams = defaultWindowParams().copy(
            size = LogicalSize(width = maxVal, height = maxVal),
            minSize = LogicalSize(width = maxVal, height = maxVal),
        )
        val w = ui { app.createWindow(windowParams) }

        ui { w.close() }

        awaitEventOfType<Event.WindowClosed> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }

        testSuccessful = true
    }

    @Test
    fun testMultipleWindowCreationWithSameId() {
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams()
        ui { app.createWindow(windowParams) }

        assertThrows<Error> { ui { app.createWindow(windowParams) } }
        testSuccessful = true
    }

    @Test
    fun testWindowDecorationsToggle() {
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams().copy(
            clientSideDecorationFrame = WindowFrame(
                padding = WindowFrame.Padding.withAll(LogicalPixelsInt(25)),
                resizerThickness = WindowFrame.ResizerThickness.withAll(LogicalPixelsInt(12)),
            ),
        )

        val requestedWindowSize = checkNotNull(windowParams.size)
        val window = createWindowAndWaitForFocus(windowParams).window

        val wmVersion = wm.getVersion()
        wm.moveWindow(windowParams.title, GlobalPosition(LogicalPixelsInt(200), LogicalPixelsInt(200)))
        val wmWindowState = wm.getWindowState(windowParams.title)

        var expectedWindowConfigure = ExpectedWindowConfigure(
            wmVersion.checkWindowCapabilities,
            windowId = windowParams.windowId,
            size = requestedWindowSize,
            active = true,
            maximized = false,
            fullscreen = false,
            decorationMode = WindowDecorationMode.Server,
        )

        val sizeWithOriginalFrame = LogicalSize(
            wmWindowState.clientArea.size.width + wmWindowState.padding.left + wmWindowState.padding.right +
                windowParams.clientSideDecorationFrame.padding.left +
                windowParams.clientSideDecorationFrame.padding.right,
            wmWindowState.clientArea.size.height + wmWindowState.padding.top + wmWindowState.padding.bottom +
                windowParams.clientSideDecorationFrame.padding.top +
                windowParams.clientSideDecorationFrame.padding.bottom,
        )
        expectedWindowConfigure = expectedWindowConfigure.copy(
            decorationMode = WindowDecorationMode.Client(
                windowParams.clientSideDecorationFrame,
                tiled = WindowFrameTiling.withAll(false),
            ),
            size = sizeWithOriginalFrame,
        )
        ui {
            window.setPreferClientSideDecoration(true)
        }
        awaitEventOfType<Event.WindowConfigure>(msg = "Change window 1 decoration mode to client-side") { event ->
            windowParams.windowId == event.windowId &&
                run {
                    expectedWindowConfigure.assertEquals(event, "Change window 1 decoration mode to client-side")
                    true
                }
        }

        val smallerWindowFrame = WindowFrame(
            padding = WindowFrame.Padding.withAll(LogicalPixelsInt.Zero),
            resizerThickness = WindowFrame.ResizerThickness.withAll(LogicalPixelsInt.Zero),
        )

        expectedWindowConfigure = expectedWindowConfigure.copy(
            decorationMode = WindowDecorationMode.Client(frame = smallerWindowFrame, tiled = WindowFrameTiling.withAll(false)),
            size = LogicalSize(
                wmWindowState.clientArea.size.width + wmWindowState.padding.left + wmWindowState.padding.right,
                wmWindowState.clientArea.size.height + wmWindowState.padding.top + wmWindowState.padding.bottom,
            ),
        )
        ui {
            lastDrawEvents.clear()
            window.setClientSideDecorationFrame(smallerWindowFrame)
        }
        awaitEventOfType<Event.WindowConfigure>(msg = "Shrink window frame") { event ->
            windowParams.windowId == event.windowId &&
                run {
                    expectedWindowConfigure.assertEquals(event, "Shrink window frame")
                    true
                }
        }
        awaitEventOfType<Event.WindowDraw> { true }

        expectedWindowConfigure = expectedWindowConfigure.copy(
            decorationMode = WindowDecorationMode.Client(
                windowParams.clientSideDecorationFrame,
                tiled = WindowFrameTiling.withAll(false),
            ),
            size = sizeWithOriginalFrame,
        )
        ui {
            lastDrawEvents.clear()
            window.setClientSideDecorationFrame(windowParams.clientSideDecorationFrame)
        }
        awaitEventOfType<Event.WindowConfigure>(msg = "Grow window frame") { event ->
            windowParams.windowId == event.windowId &&
                run {
                    expectedWindowConfigure.assertEquals(event, "Grow window frame")
                    true
                }
        }
        awaitEventOfType<Event.WindowDraw> { true }

        expectedWindowConfigure = expectedWindowConfigure.copy(
            decorationMode = WindowDecorationMode.Server,
            size = requestedWindowSize,
        )
        ui {
            window.setPreferClientSideDecoration(false)
        }
        awaitEventOfType<Event.WindowConfigure>(msg = "Change window decoration mode to server-side") { event ->
            windowParams.windowId == event.windowId &&
                run {
                    expectedWindowConfigure.assertEquals(event, "Change window decoration mode to server-side")
                    true
                }
        }
    }

    @Test
    fun testMultipleWindowCreation() {
        run(defaultApplicationConfig())

        val window1Params = defaultWindowParams()
        val requestedWindow1Size = checkNotNull(window1Params.size)
        val window1 = ui { app.createWindow(window1Params) }

        awaitEventOfType<Event.WindowDraw>(msg = "Draw first window") { event ->
            assertEquals(window1Params.windowId, event.windowId, "Draw first window: $event")
            true
        }

        val wmVersion = wm.getVersion()
        var expectedWindow1ConfigureEvent = ExpectedWindowConfigure(
            wmVersion.checkWindowCapabilities,
            windowId = window1Params.windowId,
            size = requestedWindow1Size,
            active = true,
            maximized = false,
            fullscreen = false,
            decorationMode = WindowDecorationMode.Server,
        )

        awaitEventOfType<Event.WindowConfigure>(msg = "First window active") { event ->
            assertEquals(window1Params.windowId, event.windowId)
            if (event.active) {
                expectedWindow1ConfigureEvent.assertEquals(event, "First window active")
                true
            } else {
                false
            }
        }

        val requestedWindow2Size = LogicalSize(width = LogicalPixelsInt(300), height = LogicalPixelsInt(200))
        val window2Params = WindowParams(
            windowId = 1,
            title = "Test Window 2",
            size = requestedWindow2Size,
            minSize = null,
            preferClientSideDecoration = true,
            renderingMode = RenderingMode.Software,
            appId = APP_ID,
            clientSideDecorationFrame = WindowFrame(
                padding = WindowFrame.Padding.withAll(LogicalPixelsInt(25)),
                resizerThickness = WindowFrame.ResizerThickness.withAll(LogicalPixelsInt(12)),
            ),
        )
        val window2 = ui { app.createWindow(window2Params) }

        awaitEventOfType<Event.WindowDraw>(msg = "Draw second window") { event ->
            window2Params.windowId == event.windowId
        }

        var expectedWindow2ConfigureEvent = ExpectedWindowConfigure(
            wmVersion.checkWindowCapabilities,
            windowId = window2Params.windowId,
            size = requestedWindow2Size.copy(
                width = requestedWindow2Size.width +
                    window2Params.clientSideDecorationFrame.padding.left +
                    window2Params.clientSideDecorationFrame.padding.right,
                height = requestedWindow2Size.height +
                    window2Params.clientSideDecorationFrame.padding.top +
                    window2Params.clientSideDecorationFrame.padding.bottom,
            ),
            active = true,
            maximized = false,
            fullscreen = false,
            decorationMode = WindowDecorationMode.Client(
                window2Params.clientSideDecorationFrame,
                tiled = WindowFrameTiling.withAll(false),
            ),
        )

        expectedWindow1ConfigureEvent = expectedWindow1ConfigureEvent.copy(active = false)
        checkNextEvents(
            checks = mapOf(
                "Second window active" to { event, _ ->
                    if (event is Event.WindowConfigure && event.windowId == window2Params.windowId && event.active) {
                        expectedWindow2ConfigureEvent.assertEquals(event, "Second window active")
                        true
                    } else {
                        false
                    }
                },
                "First window no longer active" to { event, _ ->
                    if (event is Event.WindowConfigure && event.windowId == window1Params.windowId && !event.active) {
                        expectedWindow1ConfigureEvent.assertEquals(event, "First window no longer active")
                        true
                    } else {
                        false
                    }
                },
            ),
        )

        withKeyPress(KeyCode.Tab) {
            awaitEvent({ it as? Event.KeyDown }, timeout = 100.milliseconds) { event ->
                assertEquals(KeyCode.Tab, event.keyCode.value)
                assertEquals(KeySym.Tab, event.key.value)
                assertFalse(event.isRepeat)
                assertEquals("\t", event.characters)
                true
            }
            val request = ui { window2.requestActivationToken() }
            assertNotNull(request)
            awaitEventOfType<Event.ActivationTokenResponse>(timeout = 100.milliseconds) { event ->
                assertEquals(request, event.requestId)
                ui { window1.activate(event.token) }
                true
            }
            awaitEvent({ it as? Event.WindowKeyboardLeave }, timeout = 100.milliseconds) { event ->
                assertEquals(window2Params.windowId, event.windowId)
                true
            }
            awaitEvent({ it as? Event.WindowKeyboardEnter }, timeout = 100.milliseconds) { event ->
                assertEquals(window1Params.windowId, event.windowId)
                val keyCode = event.keyCodes.single()
                assertEquals(KeyCode.Tab, keyCode.value)
                val keySym = event.keySyms.single()
                assertEquals(KeySym.Tab, keySym.value)
                true
            }
        }

        awaitEventOfType<Event.WindowConfigure>(msg = "Second window no longer active after activating first window") { event ->
            window2Params.windowId == event.windowId && !event.active
        }
        awaitEventOfType<Event.WindowConfigure>(msg = "First window active after activating it") { event ->
            window1Params.windowId == event.windowId && event.active
        }

        withKeyPress(KeyCode.Tab) {
            awaitEvent({ it as? Event.KeyDown }, timeout = 100.milliseconds) { event ->
                assertEquals(KeyCode.Tab, event.keyCode.value)
                assertEquals(KeySym.Tab, event.key.value)
                assertFalse(event.isRepeat)
                assertEquals("\t", event.characters)
                true
            }
            val request = ui { window1.requestActivationToken() }
            assertNotNull(request)
            awaitEventOfType<Event.ActivationTokenResponse>(timeout = 100.milliseconds) { event ->
                assertEquals(request, event.requestId)
                ui { window2.activate(event.token) }
                true
            }
            awaitEvent({ it as? Event.WindowKeyboardLeave }, timeout = 100.milliseconds) { event ->
                assertEquals(window1Params.windowId, event.windowId)
                true
            }
            awaitEvent({ it as? Event.WindowKeyboardEnter }, timeout = 100.milliseconds) { event ->
                assertEquals(window2Params.windowId, event.windowId)
                val keyCode = event.keyCodes.single()
                assertEquals(KeyCode.Tab, keyCode.value)
                val keySym = event.keySyms.single()
                assertEquals(KeySym.Tab, keySym.value)
                true
            }
        }

        awaitEventOfType<Event.WindowConfigure>(msg = "First window no longer active after activating second window") { event ->
            window1Params.windowId == event.windowId && !event.active
        }
        awaitEventOfType<Event.WindowConfigure>(msg = "Second window active after activating it") { event ->
            window2Params.windowId == event.windowId && event.active
        }
        awaitEventOfType<Event.KeyUp> { true }

        wm.tileWindows(listOf(window2Params.title, window1Params.title))

        expectedWindow1ConfigureEvent =
            expectedWindow1ConfigureEvent.copy(
                size = wm.getWindowState(window1Params.title).clientArea.size,
            )

        expectedWindow2ConfigureEvent =
            expectedWindow2ConfigureEvent.copy(
                decorationMode = WindowDecorationMode.Server,
                size = wm.getWindowState(window2Params.title).clientArea.size,
            )

        checkNextEvents(
            checks = mapOf(
                "Window 1 tiled" to { event, _ ->
                    if (event is Event.WindowConfigure &&
                        event.windowId == window1Params.windowId &&
                        event.size == expectedWindow1ConfigureEvent.size
                    ) {
                        expectedWindow1ConfigureEvent.assertEquals(event)
                        true
                    } else {
                        false
                    }
                },
                "Window 2 tiled" to { event, _ ->
                    if (event is Event.WindowConfigure &&
                        event.windowId == window2Params.windowId &&
                        event.size == expectedWindow2ConfigureEvent.size
                    ) {
                        expectedWindow2ConfigureEvent.assertEquals(event)
                        true
                    } else {
                        false
                    }
                },
            ),
        )

        ui { window1.close() }
        awaitEventOfType<Event.WindowClosed>(msg = "First window closed") { event ->
            assertEquals(window1Params.windowId, event.windowId)
            true
        }

        ui { window2.close() }
        awaitEventOfType<Event.WindowClosed>(msg = "Second window closed") { event ->
            assertEquals(window2Params.windowId, event.windowId)
            true
        }
        testSuccessful = true
    }

    @Test
    fun testSetClipboardContentForText() {
        val textContent = "a".repeat(65537).toByteArray()
        val htmlContent = """<meta http-equiv="content-type" content="text/html; charset=utf-8"><p>normal, <b>bold</b>.</p>""".toByteArray()
        val content = mapOf(
            TEXT_UTF8_MIME_TYPE to textContent,
            HTML_TEXT_MIME_TYPE to htmlContent,
        )
        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    when (dataSource) {
                        DataSource.Clipboard -> content[mimeType]
                        else -> null
                    }
                },
            ),
        )
        val keyboardEnter = createWindowAndWaitForFocus(defaultWindowParams()).keyboardEnter

        withSetClipboardContent(keyboardEnter.serial, listOf(HTML_TEXT_MIME_TYPE, TEXT_UTF8_MIME_TYPE)) {
            listClipboardFormats().also {
                val expected = """text/html
text/plain;charset=utf-8
"""
                assertEquals(expected, it?.decodeToString())
            }

//            assertArrayEquals(htmlContent, getClipboardContent(HTML_TEXT_MIME_TYPE))
            assertArrayEquals(textContent, getClipboardContent(TEXT_UTF8_MIME_TYPE))

            ui { app.clipboardPut(listOf(HTML_TEXT_MIME_TYPE, TEXT_UTF8_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransferAvailable>(event)
                assertEquals(DataSource.Clipboard, event.dataSource)
                assertIterableEquals(listOf(HTML_TEXT_MIME_TYPE, TEXT_UTF8_MIME_TYPE), event.mimeTypes)
            }
        }
        testSuccessful = true
    }

    @Test
    fun testSetClipboardContentForUnicodeText() {
        val textContent = """woman and man holding hands: medium-dark skin tone, medium skin tone:
            \uD83D\uDC69\uD83C\uDFFE\u200D\uD83E\uDD1D\u200D\uD83D\uDC68\uD83C\uDFFD""".toByteArray()

        val htmlContent = """<meta http-equiv="content-type" content="text/html; charset=utf-8">
            <p>health worker: medium-light skin tone: \uD83E\uDDD1\uD83C\uDFFC\u200D\u2695\uFE0F</p>
        """.toByteArray()

        val content = mapOf(
            TEXT_UTF8_MIME_TYPE to textContent,
            HTML_TEXT_MIME_TYPE to htmlContent,
        )
        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    when (dataSource) {
                        DataSource.Clipboard -> content[mimeType]
                        else -> null
                    }
                },
            ),
        )
        val keyboardEnter = createWindowAndWaitForFocus(defaultWindowParams()).keyboardEnter

        withSetClipboardContent(keyboardEnter.serial, listOf(HTML_TEXT_MIME_TYPE, TEXT_UTF8_MIME_TYPE)) {
            listClipboardFormats().also {
                val expected = """text/html
text/plain;charset=utf-8
"""
                assertEquals(expected, it?.decodeToString())
            }

            assertArrayEquals(htmlContent, getClipboardContent(HTML_TEXT_MIME_TYPE))
            assertArrayEquals(textContent, getClipboardContent(TEXT_UTF8_MIME_TYPE))
        }
        testSuccessful = true
    }

    @Test
    fun testSetClipboardContentForStringWithNull() {
        val textContent = byteArrayOf(0.toByte()) + "test clipboard content".toByteArray()
        val content = mapOf(
            TEXT_UTF8_MIME_TYPE to textContent,
        )
        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    when (dataSource) {
                        DataSource.Clipboard -> content[mimeType]
                        else -> null
                    }
                },
            ),
        )
        val keyboardEnter = createWindowAndWaitForFocus(defaultWindowParams()).keyboardEnter

        withSetClipboardContent(keyboardEnter.serial, listOf(TEXT_UTF8_MIME_TYPE)) {
            listClipboardFormats().also {
                val expected = "text/plain;charset=utf-8\n"
                assertEquals(expected, it?.decodeToString())
            }

            assertArrayEquals(textContent, getClipboardContent(TEXT_UTF8_MIME_TYPE))
        }
        testSuccessful = true
    }

    @Test
    fun testSetClipboardContentThenOtherApplicationSetsClipboard() {
        val textContent = "test clipboard content".toByteArray()
        val content = mapOf(TEXT_UTF8_MIME_TYPE to textContent)
        val mimeTypes = listOf(content.keys.single())

        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    when (dataSource) {
                        DataSource.Clipboard -> content[mimeType]
                        else -> null
                    }
                },
            ),
        )

        val windowParams = defaultWindowParams()
        val keyboardEnter = createWindowAndWaitForFocus(windowParams).keyboardEnter

        withSetClipboardContent(keyboardEnter.serial, mimeTypes) {
            withClipboardSourceTestApp(
                listOf(HTML_TEXT_MIME_TYPE to sequenceOf("<p>other app text</p>".encodeToByteArray())),
            ) { testAppData ->
                wm.focusWindow(testAppData.windowTitle)
                // Trigger the test app clipboard copy
                withKeyPress(KeyCode.C) {}
                awaitEvent({ it as? Event.DataTransferCancelled }) {
                    it.dataSource == DataSource.Clipboard
                }
            }
        }

        awaitEvent({ it as? Event.DataTransferAvailable }) {
            it.dataSource == DataSource.Clipboard && it.mimeTypes == listOf(HTML_TEXT_MIME_TYPE)
        }

        testSuccessful = true
    }

    @Test
    fun testSetPrimarySelectionContentThenOtherApplicationSetsPrimarySelection() {
        val textContent = "test clipboard content".toByteArray()
        val content = mapOf(TEXT_UTF8_MIME_TYPE to textContent)
        val mimeTypes = listOf(content.keys.single())

        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    when (dataSource) {
                        DataSource.PrimarySelection -> content[mimeType]
                        else -> null
                    }
                },
            ),
        )

        val windowParams = defaultWindowParams()
        val keyboardEnter = createWindowAndWaitForFocus(windowParams).keyboardEnter

        withSetPrimarySelectionContent(keyboardEnter.serial, mimeTypes) {
            withPrimarySelectionSourceTestApp(
                listOf(HTML_TEXT_MIME_TYPE to sequenceOf("<p>other app text</p>".encodeToByteArray())),
            ) { testAppData ->
                wm.focusWindow(testAppData.windowTitle)
                // Trigger the test app clipboard copy
                withKeyPress(KeyCode.C) {}
                awaitEvent({ it as? Event.DataTransferCancelled }) {
                    it.dataSource == DataSource.PrimarySelection
                }
            }
        }

        testSuccessful = true
    }

    @Test
    fun testSetPrimarySelectionContentForText() {
        val textContent = "test clipboard content".toByteArray()
        val htmlContent = """<meta http-equiv="content-type" content="text/html; charset=utf-8"><p>normal, <b>bold</b>.</p>""".toByteArray()
        val content = mapOf(
            TEXT_UTF8_MIME_TYPE to textContent,
            HTML_TEXT_MIME_TYPE to htmlContent,
        )
        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    when (dataSource) {
                        DataSource.PrimarySelection -> content[mimeType]
                        else -> {
                            log("Unexpected data source: $dataSource")
                            null
                        }
                    }
                },
            ),
        )
        val keyboardEnter = createWindowAndWaitForFocus(defaultWindowParams()).keyboardEnter

        val mimeTypes = listOf(HTML_TEXT_MIME_TYPE, TEXT_UTF8_MIME_TYPE)
        withSetPrimarySelectionContent(keyboardEnter.serial, mimeTypes) {
            listPrimarySelectionFormats().also {
                val expected = """text/html
text/plain;charset=utf-8
"""
                assertEquals(expected, it?.decodeToString())
            }

            assertArrayEquals(htmlContent, getPrimarySelectionContent(HTML_TEXT_MIME_TYPE))
            assertArrayEquals(textContent, getPrimarySelectionContent(TEXT_UTF8_MIME_TYPE))

            ui { app.primarySelectionPut(mimeTypes) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransferAvailable>(event)
                assertEquals(DataSource.PrimarySelection, event.dataSource)
                assertIterableEquals(mimeTypes, event.mimeTypes)
            }
        }
        testSuccessful = true
    }

    @Test
    fun testSetClipboardContentForNonExistingType() {
        val textContent = "test clipboard content".toByteArray()
        val content = mapOf(
            TEXT_UTF8_MIME_TYPE to textContent,
        )
        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    when (dataSource) {
                        DataSource.Clipboard -> content[mimeType]
                        else -> null
                    }
                },
            ),
        )
        val keyboardEnter = createWindowAndWaitForFocus(defaultWindowParams()).keyboardEnter

        withSetClipboardContent(keyboardEnter.serial, listOf(TEXT_UTF8_MIME_TYPE)) {
            assertNull(getClipboardContent(HTML_TEXT_MIME_TYPE))
        }
        testSuccessful = true
    }

    @Test
    fun testSetPrimarySelectionContentForNonExistingType() {
        val textContent = "test clipboard content".toByteArray()
        val content = mapOf(
            TEXT_UTF8_MIME_TYPE to textContent,
        )
        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    when (dataSource) {
                        DataSource.Clipboard -> content[mimeType]
                        else -> null
                    }
                },
            ),
        )
        val keyboardEnter = createWindowAndWaitForFocus(defaultWindowParams()).keyboardEnter

        withSetPrimarySelectionContent(keyboardEnter.serial, listOf(TEXT_UTF8_MIME_TYPE)) {
            assertNull(getPrimarySelectionContent(HTML_TEXT_MIME_TYPE))
        }
        testSuccessful = true
    }

    @Test
    fun testClipboardPasteToSelf() {
        val textContent = "a".repeat(200_000).toByteArray()
        val content = mapOf(
            TEXT_UTF8_MIME_TYPE to textContent,
        )
        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    when (dataSource) {
                        DataSource.Clipboard -> content[mimeType]
                        else -> null
                    }
                },
            ),
        )
        val keyboardEnter = createWindowAndWaitForFocus(defaultWindowParams()).keyboardEnter

        withSetClipboardContent(keyboardEnter.serial, listOf(TEXT_UTF8_MIME_TYPE)) {
            val transferSerial1 = 5
            ui { app.clipboardPaste(transferSerial1, listOf(TEXT_UTF8_MIME_TYPE, PNG_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial1, event.serial)
                val content = event.content
                assertNotNull(content)
                assertEquals(TEXT_UTF8_MIME_TYPE, content.mimeType)
                assertArrayEquals(textContent, content.data)
            }

            val transferSerial2 = 6
            ui { app.clipboardPaste(transferSerial2, listOf(PNG_MIME_TYPE, HTML_TEXT_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial2, event.serial)
                assertNull(event.content)
            }
        }
    }

    @Test
    fun testClipboardPasteToSelfForNonExistingType() {
        run(defaultApplicationConfig())
        val keyboardEnter = createWindowAndWaitForFocus(defaultWindowParams()).keyboardEnter

        withSetClipboardContent(keyboardEnter.serial, listOf(TEXT_UTF8_MIME_TYPE)) {
            val transferSerial = 6
            ui { app.clipboardPaste(transferSerial, listOf(PNG_MIME_TYPE, TEXT_UTF8_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial, event.serial)
                val content = event.content
                assertNotNull(content)
                assertEquals(TEXT_UTF8_MIME_TYPE, content.mimeType)
                assertArrayEquals(byteArrayOf(), content.data)
            }
        }
    }

    private fun testClipboardPasteImpl(
        data: List<Pair<String, Sequence<ByteArray>>>,
        operationMode: TestAppOperationMode = TestAppOperationMode.Normal,
        block: () -> Unit,
    ) {
        run(defaultApplicationConfig())
        val mimeTypes = data.map { it.first }
        withClipboardSourceTestApp(data, operationMode) { testAppData ->
            wm.focusWindow(testAppData.windowTitle)
            // Trigger the test app clipboard copy
            withKeyPress(KeyCode.C) {}

            val initialWindowData = createWindowAndWaitForFocus(defaultWindowParams())

            initialWindowData.otherEvents.firstNotNullOfOrNull { it as? Event.DataTransferAvailable }
                ?: awaitEventOfType<Event.DataTransferAvailable> { true }.let { event ->
                    assertEquals(DataSource.Clipboard, event.dataSource)
                    assertEquals(mimeTypes, event.mimeTypes)
                }

            val availableMimeTypes = ui { app.clipboardGetAvailableMimeTypes() }
            assertEquals(mimeTypes, availableMimeTypes)

            block()
        }
        testSuccessful = true
    }

    @Test
    fun testClipboardPaste() {
        val htmlData = "<p>Text from <b>TestAppClipboardSource</b></p>".encodeToByteArray()
        val uriListData =
            $$"file:///some/path/With%20Spaces/&%20$p%E2%82%AC%C2%A2%C3%AF%C3%A5%C5%82%20%C3%A7%C4%A7%C4%81%C5%99%C3%9F\r\nfile:///tmp/%5BScreenshot%20from%2012:04:42%5D.png\r\n"
                .encodeToByteArray()
        val textData = $$"/some/path/With Spaces/& $p€¢ïåł çħāřß\n/tmp/[Screenshot from 12:04:42].png".encodeToByteArray()

        testClipboardPasteImpl(
            listOf(
                HTML_TEXT_MIME_TYPE to sequenceOf(htmlData),
                URI_LIST_MIME_TYPE to sequenceOf(uriListData),
                TEXT_UTF8_MIME_TYPE to sequenceOf(textData),
            ),
        ) {
            val transferSerial1 = 5
            ui { app.clipboardPaste(transferSerial1, listOf(TEXT_UTF8_MIME_TYPE, PNG_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial1, event.serial)
                val content = event.content
                assertNotNull(content)
                assertEquals(TEXT_UTF8_MIME_TYPE, content.mimeType)
                assertArrayEquals(textData, content.data)
            }

            val transferSerial2 = 6
            ui { app.clipboardPaste(transferSerial2, listOf(URI_LIST_MIME_TYPE, TEXT_UTF8_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial2, event.serial)
                val content = event.content
                assertNotNull(content)
                assertEquals(URI_LIST_MIME_TYPE, content.mimeType)
                assertArrayEquals(uriListData, content.data)
            }

            val transferSerial3 = 6
            ui { app.clipboardPaste(transferSerial3, listOf(PNG_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial3, event.serial)
                assertNull(event.content)
            }
        }
    }

    @Test
    fun testClipboardPasteLargeData() {
        val singleUriListData = "file:///tmp\r\n".encodeToByteArray()
        val singleTextData = "qwertyuiop".encodeToByteArray()
        val repetitions = 10000

        val uriListData = (0..repetitions).asSequence().map { singleUriListData }
        val textData = (0..repetitions).asSequence().map { singleTextData }
        val data = listOf(URI_LIST_MIME_TYPE to uriListData, TEXT_UTF8_MIME_TYPE to textData)
        testClipboardPasteImpl(data) {
            val transferSerial1 = 5
            ui { app.clipboardPaste(transferSerial1, listOf(TEXT_UTF8_MIME_TYPE, PNG_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial1, event.serial)
                val content = event.content
                assertNotNull(content)
                assertEquals(TEXT_UTF8_MIME_TYPE, content.mimeType)
                assertDataMatches(textData, content.data)
            }

            val transferSerial2 = 6
            ui { app.clipboardPaste(transferSerial2, listOf(URI_LIST_MIME_TYPE, TEXT_UTF8_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial2, event.serial)
                val content = event.content
                assertNotNull(content)
                assertEquals(URI_LIST_MIME_TYPE, content.mimeType)
                assertDataMatches(uriListData, content.data)
            }

            val transferSerial3 = 6
            ui { app.clipboardPaste(transferSerial3, listOf(PNG_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial3, event.serial)
                assertNull(event.content)
            }
        }
    }

    @Test
    fun testClipboardPasteSlowSource() {
        val textData = "qwertyuiop".repeat(10).encodeToByteArray()
        val data = listOf(TEXT_UTF8_MIME_TYPE to sequenceOf(textData))

        testClipboardPasteImpl(data, TestAppOperationMode.Slow) {
            val transferSerial1 = 5
            ui { app.clipboardPaste(transferSerial1, listOf(TEXT_UTF8_MIME_TYPE)) }

            var uiCallbackCount = 0
            var eventOrNull: Event? = null
            val timeTaken = measureTime {
                waitUntilEq(true) {
                    eventOrNull = eventQueue.poll(10, TimeUnit.MILLISECONDS)
                    ui { uiCallbackCount += 1 }
                    eventOrNull != null
                }
            }
            log("clipboard paste took $timeTaken, uiCallbackCount=$uiCallbackCount")

            val event = eventOrNull
            assertInstanceOf<Event.DataTransfer>(event)
            assertEquals(transferSerial1, event.serial)
            val content = event.content
            assertNotNull(content)
            assertEquals(TEXT_UTF8_MIME_TYPE, content.mimeType)
            assertArrayEquals(textData, content.data)

            // Test app sleeps 100ms after every 10 bytes
            if (timeTaken < 1.seconds) {
                fail("Test data source implementation is too fast ($timeTaken)")
            }

            if (uiCallbackCount < 20) {
                fail("Too few UI callbacks: $uiCallbackCount")
            }
        }
    }

    @Test
    fun testClipboardPasteSourceDisappears() {
        val textData = "qwertyuiop".repeat(10).encodeToByteArray()
        val data = listOf(TEXT_UTF8_MIME_TYPE to sequenceOf(textData))
        testClipboardPasteImpl(data, TestAppOperationMode.ExitAfterStartWriting) {
            val transferSerial1 = 50
            ui { app.clipboardPaste(transferSerial1, listOf(TEXT_UTF8_MIME_TYPE)) }

            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial1, event.serial)

                // TODO: try to figure out how to prevent this (partial data read). But even GTK on Wayland behaves the same.
                val content = event.content
                assertNotNull(content)
                assertEquals(TEXT_UTF8_MIME_TYPE, content.mimeType)
                assertFalse(content.data.isEmpty())
                assertNotEquals(textData.size, content.data.size)
            }
        }
    }

    @Test
    fun testPrimarySelectionPaste() {
        run(defaultApplicationConfig())
        val htmlData = "<p>Text from <b>TestAppPrimarySelectionSource</b></p>".encodeToByteArray()
        val textData = "Text from TestAppPrimarySelectionSource".encodeToByteArray()

        withPrimarySelectionSourceTestApp(
            listOf(
                HTML_TEXT_MIME_TYPE to sequenceOf(htmlData),
                TEXT_UTF8_MIME_TYPE to sequenceOf(textData),
            ),
        ) { testAppData ->
            wm.focusWindow(testAppData.windowTitle)
            // Trigger the test app primary selection copy
            withKeyPress(KeyCode.C) {}

            val initialWindowData = createWindowAndWaitForFocus(defaultWindowParams())

            initialWindowData.otherEvents.firstNotNullOfOrNull { it as? Event.DataTransferAvailable }
                ?: awaitEventOfType<Event.DataTransferAvailable> { true }.let { event ->
                    assertEquals(DataSource.PrimarySelection, event.dataSource)
                    assertEquals(listOf(HTML_TEXT_MIME_TYPE, TEXT_UTF8_MIME_TYPE), event.mimeTypes)
                }

            val availableMimeTypes = ui { app.primarySelectionGetAvailableMimeTypes() }
            assertEquals(listOf(HTML_TEXT_MIME_TYPE, TEXT_UTF8_MIME_TYPE), availableMimeTypes)

            val transferSerial = 9
            ui { app.primarySelectionPaste(transferSerial, listOf(TEXT_UTF8_MIME_TYPE, PNG_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial, event.serial)
                val content = event.content
                assertNotNull(content)
                assertEquals(TEXT_UTF8_MIME_TYPE, content.mimeType)
                assertArrayEquals(textData, content.data)
            }
        }
        testSuccessful = true
    }

    @Test
    fun testPasteWithoutClipboard() {
        run(defaultApplicationConfig())
        ui { app.clipboardGetAvailableMimeTypes() }.also {
            assertEquals(emptyList<String>(), it)
        }
        val serial = 6
        ui { app.clipboardPaste(serial, listOf(TEXT_UTF8_MIME_TYPE)) }
        withNextEvent { event ->
            assertInstanceOf<Event.DataTransfer>(event, "event")
            assertEquals(serial, event.serial, "event serial")
            assertNull(event.content, "event content")
        }

        testSuccessful = true
    }

    @Test
    fun testPasteWithoutPrimarySelection() {
        run(defaultApplicationConfig())
        ui { app.primarySelectionGetAvailableMimeTypes() }.also {
            assertEquals(emptyList<String>(), it)
        }

        val serial = 6
        ui { app.primarySelectionPaste(serial, listOf(TEXT_UTF8_MIME_TYPE)) }
        withNextEvent { event ->
            assertInstanceOf<Event.DataTransfer>(event, "event")
            assertEquals(serial, event.serial, "event serial")
            assertNull(event.content, "event content")
        }
        testSuccessful = true
    }

    @Test
    fun testKeyEvents() {
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams()
        createWindowAndWaitForFocus(windowParams)

//        Thread.sleep(120000)
//        moveMouseTo(100, 100)
//        withNextEvent { event ->
//            assertInstanceOf<Event.MouseEntered>(event)
//            assertEquals(windowParams.windowId, event.windowId)
//        }
//        withNextEvent { event ->
//            assertInstanceOf<Event.MouseMoved>(event)
//            assertEquals(windowParams.windowId, event.windowId)
//        }

        withKeyPress(KeyCode.A) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals("a", event.characters)
                assertEquals(KeyCode.A, event.keyCode.value)
                assertEquals(KeySym.a, event.key.value)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
        }

        withKeyPress(KeyCode.Return) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(KeyCode.Return, event.keyCode.value)
                assertEquals(KeySym.Return, event.key.value)
                assertArrayEquals("\r".toByteArray(), event.characters?.toByteArray())
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Return, event.keyCode.value)
            assertEquals(KeySym.Return, event.key.value)
        }

        withKeyPress(KeyCode.Escape) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(KeyCode.Escape, event.keyCode.value)
                assertEquals(KeySym.Escape, event.key.value)
                assertArrayEquals("\u001b".toByteArray(), event.characters?.toByteArray())
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Escape, event.keyCode.value)
            assertEquals(KeySym.Escape, event.key.value)
        }

        withKeyPress(KeyCode.BackSpace) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(KeyCode.BackSpace, event.keyCode.value)
                assertEquals(KeySym.BackSpace, event.key.value)
                assertArrayEquals("\b".toByteArray(), event.characters?.toByteArray())
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.BackSpace, event.keyCode.value)
            assertEquals(KeySym.BackSpace, event.key.value)
        }

        withKeyPress(KeyCode.Tab) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(KeyCode.Tab, event.keyCode.value)
                assertEquals(KeySym.Tab, event.key.value)
                assertArrayEquals("\t".toByteArray(), event.characters?.toByteArray())
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Tab, event.keyCode.value)
            assertEquals(KeySym.Tab, event.key.value)
        }

        withKeyPress(KeyCode.Shift_L) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertNull(event.characters)
                assertEquals(KeyCode.Shift_L, event.keyCode.value)
                assertEquals(KeySym.Shift_L, event.key.value)
            }
            withNextEvent { event ->
                assertInstanceOf<Event.ModifiersChanged>(event)
                assertEquals(setOf(KeyModifiers.Shift), event.modifiers)
            }
            withKeyPress(KeyCode.A) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals("A", event.characters)
                    assertEquals(KeyCode.A, event.keyCode.value)
                    assertEquals(KeySym.A, event.key.value)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(KeyCode.A, event.keyCode.value)
                assertEquals(KeySym.A, event.key.value)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Shift_L, event.keyCode.value)
            assertEquals(KeySym.Shift_L, event.key.value)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.ModifiersChanged>(event)
            assertEquals(emptySet<KeyModifiers>(), event.modifiers)
        }

        withKeyPress(KeyCode.Control_L) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertNull(event.characters)
                assertEquals(KeyCode.Control_L, event.keyCode.value)
                assertEquals(KeySym.Control_L, event.key.value)
            }
            withNextEvent { event ->
                assertInstanceOf<Event.ModifiersChanged>(event)
                assertEquals(setOf(KeyModifiers.Control), event.modifiers)
            }
            withKeyPress(KeyCode.A) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertArrayEquals(byteArrayOf(1), event.characters?.encodeToByteArray())
                    assertEquals(KeyCode.A, event.keyCode.value)
                    assertEquals(KeySym.a, event.key.value)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(KeyCode.A, event.keyCode.value)
                assertEquals(KeySym.a, event.key.value)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Control_L, event.keyCode.value)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.ModifiersChanged>(event)
            assertEquals(emptySet<KeyModifiers>(), event.modifiers)
        }
        testSuccessful = true
    }

    @Test
    fun testTextInputContext() {
        var textInputContext = TextInputContext(
            contentPurpose = TextInputContentPurpose.Normal,
            cursorRectangle = LogicalRect.makeXYWH(x = 50, y = 20, width = 5, height = 10),
            surroundingText = "",
            cursorCodepointOffset = 0U,
            selectionStartCodepointOffset = 0U,
            hints = setOf(TextInputContentHint.Completion, TextInputContentHint.Spellcheck),
            changeCausedByInputMethod = false,
        )
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams()

        createWindowAndWaitForFocus(windowParams)

        assertEquals(
            "content_purpose: Normal, content_hints: [Completion, Spellcheck]",
            withSyncGetInputMethodState { ui { app.textInputEnable(textInputContext) } },
        )
        textInputContext = textInputContext.copy(
            hints = setOf(
                TextInputContentHint.Completion,
                TextInputContentHint.Spellcheck,
                TextInputContentHint.AutoCapitalization,
                TextInputContentHint.Lowercase,
                TextInputContentHint.Uppercase,
                TextInputContentHint.Titlecase,
                TextInputContentHint.HiddenText,
                TextInputContentHint.SensitiveData,
                TextInputContentHint.Latin,
                TextInputContentHint.Multiline,
            ),
        )

        assertEquals(
            "content_purpose: Normal, content_hints: [" +
                "Completion, Spellcheck, AutoCapitalization, Lowercase, Uppercase, Titlecase, HiddenText, SensitiveData, Latin, Multiline]",
            withSyncGetInputMethodState { ui { app.textInputUpdate(textInputContext) } },
        )

        textInputContext = textInputContext.copy(
            contentPurpose = TextInputContentPurpose.Alpha,
            hints = setOf(TextInputContentHint.Completion),
        )
        assertEquals(
            "content_purpose: Alpha, content_hints: [Completion]",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext = textInputContext.copy(
            contentPurpose = TextInputContentPurpose.Digits,
            hints = setOf(TextInputContentHint.Spellcheck),
        )
        assertEquals(
            "content_purpose: Digits, content_hints: [Spellcheck]",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext = textInputContext.copy(
            contentPurpose = TextInputContentPurpose.Number,
            hints = setOf(TextInputContentHint.AutoCapitalization),
        )
        assertEquals(
            "content_purpose: Number, content_hints: [AutoCapitalization]",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext = textInputContext.copy(
            contentPurpose = TextInputContentPurpose.Phone,
            hints = setOf(TextInputContentHint.Lowercase),
            cursorRectangle = LogicalRect.makeXYWH(x = 1000000000, y = 1000000000, width = 1000000000, height = 1000000000),
        )
        assertEquals(
            "content_purpose: Phone, content_hints: [Lowercase]",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext =
            textInputContext.copy(contentPurpose = TextInputContentPurpose.Url, hints = setOf(TextInputContentHint.Uppercase))
        assertEquals(
            "content_purpose: Url, content_hints: [Uppercase]",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext = textInputContext.copy(
            contentPurpose = TextInputContentPurpose.Email,
            hints = setOf(TextInputContentHint.Titlecase),
        )
        assertEquals(
            "content_purpose: Email, content_hints: [Titlecase]",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext = textInputContext.copy(
            contentPurpose = TextInputContentPurpose.Name,
            hints = setOf(TextInputContentHint.HiddenText),
            cursorRectangle = LogicalRect.makeXYWH(x = 1, y = 2, width = 3, height = 4),
        )
        assertEquals(
            "content_purpose: Name, content_hints: [HiddenText]",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext = textInputContext.copy(
            contentPurpose = TextInputContentPurpose.Password,
            hints = setOf(TextInputContentHint.SensitiveData),
        )
        assertEquals(
            "content_purpose: Password, content_hints: [SensitiveData]",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext =
            textInputContext.copy(contentPurpose = TextInputContentPurpose.Pin, hints = setOf(TextInputContentHint.SensitiveData))
        assertEquals(
            "content_purpose: Pin, content_hints: [SensitiveData]",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext =
            textInputContext.copy(contentPurpose = TextInputContentPurpose.Date, hints = emptySet())
        assertEquals(
            "content_purpose: Date, content_hints: []",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext =
            textInputContext.copy(contentPurpose = TextInputContentPurpose.Time, hints = emptySet())
        assertEquals(
            "content_purpose: Time, content_hints: []",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext =
            textInputContext.copy(contentPurpose = TextInputContentPurpose.Datetime, hints = setOf(TextInputContentHint.Latin))
        assertEquals(
            "content_purpose: Datetime, content_hints: [Latin]",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        textInputContext =
            textInputContext.copy(contentPurpose = TextInputContentPurpose.Terminal, hints = setOf(TextInputContentHint.Multiline))
        assertEquals(
            "content_purpose: Terminal, content_hints: [Multiline]",
            withSyncGetInputMethodState {
                ui { app.textInputUpdate(textInputContext) }
            },
        )

        testSuccessful = true
    }

    @Test
    fun testIme() {
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams()

        var textInputContext = TextInputContext(
            contentPurpose = TextInputContentPurpose.Normal,
            cursorRectangle = LogicalRect.makeXYWH(x = 200, y = 100, width = 10, height = 20),
            surroundingText = "",
            cursorCodepointOffset = 0U,
            selectionStartCodepointOffset = 0U,
            hints = setOf(TextInputContentHint.Multiline),
            changeCausedByInputMethod = false,
        )

        createWindowAndWaitForFocus(windowParams)
        withSyncGetInputMethodState {
            ui { app.textInputEnable(textInputContext) }
        }

        val keyCodes = mapOf(
            KeyCode.`1` to Pair("1", KeySym.`1`),
            KeyCode.`2` to Pair("2", KeySym.`2`),
            KeyCode.`3` to Pair("3", KeySym.`3`),
            KeyCode.`4` to Pair("4", KeySym.`4`),
            KeyCode.`5` to Pair("5", KeySym.`5`),
            KeyCode.`6` to Pair("6", KeySym.`6`),
            KeyCode.`7` to Pair("7", KeySym.`7`),
            KeyCode.`8` to Pair("8", KeySym.`8`),
            KeyCode.`9` to Pair("9", KeySym.`9`),
            KeyCode.`0` to Pair("0", KeySym.`0`),
            KeyCode.minus to Pair("-", KeySym.minus),
            KeyCode.equal to Pair("=", KeySym.equal),
            KeyCode.Q to Pair("q", KeySym.q),
            KeyCode.W to Pair("w", KeySym.w),
            KeyCode.E to Pair("e", KeySym.e),
            KeyCode.R to Pair("r", KeySym.r),
            KeyCode.T to Pair("t", KeySym.t),
            KeyCode.Y to Pair("y", KeySym.y),
            KeyCode.U to Pair("u", KeySym.u),
            KeyCode.I to Pair("i", KeySym.i),
            KeyCode.O to Pair("o", KeySym.o),
            KeyCode.P to Pair("p", KeySym.p),
            KeyCode.bracketleft to Pair("[", KeySym.bracketleft),
            KeyCode.bracketright to Pair("]", KeySym.bracketright),
            KeyCode.A to Pair("a", KeySym.a),
            KeyCode.S to Pair("s", KeySym.s),
            KeyCode.D to Pair("d", KeySym.d),
            KeyCode.F to Pair("f", KeySym.f),
            KeyCode.G to Pair("g", KeySym.g),
            KeyCode.H to Pair("h", KeySym.h),
            KeyCode.J to Pair("j", KeySym.j),
            KeyCode.K to Pair("k", KeySym.k),
            KeyCode.L to Pair("l", KeySym.l),
            KeyCode.semicolon to Pair(";", KeySym.semicolon),
            KeyCode.apostrophe to Pair("'", KeySym.apostrophe),
            KeyCode.backslash to Pair("\\", KeySym.backslash),
            KeyCode.Z to Pair("z", KeySym.z),
            KeyCode.X to Pair("x", KeySym.x),
            KeyCode.C to Pair("c", KeySym.c),
            KeyCode.V to Pair("v", KeySym.v),
            KeyCode.B to Pair("b", KeySym.b),
            KeyCode.N to Pair("n", KeySym.n),
            KeyCode.M to Pair("m", KeySym.m),
            KeyCode.comma to Pair(",", KeySym.comma),
            KeyCode.period to Pair(".", KeySym.period),
            KeyCode.slash to Pair("/", KeySym.slash),
        )

        for ((keyCode, charAndKeySym) in keyCodes) {
            val (c, keySym) = charAndKeySym
            withKeyPress(keyCode) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event, c)
                    assertEquals(c, event.characters)
                    assertEquals(keyCode, event.keyCode.value)
                    assertEquals(keySym, event.key.value)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event, c)
                assertEquals(keyCode, event.keyCode.value)
                assertEquals(keySym, event.key.value)
            }
        }

        doVirtualDeviceEvent(
            "input?preedit_string_text=${URLEncoder.encode("`", "utf-8")}&preedit_string_cursor_begin=1&preedit_string_cursor_end=1",
        )
        withNextEvent { event ->
            assertInstanceOf<Event.TextInput>(event)
            val preeditStringData = event.preeditStringData
            assertNotNull(preeditStringData)
            assertEquals("`", preeditStringData.text)
            assertEquals(1, preeditStringData.cursorBeginBytePos)
            assertEquals(1, preeditStringData.cursorEndBytePos)
            assertNull(event.commitStringData)
            assertNull(event.deleteSurroundingTextData)
        }

        doVirtualDeviceEvent("input?commit_string=${URLEncoder.encode("è", "utf-8")}")
        withNextEvent { event ->
            assertInstanceOf<Event.TextInput>(event)
            val commitStringData = event.commitStringData
            assertNotNull(commitStringData)
            assertEquals("è", commitStringData.text)
            assertNull(event.preeditStringData)
            assertNull(event.deleteSurroundingTextData)
        }

        withKeyPress(KeyCode.Shift_L) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertNull(event.characters)
                assertEquals(KeyCode.Shift_L, event.keyCode.value)
            }
            withNextEvent { event ->
                assertInstanceOf<Event.ModifiersChanged>(event)
                assertEquals(setOf(KeyModifiers.Shift), event.modifiers)
            }
            withKeyPress(KeyCode.A) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals("A", event.characters)
                    assertEquals(KeyCode.A, event.keyCode.value)
                    assertEquals(KeySym.A, event.key.value)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(KeyCode.A, event.keyCode.value)
                assertEquals(KeySym.A, event.key.value)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Shift_L, event.keyCode.value)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.ModifiersChanged>(event)
            assertEquals(emptySet<KeyModifiers>(), event.modifiers)
        }

        withKeyPress(KeyCode.Return) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(KeyCode.Return, event.keyCode.value)
                assertEquals(KeySym.Return, event.key.value)
                assertArrayEquals("\r".toByteArray(), event.characters?.toByteArray())
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Return, event.keyCode.value)
            assertEquals(KeySym.Return, event.key.value)
        }

        withKeyPress(KeyCode.Escape) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(KeyCode.Escape, event.keyCode.value)
                assertEquals(KeySym.Escape, event.key.value)
                assertArrayEquals("\u001b".toByteArray(), event.characters?.toByteArray())
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Escape, event.keyCode.value)
            assertEquals(KeySym.Escape, event.key.value)
        }

        withKeyPress(KeyCode.BackSpace) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(KeyCode.BackSpace, event.keyCode.value)
                assertEquals(KeySym.BackSpace, event.key.value)
                assertArrayEquals("\b".toByteArray(), event.characters?.toByteArray())
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.BackSpace, event.keyCode.value)
            assertEquals(KeySym.BackSpace, event.key.value)
        }

        withKeyPress(KeyCode.Tab) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(KeyCode.Tab, event.keyCode.value)
                assertEquals(KeySym.Tab, event.key.value)
                assertArrayEquals("\t".toByteArray(), event.characters?.toByteArray())
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Tab, event.keyCode.value)
            assertEquals(KeySym.Tab, event.key.value)
        }

        withKeyPress(KeyCode.Down) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(KeyCode.Down, event.keyCode.value)
                assertEquals(KeySym.Down, event.key.value)
                assertNull(event.characters?.encodeToByteArray())
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Down, event.keyCode.value)
            assertEquals(KeySym.Down, event.key.value)
        }

        withKeyPress(KeyCode.Right) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(KeyCode.Right, event.keyCode.value)
                assertEquals(KeySym.Right, event.key.value)
                assertNull(event.characters)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Right, event.keyCode.value)
            assertEquals(KeySym.Right, event.key.value)
        }

        withKeyPress(KeyCode.Control_L) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertNull(event.characters)
                assertEquals(KeyCode.Control_L, event.keyCode.value)
                assertEquals(KeySym.Control_L, event.key.value)
            }
            withNextEvent { event ->
                assertInstanceOf<Event.ModifiersChanged>(event)
                assertEquals(setOf(KeyModifiers.Control), event.modifiers)
            }
            withKeyPress(KeyCode.A) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertArrayEquals(byteArrayOf(1), event.characters?.encodeToByteArray())
                    assertEquals(KeyCode.A, event.keyCode.value)
                    assertEquals(KeySym.a, event.key.value)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(KeyCode.A, event.keyCode.value)
                assertEquals(KeySym.a, event.key.value)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(KeyCode.Control_L, event.keyCode.value)
            assertEquals(KeySym.Control_L, event.key.value)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.ModifiersChanged>(event)
            assertEquals(emptySet<KeyModifiers>(), event.modifiers)
        }

        textInputContext = textInputContext.copy(
            surroundingText = "àéîõü",
            cursorCodepointOffset = 2U,
            selectionStartCodepointOffset = 2U,
        )
        // tell test IME engine to uppercase the letter before the cursor
        withSyncVirtualDeviceCommand("uppercase") {
            ui { app.textInputUpdate(textInputContext) }
        }

        withNextEvent { event ->
            assertInstanceOf<Event.TextInput>(event)
            val deleteSurroundingTextData = event.deleteSurroundingTextData
            assertNotNull(deleteSurroundingTextData)
            assertEquals(2U, deleteSurroundingTextData.beforeLengthInBytes)
            assertEquals(0U, deleteSurroundingTextData.afterLengthInBytes)
            assertNull(event.preeditStringData)
            val commitStringData = event.commitStringData
            assertNotNull(commitStringData)
            assertEquals("É", commitStringData.text)
        }

        textInputContext = textInputContext.copy(
            surroundingText = "àéîõü",
            cursorCodepointOffset = 4U,
            selectionStartCodepointOffset = 1U,
        )

        // tell test IME engine to uppercase the selection
        withSyncVirtualDeviceCommand("uppercase") {
            ui { app.textInputUpdate(textInputContext) }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.TextInput>(event)
            val deleteSurroundingTextData = event.deleteSurroundingTextData
            assertNotNull(deleteSurroundingTextData)
            assertEquals(6U, deleteSurroundingTextData.beforeLengthInBytes)
            assertEquals(0U, deleteSurroundingTextData.afterLengthInBytes)
            assertNull(event.preeditStringData)
            val commitStringData = event.commitStringData
            assertNotNull(commitStringData)
            assertEquals("ÉÎÕ", commitStringData.text)
        }

        // The first emoji has 5 Unicode code points
        textInputContext = textInputContext.copy(
            surroundingText = "\uD83E\uDDD1\uD83C\uDFFC\u200D\u2695\uFE0Féiõü",
            cursorCodepointOffset = 7U,
            selectionStartCodepointOffset = 7U,
        )

        // tell test IME engine to uppercase the letter before the cursor
        withSyncVirtualDeviceCommand("uppercase") {
            ui { app.textInputUpdate(textInputContext) }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.TextInput>(event)
            val deleteSurroundingTextData = event.deleteSurroundingTextData
            assertNotNull(deleteSurroundingTextData)
            assertEquals(1U, deleteSurroundingTextData.beforeLengthInBytes)
            assertEquals(0U, deleteSurroundingTextData.afterLengthInBytes)
            assertNull(event.preeditStringData)
            val commitStringData = event.commitStringData
            assertNotNull(commitStringData)
            assertEquals("I", commitStringData.text)
        }

        // The first emoji has 5 Unicode code points
        textInputContext = textInputContext.copy(
            surroundingText = "\uD83E\uDDD1\uD83C\uDFFC\u200D\u2695\uFE0Féiõü",
            cursorCodepointOffset = 6U,
            selectionStartCodepointOffset = 8U,
        )

        // tell test IME engine to uppercase the selection
        withSyncVirtualDeviceCommand("uppercase") {
            ui { app.textInputUpdate(textInputContext) }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.TextInput>(event)
            val deleteSurroundingTextData = event.deleteSurroundingTextData
            assertNotNull(deleteSurroundingTextData)
            assertEquals(0U, deleteSurroundingTextData.beforeLengthInBytes)
            assertEquals(3U, deleteSurroundingTextData.afterLengthInBytes)
            assertNull(event.preeditStringData)
            val commitStringData = event.commitStringData
            assertNotNull(commitStringData)
            assertEquals("IÕ", commitStringData.text)
        }

        // After `textInputDisable`, IME APIs should return errors.
        withSyncVirtualDeviceCommand("uppercase", expectedStatusCode = 501) {
            ui { app.textInputDisable() }
        }

        testSuccessful = true
    }

    fun fixStuckPressedButtons(button: MouseButton) {
        // See https://gitlab.freedesktop.org/wlroots/wlroots/-/commit/08e779bd85b738d993007fa6a7f3f32bebc19649
        // and https://gitlab.freedesktop.org/wlroots/wlroots/-/commit/4da4269d8f707dec3691e2ffaacc106db96780d1
        val wmVersion = wm.getVersion()
        if (wmVersion.name == "sway" && (wmVersion.major <= 1 && wmVersion.minor < 11)) {
            log("Trying to fix stuck pressed button state in compositor")
            val event = withMouseButtonDown(button) {
                awaitAnyEvent { event ->
                    event is Event.MouseExited || event is Event.MouseDown
                }
            }
            if (event is Event.MouseDown) {
                awaitEventOfType<Event.MouseUp> { true }
            }
        }
    }

    fun implTestWindowMove(button: MouseButton) {
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams()
        val initialWindowData = createWindowAndWaitForFocus(windowParams)
        val window = initialWindowData.window

        val requestedWindowSize = checkNotNull(windowParams.size)
        assertEquals(requestedWindowSize, initialWindowData.configure.size)

        val clientAreaBefore = wm.getWindowState(windowParams.title).clientArea
        assertEquals(requestedWindowSize, clientAreaBefore.size)

        val mouseLocationForOurWindow = TestMousePosition(
            clientAreaBefore.pos,
            LogicalPixelsInt(50),
            LogicalPixelsInt(50),
        )
        moveMouseTo(mouseLocationForOurWindow)

        val offsetX = LogicalPixelsInt(100)
        val offsetY = LogicalPixelsInt(50)

        withMouseButtonDown(button) {
            val mouseDown = awaitEventOfType<Event.MouseDown> { true }
            ui { window.startMove(mouseDown.serial) }
            awaitEventOfType<Event.MouseExited> { true }
            moveMouseTo(mouseLocationForOurWindow.shifted(offsetX, offsetY))
        }
        awaitEventOfType<Event.MouseEntered> { true }

        val clientAreaAfter = wm.getWindowState(windowParams.title).clientArea
        assertEquals(clientAreaBefore.size, clientAreaAfter.size)
        assertEquals(clientAreaBefore.pos.x + offsetX, clientAreaAfter.pos.x)
        assertEquals(clientAreaBefore.pos.y + offsetY, clientAreaAfter.pos.y)

        fixStuckPressedButtons(button)
    }

    @Test
    fun testWindowMoveLeftButton() {
        implTestWindowMove(MouseButton.LEFT)
    }

    @Test
    fun testWindowMoveRightButton() {
        implTestWindowMove(MouseButton.RIGHT)
    }

    @Test
    fun testWindowMoveMiddleButton() {
        implTestWindowMove(MouseButton.MIDDLE)
    }

    fun implTestWindowResizeToSmaller(
        button: MouseButton,
        moveX: LogicalPixelsInt = LogicalPixelsInt(50),
        moveY: LogicalPixelsInt = LogicalPixelsInt(100),
        windowParams: WindowParams = defaultWindowParams(),
        expectedDecreaseX: LogicalPixelsInt = moveX,
        expectedDecreaseY: LogicalPixelsInt = moveY,
    ) {
        run(defaultApplicationConfig())

        val initialWindowData = createWindowAndWaitForFocus(windowParams)
        val window = initialWindowData.window

        val requestedWindowSize = checkNotNull(windowParams.size)
        assertEquals(requestedWindowSize, initialWindowData.configure.size)

        val clientAreaBefore = wm.getWindowState(windowParams.title).clientArea
        assertEquals(requestedWindowSize, clientAreaBefore.size)

        val expectedSize = LogicalSize(
            width = (requestedWindowSize.width - expectedDecreaseX),
            height = (requestedWindowSize.height - expectedDecreaseY),
        )

        // Move the mouse to the top-left part of the window
        val mousePos = TestMousePosition(
            clientAreaBefore.pos,
            LogicalPixelsInt(5),
            LogicalPixelsInt(5),
        )
        moveMouseTo(mousePos)
        withMouseButtonDown(button) {
            val mouseDown = awaitEventOfType<Event.MouseDown> { true }
            // With Sway, it doesn't matter which edge we specify; it's dependent on the mouse position
            ui { window.startResize(mouseDown.serial, WindowResizeEdge.TopLeft) }
            awaitEventOfType<Event.MouseExited> { true }
            moveMouseTo(mousePos.shifted(moveX, moveY))
            awaitEventOfType<Event.WindowConfigure> { event ->
                event.active && event.size == expectedSize
            }
        }
        val clientAreaAfter = wm.getWindowState(windowParams.title).clientArea

        moveMouseTo(TestMousePosition(clientAreaAfter.pos, LogicalPixelsInt(4), LogicalPixelsInt(4)))
        awaitAnyEvent { event ->
            event is Event.MouseMoved || event is Event.MouseEntered
        }

        assertEquals(expectedSize, clientAreaAfter.size)
        assertEquals(clientAreaBefore.pos.x + expectedDecreaseX, clientAreaAfter.pos.x)
        assertEquals(clientAreaBefore.pos.y + expectedDecreaseY, clientAreaAfter.pos.y)

        fixStuckPressedButtons(button)
    }

    @Test
    fun testWindowResizeLeftButton() {
        implTestWindowResizeToSmaller(MouseButton.LEFT)
    }

    @Test
    fun testWindowResizeRightButton() {
        implTestWindowResizeToSmaller(MouseButton.RIGHT)
    }

    @Test
    fun testWindowResizeMiddleButton() {
        implTestWindowResizeToSmaller(MouseButton.MIDDLE)
    }

    @Test
    fun testWindowResizeWithMinSize() {
        val size = LogicalSize(width = LogicalPixelsInt(200), height = LogicalPixelsInt(300))
        val minSize = LogicalSize(width = LogicalPixelsInt(100), height = LogicalPixelsInt(70))
        val windowParams = defaultWindowParams().copy(size = size, minSize = minSize)
        implTestWindowResizeToSmaller(
            MouseButton.LEFT,
            moveX = size.width,
            moveY = size.height,
            windowParams = windowParams,
            expectedDecreaseX = size.width - minSize.width,
            expectedDecreaseY = size.height - minSize.height,
        )
    }

    @Test
    fun testDragFromWindow() {
        val dataTransferTriggered = CompletableFuture<Unit>()
        val textContent = "test clipboard content".toByteArray()
        val content = mapOf(TEXT_UTF8_MIME_TYPE to textContent)

        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    log("getDataTransferData: $dataSource, $mimeType")
                    when (dataSource) {
                        DataSource.DragAndDrop -> {
                            dataTransferTriggered.complete(Unit)
                            content[mimeType]
                        }

                        else -> null
                    }
                },
            ),
        )

        val windowParams = defaultWindowParams()
        val initialWindowData = createWindowAndWaitForFocus(windowParams)
        val window = initialWindowData.window
        wm.moveWindow(windowParams.title, GlobalPosition.Zero)
        moveMouseInsideWindow(windowParams.title)

        withDropTargetTestApp { testAppData ->
            val testAppClientArea = moveWindowTo(
                testAppData.windowTitle,
                GlobalPosition(initialWindowData.configure.size.width + LogicalPixelsInt(100), LogicalPixelsInt.Zero),
            )

            withMouseButtonDown(MouseButton.LEFT) {
                waitForWindowFocusAfterMouseDown(windowParams.windowId)
                ui {
                    window.startDragAndDrop(
                        StartDragAndDropParams(
                            mimeTypes = listOf(TEXT_UTF8_MIME_TYPE),
                            actions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                            dragIconParams = null,
                        ),
                    )
                }
                awaitEventOfType<Event.MouseExited> { true }

                val testAppLocation = TestMousePosition(testAppClientArea, LogicalPixelsInt(5), LogicalPixelsInt(5))
                assertTrue(
                    wiggleMouseUntil(testAppLocation) {
                        testAppData.readLastLine(10.milliseconds) == "Received DRAG_MOTION event"
                    },
                    "Test app produced a DRAG_MOTION event",
                )
            }

            dataTransferTriggered.get(1000, TimeUnit.MILLISECONDS)
            awaitEventOfType<Event.DragAndDropFinished> { event ->
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(DragAndDropAction.Copy, event.action)
                true
            }

            assertEquals(textContent.decodeToString(), testAppData.readLastLine(5.seconds))
        }
        testSuccessful = true
    }

    @Test
    fun testDragToWindow() {
        val queryDragAndDropTargetTriggered = LinkedBlockingQueue<DragAndDropQueryData>()
        run(
            defaultApplicationConfig(
                queryDragAndDropTarget = { data ->
                    queryDragAndDropTargetTriggered.offer(data)
                    log("queryDragAndDropTarget: $data")
                    DragAndDropQueryResponse(
                        supportedActionsPerMime = listOf(
                            SupportedActionsForMime(
                                supportedMimeType = HTML_TEXT_MIME_TYPE,
                                supportedActions = setOf(DragAndDropAction.Copy),
                                preferredAction = DragAndDropAction.Copy,
                            ),
                            // The compatible type is not first on purpose, to verify the fallback behavior
                            SupportedActionsForMime(
                                supportedMimeType = TEXT_UTF8_MIME_TYPE,
                                supportedActions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                                preferredAction = DragAndDropAction.Move,
                            ),
                        ),
                    )
                },
            ),
        )

        val windowParams = defaultWindowParams()
        val initialWindowData = createWindowAndWaitForFocus(windowParams)
        wm.moveWindow(windowParams.title, GlobalPosition.Zero)

        val mouseLocationForOurWindow =
            TestMousePosition(wm.getWindowState(windowParams.title).clientArea.pos, LogicalPixelsInt(50), LogicalPixelsInt(50))

        val textData = "Text from TestAppDragSource".encodeToByteArray()
        val expectedSourceMimeTypes = listOf("application/x-gtk-local-dnd", "text/plain;charset=utf-8", "text/plain")
        withDragSourceTestApp(
            listOf(TEXT_UTF8_MIME_TYPE to sequenceOf(textData), "text/plain" to sequenceOf(textData)),
        ) { testAppData ->
            val testAppClientArea = moveWindowTo(
                testAppData.windowTitle,
                GlobalPosition(initialWindowData.configure.size.width + LogicalPixelsInt(100), LogicalPixelsInt.Zero),
            )
            wm.focusWindow(testAppData.windowTitle)
            waitForTestAppFocus(windowParams.windowId)

            val testAppLocation = TestMousePosition(testAppClientArea, LogicalPixelsInt(5), LogicalPixelsInt(5))
            moveMouseTo(testAppLocation)
            withMouseButtonDown(MouseButton.LEFT) {
                assertEquals("TestAppDragSource drag begin", testAppData.readLastLine(5.seconds))
                assertTrue(
                    wiggleMouseUntil(mouseLocationForOurWindow) {
                        !queryDragAndDropTargetTriggered.isEmpty()
                    },
                    "queryDragAndDropTarget is triggered (1)",
                )
                for (data in queryDragAndDropTargetTriggered.drainAll()) {
                    assertEquals(windowParams.windowId, data.windowId)
                    assertNotEquals(0.0, data.locationInWindow.x.rawLogical)
                    assertNotEquals(0.0, data.locationInWindow.y.rawLogical)
                    assertEquals(expectedSourceMimeTypes, data.mimeTypes)
                    assertEquals(setOf(DragAndDropAction.Copy, DragAndDropAction.Move), data.actions)
                }

                var dragAndDropLeaveEvent: Event.DragAndDropLeave? = null
                assertTrue(
                    wiggleMouseUntil(testAppLocation) {
                        dragAndDropLeaveEvent = eventQueue.drainAll().firstNotNullOfOrNull {
                            it as? Event.DragAndDropLeave
                        }
                        dragAndDropLeaveEvent != null
                    },
                    "DragAndDropLeave events is produced",
                )
                assertEquals(windowParams.windowId, dragAndDropLeaveEvent!!.windowId)

                // This is flaky, so just clear the queue
//                assertTrue(queryDragAndDropTargetTriggered.isEmpty(), "${queryDragAndDropTargetTriggered.drainAll()}")
                queryDragAndDropTargetTriggered.clear()

                assertTrue(
                    wiggleMouseUntil(mouseLocationForOurWindow) {
                        !queryDragAndDropTargetTriggered.isEmpty()
                    },
                    "queryDragAndDropTarget is triggered (2)",
                )

                ui {}
                for (data in queryDragAndDropTargetTriggered.drainAll()) {
                    assertEquals(windowParams.windowId, data.windowId)
                    assertNotEquals(0.0, data.locationInWindow.x.rawLogical)
                    assertNotEquals(0.0, data.locationInWindow.y.rawLogical)
                    assertEquals(expectedSourceMimeTypes, data.mimeTypes)
                    assertEquals(setOf(DragAndDropAction.Copy, DragAndDropAction.Move), data.actions)
                }
            }

            awaitEventOfType<Event.DropPerformed> { event ->
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(DragAndDropAction.Move, event.action)
                val content = event.content
                assertNotNull(content)
                assertEquals(TEXT_UTF8_MIME_TYPE, content.mimeType)
                assertArrayEquals("Text from TestAppDragSource".encodeToByteArray(), content.data)
                true
            }
        }
        testSuccessful = true
    }

    @Test
    fun testDragFromWindowToNonAccepting() {
        val dataTransferTriggered = CompletableFuture<Boolean>()
        val textContent = "test clipboard content".toByteArray()
        val content = mapOf(TEXT_UTF8_MIME_TYPE to textContent)
        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    when (dataSource) {
                        DataSource.DragAndDrop -> {
                            dataTransferTriggered.complete(true)
                            content[mimeType]
                        }

                        else -> null
                    }
                },
            ),
        )

        val windowParams = defaultWindowParams()
        val initialWindowData = createWindowAndWaitForFocus(windowParams)
        wm.moveWindow(windowParams.title, GlobalPosition.Zero)
        moveMouseInsideWindow(windowParams.title)
        val window = initialWindowData.window

        withBlankWindowTestApp { testAppData ->
            val testAppClientArea = moveWindowTo(
                testAppData.windowTitle,
                GlobalPosition(initialWindowData.configure.size.width + LogicalPixelsInt(100), LogicalPixelsInt.Zero),
            )

            withMouseButtonDown(MouseButton.LEFT) {
                waitForWindowFocusAfterMouseDown(windowParams.windowId)
                ui {
                    window.startDragAndDrop(
                        StartDragAndDropParams(
                            mimeTypes = listOf(TEXT_UTF8_MIME_TYPE),
                            actions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                            dragIconParams = null,
                        ),
                    )
                }
                awaitEventOfType<Event.MouseExited> { true }

                val testAppLocation = TestMousePosition(testAppClientArea, LogicalPixelsInt(5), LogicalPixelsInt(5))
                assertTrue(
                    wiggleMouseUntil(testAppLocation) {
                        testAppData.readLastLine(10.milliseconds) == "Received DRAG_MOTION event"
                    },
                    "Test app produced a DRAG_MOTION event",
                )
            }

            awaitEventOfType<Event.DataTransferCancelled>(timeout = 5.seconds) { event ->
                assertEquals(DataSource.DragAndDrop, event.dataSource)
                true
            }
            assertFalse(dataTransferTriggered.isDone)
        }
        testSuccessful = true
    }

    @Test
    fun testDragToSameWindow() {
        val dragIconDrawTriggered = LinkedBlockingQueue<Boolean>()
        val queryDragAndDropTargetTriggered = LinkedBlockingQueue<DragAndDropQueryData>()
        val textContent = "test clipboard content".toByteArray()
        val content = mapOf(TEXT_UTF8_MIME_TYPE to textContent, PNG_MIME_TYPE to byteArrayOf())
        val advertisedMimeTypes = listOf(PNG_MIME_TYPE, TEXT_UTF8_MIME_TYPE)

        eventHandler = { event ->
            assertTrue(app.isEventLoopThread())
            when (event) {
                is Event.WindowDraw -> {
                    performSoftwareDrawing(event.size, event.softwareDrawData!!) { canvas ->
                        canvas.clear(SkColor.BLUE)
                    }
                    EventHandlerResult.Stop
                }
                is Event.DragIconDraw -> {
                    performSoftwareDrawing(event.size, event.softwareDrawData!!) { canvas ->
                        canvas.clear(SkColor.RED)
                    }
                    dragIconDrawTriggered.offer(true)
                    EventHandlerResult.Stop
                }
                else -> EventHandlerResult.Continue
            }
        }

        run(
            defaultApplicationConfig(
                getDataTransferData = { dataSource, mimeType ->
                    log("getDataTransferData: $dataSource, $mimeType")
                    when (dataSource) {
                        DataSource.DragAndDrop -> content[mimeType]
                        else -> null
                    }
                },
                queryDragAndDropTarget = { data ->
                    queryDragAndDropTargetTriggered.offer(data)
                    log("queryDragAndDropTarget: $data")
                    DragAndDropQueryResponse(
                        supportedActionsPerMime = listOf(
                            SupportedActionsForMime(
                                supportedMimeType = HTML_TEXT_MIME_TYPE,
                                supportedActions = setOf(DragAndDropAction.Copy),
                                preferredAction = DragAndDropAction.Copy,
                            ),
                            // The compatible type is not first on purpose, to verify the fallback behavior
                            SupportedActionsForMime(
                                supportedMimeType = TEXT_UTF8_MIME_TYPE,
                                supportedActions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                                preferredAction = DragAndDropAction.Move,
                            ),
                        ),
                    )
                },
            ),
        )

        val windowParams = defaultWindowParams()
        val window = createWindowAndWaitForFocus(windowParams).window
        val clientAreaTopLeftGlobalPosition = wm.getWindowState(windowParams.title).clientArea.pos
        var mousePos = TestMousePosition(clientAreaTopLeftGlobalPosition, LogicalPixelsInt(5), LogicalPixelsInt(5))

        moveMouseTo(mousePos)

        withMouseButtonDown(MouseButton.LEFT) {
            awaitEventOfType<Event.MouseDown> { true }

            ui {
                window.startDragAndDrop(
                    StartDragAndDropParams(
                        mimeTypes = advertisedMimeTypes,
                        actions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                        dragIconParams = DragIconParams(
                            renderingMode = RenderingMode.Software,
                            size = LogicalSize(LogicalPixelsInt(100), LogicalPixelsInt(50)),
                        ),
                    ),
                )
            }
            mousePos = mousePos.shifted(x = LogicalPixelsInt(10))
            moveMouseTo(mousePos)
            awaitEventOfType<Event.MouseExited> { true }
            waitUntilEq(true) { dragIconDrawTriggered.isNotEmpty() }

            mousePos = mousePos.shifted(x = LogicalPixelsInt(10))
            moveMouseTo(mousePos)
            mousePos = mousePos.shifted(x = LogicalPixelsInt(10))
            moveMouseTo(mousePos)
            waitUntilEq(true) { queryDragAndDropTargetTriggered.isNotEmpty() }
        }

        for (e in queryDragAndDropTargetTriggered) {
            assertEquals(advertisedMimeTypes, e.mimeTypes)
            assertEquals(setOf(DragAndDropAction.Copy, DragAndDropAction.Move), e.actions)
        }

        awaitEventOfType<Event.DropPerformed> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(DragAndDropAction.Move, event.action)
            val content = event.content
            assertNotNull(content)
            assertEquals(TEXT_UTF8_MIME_TYPE, content.mimeType)
            assertArrayEquals(textContent, content.data)
            mousePos.assertEquals(event.locationInWindow)
            true
        }
        awaitEventOfType<Event.DragAndDropFinished> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(DragAndDropAction.Move, event.action)
            true
        }

        ui { window.close() }
        awaitEventOfType<Event.WindowClosed> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }
        testSuccessful = true
    }

    @Test
    fun testMouseScroll() {
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams()
        createWindowAndWaitForFocus(windowParams)
        moveMouseInsideWindow(windowParams.title)

        scrollMouseDown()
        withNextEvent { event ->
            assertInstanceOf<Event.ScrollWheel>(event)
            assertEquals(windowParams.windowId, event.windowId, "scrollMouseDown: windowId")
            assertEquals(0.0, event.horizontalScroll.delta.rawLogical, "scrollMouseDown: horizontalScroll.delta")
            assertEquals(0, event.horizontalScroll.wheelValue120, "scrollMouseDown: horizontalScroll.wheelValue120")
            assertFalse(event.horizontalScroll.isInverted, "scrollMouseDown: horizontalScroll.isInverted")
            assertFalse(event.horizontalScroll.isStop, "scrollMouseDown: horizontalScroll.isStop")
            assertEquals(10.0, event.verticalScroll.delta.rawLogical, "scrollMouseDown: verticalScroll.delta")
            assertEquals(120, event.verticalScroll.wheelValue120, "scrollMouseDown: verticalScroll.wheelValue120")
            assertFalse(event.verticalScroll.isInverted, "scrollMouseDown: verticalScroll.isInverted")
            assertFalse(event.verticalScroll.isStop, "scrollMouseDown: verticalScroll.isStop")
            assertNotEquals(Duration.ZERO, event.timestamp.toDuration(), "scrollMouseDown: timestamp")
        }

        scrollMouseUp()
        withNextEvent { event ->
            assertInstanceOf<Event.ScrollWheel>(event)
            assertEquals(windowParams.windowId, event.windowId, "scrollMouseUp: windowId")
            assertEquals(0.0, event.horizontalScroll.delta.rawLogical, "scrollMouseUp: horizontalScroll.delta")
            assertEquals(0, event.horizontalScroll.wheelValue120, "scrollMouseUp: horizontalScroll.wheelValue120")
            assertFalse(event.horizontalScroll.isInverted, "scrollMouseUp: horizontalScroll.isInverted")
            assertFalse(event.horizontalScroll.isStop, "scrollMouseUp: horizontalScroll.isStop")
            assertEquals(-10.0, event.verticalScroll.delta.rawLogical, "scrollMouseUp: verticalScroll.delta")
            assertEquals(-120, event.verticalScroll.wheelValue120, "scrollMouseUp: verticalScroll.wheelValue120")
            assertFalse(event.verticalScroll.isInverted, "scrollMouseUp: verticalScroll.isInverted")
            assertFalse(event.verticalScroll.isStop, "scrollMouseUp: verticalScroll.isStop")
            assertNotEquals(Duration.ZERO, event.timestamp.toDuration(), "scrollMouseUp: timestamp")
        }
        testSuccessful = true
    }

    @Test
    fun testShowNotificationServiceStartedAfterAppThenRestarted() {
        run(defaultApplicationConfig())

        val notification1RequestId = ui {
            app.requestShowNotification(
                ShowNotificationParams(
                    title = "Test Notification 1",
                    body = "Body of Test Notification 1",
                    soundFilePath = null,
                ),
            )
        }
        assertNotNull(notification1RequestId)
        withNextEvent { event ->
            assertInstanceOf<Event.NotificationShown>(event)
            assertEquals(notification1RequestId, event.requestId)
            assertNull(event.notificationId)
        }

        withMako {
            val showNotificationParams2 = ShowNotificationParams(
                title = "Test Notification 2",
                body = "Body of Test Notification 2",
                soundFilePath = null,
            )
            val notificationRequestId = ui {
                app.requestShowNotification(showNotificationParams2)
            }
            assertNotNull(notificationRequestId)
            val notification1Id = withNextEvent { event ->
                assertInstanceOf<Event.NotificationShown>(event)
                assertEquals(notificationRequestId, event.requestId)
                assertNotNull(event.notificationId)
                event.notificationId
            }
            assertNotNull(notification1Id)

            val notificationInfoList = checkNotNull(getMakoList()).data
            assertEquals(1, notificationInfoList.size)
            assertEquals(1, notificationInfoList.first().size)
            val notificationInfo = notificationInfoList.single().single()
            assertEquals(showNotificationParams2.title, notificationInfo.summary.data)
            assertEquals(showNotificationParams2.body, notificationInfo.body.data)
            assertEquals(notification1Id.toInt(), notificationInfo.id.data)

            runCommand(listOf("makoctl", "invoke"))

            withNextEvent { event ->
                assertInstanceOf<Event.NotificationClosed>(event)
                assertEquals(notification1Id, event.notificationId)
                assertEquals("default", event.action)
            }
        }

        withMako {
            val showNotificationParams3 = ShowNotificationParams(
                title = "Test Notification 3",
                body = "Body of Test Notification 3",
                soundFilePath = null,
            )
            val notificationRequestId = ui {
                app.requestShowNotification(showNotificationParams3)
            }
            assertNotNull(notificationRequestId)
            val notification1Id = withNextEvent { event ->
                assertInstanceOf<Event.NotificationShown>(event)
                assertEquals(notificationRequestId, event.requestId)
                assertNotNull(event.notificationId)
                event.notificationId
            }
            assertNotNull(notification1Id)

            val notificationInfoList = checkNotNull(getMakoList()).data
            assertEquals(1, notificationInfoList.size)
            assertEquals(1, notificationInfoList.first().size)
            val notificationInfo = notificationInfoList.single().single()
            assertEquals(showNotificationParams3.title, notificationInfo.summary.data)
            assertEquals(showNotificationParams3.body, notificationInfo.body.data)
            assertEquals(notification1Id.toInt(), notificationInfo.id.data)

            runCommand(listOf("makoctl", "invoke"))

            withNextEvent { event ->
                assertInstanceOf<Event.NotificationClosed>(event)
                assertEquals(notification1Id, event.notificationId)
                assertEquals("default", event.action)
            }
        }

        testSuccessful = true
    }

    @Test
    fun testShowNotificationWithoutNotificationServiceNotBlockingOnExit() {
        val showNotificationParams = ShowNotificationParams(
            title = "Test Notification 1",
            body = "Body of Test Notification 1",
            soundFilePath = null,
        )
        run(defaultApplicationConfig())
        val timeTaken = measureTime {
            val notification1RequestId = ui { app.requestShowNotification(showNotificationParams) }
            assertNotNull(notification1RequestId)
            tearDown()
        }
        if (timeTaken > 1.seconds) {
            fail("Took $timeTaken")
        }
    }

    @Test
    fun testActivateNotification() {
        val showNotificationParams = ShowNotificationParams(
            title = "Test Notification 1",
            body = "Body of Test Notification 1",
            soundFilePath = null,
        )
        withMako {
            run(defaultApplicationConfig())
            val notification1RequestId = ui {
                app.requestShowNotification(showNotificationParams)
            }
            assertNotNull(notification1RequestId)
            val notification1Id = withNextEvent { event ->
                assertInstanceOf<Event.NotificationShown>(event)
                assertEquals(notification1RequestId, event.requestId)
                assertNotNull(event.notificationId)
                event.notificationId
            }
            assertNotNull(notification1Id)

            val notificationInfoList = checkNotNull(getMakoList()).data
            assertEquals(1, notificationInfoList.size)
            assertEquals(1, notificationInfoList.first().size)
            val notificationInfo = notificationInfoList.single().single()
            assertEquals(showNotificationParams.title, notificationInfo.summary.data)
            assertEquals(showNotificationParams.body, notificationInfo.body.data)
            assertEquals(notification1Id.toInt(), notificationInfo.id.data)

            runCommand(listOf("makoctl", "invoke"))

            withNextEvent { event ->
                assertInstanceOf<Event.NotificationClosed>(event)
                assertEquals(notification1Id, event.notificationId)
                assertEquals("default", event.action)
//                assertNotNull(event.activationToken)
            }
        }

        testSuccessful = true
    }

    @Test
    fun testCloseNotification() {
        val showNotificationParams = ShowNotificationParams(
            title = "Test Notification 1",
            body = "Body of Test Notification 1",
            soundFilePath = null,
        )
        withMako {
            run(defaultApplicationConfig())
            val notification1RequestId = ui {
                app.requestShowNotification(showNotificationParams)
            }
            assertNotNull(notification1RequestId)
            val notification1Id = withNextEvent { event ->
                assertInstanceOf<Event.NotificationShown>(event)
                assertEquals(notification1RequestId, event.requestId)
                checkNotNull(event.notificationId)
            }

            runCommand(listOf("makoctl", "dismiss"))

            withNextEvent { event ->
                assertInstanceOf<Event.NotificationClosed>(event)
                assertEquals(notification1Id, event.notificationId)
                assertNull(event.action)
                assertNull(event.activationToken)
            }
        }

        testSuccessful = true
    }

    @Test
    fun testCloseNotificationProgrammatically() {
        val showNotificationParams = ShowNotificationParams(
            title = "Test Notification 1",
            body = "Body of Test Notification 1",
            soundFilePath = null,
        )
        withMako {
            run(defaultApplicationConfig())
            val notification1RequestId = ui {
                app.requestShowNotification(showNotificationParams)
            }
            assertNotNull(notification1RequestId)
            val notification1Id = withNextEvent { event ->
                assertInstanceOf<Event.NotificationShown>(event)
                assertEquals(notification1RequestId, event.requestId)
                event.notificationId
            }
            assertNotNull(notification1Id)

            ui { app.closeNotification(notification1Id) }

            withNextEvent { event ->
                assertInstanceOf<Event.NotificationClosed>(event)
                assertEquals(notification1Id, event.notificationId)
                assertNull(event.action)
                assertNull(event.activationToken)
            }
        }

        testSuccessful = true
    }

    @Test
    fun testRendering() {
        val backgroundColor = SkColor.BLUE
        val rectColor = SkColor.RED
        val rectSize = LogicalSize(LogicalPixelsInt(100), LogicalPixelsInt(50))
        var scale = Scale.NO_SCALE

        val draw: (Event.WindowDraw) -> Unit = { event ->
            val softwareDrawData = event.softwareDrawData
            assertNotNull(softwareDrawData)
            performSoftwareDrawing(event.size, softwareDrawData) { canvas ->
                canvas.clear(backgroundColor)
                SkPaint().use { paint ->
                    paint.color = rectColor
                    log("Draw window: scale=$scale")
                    canvas.drawRect(
                        SkRect.makeWH(
                            rectSize.width.toRawPhysical(scale).toFloat(),
                            rectSize.height.toRawPhysical(scale).toFloat(),
                        ),
                        paint,
                    )
                }
            }
        }

        eventHandler = { event ->
            assertTrue(app.isEventLoopThread())
            when (event) {
                is Event.WindowScaleChanged -> {
                    scale = event.newScale
                    EventHandlerResult.Stop
                }

                is Event.WindowDraw -> {
                    draw(event)
                    EventHandlerResult.Stop
                }

                else -> {
                    EventHandlerResult.Continue
                }
            }
        }
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams()
        val window = createWindowAndWaitForFocus(windowParams).window

        assertNotNull(scale)
        val lastScreenSize = checkNotNull(lastScreenSize)

        val tempDir = Files.createTempDirectory("test_linux_rendering")
        val screenshotPath = tempDir.resolve("1.png")

        ui { window.setFullScreen() }
        awaitEventOfType<Event.WindowDraw> {
            it.size.width.rawPhysical == lastScreenSize.width.toRawPhysical(scale).roundToInt() &&
                it.size.height.rawPhysical == lastScreenSize.height.toRawPhysical(scale).roundToInt()
        }

        awaitEventOfType<Event.MouseEntered> { true }

        window.setPointerShape(PointerShape.Hidden)
        ui { lastDrawEvents.clear() }

        awaitEventOfType<Event.WindowDraw> { true }

        // Screenshot only the window, because sometimes the previous tests can leave the drag&drop artifacts
        screenshot(screenshotPath)

        val drawEvent = lastDrawEvents[windowParams.windowId]
        assertNotNull(drawEvent)

        val image = SkImage.makeFromEncoded(screenshotPath.readBytes())
        val imageScale = lastScreenSize.width.rawLogical.toDouble() / image.width
        val bitmap = SkBitmap.makeFromImage(image)

        val path = screenshotPath.absolutePathString()

        val bottomRightRectColor = bitmap.getColor(
            ((rectSize.width.rawLogical - 1).toDouble() * imageScale).roundToInt(),
            ((rectSize.height.rawLogical - 1).toDouble() * imageScale).roundToInt(),
        )
        assertEquals(
            rectColor,
            bottomRightRectColor,
            "Expected ${rectColor.toHexString()}, got ${bottomRightRectColor.toHexString()} for $path",
        )

        val bottomRightColor = bitmap.getColor(image.width - 1, image.height - 1)
        assertEquals(
            backgroundColor,
            bottomRightColor,
            "Expected ${backgroundColor.toHexString()}, got ${bottomRightColor.toHexString()} for $path",
        )

        val topLeftColor = bitmap.getColor(0, 0)
        assertEquals(rectColor, topLeftColor, "Expected ${rectColor.toHexString()}, got ${topLeftColor.toHexString()} for $path")

        screenshotPath.deleteIfExists()
        tempDir.deleteIfExists()
        testSuccessful = true
    }

    @Test
    fun testOpenUrl() {
        run(defaultApplicationConfig())
        val window = createWindowAndWaitForFocus(defaultWindowParams()).window
        val requestId = checkNotNull(ui { window.requestActivationToken() })
        val activationTokenEvent = awaitEvent({ it as? Event.ActivationTokenResponse }) { it.requestId == requestId }
        val url = "https://localhost"
        ui { app.openURL(url, activationTokenEvent.token) }
        AutoCloseable {
            TEST_APP_BROWSER_LOG_PATH.deleteIfExists()
        }.use {
            waitUntilEq(true) { TEST_APP_BROWSER_LOG_PATH.exists() }
            waitUntilEq("startup_id=XDG_ACTIVATION_TOKEN, url=$url\n") { TEST_APP_BROWSER_LOG_PATH.readText() }
        }
        testSuccessful = true
    }

    @Test
    fun testOpenUrlWithoutActivationToken() {
        run(defaultApplicationConfig())
        val url = "https://localhost"

        Thread.sleep(1000)
        ui { app.openURL(url, null) }
        AutoCloseable {
            TEST_APP_BROWSER_LOG_PATH.deleteIfExists()
        }.use {
            waitUntilEq(true) { TEST_APP_BROWSER_LOG_PATH.exists() }
            waitUntilEq("startup_id=, url=$url\n") { TEST_APP_BROWSER_LOG_PATH.readText() }
        }
        testSuccessful = true
    }

    @Test
    @Disabled("Old Ubuntu doesn't pass activation token properly")
    fun testOpenFileManager() {
        run(defaultApplicationConfig())
        val homeDir = checkNotNull(System.getenv("HOME"))
        val logPath = Path.of(homeDir, "test_app_file_manager.log")

        val dir = Path.of(
            homeDir,
            "Some Path",
            """~ ! @ # $ % ^ & * ( ) - _ = + [ { ] } \ | ; : ' " , < . > ?""",
            "НЕШТО",
            "何か",
        ).createDirectories()
        val path = dir.resolve("\uD83D\uDC69\uD83C\uDFFE\u200D\uD83E\uDD1D\u200D\uD83D\uDC68\uD83C\uDFFD")
        path.writeText("")
        val pathString = path.absolutePathString()
        val expectedLogLineSuffix =
            """, method_name=ShowItems, uris=file://$homeDir/Some%20Path/~%20!%20@%20%23%20$%20%25%20%5E%20&%20*%20(%20)%20-%20_%20=%20+%20%5B%20%7B%20%5D%20%7D%20%5C%20%7C%20%3B%20:%20'%20%22%20,%20%3C%20.%20%3E%20%3F/%D0%9D%D0%95%D0%A8%D0%A2%D0%9E/%E4%BD%95%E3%81%8B/%F0%9F%91%A9%F0%9F%8F%BE%E2%80%8D%F0%9F%A4%9D%E2%80%8D%F0%9F%91%A8%F0%9F%8F%BD, file_paths=$pathString"""
        val expectedLogLineWithStartupId = "has_startup_id=1$expectedLogLineSuffix"
        val expectedLogLineWithoutStartupId = "has_startup_id=1$expectedLogLineSuffix"

        val windowParams = defaultWindowParams()
        val window = createWindowAndWaitForFocus(windowParams).window

        val request1Id = ui { window.requestActivationToken() }
        val activationToken1Event = awaitEvent({ it as? Event.ActivationTokenResponse }) { it.requestId == request1Id }
        ui { app.openFileManager(pathString, activationToken1Event.token) }
        waitUntilEq(true) { logPath.exists() }
        waitUntilEq(expectedLogLineWithStartupId) {
            logPath.readText()
        }
        waitForTestAppFocus(windowParams.windowId, msg = "First not active")
        wm.focusWindowWithTitle(windowParams.title)
        waitForWindowFocus(windowParams.windowId, msg = "First active")

        ui { app.openFileManager(pathString, null) }
        waitUntilEq(true) { logPath.exists() }
        waitUntilEq(expectedLogLineWithoutStartupId) {
            logPath.readText()
        }

        val request2Id = ui { window.requestActivationToken() }
        val activationToken2Event = awaitEvent({ it as? Event.ActivationTokenResponse }) { it.requestId == request2Id }
        ui { app.openFileManager(pathString, activationToken2Event.token) }
        waitUntilEq(true) { logPath.exists() }
        waitUntilEq(expectedLogLineWithStartupId) {
            logPath.readText()
        }
        waitForTestAppFocus(windowParams.windowId, msg = "Second not active")
        wm.closeFocusedWindow()
        waitForWindowFocus(windowParams.windowId, msg = "Second active")
        testSuccessful = true
    }

    @Test
    @Disabled(
        "Delegated FileChooser call failed: Failed to activate service 'org.gnome.Nautilus': timed out (service_start_timeout=1000ms)",
    )
    fun testFileOpenDialog() {
        run(defaultApplicationConfig())
        val window = createWindowAndWaitForFocus(defaultWindowParams()).window
        val requestId = ui {
            window.showOpenFileDialog(
                commonParams = FileDialog.CommonDialogParams(
                    modal = true,
                    title = "Open Files",
                    acceptLabel = null,
                    currentFolder = null,
                ),
                openParams = FileDialog.OpenDialogParams(
                    allowsMultipleSelections = true,
                    selectDirectories = false,
                ),
            )
        }
        assertNotNull(requestId, "requestId")
        waitForTestAppFocus(window.windowId)

        withKeyPress(KeyCode.Escape) {}

        checkNextEvents(
            checks = mapOf(
                "FileChooserResponse" to { event, _ ->
                    event is Event.FileChooserResponse && requestId == event.requestId
                },
                "WindowKeyboardEnter" to { event, _ ->
                    event is Event.WindowKeyboardEnter
                },
            ),
        )

        ui { window.close() }
        awaitEventOfType<Event.WindowClosed> { true }
        testSuccessful = true
    }

    @Test
    @Disabled(
        "Delegated FileChooser call failed: Failed to activate service 'org.gnome.Nautilus': timed out (service_start_timeout=1000ms)",
    )
    fun testFileSaveDialog() {
        run(defaultApplicationConfig())
        val window = createWindowAndWaitForFocus(defaultWindowParams()).window
        val requestId = ui {
            window.showSaveFileDialog(
                FileDialog.CommonDialogParams(
                    modal = true,
                    title = null,
                    acceptLabel = null,
                    currentFolder = null,
                ),
                saveParams = FileDialog.SaveDialogParams(
                    nameFieldStringValue = null,
                ),
            )
        }
        assertNotNull(requestId, "requestId")
        waitForTestAppFocus(window.windowId)

        withKeyPress(KeyCode.Escape) {}

        checkNextEvents(
            checks = mapOf(
                "FileChooserResponse" to { event, _ ->
                    event is Event.FileChooserResponse && requestId == event.requestId
                },
                "WindowKeyboardEnter" to { event, _ ->
                    event is Event.WindowKeyboardEnter
                },
            ),
        )

        ui { window.close() }
        awaitEventOfType<Event.WindowClosed> { true }
        testSuccessful = true
    }
}
