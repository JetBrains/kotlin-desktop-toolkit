package org.jetbrains.desktop.win32.tests

import org.jetbrains.desktop.win32.ClipboardChangedException
import org.jetbrains.desktop.win32.ClipboardException
import org.jetbrains.desktop.win32.ClipboardOperationQueue
import org.jetbrains.desktop.win32.ClipboardQueuedOperationKind
import org.jetbrains.desktop.win32.ClipboardRetryDispatcher
import org.jetbrains.desktop.win32.ClipboardStatus
import org.junit.jupiter.api.Assertions.assertSame
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ExecutionException
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ClipboardOperationQueueTests {
    @Test
    fun `retry dispatch failure terminally fails active and pending operations`() {
        assertRetryDispatchTerminatesQueue(
            dispatchRetry = { false },
        ) { failure ->
            assertTrue(failure is IllegalStateException)
            assertEquals(RETRY_DISPATCH_FAILURE_MESSAGE, failure.message)
            assertNull(failure.cause)
        }
    }

    @Test
    fun `retry dispatch exception terminally fails active and pending operations`() {
        val dispatchFailure = IllegalStateException("dispatcher closed")

        assertRetryDispatchTerminatesQueue(
            dispatchRetry = { throw dispatchFailure },
        ) { failure ->
            assertTrue(failure is IllegalStateException)
            assertEquals(RETRY_DISPATCH_FAILURE_MESSAGE, failure.message)
            assertSame(dispatchFailure, failure.cause)
        }
    }

    @Test
    fun `change count failure while activating read fails active operation and advances queue`() {
        val sequenceFailure = IllegalStateException("sequence unavailable")
        val readInvoked = AtomicBoolean(false)
        val queue = ClipboardOperationQueue(immediateDispatcher) { throw sequenceFailure }

        val read = queue.enqueue(ClipboardQueuedOperationKind.Read) { _: UInt? ->
            readInvoked.set(true)
        }
        val next = queue.enqueue(ClipboardQueuedOperationKind.Write) { "next operation" }

        assertSame(sequenceFailure, assertFutureFailure(read))
        assertFalse(readInvoked.get(), "Read operation must not run when activation sequence capture fails")
        assertEquals("next operation", next.get(1, TimeUnit.SECONDS))
    }

    @Test
    fun `change count failure while retrying read fails active operation and advances queue`() {
        val sequenceFailure = IllegalStateException("sequence unavailable")
        var sequenceCalls = 0
        var attempts = 0
        val queue = ClipboardOperationQueue(immediateDispatcher) {
            sequenceCalls += 1
            if (sequenceCalls == 1) 1u else throw sequenceFailure
        }

        val read = queue.enqueue(ClipboardQueuedOperationKind.Read) { _: UInt? ->
            attempts += 1
            throw ClipboardException(ClipboardStatus.Busy, 0)
        }
        val next = queue.enqueue(ClipboardQueuedOperationKind.Write) { "next operation" }

        assertSame(sequenceFailure, assertFutureFailure(read))
        assertEquals(1, attempts)
        assertEquals("next operation", next.get(1, TimeUnit.SECONDS))
    }

    @Test
    fun `read fails when sequence changes before retry`() {
        var sequence = 1u
        var attempts = 0
        val queue = ClipboardOperationQueue(immediateDispatcher) { sequence }

        val read = queue.enqueue(ClipboardQueuedOperationKind.Read) { _: UInt? ->
            attempts += 1
            throw ClipboardException(ClipboardStatus.Busy, 0)
        }

        sequence = 2u

        val failure = assertFutureFailure(read)
        assertTrue(failure is ClipboardChangedException)
        assertEquals(1u, failure.expectedChangeCount)
        assertEquals(2u, failure.actualChangeCount)
        assertEquals(1, attempts)
    }

    @Test
    fun `queued read baselines after earlier queued write changes sequence`() {
        var sequence = 1u
        var writeAttempts = 0
        val queue = ClipboardOperationQueue(immediateDispatcher) { sequence }

        val write = queue.enqueue(ClipboardQueuedOperationKind.Write) {
            writeAttempts += 1
            if (writeAttempts == 1) {
                throw ClipboardException(ClipboardStatus.Busy, 0)
            }
            sequence += 1u
        }
        val read = queue.enqueue(ClipboardQueuedOperationKind.Read) { expectedChangeCount: UInt? ->
            assertEquals(sequence, expectedChangeCount)
            "read after write"
        }

        write.get(1, TimeUnit.SECONDS)
        assertEquals("read after write", read.get(1, TimeUnit.SECONDS))
        assertEquals(2u, sequence)
        assertEquals(2, writeAttempts)
    }

    private fun assertRetryDispatchTerminatesQueue(dispatchRetry: ClipboardRetryDispatcher, assertFailure: (Throwable) -> Unit) {
        val queue = ClipboardOperationQueue(dispatchRetry)
        var firstAttempts = 0
        val pendingInvoked = AtomicBoolean(false)

        val active = queue.enqueue(ClipboardQueuedOperationKind.Write) {
            firstAttempts += 1
            throw ClipboardException(ClipboardStatus.Busy, 0)
        }
        val pending = queue.enqueue(ClipboardQueuedOperationKind.Write) {
            pendingInvoked.set(true)
        }

        val activeFailure = assertFutureFailure(active)
        val pendingFailure = assertFutureFailure(pending)
        assertSame(activeFailure, pendingFailure)
        assertFailure(activeFailure)
        assertEquals(1, firstAttempts)
        assertFalse(pendingInvoked.get(), "Pending operation must not run when retry dispatch fails")

        val next = queue.enqueue(ClipboardQueuedOperationKind.Write) { "next operation" }
        assertEquals("next operation", next.get(1, TimeUnit.SECONDS))
    }

    private fun assertFutureFailure(future: CompletableFuture<*>): Throwable {
        return assertFailsWith<ExecutionException> {
            future.get(1, TimeUnit.SECONDS)
        }.cause ?: error("Expected future failure to have a cause")
    }

    private companion object {
        const val RETRY_DISPATCH_FAILURE_MESSAGE = "Failed to dispatch clipboard retry to the application dispatcher."

        val immediateDispatcher: ClipboardRetryDispatcher = { body ->
            body()
            true
        }
    }
}
