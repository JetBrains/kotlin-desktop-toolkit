@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.compose.windows

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.LocalSystemTheme
import androidx.compose.ui.SystemTheme
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.UriHandler
import androidx.compose.ui.unit.LayoutDirection
import org.jetbrains.desktop.compose.ComposeApplication
import org.jetbrains.desktop.compose.ComposeApplicationParams
import org.jetbrains.desktop.compose.ComposeWindow
import org.jetbrains.desktop.compose.ComposeWindowParams
import org.jetbrains.desktop.compose.WindowCloseRequestReason
import org.jetbrains.desktop.compose.forgetApplication
import org.jetbrains.desktop.win32.Appearance
import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.jetbrains.desktop.win32.Logger
import org.jetbrains.desktop.win32.Screen
import org.jetbrains.desktop.win32.WindowId
import org.jetbrains.desktop.win32.openURL
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import kotlin.concurrent.thread

/**
 * Owns the win32 application: the message loop, and every open window.
 *
 * The message loop has to own a thread of its own, and that thread has to be the one every native call
 * is made from — it is the thread whose message queue the windows belong to, and the process's OLE
 * apartment (`CoInitialize` as an STA) is initialized on it, which is what the clipboard and drag and
 * drop require. `main` cannot be that thread, because it is busy running the composition, so the loop
 * gets a dedicated one and everything Compose does is dispatched back onto it through
 * [org.jetbrains.desktop.compose.KdtDispatcher].
 *
 * Unlike AppKit, win32 has no "application" object with a lifecycle of its own: there is no launch
 * notification and no termination request, so [CountDownLatch] and a run-once startup callback stand in
 * for the first, and [quit] simply asks every window to close for the second.
 */
internal class WindowsApplication(params: ComposeApplicationParams) : ComposeApplication {
    // Must run before anything else in this class: every other native call needs the library loaded.
    init {
        KotlinDesktopToolkit.init()
        // Neither creation parameter has a win32 counterpart, so both are only logged: the identifier is
        // matched against a `.desktop` file on Linux and comes from the bundle on macOS, and nothing in
        // the toolkit's Windows backend reports URLs handed to the application, so `onOpenUris` is never
        // called.
        Logger.info { "Starting Compose application ${params.identifier}" }
    }

    override val nativeApplication: Application = Application()

    private val windowsById = mutableMapOf<WindowId, WindowsWindow>()
    private val windowList = mutableStateListOf<WindowsWindow>()
    private val applicationStarted = CountDownLatch(1)
    private val quitHandlers = ConcurrentHashMap<String, () -> Boolean>()

    @Volatile
    private var isQuitInProgress = false

    private var hasShownAWindow = false

    override var systemTheme: SystemTheme by mutableStateOf(SystemTheme.Unknown)
        private set

    override var screens: List<WindowsScreen> by mutableStateOf(emptyList())
        private set

    override val windows: List<ComposeWindow> get() = windowList

    override val focusedWindow: ComposeWindow? get() = windowList.firstOrNull { it.isFocused }

    /**
     * The screen used for anything composed outside a window. Reads the observable [screens] list, so
     * that a display change recomposes; the primary screen stands in for "wherever the user is", which
     * is all this is needed for.
     */
    override val mainScreen: WindowsScreen
        get() = screens.firstOrNull { it.nativeScreen.isPrimary }
            ?: screens.firstOrNull()
            ?: WindowsScreen(Screen.allScreens().first())

    /**
     * Whether one of this application's windows has the keyboard.
     *
     * Windows has no notion of an active *application* — activation belongs to a window — so this is the
     * closest equivalent, and the same answer the frontmost-application question would give.
     */
    override val isActive: Boolean get() = windowList.any { it.isFocused }

    /** Ltr: win32 exposes no reading order for the user's locale, unlike AppKit and Wayland. */
    override val layoutDirection: LayoutDirection = LayoutDirection.Ltr

    internal val uriHandler: UriHandler = object : UriHandler {
        override fun openUri(uri: String) {
            Application.openURL(uri)
        }
    }

    internal val clipboard = WindowsClipboard(this)

    private val eventLoopThread: Thread

    // ----- Event loop -----

    private fun startEventLoopThread(): Thread = thread(start = true, name = "KDT Event Loop") {
        try {
            // win32 has no "did finish launching" event, so the toolkit runs a queue of startup blocks on
            // the loop's own thread once it is up and before the first message is dispatched. This is the
            // earliest point at which the native API may be touched.
            nativeApplication.onStartup {
                Thread.currentThread().name = "KDT Main Thread"
                screens = readScreens()
                systemTheme = Appearance.getCurrent().toComposeSystemTheme()
                applicationStarted.countDown()
            }
            nativeApplication.runEventLoop(::handleEvent)
        } catch (throwable: Throwable) {
            Logger.error(throwable) { "The event loop has stopped" }
        } finally {
            // Whatever happened, nobody may be left waiting for a launch that is not coming.
            applicationStarted.countDown()
            nativeApplication.close()
        }
    }

    private fun handleEvent(windowId: WindowId, event: Event): EventHandlerResult {
        return try {
            when (event) {
                is Event.SystemAppearanceChange -> {
                    systemTheme = event.newAppearance.toComposeSystemTheme()
                    // A window with no override of its own follows the application, and its native frame
                    // has to be told about it.
                    windowList.forEach { it.onApplicationSystemThemeChanged() }
                    EventHandlerResult.Stop
                }

                else -> {
                    // The toolkit reports no display-configuration event, and a scale change is the one
                    // moment a window is guaranteed to hear about a monitor change — so the screen list
                    // is refreshed from there.
                    if (event is Event.WindowScaleChanged) {
                        screens = readScreens()
                    }
                    // A window whose id no longer resolves — mid-creation, or already disposed — lets the
                    // toolkit's default handling run.
                    windowsById[windowId]?.handleEvent(event) ?: EventHandlerResult.Continue
                }
            }
        } catch (throwable: Throwable) {
            // A handler that throws must not take the event loop with it.
            Logger.error(throwable) { "Failed to handle event $event" }
            EventHandlerResult.Continue
        }
    }

    private fun readScreens(): List<WindowsScreen> = Screen.allScreens().filter { it.isAttached }.map(::WindowsScreen)

    // ----- Windows -----

    override fun createWindow(params: ComposeWindowParams, onCloseRequest: (WindowCloseRequestReason) -> Unit): ComposeWindow {
        return WindowsWindow(this, params, onCloseRequest)
    }

    internal fun attachWindow(window: WindowsWindow) {
        windowsById[window.windowId] = window
        windowList.add(window)
    }

    internal fun detachWindow(window: WindowsWindow) {
        windowsById.remove(window.windowId)
        windowList.remove(window)
    }

    /**
     * Brings the first window of the process to the front.
     *
     * Windows refuses to let a process that does not own the foreground activate a window, and an
     * application launched from a terminal or an IDE does not own it — its first window would open behind
     * whatever the user was looking at. Only the first one: a window opened later should follow the
     * system's foreground policy rather than interrupt whatever the user has moved on to.
     */
    internal fun onWindowFirstShown(window: WindowsWindow) {
        if (!hasShownAWindow) {
            hasShownAWindow = true
            window.requestFocus()
        }
    }

    // ----- Application-level API -----

    /** Nothing to do: on Windows activation belongs to a window, and [ComposeWindow.requestFocus] is it. */
    override fun requestActivation() = Unit

    /**
     * Runs the same path as the user closing every window, because on Windows that *is* quitting: there
     * is no application-level termination request to make, and the process ends when the composition has
     * no windows left to emit and `runApplication` returns.
     */
    override fun quit() {
        if (isQuitInProgress) {
            return
        }
        // Every handler is consulted, not just up to the first veto: a handler may be showing a dialog
        // and needs to know a quit was attempted even if another handler already said no.
        val isSafeToQuit = quitHandlers.values.fold(true) { acc, handler -> handler() and acc }
        if (!isSafeToQuit) {
            return
        }
        val openWindows = windowList.toList()
        if (openWindows.isEmpty()) {
            return
        }
        isQuitInProgress = true
        Logger.info { "Quit approved, asking ${openWindows.size} window(s) to close" }
        openWindows.forEach { it.requestCloseFromQuit() }
    }

    override fun putQuitHandler(id: String, isSafeToQuit: () -> Boolean) {
        quitHandlers[id] = isSafeToQuit
    }

    override fun removeQuitHandler(id: String) {
        quitHandlers.remove(id)
    }

    override fun openUri(uri: String) {
        uriHandler.openUri(uri)
    }

    override fun invokeOnUiThread(block: () -> Unit) {
        nativeApplication.invokeOnDispatcher(block)
    }

    override fun isUiThread(): Boolean = nativeApplication.isDispatcherThread()

    @Composable
    override fun ProvideCompositionLocals(content: @Composable () -> Unit) {
        CompositionLocalProvider(
            LocalSystemTheme provides systemTheme,
            LocalClipboard provides clipboard,
            LocalUriHandler provides uriHandler,
        ) {
            content()
        }
    }

    override fun close() {
        // Posted rather than called: `stopEventLoop` has to run on the thread whose message queue it is
        // stopping.
        nativeApplication.invokeOnDispatcher { nativeApplication.stopEventLoop() }
        eventLoopThread.join()
        forgetApplication(this)
    }

    init {
        eventLoopThread = startEventLoopThread()
        // Returns once the loop is up, so windows can be created straight away.
        applicationStarted.await()
    }
}

internal fun Appearance.toComposeSystemTheme(): SystemTheme = when (this) {
    Appearance.Dark -> SystemTheme.Dark
    Appearance.Light -> SystemTheme.Light
}
