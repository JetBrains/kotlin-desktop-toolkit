package org.jetbrains.desktop.linux.tests

import com.sun.jna.NativeLong
import com.sun.jna.platform.unix.X11
import com.sun.jna.ptr.IntByReference
import org.jetbrains.desktop.gtk.AccentColorValue
import org.jetbrains.desktop.gtk.Application
import org.jetbrains.desktop.gtk.ApplicationConfig
import org.jetbrains.desktop.gtk.ColorSchemeValue
import org.jetbrains.desktop.gtk.DataSource
import org.jetbrains.desktop.gtk.DesktopSetting
import org.jetbrains.desktop.gtk.DragAndDropAction
import org.jetbrains.desktop.gtk.DragAndDropQueryData
import org.jetbrains.desktop.gtk.DragAndDropQueryResponse
import org.jetbrains.desktop.gtk.DragIconParams
import org.jetbrains.desktop.gtk.Event
import org.jetbrains.desktop.gtk.EventHandlerResult
import org.jetbrains.desktop.gtk.FontHintingValue
import org.jetbrains.desktop.gtk.FontRgbaOrderValue
import org.jetbrains.desktop.gtk.KeyCode
import org.jetbrains.desktop.gtk.KeyModifiers
import org.jetbrains.desktop.gtk.KeySym
import org.jetbrains.desktop.gtk.KotlinDesktopToolkit
import org.jetbrains.desktop.gtk.LogLevel
import org.jetbrains.desktop.gtk.LogicalPoint
import org.jetbrains.desktop.gtk.LogicalRect
import org.jetbrains.desktop.gtk.LogicalSize
import org.jetbrains.desktop.gtk.MouseButton
import org.jetbrains.desktop.gtk.OpenGlDrawData
import org.jetbrains.desktop.gtk.PhysicalPoint
import org.jetbrains.desktop.gtk.PhysicalSize
import org.jetbrains.desktop.gtk.PointerShape
import org.jetbrains.desktop.gtk.RenderingMode
import org.jetbrains.desktop.gtk.ShowNotificationParams
import org.jetbrains.desktop.gtk.StartDragAndDropParams
import org.jetbrains.desktop.gtk.SupportedActionsForMime
import org.jetbrains.desktop.gtk.TextInputContentPurpose
import org.jetbrains.desktop.gtk.TextInputContext
import org.jetbrains.desktop.gtk.TextInputContextHint
import org.jetbrains.desktop.gtk.TextInputPreeditAttribute
import org.jetbrains.desktop.gtk.TextInputPreeditUnderlineType
import org.jetbrains.desktop.gtk.TextInputSurroundingText
import org.jetbrains.desktop.gtk.Window
import org.jetbrains.desktop.gtk.WindowDecorationMode
import org.jetbrains.desktop.gtk.WindowId
import org.jetbrains.desktop.gtk.WindowParams
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.makeGLWithInterface
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.Timeout
import org.junit.jupiter.api.assertInstanceOf
import org.junit.jupiter.api.assertThrows
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import java.io.File
import java.io.FileNotFoundException
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
import kotlin.io.path.deleteIfExists
import kotlin.io.path.readBytes
import kotlin.io.path.writeBytes
import kotlin.io.path.writeLines
import kotlin.math.roundToInt
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Ignore
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.test.fail
import kotlin.text.decodeToString
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Duration.Companion.seconds
import kotlin.time.TimeSource
import kotlin.use
import org.jetbrains.skia.BackendRenderTarget as SkBackendRenderTarget
import org.jetbrains.skia.Bitmap as SkBitmap
import org.jetbrains.skia.Color as SkColor
import org.jetbrains.skia.ColorSpace as SkColorSpace
import org.jetbrains.skia.DirectContext as SkDirectContext
import org.jetbrains.skia.FramebufferFormat as SkFramebufferFormat
import org.jetbrains.skia.GLAssembledInterface as SkGLAssembledInterface
import org.jetbrains.skia.Image as SkImage
import org.jetbrains.skia.Paint as SkPaint
import org.jetbrains.skia.Rect as SkRect
import org.jetbrains.skia.Surface as SkSurface
import org.jetbrains.skia.SurfaceColorFormat as SkSurfaceColorFormat
import org.jetbrains.skia.SurfaceOrigin as SkSurfaceOrigin

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

private fun runCommandWithOutput(command: List<String>, timeout: Duration = 5.seconds): ByteArray {
    val pb = ProcessBuilder(command)
        .redirectOutput(ProcessBuilder.Redirect.PIPE)
        .redirectError(ProcessBuilder.Redirect.INHERIT)

    val proc = pb.start()
    assertTrue(proc.waitFor(timeout.inWholeMilliseconds, TimeUnit.MILLISECONDS), command.toString())
    return proc.inputStream.readAllBytes()
}

private fun runCommand(command: List<String>, timeout: Duration = 5.seconds) {
    val pb = ProcessBuilder(command)
        .redirectOutput(ProcessBuilder.Redirect.INHERIT)
        .redirectError(ProcessBuilder.Redirect.PIPE)

    val proc = pb.start()
    if (!proc.waitFor(timeout.inWholeMilliseconds, TimeUnit.MILLISECONDS)) {
        val stderrReader = proc.errorReader()
        val stderr = if (stderrReader.ready()) proc.errorReader().readText() else ""
        fail(withTimestamp("$command failed, stderr=$stderr"))
    }
}

private fun <T> waitUntilEq(expectedValue: T, timeout: Duration = 5.seconds, actualValueGetter: () -> T) {
    val startTime = TimeSource.Monotonic.markNow()
    val waitStepMs = 10L
    var actualValue = actualValueGetter()
    while (startTime.elapsedNow() < timeout) {
        if (actualValue == expectedValue) {
            return
        }
        Thread.sleep(waitStepMs)
        actualValue = actualValueGetter()
    }
    fail(withTimestamp("waitUntilEq timed out: Expected: $expectedValue, actual: $actualValue "))
}

private enum class TestApp(private val resourcePath: String) {
    BlankWindow("/linux/test_app_blank_window.py"),
    ClipboardSource("/linux/test_app_clipboard_source.py"),
    DragSource("/linux/test_app_drag_source.py"),
    DropTarget("/linux/test_app_drop_target.py"),
    PrimarySelectionSource("/linux/test_app_primary_selection_source.py"),
    ;

    companion object {
        private fun readResourceTextFile(path: String): String {
            return this::class.java.getResource(path)!!.readText()
        }
    }

    private fun createProcessBuilder(vararg args: String): ProcessBuilder {
        log("Running test app: $this")
        val appSource = readResourceTextFile(resourcePath)
        return ProcessBuilder("python3", "-c", appSource, *args).redirectError(ProcessBuilder.Redirect.INHERIT).also {
            it.environment().remove("GTK_DEBUG")
            it.environment().remove("GDK_DEBUG")
            it.environment().remove("LD_LIBRARY_PATH")
        }
    }

    fun run(args: Array<String> = emptyArray(), block: ((Duration) -> String?) -> Unit) {
        val outputFile = File.createTempFile("linux_test_app_output", "log")
        val process = createProcessBuilder(*args)
            .redirectOutput(ProcessBuilder.Redirect.to(outputFile))
            .start()
        AutoCloseable {
            assertTrue(process.isAlive, outputFile.absolutePath)
            process.destroy()
            process.waitFor()
            log("Test app $this output:\n${outputFile.readText()}")
            outputFile.delete()
            log("Test app $this closed")
        }.use {
            waitUntilEq(true) { !process.isAlive || outputFile.length() > 0 }
            assertTrue(process.isAlive)

            val lines = outputFile.readLines()
            assertContentEquals(arrayOf("ready"), lines.toTypedArray())
            log("Test app $this ready")

            val lastLinesCount = mutableListOf(1)
            val readLastLine = { timeout: Duration ->
                var ret: String? = null
                val retryIntervalMs = 10L
                var retryTimes = timeout.inWholeMilliseconds / retryIntervalMs
                while (retryTimes > 0) {
                    val lines = outputFile.readLines()
                    if (lines.size != lastLinesCount.last()) {
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
            block(readLastLine)
        }
    }
}

private fun isNotificationServiceRunning(): Boolean {
    return ProcessBuilder("busctl", "--user", "status", "org.freedesktop.Notifications").start().waitFor() == 0
}

// `gsettings` works with dconf and XDG Desktop Portal, and GTK uses them only on Wayland.
// On X11, we need to have some xsettings daemon running.
// We're using `xsettingsd` as a desktop-agnostic xsettings daemon.
// E.g., Gnome used `gsd-xsettings` (gnome-settings-daemon) for this.
private class XSettingsD {
    companion object {
        private val configFile = Path.of(System.getenv("TEST_XSETTINGSD_CONFIG_FILE")!!)
        private val xSettingsDPid = System.getenv("TEST_XSETTINGSD_PID")!!

        private fun boolToString(value: Boolean): String = if (value) "1" else "0"

        private fun rgbaToString(value: FontRgbaOrderValue): String? = when (value) {
            FontRgbaOrderValue.None -> null
            FontRgbaOrderValue.Rgb -> quoted("rgb")
            FontRgbaOrderValue.Bgr -> quoted("bgr")
            FontRgbaOrderValue.Vrgb -> quoted("vrgb")
            FontRgbaOrderValue.Vbgr -> quoted("vbgr")
        }

        private fun quoted(value: String): String = "\"$value\""

        private fun reloadSettings(): Int {
            log("Reloading xsettingsd")
            return ProcessBuilder("kill", "-HUP", xSettingsDPid).start().waitFor()
        }

        fun withChangedSetting(setting: DesktopSetting, block: () -> Unit) {
            val toChange = buildMap {
                when (setting) {
                    is DesktopSetting.AudibleBell -> put("Net/EnableEventSounds", boolToString(setting.value))
                    is DesktopSetting.CursorBlink -> put("Net/CursorBlink", boolToString(setting.value))
                    is DesktopSetting.CursorBlinkTime -> put("Net/CursorBlinkTime", setting.value.inWholeMilliseconds.toString())
                    is DesktopSetting.CursorBlinkTimeout -> put("Gtk/CursorBlinkTimeout", setting.value.inWholeSeconds.toString())
                    is DesktopSetting.DoubleClickInterval -> put("Net/DoubleClickTime", setting.value.inWholeMilliseconds.toString())

                    is DesktopSetting.FontHinting -> when (setting.value) {
                        FontHintingValue.None -> {
                            put("Xft/Hinting", "0")
                            put("Xft/HintStyle", quoted("hintnone"))
                        }
                        FontHintingValue.Slight -> {
                            put("Xft/Hinting", "1")
                            put("Xft/HintStyle", quoted("hintslight"))
                        }
                        FontHintingValue.Medium -> {
                            put("Xft/Hinting", "1")
                            put("Xft/HintStyle", quoted("hintmedium"))
                        }
                        FontHintingValue.Full -> {
                            put("Xft/Hinting", "1")
                            put("Xft/HintStyle", quoted("hintfull"))
                        }
                    }

                    is DesktopSetting.FontRgbaOrder -> put("Xft/RGBA", rgbaToString(setting.value))
                    is DesktopSetting.MiddleClickPaste -> put("Gtk/EnablePrimaryPaste", boolToString(setting.value))
                    is DesktopSetting.OverlayScrolling -> put("Gtk/OverlayScrolling", boolToString(setting.value))
                    is DesktopSetting.AccentColor -> fail("Cannot change accent color via xsettingsd")
                    is DesktopSetting.ColorScheme -> fail("Cannot change color scheme via xsettingsd")
                    is DesktopSetting.DragAndDropDragThresholdPixels -> put("Net/DndDragThreshold", setting.value.toString())
                    is DesktopSetting.IsComposited -> fail("Cannot change composited state via xsettingsd")
                    is DesktopSetting.DoubleClickDistancePixels -> put("Net/DoubleClickDistance", setting.value.toString())
                    is DesktopSetting.EnableAnimations -> put("Gtk/EnableAnimations", boolToString(setting.value))
                    is DesktopSetting.PrimaryButtonWarpsSlider -> put("Gtk/PrimaryButtonWarpsSlider", boolToString(setting.value))
                    is DesktopSetting.RecentFilesEnabled -> put("Gtk/RecentFilesEnabled", boolToString(setting.value))
                    is DesktopSetting.RecentFilesMaxAgeDays -> put("Gtk/RecentFilesMaxAge", setting.value.toString())
                }
            }
            withChangedSettings(toChange, block)
        }

        fun withChangedSettings(toChange: Map<String, String?>, block: () -> Unit) {
            log("xsettingsd: changing $toChange")
            val oldConf = configFile.readBytes()

            val newConf = oldConf.decodeToString().lineSequence().filter { oldLine ->
                oldLine.isNotEmpty() && toChange.keys.none { oldLine.startsWith("$it ") }
            } + toChange.mapNotNull { (name, tempValue) ->
                tempValue?.let { "$name $it" }
            }

            configFile.writeLines(newConf)
            assertEquals(0, reloadSettings())

            AutoCloseable {
                configFile.writeBytes(oldConf)
                assertEquals(0, reloadSettings())
            }.use {
                block()
            }
        }
    }
}

private class Dconf private constructor() {
    companion object {
        private const val SCHEMA = "org.gnome.desktop.interface"

        private data class Setting(val name: String) {
            fun read(): String? {
                val p = ProcessBuilder("gsettings", "get", SCHEMA, name)
                    .redirectError(ProcessBuilder.Redirect.INHERIT)
                    .redirectOutput(ProcessBuilder.Redirect.PIPE)
                    .start()
                return if (p.waitFor() == 0) p.inputReader().readText().trim() else null
            }

            fun changeTo(value: String): Int {
                return ProcessBuilder("gsettings", "set", SCHEMA, name, value)
                    .redirectError(ProcessBuilder.Redirect.INHERIT)
                    .redirectOutput(ProcessBuilder.Redirect.INHERIT)
                    .start()
                    .waitFor()
            }

            override fun toString(): String {
                return "$SCHEMA $name"
            }
        }

        private fun withChangedSetting(setting: Setting, tempValue: String, block: () -> Unit) {
            val origValue = setting.read()
            assertNotNull(origValue, "Failed to read $setting")
            log("Original value of setting $setting = $origValue")

            log("Changing $setting to $tempValue")
            assertEquals(0, setting.changeTo(tempValue), "Failed to change setting $setting")

            val changedValue = setting.read()
            assertNotNull(changedValue, "Failed to read changed setting $setting")

            assertNotEquals(origValue, changedValue)
            log("Applied $setting $tempValue")

            block()

            log("Reverting $setting to $origValue")
            assertEquals(0, setting.changeTo(origValue), "Failed to revert setting $setting")
        }

        fun withChangedColorScheme(value: ColorSchemeValue, block: () -> Unit) {
            val setting = Setting("color-scheme")
            val tempValue = when (value) {
                ColorSchemeValue.NoPreference -> "default"
                ColorSchemeValue.PreferDark -> "prefer-dark"
                ColorSchemeValue.PreferLight -> "prefer-light"
            }
            withChangedSetting(setting, tempValue, block)
        }

        fun supportsAccentColor(): Boolean {
            return runCommandWithOutput(listOf("gsettings", "list-keys", SCHEMA)).decodeToString().contains("accent-color")
        }

        fun withChangedAccentColor(value: AccentColorValue, block: () -> Unit) {
            val setting = Setting("accent-color")
            val tempValue = when (value) {
                AccentColorValue.Unknown -> fail("Cannot set Accent Color to Unknown")
                AccentColorValue.Blue -> "blue"
                AccentColorValue.Teal -> "teal"
                AccentColorValue.Green -> "green"
                AccentColorValue.Yellow -> "yellow"
                AccentColorValue.Orange -> "orange"
                AccentColorValue.Red -> "red"
                AccentColorValue.Pink -> "pink"
                AccentColorValue.Purple -> "purple"
                AccentColorValue.Slate -> "slate"
            }
            withChangedSetting(setting, tempValue, block)
        }
    }
}

private fun withDunst(block: () -> Unit): List<String> {
    var output = emptyList<String>()
    val process = ProcessBuilder("dunst", "-print", "-conf", System.getenv("TEST_DUNST_CONFIG_FILE")!!)
        .redirectOutput(ProcessBuilder.Redirect.PIPE)
        .redirectError(ProcessBuilder.Redirect.INHERIT)
        .start()
    AutoCloseable {
        assertTrue(process.isAlive)
        process.toHandle().destroy()
        process.waitFor()
        output = process.inputReader().readLines()
        process.destroy()
    }.use {
        assertTrue(process.isAlive)
        waitUntilEq(true) { isNotificationServiceRunning() }
        block()
    }
    return output
}

private fun withTestImeEngine(block: () -> Unit) {
    val ibusEngineSetIntervalMs = 10L
    var ibusEngineSetTimeoutMs = 1000L
    val setEngine = { ProcessBuilder("ibus", "engine", "jb_kdt_ibus_test_engine").start().waitFor() }
    while (setEngine() != 0 && ibusEngineSetTimeoutMs > 0) {
        Thread.sleep(ibusEngineSetIntervalMs)
        ibusEngineSetTimeoutMs -= ibusEngineSetIntervalMs
    }
    check(ibusEngineSetTimeoutMs > 0) { "Setting IBus test engine failed" }

    AutoCloseable {
        ProcessBuilder("ibus", "engine", "xkb:us::eng").start().waitFor()
    }.use {
        block()
    }
}

private fun getClipboardContent(format: String): ByteArray {
    return runCommandWithOutput(listOf("xclip", "-selection", "clipboard", "-o", "-t", format))
}

private fun getPrimarySelectionContent(format: String): ByteArray {
    return runCommandWithOutput(listOf("xclip", "-selection", "primary", "-o", "-t", format))
}

private fun getActiveWindowNumber(): ULong {
    return runCommandWithOutput(listOf("xdotool", "getactivewindow")).decodeToString().trim().toULong()
}

private fun getActiveWindowTitle(): String {
    return runCommandWithOutput(listOf("xdotool", "getactivewindow", "getwindowname")).decodeToString().trimEnd('\n')
}

private fun screenshot(
    outPath: Path,
    rect: Pair<PhysicalPoint, PhysicalSize>? = null,
    hideCursor: Boolean = true,
    windowId: ULong? = null,
) {
    val cmd = buildList {
        add("maim")
        rect?.let { (point, size) ->
            add("-g")
            add("${size.width}x${size.height}+${point.x}+${point.y}")
        }
        if (hideCursor) {
            add("--hidecursor")
        }
        windowId?.let {
            add("--window=$it")
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

private class IBusTestEngineOutput(envVarName: String) {
    private val file = File(System.getenv(envVarName)!!)

    init {
        file.delete()
    }

    fun read(expectedLineCount: Int = 1): String? {
        var retryTimes = 100
        val retryIntervalMs = 10L
        while (retryTimes > 0) {
            try {
                val text = file.readLines()
                if (expectedLineCount == -1 || expectedLineCount == text.size) {
                    file.delete()
                    return text.last()
                }
            } catch (_: FileNotFoundException) {}
            retryTimes -= 1
            Thread.sleep(retryIntervalMs)
        }
        return null
    }

    fun exists(): Boolean {
        return file.exists()
    }
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

    fun isChecked(entry: String): Boolean {
        return state[entry]!!
    }

    fun isAllChecked(): Boolean {
        return state.all { it.value }
    }

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
}

private fun defaultWindowParams(): WindowParams {
    return WindowParams(
        windowId = 0,
        title = "Test Window 1",
        size = LogicalSize(width = 200, height = 300),
        minSize = null,
        decorationMode = WindowDecorationMode.Server,
        renderingMode = RenderingMode.GL_ES,
    )
}

abstract class X11TestsBase {
    companion object {
        private const val APP_ID = "org.jetbrains.desktop.linux.tests"
        internal const val TEXT_UTF8_MIME_TYPE = "text/plain;charset=utf-8"
        internal const val URI_LIST_MIME_TYPE = "text/uri-list"
        internal const val HTML_TEXT_MIME_TYPE = "text/html"
        internal const val PNG_MIME_TYPE = "image/png"

        internal val physicalScreenSize = PhysicalSize(3000, 1500)
        private val appExecutor = SingleThreadTaskQueue()

        @BeforeAll
        @JvmStatic
        fun initAll() {
            KotlinDesktopToolkit.init(
                libraryFolderPath = Path.of(System.getProperty("kdt.gtk.library.folder.path")!!),
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
    }

    private val display = X11.INSTANCE.XOpenDisplay(null)!!

    val app by lazy { Application(APP_ID) }
    private lateinit var appExecutingResult: Future<Error?>
    val eventQueue = LinkedBlockingQueue<Event>()

    internal fun defaultApplicationConfig(
        queryDragAndDropTarget: (DragAndDropQueryData) -> DragAndDropQueryResponse = { _ -> DragAndDropQueryResponse(emptyList()) },
        getDataTransferData: (DataSource, String) -> ByteArray? = { _, _ -> null },
        windowCloseRequest: (WindowId) -> Boolean = { true },
        applicationWantsToTerminate: () -> Boolean = { true },
        getSurroundingText: (WindowId) -> TextInputSurroundingText? = { null },
    ): ApplicationConfig {
        return ApplicationConfig(
            eventHandler = { event ->
                if (event !is Event.WindowFrameTick && event !is Event.DragIconFrameTick) {
                    log("Event: $event")
                    assert(eventQueue.offer(event)) { "Event queue overflow" }
                }
                eventHandler?.invoke(event) ?: EventHandlerResult.Continue
                EventHandlerResult.Stop
            },
            queryDragAndDropTarget = queryDragAndDropTarget,
            getDataTransferData = getDataTransferData,
            windowCloseRequest = windowCloseRequest,
            applicationWantsToTerminate = applicationWantsToTerminate,
            getSurroundingText = getSurroundingText,
        )
    }

    internal fun runWithoutWaitingForStart(applicationConfig: ApplicationConfig) {
        // Reset the mouse position
        moveMouseTo(50, 50)
        appExecutingResult = appExecutor.add {
            try {
                app.runEventLoop(applicationConfig)
            } catch (t: Throwable) {
                app.stopEventLoop()
                fail(withTimestamp("Application event loop finished exceptionally"), t)
            }
        }
    }

    internal fun run(applicationConfig: ApplicationConfig) {
        runWithoutWaitingForStart(applicationConfig)
        assertNotNull(awaitEvent { it == Event.ApplicationStarted })
        assertTrue(eventQueue.isEmpty())
    }

    internal fun getNextEvent(timeout: Duration = 5.seconds): Event? {
        while (true) {
            val event = eventQueue.poll(timeout.inWholeMilliseconds, TimeUnit.MILLISECONDS)
            if (event == null) {
                log("Timed out waiting for event")
                return null
            }
            return event
        }
    }

    internal fun <T> withNextEvent(timeout: Duration = 5.seconds, block: (Event?) -> T): T {
        return block(getNextEvent(timeout))
    }

    internal fun awaitEventWithHistory(timeout: Duration = 5.seconds, predicate: (Event?, List<Event>) -> Boolean): Event? {
        val otherEvents = mutableListOf<Event>()
        while (true) {
            val event: Event? = eventQueue.poll(timeout.inWholeMilliseconds, TimeUnit.MILLISECONDS)
            if (predicate(event, otherEvents) || event == null) {
                return event
            } else {
                otherEvents.add(event)
            }
        }
    }

    internal fun awaitEvent(timeout: Duration = 5.seconds, predicate: (Event?) -> Boolean): Event? {
        return awaitEventWithHistory(timeout) { event, _ ->
            predicate(event)
        }
    }

    internal inline fun <reified T : Event> awaitEventOfType(
        timeout: Duration = 5.seconds,
        msg: String? = null,
        crossinline predicate: (T) -> Boolean,
    ): T {
        return awaitEventWithHistory(timeout) { event, otherEvents ->
            if (event == null) {
                val additionalMsg = if (msg != null) ": $msg" else ""
                val otherEventsMsg = if (otherEvents.isEmpty()) "" else ". Other events:\n${otherEvents.joinToString("\n")}"
                fail(withTimestamp("Timed out waiting for event ${T::class.java.name}$additionalMsg$otherEventsMsg"))
            } else {
                event is T && predicate(event)
            }
        } as T
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

    internal fun <T> withXtest(doXtest: (X11.XTest, X11.Display) -> Unit, undoXtest: (X11.XTest, X11.Display) -> Unit, block: () -> T): T {
        val x11 = X11.INSTANCE!!
        doXtest(X11.XTest.INSTANCE, display)
        x11.XSync(display, false)
        AutoCloseable {
            undoXtest(X11.XTest.INSTANCE, display)
            x11.XSync(display, false)
        }.use {
            return block()
        }
    }

    internal fun withKeyPress(key: UInt, block: () -> Unit) {
        withXtest(
            { xtest, display -> xtest.XTestFakeKeyEvent(display, key.toInt(), true, NativeLong(0)) },
            { xtest, display -> xtest.XTestFakeKeyEvent(display, key.toInt(), false, NativeLong(0)) },
            block,
        )
    }

    internal fun <T> withMouseButtonDown(button: MouseButton, block: () -> T): T {
        return withXtest(
            { xtest, display -> xtest.XTestFakeButtonEvent(display, button.value, true, NativeLong(0)) },
            { xtest, display -> xtest.XTestFakeButtonEvent(display, button.value, false, NativeLong(0)) },
            block,
        )
    }

    internal fun getMousePosition(): PhysicalPoint? {
        val x11 = X11.INSTANCE!!
        val rootWindow = x11.XDefaultRootWindow(display)
        val rootReturn = X11.WindowByReference()
        val childReturn = X11.WindowByReference()
        val rootXReturn = IntByReference()
        val rootYReturn = IntByReference()
        val winXReturn = IntByReference()
        val winYReturn = IntByReference()
        val maskReturn = IntByReference()
        val ret = x11.XQueryPointer(
            display, rootWindow, rootReturn, childReturn, rootXReturn, rootYReturn, winXReturn, winYReturn, maskReturn,
        )
        return if (ret) {
            PhysicalPoint(rootXReturn.value, rootYReturn.value)
        } else {
            null
        }
    }

    internal fun moveMouseTo(x: Int, y: Int) {
        withXtest(
            { xtest, display ->
                xtest.XTestFakeMotionEvent(display, 0, x, y, NativeLong(0))
                val expectedPosition = PhysicalPoint(x, y)
                while (getMousePosition() != expectedPosition) {
                    Thread.sleep(1)
                }
//                log("moveMouseTo $x, $y done")
            },
            { _, _ -> },
            {},
        )
    }

    internal fun wiggleMouseUntil(x: Int, y: Int, timeout: Duration = 5.seconds, predicate: () -> Boolean): Boolean {
        val startTime = TimeSource.Monotonic.markNow()
        var moveToRight = false
        while (startTime.elapsedNow() < timeout) {
            if (predicate()) {
                return true
            }
            val offset = if (moveToRight) 5 else 0
            moveMouseTo(x + offset, y + offset)
            ui {}
            Thread.sleep(1)
            moveToRight = !moveToRight
        }
        return false
    }

    internal fun scrollMouseUp() {
        // https://askubuntu.com/a/1162351
        withXtest(
            { xtest, display ->
                xtest.XTestFakeButtonEvent(display, 4, true, NativeLong(0))
                xtest.XTestFakeButtonEvent(display, 4, false, NativeLong(0))
            },
            { _, _ -> },
            {},
        )
    }

    internal fun scrollMouseDown() {
        // https://askubuntu.com/a/1162351
        withXtest(
            { xtest, display ->
                xtest.XTestFakeButtonEvent(display, 5, true, NativeLong(0))
                xtest.XTestFakeButtonEvent(display, 5, false, NativeLong(0))
            },
            { _, _ -> },
            {},
        )
    }

    internal fun withSetClipboardContent(mimeTypes: List<String>, block: () -> Unit) {
        ui { app.clipboardPut(mimeTypes) }
        AutoCloseable {
            ui { app.clipboardPut(emptyList()) }
            // Ensure that `DataTransferAvailable` events are sent.
            // TODO: flaky, e.g.:
            // org.opentest4j.AssertionFailedError: Unexpected type, expected: <org.jetbrains.desktop.gtk.Event.NotificationShown> but was: <org.jetbrains.desktop.gtk.Event.DataTransferAvailable>
            ui {}
        }.use {
            block()
        }
    }

    internal fun withSetPrimarySelectionContent(mimeTypes: List<String>, block: () -> Unit) {
        ui { app.primarySelectionPut(mimeTypes) }
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
    )

    internal fun createWindowAndWaitForFocus(windowParams: WindowParams, onScale: ((Double) -> Unit)? = null): InitialWindowData {
        val window = ui { app.createWindow(windowParams) }
        lateinit var scale: Event.WindowScaleChanged
        lateinit var configure: Event.WindowConfigure
        lateinit var draw: Event.WindowDraw
        lateinit var keyboardEnter: Event.WindowKeyboardEnter
        lateinit var screen: Event.WindowScreenChange
        val checklist = Checklist(listOf("draw", "configure", "keyboardEnter", "scale", "screen"))
        waitUntilEq(emptySet()) {
            when (val event: Event? = eventQueue.poll()) {
                is Event.WindowConfigure -> {
                    if (windowParams.windowId == event.windowId && event.active) {
                        configure = event
                        checklist.checkEntry("configure")
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
                        checklist.checkEntry("draw")
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
                        onScale?.invoke(scale.newScale)
                    }
                }

                else -> {}
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
        )
    }

    internal fun waitForWindowFocusAfterMouseDown(windowId: WindowId) {
        val checklist = Checklist(listOf("configure", "keyboardEnter", "mouseDown"))
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
                }

                else -> {}
            }
            checklist.uncheckedEntries()
        }
    }

    internal fun waitForTestAppFocus(windowId: WindowId, previousWindowSize: LogicalSize) {
        val testAppFocusedChecklist = Checklist(listOf("configure", "draw", "keyboardLeave"))
        waitUntilEq(emptySet()) {
            when (val event: Event? = eventQueue.poll()) {
                is Event.WindowConfigure -> {
                    assertEquals(windowId, event.windowId)
                    if (!event.active && event.size != previousWindowSize) {
                        testAppFocusedChecklist.checkEntry("configure")
                    }
                }

                is Event.WindowDraw -> {
                    assertEquals(windowId, event.windowId)
                    if (event.size != previousWindowSize) {
                        testAppFocusedChecklist.checkEntry("draw")
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

    // Older GTK versions (e.g., 4.6.9) don't report `Event.WindowKeyboardLeave` on window close
    fun waitForWindowClosed(windowId: WindowId) {
        withNextEvent { event ->
            if (event is Event.WindowKeyboardLeave) {
                assertEquals(windowId, event.windowId)
                withNextEvent { event ->
                    assertInstanceOf<Event.WindowClosed>(event)
                    assertEquals(windowId, event.windowId)
                }
            } else {
                assertInstanceOf<Event.WindowClosed>(event)
                assertEquals(windowId, event.windowId)
            }
        }
    }

    @Volatile
    internal var eventHandler: ((Event) -> EventHandlerResult)? = null

    @BeforeTest
    fun setUp() {
        testStart.elapsedNow() // trigger lazy evaluation
    }

    @AfterTest
    @Timeout(value = 20, unit = TimeUnit.SECONDS)
    fun tearDown() {
        log("tearDown start")
        if (!appExecutingResult.isDone) {
            app.stopEventLoop()
        }
        assertNull(appExecutingResult.get())
        log("tearDown end")
    }
}

@EnabledOnOs(OS.LINUX)
@Execution(ExecutionMode.SAME_THREAD)
class X11Tests : X11TestsBase() {
    @Test
    fun testIsEventLoopThread() {
        run(defaultApplicationConfig())
        assertFalse(app.isEventLoopThread())
        ui { assertTrue(app.isEventLoopThread()) }
    }

    @Test
    fun testSettings() {
        runWithoutWaitingForStart(defaultApplicationConfig())

        lateinit var initialAccentColor: DesktopSetting.AccentColor
        lateinit var initialAudibleBell: DesktopSetting.AudibleBell
        lateinit var initialColorScheme: DesktopSetting.ColorScheme
        lateinit var initialCursorBlink: DesktopSetting.CursorBlink
        lateinit var initialCursorBlinkTime: DesktopSetting.CursorBlinkTime
        lateinit var initialCursorBlinkTimeout: DesktopSetting.CursorBlinkTimeout
        lateinit var initialDoubleClickInterval: DesktopSetting.DoubleClickInterval
        lateinit var initialDragAndDropDragThresholdPixels: DesktopSetting.DragAndDropDragThresholdPixels
        lateinit var initialFontHinting: DesktopSetting.FontHinting
        lateinit var initialFontRgbaOrder: DesktopSetting.FontRgbaOrder
        lateinit var initialIsComposited: DesktopSetting.IsComposited
        lateinit var initialMiddleClickPaste: DesktopSetting.MiddleClickPaste
        lateinit var initialOverlayScrolling: DesktopSetting.OverlayScrolling
        lateinit var initialDoubleClickDistancePixels: DesktopSetting.DoubleClickDistancePixels
        lateinit var initialEnableAnimations: DesktopSetting.EnableAnimations
        lateinit var initialPrimaryButtonWarpsSlider: DesktopSetting.PrimaryButtonWarpsSlider
        lateinit var initialRecentFilesEnabled: DesktopSetting.RecentFilesEnabled
        lateinit var initialRecentFilesMaxAgeDays: DesktopSetting.RecentFilesMaxAgeDays

        var remainingSettingsReceived = 18
        while (remainingSettingsReceived > 0) {
            val event = getNextEvent()
            assertIs<Event.DesktopSettingChange>(event, "Remaining: $remainingSettingsReceived")
            when (event.setting) {
                is DesktopSetting.AccentColor -> initialAccentColor = event.setting
                is DesktopSetting.AudibleBell -> initialAudibleBell = event.setting
                is DesktopSetting.ColorScheme -> initialColorScheme = event.setting
                is DesktopSetting.CursorBlink -> initialCursorBlink = event.setting
                is DesktopSetting.CursorBlinkTime -> initialCursorBlinkTime = event.setting
                is DesktopSetting.CursorBlinkTimeout -> initialCursorBlinkTimeout = event.setting
                is DesktopSetting.DoubleClickInterval -> initialDoubleClickInterval = event.setting
                is DesktopSetting.DragAndDropDragThresholdPixels -> initialDragAndDropDragThresholdPixels = event.setting
                is DesktopSetting.FontHinting -> initialFontHinting = event.setting
                is DesktopSetting.FontRgbaOrder -> initialFontRgbaOrder = event.setting
                is DesktopSetting.IsComposited -> initialIsComposited = event.setting
                is DesktopSetting.MiddleClickPaste -> initialMiddleClickPaste = event.setting
                is DesktopSetting.OverlayScrolling -> initialOverlayScrolling = event.setting
                is DesktopSetting.DoubleClickDistancePixels -> initialDoubleClickDistancePixels = event.setting
                is DesktopSetting.EnableAnimations -> initialEnableAnimations = event.setting
                is DesktopSetting.PrimaryButtonWarpsSlider -> initialPrimaryButtonWarpsSlider = event.setting
                is DesktopSetting.RecentFilesEnabled -> initialRecentFilesEnabled = event.setting
                is DesktopSetting.RecentFilesMaxAgeDays -> initialRecentFilesMaxAgeDays = event.setting
            }
            remainingSettingsReceived -= 1
        }
        assertIs<Event.ApplicationStarted>(getNextEvent())

        val windowParams = defaultWindowParams()
        val initialWindowData = createWindowAndWaitForFocus(windowParams)

        val changedColorScheme = when (initialColorScheme.value) {
            ColorSchemeValue.NoPreference -> ColorSchemeValue.PreferDark
            ColorSchemeValue.PreferDark -> ColorSchemeValue.PreferLight
            ColorSchemeValue.PreferLight -> ColorSchemeValue.NoPreference
        }
        Dconf.withChangedColorScheme(changedColorScheme) {
            awaitEventOfType<Event.DesktopSettingChange> {
                assertEquals(DesktopSetting.ColorScheme(changedColorScheme), it.setting)
                true
            }
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialColorScheme, it.setting)
            true
        }

        if (Dconf.supportsAccentColor()) {
            val changedAccentColor = if (initialAccentColor.value ==
                AccentColorValue.Purple
            ) {
                AccentColorValue.Blue
            } else {
                AccentColorValue.Purple
            }
            Dconf.withChangedAccentColor(changedAccentColor) {
                awaitEventOfType<Event.DesktopSettingChange> {
                    assertEquals(DesktopSetting.AccentColor(changedAccentColor), it.setting)
                    true
                }
            }
            awaitEventOfType<Event.DesktopSettingChange> {
                assertEquals(initialAccentColor, it.setting)
                true
            }
        }

        val initialAudibleBellChanged = DesktopSetting.AudibleBell(!initialAudibleBell.value)
        XSettingsD.withChangedSetting(initialAudibleBellChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(initialAudibleBellChanged, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialAudibleBell, it.setting)
            true
        }

        val initialCursorBlinkChanged = DesktopSetting.CursorBlink(!initialCursorBlink.value)
        XSettingsD.withChangedSetting(initialCursorBlinkChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(initialCursorBlinkChanged, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialCursorBlink, it.setting)
            true
        }

        val initialCursorBlinkTimeChanged = DesktopSetting.CursorBlinkTime(initialCursorBlinkTime.value + 100.milliseconds)
        XSettingsD.withChangedSetting(initialCursorBlinkTimeChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(initialCursorBlinkTimeChanged, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialCursorBlinkTime, it.setting)
            true
        }

        val initialCursorBlinkTimeoutChanged = DesktopSetting.CursorBlinkTimeout(initialCursorBlinkTimeout.value + 2.seconds)
        XSettingsD.withChangedSetting(initialCursorBlinkTimeoutChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(initialCursorBlinkTimeoutChanged, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialCursorBlinkTimeout, it.setting)
            true
        }

        val initialDoubleClickIntervalChanged = DesktopSetting.DoubleClickInterval(initialDoubleClickInterval.value + 100.milliseconds)
        XSettingsD.withChangedSetting(initialDoubleClickIntervalChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(initialDoubleClickIntervalChanged, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialDoubleClickInterval, it.setting)
            true
        }

        val initialDragAndDropDragThresholdPixelsChanged = DesktopSetting.DragAndDropDragThresholdPixels(
            initialDragAndDropDragThresholdPixels.value + 5,
        )
        XSettingsD.withChangedSetting(initialDragAndDropDragThresholdPixelsChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(initialDragAndDropDragThresholdPixelsChanged, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialDragAndDropDragThresholdPixels, it.setting)
            true
        }

        val initialFontRgbaOrderChanged = DesktopSetting.FontRgbaOrder(
            when (initialFontRgbaOrder.value) {
                FontRgbaOrderValue.None -> FontRgbaOrderValue.Rgb
                FontRgbaOrderValue.Rgb -> FontRgbaOrderValue.Bgr
                FontRgbaOrderValue.Bgr -> FontRgbaOrderValue.Vrgb
                FontRgbaOrderValue.Vrgb -> FontRgbaOrderValue.Vbgr
                FontRgbaOrderValue.Vbgr -> FontRgbaOrderValue.None
            },
        )
        XSettingsD.withChangedSetting(initialFontRgbaOrderChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(initialFontRgbaOrderChanged, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialFontRgbaOrder, it.setting)
            true
        }

        val initialMiddleClickPasteChanged = DesktopSetting.MiddleClickPaste(!initialMiddleClickPaste.value)
        XSettingsD.withChangedSetting(initialMiddleClickPasteChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(initialMiddleClickPasteChanged, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialMiddleClickPaste, it.setting)
            true
        }

        val initialFontHintingChanged = DesktopSetting.FontHinting(
            when (initialFontHinting.value) {
                FontHintingValue.None -> FontHintingValue.Slight
                FontHintingValue.Slight -> FontHintingValue.Medium
                FontHintingValue.Medium -> FontHintingValue.Full
                FontHintingValue.Full -> FontHintingValue.None
            },
        )
        XSettingsD.withChangedSetting(initialFontHintingChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(initialFontHintingChanged, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialFontHinting, it.setting)
            true
        }

        val initialOverlayScrollingChanged = DesktopSetting.OverlayScrolling(!initialOverlayScrolling.value)
        XSettingsD.withChangedSetting(initialOverlayScrollingChanged) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(initialOverlayScrollingChanged, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialOverlayScrolling, it.setting)
            true
        }

        val changedDoubleClickDistancePixels = DesktopSetting.DoubleClickDistancePixels(initialDoubleClickDistancePixels.value + 5)
        XSettingsD.withChangedSetting(changedDoubleClickDistancePixels) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(changedDoubleClickDistancePixels, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialDoubleClickDistancePixels, it.setting)
            true
        }

        val changedEnableAnimations = DesktopSetting.EnableAnimations(!initialEnableAnimations.value)
        XSettingsD.withChangedSetting(changedEnableAnimations) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(changedEnableAnimations, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialEnableAnimations, it.setting)
            true
        }

        val changedPrimaryButtonWarpsSlider = DesktopSetting.PrimaryButtonWarpsSlider(!initialPrimaryButtonWarpsSlider.value)
        XSettingsD.withChangedSetting(changedPrimaryButtonWarpsSlider) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(changedPrimaryButtonWarpsSlider, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialPrimaryButtonWarpsSlider, it.setting)
            true
        }

        val changedRecentFilesEnabled = DesktopSetting.RecentFilesEnabled(!initialRecentFilesEnabled.value)
        XSettingsD.withChangedSetting(changedRecentFilesEnabled) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(changedRecentFilesEnabled, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialRecentFilesEnabled, it.setting)
            true
        }

        val changedRecentFilesMaxAgeDays = DesktopSetting.RecentFilesMaxAgeDays(initialRecentFilesMaxAgeDays.value + 2)
        XSettingsD.withChangedSetting(changedRecentFilesMaxAgeDays) {
            val settingChangeEvent = awaitEventOfType<Event.DesktopSettingChange> { true }
            assertEquals(changedRecentFilesMaxAgeDays, settingChangeEvent.setting)
        }
        awaitEventOfType<Event.DesktopSettingChange> {
            assertEquals(initialRecentFilesMaxAgeDays, it.setting)
            true
        }

        assertFalse(initialIsComposited.value) // started without compositor

        val initialScale = initialWindowData.scale.newScale
        assertEquals(2.0, initialScale)

        assertEquals(initialWindowData.configure.size.width * initialScale, initialWindowData.draw.size.width.toDouble())
        assertEquals(initialWindowData.configure.size.height * initialScale, initialWindowData.draw.size.height.toDouble())

        XSettingsD.withChangedSettings(
            mapOf(
                "Gdk/WindowScalingFactor" to "1",
                "Gtk/CursorThemeSize" to "24",
                "Xft/DPI" to "98304",
            ),
        ) {
            val event = getNextEvent()
            assertIs<Event.WindowScaleChanged>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(1.0, event.newScale)

            ui { initialWindowData.window.requestRedraw() }

            withNextEvent { event ->
                assertIs<Event.WindowDraw>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(initialWindowData.configure.size.width, event.size.width)
                assertEquals(initialWindowData.configure.size.height, event.size.height)
            }
        }
        awaitEventOfType<Event.WindowScaleChanged> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(initialScale, event.newScale)
            true
        }
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
    }

    @Test
    fun windowCloseRequestReturningFalse() {
        run(
            defaultApplicationConfig(
                windowCloseRequest = { false },
            ),
        )
        val windowParams = defaultWindowParams()
        val window = createWindowAndWaitForFocus(windowParams).window

        moveMouseTo(50, 50)
        awaitEventOfType<Event.MouseMoved> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }

        runCommand(listOf("i3-msg", "kill"))

        withKeyPress(KeyCode.A) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }

        ui { window.close() }
        withNextEvent { event ->
            assertInstanceOf<Event.MouseExited>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }

        waitForWindowClosed(windowParams.windowId)
        withKeyPress(KeyCode.A) {}
        assertTrue(eventQueue.isEmpty())
    }

    @Test
    fun testSingleWindowCreation() {
        run(defaultApplicationConfig())

        val screens = ui { app.allScreens() }.screens
        val screen = screens.firstOrNull()
        assertNotNull(screen)
        val fullscreenWindowSize = screen.size
        assertEquals(LogicalPoint(0f, 0f), screen.origin)
        assertNotEquals(0.0, screen.scale)
        assertNotNull(screen.name)
        assertNotEquals(0U, screen.screenId)

        val windowParams = defaultWindowParams().copy(minSize = LogicalSize(width = 100, height = 70))
        val window = ui { app.createWindow(windowParams) }

        withNextEvent { event ->
            assertInstanceOf<Event.WindowScaleChanged>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(event.newScale, screen.scale)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.WindowScreenChange>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(screen.screenId, event.newScreenId)
        }

        assertNotNull(
            awaitEvent { event ->
                if (event is Event.WindowScreenChange) {
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(screen.screenId, event.newScreenId)
                    // Ignore additional WindowScreenChange events
                    false
                } else {
                    assertInstanceOf<Event.WindowKeyboardEnter>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    true
                }
            },
        )

        // i3 versions older than 4.24 don't report windows as maximized.
        // https://github.com/i3/i3/commit/b660d6a902cf44be22c434101dd2a4e6743e26bc
        var reportsMaximized = true

        var expectedConfigureEvent = Event.WindowConfigure(
            windowId = windowParams.windowId,
            size = fullscreenWindowSize,
            active = true,
            maximized = true,
            fullscreen = false,
            decorationMode = WindowDecorationMode.Server,
            insetStart = LogicalSize(width = 0, height = 0),
            insetEnd = LogicalSize(width = 0, height = 0),
        )

        withNextEvent { event ->
            assertInstanceOf<Event.WindowConfigure>(event)
            reportsMaximized = event.maximized
            expectedConfigureEvent = expectedConfigureEvent.copy(maximized = reportsMaximized)
            assertEquals(expectedConfigureEvent, event)
        }

        withNextEvent { event ->
            assertInstanceOf<Event.WindowDraw>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(physicalScreenSize, event.size)
        }

        runCommand(listOf("i3-msg", "floating enable, move position 0 0"))

        if (reportsMaximized) {
            expectedConfigureEvent = expectedConfigureEvent.copy(maximized = false)

            withNextEvent { event ->
                assertEquals(expectedConfigureEvent, event)
            }
        }
        expectedConfigureEvent = expectedConfigureEvent.copy(size = windowParams.size)
        withNextEvent { event ->
            assertEquals(expectedConfigureEvent, event)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.WindowDraw>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }

        moveMouseTo(50, 50)
        withNextEvent { event ->
            assertInstanceOf<Event.MouseEntered>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.MouseMoved>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(
                LogicalPoint(
                    x = (50 / screen.scale).toFloat(),
                    y = (50 / screen.scale).toFloat(),
                ),
                event.locationInWindow,
            )
        }

        withMouseButtonDown(MouseButton.LEFT) {
            withNextEvent { event ->
                assertInstanceOf<Event.MouseDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(MouseButton.LEFT, event.button)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.MouseUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(MouseButton.LEFT, event.button)
        }

        moveMouseTo(physicalScreenSize.width - 50, physicalScreenSize.height - 50)
        withNextEvent { event ->
            assertInstanceOf<Event.MouseExited>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }

        runCommand(listOf("i3-msg", "resize set width 1 height 1"))

        expectedConfigureEvent = expectedConfigureEvent.copy(size = windowParams.minSize!!)
        withNextEvent { event ->
            assertEquals(expectedConfigureEvent, event)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.WindowDraw>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }

        assertEquals(windowParams.title, getActiveWindowTitle())
        "New title 🙂".also {
            ui { window.setTitle(it) }
            assertEquals(it, getActiveWindowTitle())
        }

        for (useI3 in listOf(true, false)) {
            if (useI3) {
                runCommand(listOf("i3-msg", "fullscreen enable"))
            } else {
                ui { window.setFullScreen() }
            }

            val fullscreenEnterChecklist = Checklist(
                listOf(
                    "mouseEntered",
                    "mouseMoved",
                    "fullscreen",
                    "resized",
                    "draw",
                ) + if (reportsMaximized) listOf("maximized") else emptyList(),
            )

            while (!fullscreenEnterChecklist.isAllChecked()) {
                val event = getNextEvent()
                val failMsg = { "useI3=$useI3, fullscreenEnterChecklist.uncheckedEntries=${fullscreenEnterChecklist.uncheckedEntries()}" }
                when (event) {
                    is Event.WindowConfigure -> {
                        if (fullscreenEnterChecklist.checkEntry("fullscreen")) {
                            expectedConfigureEvent = expectedConfigureEvent.copy(fullscreen = true)
                            assertEquals(expectedConfigureEvent, event, failMsg())
                        } else if (reportsMaximized && fullscreenEnterChecklist.checkEntry("maximized")) {
                            expectedConfigureEvent = expectedConfigureEvent.copy(maximized = true)
                            assertEquals(expectedConfigureEvent, event, failMsg())
                        } else if (fullscreenEnterChecklist.checkEntry("resized")) {
                            expectedConfigureEvent = expectedConfigureEvent.copy(size = fullscreenWindowSize)
                            assertEquals(expectedConfigureEvent, event, failMsg())
                            moveMouseTo(physicalScreenSize.width - 51, physicalScreenSize.height - 51)
                        } else {
                            fail(withTimestamp("Unexpected event: $event, ${failMsg()}"))
                        }
                    }

                    is Event.MouseEntered -> {
                        assertTrue(fullscreenEnterChecklist.checkEntry("mouseEntered"), failMsg())
                        assertEquals(windowParams.windowId, event.windowId, failMsg())
                    }

                    is Event.MouseMoved -> {
                        assertTrue(fullscreenEnterChecklist.checkEntry("mouseMoved"), failMsg())
                        assertEquals(windowParams.windowId, event.windowId, failMsg())
                    }

                    is Event.WindowDraw -> {
                        assertTrue(fullscreenEnterChecklist.checkEntry("draw"), failMsg())
                        assertTrue(fullscreenEnterChecklist.isChecked("fullscreen"), failMsg())
                        if (reportsMaximized) {
                            assertTrue(fullscreenEnterChecklist.isChecked("maximized"), failMsg())
                        }
                        assertTrue(fullscreenEnterChecklist.isChecked("resized"), failMsg())
                        assertEquals(windowParams.windowId, event.windowId, failMsg())
                    }

                    else -> {
                        fail(withTimestamp("Unexpected event: $event, ${failMsg()}"))
                    }
                }
            }

            if (useI3) {
                runCommand(listOf("i3-msg", "fullscreen disable"))
            } else {
                ui { window.unsetFullScreen() }
            }

            val fullscreenExitChecklist = Checklist(
                listOf(
                    "mouseExited",
                    "notFullscreen",
                    "resized",
                    "draw",
                ) + if (reportsMaximized) listOf("notMaximized") else emptyList(),
            )

            while (!fullscreenExitChecklist.isAllChecked()) {
                val event = getNextEvent()
                val failMsg = { "useI3=$useI3, fullscreenExitChecklist.uncheckedEntries=${fullscreenExitChecklist.uncheckedEntries()}" }
                when (event) {
                    is Event.WindowConfigure -> {
                        if (fullscreenExitChecklist.checkEntry("notFullscreen")) {
                            expectedConfigureEvent = expectedConfigureEvent.copy(fullscreen = false)
                            assertEquals(expectedConfigureEvent, event, failMsg())
                        } else if (reportsMaximized && fullscreenExitChecklist.checkEntry("notMaximized")) {
                            expectedConfigureEvent = expectedConfigureEvent.copy(maximized = false)
                            assertEquals(expectedConfigureEvent, event, failMsg())
                        } else if (fullscreenExitChecklist.checkEntry("resized")) {
                            expectedConfigureEvent = expectedConfigureEvent.copy(size = windowParams.minSize)
                            assertEquals(expectedConfigureEvent, event, failMsg())
                            moveMouseTo(physicalScreenSize.width - 52, physicalScreenSize.height - 52)
                        } else {
                            fail(withTimestamp("Unexpected event: $event, ${failMsg()}"))
                        }
                    }

                    is Event.MouseExited -> {
                        assertTrue(fullscreenExitChecklist.checkEntry("mouseExited"), failMsg())
                        assertEquals(windowParams.windowId, event.windowId)
                    }

                    is Event.WindowDraw -> {
                        assertTrue(fullscreenExitChecklist.checkEntry("draw"), failMsg())
                        assertTrue(fullscreenExitChecklist.isChecked("notFullscreen"), failMsg())
                        if (reportsMaximized) {
                            assertTrue(fullscreenExitChecklist.isChecked("notMaximized"), failMsg())
                        }
                        assertTrue(fullscreenExitChecklist.isChecked("resized"), failMsg())
                        assertEquals(windowParams.windowId, event.windowId, failMsg())
                    }

                    else -> {
                        fail(withTimestamp("Unexpected event: $event, ${failMsg()}"))
                    }
                }
            }
        }

        ui { window.close() }
        waitForWindowClosed(windowParams.windowId)
        withKeyPress(KeyCode.A) {}
        assertTrue(eventQueue.isEmpty())
    }

    @Test
    fun testWindowMinSizeWithInitialMinSize() {
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams().copy(
            size = LogicalSize(width = 50, height = 50),
            minSize = LogicalSize(width = 200, height = 150),
        )

        val window = createWindowAndWaitForFocus(windowParams).window

        runCommand(listOf("i3-msg", "floating enable"))
        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            event.active && event.size == windowParams.minSize!!
        }

        val newMinSize = LogicalSize(windowParams.minSize!!.width + 10, windowParams.minSize.height + 10)
        ui { window.setMinSize(newMinSize) }

        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            event.active && event.size == newMinSize
        }

        val newMinSize2 = LogicalSize(newMinSize.width + 10, newMinSize.height + 10)
        ui { window.setMinSize(newMinSize2) }

        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            event.active && event.size == newMinSize2
        }
    }

    @Test
    fun testWindowMinSizeWithInitialMinSizeNewSizeSmaller() {
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams().copy(
            size = LogicalSize(width = 50, height = 50),
            minSize = LogicalSize(width = 200, height = 150),
        )

        val initialWindowData = createWindowAndWaitForFocus(windowParams)
        val window = initialWindowData.window

        val newMinSize = LogicalSize(windowParams.minSize!!.width - 10, windowParams.minSize.height - 10)
        ui { window.setMinSize(newMinSize) }
        awaitEventOfType<Event.WindowDraw>(msg = "WindowDraw after setMinSize") { true }

        runCommand(listOf("i3-msg", "floating enable"))
        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            if (event.active && event.size != initialWindowData.configure.size) {
                assertEquals(windowParams.minSize, event.size)
                true
            } else {
                false
            }
        }
    }

    @Test
    fun testWindowMinSizeWithoutInitialMinSize() {
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams().copy(
            size = LogicalSize(width = 50, height = 50),
        )

        val window = createWindowAndWaitForFocus(windowParams).window

        val newMinSize = LogicalSize(150, 90)
        ui { window.setMinSize(newMinSize) }
        awaitEventOfType<Event.WindowDraw>(msg = "WindowDraw after setMinSize") { true }

        runCommand(listOf("i3-msg", "floating enable"))
        awaitEventOfType<Event.WindowConfigure>(msg = "active with size $newMinSize") { event ->
            assertEquals(windowParams.windowId, event.windowId)
            event.active && event.size == newMinSize
        }
        ui { window.close() }
        awaitEventOfType<Event.WindowClosed> { true }
    }

    @Test
    fun testWindowSetPointerShape() {
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams()
        val window = createWindowAndWaitForFocus(windowParams).window

        moveMouseTo(physicalScreenSize.width - 100, physicalScreenSize.height - 100)
        awaitEventOfType<Event.MouseMoved> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }

        val screenshots = mutableListOf<Path>()
        val tempDir = Files.createTempDirectory("test_linux_screenshots")

        val blank = tempDir.resolve("_blank.png").also {
            screenshot(it, Pair(PhysicalPoint(0, 0), PhysicalSize(100, 100)), hideCursor = false)
            screenshots.add(it)
        }

        moveMouseTo(50, 50)
        awaitEventOfType<Event.MouseMoved> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }

        val errors = mutableListOf<String>()
        for (shape in PointerShape.entries) {
            ui { window.setPointerShape(shape) }

            tempDir.resolve("$shape.png").also { shapeScreenshotPath ->
                var retryCount = 20
                while (retryCount > 0) {
                    screenshot(shapeScreenshotPath, Pair(PhysicalPoint(0, 0), PhysicalSize(100, 100)), hideCursor = false)
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
        screenshots.forEach { it.deleteIfExists() }
        tempDir.deleteIfExists()
    }

    @Test
    fun testMultipleMouseButtonsAtOnce() {
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams()
        createWindowAndWaitForFocus(windowParams)

        moveMouseTo(101, 10)

        awaitEventOfType<Event.MouseMoved> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }

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
    }

    @Test
    fun testMultipleWindowCreationWithSameId() {
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams()
        ui { app.createWindow(windowParams) }

        assertThrows<Error> { ui { app.createWindow(windowParams) } }
    }

    @Test
    fun testMultipleWindowCreation() {
        run(defaultApplicationConfig())

        val window1Params = defaultWindowParams()
        val window1 = ui { app.createWindow(window1Params) }

        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(window1Params.windowId, event.windowId)
            event.active
        }

        var window1Framebuffer: Int? = null
        awaitEventOfType<Event.WindowDraw> { event ->
            assertEquals(window1Params.windowId, event.windowId, "Draw first window: $event")
            window1Framebuffer = event.openGlDrawData.framebuffer
            true
        }
        assertNotNull(window1Framebuffer)

        val window2Params = WindowParams(
            windowId = 1,
            title = "Test Window 2",
            size = LogicalSize(width = 300, height = 200),
            minSize = null,
            decorationMode = WindowDecorationMode.CustomTitlebar(40),
            renderingMode = RenderingMode.Auto,
        )
        val window2 = ui { app.createWindow(window2Params) }

        awaitEventOfType<Event.WindowConfigure> { event ->
            window1Params.windowId == event.windowId && !event.active
        }

        awaitEventOfType<Event.WindowConfigure> { event ->
            window2Params.windowId == event.windowId && event.active && window2Params.decorationMode == event.decorationMode
        }

        var window2Framebuffer: Int? = null
        awaitEventOfType<Event.WindowDraw> { event ->
            if (window2Params.windowId == event.windowId) {
                window2Framebuffer = event.openGlDrawData.framebuffer
                true
            } else {
                false
            }
        }
        assertNotNull(window2Framebuffer)
        assertNotEquals(window1Framebuffer, window2Framebuffer)

        ui { window1.activate(null) }

        awaitEventOfType<Event.WindowConfigure> { event ->
            window1Params.windowId == event.windowId && event.active
        }
        awaitEventOfType<Event.WindowConfigure> { event ->
            window2Params.windowId == event.windowId && !event.active
        }

        ui { window2.activate("something") } // i3 does not have focus stealing protection, so any token is valid

        awaitEventOfType<Event.WindowConfigure> { event ->
            window2Params.windowId == event.windowId && event.active
        }
        awaitEventOfType<Event.WindowConfigure> { event ->
            window1Params.windowId == event.windowId && !event.active
        }

        ui {
            window1.close()
            window2.close()
        }
        awaitEventOfType<Event.WindowClosed> { event ->
            assertEquals(window1Params.windowId, event.windowId)
            true
        }
        awaitEventOfType<Event.WindowClosed> { event ->
            assertEquals(window2Params.windowId, event.windowId)
            true
        }
    }

    @Test
    fun testSetClipboardContentForText() {
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
                        DataSource.Clipboard -> content[mimeType]
                        else -> null
                    }
                },
            ),
        )

        withSetClipboardContent(listOf(HTML_TEXT_MIME_TYPE, TEXT_UTF8_MIME_TYPE)) {
            runCommandWithOutput(listOf("xclip", "-selection", "clipboard", "-o", "-t", "TARGETS")).also {
                val expected = """SAVE_TARGETS
TIMESTAMP
TARGETS
text/html
UTF8_STRING
COMPOUND_TEXT
TEXT
STRING
text/plain;charset=utf-8
""".toByteArray()
                assertContentEquals(expected, it)
            }

            assertContentEquals(htmlContent, getClipboardContent(HTML_TEXT_MIME_TYPE))
            assertContentEquals(textContent, getClipboardContent(TEXT_UTF8_MIME_TYPE))
            assertContentEquals(textContent, getClipboardContent("UTF8_STRING"))
            assertContentEquals(textContent + 0.toByte(), getClipboardContent("TEXT"))
            assertContentEquals(textContent + 0.toByte(), getClipboardContent("STRING"))
        }
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

        withSetClipboardContent(listOf(HTML_TEXT_MIME_TYPE, TEXT_UTF8_MIME_TYPE)) {
            runCommandWithOutput(listOf("xclip", "-selection", "clipboard", "-o", "-t", "TARGETS")).also {
                val expected = """SAVE_TARGETS
TIMESTAMP
TARGETS
text/html
UTF8_STRING
COMPOUND_TEXT
TEXT
STRING
text/plain;charset=utf-8
""".toByteArray()
                assertContentEquals(expected, it)
            }

            assertContentEquals(htmlContent, getClipboardContent(HTML_TEXT_MIME_TYPE))
            assertContentEquals(textContent, getClipboardContent(TEXT_UTF8_MIME_TYPE))
            assertContentEquals(textContent, getClipboardContent("UTF8_STRING"))
            // TODO? "TEXT" and "STRING" types are not working well with unicode content
        }
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

        withSetClipboardContent(listOf(TEXT_UTF8_MIME_TYPE)) {
            runCommandWithOutput(listOf("xclip", "-selection", "clipboard", "-o", "-t", "TARGETS")).also {
                val expected = """SAVE_TARGETS
TIMESTAMP
TARGETS
UTF8_STRING
COMPOUND_TEXT
TEXT
STRING
text/plain;charset=utf-8
""".toByteArray()
                assertContentEquals(expected, it)
            }

            assertContentEquals(textContent, getClipboardContent(TEXT_UTF8_MIME_TYPE))
            assertContentEquals(textContent, getClipboardContent("UTF8_STRING"))
            assertContentEquals(byteArrayOf(0), getClipboardContent("TEXT"))
            assertContentEquals(byteArrayOf(0), getClipboardContent("STRING"))
        }
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

        withSetPrimarySelectionContent(listOf(HTML_TEXT_MIME_TYPE, TEXT_UTF8_MIME_TYPE)) {
            runCommandWithOutput(listOf("xclip", "-selection", "primary", "-o", "-t", "TARGETS")).also {
                val expected = """SAVE_TARGETS
TIMESTAMP
TARGETS
text/html
UTF8_STRING
COMPOUND_TEXT
TEXT
STRING
text/plain;charset=utf-8
""".toByteArray()
                assertContentEquals(expected, it)
            }

            assertContentEquals(htmlContent, getPrimarySelectionContent(HTML_TEXT_MIME_TYPE))
            assertContentEquals(textContent, getPrimarySelectionContent(TEXT_UTF8_MIME_TYPE))
            assertContentEquals(textContent, getPrimarySelectionContent("UTF8_STRING"))
            assertContentEquals(textContent + 0.toByte(), getPrimarySelectionContent("TEXT"))
            assertContentEquals(textContent + 0.toByte(), getPrimarySelectionContent("STRING"))
        }
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

        withSetClipboardContent(listOf(TEXT_UTF8_MIME_TYPE)) {
            assertContentEquals(byteArrayOf(), getClipboardContent(HTML_TEXT_MIME_TYPE))
        }
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

        withSetPrimarySelectionContent(listOf(TEXT_UTF8_MIME_TYPE)) {
            assertContentEquals(byteArrayOf(), getPrimarySelectionContent(HTML_TEXT_MIME_TYPE))
        }
    }

    @Test
    fun testClipboardPaste() {
        run(defaultApplicationConfig())

        TestApp.ClipboardSource.run {
            awaitEventOfType<Event.DataTransferAvailable> { event ->
                assertEquals(DataSource.Clipboard, event.dataSource)
                assertEquals(emptyList(), event.mimeTypes)
                true
            }
            awaitEventOfType<Event.DataTransferAvailable> { event ->
                assertEquals(DataSource.Clipboard, event.dataSource)
                assertEquals(listOf(HTML_TEXT_MIME_TYPE, URI_LIST_MIME_TYPE, TEXT_UTF8_MIME_TYPE), event.mimeTypes)
                true
            }

            val availableMimeTypes = ui { app.clipboardGetAvailableMimeTypes() }
            assertEquals(listOf(HTML_TEXT_MIME_TYPE, URI_LIST_MIME_TYPE, TEXT_UTF8_MIME_TYPE), availableMimeTypes)

            val transferSerial1 = 5
            ui { app.clipboardPaste(transferSerial1, listOf(TEXT_UTF8_MIME_TYPE, PNG_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial1, event.serial)
                val content = event.content
                assertNotNull(content)
                assertEquals(TEXT_UTF8_MIME_TYPE, content.mimeType)
                assertContentEquals(
                    $$"/some/path/With Spaces/& $p€¢ïåł çħāřß\n/tmp/[Screenshot from 12:04:42].png".encodeToByteArray(),
                    content.data,
                )
            }

            val transferSerial2 = 6
            ui { app.clipboardPaste(transferSerial2, listOf(URI_LIST_MIME_TYPE, TEXT_UTF8_MIME_TYPE)) }
            withNextEvent { event ->
                assertInstanceOf<Event.DataTransfer>(event)
                assertEquals(transferSerial2, event.serial)
                val content = event.content
                assertNotNull(content)
                assertEquals(URI_LIST_MIME_TYPE, content.mimeType)
                assertContentEquals(
                    (
                        $$"file:///some/path/With%20Spaces/&%20$p%E2%82%AC%C2%A2%C3%AF%C3%A5%C5%82%20%C3%A7%C4%A7%C4%81%C5%99%C3%9F\r\n" +
                            "file:///tmp/%5BScreenshot%20from%2012:04:42%5D.png\r\n"
                        ).encodeToByteArray(),
                    content.data,
                )
            }
        }
    }

    @Test
    fun testPrimarySelectionPaste() {
        run(defaultApplicationConfig())

        TestApp.PrimarySelectionSource.run {
            awaitEventOfType<Event.DataTransferAvailable> { event ->
                assertEquals(DataSource.PrimarySelection, event.dataSource)
                assertEquals(emptyList(), event.mimeTypes)
                true
            }
            awaitEventOfType<Event.DataTransferAvailable> { event ->
                assertEquals(DataSource.PrimarySelection, event.dataSource)
                assertEquals(listOf(HTML_TEXT_MIME_TYPE, TEXT_UTF8_MIME_TYPE), event.mimeTypes)
                true
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
                assertContentEquals("Text from TestAppPrimarySelectionSource".encodeToByteArray(), content.data)
            }
        }
    }

    @Test
    fun testPasteWithoutClipboard() {
        run(defaultApplicationConfig())
        ui { app.clipboardGetAvailableMimeTypes() }.also {
            assertEquals(emptyList(), it)
        }
        ui { app.primarySelectionGetAvailableMimeTypes() }.also {
            assertEquals(emptyList(), it)
        }

        val clipboardPasteSerial = 6
        ui { app.clipboardPaste(clipboardPasteSerial, listOf(TEXT_UTF8_MIME_TYPE)) }
        withNextEvent { event ->
            assertInstanceOf<Event.DataTransfer>(event)
            assertEquals(clipboardPasteSerial, event.serial)
            assertNull(event.content)
        }

        val primarySelectionPasteSerial = 6
        ui { app.primarySelectionPaste(primarySelectionPasteSerial, listOf(TEXT_UTF8_MIME_TYPE)) }
        withNextEvent { event ->
            assertInstanceOf<Event.DataTransfer>(event)
            assertEquals(primarySelectionPasteSerial, event.serial)
            assertNull(event.content)
        }
    }

    @Test
    fun testKeyEvents() {
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams()
        createWindowAndWaitForFocus(windowParams)

        moveMouseTo(100, 100)
        withNextEvent { event ->
            assertInstanceOf<Event.MouseEntered>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.MouseMoved>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }

        withKeyPress(KeyCode.A) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals("a", event.characters)
                assertEquals(KeyCode.A, event.keyCode.value)
                assertEquals(KeySym.a, event.key.value)
                assertEquals(emptySet(), event.modifiers)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }

        withKeyPress(KeyCode.Return) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.Return, event.keyCode.value)
                assertEquals(KeySym.Return, event.key.value)
                assertContentEquals("\r".toByteArray(), event.characters?.toByteArray())
                assertEquals(emptySet(), event.modifiers)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(KeyCode.Return, event.keyCode.value)
            assertEquals(KeySym.Return, event.key.value)
        }

        withKeyPress(KeyCode.Escape) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.Escape, event.keyCode.value)
                assertEquals(KeySym.Escape, event.key.value)
                assertContentEquals("\u001b".toByteArray(), event.characters?.toByteArray())
                assertEquals(emptySet(), event.modifiers)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(KeyCode.Escape, event.keyCode.value)
            assertEquals(KeySym.Escape, event.key.value)
        }

        withKeyPress(KeyCode.BackSpace) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.BackSpace, event.keyCode.value)
                assertEquals(KeySym.BackSpace, event.key.value)
                assertContentEquals("\b".toByteArray(), event.characters?.toByteArray())
                assertEquals(emptySet(), event.modifiers)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(KeyCode.BackSpace, event.keyCode.value)
            assertEquals(KeySym.BackSpace, event.key.value)
        }

        withKeyPress(KeyCode.Tab) {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.Tab, event.keyCode.value)
                assertEquals(KeySym.Tab, event.key.value)
                assertContentEquals("\t".toByteArray(), event.characters?.toByteArray())
                assertEquals(emptySet(), event.modifiers)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(KeyCode.Tab, event.keyCode.value)
            assertEquals(KeySym.Tab, event.key.value)
        }

        withKeyPress(KeyCode.Shift_L) {
            withNextEvent { event ->
                assertInstanceOf<Event.ModifiersChanged>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(setOf(KeyModifiers.Shift), event.modifiers)
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertNull(event.characters)
                assertEquals(KeyCode.Shift_L, event.keyCode.value)
                assertEquals(KeySym.Shift_L, event.key.value)
                assertEquals(setOf(KeyModifiers.Shift), event.modifiers)
            }
            withKeyPress(KeyCode.A) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals("A", event.characters)
                    assertEquals(KeyCode.A, event.keyCode.value)
                    assertEquals(KeySym.A, event.key.value)
                    assertEquals(setOf(KeyModifiers.Shift), event.modifiers)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.A, event.keyCode.value)
                assertEquals(KeySym.A, event.key.value)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.ModifiersChanged>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(emptySet(), event.modifiers)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(KeyCode.Shift_L, event.keyCode.value)
            assertEquals(KeySym.Shift_L, event.key.value)
        }

        withKeyPress(KeyCode.Control_L) {
            withNextEvent { event ->
                assertInstanceOf<Event.ModifiersChanged>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(setOf(KeyModifiers.Control), event.modifiers)
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertNull(event.characters)
                assertEquals(KeyCode.Control_L, event.keyCode.value)
                assertEquals(KeySym.Control_L, event.key.value)
                assertEquals(setOf(KeyModifiers.Control), event.modifiers)
            }
            withKeyPress(KeyCode.A) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals("a", event.characters) // TODO(nk): should this be control character instead?
                    assertEquals(KeyCode.A, event.keyCode.value)
                    assertEquals(KeySym.a, event.key.value)
                    assertEquals(setOf(KeyModifiers.Control), event.modifiers)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.A, event.keyCode.value)
                assertEquals(KeySym.a, event.key.value)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.ModifiersChanged>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(emptySet(), event.modifiers)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(KeyCode.Control_L, event.keyCode.value)
        }
    }

    private fun getExpectedImePopupPositionLogLine(pos: LogicalRect, scale: Double): String {
        val x = (pos.x * scale).roundToInt()
        val y = (pos.y * scale).roundToInt()
        val w = (pos.width * scale).roundToInt()
        val h = (pos.height * scale).roundToInt()
        return "do_set_cursor_location: x=$x, y=$y, w=$w, h=$h"
    }

    /** Returns `expectedLine` if the actual line is expected, otherwise `actualLine` */
    private fun checkImePopupPosition(expectedLine: String, actualLine: String?, pos: LogicalRect, scale: Double): String? {
        return if (actualLine == expectedLine) {
            expectedLine
        } else {
            // Before IBus 1.5.29, the IBus implementation manually adjusted the popup positioning, see
            // https://github.com/ibus/ibus/commit/86d9bb9a1cbd4ffbd6bc2a4de85cb76a43bc2ced#diff-38bdd5907be96a96f7865645bc04415affe87a1a63be05cbc3b01e7bddff6d22L1633
            val x = (pos.x * scale).roundToInt()
            val y = ((pos.y + 32) * scale).roundToInt()
            val w = (50 * scale).roundToInt()
            val h = (physicalScreenSize.height * scale).roundToInt()
            val adjustedExpectedLine = "do_set_cursor_location: x=$x, y=$y, w=$w, h=$h"
            if (actualLine == adjustedExpectedLine) {
                expectedLine
            } else {
                actualLine
            }
        }
    }

    private fun assertImePopupPosition(actualLine: String?, pos: LogicalRect, scale: Double) {
        val expectedLine = getExpectedImePopupPositionLogLine(pos, scale)
        assertEquals(expectedLine, checkImePopupPosition(expectedLine, actualLine, pos, scale))
    }

    @Test
    fun testTextInputContext() {
        val capsTextOutput = IBusTestEngineOutput("TEST_IBUS_ENGINE_CAPS_OUT_FILE")
        val cursorLocTextOutput = IBusTestEngineOutput("TEST_IBUS_ENGINE_CURSOR_LOCATION_OUT_FILE")
        val contentTypeTextOutput = IBusTestEngineOutput("TEST_IBUS_ENGINE_CONTENT_TYPE_OUT_FILE")

        withTestImeEngine {
            run(
                defaultApplicationConfig(getSurroundingText = {
                    TextInputSurroundingText(
                        surroundingText = "",
                        cursorCodepointOffset = 0U,
                        selectionStartCodepointOffset = 0U,
                    )
                }),
            )
            val ibusEngine = runCommandWithOutput(listOf("ibus", "engine"))
            assertEquals("jb_kdt_ibus_test_engine\n", ibusEngine.decodeToString())
            val windowParams = defaultWindowParams()

            var textInputContext = TextInputContext(
                hints = setOf(TextInputContextHint.WordCompletion, TextInputContextHint.Spellcheck),
                contentPurpose = TextInputContentPurpose.Normal,
                cursorRectangle = LogicalRect(x = 50, y = 20, width = 5, height = 10),
            )

            val initialWindowData = createWindowAndWaitForFocus(windowParams)
            val window = initialWindowData.window
            val scale = initialWindowData.scale.newScale
            ui { window.textInputEnable(textInputContext) }

            assertEquals("do_set_capabilities: ['PREEDIT_TEXT', 'FOCUS', 'SURROUNDING_TEXT']", capsTextOutput.read())
            assertEquals(
                "do_set_content_type: purpose = FREE_FORM, hints = ['SPELLCHECK', 'WORD_COMPLETION']",
                contentTypeTextOutput.read(),
            )
            getExpectedImePopupPositionLogLine(textInputContext.cursorRectangle, scale).also { expectedLine ->
                waitUntilEq(expectedLine) {
                    checkImePopupPosition(expectedLine, cursorLocTextOutput.read(-1), textInputContext.cursorRectangle, scale)
                }
            }

            textInputContext = textInputContext.copy(
                hints = setOf(
                    TextInputContextHint.Spellcheck,
                    TextInputContextHint.NoSpellcheck,
                    TextInputContextHint.WordCompletion,
                    TextInputContextHint.Lowercase,
                    TextInputContextHint.UppercaseChars,
                    TextInputContextHint.UppercaseWords,
                    TextInputContextHint.UppercaseSentences,
                    TextInputContextHint.InhibitOsk,
                    TextInputContextHint.VerticalWriting,
                    TextInputContextHint.Emoji,
                    TextInputContextHint.NoEmoji,
                    TextInputContextHint.Private,
                ),
            )
            ui { window.textInputUpdate(textInputContext) }

            assertEquals(
                "do_set_content_type: purpose = FREE_FORM, hints = [" +
                    "'SPELLCHECK', 'NO_SPELLCHECK', 'WORD_COMPLETION', 'LOWERCASE', 'UPPERCASE_CHARS', 'UPPERCASE_WORDS', " +
                    "'UPPERCASE_SENTENCES', 'INHIBIT_OSK', 'VERTICAL_WRITING', 'EMOJI', 'NO_EMOJI', 'PRIVATE']",
                contentTypeTextOutput.read(),
            )

//            assertFalse(cursorLocTextOutput.exists())
            textInputContext = textInputContext.copy(
                contentPurpose = TextInputContentPurpose.Alpha,
                hints = setOf(TextInputContextHint.Spellcheck),
                cursorRectangle = LogicalRect(x = 0, y = 0, width = 0, height = 0),
            )
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = ALPHA, hints = ['SPELLCHECK']", contentTypeTextOutput.read(2))
            assertImePopupPosition(cursorLocTextOutput.read(), textInputContext.cursorRectangle, scale)

            textInputContext = textInputContext.copy(
                contentPurpose = TextInputContentPurpose.Digits,
                hints = setOf(TextInputContextHint.NoSpellcheck),
                cursorRectangle = LogicalRect(x = -1, y = -1, width = -1, height = -1),
            )
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = DIGITS, hints = ['NO_SPELLCHECK']", contentTypeTextOutput.read(2))
            assertImePopupPosition(cursorLocTextOutput.read(), textInputContext.cursorRectangle, scale)

            textInputContext = textInputContext.copy(
                contentPurpose = TextInputContentPurpose.Number,
                hints = setOf(TextInputContextHint.WordCompletion),
            )
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = NUMBER, hints = ['WORD_COMPLETION']", contentTypeTextOutput.read(2))

            textInputContext = textInputContext.copy(
                contentPurpose = TextInputContentPurpose.Phone,
                hints = setOf(TextInputContextHint.UppercaseChars),
                cursorRectangle = LogicalRect(x = 1000000000, y = 1000000000, width = 1000000000, height = 1000000000),
            )
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = PHONE, hints = ['UPPERCASE_CHARS']", contentTypeTextOutput.read(2))
            assertImePopupPosition(cursorLocTextOutput.read(), textInputContext.cursorRectangle, scale)

            textInputContext =
                textInputContext.copy(contentPurpose = TextInputContentPurpose.Url, hints = setOf(TextInputContextHint.Lowercase))
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = URL, hints = ['LOWERCASE']", contentTypeTextOutput.read(2))

            textInputContext = textInputContext.copy(
                contentPurpose = TextInputContentPurpose.Email,
                hints = setOf(TextInputContextHint.UppercaseSentences),
                cursorRectangle = LogicalRect(x = -1000000000, y = -1000000000, width = -1000000000, height = -1000000000),
            )
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = EMAIL, hints = ['UPPERCASE_SENTENCES']", contentTypeTextOutput.read(2))
            assertImePopupPosition(cursorLocTextOutput.read(), textInputContext.cursorRectangle, scale)

            textInputContext = textInputContext.copy(
                contentPurpose = TextInputContentPurpose.Name,
                hints = setOf(TextInputContextHint.UppercaseWords),
                cursorRectangle = LogicalRect(x = 1, y = 2, width = 3, height = 4),
            )
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = NAME, hints = ['UPPERCASE_WORDS']", contentTypeTextOutput.read(2))
            assertImePopupPosition(cursorLocTextOutput.read(), textInputContext.cursorRectangle, scale)

            textInputContext = textInputContext.copy(
                contentPurpose = TextInputContentPurpose.Password,
                hints = setOf(TextInputContextHint.InhibitOsk),
            )
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = PASSWORD, hints = ['INHIBIT_OSK']", contentTypeTextOutput.read(2))

            textInputContext =
                textInputContext.copy(contentPurpose = TextInputContentPurpose.Pin, hints = setOf(TextInputContextHint.Private))
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = PIN, hints = ['PRIVATE']", contentTypeTextOutput.read(2))

            textInputContext =
                textInputContext.copy(contentPurpose = TextInputContentPurpose.Alpha, hints = setOf(TextInputContextHint.Emoji))
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = ALPHA, hints = ['EMOJI']", contentTypeTextOutput.read(2))

            textInputContext = textInputContext.copy(
                contentPurpose = TextInputContentPurpose.Terminal,
                hints = setOf(TextInputContextHint.NoEmoji),
            )
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = TERMINAL, hints = ['NO_EMOJI']", contentTypeTextOutput.read(2))

            assertFalse(cursorLocTextOutput.exists())
            textInputContext = textInputContext.copy(
                contentPurpose = TextInputContentPurpose.Normal,
                hints = setOf(TextInputContextHint.VerticalWriting),
                cursorRectangle = LogicalRect(x = 0, y = 0, width = 0, height = 0),
            )
            ui { window.textInputUpdate(textInputContext) }
            assertEquals("do_set_content_type: purpose = FREE_FORM, hints = ['VERTICAL_WRITING']", contentTypeTextOutput.read(2))
            assertImePopupPosition(cursorLocTextOutput.read(), textInputContext.cursorRectangle, scale)
        }
    }

    @Test
    fun testIme() {
        var currentSurroundingText = TextInputSurroundingText(
            surroundingText = "",
            cursorCodepointOffset = 0U,
            selectionStartCodepointOffset = 0U,
        )
        val getSurroundingTextTriggered = CompletableFuture<Unit>()
        withTestImeEngine {
            run(
                defaultApplicationConfig(getSurroundingText = {
                    getSurroundingTextTriggered.complete(Unit)
                    currentSurroundingText
                }),
            )
            val ibusEngine = runCommandWithOutput(listOf("ibus", "engine"))
            assertEquals("jb_kdt_ibus_test_engine\n", ibusEngine.decodeToString())
            val windowParams = defaultWindowParams()

            val defaultTextInputContext = TextInputContext(
                hints = emptySet(),
                contentPurpose = TextInputContentPurpose.Normal,
                cursorRectangle = LogicalRect(x = 200, y = 100, width = 10, height = 20),
            )

            val window = createWindowAndWaitForFocus(windowParams).window
            ui { window.textInputEnable(defaultTextInputContext) }

            moveMouseTo(100, 100)
            withNextEvent { event ->
                assertInstanceOf<Event.MouseEntered>(event)
                assertEquals(windowParams.windowId, event.windowId)
            }
            withNextEvent { event ->
                assertInstanceOf<Event.MouseMoved>(event)
                assertEquals(windowParams.windowId, event.windowId)
            }

            val keyCodes = mapOf(
                KeyCode.`1` to "1",
                KeyCode.`2` to "2",
                KeyCode.`3` to "3",
                KeyCode.`4` to "4",
                KeyCode.`5` to "5",
                KeyCode.`6` to "6",
                KeyCode.`7` to "7",
                KeyCode.`8` to "8",
                KeyCode.`9` to "9",
                KeyCode.`0` to "0",
                KeyCode.minus to "-",
                KeyCode.equal to "=",
                KeyCode.Q to "q",
                KeyCode.W to "w",
                KeyCode.E to "e",
                KeyCode.R to "r",
                KeyCode.T to "t",
                KeyCode.Y to "y",
                KeyCode.U to "u",
                KeyCode.I to "i",
                KeyCode.O to "o",
                KeyCode.P to "p",
                KeyCode.bracketleft to "[",
                KeyCode.bracketright to "]",
                KeyCode.A to "a",
                KeyCode.S to "s",
                KeyCode.D to "d",
                KeyCode.F to "f",
                KeyCode.G to "g",
                KeyCode.H to "h",
                KeyCode.J to "j",
                KeyCode.K to "k",
                KeyCode.L to "l",
                KeyCode.semicolon to ";",
                KeyCode.apostrophe to "'",
                KeyCode.backslash to "\\",
                KeyCode.Z to "z",
                KeyCode.X to "x",
                KeyCode.C to "c",
                KeyCode.V to "v",
                KeyCode.B to "b",
                KeyCode.N to "n",
                KeyCode.M to "m",
                KeyCode.comma to ",",
                KeyCode.period to ".",
                KeyCode.slash to "/",
            )

            for ((keyCode, c) in keyCodes) {
                withKeyPress(keyCode) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event, c)
                        assertEquals(windowParams.windowId, event.windowId, c)
                        val commitStringData = event.commitStringData
                        assertNotNull(commitStringData, c)
                        assertEquals(c, commitStringData.text)
                        assertNull(event.preeditStringData, c)
                        assertNull(event.deleteSurroundingTextData, c)
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyUp>(event, c)
                    assertEquals(windowParams.windowId, event.windowId, c)
                }
            }

            // Have to do some "regular" keypresses first; otherwise older IBus (e.g., version 1.5.26) will not trigger
            // fetching of surrounding text.
            // This synchronization is needed to prevent "preedit-end" signal, which happens when the surrounding text
            // fetching happens after the dead key press.
            getSurroundingTextTriggered.get(1000, TimeUnit.MILLISECONDS)

            // This is a dead key in us(intl) layout, which is set by default for tests.
            withKeyPress(KeyCode.grave) {
                withNextEvent { event ->
                    assertInstanceOf<Event.TextInput>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    val preeditStringData = event.preeditStringData
                    assertNotNull(preeditStringData)
                    assertEquals("`", preeditStringData.text)
                    assertEquals(1, preeditStringData.cursorBytePos)
                    assertContentEquals(
                        arrayOf(
                            TextInputPreeditAttribute(
                                beginBytePos = 0U,
                                endBytePos = 1U,
                                underline = TextInputPreeditUnderlineType.Low,
                                foregroundHighlight = false,
                                backgroundHighlight = true,
                                strikethrough = false,
                                bold = false,
                                italic = false,
                            ),
                        ),
                        preeditStringData.attributes.toTypedArray(),
                    )
                    assertNull(event.commitStringData)
                    assertNull(event.deleteSurroundingTextData)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.grave, event.keyCode.value)
//            assertEquals(0xfe50.toUInt(), event.key.value)  // XKB_KEY_dead_grave
            }

            withKeyPress(KeyCode.E) {
                withNextEvent { event ->
                    assertInstanceOf<Event.TextInput>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    val commitStringData = event.commitStringData
                    assertNotNull(commitStringData)
                    assertEquals("è", commitStringData.text)
                    assertNull(event.preeditStringData)
                    assertNull(event.deleteSurroundingTextData)
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.TextInput>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    val preeditStringData = event.preeditStringData
                    assertNotNull(preeditStringData)
                    assertEquals("", preeditStringData.text)
                    assertEquals(0, preeditStringData.cursorBytePos)
                    assertContentEquals(emptyArray(), preeditStringData.attributes.toTypedArray())
                    assertNull(event.commitStringData)
                    assertNull(event.deleteSurroundingTextData)
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.TextInput>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertNull(event.preeditStringData)
                    assertNull(event.commitStringData)
                    assertNull(event.deleteSurroundingTextData)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
            }

            withKeyPress(KeyCode.Shift_L) {
                withNextEvent { event ->
                    assertInstanceOf<Event.ModifiersChanged>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(setOf(KeyModifiers.Shift), event.modifiers)
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertNull(event.characters)
                    assertEquals(KeyCode.Shift_L, event.keyCode.value)
                    assertEquals(setOf(KeyModifiers.Shift), event.modifiers)
                }
                withKeyPress(KeyCode.A) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        val commitStringData = event.commitStringData
                        assertNotNull(commitStringData)
                        assertEquals("A", commitStringData.text)
                        assertNull(event.preeditStringData)
                        assertNull(event.deleteSurroundingTextData)
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyUp>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.A, event.keyCode.value)
                    assertEquals(KeySym.A, event.key.value)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.ModifiersChanged>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(emptySet(), event.modifiers)
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.Shift_L, event.keyCode.value)
            }

            // Control characters should still be reported as just KeyDown events.

            withKeyPress(KeyCode.Return) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.Return, event.keyCode.value)
                    assertEquals(KeySym.Return, event.key.value)
                    assertContentEquals("\r".toByteArray(), event.characters?.toByteArray())
                    assertEquals(emptySet(), event.modifiers)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.Return, event.keyCode.value)
                assertEquals(KeySym.Return, event.key.value)
            }

            withKeyPress(KeyCode.Escape) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.Escape, event.keyCode.value)
                    assertEquals(KeySym.Escape, event.key.value)
                    assertContentEquals("\u001b".toByteArray(), event.characters?.toByteArray())
                    assertEquals(emptySet(), event.modifiers)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.Escape, event.keyCode.value)
                assertEquals(KeySym.Escape, event.key.value)
            }

            withKeyPress(KeyCode.BackSpace) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.BackSpace, event.keyCode.value)
                    assertEquals(KeySym.BackSpace, event.key.value)
                    assertContentEquals("\b".toByteArray(), event.characters?.toByteArray())
                    assertEquals(emptySet(), event.modifiers)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.BackSpace, event.keyCode.value)
                assertEquals(KeySym.BackSpace, event.key.value)
            }

            withKeyPress(KeyCode.Tab) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.Tab, event.keyCode.value)
                    assertEquals(KeySym.Tab, event.key.value)
                    assertContentEquals("\t".toByteArray(), event.characters?.toByteArray())
                    assertEquals(emptySet(), event.modifiers)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.Tab, event.keyCode.value)
                assertEquals(KeySym.Tab, event.key.value)
            }

            withKeyPress(KeyCode.Down) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.Down, event.keyCode.value)
                    assertEquals(KeySym.Down, event.key.value)
                    assertNull(event.characters)
                    assertEquals(emptySet(), event.modifiers)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.Down, event.keyCode.value)
                assertEquals(KeySym.Down, event.key.value)
            }

            withKeyPress(KeyCode.Right) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.Right, event.keyCode.value)
                    assertEquals(KeySym.Right, event.key.value)
                    assertNull(event.characters)
                    assertEquals(emptySet(), event.modifiers)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.Right, event.keyCode.value)
                assertEquals(KeySym.Right, event.key.value)
            }

            withKeyPress(KeyCode.Control_L) {
                withNextEvent { event ->
                    assertInstanceOf<Event.ModifiersChanged>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(setOf(KeyModifiers.Control), event.modifiers)
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertNull(event.characters)
                    assertEquals(KeyCode.Control_L, event.keyCode.value)
                    assertEquals(KeySym.Control_L, event.key.value)
                    assertEquals(setOf(KeyModifiers.Control), event.modifiers)
                }
                currentSurroundingText = TextInputSurroundingText(
                    surroundingText = "àéîõü",
                    cursorCodepointOffset = 2U,
                    selectionStartCodepointOffset = 2U,
                )

                withKeyPress(KeyCode.A) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.KeyDown>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertEquals("a", event.characters) // TODO(nk): should this be control character instead?
                        assertEquals(KeyCode.A, event.keyCode.value)
                        assertEquals(KeySym.a, event.key.value)
                        assertEquals(setOf(KeyModifiers.Control), event.modifiers)
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyUp>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.A, event.keyCode.value)
                    assertEquals(KeySym.a, event.key.value)
                }

                // tell test IME engine to uppercase the letter before the cursor
                withKeyPress(KeyCode.U) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertNull(event.commitStringData)
                        assertNull(event.preeditStringData)
                        val deleteSurroundingTextData = event.deleteSurroundingTextData
                        assertNotNull(deleteSurroundingTextData)
                        assertEquals(2U, deleteSurroundingTextData.beforeLengthInBytes)
                        assertEquals(0U, deleteSurroundingTextData.afterLengthInBytes)
                    }
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertNull(event.deleteSurroundingTextData)
                        assertNull(event.preeditStringData)
                        val commitStringData = event.commitStringData
                        assertNotNull(commitStringData)
                        assertEquals("É", commitStringData.text)
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyUp>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.U, event.keyCode.value)
                    assertEquals(KeySym.u, event.key.value)
                }

                // The first emoji has 5 Unicode code points
                currentSurroundingText = TextInputSurroundingText(
                    surroundingText = "\uD83E\uDDD1\uD83C\uDFFC\u200D\u2695\uFE0Féiõü",
                    cursorCodepointOffset = 7U,
                    selectionStartCodepointOffset = 7U,
                )

                // tell test IME engine to uppercase the letter before the cursor
                withKeyPress(KeyCode.U) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertNull(event.commitStringData)
                        assertNull(event.preeditStringData)
                        val deleteSurroundingTextData = event.deleteSurroundingTextData
                        assertNotNull(deleteSurroundingTextData)
                        assertEquals(1U, deleteSurroundingTextData.beforeLengthInBytes)
                        assertEquals(0U, deleteSurroundingTextData.afterLengthInBytes)
                    }
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertNull(event.deleteSurroundingTextData)
                        assertNull(event.preeditStringData)
                        val commitStringData = event.commitStringData
                        assertNotNull(commitStringData)
                        assertEquals("I", commitStringData.text)
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyUp>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.U, event.keyCode.value)
                    assertEquals(KeySym.u, event.key.value)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.ModifiersChanged>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(emptySet(), event.modifiers)
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.Control_L, event.keyCode.value)
                assertEquals(KeySym.Control_L, event.key.value)
            }

            withKeyPress(KeyCode.Alt_L) {
                withNextEvent { event ->
                    assertInstanceOf<Event.ModifiersChanged>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(setOf(KeyModifiers.Alt), event.modifiers)
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertNull(event.characters)
                    assertEquals(KeyCode.Alt_L, event.keyCode.value)
                    assertEquals(KeySym.Alt_L, event.key.value)
                    assertEquals(setOf(KeyModifiers.Alt), event.modifiers)
                }
                withKeyPress(KeyCode.A) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertNull(event.commitStringData)
                        assertNull(event.deleteSurroundingTextData)
                        val preeditStringData = event.preeditStringData
                        assertNotNull(preeditStringData)
                        assertEquals("a", preeditStringData.text)
                        assertEquals(1, preeditStringData.cursorBytePos)
                        assertContentEquals(
                            arrayOf(
                                TextInputPreeditAttribute(
                                    beginBytePos = 0U,
                                    endBytePos = 1U,
                                    underline = TextInputPreeditUnderlineType.Single,
                                    foregroundHighlight = false,
                                    backgroundHighlight = false,
                                    strikethrough = false,
                                    bold = false,
                                    italic = false,
                                ),
                            ),
                            preeditStringData.attributes.toTypedArray(),
                        )
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyUp>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.A, event.keyCode.value)
                    assertEquals(KeySym.a, event.key.value)
                }
                withKeyPress(KeyCode.M) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertNull(event.commitStringData)
                        assertNull(event.deleteSurroundingTextData)
                        val preeditStringData = event.preeditStringData
                        assertNotNull(preeditStringData)
                        assertEquals("am", preeditStringData.text)
                        assertEquals(2, preeditStringData.cursorBytePos)
                        assertContentEquals(
                            arrayOf(
                                TextInputPreeditAttribute(
                                    beginBytePos = 0U,
                                    endBytePos = 2U,
                                    underline = TextInputPreeditUnderlineType.Single,
                                    foregroundHighlight = false,
                                    backgroundHighlight = false,
                                    strikethrough = false,
                                    bold = false,
                                    italic = false,
                                ),
                            ),
                            preeditStringData.attributes.toTypedArray(),
                        )
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyUp>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.M, event.keyCode.value)
                    assertEquals(KeySym.m, event.key.value)
                }
                withKeyPress(KeyCode.Down) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertNull(event.commitStringData)
                        assertNull(event.deleteSurroundingTextData)
                        val preeditStringData = event.preeditStringData
                        assertNotNull(preeditStringData)
                        assertEquals("first", preeditStringData.text)
                        assertEquals(0, preeditStringData.cursorBytePos)
                        assertContentEquals(
                            arrayOf(
                                TextInputPreeditAttribute(
                                    beginBytePos = 0U,
                                    endBytePos = 5U,
                                    underline = TextInputPreeditUnderlineType.Single,
                                    foregroundHighlight = false,
                                    backgroundHighlight = false,
                                    strikethrough = false,
                                    bold = false,
                                    italic = false,
                                ),
                            ),
                            preeditStringData.attributes.toTypedArray(),
                        )
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyUp>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.Down, event.keyCode.value)
                    assertEquals(KeySym.Down, event.key.value)
                }
                withKeyPress(KeyCode.Down) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertNull(event.commitStringData)
                        assertNull(event.deleteSurroundingTextData)
                        val preeditStringData = event.preeditStringData
                        assertNotNull(preeditStringData)
                        assertEquals("second", preeditStringData.text)
                        assertEquals(0, preeditStringData.cursorBytePos)
                        assertContentEquals(emptyArray(), preeditStringData.attributes.toTypedArray())
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyUp>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.Down, event.keyCode.value)
                    assertEquals(KeySym.Down, event.key.value)
                }
                withKeyPress(KeyCode.Down) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertNull(event.commitStringData)
                        assertNull(event.deleteSurroundingTextData)
                        val preeditStringData = event.preeditStringData
                        assertNotNull(preeditStringData)
                        assertEquals("❌ error, \uD83D\uDD8D highlighted error, ❗important❗", preeditStringData.text)
                        assertEquals(0, preeditStringData.cursorBytePos)
                        assertContentEquals(
                            arrayOf(
                                TextInputPreeditAttribute(
                                    beginBytePos = 0U,
                                    endBytePos = 9U,
                                    underline = TextInputPreeditUnderlineType.Error,
                                    foregroundHighlight = true,
                                    backgroundHighlight = false,
                                    strikethrough = false,
                                    bold = false,
                                    italic = false,
                                ),
                                TextInputPreeditAttribute(
                                    beginBytePos = 9U,
                                    endBytePos = 11U,
                                    underline = TextInputPreeditUnderlineType.Single,
                                    foregroundHighlight = false,
                                    backgroundHighlight = false,
                                    strikethrough = false,
                                    bold = false,
                                    italic = false,
                                ),
                                TextInputPreeditAttribute(
                                    beginBytePos = 11U,
                                    endBytePos = 33U,
                                    underline = TextInputPreeditUnderlineType.Error,
                                    foregroundHighlight = false,
                                    backgroundHighlight = true,
                                    strikethrough = false,
                                    bold = false,
                                    italic = false,
                                ),
                                TextInputPreeditAttribute(
                                    beginBytePos = 33U,
                                    endBytePos = 35U,
                                    underline = TextInputPreeditUnderlineType.Single,
                                    foregroundHighlight = false,
                                    backgroundHighlight = false,
                                    strikethrough = false,
                                    bold = false,
                                    italic = false,
                                ),
                                TextInputPreeditAttribute(
                                    beginBytePos = 35U,
                                    endBytePos = 50U,
                                    underline = TextInputPreeditUnderlineType.Double,
                                    foregroundHighlight = false,
                                    backgroundHighlight = false,
                                    strikethrough = false,
                                    bold = false,
                                    italic = false,
                                ),
                            ),
                            preeditStringData.attributes.toTypedArray(),
                        )
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyUp>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.Down, event.keyCode.value)
                    assertEquals(KeySym.Down, event.key.value)
                }
                withKeyPress(KeyCode.Return) {
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertNotNull(event.commitStringData)
                        assertEquals("❌ error, \uD83D\uDD8D highlighted error, ❗important❗", event.commitStringData.text)
                        assertNull(event.deleteSurroundingTextData)
                        assertNull(event.preeditStringData)
                    }
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        val preeditStringData = event.preeditStringData
                        assertNotNull(preeditStringData)
                        assertEquals("", preeditStringData.text)
                        assertEquals(0, preeditStringData.cursorBytePos)
                        assertContentEquals(arrayOf(), preeditStringData.attributes.toTypedArray())
                        assertNull(event.commitStringData)
                        assertNull(event.deleteSurroundingTextData)
                    }
                    withNextEvent { event ->
                        assertInstanceOf<Event.TextInput>(event)
                        assertEquals(windowParams.windowId, event.windowId)
                        assertNull(event.preeditStringData)
                        assertNull(event.commitStringData)
                        assertNull(event.deleteSurroundingTextData)
                    }
                }
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyUp>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals(KeyCode.Return, event.keyCode.value)
                    assertEquals(KeySym.Return, event.key.value)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.ModifiersChanged>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(emptySet(), event.modifiers)
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(KeyCode.Alt_L, event.keyCode.value)
                assertEquals(KeySym.Alt_L, event.key.value)
            }

            // After `textInputDisable`, KeyDown events should be reported for character keys.

            ui { window.textInputDisable() }

            withKeyPress(KeyCode.A) {
                withNextEvent { event ->
                    assertInstanceOf<Event.KeyDown>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    assertEquals("a", event.characters)
                    assertEquals(KeyCode.A, event.keyCode.value)
                    assertEquals(KeySym.a, event.key.value)
                    assertEquals(emptySet(), event.modifiers)
                }
            }
            withNextEvent { event ->
                assertInstanceOf<Event.KeyUp>(event)
                assertEquals(windowParams.windowId, event.windowId)
            }
        }
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

        TestApp.DropTarget.run { readTestAppOutputLastLine ->
            waitForTestAppFocus(windowParams.windowId, initialWindowData.configure.size)

            // Move the mouse to the left part of the screen
            val mouseY = physicalScreenSize.height / 2
            moveMouseTo((physicalScreenSize.width / 2) - 100, mouseY)

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

                assertTrue(
                    wiggleMouseUntil(physicalScreenSize.width - 100, mouseY) {
                        readTestAppOutputLastLine(10.milliseconds) == "Received DRAG_MOTION event"
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

            assertEquals(textContent.decodeToString(), readTestAppOutputLastLine(5.seconds))

            awaitEventOfType<Event.DragAndDropFeedbackFinished>(timeout = 5.seconds) { event ->
                assertEquals(windowParams.windowId, event.windowId)
                true
            }
        }
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
        val originalWindowSize = createWindowAndWaitForFocus(windowParams).configure.size

        TestApp.DragSource.run { readTestAppOutputLastLine ->
            waitForTestAppFocus(windowParams.windowId, originalWindowSize)

            // Move the mouse to the right part of the screen
            var mouseX = (physicalScreenSize.width / 2) + 100
            val mouseY = physicalScreenSize.height / 2
            moveMouseTo(mouseX, mouseY)
            withMouseButtonDown(MouseButton.LEFT) {
                assertEquals("TestAppDragSource drag begin", readTestAppOutputLastLine(5.seconds))
                mouseX = (physicalScreenSize.width / 2) - 100
                assertTrue(
                    wiggleMouseUntil(mouseX, mouseY) {
                        !queryDragAndDropTargetTriggered.isEmpty()
                    },
                    "queryDragAndDropTarget is triggered (1)",
                )
                for (data in queryDragAndDropTargetTriggered.drainAll()) {
                    assertEquals(windowParams.windowId, data.windowId)
                    assertNotEquals(0f, data.locationInWindow.x)
                    assertNotEquals(0f, data.locationInWindow.y)
                }

                mouseX = (physicalScreenSize.width / 2) + 100
                var dragAndDropLeaveEvent: Event.DragAndDropLeave? = null
                assertTrue(
                    wiggleMouseUntil(mouseX, mouseY) {
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

                mouseX = (physicalScreenSize.width / 2) - 100
                assertTrue(
                    wiggleMouseUntil(mouseX, mouseY) {
                        !queryDragAndDropTargetTriggered.isEmpty()
                    },
                    "queryDragAndDropTarget is triggered (2)",
                )

                ui {}
                for (data in queryDragAndDropTargetTriggered.drainAll()) {
                    assertEquals(windowParams.windowId, data.windowId)
                    assertNotEquals(0f, data.locationInWindow.x)
                    assertNotEquals(0f, data.locationInWindow.y)
                }
            }

            awaitEventOfType<Event.DropPerformed> { event ->
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(DragAndDropAction.Move, event.action)
                assertNotNull(event.content)
                assertEquals(TEXT_UTF8_MIME_TYPE, event.content.mimeType)
                assertContentEquals("Text from TestAppDragSource".encodeToByteArray(), event.content.data)
                true
            }
        }
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
        val window = initialWindowData.window

        TestApp.BlankWindow.run { readTestAppOutputLastLine ->
            waitForTestAppFocus(windowParams.windowId, initialWindowData.configure.size)

            // Move the mouse to the left part of the screen
            val mouseY = physicalScreenSize.height / 2
            moveMouseTo((physicalScreenSize.width / 2) - 100, mouseY)

            awaitEventOfType<Event.MouseEntered> { true }

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

                assertTrue(
                    wiggleMouseUntil(physicalScreenSize.width - 100, mouseY) {
                        readTestAppOutputLastLine(10.milliseconds) == "Received DRAG_MOTION event"
                    },
                    "Test app produced a DRAG_MOTION event",
                )
            }

            awaitEventOfType<Event.DataTransferCancelled>(timeout = 5.seconds) { event ->
                assertEquals(DataSource.DragAndDrop, event.dataSource)
                true
            }
            assertFalse(dataTransferTriggered.isDone)
            awaitEventOfType<Event.DragAndDropFeedbackFinished>(timeout = 5.seconds) { event ->
                assertEquals(windowParams.windowId, event.windowId)
                true
            }
        }
    }

    @Test
    @Ignore($$"Flaky with seed 96788343850500: Timed out waiting for event org.jetbrains.desktop.gtk.Event$MouseExited.")
    fun testDragToSameWindow() {
        val dataTransferTriggered = CompletableFuture<Unit>()
        val dragIconDrawTriggered = LinkedBlockingQueue<Boolean>()
        val queryDragAndDropTargetTriggered = LinkedBlockingQueue<DragAndDropQueryData>()
        val textContent = "test clipboard content".toByteArray()
        val content = mapOf(TEXT_UTF8_MIME_TYPE to textContent)
        val directContextCache = mutableMapOf<Int, SkDirectContext>()

        val makeDirectContext = {
            val glProcFunc = app.getEglProcFunc()
            assertNotNull(glProcFunc)
            val openGlInterface = SkGLAssembledInterface.createFromNativePointers(ctxPtr = glProcFunc.ctxPtr, fPtr = glProcFunc.fPtr)
            SkDirectContext.makeGLWithInterface(openGlInterface)
        }

        val draw: (PhysicalSize, OpenGlDrawData, (Canvas) -> Unit) -> Unit = { size, openGlDrawData, f ->
            SkBackendRenderTarget.makeGL(
                width = size.width,
                height = size.height,
                sampleCnt = 1,
                stencilBits = 0,
                fbId = openGlDrawData.framebuffer,
                fbFormat = SkFramebufferFormat.GR_GL_RGBA8,
            ).use { renderTarget ->
                SkSurface.makeFromBackendRenderTarget(
                    context = directContextCache.getOrPut(openGlDrawData.framebuffer, makeDirectContext),
                    rt = renderTarget,
                    origin = SkSurfaceOrigin.TOP_LEFT,
                    colorFormat = SkSurfaceColorFormat.RGBA_8888,
                    colorSpace = SkColorSpace.sRGB,
                    surfaceProps = null,
                )!!.use { surface ->
                    f(surface.canvas)
                    surface.flushAndSubmit()
                }
            }
        }

        eventHandler = { event ->
            assertTrue(app.isEventLoopThread())
            when (event) {
                is Event.WindowDraw -> {
                    draw(event.size, event.openGlDrawData) {
                        it.clear(SkColor.BLUE)
                    }
                }
                is Event.DragIconDraw -> {
                    draw(event.size, event.openGlDrawData) {
                        it.clear(SkColor.RED)
                    }
                    dragIconDrawTriggered.offer(true)
                }
                else -> {}
            }
            EventHandlerResult.Continue
        }

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
        val window = initialWindowData.window
        val scale = initialWindowData.scale.newScale.toFloat()

        var mouseX = 100
        val mouseY = physicalScreenSize.height / 2
        moveMouseTo(mouseX, mouseY)

        withMouseButtonDown(MouseButton.LEFT) {
            ui {
                window.startDragAndDrop(
                    StartDragAndDropParams(
                        mimeTypes = listOf(TEXT_UTF8_MIME_TYPE),
                        actions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                        dragIconParams = DragIconParams(
                            renderingMode = RenderingMode.GL_ES,
                            size = LogicalSize(100, 50),
                        ),
                    ),
                )
            }
            mouseX += 100
            moveMouseTo(mouseX, mouseY)
            awaitEventOfType<Event.MouseExited> { true }
            waitUntilEq(false) { dragIconDrawTriggered.isEmpty() }

            mouseX += 100
            moveMouseTo(mouseX, mouseY)
            mouseX += 100
            moveMouseTo(mouseX, mouseY)
            waitUntilEq(false) { queryDragAndDropTargetTriggered.isEmpty() }
        }

        dataTransferTriggered.get(1000, TimeUnit.MILLISECONDS)
        awaitEventOfType<Event.DragAndDropFinished> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(DragAndDropAction.Move, event.action)
            true
        }
        awaitEventOfType<Event.DropPerformed> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(DragAndDropAction.Move, event.action)
            assertNotNull(event.content)
            assertEquals(TEXT_UTF8_MIME_TYPE, event.content.mimeType)
            assertContentEquals(textContent, event.content.data)
            assertEquals(
                LogicalPoint(
                    x = mouseX / scale,
                    y = mouseY / scale,
                ),
                event.locationInWindow,
            )
            true
        }

        ui { window.close() }
        awaitEventOfType<Event.WindowClosed> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }
    }

    @Test
    fun testMouseScroll() {
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams()
        val scale = createWindowAndWaitForFocus(windowParams).scale.newScale

        moveMouseTo((100 * scale).roundToInt(), (100 * scale).roundToInt())
        withNextEvent { event ->
            assertInstanceOf<Event.MouseEntered>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(LogicalPoint(100f, 100f), event.locationInWindow)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.MouseMoved>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertNotEquals(Duration.ZERO, event.timestamp.toDuration())
        }

        scrollMouseDown()
        withNextEvent { event ->
            assertInstanceOf<Event.ScrollWheel>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(0f, event.scrollingDeltaX)
            assertEquals(1f, event.scrollingDeltaY)
            assertNotEquals(Duration.ZERO, event.timestamp.toDuration())
        }

        scrollMouseUp()
        withNextEvent { event ->
            assertInstanceOf<Event.ScrollWheel>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(0f, event.scrollingDeltaX)
            assertEquals(-1f, event.scrollingDeltaY)
            assertNotEquals(Duration.ZERO, event.timestamp.toDuration())
        }
    }

    @Test
    fun testWindowRequestDraw() {
        lateinit var window: Window
        val windowIsActive = CompletableFuture<Unit>()
        eventHandler = { event ->
            assertTrue(app.isEventLoopThread())
            if (event is Event.WindowFrameTick && windowIsActive.isDone) {
                assertEquals(window.windowId, event.windowId)
                assertNotEquals(0, event.frameTimeMicroseconds)
                window.requestRedraw()
            }
            EventHandlerResult.Continue
        }
        run(defaultApplicationConfig())
        val windowParams = defaultWindowParams()
        window = createWindowAndWaitForFocus(windowParams).window

        windowIsActive.complete(Unit)

        awaitEventOfType<Event.WindowDraw> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }
        awaitEventOfType<Event.WindowDraw> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }
    }

    @Test
    fun testShowNotificationWithoutNotificationService() {
        val showNotificationParams = ShowNotificationParams(
            title = "Test Notification 1",
            body = "Body of Test Notification 1",
            soundFilePath = null,
        )
        run(defaultApplicationConfig())
        val notification1RequestId = ui { app.requestShowNotification(showNotificationParams) }
        assertNotNull(notification1RequestId)
        withNextEvent { event ->
            assertInstanceOf<Event.NotificationShown>(event)
            assertEquals(notification1RequestId, event.requestId)
            assertNull(event.notificationId)
        }
    }

    private fun checkDunstOutput(dunstOutput: List<String>, showNotificationParams: ShowNotificationParams) {
        assertNotEquals(emptyList(), dunstOutput)
        val checklist = Checklist(listOf("summary", "body"))
        for (line in dunstOutput) {
            val line = line.trim()
            if (line.startsWith("summary:")) {
                checklist.checkEntry("summary")
                assertEquals("summary: '${showNotificationParams.title}'", line)
            } else if (line.startsWith("body:")) {
                checklist.checkEntry("body")
                assertEquals("body: '${showNotificationParams.body}'", line)
            }
        }
        assertEquals(emptySet(), checklist.uncheckedEntries())
    }

    @Test
    fun testActivateNotification() {
        val showNotificationParams = ShowNotificationParams(
            title = "Test Notification 1",
            body = "Body of Test Notification 1",
            soundFilePath = null,
        )
        val dunstOutput = withDunst {
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

            runCommand(listOf("dunstctl", "action"))

            withNextEvent { event ->
                assertInstanceOf<Event.NotificationClosed>(event)
                assertEquals(notification1Id, event.notificationId)
                assertEquals("default", event.action)
            }
        }

        checkDunstOutput(dunstOutput, showNotificationParams)
    }

    @Test
    fun testCloseNotification() {
        val showNotificationParams = ShowNotificationParams(
            title = "Test Notification 1",
            body = "Body of Test Notification 1",
            soundFilePath = null,
        )
        val dunstOutput = withDunst {
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

            runCommand(listOf("dunstctl", "close"))

            withNextEvent { event ->
                assertInstanceOf<Event.NotificationClosed>(event)
                assertEquals(notification1Id, event.notificationId)
                assertNull(event.action)
                assertNull(event.activationToken)
            }
        }

        checkDunstOutput(dunstOutput, showNotificationParams)
    }

    @Test
    fun testCloseNotificationProgrammatically() {
        val showNotificationParams = ShowNotificationParams(
            title = "Test Notification 1",
            body = "Body of Test Notification 1",
            soundFilePath = null,
        )
        val dunstOutput = withDunst {
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

            ui { app.closeNotification(notification1Id) }

            withNextEvent { event ->
                assertInstanceOf<Event.NotificationClosed>(event)
                assertEquals(notification1Id, event.notificationId)
                assertNull(event.action)
                assertNull(event.activationToken)
            }
        }

        checkDunstOutput(dunstOutput, showNotificationParams)
    }

    @Test
    fun testRendering() {
        val backgroundColor = SkColor.BLUE
        val rectColor = SkColor.RED
        val rectSize = LogicalSize(100, 50)
        var scale: Float? = null

        val directContext: SkDirectContext by lazy {
            val glProcFunc = app.getEglProcFunc()
            assertNotNull(glProcFunc)
            val openGlInterface = SkGLAssembledInterface.createFromNativePointers(ctxPtr = glProcFunc.ctxPtr, fPtr = glProcFunc.fPtr)
            SkDirectContext.makeGLWithInterface(openGlInterface)
        }

        val draw: (Event.WindowDraw) -> Unit = { event ->
            SkBackendRenderTarget.makeGL(
                width = event.size.width,
                height = event.size.height,
                sampleCnt = 1,
                stencilBits = 0,
                fbId = event.openGlDrawData.framebuffer,
                fbFormat = SkFramebufferFormat.GR_GL_RGBA8,
            ).use { renderTarget ->
                SkSurface.makeFromBackendRenderTarget(
                    context = directContext,
                    rt = renderTarget,
                    origin = SkSurfaceOrigin.TOP_LEFT,
                    colorFormat = SkSurfaceColorFormat.RGBA_8888,
                    colorSpace = SkColorSpace.sRGB,
                    surfaceProps = null,
                )!!.use { surface ->
                    val canvas = surface.canvas
                    canvas.clear(backgroundColor)
                    SkPaint().use { paint ->
                        paint.color = rectColor
                        log("Draw window: scale=$scale")
                        canvas.drawRect(SkRect.makeXYWH(0f, 0f, rectSize.width * scale!!, rectSize.height * scale!!), paint)
                    }
                    surface.flushAndSubmit()
                }
            }
        }

        eventHandler = { event ->
            assertTrue(app.isEventLoopThread())
            if (event is Event.WindowDraw) {
                draw(event)
            }
            EventHandlerResult.Continue
        }
        run(defaultApplicationConfig())

        val windowParams = defaultWindowParams()
        val initialWindowData = createWindowAndWaitForFocus(windowParams, onScale = { drawScale ->
            scale = drawScale.toFloat()
        })

        assertNotNull(scale)

        val tempDir = Files.createTempDirectory("test_linux_rendering")
        val screenshotPath = tempDir.resolve("1.png")

        ui {}
        // Screenshot only the window, because sometimes the previous tests can leave the drag&drop artifacts
        screenshot(screenshotPath, windowId = getActiveWindowNumber())

        val image = SkImage.makeFromEncoded(screenshotPath.readBytes())
        assertEquals(initialWindowData.draw.size.width, image.width)
        assertEquals(initialWindowData.draw.size.height, image.height)
        val bitmap = SkBitmap.makeFromImage(image, directContext)
        assertEquals(initialWindowData.draw.size.width, bitmap.width)
        assertEquals(initialWindowData.draw.size.height, bitmap.height)

        val path = screenshotPath.absolutePathString()

        val bottomRightRectColor = bitmap.getColor(
            ((rectSize.width - 1) * scale).roundToInt(),
            ((rectSize.height - 1) * scale).roundToInt(),
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
    }
}

@EnabledOnOs(OS.LINUX)
@Execution(ExecutionMode.SAME_THREAD)
class CompositedX11Tests : X11TestsBase() {
    // Has to be a separate test, in a separate class (together with a Gradle configuration to run each test class in a separate process)
    // due to a bug in GTK older than 4.22, which is not even in the newest, at the time of writing, Ubuntu version (25.10):
    // https://github.com/GNOME/gtk/commit/ccf3908376fa1b5c02cb3d4d558e307938e65561
    @Test
    fun testIsCompositing() {
        val process = ProcessBuilder("picom", "--backend", "xrender", "--log-level", "DEBUG").start()
        AutoCloseable {
            assertTrue(process.isAlive)
            process.destroy()
            process.waitFor()
        }.use {
            assertTrue(process.isAlive)
            val startTime = TimeSource.Monotonic.markNow()
            while (startTime.elapsedNow() < 10.seconds) {
                val line = process.errorReader().readLine()
                if (line.contains("Screen redirected.")) {
                    runWithoutWaitingForStart(defaultApplicationConfig())
                    awaitEventOfType<Event.DesktopSettingChange> {
                        it.setting == DesktopSetting.IsComposited(true)
                    }
                    assertNotNull(awaitEvent { it == Event.ApplicationStarted })
                    return
                }
            }
            fail(withTimestamp("Timed out waiting for picom to start"))
        }
    }
}
