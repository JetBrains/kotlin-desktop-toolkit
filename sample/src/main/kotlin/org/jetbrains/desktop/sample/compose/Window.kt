package org.jetbrains.desktop.sample.compose

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.currentCompositionLocalContext
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.SystemTheme
import androidx.compose.ui.draganddrop.DragAndDropTransferData
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpRect
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.awaitCancellation
import java.nio.file.Path

/** Why a window was asked to close, so that the application can react differently to a user Quit. */
enum class WindowCloseRequestReason {
    UserRequest,
    ApplicationQuit,
}

enum class WindowPlacement {
    Floating,
    Maximized,
    Fullscreen,
}

/** How much of the window frame the platform draws itself. */
sealed interface WindowDecoration {
    /** The platform titlebar. */
    data object Decorated : WindowDecoration

    /**
     * A titlebar of [height] that the platform keeps the traffic-light buttons in but otherwise leaves
     * to the content: the top [height] of the scene is drawn under it, and dragging or double-clicking
     * an area the content did not consume moves or maximizes the window.
     */
    data class CustomTitleBar(
        val height: Dp,
        val roundedCorners: Boolean = false,
    ) : WindowDecoration
}

/** Everything a window needs at creation time; the rest is settable afterwards through [ComposeWindow]. */
class ComposeWindowParams(
    val title: String = "Untitled",
    /** `Unspecified` lets the platform place the window. */
    val position: DpOffset = DpOffset.Unspecified,
    val size: DpSize = DpSize(800.dp, 600.dp),
    val minSize: DpSize = DpSize(320.dp, 240.dp),
    val isResizable: Boolean = true,
    val decoration: WindowDecoration = WindowDecoration.Decorated,
)

/**
 * A native window hosting a Compose scene.
 *
 * Every observable property is Compose state, so content can simply read `window.size` and be
 * recomposed on a resize. Mutating the window goes through `request*` methods rather than setters,
 * because the platform is free to answer with something else — or nothing at all — and the property
 * only changes once the platform reports back.
 *
 * All members must be touched on the event loop thread.
 */
interface ComposeWindow {
    var title: String

    /** Position of the window's top-left corner in the global desktop coordinate space. Observable. */
    val position: DpOffset

    /** Size of the whole window, titlebar included. Observable. */
    val size: DpSize

    /** Size of the area the content is drawn into. Observable. */
    val contentSize: DpSize

    val bounds: DpRect get() = DpRect(position, size)

    val minSize: DpSize
    val maxSize: DpSize
    val isResizable: Boolean

    /** Whether this window receives keyboard input. Observable. */
    val isFocused: Boolean

    /** Observable. */
    val placement: WindowPlacement

    /** The screen showing most of this window. Observable. */
    val screen: ComposeScreen

    /** Observable. */
    val density: Density

    val layoutDirection: LayoutDirection

    /** This window's theme: its own override if it has one, otherwise the application's. Observable. */
    val systemTheme: SystemTheme

    /** Observable. */
    val decoration: WindowDecoration

    /**
     * Insets the content has to keep clear when [decoration] is a [WindowDecoration.CustomTitleBar]:
     * `first` on the leading edge, `second` on the trailing edge, so that the content does not end up
     * underneath the platform's own titlebar buttons. `null` for a [WindowDecoration.Decorated] window.
     */
    val customTitleBarInsets: Pair<Dp, Dp>?

    /** The platform object behind this window, for code that needs to reach past this abstraction. */
    val nativeWindow: Any

    fun requestPosition(position: DpOffset)

    fun requestSize(size: DpSize)

    fun requestBounds(bounds: DpRect)

    fun requestMinSize(minSize: DpSize)

    fun requestMaxSize(maxSize: DpSize)

    fun requestResizable(resizable: Boolean)

    fun requestFocus()

    fun requestBringToFront()

    fun requestFocusAndBringToFront()

    fun requestPlacement(placement: WindowPlacement)

    fun requestMinimized(minimized: Boolean)

    /** Overrides the window's theme, or follows the application's again when passed `null`. */
    fun requestSystemTheme(systemTheme: SystemTheme?)

    fun requestDecoration(decoration: WindowDecoration)

    /** Asks this window's owner to close it, the same way the platform's own close button does. */
    fun requestClose()

    /**
     * Hands the window over to the platform's own move loop.
     *
     * Only valid from within a pointer event handler: the platform starts the move from the press that
     * originated it, so [pointerEvent] has to be the event currently being handled. macOS reads the current
     * event and ignores the argument; Wayland needs the press's serial and ignores an event that is not a
     * press, which is what makes this safe to call from a handler that sees every pointer event.
     */
    fun startInteractiveMove(pointerEvent: PointerEvent)

    /**
     * Shows a modal file-open dialog and returns the chosen paths, empty if the user cancelled.
     *
     * Must be called on the event loop thread. On macOS the dialog runs a nested run loop, so the call
     * blocks that thread until the dialog is dismissed — being `suspend` is what lets the caller's
     * coroutine wait for it, not a relaxation of the threading requirement.
     */
    suspend fun showOpenFileDialog(
        title: String = "Open",
        allowsMultipleSelections: Boolean = false,
        canChooseFiles: Boolean = true,
        canChooseDirectories: Boolean = false,
    ): List<Path>

    /** Shows a modal file-save dialog and returns the chosen path, or `null` if the user cancelled. */
    suspend fun showSaveFileDialog(title: String = "Save", nameFieldStringValue: String? = null): Path?

    fun setContent(
        onPreviewKeyEvent: (KeyEvent) -> Boolean,
        onKeyEvent: (KeyEvent) -> Boolean,
        content: @Composable ComposeWindowScope.() -> Unit,
    )

    /** Closes the window and releases its native resources. Called by the [Window] composable. */
    fun dispose()
}

/** Receiver of a window's content, so the content can look at the window hosting it. */
interface ComposeWindowScope {
    val window: ComposeWindow
    val application: ComposeApplication get() = currentApplication()
}

/** The window hosting the current composition. Provided by each window's scene. */
val LocalComposeWindow = staticCompositionLocalOf<ComposeWindow> {
    error("No ComposeWindow provided, this composable has to live inside a Window {}")
}

/**
 * A source of drag-and-drop sessions. Kept apart from [ComposeWindow] because starting a session needs
 * the platform's own transferable representation, which is not part of the window abstraction.
 */
internal interface DragAndDropSourceWindow {
    fun startDragSession(
        offset: Offset,
        transferData: DragAndDropTransferData,
        decorationSize: Size,
        drawDragDecoration: DrawScope.() -> Unit,
    )
}

/**
 * Opens a native window for as long as this composable stays in the composition.
 *
 * Closing it is up to the caller: [onCloseRequest] fires when the user clicks the close button or the
 * application is quitting, and the window goes away once the caller stops emitting this composable.
 *
 * [title], [decoration] and anything set from [configure] are applied on every recomposition, so they
 * can be driven by state; the remaining creation parameters are only read once, because the platform
 * owns them from then on.
 */
@Composable
fun Window(
    title: String = "Untitled",
    size: DpSize = DpSize(800.dp, 600.dp),
    position: DpOffset = DpOffset.Unspecified,
    minSize: DpSize = DpSize(320.dp, 240.dp),
    isResizable: Boolean = true,
    decoration: WindowDecoration = WindowDecoration.Decorated,
    onCloseRequest: (WindowCloseRequestReason) -> Unit,
    onPreviewKeyEvent: (KeyEvent) -> Boolean = { false },
    onKeyEvent: (KeyEvent) -> Boolean = { false },
    configure: ComposeWindow.() -> Unit = {},
    content: @Composable ComposeWindowScope.() -> Unit,
) {
    val application = LocalComposeApplication.current
    // The callbacks can change between recompositions while the window itself must not be recreated.
    val currentOnCloseRequest by rememberUpdatedState(onCloseRequest)
    val currentOnPreviewKeyEvent by rememberUpdatedState(onPreviewKeyEvent)
    val currentOnKeyEvent by rememberUpdatedState(onKeyEvent)
    val window = remember {
        application.createWindow(
            params = ComposeWindowParams(
                title = title,
                position = position,
                size = size,
                minSize = minSize,
                isResizable = isResizable,
                decoration = decoration,
            ),
            onCloseRequest = { reason -> currentOnCloseRequest(reason) },
        )
    }
    DisposableEffect(Unit) {
        onDispose {
            window.dispose()
        }
    }
    SideEffect {
        window.title = title
        window.requestDecoration(decoration)
        window.configure()
    }

    // Carried into the window's own composition so that locals provided around `Window` — a theme, a
    // repository — reach the content, which a separate ComposeScene would otherwise not see.
    val parentLocals by rememberUpdatedState(currentCompositionLocalContext)
    // Setting content from an effect that never completes keeps the Recomposer from deciding the
    // composition is finished while this window is still on screen.
    LaunchedEffect(window) {
        window.setContent(
            onPreviewKeyEvent = { currentOnPreviewKeyEvent(it) },
            onKeyEvent = { currentOnKeyEvent(it) },
        ) {
            CompositionLocalProvider(parentLocals) {
                content()
            }
        }
        awaitCancellation()
    }
}
