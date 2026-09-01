package org.jetbrains.desktop.compose

import androidx.compose.runtime.Applier
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Composition
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.MonotonicFrameClock
import androidx.compose.runtime.Recomposer
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.SystemTheme
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.LayoutDirection
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.yield
import org.jetbrains.desktop.compose.gtk.GtkApplication
import org.jetbrains.desktop.compose.linux.LinuxApplication
import org.jetbrains.desktop.compose.macos.MacOsApplication
import org.jetbrains.desktop.compose.windows.WindowsApplication
import java.nio.file.Path
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
    /**
     * Reverse-DNS name of the application, which the desktop matches its windows against a `.desktop` file
     * with. Used as the Wayland app id on Linux; macOS takes it from the bundle instead and ignores this.
     */
    val identifier: String = "org.jetbrains.desktop.compose",
    /**
     * Invoked when the OS hands the application URLs to open (`open -a`, a custom URL scheme, ...). Only
     * macOS reports these; the toolkit's Wayland backend has no equivalent event.
     */
    val onOpenUris: (List<String>) -> Unit = {},
    /** Where the toolkit's native library lives. Falls back to the `kdt.library.folder.path` property. */
    val libraryFolderPath: Path? = null,
    /** Where the native library writes its log. Falls back to the `kdt.native.log.path` property. */
    val logFilePath: Path? = null,
    val consoleLogLevel: ComposeLogLevel = ComposeLogLevel.Info,
    val fileLogLevel: ComposeLogLevel = ComposeLogLevel.Info,
    /** Routes the toolkit's logs into the host application's logger; console-only when left null. */
    val logAppender: ComposeLogAppender? = null,
)

val LocalComposeApplication = staticCompositionLocalOf<ComposeApplication> {
    error("No ComposeApplication provided")
}

/**
 * What the application composition can do to the application as a whole.
 *
 * Deliberately the same shape as Compose Desktop's `ApplicationScope`, so that `onCloseRequest =
 * ::exitApplication` means here what it means there and content can move between the two.
 */
interface ApplicationScope {
    /**
     * The running application: screens, theme, quit handlers — everything that is not per-window.
     *
     * Also reachable as [LocalComposeApplication] from anywhere in the composition; this is for the
     * content function itself, which has the scope in hand before it has provided any locals.
     */
    val application: ComposeApplication

    /**
     * Ends the application: the content leaves the composition, every window it opened is disposed, and
     * the [application] call returns.
     *
     * Not the same as [ComposeApplication.quit], which asks the *platform* to terminate and therefore
     * runs the quit handlers and each window's `onCloseRequest` first. This one is the decision those
     * handlers eventually lead to.
     */
    fun exitApplication()
}

/**
 * Runs a Compose application: boots the toolkit, opens the composition, and blocks until it is done.
 *
 * This is the whole entry point — the equivalent of Compose Desktop's `application { }`, and the reason
 * a `main` needs to know nothing about the platform it is on:
 *
 * ```
 * fun main() = application {
 *     Window(onCloseRequest = ::exitApplication) { ... }
 * }
 * ```
 *
 * Which native backend serves the windows is [DesktopBackend.default]'s decision, unless the caller has a
 * policy of its own and passes [backend]; the toolkit is torn down on the way out whether the composition
 * ended or threw. Returns when nothing is left to keep the application alive: the last window closed, or
 * [ApplicationScope.exitApplication] was called.
 */
fun application(
    params: ComposeApplicationParams = ComposeApplicationParams(),
    backend: DesktopBackend = DesktopBackend.default,
    content: @Composable ApplicationScope.() -> Unit,
) {
    initApplication(params, backend).use { application ->
        val scope = ApplicationScopeImpl(application)
        ApplicationExit.installedWhile(scope::exitApplication) {
            runApplication(application) {
                // Reading `isRunning` inside the composition is what makes `exitApplication` work: flipping
                // it recomposes this, the content leaves, and its windows go with it.
                if (scope.isRunning) {
                    scope.content()
                }
            }
        }
    }
}

/**
 * Ends the application the way [ApplicationScope.exitApplication] does, for the quit sequence, which has
 * no scope in hand and reaches this only when there is no window left to decide instead. A composition
 * driven by a bare [runApplication] has no such lever, and only its own caller can end it.
 */
internal fun requestApplicationExit(backend: DesktopBackend) {
    val exit = ApplicationExit.current
    if (exit == null) {
        logWarn(backend) { "Nothing to quit: this composition is not run by application()" }
        return
    }
    exit()
}

/** The exit lever [application] holds open for the duration of its composition. */
private object ApplicationExit {
    private val exit = AtomicReference<(() -> Unit)?>(null)

    val current: (() -> Unit)? get() = exit.get()

    fun <T> installedWhile(exit: () -> Unit, block: () -> T): T {
        check(this.exit.compareAndSet(null, exit)) { "Another application composition is already running" }
        try {
            return block()
        } finally {
            this.exit.compareAndSet(exit, null)
        }
    }
}

private class ApplicationScopeImpl(override val application: ComposeApplication) : ApplicationScope {
    var isRunning by mutableStateOf(true)
        private set

    override fun exitApplication() {
        isRunning = false
    }
}

/**
 * The one line worth logging before the first window: which backend was picked, and on what.
 *
 * The JVM identity is in here because the toolkit needs a recent one — the FFI it is built on is only
 * available from Java 22 — so a mismatch is a likely first suspect when a run fails early.
 */
internal fun startupInfo(backend: DesktopBackend): String {
    val platform = DesktopPlatform.current
    val windowSystem = if (platform == DesktopPlatform.Linux) ", window system: ${currentLinuxWindowSystem()}" else ""
    val javaVersion = System.getProperty("java.runtime.version", System.getProperty("java.version", "unknown"))
    return "Starting Compose application on $platform$windowSystem, backend: $backend " +
        "(Java $javaVersion, ${System.getProperty("java.vendor")})"
}

/**
 * Boots the native application for [backend] and blocks until it has finished launching, so that windows
 * can be created straight away.
 *
 * This is the single point where the backend implementation is chosen; adding a backend means adding a
 * branch here plus a [ComposeApplication] implementation next to [MacOsApplication], [LinuxApplication],
 * [GtkApplication] and [WindowsApplication].
 */
fun initApplication(
    params: ComposeApplicationParams = ComposeApplicationParams(),
    backend: DesktopBackend = DesktopBackend.default,
): ComposeApplication {
    val application = when (backend) {
        DesktopBackend.MacOs -> MacOsApplication(params)
        DesktopBackend.Wayland -> LinuxApplication(params)
        DesktopBackend.Gtk -> GtkApplication(params)
        DesktopBackend.Windows -> WindowsApplication(params)
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
        // The application composition drives no pixels of its own, so nothing rate-limits it: an
        // infinite animation outside any `Window` would spin `YieldFrameClock` at 100% CPU.
        withContext(YieldFrameClock + noWindowAnimationCoroutineContext { application.windows.isNotEmpty() }) {
            // Before the first composition, so that state written from an event handler invalidates it,
            // and undone at the very end: the observer it installs is global and outlives this call
            // otherwise.
            val snapshots = KdtSnapshotManager.activateSnapshots()
            try {
                // `coroutineScope` and not the two bare `launch`es: they return the moment they are
                // started, and the flush has to stay installed until they have both finished.
                coroutineScope {
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
            } finally {
                snapshots.dispose()
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
