@file:OptIn(ExperimentalComposeUiApi::class)

package org.jetbrains.desktop.sample.compose.windows

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.pointer.isAltPressed
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.unit.Density
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.withContext
import org.jetbrains.desktop.sample.compose.KdtImmediateDispatcher
import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.LogicalPoint
import org.jetbrains.desktop.win32.LogicalRect
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.TextCompositionSegment
import org.jetbrains.desktop.win32.TextInputClient
import org.jetbrains.desktop.win32.Window
import androidx.compose.ui.text.TextRange as ComposeTextRange
import org.jetbrains.desktop.win32.TextRange as KdtTextRange

/**
 * Input-method support for one window: the bridge between Compose's text field and IMM32.
 *
 * This is what makes dead keys and CJK candidate windows work, and — unlike macOS — it is also what
 * makes plain typing work at all. Windows does not hand an application the character a key produced; it
 * hands it the key, and the character only appears once the message has been through `TranslateMessage`,
 * which posts a `WM_CHAR` back. So a key that produces text is *given* to the IME
 * ([offerKeyDownToInputMethod]) and its text is collected later from the resulting
 * [Event.CharacterReceived] ([tryHandleTextInputEvent]).
 *
 * Compose starts and ends a session through `PlatformContext.startInputMethod`, so the session's
 * lifetime is the lifetime of a focused text field, and only one can be active at a time.
 */
internal class WindowsTextInput(
    private val nativeWindow: Window,
    private val density: () -> Density,
) {
    private var activeClient: WindowsTextInputClient? = null

    /**
     * Set once the native window is on its way out. Every call below goes through the native window, and
     * a destroyed one must not be touched.
     */
    private var isWindowGone = false

    /** Whether a Compose text field currently holds the input session. */
    val isSessionActive: Boolean get() = activeClient != null

    /**
     * Installs [request] as the window's text input client and keeps it installed until the calling
     * coroutine is cancelled, which is how Compose ends a session.
     */
    suspend fun startInputMethod(request: PlatformTextInputMethodRequest): Nothing {
        withContext(KdtImmediateDispatcher) {
            val client = WindowsTextInputClient(request, nativeWindow, density)
            nativeWindow.setTextInputClient(client)
            activeClient = client
        }
        try {
            awaitCancellation()
        } finally {
            // NonCancellable: the coroutine is already cancelled by the time we get here, so a plain
            // withContext would refuse to run and the native client would stay installed.
            withContext(NonCancellable + KdtImmediateDispatcher) {
                activeClient = null
                if (!isWindowGone) {
                    // Clearing the client finalizes an open composition, which calls back into it — hence
                    // this order, and hence the toolkit swapping the recipient only afterwards.
                    nativeWindow.clearTextInputClient()
                    nativeWindow.notifySelectionChanged()
                }
            }
        }
    }

    /** Must be called before the native window is destroyed; see [isWindowGone]. */
    fun onWindowClosing() {
        isWindowGone = true
    }

    /**
     * Offers a key press to the input method, returning whether it took it.
     *
     * Taking it means handing the original message to `TranslateMessage`: Windows then decides what the
     * key means for the active layout and input method — a plain character, a dead key waiting for the
     * next one, a candidate list — and posts the result back as [Event.CharacterReceived] or a
     * composition message.
     *
     * Only keys that would produce text are offered, which is what keeps navigation, editing and
     * shortcuts flowing to Compose instead: a control character (Enter, Tab, Backspace) is an editing
     * command, and a key held with Ctrl, Alt or Win is a shortcut — except for Ctrl+Alt, which is how
     * AltGr reaches the layouts that type real characters with it.
     */
    fun offerKeyDownToInputMethod(event: Event.KeyDown): Boolean {
        if (activeClient == null) {
            return false
        }
        val producesText = event.toUnicode().firstOrNull()?.isISOControl() == false
        if (!producesText) {
            return false
        }
        val modifiers = currentKeyboardModifiers()
        val isAltGraph = modifiers.isCtrlPressed && modifiers.isAltPressed
        val isShortcut = modifiers.isMetaPressed || ((modifiers.isCtrlPressed || modifiers.isAltPressed) && !isAltGraph)
        return !isShortcut && event.translate()
    }

    /**
     * First refusal on the native text events, offered at the top of the window's event handler.
     *
     * A committed character is inserted straight away; a dead-key character opens a composition instead,
     * so that the accent is shown while the next key decides what it becomes. Returns whether the event
     * was consumed.
     */
    fun tryHandleTextInputEvent(event: Event): Boolean {
        if (event !is Event.CharacterReceived) {
            return false
        }
        val client = activeClient ?: return false
        // Control characters are Compose's editing commands, and a system key (Alt held) is a menu
        // accelerator; neither is text.
        if (event.character.isISOControl() || event.isSystemKey) {
            return false
        }
        if (event.isDeadChar) {
            client.setMarkedText(event.character.toString(), selectedRange = null, segments = emptyList())
        } else {
            client.insertText(event.character.toString())
        }
        return true
    }
}

/**
 * Answers IMM32's questions out of a Compose [PlatformTextInputMethodRequest].
 *
 * Coordinates: Compose works in pixels relative to the scene root, and on win32 the scene root is the
 * window's whole client area, so a rectangle only has to be converted from pixels to logical units —
 * there is no content origin to offset by, unlike macOS.
 */
private class WindowsTextInputClient(
    private val request: PlatformTextInputMethodRequest,
    private val nativeWindow: Window,
    private val density: () -> Density,
) : TextInputClient {
    override fun selectedRange(): KdtTextRange = request.state.selection.toKdtTextRange()

    override fun caretRect(range: KdtTextRange): LogicalRect {
        val rect = firstRectInRoot(range.toComposeTextRange())
        return with(density()) {
            LogicalRect(
                origin = LogicalPoint(rect.left.toDp().value, rect.top.toDp().value),
                size = LogicalSize(rect.width.toDp().value, rect.height.toDp().value),
            )
        }
    }

    override fun insertText(text: String) {
        request.editText { commitText(text, 1) }
        changed()
    }

    override fun setMarkedText(text: String, selectedRange: KdtTextRange?, segments: List<TextCompositionSegment>) {
        // `setComposingText` already replaces the current composition, or the selection when there is
        // none, which is what an IME means by marked text with no replacement range of its own.
        //
        // The composition's own clauses ([segments]) are dropped: the released text-editing API can set
        // the composing text but not style parts of it, so a multi-clause conversion shows no per-clause
        // highlighting. The caret [selectedRange] inside the composition goes the same way.
        request.editText { setComposingText(text, 1) }
        changed()
    }

    override fun unmarkText() {
        // Keeps the composed text as it stands, which is what an IME asks for when it commits.
        request.editText { finishComposingText() }
        changed()
    }

    override fun discardMarkedText() {
        // Abandons the composition instead of committing it: the composing text is replaced by nothing,
        // and only then is the composing region cleared.
        request.editText {
            setComposingText("", 1)
            finishComposingText()
        }
        changed()
    }

    /**
     * Tells the IME that the text moved out from under it.
     *
     * The candidate window is positioned from [caretRect], and IMM32 only asks again when it is told to,
     * so an insertion that pushed the caret along would otherwise leave the window behind.
     */
    private fun changed() {
        nativeWindow.notifySelectionChanged()
    }

    /**
     * The rectangle around the part of [range] that lies on its first line — an IME asks about ranges
     * that may wrap, and wants somewhere to put a single popup.
     */
    private fun firstRectInRoot(range: ComposeTextRange): Rect {
        val layout = request.textLayoutResult() ?: return Rect.Zero
        val textOriginInRoot = request.unclippedTextOffsetInRoot() ?: Offset.Zero
        val textLength = layout.layoutInput.text.length
        val start = range.start.coerceIn(0, textLength)
        val line = layout.getLineForOffset(start)
        val end = range.end.coerceIn(start, layout.getLineEnd(line, visibleEnd = true))
        val rect = Rect(
            left = layout.getHorizontalPosition(start, usePrimaryDirection = true),
            top = layout.getLineTop(line),
            right = layout.getHorizontalPosition(end, usePrimaryDirection = true),
            bottom = layout.getLineBottom(line),
        )
        return rect.translate(textOriginInRoot)
    }
}

private fun ComposeTextRange.toKdtTextRange(): KdtTextRange = KdtTextRange(start.toLong(), length.toLong())

private fun KdtTextRange.toComposeTextRange(): ComposeTextRange = ComposeTextRange(location.toInt(), (location + length).toInt())
