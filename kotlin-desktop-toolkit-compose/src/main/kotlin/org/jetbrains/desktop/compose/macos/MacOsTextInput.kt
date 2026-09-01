@file:OptIn(ExperimentalComposeUiApi::class)

package org.jetbrains.desktop.compose.macos

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.text.input.CommitTextCommand
import androidx.compose.ui.text.input.SetComposingTextCommand
import androidx.compose.ui.text.input.SetSelectionCommand
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpRect
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.withContext
import org.jetbrains.desktop.compose.KdtImmediateDispatcher
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.TextInputClient
import org.jetbrains.desktop.macos.Window
import androidx.compose.ui.text.TextRange as ComposeTextRange
import org.jetbrains.desktop.macos.TextRange as KdtTextRange

/**
 * Input-method support for one window: the bridge between Compose's text field and `NSTextInputClient`.
 *
 * This is what makes dead keys, accent popovers and CJK candidate windows work. macOS does not hand an
 * application "the typed character"; it hands the application's input context a key event and then calls
 * back into the focused text object — asking what is selected, where a character is on screen, and
 * telling it to insert text or to mark text as in-composition. [MacOsTextInputClient] answers those calls
 * from the Compose text field that owns the current session.
 *
 * Compose starts and ends a session through `PlatformContext.startInputMethod`, so its lifetime is the
 * lifetime of a focused text field, and only one can be active at a time.
 */
/** The keys an input method drives its own UI with: a candidate list, or the accent popover. */
private val imeNavigationKeys = setOf(
    KeyCode.LeftArrow,
    KeyCode.RightArrow,
    KeyCode.UpArrow,
    KeyCode.DownArrow,
    KeyCode.Escape,
    KeyCode.Return,
)

internal class MacOsTextInput(
    private val nativeWindow: Window,
    private val density: () -> Density,
) {
    private var activeClient: MacOsTextInputClient? = null

    /**
     * Set once the native window is on its way out. Every call below goes through the native window, and
     * a closed one throws rather than tolerating it.
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
            val client = MacOsTextInputClient(request, nativeWindow, density)
            nativeWindow.setTextInputClient(client)
            // AppKit activates the first responder's context by itself when the window becomes key, but
            // not when focus moves between fields inside one window, and not for a context this code
            // deactivated when the previous session ended.
            nativeWindow.textInputContext.activate()
            activeClient = client
        }
        try {
            awaitCancellation()
        } finally {
            // NonCancellable: the coroutine is already cancelled by the time we get here, so a plain
            // withContext would refuse to run and the native client would stay installed.
            withContext(NonCancellable + KdtImmediateDispatcher) {
                val client = activeClient
                activeClient = null
                // Closing the window ends the session, but cancellation reaches this coroutine through
                // the scene's dispatcher, so it can land after `Window.close` — which already discards
                // the composition and drops the client itself.
                if (!isWindowGone) {
                    nativeWindow.textInputContext.discardMarkedText()
                    // `discardMarkedText` abandons the composition on AppKit's side only: without this the
                    // Compose field keeps its composing range, so the underline outlives the focus and the
                    // next edit works against a stale range.
                    client?.unmarkText()
                    // Otherwise this window's context stays the application's active one, leaving a CJK
                    // input method armed with nothing editable under it.
                    nativeWindow.textInputContext.deactivate()
                    nativeWindow.setTextInputClient(TextInputClient.Noop)
                    // The cached character coordinates point into a field that is gone.
                    nativeWindow.textInputContext.invalidateCharacterCoordinates()
                }
            }
        }
    }

    /** Must be called before the native window is closed; see [isWindowGone]. */
    fun onWindowClosing() {
        isWindowGone = true
    }

    /** Whether a composition is currently open, i.e. the text field holds provisional marked text. */
    fun hasMarkedText(): Boolean = activeClient?.hasMarkedText() == true

    /**
     * Whether this key has to be offered to the input method before Compose sees it.
     *
     * A composition in progress is the obvious case. The other is macOS's press-and-hold accent
     * popover, which uses **no marked text at all** — the base character is committed and the popover
     * then replaces it — so the only sign that the input method wants the arrows is the keys themselves.
     * Offering them afterwards is too late: Compose consumes an arrow as caret movement and the popover
     * never sees it.
     */
    fun ownsKeyDown(event: Event.KeyDown): Boolean {
        if (activeClient == null || event.modifiers.command || event.modifiers.control) {
            return false
        }
        return hasMarkedText() || event.keyCode in imeNavigationKeys
    }

    /**
     * Offers a key press to the input method, returning whether it consumed it.
     *
     * The input method has to see the keys, because that is the only way a dead key, an accent popover
     * or a CJK composition can work: the character the user ends up with is not a function of this one
     * keystroke. The toolkit's answer already accounts for the awkward case — when the input method
     * turns a key into a `doCommandBySelector:` call, `handleCurrentEvent` reports what [doCommand]
     * returned rather than claiming the key — so it can be taken at face value.
     *
     * A key held with Command or Control is a shortcut, never text, so it is not offered at all.
     */
    fun offerKeyDownToInputMethod(event: Event.KeyDown): Boolean {
        val client = activeClient ?: return false
        if (event.modifiers.command || event.modifiers.control) {
            return false
        }
        client.beginEventInterpretation()
        val handledNatively = nativeWindow.textInputContext.handleCurrentEvent() == EventHandlerResult.Stop
        val editedText = client.endEventInterpretation()
        return editedText || handledNatively
    }
}

/**
 * Answers macOS's `NSTextInputClient` questions out of a Compose [PlatformTextInputMethodRequest].
 *
 * Coordinates: Compose works in pixels relative to the scene root, which is the window's content area;
 * macOS wants logical points in the global desktop space. The window's content origin is what bridges
 * the two.
 */
private class MacOsTextInputClient(
    private val request: PlatformTextInputMethodRequest,
    private val nativeWindow: Window,
    private val density: () -> Density,
) : TextInputClient {
    /**
     * Set whenever macOS edits the text through us, so that [MacOsTextInput.offerKeyDownToInputMethod]
     * can tell "the input method consumed this key" from "the input method ignored it".
     */
    private var didEditText = false

    fun beginEventInterpretation() {
        didEditText = false
    }

    fun endEventInterpretation(): Boolean = didEditText

    override fun hasMarkedText(): Boolean = request.state.composition != null

    override fun markedRange(): KdtTextRange? = request.state.composition?.toKdtTextRange()

    override fun selectedRange(): KdtTextRange = request.state.selection.toKdtTextRange()

    override fun insertText(text: String, replacementRange: KdtTextRange?) {
        didEditText = true
        val replacement = replacementRange?.toComposeTextRange()
        if (replacement == null || replacement == request.state.selection) {
            request.editText { commitText(text, 1) }
            return
        }
        // The accent popover commits its base character and then replaces it once a candidate is picked,
        // so the range it names is behind the caret rather than the selection. `TextEditingScope` cannot
        // move the selection, so the replacement goes through the command path, as in `setMarkedText`.
        request.onEditCommand(
            listOf(
                SetSelectionCommand(replacement.start, replacement.end),
                CommitTextCommand(text, 1),
            ),
        )
    }

    override fun doCommand(command: String): Boolean {
        // Editing commands (`moveLeft:`, `deleteBackward:`, `insertNewline:`, ...) are Compose's job;
        // refusing them here is what lets the key fall through to it.
        return false
    }

    override fun unmarkText() {
        didEditText = true
        request.editText { finishComposingText() }
    }

    override fun setMarkedText(text: String, selectedRange: KdtTextRange?, replacementRange: KdtTextRange?) {
        didEditText = true
        // `setComposingText` already replaces the current composition, or the selection when there is
        // none, which is exactly what macOS specifies for a null `replacementRange`, and leaves the
        // caret at the end of what it inserted.
        val compositionStart = (request.state.composition ?: request.state.selection).min
        val selection = selectedRange?.let {
            ComposeTextRange(compositionStart + it.location.toInt(), compositionStart + (it.location + it.length).toInt())
        }
        if (selection == null || selection == ComposeTextRange(compositionStart + text.length)) {
            request.editText { setComposingText(text, 1) }
        } else {
            // `selectedRange` is the clause the input method is converting, or where inside the
            // composition it wants the caret — multi-clause Japanese conversion and Hangul depend on it.
            // `TextEditingScope` cannot move the selection, so the edit goes through the command path.
            request.onEditCommand(
                listOf(
                    SetComposingTextCommand(text, 1),
                    SetSelectionCommand(selection.start, selection.end),
                ),
            )
        }
    }

    override fun attributedStringForRange(range: KdtTextRange): TextInputClient.StringAndRange {
        val text = request.state
        val start = range.location.toInt()
        if (start >= text.length) {
            return TextInputClient.StringAndRange(null, null)
        }
        val requestedEnd = (range.location + range.length).toInt()
        val end = requestedEnd.coerceAtMost(text.length)
        return TextInputClient.StringAndRange(
            text.subSequence(start, end).toString(),
            // Only reported when it differs from what was asked for, as the protocol expects.
            ComposeTextRange(start, end).toKdtTextRange().takeIf { end != requestedEnd },
        )
    }

    override fun firstRectForCharacterRange(range: KdtTextRange): TextInputClient.RectAndRange {
        val (coveredRange, rectInRoot) = firstRectInRoot(range.toComposeTextRange())
        val rect = with(density()) {
            DpRect(
                left = rectInRoot.left.toDp(),
                top = rectInRoot.top.toDp(),
                right = rectInRoot.right.toDp(),
                bottom = rectInRoot.bottom.toDp(),
            )
        }
        return TextInputClient.RectAndRange(
            rect.toScreenLogicalRect(nativeWindow.contentOrigin),
            coveredRange.toKdtTextRange().takeIf { coveredRange != range.toComposeTextRange() },
        )
    }

    override fun characterIndexForPoint(point: LogicalPoint): Long? {
        val layout = request.textLayoutResult() ?: return null
        val textOriginInRoot = request.unclippedTextOffsetInRoot() ?: Offset.Zero
        val positionInContent = (point - nativeWindow.contentOrigin).toDpOffset().toPxOffset(density())
        return layout.getOffsetForPosition(positionInContent - textOriginInRoot).toLong()
    }

    /**
     * The rectangle around the part of [range] that lies on its first line, plus the range that
     * rectangle actually covers — macOS asks about ranges that may wrap, and wants only the first line.
     */
    private fun firstRectInRoot(range: ComposeTextRange): Pair<ComposeTextRange, Rect> {
        val layout = request.textLayoutResult() ?: return range to Rect.Zero
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
        return ComposeTextRange(start, end) to rect.translate(textOriginInRoot)
    }
}

private fun ComposeTextRange.toKdtTextRange(): KdtTextRange = KdtTextRange(start.toLong(), length.toLong())

private fun KdtTextRange.toComposeTextRange(): ComposeTextRange = ComposeTextRange(location.toInt(), (location + length).toInt())
