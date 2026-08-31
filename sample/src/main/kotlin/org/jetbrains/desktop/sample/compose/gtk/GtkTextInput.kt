@file:OptIn(ExperimentalComposeUiApi::class)

package org.jetbrains.desktop.sample.compose.gtk

import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers
import androidx.compose.ui.input.pointer.isAltPressed
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpRect
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.jetbrains.desktop.gtk.Event
import org.jetbrains.desktop.gtk.KeyCode
import org.jetbrains.desktop.gtk.TextInputContentPurpose
import org.jetbrains.desktop.gtk.TextInputContext
import org.jetbrains.desktop.gtk.TextInputContextHint
import org.jetbrains.desktop.gtk.TextInputSurroundingText
import org.jetbrains.desktop.sample.compose.KdtImmediateDispatcher

/**
 * Input-method support for one window: the bridge between Compose's text field and GTK's `GtkIMContext`.
 *
 * The protocol runs in both directions. Towards the input method the application publishes a
 * [TextInputContext] — where the caret is on screen, what kind of field this is — and republishes it
 * whenever any of that changes; that context is what lets an IME place its candidate window and do
 * context-sensitive conversion. Towards the application the input method sends `Event.TextInput`, which
 * carries up to three things at once: text to delete around the cursor, text to commit, and the new
 * provisional (preedit) string.
 *
 * Two things differ from the Wayland backend and account for the shape of this class:
 *  - the input method is armed on the *window* rather than on the seat, so [GtkWindow] drives it directly
 *    instead of asking the application to;
 *  - the text around the cursor is *pulled* rather than pushed. It is not part of [TextInputContext];
 *    GTK asks for it through the application's `getSurroundingText` callback, which lands in
 *    [surroundingTextForInputMethod]. That is also why the answer is cached: the byte offsets in a later
 *    delete request are relative to the text we last handed out, so that exact text has to be kept.
 *
 * Compose starts and ends a session through `PlatformContext.startInputMethod`, so the session's lifetime
 * is the lifetime of a focused text field, and only one can be active at a time.
 *
 * Plain typing does not go through this class on the way in: a key that produced characters and that
 * nothing in Compose claimed is committed through [commitCharacters] instead. Compose's own text field
 * decides whether a key inserts text by looking for an AWT `KEY_TYPED` event, which a toolkit key event
 * cannot be, so the insertion has to be made explicitly.
 */
internal class GtkTextInput(
    private val onStartInputMethod: (TextInputContext) -> Unit,
    private val onUpdateInputMethod: (TextInputContext) -> Unit,
    private val onStopInputMethod: () -> Unit,
    private val density: () -> Density,
) {
    private var activeRequest: PlatformTextInputMethodRequest? = null

    /**
     * The text around the cursor as last handed to the input method, needed to interpret the byte offsets
     * it sends back. Set by [surroundingTextForInputMethod], never guessed.
     */
    private var publishedSurroundingText: SurroundingText? = null

    /** Whether the input method currently has a provisional string in the field. */
    var hasPreeditString: Boolean = false
        private set

    /** Whether a Compose text field currently holds the input session. */
    val isSessionActive: Boolean get() = activeRequest != null

    /** The context last published, so the window can re-arm the input method when focus comes back. */
    var currentContext: TextInputContext? = null
        private set

    /**
     * Publishes [request]'s editor state to the input method and keeps it published until the calling
     * coroutine is cancelled, which is how Compose ends a session.
     */
    suspend fun startInputMethod(request: PlatformTextInputMethodRequest): Nothing {
        val initialContext = withContext(KdtImmediateDispatcher) {
            activeRequest = request
            hasPreeditString = false
            request.toTextInputContext().also {
                currentContext = it
                onStartInputMethod(it)
            }
        }
        try {
            coroutineScope {
                // The context has to be republished on every edit, or the IME puts its candidate window in
                // the wrong place. Reading the request inside `snapshotFlow` picks those changes up for as
                // long as what it reads is snapshot state, which is the text field's own editor state and
                // layout; anything it does not observe just means the input method keeps the context
                // published at the start of the session.
                launch {
                    var lastPublished = initialContext
                    snapshotFlow { request.toTextInputContext() }.collect { context ->
                        if (context == lastPublished) return@collect
                        lastPublished = context
                        currentContext = context
                        onUpdateInputMethod(context)
                    }
                }
                awaitCancellation()
            }
        } finally {
            // NonCancellable: the coroutine is already cancelled by the time we get here, so a plain
            // withContext would refuse to run and the input method would stay armed for a field that is gone.
            withContext(NonCancellable + KdtImmediateDispatcher) {
                activeRequest = null
                publishedSurroundingText = null
                currentContext = null
                hasPreeditString = false
                onStopInputMethod()
            }
        }
    }

    /**
     * Answers GTK's request for the text around the cursor, and remembers the answer.
     *
     * `null` when no text field holds the session, which the application turns into an empty answer — GTK
     * asks the window, not the field, and has no way of knowing there is nothing to ask about.
     */
    fun surroundingTextForInputMethod(): TextInputSurroundingText? {
        val request = activeRequest ?: return null
        val surrounding = request.surroundingText()
        publishedSurroundingText = surrounding
        return TextInputSurroundingText(
            surroundingText = surrounding.text,
            cursorCodepointOffset = surrounding.cursorCodePointOffset(),
            selectionStartCodepointOffset = surrounding.selectionStartCodePointOffset(),
        )
    }

    /**
     * Applies one input-method event to the focused text field.
     *
     * The three parts have to be applied in the order the protocol specifies — delete around the cursor,
     * commit, then set the new provisional string — because each is expressed relative to the state the
     * previous one leaves behind.
     */
    fun handleTextInputEvent(event: Event.TextInput) {
        val request = activeRequest ?: return
        val committedText = event.commitStringData?.text
        val preeditText = event.preeditStringData?.text
        val deleteSurrounding = event.deleteSurroundingTextData
        hasPreeditString = preeditText != null

        val text = publishedSurroundingText
        request.editText {
            if (deleteSurrounding != null && text != null) {
                deleteSurroundingTextInCodePoints(
                    text.codePointsBefore(deleteSurrounding.beforeLengthInBytes.toInt()),
                    text.codePointsAfter(deleteSurrounding.afterLengthInBytes.toInt()),
                )
            }
            if (committedText != null) {
                commitText(committedText, 1)
            }
            when {
                // `setComposingText` replaces the current composition, which is exactly step 5.
                preeditText != null -> setComposingText(preeditText, 1)
                // Nothing was committed either: the input method abandoned the composition.
                committedText == null -> finishComposingText()
            }
        }
    }

    /** Drops a provisional string without committing it, for when the session is being taken away. */
    fun discardActivePreedit() {
        val request = activeRequest ?: return
        if (!hasPreeditString) return
        hasPreeditString = false
        request.editText {
            setComposingText("", 1)
            finishComposingText()
        }
    }

    /**
     * Inserts the characters a key press produced, returning whether there was a text field to insert them
     * into.
     *
     * Command and control chords are shortcuts rather than text and are refused. AltGr arrives as Ctrl+Alt
     * together and is deliberately allowed, because that is how the third level of a layout is typed.
     */
    fun commitCharacters(event: Event.KeyDown, modifiers: PointerKeyboardModifiers): Boolean {
        val request = activeRequest ?: return false
        val characters = event.characters?.takeIf { it.isNotEmpty() } ?: return false
        // Escape reports the 0x1B control character as its "characters"; typing that is never wanted.
        if (event.keyCode.value == KeyCode.Escape) return false
        if (modifiers.isMetaPressed) return false
        if (modifiers.isCtrlPressed != modifiers.isAltPressed) return false
        request.editText { commitText(characters, 1) }
        return true
    }

    private fun PlatformTextInputMethodRequest.toTextInputContext(): TextInputContext {
        val imeOptions = imeOptions
        val contentPurpose = imeOptions.keyboardType.toTextInputContentPurpose()
        return TextInputContext(
            hints = buildSet {
                @Suppress("DEPRECATION")
                add(if (imeOptions.autoCorrect) TextInputContextHint.Spellcheck else TextInputContextHint.NoSpellcheck)
                when (imeOptions.capitalization) {
                    KeyboardCapitalization.None -> add(TextInputContextHint.Lowercase)
                    KeyboardCapitalization.Characters -> add(TextInputContextHint.UppercaseChars)
                    KeyboardCapitalization.Words -> add(TextInputContextHint.UppercaseWords)
                    KeyboardCapitalization.Sentences -> add(TextInputContextHint.UppercaseSentences)
                    else -> {}
                }
                // GTK has no "hidden text" hint the way Wayland does; `Private` is the one that asks the
                // input method not to remember or learn from what is typed, which is the part that matters.
                if (contentPurpose == TextInputContentPurpose.Password || contentPurpose == TextInputContentPurpose.Pin) {
                    add(TextInputContextHint.Private)
                }
            },
            contentPurpose = contentPurpose,
            cursorRectangle = caretRectangle().roundToLogicalRect(),
        )
    }

    private fun PlatformTextInputMethodRequest.surroundingText(): SurroundingText {
        val text = state.toString()
        val selection = state.selection
        return SurroundingText(
            text = text,
            cursorOffset = selection.end.coerceIn(0, text.length),
            selectionStartOffset = selection.start.coerceIn(0, text.length),
        )
    }

    /**
     * Where the caret is, in dp relative to the window's content, so the IME can put its candidate window
     * next to it. An empty rectangle when the field has not been laid out yet, which GTK reads as "no idea,
     * place it wherever".
     */
    private fun PlatformTextInputMethodRequest.caretRectangle(): DpRect {
        val layout = textLayoutResult() ?: return DpRect(0.dp, 0.dp, 0.dp, 0.dp)
        val textOriginInRoot = unclippedTextOffsetInRoot() ?: Offset.Zero
        val offset = state.selection.end.coerceIn(0, layout.layoutInput.text.length)
        val line = layout.getLineForOffset(offset)
        val x = layout.getHorizontalPosition(offset, usePrimaryDirection = true)
        val caretInRoot = Rect(
            left = x,
            top = layout.getLineTop(line),
            right = x,
            bottom = layout.getLineBottom(line),
        ).translate(textOriginInRoot)
        return with(density()) {
            DpRect(
                left = caretInRoot.left.toDp(),
                top = caretInRoot.top.toDp(),
                // A zero-width rectangle leaves the input method nothing to anchor to.
                right = caretInRoot.right.toDp() + 1.dp,
                bottom = caretInRoot.bottom.toDp(),
            )
        }
    }
}

/**
 * The text around the cursor as published to the input method.
 *
 * The input method addresses it in UTF-8 bytes, Compose's edit commands in code points and the string
 * itself in UTF-16 chars, so all three have to be kept apart; the conversions below are the only place
 * that has to know about it.
 */
private data class SurroundingText(
    val text: String,
    /** UTF-16 offset of the caret, i.e. the selection's end. */
    val cursorOffset: Int,
    /** UTF-16 offset of the selection's other end. */
    val selectionStartOffset: Int,
) {
    fun cursorCodePointOffset(): UShort = text.codePointCount(0, cursorOffset).toUShort()

    fun selectionStartCodePointOffset(): UShort = text.codePointCount(0, selectionStartOffset).toUShort()

    /** How many code points before the caret the given number of UTF-8 bytes covers. */
    fun codePointsBefore(byteLength: Int): Int {
        var remainingBytes = byteLength
        var index = cursorOffset
        var codePoints = 0
        while (remainingBytes > 0 && index > 0) {
            val codePoint = text.codePointBefore(index)
            remainingBytes -= codePoint.utf8Length()
            index -= Character.charCount(codePoint)
            codePoints++
        }
        return codePoints
    }

    /** How many code points after the caret the given number of UTF-8 bytes covers. */
    fun codePointsAfter(byteLength: Int): Int {
        var remainingBytes = byteLength
        var index = cursorOffset
        var codePoints = 0
        while (remainingBytes > 0 && index < text.length) {
            val codePoint = text.codePointAt(index)
            remainingBytes -= codePoint.utf8Length()
            index += Character.charCount(codePoint)
            codePoints++
        }
        return codePoints
    }
}

private fun Int.utf8Length(): Int = when {
    this < 0x80 -> 1
    this < 0x800 -> 2
    this < 0x10000 -> 3
    else -> 4
}

private fun KeyboardType.toTextInputContentPurpose(): TextInputContentPurpose = when (this) {
    KeyboardType.Uri -> TextInputContentPurpose.Url
    KeyboardType.Email -> TextInputContentPurpose.Email
    KeyboardType.Number -> TextInputContentPurpose.Digits
    KeyboardType.Decimal -> TextInputContentPurpose.Number
    KeyboardType.Phone -> TextInputContentPurpose.Phone
    KeyboardType.Password -> TextInputContentPurpose.Password
    KeyboardType.NumberPassword -> TextInputContentPurpose.Pin
    else -> TextInputContentPurpose.Normal
}
