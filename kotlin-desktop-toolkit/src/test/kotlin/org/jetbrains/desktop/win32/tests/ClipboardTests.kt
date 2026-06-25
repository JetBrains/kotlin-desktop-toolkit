package org.jetbrains.desktop.win32.tests

import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.Clipboard
import org.jetbrains.desktop.win32.ClipboardResult
import org.jetbrains.desktop.win32.DataFormat
import org.jetbrains.desktop.win32.DataObject
import org.jetbrains.desktop.win32.DataObjectBuilder
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.jetbrains.desktop.win32.TransferException
import org.jetbrains.desktop.win32.TransferStatus
import org.jetbrains.desktop.win32.WindowParams
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotEquals
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
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import kotlin.concurrent.thread
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertContentEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
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
    @Timeout(20)
    fun `set and flush publishes text`() = runClipboardTest {
        publish { addTextItem("hello clipboard") }
        readClipboard { assertEquals("hello clipboard", it.readTextItem()) }
    }

    @Test
    @Timeout(20)
    fun `data object round trips multiple formats`() = runClipboardTest {
        val expectedText = "batch text"
        val expectedHtml = "<i>batch html</i>"
        val expectedFiles = listOf("C:\\Temp\\kdt-one.txt", "C:\\Temp\\kdt-two.txt")
        publish {
            addTextItem(expectedText)
            addHtmlFragment(expectedHtml)
            addListOfFiles(expectedFiles)
        }

        readClipboard {
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
    }

    @Test
    @Timeout(20)
    fun `custom byte format round trips`() = runClipboardTest {
        val format = DataFormat.register("KDT_TEST_BYTES_${System.nanoTime()}")
        val expected = byteArrayOf(4, 5, 6, 7)
        publish { addItemOfType(format, expected) }
        readClipboard { assertContentEquals(expected, it.readItemOfType(format)) }
    }

    @Test
    @Timeout(20)
    fun `reads unicode text written without OLE`() = runClipboardTest {
        User32Clipboard.writeUnicodeText("written via user32")
        readClipboard { assertEquals("written via user32", it.readTextItem()) }
    }

    @Test
    @Timeout(20)
    fun `tryRead returns null and read throws for unavailable format`() = runClipboardTest {
        val unavailable = DataFormat.register("KDT_TEST_UNAVAILABLE_${System.nanoTime()}")
        publish { addTextItem("available text") }
        readClipboard {
            assertNull(it.tryReadItemOfType(unavailable))
            val exception = assertFailsWith<TransferException> { it.readItemOfType(unavailable) }
            assertEquals(TransferStatus.FormatUnavailable, exception.status)
        }
    }

    @Test
    @Timeout(20)
    fun `clear empties clipboard`() = runClipboardTest {
        publish { addTextItem("content to clear") }
        withClipboardRetry { Clipboard.clear() }
        assertEquals(0, User32Clipboard.countClipboardFormats())
    }

    @Test
    @Timeout(20)
    fun `sequence number changes after publishing`() = runClipboardTest {
        val before = Clipboard.sequenceNumber()
        publish { addTextItem("bump the sequence") }
        assertNotEquals(before, Clipboard.sequenceNumber())
    }

    @Test
    @Timeout(20)
    fun `set without flush keeps the data object current and readable`() = runClipboardTest {
        DataObject.build { addTextItem("delayed rendering") }.use { data ->
            withClipboardRetry { Clipboard.set(data) }
            assertTrue(withClipboardRetry { Clipboard.isCurrent(data) }, "data object should be current right after set")
            readClipboard { assertEquals("delayed rendering", it.readTextItem()) }
            withClipboardRetry { Clipboard.clear() }
            assertFalse(withClipboardRetry { Clipboard.isCurrent(data) }, "data object should not be current after clear")
        }
    }

    @Test
    @Timeout(20)
    fun `retained data object survives original wrapper close`() = runClipboardTest {
        val retained = DataObject.build { addTextItem("retained text") }.use { data ->
            data.retain()
        }
        retained.use { data ->
            withClipboardRetry { Clipboard.set(data) }
            withClipboardRetry { Clipboard.flush() }
        }
        readClipboard { assertEquals("retained text", it.readTextItem()) }
    }

    @Test
    @Timeout(20)
    fun `flush renders the set data object and drops the current flag`() = runClipboardTest {
        DataObject.build { addTextItem("flush me") }.use { data ->
            withClipboardRetry { Clipboard.set(data) }
            assertTrue(withClipboardRetry { Clipboard.isCurrent(data) })
            withClipboardRetry { Clipboard.flush() }
            assertFalse(withClipboardRetry { Clipboard.isCurrent(data) }, "flush releases OLE's reference to the data object")
            readClipboard { assertEquals("flush me", it.readTextItem()) }
        }
    }

    @Test
    @Timeout(20)
    fun `data object builder reports duplicate formats`() = runClipboardTest {
        DataObject.build {
            assertTrue(addTextItem("first"))
            assertFalse(addTextItem("second"))
        }.close()
    }

    @Test
    @Timeout(20)
    fun `data object builder rejects empty payload`() = runClipboardTest {
        val format = DataFormat.register("KDT_TEST_EMPTY_${System.nanoTime()}")
        assertFailsWith<IllegalArgumentException> {
            DataObject.build { addItemOfType(format, byteArrayOf()) }
        }
    }

    @Test
    @Timeout(20)
    fun `get reports Busy while the clipboard is held`() = runClipboardTest {
        HeldClipboard.open().use {
            val result = Clipboard.get()
            assertIs<ClipboardResult.Failure>(result)
            assertEquals(TransferStatus.Busy, result.status)
        }
    }

    @Test
    @Timeout(20)
    fun `set reports Busy while the clipboard is held`() = runClipboardTest {
        DataObject.build { addTextItem("blocked write") }.use { data ->
            HeldClipboard.open().use {
                val result = Clipboard.set(data)
                assertIs<ClipboardResult.Failure>(result)
                assertEquals(TransferStatus.Busy, result.status)
            }
        }
    }

    @Test
    @Timeout(20)
    fun `caller can retry until contention clears`() = runClipboardTest {
        val held = HeldClipboard.open()
        val releaser = thread(name = "clipboard-releaser", isDaemon = true) {
            Thread.sleep(100)
            held.close()
        }
        try {
            publish { addTextItem("written after retry") }
        } finally {
            releaser.join(5_000)
        }
        readClipboard { assertEquals("written after retry", it.readTextItem()) }
    }
}

/**
 * Unwraps a [ClipboardResult], retrying only while it reports [TransferStatus.Busy]. This is the
 * caller-owned retry contract the toolkit deliberately leaves to the application; the success-path
 * tests use it so incidental clipboard contention from other processes does not make them flaky.
 */
private fun <T> withClipboardRetry(timeoutMs: Long = 5_000, block: () -> ClipboardResult<T>): T {
    val deadline = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(timeoutMs)
    while (true) {
        when (val result = block()) {
            is ClipboardResult.Success -> return result.value
            is ClipboardResult.Failure -> {
                check(result.status == TransferStatus.Busy && System.nanoTime() < deadline) {
                    "clipboard operation failed: ${result.status} (0x${result.nativeCode.toUInt().toString(16)})"
                }
                Thread.sleep(10)
            }
        }
    }
}

/** Builds a [DataObject], publishes it with `set` + `flush`, and retries past transient contention. */
private fun publish(build: DataObjectBuilder.() -> Unit) {
    DataObject.build(build).use { data ->
        withClipboardRetry { Clipboard.set(data) }
        withClipboardRetry { Clipboard.flush() }
    }
}

/** Reads the current clipboard contents (retrying past transient contention) and closes the data object. */
private fun <T> readClipboard(read: (DataObject) -> T): T = withClipboardRetry { Clipboard.get() }.use(read)

/**
 * Runs [body] on the application dispatcher thread (the OLE STA), where the synchronous clipboard
 * API must be used, then stops the event loop.
 */
private fun runClipboardTest(body: () -> Unit) {
    val failure = AtomicReference<Throwable>()
    Application().use { app ->
        app.onStartup {
            val window = app.newWindow()
            try {
                window.create(WindowParams(title = "Clipboard Test"))
                body()
            } catch (t: Throwable) {
                failure.set(t)
            } finally {
                runCatching { Clipboard.clear() }
                runCatching { window.destroy() }
                window.close()
                app.stopEventLoop()
            }
        }
        app.runEventLoop { _, _ -> EventHandlerResult.Continue }
    }
    failure.get()?.let { throw it }
}

/** Holds the Win32 clipboard open on a background thread so toolkit calls observe contention. */
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
        // CF_UNICODETEXT must be NUL-terminated; append a UTF-16LE NUL code unit (two zero bytes).
        val bytes = text.toByteArray(Charsets.UTF_16LE) + byteArrayOf(0, 0)
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
