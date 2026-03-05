package org.jetbrains.desktop.linux.tests

import org.jetbrains.desktop.gtk.Application
import org.jetbrains.desktop.gtk.ApplicationConfig
import org.jetbrains.desktop.gtk.DataSource
import org.jetbrains.desktop.gtk.DragAndDropAction
import org.jetbrains.desktop.gtk.DragAndDropQueryData
import org.jetbrains.desktop.gtk.DragAndDropQueryResponse
import org.jetbrains.desktop.gtk.Event
import org.jetbrains.desktop.gtk.EventHandlerResult
import org.jetbrains.desktop.gtk.KeyCode
import org.jetbrains.desktop.gtk.KeyModifiers
import org.jetbrains.desktop.gtk.KeySym
import org.jetbrains.desktop.gtk.KotlinDesktopToolkit
import org.jetbrains.desktop.gtk.LogLevel
import org.jetbrains.desktop.gtk.LogicalPoint
import org.jetbrains.desktop.gtk.LogicalRect
import org.jetbrains.desktop.gtk.LogicalSize
import org.jetbrains.desktop.gtk.MouseButton
import org.jetbrains.desktop.gtk.ShowNotificationParams
import org.jetbrains.desktop.gtk.StartDragAndDropParams
import org.jetbrains.desktop.gtk.SupportedActionsForMime
import org.jetbrains.desktop.gtk.TextInputContentPurpose
import org.jetbrains.desktop.gtk.TextInputContext
import org.jetbrains.desktop.gtk.TextInputContextHint
import org.jetbrains.desktop.gtk.Window
import org.jetbrains.desktop.gtk.WindowCapabilities
import org.jetbrains.desktop.gtk.WindowDecorationMode
import org.jetbrains.desktop.gtk.WindowId
import org.jetbrains.desktop.gtk.WindowParams
import org.jetbrains.desktop.gtk.WindowResizeEdge
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.Timeout
import org.junit.jupiter.api.assertInstanceOf
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import java.io.File
import java.util.concurrent.CompletableFuture
import java.util.concurrent.Future
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread
import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.test.fail
import kotlin.time.Duration

private fun runCommandWithOutput(command: List<String>, timeoutMs: Long = 1000): ByteArray {
    val pb = ProcessBuilder(command)
        .redirectOutput(ProcessBuilder.Redirect.PIPE)
        .redirectError(ProcessBuilder.Redirect.INHERIT)

    val proc = pb.start()
    assertTrue(proc.waitFor(timeoutMs, TimeUnit.MILLISECONDS), command.toString())
    return proc.inputStream.readAllBytes()
}

private fun runCommand(command: List<String>, timeoutMs: Long = 1000) {
    val pb = ProcessBuilder(command)
        .redirectOutput(ProcessBuilder.Redirect.INHERIT)
        .redirectError(ProcessBuilder.Redirect.INHERIT)

    val proc = pb.start()
    assertTrue(proc.waitFor(timeoutMs, TimeUnit.MILLISECONDS), command.toString())
}

private fun waitUntil(timeoutMs: Long = 10000, block: () -> Boolean) {
    var waitDurationMs = 0L
    val waitStepMs = 10L
    while (!block() && waitDurationMs < timeoutMs) {
        waitDurationMs += waitStepMs
        Thread.sleep(waitStepMs)
    }
}

private fun withTestApp(appSource: String, block: () -> Unit) {
    val outputFile = File.createTempFile("test_app_output", "log")
    outputFile.deleteOnExit()
    val testApp = ProcessBuilder("python3", "-c", appSource)
        .redirectOutput(ProcessBuilder.Redirect.to(outputFile))
        .redirectError(ProcessBuilder.Redirect.INHERIT)
        .start()
    AutoCloseable {
        testApp.destroy()
        testApp.waitFor()
        outputFile.delete()
    }.use {
        waitUntil { !testApp.isAlive || outputFile.length() > 0 }
        assertTrue(testApp.isAlive)

        val line = outputFile.readText()
        assertEquals("ready\n", line)
        block()
    }
}

private fun withDunst(block: () -> Unit): List<String> {
    val outputFile = File.createTempFile("test_dunst_output", "log")
    var output = emptyList<String>()
    outputFile.deleteOnExit()
    val proc = ProcessBuilder("dunst", "-print")
        .redirectOutput(ProcessBuilder.Redirect.to(outputFile))
        .redirectError(ProcessBuilder.Redirect.INHERIT)
        .start()
    AutoCloseable {
        proc.destroy()
        proc.waitFor()
        output = outputFile.readLines()
        outputFile.delete()
    }.use {
        assertTrue(proc.isAlive)
        block()
    }
    return output
}

private fun <T> withKeyPress(key: String, block: () -> T): T {
    runCommand(listOf("xdotool", "keydown", key))

    AutoCloseable {
        runCommand(listOf("xdotool", "keyup", key))
    }.use {
        return block()
    }
}

private val fakeX11InputPythonApp: String by lazy { object {}.javaClass.getResource("/test_app_x11_fake_input.py")!!.readText() }
private fun withRawKeyPress(key: UInt, block: () -> Unit) {
    println("KeyPress $key")
    runCommand(listOf("python3", "-c", fakeX11InputPythonApp, "--operation", "KeyPress", "--value", key.toString()))
    AutoCloseable {
        println("KeyRelease $key")
        runCommand(listOf("python3", "-c", fakeX11InputPythonApp, "--operation", "KeyRelease", "--value", key.toString()))
    }.use {
        block()
    }
}

private fun <T> withMouseButtonDown(button: MouseButton, block: () -> T): T {
    runCommand(listOf("xdotool", "mousedown", button.value.toString()))

    AutoCloseable {
        runCommand(listOf("xdotool", "mouseup", button.value.toString()))
    }.use {
        return block()
    }
}

private fun getClipboardContent(format: String): ByteArray {
    return runCommandWithOutput(listOf("xclip", "-selection", "clipboard", "-o", "-t", format))
}

private fun getPrimarySelectionContent(format: String): ByteArray {
    return runCommandWithOutput(listOf("xclip", "-selection", "primary", "-o", "-t", format))
}

private fun moveMouseTo(x: Int, y: Int) {
    runCommand(listOf("xdotool", "mousemove", x.toString(), y.toString()))
}

private fun scrollMouseUp() {
    // https://askubuntu.com/a/1162351
    runCommand(listOf("xdotool", "click", "4"))
}

private fun scrollMouseDown() {
    // https://askubuntu.com/a/1162351
    runCommand(listOf("xdotool", "click", "5"))
}

class SingleThreadTaskQueue {
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

class Checklist(entries: List<String>) {
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

@EnabledOnOs(OS.LINUX)
@Execution(ExecutionMode.SAME_THREAD)
class X11Tests {
    companion object {
        const val APP_ID = "org.jetbrains.desktop.linux.tests"
        const val TEXT_UTF8_MIME_TYPE = "text/plain;charset=utf-8"
        const val URI_LIST_MIME_TYPE = "text/uri-list"
        const val HTML_TEXT_MIME_TYPE = "text/html"
        const val PNG_MIME_TYPE = "image/png"

        val testPythonAppClipboardSource: String by lazy { object {}.javaClass.getResource("/test_app_clipboard_source.py")!!.readText() }
        val testPythonAppDragSource: String by lazy { object {}.javaClass.getResource("/test_app_drag_source.py")!!.readText() }
        val testPythonAppDragTarget: String by lazy { object {}.javaClass.getResource("/test_app_drag_target.py")!!.readText() }
        val testPythonAppPrimarySelectionSource: String by lazy {
            object {}.javaClass.getResource("/test_app_primary_selection_source.py")!!.readText()
        }

        val expectedScreenSize = LogicalSize(2000, 1000)
        private val appExecutor = SingleThreadTaskQueue()

        @BeforeAll
        @JvmStatic
        fun initAll() {
            KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Debug)
        }

        @AfterAll
        @JvmStatic
        fun tearDownAll() {
            println("tearDownAll start")
            appExecutor.exit()

            println("tearDownAll end")
        }
    }

    private val app by lazy { Application() }
    private lateinit var appExecutingResult: Future<Error?>
    private val eventQueue by lazy { LinkedBlockingQueue<Event>() }

    fun defaultApplicationConfig(
        appId: String = APP_ID,
        queryDragAndDropTarget: (DragAndDropQueryData) -> DragAndDropQueryResponse = { _ -> DragAndDropQueryResponse(emptyList()) },
        getDataTransferData: (DataSource, String) -> ByteArray? = { _, _ -> null },
        windowCloseRequest: (WindowId) -> Boolean = { true },
        applicationWantsToTerminate: () -> Boolean = { true },
    ): ApplicationConfig {
        return ApplicationConfig(
            appId = appId,
            eventHandler = { event ->
                if (event !is Event.ShouldRedraw) {
                    println("Event: $event")
                    assert(eventQueue.offer(event)) { "Event queue overflow" }
                }
                eventHandler?.invoke(event) ?: EventHandlerResult.Continue
                EventHandlerResult.Stop
            },
            queryDragAndDropTarget = queryDragAndDropTarget,
            getDataTransferData = getDataTransferData,
            windowCloseRequest = windowCloseRequest,
            applicationWantsToTerminate = applicationWantsToTerminate,
        )
    }

    fun run(applicationConfig: ApplicationConfig) {
        // Reset the mouse position
        moveMouseTo(50, 50)
        appExecutingResult = appExecutor.add {
            try {
                app.runEventLoop(applicationConfig)
            } finally {
                app.stopEventLoop()
                app.close()
            }
        }
        assertNotNull(awaitEvent { it == Event.ApplicationStarted })
    }

    @AfterTest
    @Timeout(value = 20, unit = TimeUnit.SECONDS)
    fun tearDown() {
        println("tearDown start")
        if (!appExecutingResult.isDone) {
            ui { app.stopEventLoop() }
        }
        assertNull(appExecutingResult.get())
        println("tearDown end")
    }

    fun getNextEvent(timeoutMs: Long = 1000): Event? {
        while (true) {
            val event = eventQueue.poll(timeoutMs, TimeUnit.MILLISECONDS)
            if (event == null) {
                println("Timed out waiting for event")
                return null
            }
            return event
        }
    }

    fun <T> withNextEvent(timeoutMs: Long = 1000, block: (Event?) -> T): T {
        return block(getNextEvent(timeoutMs))
    }

    fun awaitEvent(timeoutMs: Long = 1000, predicate: (Event?) -> Boolean) {
        while (true) {
            val event = eventQueue.poll(timeoutMs, TimeUnit.MILLISECONDS)
            if (predicate(event)) return
        }
    }

    inline fun <reified T : Event> awaitEventOfType(timeoutMs: Long = 1000, crossinline predicate: (T) -> Boolean) {
        return awaitEvent(timeoutMs) {
            if (it == null) {
                fail("Timed out waiting for event ${T::class}")
            } else {
                it is T && predicate(it)
            }
        }
    }

    fun <T> ui(timeoutMs: Long = 1000, body: () -> T): T {
        val future = CompletableFuture<T>()
        app.runOnEventLoopAsync {
            val ret = body()
            future.complete(ret)
        }
        return future.get(timeoutMs, TimeUnit.MILLISECONDS)
    }

    private fun withSetClipboardContent(mimeTypes: List<String>, block: () -> Unit) {
        ui { app.clipboardPut(mimeTypes) }
        AutoCloseable {
            ui { app.clipboardPut(emptyList()) }
            // Ensure that `DataTransferAvailable` events are sent.
            ui {}
        }.use {
            block()
        }
    }

    private fun withSetPrimarySelectionContent(mimeTypes: List<String>, block: () -> Unit) {
        ui { app.primarySelectionPut(mimeTypes) }
        AutoCloseable {
            ui { app.primarySelectionPut(emptyList()) }
            // Ensure that `DataTransferAvailable` events are sent.
            ui {}
        }.use {
            block()
        }
    }

    @Volatile
    var eventHandler: ((Event) -> EventHandlerResult)? = null

    @Test
    fun testIsEventLoopThread() {
        run(defaultApplicationConfig())
        assertFalse(app.isEventLoopThread())
        ui { assertTrue(app.isEventLoopThread()) }
    }

    @Test
    fun exitWithoutClosingWindow() {
        run(defaultApplicationConfig())
        val windowParams = WindowParams(
            windowId = 0,
            title = "Test Window 1",
            size = LogicalSize(width = 200, height = 300),
            minSize = null,
        )
        val window = ui { app.createWindow(windowParams) }

        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }
        assertNotNull(window)
    }

    @Test
    fun windowCloseRequestReturningFalse() {
        run(
            defaultApplicationConfig(
                windowCloseRequest = { false },
            ),
        )
        val windowParams = WindowParams(
            windowId = 0,
            title = "Test Window 1",
            size = LogicalSize(width = 200, height = 300),
            minSize = null,
        )
        val window = ui { app.createWindow(windowParams) }
        assertNotNull(window)

        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            event.active
        }
        awaitEventOfType<Event.WindowDraw> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }

        runCommand(listOf("i3-msg", "kill"))

        withKeyPress("a") {
            withNextEvent { event ->
                assertInstanceOf<Event.KeyDown>(event)
                assertEquals(windowParams.windowId, event.windowId)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.MouseEntered>(event)
            assertEquals(windowParams.windowId, event.windowId)
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
        var encounteredWindowClosed = false
        withNextEvent { event ->
            if (event is Event.WindowClosed) {
                assertEquals(windowParams.windowId, event.windowId)
                encounteredWindowClosed = true
            } else {
                assertInstanceOf<Event.WindowKeyboardLeave>(event)
                assertEquals(windowParams.windowId, event.windowId)
            }
        }
        withKeyPress("a") {}
        if (!encounteredWindowClosed) {
            withNextEvent { event ->
                assertInstanceOf<Event.WindowClosed>(event)
                assertEquals(windowParams.windowId, event.windowId)
            }
        }
        assertTrue(eventQueue.isEmpty())
    }

    @Test
    fun testSingleWindowCreation() {
        run(defaultApplicationConfig())
        assertTrue(eventQueue.isEmpty())

        val screens = ui { app.allScreens() }.screens
        val screen = screens.firstOrNull()
        assertNotNull(screen)
        assertEquals(expectedScreenSize, screen.size)
        assertEquals(LogicalPoint(0f, 0f), screen.origin)
        assertEquals(1.0, screen.scale)
        assertNotNull(screen.name)
        assertNotEquals(0U, screen.screenId)

        val windowParams = WindowParams(
            windowId = 1,
            title = "Test Window 2",
            size = LogicalSize(width = 200, height = 300),
            minSize = LogicalSize(width = 100, height = 70),
        )
        val window = ui { app.createWindow(windowParams) }
        assertNotNull(window)

        withNextEvent { event ->
            assertInstanceOf<Event.WindowScaleChanged>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.WindowScreenChange>(event)
            assertEquals(windowParams.windowId, event.windowId)
            assertEquals(screen.screenId, event.newScreenId)
        }

        var expectedConfigureEvent = Event.WindowConfigure(
            windowId = windowParams.windowId,
            size = expectedScreenSize,
            active = true,
            maximized = true,
            fullscreen = false,
            decorationMode = WindowDecorationMode.Server,
            capabilities = WindowCapabilities(
                windowMenu = true,
                maximize = true,
                fullscreen = true,
                minimize = true,
            ),
        )
        awaitEvent { event ->
            if (event is Event.WindowScreenChange) {
                false
            } else {
                assertInstanceOf<Event.WindowKeyboardEnter>(event)
                assertEquals(windowParams.windowId, event.windowId)
                true
            }
        }

        // Means that it's a newer i3 version, with a better behavior.
        // But we also test with the old one on the CI, so the logic needs to work for both.
        var reportsMaximized = true
        expectedConfigureEvent = expectedConfigureEvent.copy(active = true)
        withNextEvent { event ->
            assertInstanceOf<Event.WindowConfigure>(event)
            expectedConfigureEvent = expectedConfigureEvent.copy(maximized = event.maximized)
            reportsMaximized = event.maximized
            assertEquals(expectedConfigureEvent, event)
        }

        withNextEvent { event ->
            assertInstanceOf<Event.WindowDraw>(event)
            assertEquals(windowParams.windowId, event.windowId)
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

        moveMouseTo(expectedScreenSize.width - 50, expectedScreenSize.height - 50)
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
                            expectedConfigureEvent = expectedConfigureEvent.copy(size = expectedScreenSize)
                            assertEquals(expectedConfigureEvent, event, failMsg())
                            moveMouseTo(expectedScreenSize.width - 51, expectedScreenSize.height - 51)
                        } else {
                            fail("Unexpected event: $event, ${failMsg()}")
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
                        fail("Unexpected event: $event, ${failMsg()}")
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
                            moveMouseTo(expectedScreenSize.width - 52, expectedScreenSize.height - 52)
                        } else {
                            fail("Unexpected event: $event, ${failMsg()}")
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
                        fail("Unexpected event: $event, ${failMsg()}")
                    }
                }
            }
        }

        ui { window.close() }
        if (reportsMaximized) {
            withNextEvent { event ->
                assertInstanceOf<Event.WindowKeyboardLeave>(event)
                assertEquals(windowParams.windowId, event.windowId)
            }
        }
        withKeyPress("a") {}
        withNextEvent { event ->
            assertInstanceOf<Event.WindowClosed>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }
        assertTrue(eventQueue.isEmpty())
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
                            println("Unexpected data source: $dataSource")
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

        withTestApp(testPythonAppClipboardSource) {
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
            assertTrue(ui { app.clipboardPaste(transferSerial1, listOf(TEXT_UTF8_MIME_TYPE, PNG_MIME_TYPE)) })
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
            assertTrue(ui { app.clipboardPaste(transferSerial2, listOf(URI_LIST_MIME_TYPE, TEXT_UTF8_MIME_TYPE)) })
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

        withTestApp(testPythonAppPrimarySelectionSource) {
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
            assertTrue(ui { app.primarySelectionPaste(transferSerial, listOf(TEXT_UTF8_MIME_TYPE, PNG_MIME_TYPE)) })
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
        ui { app.clipboardPaste(clipboardPasteSerial, listOf(TEXT_UTF8_MIME_TYPE)) }.also {
            assertTrue(it)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.DataTransfer>(event)
            assertEquals(clipboardPasteSerial, event.serial)
            assertNull(event.content)
        }

        val primarySelectionPasteSerial = 6
        ui { app.primarySelectionPaste(primarySelectionPasteSerial, listOf(TEXT_UTF8_MIME_TYPE)) }.also {
            assertTrue(it)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.DataTransfer>(event)
            assertEquals(primarySelectionPasteSerial, event.serial)
            assertNull(event.content)
        }
    }

    @Test
    fun testKeyEvents() {
        run(defaultApplicationConfig())
        val windowParams = WindowParams(
            windowId = 0,
            title = "Test Window 1",
            size = LogicalSize(width = 200, height = 300),
            minSize = null,
        )
        val window = ui { app.createWindow(windowParams) }
        assertNotNull(window)

        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            event.active
        }
        awaitEventOfType<Event.WindowDraw> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }

        moveMouseTo(100, 100)
        withNextEvent { event ->
            assertInstanceOf<Event.MouseEntered>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.MouseMoved>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }

        withKeyPress("a") {
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

        withKeyPress("Return") {
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

        withKeyPress("Escape") {
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

        withKeyPress("BackSpace") {
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

        withKeyPress("Tab") {
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

        withKeyPress("shift") {
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
            withKeyPress("a") {
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

        withKeyPress("ctrl") {
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
            withKeyPress("a") {
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

    @Test
    fun testIme() {
        run(defaultApplicationConfig())
        val windowParams = WindowParams(
            windowId = 0,
            title = "Test Window 1",
            size = LogicalSize(width = 200, height = 300),
            minSize = null,
        )
        var window: Window? = null

        ui {
            window = app.createWindow(windowParams)
            window.textInputEnable(
                TextInputContext(
                    surroundingText = "existing ",
                    cursorCodepointOffset = 9U,
                    selectionStartCodepointOffset = 4U,
                    hints = setOf(
                        TextInputContextHint.WordCompletion,
                        TextInputContextHint.Spellcheck,
                        TextInputContextHint.Lowercase,
                        TextInputContextHint.UppercaseChars,
                        TextInputContextHint.UppercaseWords,
                        TextInputContextHint.UppercaseSentences,
                    ),
                    contentPurpose = TextInputContentPurpose.Normal,
                    cursorRectangle = LogicalRect(
                        x = 100,
                        y = 100,
                        width = 10,
                        height = 20,
                    ),
                ),
            )
        }
        assertNotNull(window)

        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            event.active
        }
        awaitEventOfType<Event.WindowDraw> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }

        moveMouseTo(100, 100)
        withNextEvent { event ->
            assertInstanceOf<Event.MouseEntered>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }
        withNextEvent { event ->
            assertInstanceOf<Event.MouseMoved>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }

        // This is a dead key in us(intl) layout, which is set by default for tests.
        withRawKeyPress(KeyCode.grave) {
            withNextEvent { event ->
                assertInstanceOf<Event.TextInput>(event)
                assertEquals(windowParams.windowId, event.windowId)
                val preeditStringData = event.preeditStringData
                assertNotNull(preeditStringData)
                if (preeditStringData.text != "`") {
                    assertEquals("", preeditStringData.text) // Without a proper IME server running, an empty string is reported here.
                    assertEquals(0, preeditStringData.cursorBeginBytePos)
                    assertEquals(0, preeditStringData.cursorEndBytePos)
                } else {
                    assertEquals(1, preeditStringData.cursorBeginBytePos)
                    assertEquals(1, preeditStringData.cursorEndBytePos)
                }
                assertNull(event.commitStringData)
                assertNull(event.deleteSurroundingTextData)
            }
        }

        withRawKeyPress(KeyCode.E) {
            withNextEvent { event ->
                assertInstanceOf<Event.TextInput>(event)
                assertEquals(windowParams.windowId, event.windowId)
                val preeditStringData = event.preeditStringData
                assertNotNull(preeditStringData)
                if (preeditStringData.text != "`") {
                    assertEquals("", preeditStringData.text) // Without a proper IME server running, an empty string is reported here.
                    assertEquals(0, preeditStringData.cursorBeginBytePos)
                    assertEquals(0, preeditStringData.cursorEndBytePos)
                } else {
                    assertEquals(1, preeditStringData.cursorBeginBytePos)
                    assertEquals(1, preeditStringData.cursorEndBytePos)
                }
                assertEquals(0, preeditStringData.cursorBeginBytePos)
                assertEquals(0, preeditStringData.cursorEndBytePos)
                assertNull(event.commitStringData)
                assertNull(event.deleteSurroundingTextData)
            }
            withNextEvent { event ->
                assertInstanceOf<Event.TextInput>(event)
                assertEquals(windowParams.windowId, event.windowId)
                val commitStringData = event.commitStringData
                assertNotNull(commitStringData)
                assertEquals("è", commitStringData.text)
                assertNull(event.preeditStringData)
                assertNull(event.deleteSurroundingTextData)
            }
        }
        withNextEvent { event ->
            assertInstanceOf<Event.KeyUp>(event)
            assertEquals(windowParams.windowId, event.windowId)
        }

        for (c in (('a'..'z') + ('0'..'9')).map { it.toString() }) {
            withKeyPress(c) {
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

        withKeyPress("shift") {
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
            withKeyPress("a") {
                withNextEvent { event ->
                    assertInstanceOf<Event.TextInput>(event)
                    assertEquals(windowParams.windowId, event.windowId)
                    val commitStringData = event.commitStringData
                    assertNotNull(commitStringData)
                    assertEquals(commitStringData.text, "A")
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

        withKeyPress("Return") {
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

        withKeyPress("Escape") {
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

        withKeyPress("BackSpace") {
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

        withKeyPress("Tab") {
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

        withKeyPress("Down") {
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

        withKeyPress("Right") {
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

        withKeyPress("ctrl") {
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
            withKeyPress("a") {
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
            assertEquals(KeySym.Control_L, event.key.value)
        }

        // After `textInputDisable`, KeyDown events should be reported for character keys.

        window.textInputDisable()

        withKeyPress("a") {
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

    fun implTestWindowMove(button: MouseButton) {
        run(defaultApplicationConfig())

        val windowParams = WindowParams(
            windowId = 0,
            title = "Test Window 1",
            size = LogicalSize(width = 200, height = 300),
            minSize = null,
        )
        val window = ui { app.createWindow(windowParams) }

        awaitEventOfType<Event.WindowConfigure> { it.active }

        runCommand(listOf("i3-msg", "floating enable, move position 0 0"))

        awaitEventOfType<Event.WindowConfigure> { event ->
            event.active && event.size == windowParams.size
        }

        val activeWindow = runCommandWithOutput(listOf("xdotool", "getactivewindow")).decodeToString().trim()

        val windowGeometryBefore =
            runCommandWithOutput(listOf("xdotool", "getactivewindow", "getwindowgeometry", "--shell")).decodeToString()
        assertEquals(
            """WINDOW=$activeWindow
X=0
Y=0
WIDTH=200
HEIGHT=300
SCREEN=0
""",
            windowGeometryBefore,
        )

        val mouseX = 50
        val mouseY = 50
        moveMouseTo(mouseX, mouseY)
        val windowGeometryAfter = withMouseButtonDown(button) {
            awaitEventOfType<Event.MouseDown> { true }
            ui { window.startMove() }
            moveMouseTo(mouseX + 100, mouseY + 100)
            runCommandWithOutput(listOf("xdotool", "getactivewindow", "getwindowgeometry", "--shell")).decodeToString()
        }
//        awaitEventOfType<Event.MouseUp> { true }

        assertEquals(
            """WINDOW=$activeWindow
X=100
Y=100
WIDTH=200
HEIGHT=300
SCREEN=0
""",
            windowGeometryAfter,
        )
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

    fun implTestWindowResize(button: MouseButton) {
        run(defaultApplicationConfig())

        val windowParams = WindowParams(
            windowId = 0,
            title = "Test Window 1",
            size = LogicalSize(width = 200, height = 300),
            minSize = null,
        )
        val window = ui { app.createWindow(windowParams) }

        awaitEventOfType<Event.WindowConfigure> { it.active }

        runCommand(listOf("i3-msg", "floating enable, move position 0 0"))

        awaitEventOfType<Event.WindowConfigure> { event ->
            event.active && event.size == windowParams.size
        }

        val activeWindow = runCommandWithOutput(listOf("xdotool", "getactivewindow")).decodeToString().trim()

        val windowGeometryBefore =
            runCommandWithOutput(listOf("xdotool", "getactivewindow", "getwindowgeometry", "--shell")).decodeToString()
        assertEquals(
            """WINDOW=$activeWindow
X=0
Y=0
WIDTH=200
HEIGHT=300
SCREEN=0
""",
            windowGeometryBefore,
        )

        // Move the mouse to the top-left part of the window
        val mouseX = 50
        val mouseY = 50
        moveMouseTo(mouseX, mouseY)
        val windowGeometryAfter = withMouseButtonDown(button) {
            awaitEventOfType<Event.MouseDown> { true }
            // With i3, it doesn't matter which edge we specify; it's dependent on the mouse position
            ui { window.startResize(WindowResizeEdge.TopLeft) }
            moveMouseTo(mouseX + 100, mouseY + 50)
            runCommandWithOutput(listOf("xdotool", "getactivewindow", "getwindowgeometry", "--shell")).decodeToString()
        }
//        awaitEventOfType<Event.MouseUp> { true }
        assertEquals(
            """WINDOW=$activeWindow
X=100
Y=50
WIDTH=100
HEIGHT=250
SCREEN=0
""",
            windowGeometryAfter,
        )
    }

    @Test
    fun testWindowResizeLeftButton() {
        implTestWindowResize(MouseButton.LEFT)
    }

    @Test
    fun testWindowResizeRightButton() {
        implTestWindowResize(MouseButton.RIGHT)
    }

    @Test
    fun testWindowResizeMiddleButton() {
        implTestWindowResize(MouseButton.MIDDLE)
    }

    @Test
    fun testDragFromWindow() {
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

        val windowParams = WindowParams(
            windowId = 0,
            title = "Test Window 1",
            size = LogicalSize(width = 200, height = 300),
            minSize = null,
        )
        val window = ui { app.createWindow(windowParams) }

        awaitEventOfType<Event.WindowConfigure> { it.active }

        withTestApp(testPythonAppDragTarget) {
            awaitEventOfType<Event.WindowConfigure> { !it.active && it.size != expectedScreenSize }

            // Move the mouse to the left part of the screen
            var mouseX = (expectedScreenSize.width / 2) - 100
            val mouseY = expectedScreenSize.height / 2
            moveMouseTo(mouseX, mouseY)

            withMouseButtonDown(MouseButton.LEFT) {
                awaitEventOfType<Event.MouseDown> { true }
                awaitEventOfType<Event.WindowConfigure> { it.active }
                ui {
                    window.startDragAndDrop(
                        StartDragAndDropParams(
                            mimeTypes = listOf(TEXT_UTF8_MIME_TYPE),
                            actions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                            dragIconParams = null,
                        ),
                    )
                }

                mouseX += 10
                while (mouseX < expectedScreenSize.width - 100) {
                    moveMouseTo(mouseX, mouseY)
                    mouseX += 10
                }
            }

            dataTransferTriggered.get(1000, TimeUnit.MILLISECONDS)
            awaitEventOfType<Event.DragAndDropFinished> { event ->
                assertEquals(windowParams.windowId, event.windowId)
                assertEquals(DragAndDropAction.Copy, event.action)
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
                    println("queryDragAndDropTarget")
                    DragAndDropQueryResponse(
                        supportedActionsPerMime = listOf(
                            SupportedActionsForMime(
                                supportedMimeType = TEXT_UTF8_MIME_TYPE,
                                supportedActions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                                preferredAction = DragAndDropAction.Move,
                            ),
                            SupportedActionsForMime(
                                supportedMimeType = HTML_TEXT_MIME_TYPE,
                                supportedActions = setOf(DragAndDropAction.Copy),
                                preferredAction = DragAndDropAction.Copy,
                            ),
                        ),
                    )
                },
            ),
        )

        val windowParams = WindowParams(
            windowId = 0,
            title = "Test Window 1",
            size = LogicalSize(width = 200, height = 300),
            minSize = null,
        )
        ui { app.createWindow(windowParams) }

        awaitEventOfType<Event.WindowConfigure> { it.active }

        withTestApp(testPythonAppDragSource) {
            awaitEventOfType<Event.WindowConfigure> { !it.active && it.size != expectedScreenSize }

            // Move the mouse to the right part of the screen
            var mouseX = (expectedScreenSize.width / 2) + 200
            val mouseY = expectedScreenSize.height / 2
            moveMouseTo(mouseX, mouseY)
            withMouseButtonDown(MouseButton.LEFT) {
                mouseX -= 10
                while (mouseX > 0) {
                    moveMouseTo(mouseX, mouseY)
                    if (!queryDragAndDropTargetTriggered.isEmpty()) {
                        break
                    }
                    mouseX -= 10
                }
                assertFalse(queryDragAndDropTargetTriggered.isEmpty())
                for (data in queryDragAndDropTargetTriggered) {
                    assertEquals(windowParams.windowId, data.windowId)
                    assertNotEquals(0f, data.locationInWindow.x)
                    assertNotEquals(0f, data.locationInWindow.y)
                }

                mouseX += 10
                var dragAndDropLeaveEvent: Event.DragAndDropLeave? = null
                mousemove@ while (mouseX < expectedScreenSize.width) {
                    moveMouseTo(mouseX, mouseY)
                    var lastEvent = eventQueue.poll()
                    while (lastEvent != null) {
                        if (lastEvent is Event.DragAndDropLeave) {
                            dragAndDropLeaveEvent = lastEvent
                            break@mousemove
                        }
                        lastEvent = eventQueue.poll()
                    }
                    mouseX += 10
                }
                assertNotNull(dragAndDropLeaveEvent)
                assertEquals(windowParams.windowId, dragAndDropLeaveEvent.windowId)

                queryDragAndDropTargetTriggered.clear()

                mouseX -= 10
                while (mouseX > 0) {
                    moveMouseTo(mouseX, mouseY)
                    if (!queryDragAndDropTargetTriggered.isEmpty()) {
                        break
                    }
                    mouseX -= 10
                }
                assertFalse(queryDragAndDropTargetTriggered.isEmpty())
                for (data in queryDragAndDropTargetTriggered) {
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

        val windowParams = WindowParams(
            windowId = 0,
            title = "Test Window 1",
            size = LogicalSize(width = 200, height = 300),
            minSize = null,
        )
        val window = ui { app.createWindow(windowParams) }

        awaitEventOfType<Event.WindowConfigure> { it.active }

        withTestApp(testPythonAppDragSource) {
            awaitEventOfType<Event.WindowConfigure> { !it.active && it.size != expectedScreenSize }

            // Move the mouse to the left part of the screen
            var mouseX = (expectedScreenSize.width / 2) - 100
            val mouseY = expectedScreenSize.height / 2
            moveMouseTo(mouseX, mouseY)

            withMouseButtonDown(MouseButton.LEFT) {
                awaitEventOfType<Event.MouseDown> { true }
                awaitEventOfType<Event.WindowConfigure> { it.active }
                ui {
                    window.startDragAndDrop(
                        StartDragAndDropParams(
                            mimeTypes = listOf(TEXT_UTF8_MIME_TYPE),
                            actions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                            dragIconParams = null,
                        ),
                    )
                }

                mouseX += 10
                while (mouseX < expectedScreenSize.width - 100) {
                    moveMouseTo(mouseX, mouseY)
                    mouseX += 10
                }
            }

            awaitEventOfType<Event.DataTransferCancelled> { event ->
                assertEquals(DataSource.DragAndDrop, event.dataSource)
                true
            }
            assertFalse(dataTransferTriggered.isDone)
        }
    }

    @Test
    fun testMouseScroll() {
        run(defaultApplicationConfig())
        val windowParams = WindowParams(
            windowId = 0,
            title = "Test Window 1",
            size = LogicalSize(width = 200, height = 300),
            minSize = null,
        )
        val window = ui { app.createWindow(windowParams) }
        assertNotNull(window)

        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            event.active
        }
        awaitEventOfType<Event.WindowDraw> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }

        moveMouseTo(100, 100)
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
        val windowFuture = CompletableFuture<Window>()
        val windowIsActive = CompletableFuture<Unit>()
        eventHandler = { event ->
            assertTrue(app.isEventLoopThread())
            if (event is Event.ShouldRedraw && windowIsActive.isDone) {
                val window = windowFuture.get(0, TimeUnit.MILLISECONDS)
                assertEquals(window.windowId, event.windowId)
                window.requestRedraw()
            }
            EventHandlerResult.Continue
        }
        run(defaultApplicationConfig())
        val windowParams = WindowParams(
            windowId = 0,
            title = "Test Window 1",
            size = LogicalSize(width = 200, height = 300),
            minSize = null,
        )
        windowFuture.complete(ui { app.createWindow(windowParams) })

        awaitEventOfType<Event.WindowConfigure> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            event.active
        }
        windowIsActive.complete(Unit)

        awaitEventOfType<Event.WindowDraw> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }
        awaitEventOfType<Event.WindowDraw> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
        }
        awaitEventOfType<Event.WindowDraw> { event ->
            assertEquals(windowParams.windowId, event.windowId)
            true
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
}
