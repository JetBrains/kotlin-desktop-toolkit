package org.jetbrains.desktop.sample.compose

import androidx.compose.runtime.Applier
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Composition
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.MonotonicFrameClock
import androidx.compose.runtime.Recomposer
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.SystemTheme
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.LayoutDirection
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.yield
import org.jetbrains.desktop.sample.compose.macos.MacOsApplication
import java.util.concurrent.atomic.AtomicReference

/**
 * The native application: the platform event loop, the shared GPU context, and every open window.
 *
 * There is at most one live instance per process — the platform event loop is a process-wide
 * singleton — and [currentApplication] returns it. Everything the toolkit exposes has to be touched
 * from the event loop's own thread; [invokeOnUiThread] is how you get there, and [KdtDispatcher] is
 * the coroutine dispatcher built on top of it.
 */
interface ComposeApplication : AutoCloseable {
    /** Light/dark preference configured for the system. Observable. */
    val systemTheme: SystemTheme

    /** Every attached display. Observable: replaced when the display configuration changes. */
    val screens: List<ComposeScreen>

    val mainScreen: ComposeScreen

    /** Reading order configured for the system. */
    val layoutDirection: LayoutDirection

    /** Windows currently open, in creation order. Observable. */
    val windows: List<ComposeWindow>

    /** The window that currently receives keyboard input, if this application has it. Observable. */
    val focusedWindow: ComposeWindow?

    /** Whether this application is the frontmost one. */
    val isActive: Boolean

    /** The platform object behind this application, for code that needs to reach past this abstraction. */
    val nativeApplication: Any

    fun requestActivation()

    /**
     * Asks the platform to terminate the application, which runs the same path as the user choosing
     * Quit: every handler registered with [putQuitHandler] gets a veto, and then every window is asked
     * to close through its `onCloseRequest`.
     */
    fun quit()

    /**
     * Registers a veto over termination. Returning `false` from [isSafeToQuit] cancels the quit.
     *
     * Keyed by [id] so that a handler can be replaced or dropped again; all handlers are consulted, so
     * none of them may assume it is the only one.
     */
    fun putQuitHandler(id: String, isSafeToQuit: () -> Boolean)

    fun removeQuitHandler(id: String)

    fun openUri(uri: String)

    /** Posts [block] to the event loop thread. */
    fun invokeOnUiThread(block: () -> Unit)

    /** Whether the caller is already on the event loop thread. */
    fun isUiThread(): Boolean

    /**
     * Provides the composition locals that are application-wide rather than per-window. Per-window
     * locals (density, layout direction, window info, ...) are provided by each window's scene.
     */
    @Composable
    fun ProvideCompositionLocals(content: @Composable () -> Unit)

    /** Creates a native window. Called by the [Window] composable; prefer that. */
    fun createWindow(params: ComposeWindowParams, onCloseRequest: (WindowCloseRequestReason) -> Unit): ComposeWindow

    /** Stops the event loop and releases every native resource still held. */
    override fun close()
}

class ComposeApplicationParams(
    /** Invoked when the OS hands the application URLs to open (`open -a`, a custom URL scheme, ...). */
    val onOpenUris: (List<String>) -> Unit = {},
)

val LocalComposeApplication = staticCompositionLocalOf<ComposeApplication> {
    error("No ComposeApplication provided")
}

/**
 * Boots the native application for the current platform and blocks until it has finished launching, so
 * that windows can be created straight away.
 *
 * This is the single point where the platform implementation is chosen; adding a platform means adding
 * a branch here plus a [ComposeApplication] implementation next to [MacOsApplication].
 */
fun initApplication(params: ComposeApplicationParams = ComposeApplicationParams()): ComposeApplication {
    val application = when (DesktopPlatform.current) {
        DesktopPlatform.MacOs -> MacOsApplication(params)
        DesktopPlatform.Linux,
        DesktopPlatform.Windows,
        -> error("Compose integration is only implemented for macOS so far, got ${DesktopPlatform.current}")
    }
    ApplicationRegistry.register(application)
    return application
}

/**
 * The live application. Used by code that cannot be handed one — most importantly [KdtDispatcher],
 * which has to resolve the UI thread without knowing which platform it is dispatching to.
 */
fun currentApplication(): ComposeApplication = ApplicationRegistry.current()

/**
 * Entry point into composition. Blocks until the composition has nothing left to do — that is, until
 * every window is gone and no `LaunchedEffect` is still running.
 */
fun runApplication(application: ComposeApplication, content: @Composable () -> Unit) {
    runBlocking(KdtDispatcher) {
        withContext(YieldFrameClock) {
            // Before the first composition, so that state written from an event handler invalidates it.
            KdtSnapshotManager.ensureStarted()

            val recomposer = Recomposer(coroutineContext)
            launch {
                recomposer.runRecomposeAndApplyChanges()
            }
            launch {
                val composition = Composition(ApplicationApplier(), recomposer)
                try {
                    composition.setContent {
                        application.ProvideCompositionLocals {
                            CompositionLocalProvider(
                                LocalComposeApplication provides application,
                                LocalDensity provides application.mainScreen.density,
                                LocalLayoutDirection provides application.layoutDirection,
                            ) {
                                content()
                            }
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
 * [yield] hands the event loop thread back between frames, so pending input is not starved.
 */
private object YieldFrameClock : MonotonicFrameClock {
    override suspend fun <R> withFrameNanos(onFrame: (frameTimeNanos: Long) -> R): R {
        yield()
        return onFrame(System.nanoTime())
    }
}

/**
 * The application composition emits no nodes: [Window] is a side effect, not a layout node.
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

private object ApplicationRegistry {
    private val application = AtomicReference<ComposeApplication?>(null)

    fun register(application: ComposeApplication) {
        check(this.application.compareAndSet(null, application)) {
            "Another ComposeApplication is already running in this process"
        }
    }

    fun unregister(application: ComposeApplication) {
        this.application.compareAndSet(application, null)
    }

    fun current(): ComposeApplication = checkNotNull(application.get()) {
        "No ComposeApplication has been initialized, call initApplication() first"
    }
}

/** Called by implementations from `close()`, so that a new application can be started afterwards. */
internal fun forgetApplication(application: ComposeApplication) {
    ApplicationRegistry.unregister(application)
}
