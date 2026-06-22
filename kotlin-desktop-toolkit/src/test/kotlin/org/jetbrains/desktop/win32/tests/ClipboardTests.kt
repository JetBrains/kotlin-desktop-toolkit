package org.jetbrains.desktop.win32.tests

import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.Clipboard
import org.jetbrains.desktop.win32.ClipboardException
import org.jetbrains.desktop.win32.ClipboardStatus
import org.jetbrains.desktop.win32.DataFormat
import org.jetbrains.desktop.win32.DataObject
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.jetbrains.desktop.win32.OleClipboard
import org.jetbrains.desktop.win32.Window
import org.jetbrains.desktop.win32.WindowParams
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.Timeout
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.lang.foreign.Arena
import java.lang.foreign.FunctionDescriptor
import java.lang.foreign.Linker
import java.lang.foreign.MemorySegment
import java.lang.foreign.SymbolLookup
import java.lang.foreign.ValueLayout
import java.lang.invoke.MethodHandle
import java.nio.file.Path
import java.util.concurrent.CompletableFuture
import java.util.concurrent.CompletionException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.ExecutionException
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference
import kotlin.concurrent.thread
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertContentEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

@EnabledOnOs(OS.WINDOWS)
class ClipboardTests {
    companion object {
        @BeforeAll
        @JvmStatic
        fun loadLibrary() {
            KotlinDesktopToolkit.init(
                libraryFolderPath = Path.of(System.getProperty("kdt.win32.library.folder.path")!!),
            )
        }
    }

    @Test
    @Timeout(10)
    fun `synchronous write fails fast when clipboard is busy`() {
        runClipboardTest { _, window, finish ->
            val lock = HeldClipboard.open()
            try {
                val exception = assertFailsWith<ClipboardException> {
                    @Suppress("DEPRECATION")
                    Clipboard.writeTextItem(window, "sync write should fail")
                }
                assertEquals(ClipboardStatus.Busy, exception.status)
                finish(null)
            } catch (t: Throwable) {
                finish(t)
            } finally {
                lock.close()
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async write retries busy clipboard and succeeds`() {
        runClipboardTest { app, window, finish ->
            val expected = "async write after contention"
            val lock = HeldClipboard.open()
            val future = Clipboard.writeAsync(app, window) {
                setText(expected)
            }
            assertFalse(future.isDone, "Expected first attempt to hit the held clipboard and schedule a retry")

            releaseLater(lock)
            future.whenComplete { _, throwable ->
                if (throwable != null) {
                    finish(throwable)
                    return@whenComplete
                }
                try {
                    @Suppress("DEPRECATION")
                    assertEquals(expected, Clipboard.readTextItem(window))
                    finish(null)
                } catch (t: Throwable) {
                    finish(t)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async read retries busy clipboard and succeeds`() {
        runClipboardTest { app, window, finish ->
            val expected = "async read after contention"
            @Suppress("DEPRECATION")
            Clipboard.writeTextItem(window, expected)

            val lock = HeldClipboard.open()
            val future = Clipboard.readTextItemAsync(app, window)
            assertFalse(future.isDone, "Expected first attempt to hit the held clipboard and schedule a retry")

            releaseLater(lock)
            future.whenComplete { actual, throwable ->
                if (throwable != null) {
                    finish(throwable)
                } else {
                    try {
                        assertEquals(expected, actual)
                        finish(null)
                    } catch (t: Throwable) {
                        finish(t)
                    }
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async clipboard APIs must be called from dispatcher thread`() {
        runClipboardTest { app, window, finish ->
            thread(name = "clipboard-wrong-thread", isDaemon = true) {
                try {
                    Clipboard.readTextItemAsync(app, window).whenComplete { _, throwable ->
                        try {
                            val error = checkNotNull(throwable) { "Expected wrong-thread call to fail" }
                            assertTrue(error is IllegalStateException)
                            assertContains(error.message.orEmpty(), "dispatcher thread")
                            finish(null)
                        } catch (t: Throwable) {
                            finish(t)
                        }
                    }
                } catch (t: Throwable) {
                    finish(t)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async byte write snapshots mutable payload before retry`() {
        runClipboardTest { app, window, finish ->
            val format = DataFormat.register("KDT_TEST_SINGLE_BYTES_${System.nanoTime()}")
            val expected = byteArrayOf(4, 5, 6)
            val data = expected.copyOf()
            val lock = HeldClipboard.open()
            val future = Clipboard.writeAsync(app, window) {
                setItemOfType(format, data)
            }
            assertFalse(future.isDone, "Expected first attempt to hit the held clipboard and schedule a retry")

            data[0] = 9

            releaseLater(lock)
            future.whenComplete { _, throwable ->
                if (throwable != null) {
                    finish(throwable)
                    return@whenComplete
                }
                try {
                    @Suppress("DEPRECATION")
                    assertContentEquals(expected, Clipboard.readItemOfType(window, format))
                    finish(null)
                } catch (t: Throwable) {
                    finish(t)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async write gives up while clipboard stays busy`() {
        runClipboardTest { app, window, finish ->
            val lock = HeldClipboard.open(releaseTimeoutSeconds = 30)
            val future = Clipboard.writeAsync(app, window) {
                setText("async write should fail")
            }
                .orTimeout(8, TimeUnit.SECONDS)
            assertFalse(future.isDone, "Expected first attempt to hit the held clipboard and schedule a retry")

            future.whenComplete { _, throwable ->
                try {
                    val exception = assertClipboardException(throwable)
                    assertEquals(ClipboardStatus.Busy, exception.status)
                    finish(null)
                } catch (t: Throwable) {
                    finish(t)
                } finally {
                    lock.close()
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async write builder rejects duplicate formats without retrying`() {
        runClipboardTest { app, window, finish ->
            Clipboard.writeAsync(app, window) {
                setText("first")
                setText("second")
            }.whenComplete { _, throwable ->
                try {
                    val exception = checkNotNull(throwable) { "Expected duplicate format write to fail" }
                    assertTrue(exception is IllegalArgumentException)
                    assertContains(exception.message.orEmpty(), "already been set")
                    finish(null)
                } catch (t: Throwable) {
                    finish(t)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async try read returns null for unavailable format`() {
        runClipboardTest { app, window, finish ->
            val unavailableFormat = DataFormat.register("KDT_TEST_UNAVAILABLE_${System.nanoTime()}")
            Clipboard.tryReadItemOfTypeAsync(app, window, unavailableFormat).whenComplete { value, throwable ->
                if (throwable != null) {
                    finish(throwable)
                } else {
                    try {
                        assertNull(value)
                        finish(null)
                    } catch (t: Throwable) {
                        finish(t)
                    }
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async batch write publishes multiple formats in one operation`() {
        runClipboardTest { app, window, finish ->
            val expectedText = "batch text"
            val expectedHtml = "<b>batch html</b>"
            Clipboard.writeAsync(app, window) {
                setText(expectedText)
                setHtmlFragment(expectedHtml)
            }.whenComplete { _, throwable ->
                if (throwable != null) {
                    finish(throwable)
                    return@whenComplete
                }
                try {
                    @Suppress("DEPRECATION")
                    assertEquals(expectedText, Clipboard.readTextItem(window))
                    @Suppress("DEPRECATION")
                    assertEquals(expectedHtml, Clipboard.readHtmlFragment(window))
                    @Suppress("DEPRECATION")
                    val formats = Clipboard.listItemFormats(window)
                    assertContains(formats, DataFormat.Text)
                    assertContains(formats, DataFormat.Html)
                    finish(null)
                } catch (t: Throwable) {
                    finish(t)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async batch write snapshots mutable inputs before retry`() {
        runClipboardTest { app, window, finish ->
            val bytesFormat = DataFormat.register("KDT_TEST_BYTES_${System.nanoTime()}")
            val expectedBytes = byteArrayOf(1, 2, 3)
            val bytes = expectedBytes.copyOf()
            val expectedFiles = listOf("C:\\Temp\\kdt-snapshot-one.txt", "C:\\Temp\\kdt-snapshot-two.txt")
            val files = expectedFiles.toMutableList()
            val lock = HeldClipboard.open()
            val future = Clipboard.writeAsync(app, window) {
                setItemOfType(bytesFormat, bytes)
                setListOfFiles(files)
            }
            assertFalse(future.isDone, "Expected first attempt to hit the held clipboard and schedule a retry")

            bytes[0] = 9
            files[0] = "C:\\Temp\\mutated.txt"

            releaseLater(lock)
            future.whenComplete { _, throwable ->
                if (throwable != null) {
                    finish(throwable)
                    return@whenComplete
                }
                try {
                    @Suppress("DEPRECATION")
                    assertContentEquals(expectedBytes, Clipboard.readItemOfType(window, bytesFormat))
                    @Suppress("DEPRECATION")
                    assertEquals(expectedFiles, Clipboard.readListOfFiles(window))
                    finish(null)
                } catch (t: Throwable) {
                    finish(t)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `ole async write retries busy clipboard and succeeds`() {
        runClipboardTest { app, _, finish ->
            val expected = "ole async write after contention"
            val lock = HeldClipboard.open()
            val future = DataObject.build {
                addTextItem(expected)
            }.use { dataObject ->
                OleClipboard.writeDataObjectAsync(app, dataObject)
            }
            assertFalse(future.isDone, "Expected first OLE write attempt to hit the held clipboard and schedule a retry")

            releaseLater(lock)
            future.whenComplete { _, throwable ->
                try {
                    if (throwable != null) {
                        throw throwable
                    }
                    @Suppress("DEPRECATION")
                    OleClipboard.readClipboard().use { clipboardData ->
                        assertEquals(expected, clipboardData.readTextItem())
                    }
                    finish(null)
                } catch (t: Throwable) {
                    finish(t)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `ole async read retries busy clipboard and succeeds`() {
        runClipboardTest { app, _, finish ->
            val expected = "ole async read after contention"
            DataObject.build {
                addTextItem(expected)
            }.use { dataObject ->
                @Suppress("DEPRECATION")
                OleClipboard.writeToClipboard(dataObject)
            }

            val lock = HeldClipboard.open()
            val future = OleClipboard.readClipboardAsync(app)
            assertFalse(future.isDone, "Expected first OLE read attempt to hit the held clipboard and schedule a retry")

            releaseLater(lock)
            future.whenComplete { clipboardData, throwable ->
                if (throwable != null) {
                    finish(throwable)
                    return@whenComplete
                }
                try {
                    clipboardData.use {
                        assertEquals(expected, it.readTextItem())
                    }
                    finish(null)
                } catch (t: Throwable) {
                    finish(t)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `ole async read gives up while clipboard stays busy`() {
        runClipboardTest { app, _, finish ->
            DataObject.build {
                addTextItem("ole async read should fail while busy")
            }.use { dataObject ->
                @Suppress("DEPRECATION")
                OleClipboard.writeToClipboard(dataObject)
            }
            val lock = HeldClipboard.open(releaseTimeoutSeconds = 30)
            val future = OleClipboard.readClipboardAsync(app)
                .orTimeout(8, TimeUnit.SECONDS)
            assertFalse(future.isDone, "Expected first OLE read attempt to hit the held clipboard and schedule a retry")

            future.whenComplete { clipboardData, throwable ->
                try {
                    clipboardData?.close()
                    val exception = assertClipboardException(throwable)
                    assertEquals(ClipboardStatus.Busy, exception.status)
                    finish(null)
                } catch (t: Throwable) {
                    finish(t)
                } finally {
                    lock.close()
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `ole async clear empties clipboard`() {
        runClipboardTest { app, window, finish ->
            @Suppress("DEPRECATION")
            Clipboard.writeTextItem(window, "content to clear")

            OleClipboard.clearAsync(app).whenComplete { _, throwable ->
                if (throwable != null) {
                    finish(throwable)
                    return@whenComplete
                }
                try {
                    @Suppress("DEPRECATION")
                    assertEquals(0, Clipboard.itemCount(window))
                    finish(null)
                } catch (t: Throwable) {
                    finish(t)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `ole async builder write publishes multiple formats`() {
        runClipboardTest { app, _, finish ->
            val expectedText = "ole builder text"
            val expectedHtml = "<u>ole builder html</u>"
            val expectedFiles = listOf("C:\\Temp\\kdt-builder-one.txt", "C:\\Temp\\kdt-builder-two.txt")

            OleClipboard.writeAsync(app) {
                setText(expectedText)
                setHtmlFragment(expectedHtml)
                setListOfFiles(expectedFiles)
            }.whenComplete { _, writeThrowable ->
                if (writeThrowable != null) {
                    finish(writeThrowable)
                    return@whenComplete
                }
                OleClipboard.readClipboardAsync(app).whenComplete { clipboardData, readThrowable ->
                    try {
                        if (readThrowable != null) {
                            throw readThrowable
                        }
                        clipboardData.use {
                            assertEquals(expectedText, it.readTextItem())
                            assertEquals(expectedHtml, it.readHtmlFragment())
                            assertEquals(expectedFiles, it.readListOfFiles())
                        }
                        finish(null)
                    } catch (t: Throwable) {
                        finish(t)
                    }
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `ole data object round trips multiple formats`() {
        runClipboardTest { app, _, finish ->
            val expectedText = "ole batch text"
            val expectedHtml = "<i>ole batch html</i>"
            val expectedFiles = listOf("C:\\Temp\\kdt-one.txt", "C:\\Temp\\kdt-two.txt")
            val dataObject = DataObject.build {
                addTextItem(expectedText)
                addHtmlFragment(expectedHtml)
                addListOfFiles(expectedFiles)
            }

            OleClipboard.writeDataObjectAsync(app, dataObject).whenComplete { _, writeThrowable ->
                if (writeThrowable != null) {
                    dataObject.close()
                    finish(writeThrowable)
                    return@whenComplete
                }
                OleClipboard.readClipboardAsync(app).whenComplete { clipboardData, readThrowable ->
                    try {
                        if (readThrowable != null) {
                            throw readThrowable
                        }
                        clipboardData.use {
                            assertEquals(expectedText, it.readTextItem())
                            assertEquals(expectedHtml, it.readHtmlFragment())
                            assertEquals(expectedFiles, it.readListOfFiles())
                            val formats = it.listItemFormats()
                            assertContains(formats, DataFormat.Text)
                            assertContains(formats, DataFormat.Html)
                            assertContains(formats, DataFormat.FileList)
                            assertTrue(it.isFormatAvailable(DataFormat.Text))
                            assertTrue(it.isFormatAvailable(DataFormat.Html))
                            assertTrue(it.isFormatAvailable(DataFormat.FileList))
                        }
                        finish(null)
                    } catch (t: Throwable) {
                        finish(t)
                    } finally {
                        dataObject.close()
                    }
                }
            }
        }
    }
}

private fun runClipboardTest(body: (Application, Window, (Throwable?) -> Unit) -> Unit) {
    val failure = AtomicReference<Throwable>()
    val completed = AtomicBoolean(false)
    Application().use { app ->
        app.onStartup {
            val window = app.newWindow()

            fun stop(t: Throwable?) {
                if (!completed.compareAndSet(false, true)) {
                    return
                }
                if (t != null) {
                    failure.compareAndSet(null, t)
                }
                fun stopOnDispatcher() {
                    try {
                        window.destroy()
                    } finally {
                        window.close()
                        app.stopEventLoop()
                    }
                }
                if (app.isDispatcherThread()) {
                    stopOnDispatcher()
                } else {
                    app.invokeOnDispatcher(::stopOnDispatcher)
                }
            }

            try {
                window.create(WindowParams(title = "Clipboard Test"))
                CompletableFuture.delayedExecutor(5, TimeUnit.SECONDS).execute {
                    stop(AssertionError("Timed out waiting for clipboard async operation"))
                }
                body(app, window, ::stop)
            } catch (t: Throwable) {
                stop(t)
            }
        }
        app.runEventLoop { _, _ -> EventHandlerResult.Continue }
    }
    failure.get()?.let { throw it }
}

private fun releaseLater(lock: HeldClipboard) {
    CompletableFuture.delayedExecutor(75, TimeUnit.MILLISECONDS).execute {
        lock.close()
    }
}

private fun assertClipboardException(throwable: Throwable?): ClipboardException {
    val failure = checkNotNull(throwable) { "Expected clipboard operation to fail" }
    val cause = when (failure) {
        is CompletionException -> failure.cause ?: failure
        is ExecutionException -> failure.cause ?: failure
        else -> failure
    }
    return cause as? ClipboardException ?: throw cause
}

private class HeldClipboard private constructor(
    private val release: CountDownLatch,
    private val closed: CountDownLatch,
    private val failure: AtomicReference<Throwable>,
) : AutoCloseable {
    override fun close() {
        release.countDown()
        check(closed.await(5, TimeUnit.SECONDS)) { "Timed out closing held clipboard" }
        failure.get()?.let { throw it }
    }

    companion object {
        fun open(releaseTimeoutSeconds: Long = 5): HeldClipboard {
            val opened = CountDownLatch(1)
            val release = CountDownLatch(1)
            val closed = CountDownLatch(1)
            val failure = AtomicReference<Throwable>()

            thread(name = "held-clipboard", isDaemon = true) {
                var openedClipboard = false
                try {
                    val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5)
                    while (!openedClipboard && System.nanoTime() < deadline) {
                        openedClipboard = User32Clipboard.openClipboard()
                        if (!openedClipboard) {
                            Thread.sleep(10)
                        }
                    }
                    check(openedClipboard) { "Failed to open clipboard for contention test" }
                    opened.countDown()
                    check(release.await(releaseTimeoutSeconds, TimeUnit.SECONDS)) { "Timed out waiting to release held clipboard" }
                } catch (t: Throwable) {
                    failure.compareAndSet(null, t)
                    opened.countDown()
                } finally {
                    if (openedClipboard) {
                        try {
                            check(User32Clipboard.closeClipboard()) { "Failed to close held clipboard" }
                        } catch (t: Throwable) {
                            failure.compareAndSet(null, t)
                        }
                    }
                    closed.countDown()
                }
            }

            check(opened.await(5, TimeUnit.SECONDS)) { "Timed out opening clipboard for contention test" }
            failure.get()?.let { throw it }
            return HeldClipboard(release, closed, failure)
        }
    }
}

private object User32Clipboard {
    private val linker: Linker = Linker.nativeLinker()
    private val user32: SymbolLookup = SymbolLookup.libraryLookup("user32.dll", Arena.global())
    private val openClipboard: MethodHandle = linker.downcallHandle(
        user32.find("OpenClipboard").orElseThrow(),
        FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS),
    )
    private val closeClipboard: MethodHandle = linker.downcallHandle(
        user32.find("CloseClipboard").orElseThrow(),
        FunctionDescriptor.of(ValueLayout.JAVA_INT),
    )

    fun openClipboard(): Boolean = (openClipboard.invoke(MemorySegment.NULL) as Int) != 0

    fun closeClipboard(): Boolean = (closeClipboard.invoke() as Int) != 0
}
