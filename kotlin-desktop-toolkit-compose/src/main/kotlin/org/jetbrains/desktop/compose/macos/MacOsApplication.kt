@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.compose.macos

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
import androidx.compose.ui.input.InputMode
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.UriHandler
import androidx.compose.ui.unit.LayoutDirection
import org.jetbrains.desktop.compose.ComposeApplication
import org.jetbrains.desktop.compose.ComposeApplicationParams
import org.jetbrains.desktop.compose.ComposeWindow
import org.jetbrains.desktop.compose.ComposeWindowParams
import org.jetbrains.desktop.compose.KdtInputModeManager
import org.jetbrains.desktop.compose.WindowCloseRequestReason
import org.jetbrains.desktop.compose.forgetApplication
import org.jetbrains.desktop.macos.Appearance
import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Cursor
import org.jetbrains.desktop.macos.DragAndDropHandler
import org.jetbrains.desktop.macos.DragInfo
import org.jetbrains.desktop.macos.DragOperation
import org.jetbrains.desktop.macos.DragOperationsSet
import org.jetbrains.desktop.macos.DragSourceCallbacks
import org.jetbrains.desktop.macos.DragTargetCallbacks
import org.jetbrains.desktop.macos.DraggingContext
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Screen
import org.jetbrains.desktop.macos.TextDirection
import org.jetbrains.desktop.macos.WindowEvent
import org.jetbrains.desktop.macos.WindowId
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import kotlin.concurrent.thread

/**
 * Owns the AppKit application: the event loop, the shared Metal device, and every open window.
 *
 * The kotlin-desktop-toolkit event loop must own the real macOS main thread. The JVM launcher does not
 * run `main` on that thread, so [GrandCentralDispatch.startOnMainThread] hands the loop over to it and
 * blocks its caller for as long as the loop runs — hence the dedicated thread below. Everything Compose
 * does is then dispatched back onto that same thread through
 * [org.jetbrains.desktop.compose.KdtDispatcher].
 */
internal class MacOsApplication(private val params: ComposeApplicationParams) : ComposeApplication {
    // Must run before anything else in this class: every other native call needs the library loaded.
    init {
        KotlinDesktopToolkit.init()
    }

    private val windowsById = mutableMapOf<WindowId, MacOsWindow>()
    private val windowList = mutableStateListOf<MacOsWindow>()
    private val applicationStarted = CountDownLatch(1)
    private val quitHandlers = ConcurrentHashMap<String, () -> Boolean>()

    @Volatile
    private var isQuitInProgress = false

    // Deferred so the Metal device is created on the main thread, once the event loop is up.
    private val lazyGpuContext = lazy { GpuContext() }
    internal val gpuContext: GpuContext get() = lazyGpuContext.value

    override var systemTheme: SystemTheme by mutableStateOf(SystemTheme.Unknown)
        private set

    override var screens: List<MacOsScreen> by mutableStateOf(emptyList())
        private set

    override val windows: List<ComposeWindow> get() = windowList

    override val focusedWindow: ComposeWindow? get() = windowList.firstOrNull { it.isFocused }

    /**
     * The screen used for anything composed outside a window. Reads the observable [screens] list, so
     * that a display change recomposes; the primary screen stands in for "wherever the user is", which
     * is all this is needed for.
     */
    override val mainScreen: MacOsScreen
        get() = screens.firstOrNull { it.nativeScreen.isPrimary }
            ?: screens.firstOrNull()
            ?: MacOsScreen(Screen.allScreens().mainScreen())

    override val isActive: Boolean get() = Application.isActive

    override val nativeApplication: Any get() = Application

    override val layoutDirection: LayoutDirection
        get() = when (Application.textDirection) {
            TextDirection.LeftToRight -> LayoutDirection.Ltr
            TextDirection.RightToLeft -> LayoutDirection.Rtl
        }

    internal val uriHandler: UriHandler = object : UriHandler {
        override fun openUri(uri: String) {
            Application.openURL(uri)
        }
    }

    /**
     * Application-wide rather than per-window: whether the user is driving the UI by keyboard or by
     * pointer is a property of the user, and macOS hides the cursor while they type.
     */
    internal val inputModeManager = KdtInputModeManager(InputMode.Touch) { inputMode ->
        Cursor.setHiddenUntilMouseMoves(inputMode == InputMode.Keyboard)
    }

    private val eventLoopThread: Thread

    // ----- Event loop -----

    private fun startEventLoopThread(): Thread = thread(start = true, name = "KDT Event Loop") {
        GrandCentralDispatch.startOnMainThread {
            Application.init()
            Thread.currentThread().name = "KDT Main Thread"
            systemTheme = Application.appearance.toComposeSystemTheme()
            screens = readScreens()
            Application.setQuitHandler(::onQuitRequested)
            DragAndDropHandler.init(dragTargetCallbacks, dragSourceCallbacks)

            Application.runEventLoop(::handleEvent)

            if (lazyGpuContext.isInitialized()) {
                lazyGpuContext.value.close()
            }
            GrandCentralDispatch.close()
        }
    }

    private fun handleEvent(event: Event): EventHandlerResult {
        return try {
            when (event) {
                is WindowEvent -> {
                    if (event.isPointerMovement()) {
                        // The cursor was hidden because the user started typing; they are back on the
                        // pointer now, so stop hiding it.
                        Cursor.setHiddenUntilMouseMoves(false)
                    }
                    windowsById[event.windowId]?.handleEvent(event) ?: EventHandlerResult.Continue
                }

                is Event.ApplicationDidFinishLaunching -> {
                    applicationStarted.countDown()
                    EventHandlerResult.Stop
                }

                is Event.ApplicationAppearanceChange -> {
                    systemTheme = event.newAppearance.toComposeSystemTheme()
                    EventHandlerResult.Stop
                }

                is Event.DisplayConfigurationChange -> {
                    screens = readScreens()
                    EventHandlerResult.Stop
                }

                is Event.ApplicationOpenUrls -> {
                    params.onOpenUris(event.urls)
                    EventHandlerResult.Stop
                }

                else -> EventHandlerResult.Continue
            }
        } catch (throwable: Throwable) {
            // A handler that throws must not take the event loop with it; let the event fall through to
            // AppKit's own handling instead.
            Logger.error { "Failed to handle event $event: $throwable" }
            EventHandlerResult.Continue
        }
    }

    /**
     * Runs when the user chooses Quit (or something calls [quit]).
     *
     * Returning `false` cancels AppKit's own termination, which is what we want in every branch: either
     * a handler vetoed, or there are windows to close first. Closing them goes through each window's
     * `onCloseRequest`, so the application decides — and the composition, once it stops emitting those
     * windows, is what ends `runApplication` and unwinds the process from `main`.
     */
    private fun onQuitRequested(): Boolean {
        // Every handler is consulted, not just up to the first veto: a handler may be showing a dialog
        // and needs to know a quit was attempted even if another handler already said no.
        val isSafeToQuit = quitHandlers.values.fold(true) { acc, handler -> handler() and acc }
        if (!isSafeToQuit || isQuitInProgress) {
            return false
        }
        val openWindows = windowList.toList()
        if (openWindows.isEmpty()) {
            return true
        }
        isQuitInProgress = true
        Logger.info { "Quit approved, asking ${openWindows.size} window(s) to close" }
        openWindows.forEach { it.requestCloseFromQuit() }
        return false
    }

    private fun readScreens(): List<MacOsScreen> = Screen.allScreens().screens.map(::MacOsScreen)

    // ----- Drag and drop -----
    //
    // AppKit reports drags application-wide; each callback carries the window it belongs to, so all this
    // layer does is resolve that window and hand the drag to its own manager.

    private val dragTargetCallbacks = object : DragTargetCallbacks {
        override fun onDragEntered(info: DragInfo): DragOperation {
            return windowFor(info.destinationWindowId)?.dragAndDropManager?.onDragEntered(info) ?: DragOperation.NONE
        }

        override fun onDragExited(info: DragInfo?) {
            val windowId = info?.destinationWindowId ?: return
            windowFor(windowId)?.dragAndDropManager?.onDragExited()
        }

        override fun onDragPerformed(info: DragInfo): Boolean {
            return windowFor(info.destinationWindowId)?.dragAndDropManager?.onDragPerformed(info) ?: false
        }

        override fun onDragUpdated(info: DragInfo): DragOperation {
            return windowFor(info.destinationWindowId)?.dragAndDropManager?.onDragUpdated(info) ?: DragOperation.NONE
        }
    }

    private val dragSourceCallbacks = object : DragSourceCallbacks {
        override fun onDragSourceOperationMask(
            sourceWindowId: WindowId,
            sequenceNumber: Long,
            context: DraggingContext,
        ): DragOperationsSet {
            val data = windowFor(sourceWindowId)?.activeDragAndDropTransferData ?: return DragOperationsSet.NONE
            return data.supportedActions
                .map { action -> DragOperationsSet.of(action.toDragOperation()) }
                .fold(DragOperationsSet.NONE) { acc, set -> acc + set }
        }

        override fun onDragSourceSessionEndedAt(
            sourceWindowId: WindowId,
            sequenceNumber: Long,
            locationOnScreen: LogicalPoint,
            dragOperation: DragOperation,
        ) {
            val window = windowFor(sourceWindowId) ?: return
            window.activeDragAndDropTransferData?.onTransferCompleted?.invoke(dragOperation.toDragAndDropTransferAction())
            window.activeDragAndDropTransferData = null
        }
    }

    private fun windowFor(windowId: WindowId): MacOsWindow? = windowsById[windowId]

    // ----- Windows -----

    override fun createWindow(params: ComposeWindowParams, onCloseRequest: (WindowCloseRequestReason) -> Unit): ComposeWindow {
        return MacOsWindow(this, params, onCloseRequest)
    }

    internal fun attachWindow(window: MacOsWindow) {
        windowsById[window.windowId] = window
        windowList.add(window)
    }

    internal fun detachWindow(window: MacOsWindow) {
        windowsById.remove(window.windowId)
        windowList.remove(window)
    }

    // ----- Application-level API -----

    override fun requestActivation() {
        Application.activateIgnoringOtherApps()
    }

    override fun quit() {
        Application.requestTermination()
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
        GrandCentralDispatch.dispatchOnMain { block() }
    }

    override fun isUiThread(): Boolean = GrandCentralDispatch.isMainThread()

    @Composable
    override fun ProvideCompositionLocals(content: @Composable () -> Unit) {
        CompositionLocalProvider(
            LocalSystemTheme provides systemTheme,
            LocalClipboard provides MacOsClipboard,
            LocalUriHandler provides uriHandler,
        ) {
            content()
        }
    }

    override fun close() {
        GrandCentralDispatch.dispatchOnMain {
            Application.stopEventLoop()
        }
        eventLoopThread.join()
        forgetApplication(this)
    }

    init {
        // Started from the last initializer in the class, and not from a property initializer: the
        // thread body runs immediately and reads the drag-and-drop callbacks declared below it, which
        // a property initializer here would leave null.
        eventLoopThread = startEventLoopThread()
        // Returns once AppKit reports the application has launched, so windows can be created.
        applicationStarted.await()
    }
}

private fun WindowEvent.isPointerMovement(): Boolean =
    this is Event.MouseMoved || this is Event.MouseDragged || this is Event.MouseEntered || this is Event.MouseExited

internal fun Appearance.toComposeSystemTheme(): SystemTheme = when (this) {
    Appearance.Dark -> SystemTheme.Dark
    Appearance.Light -> SystemTheme.Light
}
