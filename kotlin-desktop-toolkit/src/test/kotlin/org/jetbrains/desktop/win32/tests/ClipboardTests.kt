package org.jetbrains.desktop.win32.tests

import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.Clipboard
import org.jetbrains.desktop.win32.ClipboardChangedException
import org.jetbrains.desktop.win32.ClipboardException
import org.jetbrains.desktop.win32.ClipboardStatus
import org.jetbrains.desktop.win32.DataFormat
import org.jetbrains.desktop.win32.DataObject
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
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
    fun `async write retries busy clipboard and succeeds`() {
        runClipboardTest { app, _, finish ->
            val expected = "async write after contention"
            val lock = HeldClipboard.open()
            val future = Clipboard.writeAsync(app) {
                setText(expected)
            }
            assertFalse(future.isDone, "Expected first attempt to hit the held clipboard and schedule a retry")

            releaseLater(lock)
            future.thenCompose {
                readClipboardValueAsync(app) { it.readTextItem() }
            }.whenComplete { actual, throwable ->
                finishFromCallback(finish, throwable) {
                    assertEquals(expected, actual)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async clipboard operations retry in FIFO order`() {
        runClipboardTest { app, _, finish ->
            val lock = HeldClipboard.open()
            val first = Clipboard.writeAsync(app) {
                setText("first delayed write")
            }
            assertFalse(first.isDone, "Expected first write to hit the held clipboard and schedule a retry")

            releaseAndInvokeLater(app, lock, finish) {
                val second = Clipboard.writeAsync(app) {
                    setText("second queued write")
                }
                CompletableFuture.allOf(first, second).thenCompose {
                    readClipboardValueAsync(app) { it.readTextItem() }
                }.whenComplete { actual, throwable ->
                    finishFromCallback(finish, throwable) {
                        assertEquals("second queued write", actual)
                    }
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async read retries busy clipboard and succeeds`() {
        runClipboardTest { app, _, finish ->
            val expected = "async read after contention"
            User32Clipboard.writeUnicodeText(expected)

            val lock = HeldClipboard.open()
            val future = readClipboardValueAsync(app) { it.readTextItem() }
            assertFalse(future.isDone, "Expected first attempt to hit the held clipboard and schedule a retry")

            releaseLater(lock)
            future.whenComplete { actual, throwable ->
                finishFromCallback(finish, throwable) {
                    assertEquals(expected, actual)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async read fails if clipboard changes before retry`() {
        runClipboardTest { app, _, finish ->
            User32Clipboard.writeUnicodeText("initial text")
            val expected = Clipboard.changeCount()

            val lock = HeldClipboard.open()
            val future = readClipboardValueAsync(app) { it.readTextItem() }
            assertFalse(future.isDone, "Expected first attempt to hit the held clipboard and schedule a retry")

            releaseAndWriteLater(lock, "changed text")
            future.whenComplete { _, throwable ->
                try {
                    val exception = assertCause<ClipboardChangedException>(throwable)
                    assertEquals(expected, exception.expectedChangeCount)
                    assertEquals(Clipboard.changeCount(), exception.actualChangeCount)
                    finish(null)
                } catch (t: Throwable) {
                    finish(t)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async clipboard APIs must be called from dispatcher thread`() {
        runClipboardTest { app, _, finish ->
            thread(name = "clipboard-wrong-thread", isDaemon = true) {
                try {
                    Clipboard.readAsync(app).whenComplete { clipboardData, throwable ->
                        clipboardData?.close()
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
        runClipboardTest { app, _, finish ->
            val format = DataFormat.register("KDT_TEST_SINGLE_BYTES_${System.nanoTime()}")
            val expected = byteArrayOf(4, 5, 6)
            val data = expected.copyOf()
            val lock = HeldClipboard.open()
            val future = Clipboard.writeAsync(app) {
                setItemOfType(format, data)
            }
            assertFalse(future.isDone, "Expected first attempt to hit the held clipboard and schedule a retry")

            data[0] = 9

            releaseLater(lock)
            future.thenCompose {
                readClipboardValueAsync(app) { it.readItemOfType(format) }
            }.whenComplete { actual, throwable ->
                finishFromCallback(finish, throwable) {
                    assertContentEquals(expected, actual)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async write builder rejects duplicate formats without retrying`() {
        runClipboardTest { app, _, finish ->
            Clipboard.writeAsync(app) {
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
    fun `data object try read returns null for unavailable format`() {
        runClipboardTest { app, _, finish ->
            val unavailableFormat = DataFormat.register("KDT_TEST_UNAVAILABLE_${System.nanoTime()}")
            Clipboard.writeAsync(app) {
                setText("available text")
            }.thenCompose {
                readClipboardValueAsync(app) { it.tryReadItemOfType(unavailableFormat) }
            }.whenComplete { value, throwable ->
                finishFromCallback(finish, throwable) {
                    assertNull(value)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async builder write publishes multiple formats`() {
        runClipboardTest { app, _, finish ->
            val expectedText = "builder text"
            val expectedHtml = "<b>builder html</b>"
            Clipboard.writeAsync(app) {
                setText(expectedText)
                setHtmlFragment(expectedHtml)
            }.thenCompose {
                readClipboardValueAsync(app) {
                    Triple(it.readTextItem(), it.readHtmlFragment(), it.listItemFormats())
                }
            }.whenComplete { actual, throwable ->
                finishFromCallback(finish, throwable) {
                    val (text, html, formats) = actual
                    assertEquals(expectedText, text)
                    assertEquals(expectedHtml, html)
                    assertContains(formats, DataFormat.Text)
                    assertContains(formats, DataFormat.Html)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async batch write snapshots mutable inputs before retry`() {
        runClipboardTest { app, _, finish ->
            val bytesFormat = DataFormat.register("KDT_TEST_BYTES_${System.nanoTime()}")
            val expectedBytes = byteArrayOf(1, 2, 3)
            val bytes = expectedBytes.copyOf()
            val expectedFiles = listOf("C:\\Temp\\kdt-snapshot-one.txt", "C:\\Temp\\kdt-snapshot-two.txt")
            val files = expectedFiles.toMutableList()
            val lock = HeldClipboard.open()
            val future = Clipboard.writeAsync(app) {
                setItemOfType(bytesFormat, bytes)
                setListOfFiles(files)
            }
            assertFalse(future.isDone, "Expected first attempt to hit the held clipboard and schedule a retry")

            bytes[0] = 9
            files[0] = "C:\\Temp\\mutated.txt"

            releaseLater(lock)
            future.thenCompose {
                readClipboardValueAsync(app) {
                    it.readItemOfType(bytesFormat) to it.readListOfFiles()
                }
            }.whenComplete { actual, throwable ->
                finishFromCallback(finish, throwable) {
                    val (actualBytes, actualFiles) = actual
                    assertContentEquals(expectedBytes, actualBytes)
                    assertEquals(expectedFiles, actualFiles)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `write data object retries busy clipboard and succeeds`() {
        runClipboardTest { app, _, finish ->
            val expected = "data object write after contention"
            val lock = HeldClipboard.open()
            val future = DataObject.build {
                addTextItem(expected)
            }.use { dataObject ->
                Clipboard.writeDataObjectAsync(app, dataObject)
            }
            assertFalse(future.isDone, "Expected first write attempt to hit the held clipboard and schedule a retry")

            releaseLater(lock)
            future.thenCompose {
                readClipboardValueAsync(app) { it.readTextItem() }
            }.whenComplete { actual, throwable ->
                finishFromCallback(finish, throwable) {
                    assertEquals(expected, actual)
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `async read gives up while clipboard stays busy`() {
        runClipboardTest { app, _, finish ->
            User32Clipboard.writeUnicodeText("async read should fail while busy")
            val lock = HeldClipboard.open(releaseTimeoutSeconds = 30)
            val future = Clipboard.readAsync(app)
                .orTimeout(8, TimeUnit.SECONDS)
            assertFalse(future.isDone, "Expected first read attempt to hit the held clipboard and schedule a retry")

            future.whenComplete { clipboardData, throwable ->
                try {
                    clipboardData?.close()
                    val exception = assertCause<ClipboardException>(throwable)
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
    fun `async clear empties clipboard`() {
        runClipboardTest { app, _, finish ->
            Clipboard.writeAsync(app) {
                setText("content to clear")
            }.thenCompose {
                Clipboard.clearAsync(app)
            }.whenComplete { _, throwable ->
                finishFromCallback(finish, throwable) {
                    assertEquals(0, User32Clipboard.countClipboardFormats())
                }
            }
        }
    }

    @Test
    @Timeout(10)
    fun `data object round trips multiple formats`() {
        runClipboardTest { app, _, finish ->
            val expectedText = "batch text"
            val expectedHtml = "<i>batch html</i>"
            val expectedFiles = listOf("C:\\Temp\\kdt-one.txt", "C:\\Temp\\kdt-two.txt")
            val dataObject = DataObject.build {
                addTextItem(expectedText)
                addHtmlFragment(expectedHtml)
                addListOfFiles(expectedFiles)
            }

            Clipboard.writeDataObjectAsync(app, dataObject).thenCompose {
                readClipboardValueAsync(app) {
                    ClipboardContents(
                        text = it.readTextItem(),
                        html = it.readHtmlFragment(),
                        files = it.readListOfFiles(),
                        formats = it.listItemFormats(),
                        hasText = it.isFormatAvailable(DataFormat.Text),
                        hasHtml = it.isFormatAvailable(DataFormat.Html),
                        hasFileList = it.isFormatAvailable(DataFormat.FileList),
                    )
                }
            }.whenComplete { actual, throwable ->
                dataObject.close()
                finishFromCallback(finish, throwable) {
                    assertEquals(expectedText, actual.text)
                    assertEquals(expectedHtml, actual.html)
                    assertEquals(expectedFiles, actual.files)
                    assertContains(actual.formats, DataFormat.Text)
                    assertContains(actual.formats, DataFormat.Html)
                    assertContains(actual.formats, DataFormat.FileList)
                    assertTrue(actual.hasText)
                    assertTrue(actual.hasHtml)
                    assertTrue(actual.hasFileList)
                }
            }
        }
    }
}

private data class ClipboardContents(
    val text: String,
    val html: String,
    val files: List<String>,
    val formats: List<DataFormat>,
    val hasText: Boolean,
    val hasHtml: Boolean,
    val hasFileList: Boolean,
)

private fun <T> readClipboardValueAsync(app: Application, read: (DataObject) -> T): CompletableFuture<T> =
    Clipboard.readAsync(app).thenApply { clipboardData ->
        clipboardData.use { read(it) }
    }

private fun finishFromCallback(finish: (Throwable?) -> Unit, throwable: Throwable?, assertions: () -> Unit) {
    try {
        if (throwable != null) {
            throw unwrapCompletion(throwable)
        }
        assertions()
        finish(null)
    } catch (t: Throwable) {
        finish(t)
    }
}

private inline fun <reified T : Throwable> assertCause(throwable: Throwable?): T {
    val cause = unwrapCompletion(checkNotNull(throwable) { "Expected clipboard operation to fail" })
    return cause as? T ?: throw cause
}

private tailrec fun unwrapCompletion(throwable: Throwable): Throwable {
    return when (throwable) {
        is CompletionException -> unwrapCompletion(throwable.cause ?: throwable)
        is ExecutionException -> unwrapCompletion(throwable.cause ?: throwable)
        else -> throwable
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

private fun releaseAndInvokeLater(app: Application, lock: HeldClipboard, finish: (Throwable?) -> Unit, body: () -> Unit) {
    CompletableFuture.delayedExecutor(15, TimeUnit.MILLISECONDS).execute {
        try {
            lock.close()
            app.invokeOnDispatcher(body)
        } catch (t: Throwable) {
            finish(t)
        }
    }
}

private fun releaseAndWriteLater(lock: HeldClipboard, text: String) {
    CompletableFuture.delayedExecutor(45, TimeUnit.MILLISECONDS).execute {
        lock.close()
        User32Clipboard.writeUnicodeText(text)
    }
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
    private const val CF_UNICODETEXT = 13
    private const val GMEM_MOVEABLE = 0x0002

    private val linker: Linker = Linker.nativeLinker()
    private val user32: SymbolLookup = SymbolLookup.libraryLookup("user32.dll", Arena.global())
    private val kernel32: SymbolLookup = SymbolLookup.libraryLookup("kernel32.dll", Arena.global())
    private val openClipboard: MethodHandle = linker.downcallHandle(
        user32.find("OpenClipboard").orElseThrow(),
        FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS),
    )
    private val closeClipboard: MethodHandle = linker.downcallHandle(
        user32.find("CloseClipboard").orElseThrow(),
        FunctionDescriptor.of(ValueLayout.JAVA_INT),
    )
    private val countClipboardFormats: MethodHandle = linker.downcallHandle(
        user32.find("CountClipboardFormats").orElseThrow(),
        FunctionDescriptor.of(ValueLayout.JAVA_INT),
    )
    private val emptyClipboard: MethodHandle = linker.downcallHandle(
        user32.find("EmptyClipboard").orElseThrow(),
        FunctionDescriptor.of(ValueLayout.JAVA_INT),
    )
    private val setClipboardData: MethodHandle = linker.downcallHandle(
        user32.find("SetClipboardData").orElseThrow(),
        FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.JAVA_INT, ValueLayout.ADDRESS),
    )
    private val globalAlloc: MethodHandle = linker.downcallHandle(
        kernel32.find("GlobalAlloc").orElseThrow(),
        FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.JAVA_INT, ValueLayout.JAVA_LONG),
    )
    private val globalFree: MethodHandle = linker.downcallHandle(
        kernel32.find("GlobalFree").orElseThrow(),
        FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS),
    )
    private val globalLock: MethodHandle = linker.downcallHandle(
        kernel32.find("GlobalLock").orElseThrow(),
        FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS),
    )
    private val globalUnlock: MethodHandle = linker.downcallHandle(
        kernel32.find("GlobalUnlock").orElseThrow(),
        FunctionDescriptor.of(ValueLayout.JAVA_INT, ValueLayout.ADDRESS),
    )

    fun openClipboard(): Boolean = (openClipboard.invoke(MemorySegment.NULL) as Int) != 0

    fun closeClipboard(): Boolean = (closeClipboard.invoke() as Int) != 0

    fun countClipboardFormats(): Int = countClipboardFormats.invoke() as Int

    fun writeUnicodeText(text: String) {
        val bytes = "$text\u0000".toByteArray(Charsets.UTF_16LE)
        val mem = globalAlloc.invoke(GMEM_MOVEABLE, bytes.size.toLong()) as MemorySegment
        check(mem != MemorySegment.NULL) { "GlobalAlloc failed" }

        var transferred = false
        try {
            val content = globalLock.invoke(mem) as MemorySegment
            check(content != MemorySegment.NULL) { "GlobalLock failed" }
            MemorySegment.copy(MemorySegment.ofArray(bytes), 0, content.reinterpret(bytes.size.toLong()), 0, bytes.size.toLong())
            globalUnlock.invoke(mem)

            check(openClipboard()) { "Failed to open clipboard for external write" }
            try {
                check((emptyClipboard.invoke() as Int) != 0) { "EmptyClipboard failed" }
                val result = setClipboardData.invoke(CF_UNICODETEXT, mem) as MemorySegment
                check(result != MemorySegment.NULL) { "SetClipboardData failed" }
                transferred = true
            } finally {
                check(closeClipboard()) { "Failed to close clipboard after external write" }
            }
        } finally {
            if (!transferred) {
                globalFree.invoke(mem)
            }
        }
    }
}
