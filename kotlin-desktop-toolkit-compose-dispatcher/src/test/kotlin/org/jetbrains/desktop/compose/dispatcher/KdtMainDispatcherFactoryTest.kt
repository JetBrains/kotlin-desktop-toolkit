package org.jetbrains.desktop.compose.dispatcher

import kotlinx.coroutines.Dispatchers
import org.jetbrains.desktop.compose.KdtDispatcher
import org.jetbrains.desktop.compose.KdtImmediateDispatcher
import kotlin.test.Test
import kotlin.test.assertSame

/**
 * That the `ServiceLoader` registration actually takes effect.
 *
 * Worth a test because nothing in the compiler checks it: the factory is reached only through the
 * `META-INF/services` file, so a typo in that resource, or a shaded jar that drops it, leaves
 * `Dispatchers.Main` silently unresolved — and the failure would surface far from the cause.
 *
 * Needs no running application: `Dispatchers.Main` is resolved to the dispatcher object here, and only
 * dispatching through it would reach for an event loop.
 */
class KdtMainDispatcherFactoryTest {
    @Test
    fun `main dispatcher is the toolkit event loop`() {
        assertSame(KdtDispatcher, Dispatchers.Main)
    }

    @Test
    fun `immediate main dispatcher is the toolkit event loop`() {
        assertSame(KdtImmediateDispatcher, Dispatchers.Main.immediate)
    }
}
