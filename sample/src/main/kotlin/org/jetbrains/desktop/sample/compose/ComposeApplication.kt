package org.jetbrains.desktop.sample.compose

import androidx.compose.runtime.Applier
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Composition
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.MonotonicFrameClock
import androidx.compose.runtime.Recomposer
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.yield
import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.Screen
import org.jetbrains.desktop.macos.TextDirection
import org.jetbrains.desktop.macos.WindowEvent
import org.jetbrains.desktop.macos.WindowId
import java.util.concurrent.CountDownLatch
import kotlin.concurrent.thread

/**
 * Owns the native application: the AppKit event loop, the shared GPU context, and every open window.
 *
 * The kotlin-desktop-toolkit event loop must own the real macOS main thread. The JVM launcher does not
 * run `main` on that thread, so [GrandCentralDispatch.startOnMainThread] hands the loop over to it and
 * blocks its caller for as long as the loop runs — hence the dedicated thread below. Everything Compose
 * does is then dispatched back onto that same thread through [KdtDispatcher].
 */
class ComposeApplication : AutoCloseable {
    // Must run before anything else in this class: every other native call needs the library loaded.
    init {
        KotlinDesktopToolkit.init()
    }

    private val windows = mutableMapOf<WindowId, KdtComposeWindow>()
    private val applicationStarted = CountDownLatch(1)

    // Deferred so the Metal device is created on the main thread, once the event loop is up.
    private val lazyGpuContext = lazy { GpuContext() }
    internal val gpuContext: GpuContext get() = lazyGpuContext.value

    private val eventLoopThread = thread(start = true, name = "KDT Event Loop") {
        GrandCentralDispatch.startOnMainThread {
            Application.init()
            Application.runEventLoop { event ->
                when (event) {
                    is Event.ApplicationDidFinishLaunching -> applicationStarted.countDown()
                    is WindowEvent -> windows[event.windowId]?.handleEvent(event)
                    else -> {}
                }
                EventHandlerResult.Continue
            }
            if (lazyGpuContext.isInitialized()) {
                lazyGpuContext.value.close()
            }
            GrandCentralDispatch.close()
        }
    }

    init {
        applicationStarted.await()
    }

    /**
     * Density of the main screen, used for composables that live outside of any window.
     */
    fun globalDensity(): Density {
        return Density(Screen.allScreens().mainScreen().scale.toFloat(), fontScale = 1f)
    }

    /**
     * Reading order configured for the system. This rarely changes without a restart, so it is not
     * invalidated.
     */
    fun globalLayoutDirection(): LayoutDirection {
        return when (Application.textDirection) {
            TextDirection.LeftToRight -> LayoutDirection.Ltr
            TextDirection.RightToLeft -> LayoutDirection.Rtl
        }
    }

    internal fun createWindow(title: String, origin: LogicalPoint, size: LogicalSize, onCloseRequested: () -> Unit): ComposeWindow {
        val window = KdtComposeWindow(this, onCloseRequested, title, origin, size)
        windows[window.window.windowId()] = window
        return window
    }

    internal fun forgetWindow(windowId: WindowId) {
        windows.remove(windowId)
    }

    override fun close() {
        GrandCentralDispatch.dispatchOnMain {
            Application.stopEventLoop()
        }
        eventLoopThread.join()
    }
}

val LocalComposeApplication = staticCompositionLocalOf<ComposeApplication> {
    error("No ComposeApplication provided")
}

/**
 * Boots the native application. Returns once it has finished launching, so windows can be created.
 */
fun initApplication(): ComposeApplication {
    return ComposeApplication()
}

/**
 * Entry point into composition. Blocks until the composition has nothing left to do — that is, until
 * every window is gone and no `LaunchedEffect` is still running.
 */
fun runApplication(application: ComposeApplication, content: @Composable () -> Unit) {
    runBlocking(KdtDispatcher) {
        withContext(YieldFrameClock) {
            // SnapshotNotifier.ensureStarted()

            val recomposer = Recomposer(coroutineContext)
            launch {
                recomposer.runRecomposeAndApplyChanges()
            }
            launch {
                val composition = Composition(ApplicationApplier(), recomposer)
                try {
                    composition.setContent {
                        CompositionLocalProvider(
                            LocalComposeApplication provides application,
                            LocalDensity provides application.globalDensity(),
                            LocalLayoutDirection provides application.globalLayoutDirection(),
                        ) {
                            content()
                        }
                    }
                    recomposer.close()
                    // Returns only once nothing is keeping the composition alive.
                    recomposer.join()
                } finally {
                    composition.dispose()
                }
            }
        }
    }
}

/**
 * Frame clock for the application-level composition, which emits no pixels of its own — each window
 * drives its own frames from its display link.
 *
 * [yield] hands the main thread back to the event loop between frames, so pending input is not starved.
 */
private object YieldFrameClock : MonotonicFrameClock {
    override suspend fun <R> withFrameNanos(onFrame: (frameTimeNanos: Long) -> R): R {
        yield()
        return onFrame(System.nanoTime())
    }
}

/**
 * The application composition emits no nodes: `Window` is a side effect, not a layout node.
 */
private class ApplicationApplier : Applier<Any> {
    override val current: Any = Unit

    override fun down(node: Any) = Unit

    override fun up() = Unit

    override fun insertTopDown(index: Int, instance: Any) = checkNoNodes(instance)

    override fun insertBottomUp(index: Int, instance: Any) = checkNoNodes(instance)

    override fun remove(index: Int, count: Int) = Unit

    override fun move(from: Int, to: Int, count: Int) = Unit

    override fun clear() = Unit

    override fun onEndChanges() = Unit

    private fun checkNoNodes(instance: Any) {
        check(instance is Unit) { "Composable content may not be added directly into an application, use Window instead" }
    }
}
