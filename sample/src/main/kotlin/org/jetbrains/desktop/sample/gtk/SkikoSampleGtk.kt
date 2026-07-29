@file:Suppress("DuplicatedCode")

package org.jetbrains.desktop.sample.gtk

import org.jetbrains.desktop.gtk.AccentColorValue
import org.jetbrains.desktop.gtk.Application
import org.jetbrains.desktop.gtk.ApplicationConfig
import org.jetbrains.desktop.gtk.ColorSchemeValue
import org.jetbrains.desktop.gtk.DataSource
import org.jetbrains.desktop.gtk.DataTransferContent
import org.jetbrains.desktop.gtk.DesktopSetting
import org.jetbrains.desktop.gtk.DragAndDropAction
import org.jetbrains.desktop.gtk.DragAndDropQueryData
import org.jetbrains.desktop.gtk.DragAndDropQueryResponse
import org.jetbrains.desktop.gtk.DragIconParams
import org.jetbrains.desktop.gtk.Event
import org.jetbrains.desktop.gtk.EventHandlerResult
import org.jetbrains.desktop.gtk.FileDialog
import org.jetbrains.desktop.gtk.KeyCode
import org.jetbrains.desktop.gtk.KeyModifiers
import org.jetbrains.desktop.gtk.KotlinDesktopToolkit
import org.jetbrains.desktop.gtk.LogLevel
import org.jetbrains.desktop.gtk.Logger
import org.jetbrains.desktop.gtk.LogicalPixels
import org.jetbrains.desktop.gtk.LogicalPixelsInt
import org.jetbrains.desktop.gtk.LogicalPoint
import org.jetbrains.desktop.gtk.LogicalRect
import org.jetbrains.desktop.gtk.LogicalSize
import org.jetbrains.desktop.gtk.MouseButton
import org.jetbrains.desktop.gtk.PhysicalSize
import org.jetbrains.desktop.gtk.PointerShape
import org.jetbrains.desktop.gtk.RenderingMode
import org.jetbrains.desktop.gtk.RequestId
import org.jetbrains.desktop.gtk.Scale
import org.jetbrains.desktop.gtk.ShowNotificationParams
import org.jetbrains.desktop.gtk.StartDragAndDropParams
import org.jetbrains.desktop.gtk.SupportedActionsForMime
import org.jetbrains.desktop.gtk.TextInputContentPurpose
import org.jetbrains.desktop.gtk.TextInputContext
import org.jetbrains.desktop.gtk.TextInputContextHint
import org.jetbrains.desktop.gtk.TextInputPreeditAttribute
import org.jetbrains.desktop.gtk.TextInputSurroundingText
import org.jetbrains.desktop.gtk.Window
import org.jetbrains.desktop.gtk.WindowDecorationMode
import org.jetbrains.desktop.gtk.WindowId
import org.jetbrains.desktop.gtk.WindowParams
import org.jetbrains.desktop.gtk.utf8OffsetToUtf16Offset
import org.jetbrains.desktop.sample.common.runtimeInfo
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.DirectContext
import org.jetbrains.skia.Font
import org.jetbrains.skia.FontMgr
import org.jetbrains.skia.FontStyle
import org.jetbrains.skia.GLAssembledInterface
import org.jetbrains.skia.Image
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import org.jetbrains.skia.Surface
import org.jetbrains.skia.SurfaceOrigin
import org.jetbrains.skia.TextLine
import org.jetbrains.skia.makeGLWithInterface
import org.jetbrains.skia.paragraph.RectHeightMode
import org.jetbrains.skia.paragraph.RectWidthMode
import org.jetbrains.skia.paragraph.TextStyle
import java.lang.AutoCloseable
import java.net.URI
import java.text.BreakIterator
import kotlin.Array
import kotlin.Boolean
import kotlin.ByteArray
import kotlin.Int
import kotlin.Long
import kotlin.Pair
import kotlin.String
import kotlin.Suppress
import kotlin.UInt
import kotlin.UShort
import kotlin.Unit
import kotlin.also
import kotlin.check
import kotlin.error
import kotlin.io.path.Path
import kotlin.let
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin
import kotlin.run
import kotlin.time.Duration.Companion.seconds
import kotlin.time.TimeSource
import kotlin.toUInt
import kotlin.toUShort
import kotlin.use

private const val TEXT_MIME_TYPE = "text/plain;charset=utf-8"
private const val URI_LIST_MIME_TYPE = "text/uri-list"
private const val PNG_MIME_TYPE = "image/png"

// TODO
private val EXAMPLE_FILES: List<String> = listOf(
    "/home/nikola/Pictures/Screenshots/Screenshot From 2025-01-15 12-08-34.png",
    "/home/nikola/Pictures/Screenshots/Screenshot From 2025-01-15 13-55-25.png",
//    "/home/nikola/Pictures/Screenshots/Screenshot From 2025-01-15 14-02-45.png",
//    "/etc/hosts",
//    "/boot/efi/",
)

private sealed class DataTransferContentType {
    data class Text(val text: String) : DataTransferContentType()
    data class UriList(val files: List<String>) : DataTransferContentType()

    fun mimeTypes(): List<String> {
        return when (this) {
            is Text -> listOf(TEXT_MIME_TYPE)
            is UriList -> listOf(URI_LIST_MIME_TYPE, TEXT_MIME_TYPE)
        }
    }
}

private fun KeyCode.isModifierKey(): Boolean {
    return when (this.value) {
        KeyCode.Alt_L, KeyCode.Alt_R,
        KeyCode.Control_L, KeyCode.Control_R,
        KeyCode.Shift_L, KeyCode.Shift_R,
        KeyCode.Super_L, KeyCode.Super_R,
        -> true

        else -> false
    }
}

private fun Set<KeyModifiers>.shortcutModifiers(): Set<KeyModifiers> = toMutableSet().also {
    it.remove(KeyModifiers.CapsLock)
    it.remove(KeyModifiers.NumLock)
}

internal data class DesktopSettings(
    var colorScheme: ColorSchemeValue = ColorSchemeValue.NoPreference,
    var accentColor: Int = Color.BLUE,
    var middleClickPaste: Boolean = true,
) {
    fun update(s: DesktopSetting, app: Application) {
        when (s) {
            is DesktopSetting.ColorScheme -> {
                colorScheme = s.value
                app.setPreferDarkTheme(s.value == ColorSchemeValue.PreferDark)
            }

            is DesktopSetting.AccentColor -> accentColor = when (s.value) {
                AccentColorValue.Unknown -> Color.MAGENTA
                AccentColorValue.Blue -> Color.BLUE
                AccentColorValue.Teal -> Color.makeRGB(0, 128, 128)
                AccentColorValue.Green -> Color.GREEN
                AccentColorValue.Yellow -> Color.YELLOW
                AccentColorValue.Orange -> Color.makeRGB(255, 165, 0)
                AccentColorValue.Red -> Color.RED
                AccentColorValue.Pink -> Color.makeRGB(255, 192, 203)
                AccentColorValue.Purple -> Color.makeRGB(128, 0, 128)
                AccentColorValue.Slate -> Color.makeRGB(112, 128, 144)
            }

            is DesktopSetting.MiddleClickPaste -> middleClickPaste = s.value
            else -> {}
        }
    }
}

private interface ClipboardHandler {
    fun copy(content: DataTransferContentType)
    fun copyToPrimarySelection(content: DataTransferContentType)
    fun paste(supportedMimeTypes: List<String>, windowId: WindowId)
    fun pasteFromPrimarySelection(supportedMimeTypes: List<String>, windowId: WindowId)
    fun startDrag(content: DataTransferContentType, params: StartDragAndDropParams, draw: (Canvas, Scale) -> Unit)
    fun stopDrag()
}

private data class PreeditData(
    val cursorOffset: Int?,
    val text: List<Pair<String, TextStyle?>>,
)

private class EditorState {
    private var textInputAvailable: Boolean = false
    private var textInputEnabled: Boolean = false
    private var preedit: PreeditData? = null
    private var text: StringBuilder = StringBuilder()
    private var cursorOffset: Int = 0
    private var cursorRectangle = LogicalDoubleRect.Zero
    private var selectionStartOffset: Int? = null
    private var textLineCreator = TextLineCreator()
    private var textLine = textLineCreator.makeTextLine("", 0f, Color.WHITE)
    private var statsTextLineCreator = TextLineCreator()
    private var textLineStats = statsTextLineCreator.makeTextLine("", 0f, Color.WHITE)
    private var fpsTextLineCreator = TextLineCreator()
    private var drawCallCount = 0
    private var lastFps = 0
    private var lastDrawMeasureTime = TimeSource.Monotonic.markNow()
    private var pastedImage: Image? = null

    companion object {
        val FONT_SIZE = LogicalPixels(50.0)

        private fun codepointFromOffset(sb: StringBuilder, offset: Int): UShort {
            if (offset == 0) {
                return 0U
            }
            return sb.codePointCount(0, offset).toUShort()
        }

        private fun getPreviousGlyphOffset(text: String, offset: Int): Int {
            if (offset == 0) {
                return 0
            }
            val bi = BreakIterator.getCharacterInstance()
            bi.setText(text)
            return bi.preceding(offset)
        }

        private fun getNextGlyphOffset(text: String, offset: Int): Int {
            val bi = BreakIterator.getCharacterInstance()
            bi.setText(text)
            return bi.following(offset).let {
                if (it == BreakIterator.DONE) {
                    text.length
                } else {
                    it
                }
            }
        }
    }

    fun getSurroundingText(): TextInputSurroundingText {
        val cursorCodepoint = codepointFromOffset(text, cursorOffset)
        return TextInputSurroundingText(
            surroundingText = text.toString(),
            cursorCodepointOffset = cursorCodepoint,
            selectionStartCodepointOffset = selectionStartOffset?.let { codepointFromOffset(text, it) } ?: cursorCodepoint,
        )
    }

    fun createTextInputContext(): TextInputContext {
        return TextInputContext(
            hints = setOf(TextInputContextHint.Spellcheck, TextInputContextHint.WordCompletion),
            contentPurpose = TextInputContentPurpose.Normal,
            cursorRectangle = cursorRectangle.round(),
        )
    }

    private fun getTextLineStatsString(selectionRange: Pair<Int, Int>?): String {
        val s = StringBuilder()
        s.append("Cursor pos: $cursorOffset")
        if (selectionRange != null) {
            s.append(", selection: ${selectionRange.first} - ${selectionRange.second}")
        }
        if (!textInputEnabled) {
            s.append(", IME disabled (press TAB to enable)")
        }
        preedit?.cursorOffset?.let {
            s.append(", compose: $it")
        }
        return s.toString()
    }

    fun onTextChanged(container: LogicalDoubleRect, scale: Scale) {
        val selectionRange = getSelectionRange()
        var yOffset = container.y
        textLineStats =
            statsTextLineCreator.makeTextLine(getTextLineStatsString(selectionRange), LogicalPixelsInt(20).toSkiko(scale), Color.WHITE)
        yOffset += scale.rawPhysicalToLogical(textLineStats.height.toDouble())

        val cursorOffset = cursorOffset
        val fontSize = FONT_SIZE.toSkiko(scale)
        val preedit = preedit
        val stringLine: List<Pair<String, TextStyle?>> = if (selectionRange != null) {
            listOf(
                Pair(text.substring(0, selectionRange.first), null),
                Pair(text.substring(selectionRange.first, selectionRange.second), TextLineCreator.selectionStyle(fontSize, Color.WHITE)),
                Pair(text.substring(selectionRange.second, text.length), null),
            )
        } else if (preedit != null) {
            buildList {
                if (cursorOffset != 0) {
                    add(Pair(text.substring(0, cursorOffset), null))
                }
                preedit.text.forEach { it.second?.fontSize = FONT_SIZE.toSkiko(scale) }
                addAll(preedit.text)
                if (cursorOffset != text.length) {
                    add(Pair(text.substring(cursorOffset, text.length), null))
                }
            }
        } else {
            listOf(Pair(text.toString(), null))
        }
        textLine = textLineCreator.makeTextLine(stringLine, fontSize, Color.WHITE)
        val offset = cursorOffset + (preedit?.cursorOffset ?: 0)
        val nestedPhysicalX = if (stringLine.isEmpty() || offset == 0) {
            0f
        } else {
            if (preedit?.text.isNullOrEmpty() || preedit.cursorOffset == 0) {
                val start = getPreviousGlyphOffset(text.toString(), cursorOffset)
                val end = getNextGlyphOffset(text.toString(), start)
                textLine.getRectsForRange(start, end, RectHeightMode.MAX, RectWidthMode.MAX).single().rect.right
            } else if (preedit.cursorOffset == null) {
                null
            } else {
                val preeditText = preedit.text.reduce { acc, e -> Pair(acc.first + e.first, null) }.first
                val preeditStart = getPreviousGlyphOffset(preeditText, preedit.cursorOffset)
                val preeditEnd = getNextGlyphOffset(preeditText, preeditStart)
                textLine.getRectsForRange(
                    cursorOffset + preeditStart,
                    cursorOffset + preeditEnd,
                    RectHeightMode.MAX,
                    RectWidthMode.MAX,
                ).single().rect.right
            }
        }

        if (nestedPhysicalX != null) {
            cursorRectangle = LogicalDoubleRect(
                x = container.x + scale.rawPhysicalToLogical(nestedPhysicalX.toDouble()),
                y = yOffset,
                width = LogicalPixels(2.0),
                height = scale.rawPhysicalToLogical(textLine.ideographicBaseline.toDouble()),
            )
        }
    }

    fun draw(canvas: Canvas, container: LogicalDoubleRect, scale: Scale) {
        val now = TimeSource.Monotonic.markNow()
        drawCallCount += 1
        if (now - lastDrawMeasureTime >= 1.seconds) {
            lastFps = drawCallCount
            lastDrawMeasureTime = now
            drawCallCount = 0
        }

        pastedImage?.let {
            Paint().use { paint ->
                val imageRect = it.imageInfo.bounds
                canvas.drawImageRect(
                    it,
                    src = imageRect.toRect(),
                    dst = LogicalRect.makeWH(imageRect.width, imageRect.height).toSkiko(scale),
                    paint,
                )
            }
        }

        // Draw text caret
        Paint().use { paint ->
            paint.color = Color.GREEN
            paint.strokeWidth = cursorRectangle.width.toSkiko(scale)

            val x = cursorRectangle.x.toSkiko(scale)
            val y = cursorRectangle.y.toSkiko(scale)
            val h = cursorRectangle.height.toSkiko(scale)

            canvas.drawLine(
                x0 = x,
                y0 = y,
                x1 = x,
                y1 = y + h,
                paint = paint,
            )
        }

        val textLineFps = fpsTextLineCreator.makeTextLine("$lastFps FPS", LogicalPixelsInt(20).toSkiko(scale), Color.WHITE)
        textLineStats.paint(canvas, (container.x + LogicalPixels(10.0)).toSkiko(scale), container.y.toSkiko(scale))
        textLineFps.paint(
            canvas,
            (container.x + container.width - LogicalPixels(100.0)).toSkiko(scale),
            (container.y + FONT_SIZE).toSkiko(scale),
        )
        textLine.paint(canvas, container.x.toSkiko(scale), container.y.toSkiko(scale) + textLineStats.height)
        Paint().use { paint ->
            paint.color = Color.WHITE
        }
    }

    private fun getSelectionRange(): Pair<Int, Int>? {
        val selectionStartOffset = selectionStartOffset
        if (selectionStartOffset != null) {
            val range = Pair(selectionStartOffset, cursorOffset)
            if (range.first > range.second) {
                return Pair(range.second, range.first)
            }
            return range
        }
        return null
    }

    fun getCurrentSelection(): String? {
        return getSelectionRange()?.let { text.substring(it.first, it.second) }
    }

    private fun deleteSelection(): Boolean {
        getSelectionRange()?.let {
            Logger.info { "deleteSelection: $it" }
            text.delete(it.first, it.second)
            cursorOffset = it.first
            selectionStartOffset = null
            return true
        }
        return false
    }

    fun typeIn(characters: String) {
        deleteSelection()
        text.insert(cursorOffset, characters)
        cursorOffset += characters.length
    }

    fun onKeyDown(
        event: Event.KeyDown,
        window: Window,
        windowState: WindowState,
        modifiers: Set<KeyModifiers>,
        clipboardHandler: ClipboardHandler,
        container: LogicalDoubleRect,
    ): EventHandlerResult {
        if (preedit != null && event.keyCode.value == KeyCode.Shift_R) {
            val disableTextInput = modifiers.contains(KeyModifiers.Alt)
            resetTextInput(window, reenable = !disableTextInput, clear = modifiers.contains(KeyModifiers.Control))
            textInputEnabled = !disableTextInput
        }

        val shortcutModifiers = modifiers.shortcutModifiers()
        when (shortcutModifiers) {
            setOf(KeyModifiers.Control, KeyModifiers.Shift) -> when (event.keyCode.value) {
                KeyCode.V -> {
                    clipboardHandler.paste(listOf(PNG_MIME_TYPE, URI_LIST_MIME_TYPE, TEXT_MIME_TYPE), event.windowId)
                }

                KeyCode.C -> {
                    clipboardHandler.copy(DataTransferContentType.UriList(EXAMPLE_FILES))
                }

                KeyCode.O -> {
                    window.showOpenFileDialog(
                        commonParams = FileDialog.CommonDialogParams(
                            modal = false,
                            title = "Open Directory",
                            acceptLabel = null,
                            currentFolder = null,
                        ),
                        openParams = FileDialog.OpenDialogParams(
                            allowsMultipleSelections = false,
                            selectDirectories = true,
                        ),
                    )
                }
            }

            setOf(KeyModifiers.Control) -> when (event.keyCode.value) {
                KeyCode.V -> {
                    clipboardHandler.paste(listOf(PNG_MIME_TYPE, TEXT_MIME_TYPE, URI_LIST_MIME_TYPE), event.windowId)
                }

                KeyCode.C -> {
                    getCurrentSelection()?.let { selection ->
                        clipboardHandler.copy(DataTransferContentType.Text(selection))
                    }
                }

                KeyCode.O -> {
                    window.showOpenFileDialog(
                        commonParams = FileDialog.CommonDialogParams(
                            modal = true,
                            title = "Open Files",
                            acceptLabel = null,
                            currentFolder = null,
                        ),
                        openParams = FileDialog.OpenDialogParams(
                            allowsMultipleSelections = true,
                            selectDirectories = false,
                        ),
                    )
                }

                KeyCode.S -> {
                    window.showSaveFileDialog(
                        commonParams = FileDialog.CommonDialogParams(
                            modal = true,
                            title = "Open Files",
                            acceptLabel = null,
                            currentFolder = null,
                        ),
                        saveParams = FileDialog.SaveDialogParams(
                            nameFieldStringValue = null,
                        ),
                    )
                }
            }

            setOf(KeyModifiers.Shift) -> when (event.keyCode.value) {
                KeyCode.Up -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    cursorOffset = 0
                }

                KeyCode.Down -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    val end = text.length
                    cursorOffset = end
                }

                KeyCode.Left -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    cursorOffset = getPreviousGlyphOffset(text.toString(), cursorOffset)
                }

                KeyCode.Right -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    cursorOffset = getNextGlyphOffset(text.toString(), cursorOffset)
                }

                else -> {
                    event.characters?.also(::typeIn)
                }
            }

            else -> when (event.keyCode.value) {
                KeyCode.Tab -> {
                    if (!textInputEnabled) {
                        textInputEnabled = true
                        window.textInputEnable(createTextInputContext())
                    }
                }

                KeyCode.BackSpace -> {
                    if (!deleteSelection() && cursorOffset > 0) {
                        val newCursorOffset = getPreviousGlyphOffset(text.toString(), cursorOffset)
                        text.delete(newCursorOffset, cursorOffset)
                        cursorOffset = newCursorOffset
                    }
                }

                KeyCode.F11 -> {
                    if (windowState.fullscreen) {
                        window.unsetFullScreen()
                    } else {
                        window.setFullScreen()
                    }
                }

                KeyCode.Up -> {
                    cursorOffset = 0
                }

                KeyCode.Down -> {
                    cursorOffset = text.length
                }

                KeyCode.Left -> {
                    cursorOffset = getPreviousGlyphOffset(text.toString(), cursorOffset)
                }

                KeyCode.Right -> {
                    cursorOffset = getNextGlyphOffset(text.toString(), cursorOffset)
                }

                else -> {
                    event.characters?.also(::typeIn)
                }
            }
        }

        if (shortcutModifiers.all { it != KeyModifiers.Shift && it != KeyModifiers.Control && it != KeyModifiers.Logo } &&
            !event.keyCode.isModifierKey()
        ) {
            selectionStartOffset = null
        }

        onTextChanged(container, windowState.scale)
        if (textInputAvailable && textInputEnabled) {
            window.textInputUpdate(createTextInputContext())
        }
        return EventHandlerResult.Stop
    }

    fun onDataTransfer(content: DataTransferContent, window: Window, container: LogicalDoubleRect, scale: Scale) {
        when (content.mimeType) {
            URI_LIST_MIME_TYPE -> {
                val files = content.data.decodeToString().trimEnd().split("\r\n")
                Logger.info { "Pasted ${files.size} files:" }
                for (file in files) {
                    val path = URI(file).path
                    Logger.info { path }
                }
            }

            TEXT_MIME_TYPE -> {
                deleteSelection()
                val pastedText = content.data.decodeToString()
                text.insert(cursorOffset, pastedText)
                cursorOffset += pastedText.length
                onTextChanged(container, scale)
                if (textInputAvailable && textInputEnabled) {
                    window.textInputUpdate(createTextInputContext())
                }
            }

            PNG_MIME_TYPE -> {
                pastedImage = Image.makeFromEncoded(content.data)
            }
        }
    }

    fun onDragAndDropFinished(action: DragAndDropAction?) {
        if (action == DragAndDropAction.Move) {
            deleteSelection()
        }
    }

    fun onKeyboardFocusChanged(focused: Boolean, window: Window, container: LogicalDoubleRect, scale: Scale) {
        if (focused && textInputEnabled) {
            window.textInputEnable(createTextInputContext())
        } else {
            resetTextInput(window, reenable = false, clear = false)
        }
        textInputAvailable = focused
        onTextChanged(container, scale)
    }

    fun resetTextInput(window: Window, reenable: Boolean = true, clear: Boolean = false) {
        if (!textInputAvailable) {
            return
        }

        window.textInputDisable()

        if (clear) {
            cursorOffset = 0
            text.clear()
        }

        if (textInputEnabled && reenable) {
            window.textInputEnable(createTextInputContext())
        }
    }

    fun onTextInput(event: Event.TextInput, window: Window, container: LogicalDoubleRect, scale: Scale): EventHandlerResult {
        preedit = null
        deleteSelection()
        event.deleteSurroundingTextData?.let { deleteSurroundingTextData ->
            val prevCursorOffset = cursorOffset
            Logger.info { "deleteSurroundingTextData: cursorOffset = $prevCursorOffset" }
            val beforeCursorOffset = if (prevCursorOffset == 0) {
                0
            } else {
                val textBeforeCursor = text.subSequence(0, prevCursorOffset)
                Logger.info { "deleteSurroundingTextData: textBeforeCursor = $textBeforeCursor" }
                utf8OffsetToUtf16Offset(textBeforeCursor.reversed(), deleteSurroundingTextData.beforeLengthInBytes)
            }
            Logger.info { "deleteSurroundingTextData: beforeCursorOffset = $beforeCursorOffset" }
            cursorOffset -= beforeCursorOffset

            val afterCursorOffset = if (prevCursorOffset == text.length) {
                0
            } else {
                val textAfterCursor = text.subSequence(prevCursorOffset + 1, text.length)
                Logger.info { "deleteSurroundingTextData: textAfterCursor = $textAfterCursor" }
                utf8OffsetToUtf16Offset(textAfterCursor, deleteSurroundingTextData.afterLengthInBytes)
            }
            Logger.info { "deleteSurroundingTextData: afterCursorOffset = $afterCursorOffset" }

            this.text.delete(prevCursorOffset - beforeCursorOffset, prevCursorOffset + afterCursorOffset)
        }
        event.commitStringData?.let { commitStringData ->
            commitStringData.text?.let { commitString ->
                this.text.insert(cursorOffset, commitString)
                cursorOffset += commitString.length
            }
        }
        event.preeditStringData?.let { preeditStringData ->
            val textWithStyles: List<Pair<String, TextStyle?>> = buildList {
                val preeditText = preeditStringData.text!!
                var previousEndOffset: Int? = null
                val applyAttribute = { beginBytePos: UInt, endBytePos: UInt, attribute: TextInputPreeditAttribute? ->
                    val startOffset = utf8OffsetToUtf16Offset(preeditText, beginBytePos)
                    if ((previousEndOffset == null && beginBytePos != 0U) ||
                        (previousEndOffset != null && startOffset > previousEndOffset!!)
                    ) {
                        val substring = preeditText.substring(previousEndOffset ?: 0, startOffset)
                        add(Pair(substring, null))
                    }
                    val endOffset = utf8OffsetToUtf16Offset(preeditText, endBytePos)
                    if (startOffset != endOffset) {
                        val textStyle = TextLineCreator.preeditStyle(attribute)
                        val substring = preeditText.substring(startOffset, endOffset)
                        Logger.info { "Preedit style from $startOffset to $endOffset ($substring): $attribute" }
                        add(Pair(substring, textStyle))
                    }
                    previousEndOffset = endOffset
                }
                preeditStringData.attributes.forEach {
                    applyAttribute(it.beginBytePos, it.endBytePos, it)
                }
                val utf8Len = preeditText.encodeToByteArray().size
                applyAttribute(utf8Len.toUInt(), utf8Len.toUInt(), null)
            }

            preedit = PreeditData(
                text = textWithStyles,
                cursorOffset = preeditStringData.text?.let { preeditString ->
                    if (preeditStringData.cursorBytePos == -1) {
                        null
                    } else {
                        check(preeditStringData.cursorBytePos >= 0)
                        utf8OffsetToUtf16Offset(preeditString, preeditStringData.cursorBytePos.toUInt())
                    }
                } ?: 0,
            )
        }
        onTextChanged(container, scale)
        if (textInputAvailable &&
            textInputEnabled &&
            (preedit != null || event.deleteSurroundingTextData != null || event.commitStringData != null)
        ) {
            window.textInputUpdate(createTextInputContext())
        }
        return EventHandlerResult.Stop
    }
}

private class WindowState {
    var active: Boolean = false
    var maximized: Boolean = false
    var fullscreen: Boolean = false
    var pointerShape: PointerShape = PointerShape.Default
    var scale: Scale = Scale.NO_SCALE
    var size: LogicalSize = LogicalSize.makeWH(0, 0)
    var insetStart: LogicalSize = LogicalSize.makeWH(0, 0)
    var insetEnd: LogicalSize = LogicalSize.makeWH(0, 0)

    fun configure(event: Event.WindowConfigure) {
        active = event.active
        maximized = event.maximized
        fullscreen = event.fullscreen
        size = event.size
        insetStart = event.insetStart
        insetEnd = event.insetEnd
    }
}

private class ContentArea {
    var contentRect = LogicalDoubleRect.Zero
    private var markerPosition: LogicalPoint? = null
    private var dragIconTextLineCreator = TextLineCreator()

    fun onMouseMoved(locationInWindow: LogicalPoint): EventHandlerResult {
        markerPosition = locationInWindow
        return EventHandlerResult.Continue
    }

    fun onMouseDown(
        event: Event.MouseDown,
        clipboardHandler: ClipboardHandler,
        modifiers: Set<KeyModifiers>,
        editorState: EditorState,
        windowState: WindowState,
    ): EventHandlerResult {
        val x = event.locationInWindow.x
        return if (event.locationInWindow.y <= windowState.insetStart.height.toLogicalPixels() &&
            (
                x <= windowState.insetStart.width.toLogicalPixels() ||
                    x >= (windowState.size.width - windowState.insetEnd.width).toLogicalPixels()
                )
        ) {
            EventHandlerResult.Continue
        } else {
            when (event.button) {
                MouseButton.LEFT -> when (modifiers.shortcutModifiers()) {
                    setOf(KeyModifiers.Alt) -> {
                        val skikoTextLine = dragIconTextLineCreator.makeTextLine(
                            EXAMPLE_FILES.joinToString("\n"),
                            LogicalPixelsInt(30).toSkiko(windowState.scale),
                            Color.BLACK,
                        )
                        val dragIconParams = DragIconParams(
                            renderingMode = RenderingMode.Auto,
                            size = skikoTextLine.toLogicalSize(windowState.scale),
                        )
                        val content = DataTransferContentType.UriList(EXAMPLE_FILES)
                        val startDragAndDropParams = StartDragAndDropParams(
                            mimeTypes = content.mimeTypes(),
                            actions = setOf(DragAndDropAction.Copy),
                            dragIconParams,
                        )
                        clipboardHandler.startDrag(content, startDragAndDropParams) { canvas, scale ->
                            canvas.clear(0x77777777)
                            val skikoTextLine = dragIconTextLineCreator.makeTextLine(
                                EXAMPLE_FILES.joinToString("\n"),
                                LogicalPixelsInt(10).toSkiko(scale),
                                Color.BLACK,
                            )
                            skikoTextLine.paint(canvas, 0f, 0f)
                        }
                        EventHandlerResult.Stop
                    }

                    else -> {
                        editorState.getCurrentSelection()?.let {
                            val skikoTextLine = dragIconTextLineCreator.makeTextLine(
                                it,
                                LogicalPixelsInt(30).toSkiko(windowState.scale),
                                Color.WHITE,
                            )
                            val dragIconParams = DragIconParams(
                                renderingMode = RenderingMode.Auto,
                                size = skikoTextLine.toLogicalSize(windowState.scale),
                            )
                            val content = DataTransferContentType.Text(it)
                            val startDragAndDropParams = StartDragAndDropParams(
                                mimeTypes = content.mimeTypes(),
                                actions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                                dragIconParams,
                            )
                            clipboardHandler.startDrag(content, startDragAndDropParams) { canvas, scale ->
                                canvas.clear(Color.BLUE)
                                val skikoTextLine = dragIconTextLineCreator.makeTextLine(
                                    it,
                                    LogicalPixelsInt(30).toSkiko(scale),
                                    Color.WHITE,
                                )
                                skikoTextLine.paint(canvas, 0f, 0f)
                            }
                            EventHandlerResult.Stop
                        } ?: EventHandlerResult.Continue
                    }
                }

                else -> EventHandlerResult.Continue
            }
        }
    }

    fun draw(canvas: Canvas, time: Long, scale: Scale, editorState: EditorState, accentColor: Int) {
        Paint().use { paint ->
            paint.color = 0xaa264653.toInt()
            canvas.drawRect(contentRect.toSkiko(scale), paint)
        }
//        Paint().use { paint ->
//            paint.color = Color.WHITE
//            paint.strokeWidth = scale.toFloat()
//            canvas.drawLine(
//                contentRect.width.toSkiko(scale),
//                0f,
//                0f,
//                contentRect.height.toSkiko(scale),
//                paint,
//            )
//        }
        drawSpinningCircle(canvas, contentRect, scale, time)
        val borderThickness = LogicalPixels(3.0)
        editorState.draw(canvas, contentRect.copy(x = contentRect.x + borderThickness), scale)
        drawWindowBorders(canvas, contentRect, scale, borderThickness, accentColor)
        drawCursor(canvas, contentRect, scale)
    }

    private fun drawSpinningCircle(canvas: Canvas, container: LogicalDoubleRect, scale: Scale, t: Long) {
        val width = container.width.toSkiko(scale)
        val height = container.height.toSkiko(scale)
        val angle = (t / 2000f) * 2f * PI
        val r = width / 4
        val x = r * sin(angle).toFloat() + width / 2f
        val y = r * cos(angle).toFloat() + height / 2f
        Paint().use { paint ->
            paint.color = Color.GREEN
            canvas.drawCircle(container.x.toSkiko(scale) + x, container.y.toSkiko(scale) + y, LogicalPixels(30.0).toSkiko(scale), paint)
        }
    }

    private fun drawWindowBorders(canvas: Canvas, container: LogicalDoubleRect, scale: Scale, thickness: LogicalPixels, accentColor: Int) {
        Paint().use { paint ->
            val len = LogicalPixels(100.0)

            val verticalCenterY = container.y + (container.height / 2) - (len / 2)
            val verticalBottomY = container.y + (container.height - len)
            // left
            paint.color = 0xFFe76f51.toInt()
            canvas.drawRect(LogicalDoubleRect(container.x, container.y, thickness, len).toSkiko(scale), paint)
            canvas.drawRect(LogicalDoubleRect(container.x, verticalCenterY, thickness, len).toSkiko(scale), paint)
            canvas.drawRect(LogicalDoubleRect(container.x, verticalBottomY, thickness, len).toSkiko(scale), paint)

            // right
            paint.color = 0xFFe9c46a.toInt()
            val rightX = container.x + (container.width - thickness)
            canvas.drawRect(LogicalDoubleRect(rightX, container.y, thickness, len).toSkiko(scale), paint)
            canvas.drawRect(LogicalDoubleRect(rightX, verticalCenterY, thickness, len).toSkiko(scale), paint)
            canvas.drawRect(LogicalDoubleRect(rightX, verticalBottomY, thickness, len).toSkiko(scale), paint)

            val horizontalCenterX = container.x + (container.width / 2) - (len / 2)
            val horizontalRightX = container.x + (container.width - len)
            // top
            paint.color = 0xFF2a9d8f.toInt()
            canvas.drawRect(LogicalDoubleRect(container.x, container.y, len, thickness).toSkiko(scale), paint)
            canvas.drawRect(LogicalDoubleRect(horizontalCenterX, container.y, len, thickness).toSkiko(scale), paint)
            canvas.drawRect(LogicalDoubleRect(horizontalRightX, container.y, len, thickness).toSkiko(scale), paint)

            // bottom
            paint.color = accentColor
            val bottomY = container.y + (container.height - thickness)
            canvas.drawRect(LogicalDoubleRect(container.x, bottomY, len, thickness).toSkiko(scale), paint)
            canvas.drawRect(LogicalDoubleRect(horizontalCenterX, bottomY, len, thickness).toSkiko(scale), paint)
            canvas.drawRect(LogicalDoubleRect(horizontalRightX, bottomY, len, thickness).toSkiko(scale), paint)
        }
    }

    private fun drawCursor(canvas: Canvas, container: LogicalDoubleRect, scale: Scale) {
        markerPosition?.let { curs ->
            if (container.contains(curs)) {
                Paint().use { paint ->
                    paint.color = 0x40FFFFFF
                    canvas.drawRect(
                        LogicalDoubleRect(
                            container.x,
                            curs.y,
                            container.width,
                            LogicalPixels(2.0),
                        ).toSkiko(scale),
                        paint,
                    )
                    canvas.drawRect(
                        LogicalDoubleRect(
                            curs.x,
                            container.y,
                            LogicalPixels(2.0),
                            container.height,
                        ).toSkiko(scale),
                        paint,
                    )
                }
            }
        }
    }
}

private class WindowContainer(
    private var customTitle: TextLine?,
    val contentArea: ContentArea,
    private var desktopSettings: DesktopSettings,
    private val requestClose: () -> Unit,
) {
    companion object {
        val COLOR_LIGHT_GRAY = Color.makeRGB(211, 211, 211)

        fun create(desktopSettings: DesktopSettings, requestClose: () -> Unit): WindowContainer {
            val contentArea = ContentArea()
            return WindowContainer(
                customTitle = null,
                contentArea,
                desktopSettings,
                requestClose,
            )
        }
    }

    fun settingsChanged(desktopSettings: DesktopSettings) {
        this.desktopSettings = desktopSettings
    }

    fun configure(event: Event.WindowConfigure, title: String, scale: Scale) {
        when (val decorationMode = event.decorationMode) {
            WindowDecorationMode.Server -> {
                customTitle = null
                contentArea.contentRect =
                    LogicalDoubleRect(
                        LogicalPixels.Zero,
                        LogicalPixels.Zero,
                        event.size.width.toLogicalPixels(),
                        event.size.height.toLogicalPixels(),
                    )
            }

            is WindowDecorationMode.CustomTitlebar -> {
                val font = FontMgr.default.matchFamilyStyle("sans-serif", FontStyle.BOLD)?.let { typeface ->
                    Font(typeface, (decorationMode.height.toSkiko(scale)))
                }
                customTitle = TextLine.make(title, font)
                contentArea.contentRect =
                    LogicalDoubleRect(
                        LogicalPixels.Zero,
                        decorationMode.height.toLogicalPixels(),
                        event.size.width.toLogicalPixels(),
                        event.size.height.toLogicalPixels(),
                    )
            }
        }
    }

    fun onScaleChange(windowState: WindowState, title: String) {
        val font = FontMgr.default.matchFamilyStyle("sans-serif", FontStyle.BOLD)?.let { typeface ->
            Font(typeface, windowState.insetStart.height.toSkiko(windowState.scale))
        }
        customTitle = TextLine.make(title, font)
    }

    fun onKeyDown(event: Event.KeyDown, clipboardHandler: ClipboardHandler, modifiers: Set<KeyModifiers>): EventHandlerResult {
        return when (event.keyCode.value) {
            KeyCode.Escape -> {
                clipboardHandler.stopDrag()
                EventHandlerResult.Stop
            }

            KeyCode.Q -> {
                if (modifiers.shortcutModifiers() == setOf(KeyModifiers.Control)) {
                    requestClose()
                    EventHandlerResult.Stop
                } else {
                    EventHandlerResult.Continue
                }
            }

            else -> EventHandlerResult.Continue
        }
    }

    fun onMouseMoved(locationInWindow: LogicalPoint): EventHandlerResult {
        return contentArea.onMouseMoved(locationInWindow)
    }

    fun onMouseDown(
        event: Event.MouseDown,
        editorState: EditorState,
        modifiers: Set<KeyModifiers>,
        clipboardHandler: ClipboardHandler,
        desktopSettings: DesktopSettings,
        windowState: WindowState,
    ): EventHandlerResult {
        if (contentArea.onMouseDown(event, clipboardHandler, modifiers, editorState, windowState) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }

        return if (event.button == MouseButton.MIDDLE) {
            when (modifiers.shortcutModifiers()) {
                setOf(KeyModifiers.Control) -> {
                    editorState.getCurrentSelection()?.let { selection ->
                        clipboardHandler.copyToPrimarySelection(DataTransferContentType.Text(selection))
                        EventHandlerResult.Stop
                    } ?: EventHandlerResult.Continue
                }

                setOf(KeyModifiers.Shift) -> {
                    if (desktopSettings.middleClickPaste) {
                        clipboardHandler.pasteFromPrimarySelection(
                            listOf(PNG_MIME_TYPE, URI_LIST_MIME_TYPE, TEXT_MIME_TYPE),
                            event.windowId,
                        )
                        EventHandlerResult.Stop
                    } else {
                        EventHandlerResult.Continue
                    }
                }

                else -> EventHandlerResult.Continue
            }
        } else {
            EventHandlerResult.Continue
        }
    }

    fun draw(canvas: Canvas, time: Long, editorState: EditorState, windowState: WindowState) {
        val scale = windowState.scale
        val backgroundColor = if (desktopSettings.colorScheme == ColorSchemeValue.PreferDark) {
            Color.makeARGB(
                240,
                32,
                32,
                32,
            )
        } else {
            Color.makeARGB(240, 200, 200, 200)
        }
        canvas.clear(backgroundColor)

        contentArea.draw(canvas, time, scale, editorState, desktopSettings.accentColor)

        Paint().use { paint ->
            paint.color = if (desktopSettings.colorScheme == ColorSchemeValue.PreferDark) Color.BLACK else Color.WHITE

            canvas.drawRect(
                Rect.makeXYWH(0f, 0f, windowState.insetStart.width.toSkiko(scale), windowState.insetStart.height.toSkiko(scale)),
                paint,
            )
            canvas.drawRect(
                Rect.makeXYWH(
                    (windowState.size.width - windowState.insetEnd.width).toSkiko(scale),
                    0f,
                    windowState.insetEnd.width.toSkiko(scale),
                    windowState.insetEnd.height.toSkiko(scale),
                ),
                paint,
            )
        }

        customTitle?.let {
            Paint().use { paint ->
                paint.color = if (windowState.active) Color.WHITE else COLOR_LIGHT_GRAY
                canvas.drawTextLine(
                    it,
                    windowState.insetStart.width.toSkiko(scale),
                    windowState.insetStart.height.toSkiko(scale),
                    paint,
                )
            }
        }
    }
}

private class RotatingBallWindow(
    private val windowContainer: WindowContainer,
    app: Application,
    windowParams: WindowParams,
    glProcFunc: Application.GlProcFunc,
) : SkikoWindowGtk(app, windowParams, glProcFunc) {
    private val editorState = EditorState()
    private var title: String = windowParams.title
    private var windowState = WindowState()

    companion object {
        fun createWindow(
            app: Application,
            windowParams: WindowParams,
            glProcFunc: Application.GlProcFunc,
            desktopSettings: DesktopSettings,
            requestClose: () -> Unit,
        ): RotatingBallWindow {
            val container = WindowContainer.create(desktopSettings, requestClose)
            return RotatingBallWindow(container, app, windowParams, glProcFunc)
        }
    }

    fun settingsChanged(desktopSettings: DesktopSettings) {
        windowContainer.settingsChanged(desktopSettings)
    }

    fun queryDragAndDropTarget(locationInWindow: LogicalPoint): DragAndDropQueryResponse {
        val response = if (locationInWindow.x < (windowContainer.contentArea.contentRect.width / 2)) {
            DragAndDropQueryResponse(
                supportedActionsPerMime = listOf(
                    SupportedActionsForMime(
                        supportedMimeType = URI_LIST_MIME_TYPE,
                        supportedActions = setOf(DragAndDropAction.Copy),
                        preferredAction = DragAndDropAction.Copy,
                    ),
                    SupportedActionsForMime(
                        supportedMimeType = TEXT_MIME_TYPE,
                        supportedActions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                        preferredAction = DragAndDropAction.Copy,
                    ),
                ),
            )
        } else {
            DragAndDropQueryResponse(
                supportedActionsPerMime = listOf(
                    SupportedActionsForMime(
                        supportedMimeType = TEXT_MIME_TYPE,
                        supportedActions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                        preferredAction = DragAndDropAction.Copy,
                    ),
                    SupportedActionsForMime(
                        supportedMimeType = URI_LIST_MIME_TYPE,
                        supportedActions = setOf(DragAndDropAction.Copy),
                        preferredAction = DragAndDropAction.Copy,
                    ),
                ),
            )
        }

        return response
    }

    fun getSurroundingText(): TextInputSurroundingText {
        return editorState.getSurroundingText()
    }

    override fun Canvas.draw(size: PhysicalSize, time: Long) {
        val canvas = this
        windowContainer.draw(canvas, time, editorState, windowState)
    }

    fun onWindowDraw(event: Event.WindowDraw): EventHandlerResult {
        return if (performDrawing(event)) {
            EventHandlerResult.Stop
        } else {
            EventHandlerResult.Continue
        }
    }

    fun configure(event: Event.WindowConfigure): EventHandlerResult {
        windowState.configure(event)
        windowContainer.configure(event, title, windowState.scale)
        // performDrawing(syncWithCA = true)
        return EventHandlerResult.Stop
    }

    private fun changePointerShape(newPointerShape: PointerShape) {
        if (windowState.pointerShape != newPointerShape) {
            windowState.pointerShape = newPointerShape
            window.setPointerShape(newPointerShape)
        }
    }

    fun onDataTransfer(content: DataTransferContent?): EventHandlerResult {
        content?.let {
            editorState.onDataTransfer(it, window, windowContainer.contentArea.contentRect, windowState.scale)
        }
        return EventHandlerResult.Stop
    }

    fun onDragAndDropFinished(action: DragAndDropAction?): EventHandlerResult {
        editorState.onDragAndDropFinished(action)
        return EventHandlerResult.Stop
    }

    fun onMouseMoved(locationInWindow: LogicalPoint): EventHandlerResult {
        changePointerShape(PointerShape.Default)
        return windowContainer.onMouseMoved(locationInWindow)
    }

    fun onKeyDown(event: Event.KeyDown, modifiers: Set<KeyModifiers>, clipboardHandler: ClipboardHandler): EventHandlerResult {
        if (modifiers.shortcutModifiers() == setOf(KeyModifiers.Control) && event.keyCode.value == KeyCode.H) {
            changePointerShape(PointerShape.Hidden)
            return EventHandlerResult.Stop
        }
        if (windowContainer.onKeyDown(event, clipboardHandler, modifiers) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        return editorState.onKeyDown(event, window, windowState, modifiers, clipboardHandler, windowContainer.contentArea.contentRect)
    }

    fun onScreenChange(event: Event.WindowScreenChange, app: Application): EventHandlerResult {
        val screen = app.allScreens().findById(event.newScreenId)!!
        return EventHandlerResult.Stop
    }

    fun onScaleChange(event: Event.WindowScaleChanged): EventHandlerResult {
        windowState.scale = event.newScale
        windowContainer.onScaleChange(windowState, title)
        return EventHandlerResult.Stop
    }

    fun onKeyboardFocusChanged(focused: Boolean): EventHandlerResult {
        editorState.onKeyboardFocusChanged(focused, window, windowContainer.contentArea.contentRect, windowState.scale)
        return EventHandlerResult.Stop
    }

    fun onTextInput(event: Event.TextInput): EventHandlerResult {
        return editorState.onTextInput(event, window, windowContainer.contentArea.contentRect, windowState.scale)
    }

    fun onMouseEntered(locationInWindow: LogicalPoint): EventHandlerResult {
        if (onMouseMoved(locationInWindow) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        return windowContainer.onMouseMoved(locationInWindow)
    }

    fun onMouseDown(
        event: Event.MouseDown,
        modifiers: Set<KeyModifiers>,
        clipboardHandler: ClipboardHandler,
        desktopSettings: DesktopSettings,
    ): EventHandlerResult {
        editorState.resetTextInput(window, reenable = true, clear = modifiers.contains(KeyModifiers.Control))
        return windowContainer.onMouseDown(event, editorState, modifiers, clipboardHandler, desktopSettings, windowState)
    }
}

private class ApplicationState(
    private val app: Application,
    private val glProcFunc: Application.GlProcFunc,
    private val renderingMode: RenderingMode,
) : AutoCloseable {
    private var nextWindowId = 0L
    private val windows = mutableMapOf<WindowId, RotatingBallWindow>()
    private var modifiers = setOf<KeyModifiers>()
    private val desktopSettings = DesktopSettings()
    private val windowClipboardHandlers = mutableMapOf<WindowId, ClipboardHandler>()
    private var currentClipboard: DataTransferContentType? = null
    private var currentClipboardPasteSerial = 0
    private val clipboardPasteSerialToWindow = mutableMapOf<Int, WindowId>()
    private var currentDragContent: DataTransferContentType? = null
    private var currentPrimarySelectionContent: DataTransferContentType? = null
    private var currentDragIconDraw: ((Canvas, Scale) -> Unit)? = null
    private var dragIconDirectContext: DirectContext? = null
    private val requestSources = mutableMapOf<RequestId, WindowId>()
    private val notificationSources = mutableMapOf<UInt, WindowId>()

    fun createWindow(decorationMode: WindowDecorationMode) {
        val windowId = nextWindowId
        nextWindowId += 1
        val windowParams = WindowParams(
            windowId = windowId,
            size = LogicalSize.makeWH(width = 640, height = 480),
            minSize = LogicalSize.makeWH(width = 320, height = 240),
            title = "Window $windowId",
            decorationMode = decorationMode,
            renderingMode = renderingMode,
        )

        val window = RotatingBallWindow.createWindow(
            app,
            windowParams,
            glProcFunc,
            desktopSettings,
        ) {
            windows[windowId]?.let { window ->
                onWindowCloseRequest(windowId)
                window.close()
            }
        }
        windows[windowId] = window
        windowClipboardHandlers[windowId] = object : ClipboardHandler {
            override fun copy(content: DataTransferContentType) {
                currentClipboard = content
                app.clipboardPut(content.mimeTypes())
            }

            override fun copyToPrimarySelection(content: DataTransferContentType) {
                currentPrimarySelectionContent = content
                app.primarySelectionPut(content.mimeTypes())
            }

            override fun paste(supportedMimeTypes: List<String>, windowId: WindowId) {
                currentClipboardPasteSerial += 1
                clipboardPasteSerialToWindow[currentClipboardPasteSerial] = windowId
                app.clipboardPaste(currentClipboardPasteSerial, supportedMimeTypes)
            }

            override fun pasteFromPrimarySelection(supportedMimeTypes: List<String>, windowId: WindowId) {
                currentClipboardPasteSerial += 1
                clipboardPasteSerialToWindow[currentClipboardPasteSerial] = windowId
                app.primarySelectionPaste(currentClipboardPasteSerial, supportedMimeTypes)
            }

            override fun startDrag(content: DataTransferContentType, params: StartDragAndDropParams, draw: (Canvas, Scale) -> Unit) {
                currentDragContent = content
                currentDragIconDraw = draw
                window.window.startDragAndDrop(params)
            }

            override fun stopDrag() {
                app.stopDragAndDrop()
            }
        }
    }

    fun handleEvent(event: Event): EventHandlerResult {
        if (event !is Event.MouseMoved && event !is Event.WindowDraw && event !is Event.WindowFrameTick) {
            Logger.info { "$event" }
        }

        return when (event) {
            is Event.ApplicationStarted -> {
                createWindow(decorationMode = WindowDecorationMode.CustomTitlebar(LogicalPixelsInt(50)))
                EventHandlerResult.Stop
            }

            is Event.DisplayConfigurationChange -> EventHandlerResult.Continue
            is Event.DesktopSettingChange -> {
                settingChanged(event.setting)
                EventHandlerResult.Stop
            }

            is Event.WindowDraw -> windows[event.windowId]?.onWindowDraw(event) ?: EventHandlerResult.Continue
            is Event.WindowConfigure -> {
                windows[event.windowId]?.configure(event) ?: EventHandlerResult.Continue
            }

            is Event.MouseMoved -> {
                windows[event.windowId]?.onMouseMoved(event.locationInWindow) ?: EventHandlerResult.Continue
            }

            is Event.DataTransfer -> {
                clipboardPasteSerialToWindow.remove(event.serial)?.let { windowId ->
                    windows[windowId]?.onDataTransfer(event.content)
                } ?: EventHandlerResult.Continue
            }

            is Event.DropPerformed -> {
                windows[event.windowId]?.onDataTransfer(event.content) ?: EventHandlerResult.Continue
            }

            is Event.DragAndDropLeave -> EventHandlerResult.Stop
            is Event.DragIconDraw -> {
                currentDragIconDraw?.let { draw ->
                    val drawImpl = { surface: Surface ->
                        draw(surface.canvas, event.scale)
                        surface.flushAndSubmit()
                        true
                    }
                    dragIconDirectContext?.let {
                        performOpenGlDrawing(event.openGlDrawData.framebuffer, event.size, SurfaceOrigin.TOP_LEFT, it, drawImpl)
                    } ?: run {
                        val openGlInterface = GLAssembledInterface.createFromNativePointers(
                            ctxPtr = glProcFunc.ctxPtr,
                            fPtr = glProcFunc.fPtr,
                        )
                        val directContext = DirectContext.makeGLWithInterface(openGlInterface)
                        dragIconDirectContext = directContext
                        performOpenGlDrawing(
                            event.openGlDrawData.framebuffer,
                            event.size,
                            SurfaceOrigin.TOP_LEFT,
                            directContext,
                            drawImpl,
                        )
                    }
                    EventHandlerResult.Stop
                } ?: EventHandlerResult.Continue
            }

            is Event.DragAndDropFinished -> {
                currentDragIconDraw = null
                dragIconDirectContext = null
                windows[event.windowId]?.onDragAndDropFinished(event.action) ?: EventHandlerResult.Continue
                EventHandlerResult.Stop
            }

            is Event.DragAndDropFeedbackFinished -> {
                currentDragIconDraw = null
                dragIconDirectContext = null
                EventHandlerResult.Continue
            }

            is Event.DataTransferCancelled -> {
                onDataTransferCancelled(event.dataSource)
                EventHandlerResult.Stop
            }

            is Event.DataTransferAvailable -> EventHandlerResult.Continue
            is Event.FileChooserResponse -> {
                Logger.info { "File chooser response: $event" }
                EventHandlerResult.Stop
            }

            is Event.KeyDown -> {
                val windowId = event.windowId
                if (modifiers.shortcutModifiers() == setOf(KeyModifiers.Control) && event.keyCode.value == KeyCode.N) {
                    createWindow(WindowDecorationMode.Server)
                    EventHandlerResult.Stop
                } else if (modifiers.shortcutModifiers() == setOf(KeyModifiers.Control) && event.keyCode.value == KeyCode.P) {
                    val params = ShowNotificationParams(
                        title = "Notification from window $windowId",
                        body = "Clicking this notification will activate window $windowId",
                        soundFilePath = null,
                    )
                    app.requestShowNotification(params)?.let { requestId ->
                        requestSources[requestId] = windowId
                    }
                    EventHandlerResult.Stop
                } else if (modifiers.shortcutModifiers() == setOf(KeyModifiers.Control) && event.keyCode.value == KeyCode.Tab) {
                    windows.firstNotNullOfOrNull { if (it.key == windowId) null else it.value }?.window?.activate(null)
                    EventHandlerResult.Stop
                } else {
                    windows[event.windowId]?.onKeyDown(event, modifiers, windowClipboardHandlers[event.windowId]!!)
                        ?: EventHandlerResult.Continue
                }
            }

            is Event.KeyUp -> EventHandlerResult.Continue
            is Event.ModifiersChanged -> {
                modifiers = event.modifiers
                EventHandlerResult.Stop
            }

            is Event.MouseDown -> windows[event.windowId]?.onMouseDown(
                event,
                modifiers,
                windowClipboardHandlers[event.windowId]!!,
                desktopSettings,
            )
                ?: EventHandlerResult.Continue

            is Event.MouseEntered -> windows[event.windowId]?.onMouseEntered(event.locationInWindow) ?: EventHandlerResult.Continue
            is Event.MouseExited -> EventHandlerResult.Continue
            is Event.MouseUp -> {
                if (event.button == MouseButton.LEFT) {
                    currentDragContent = null
                }
                EventHandlerResult.Continue
            }

            is Event.ScrollWheel -> EventHandlerResult.Continue
            is Event.TextInput -> windows[event.windowId]?.onTextInput(event) ?: EventHandlerResult.Continue
            is Event.WindowKeyboardEnter -> {
                windows[event.windowId]?.onKeyboardFocusChanged(true) ?: EventHandlerResult.Continue
            }

            is Event.WindowKeyboardLeave -> {
                windows[event.windowId]?.onKeyboardFocusChanged(false) ?: EventHandlerResult.Continue
            }

            is Event.WindowScreenChange -> windows[event.windowId]?.onScreenChange(event, app) ?: EventHandlerResult.Continue
            is Event.WindowScaleChanged -> windows[event.windowId]?.onScaleChange(event) ?: EventHandlerResult.Continue
            is Event.NotificationShown -> {
                event.notificationId?.let { notificationId ->
                    requestSources.remove(event.requestId)?.let { requester ->
                        notificationSources[notificationId] = requester
                    } ?: run {
                        app.closeNotification(notificationId)
                    }
                }
                EventHandlerResult.Stop
            }

            is Event.NotificationClosed -> {
                notificationSources.remove(event.notificationId)?.let { windowIdToActivate ->
                    if (event.action != null) {
                        val w = windows[windowIdToActivate]!!
                        w.window.activate(event.activationToken)
                    }
                }
                EventHandlerResult.Stop
            }

            is Event.WindowFrameTick -> {
                windows[event.windowId]?.window?.requestRedraw()
                EventHandlerResult.Stop
            }

            is Event.DragIconFrameTick -> {
                app.requestRedrawDragIcon()
                EventHandlerResult.Stop
            }

            is Event.WindowClosed -> {
                val windowId = event.windowId
                windows.remove(windowId)
                windowClipboardHandlers.remove(windowId)

                requestSources.entries.removeIf { it.value == windowId }
                notificationSources.entries.removeIf { entry ->
                    (entry.value == windowId).also { shouldRemove ->
                        if (shouldRemove) {
                            app.closeNotification(entry.key)
                        }
                    }
                }

                if (windows.isEmpty()) {
                    app.stopEventLoop()
                }

                EventHandlerResult.Stop
            }
        }
    }

    fun settingChanged(s: DesktopSetting) {
        this.desktopSettings.update(s, app)
        windows.values.forEach { it.settingsChanged(desktopSettings) }
    }

    fun queryDragAndDropTarget(queryData: DragAndDropQueryData): DragAndDropQueryResponse {
        return windows[queryData.windowId]!!.queryDragAndDropTarget(queryData.locationInWindow)
    }

    fun getDataTransferData(dataSource: DataSource, mimeType: String): ByteArray {
        val content = when (dataSource) {
            DataSource.Clipboard -> currentClipboard
            DataSource.DragAndDrop -> currentDragContent
            DataSource.PrimarySelection -> currentPrimarySelectionContent
        }
        return when (content) {
            is DataTransferContentType.Text -> {
                check(mimeType == TEXT_MIME_TYPE) { "Unsupported mime type for text content: $mimeType" }
                content.text.encodeToByteArray()
            }

            is DataTransferContentType.UriList -> {
                when (mimeType) {
                    TEXT_MIME_TYPE -> {
                        content.files.joinToString("\n").encodeToByteArray()
                    }

                    URI_LIST_MIME_TYPE -> {
                        content.files.joinToString("\r\n", postfix = "\r\n") { Path(it).toUri().toString() }.encodeToByteArray()
                    }

                    else -> {
                        error("Unsupported mime type: $mimeType")
                    }
                }
            }

            null -> {
                error("Trying to paste from $dataSource with empty content")
            }
        }
    }

    fun onWindowCloseRequest(@Suppress("unused") windowId: WindowId): Boolean {
        return true
    }

    fun onDataTransferCancelled(dataSource: DataSource) {
        when (dataSource) {
            DataSource.Clipboard -> currentClipboard = null
            DataSource.DragAndDrop -> currentDragContent = null
            DataSource.PrimarySelection -> currentPrimarySelectionContent = null
        }
    }

    fun getSurroundingText(windowId: WindowId): TextInputSurroundingText {
        return windows[windowId]!!.getSurroundingText()
    }

    override fun close() {
        windows.values.forEach(AutoCloseable::close)
        windows.clear()
        windowClipboardHandlers.clear()
    }
}

fun main(args: Array<String>) {
    if (args.isNotEmpty()) {
        Logger.info { "args = ${args.contentToString()}" }
    }
    Logger.info { runtimeInfo() }
    KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Debug)
    val app = Application("org.jetbrains.desktop.gtk.skikoSample1")
    val (glProcFunc, renderingMode) = app.getEglProcFunc()?.let { Pair(it, RenderingMode.GL_ES) }
        ?: Pair(app.initializeGl("libGL.so")!!, RenderingMode.GL)
    ApplicationState(app, glProcFunc, renderingMode).use { state ->
        app.runEventLoop(
            ApplicationConfig(
                eventHandler = { state.handleEvent(it) },
                queryDragAndDropTarget = { queryData ->
                    state.queryDragAndDropTarget(queryData)
                },
                getDataTransferData = { dataSource, mimeType ->
                    state.getDataTransferData(dataSource, mimeType)
                },
                windowCloseRequest = { windowId ->
                    state.onWindowCloseRequest(windowId)
                },
                applicationWantsToTerminate = { true },
                getSurroundingText = { windowId ->
                    state.getSurroundingText(windowId)
                },
            ),
        )
    }
}
