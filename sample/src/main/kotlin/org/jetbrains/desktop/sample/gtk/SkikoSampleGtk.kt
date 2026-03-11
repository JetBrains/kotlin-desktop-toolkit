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
import org.jetbrains.desktop.gtk.LogicalPoint
import org.jetbrains.desktop.gtk.LogicalRect
import org.jetbrains.desktop.gtk.LogicalSize
import org.jetbrains.desktop.gtk.MouseButton
import org.jetbrains.desktop.gtk.PhysicalPoint
import org.jetbrains.desktop.gtk.PhysicalSize
import org.jetbrains.desktop.gtk.PointerShape
import org.jetbrains.desktop.gtk.RenderingMode
import org.jetbrains.desktop.gtk.RequestId
import org.jetbrains.desktop.gtk.ShowNotificationParams
import org.jetbrains.desktop.gtk.StartDragAndDropParams
import org.jetbrains.desktop.gtk.SupportedActionsForMime
import org.jetbrains.desktop.gtk.TextInputContentPurpose
import org.jetbrains.desktop.gtk.TextInputContext
import org.jetbrains.desktop.gtk.TextInputContextHint
import org.jetbrains.desktop.gtk.TextInputPreeditAttribute
import org.jetbrains.desktop.gtk.TextInputPreeditUnderlineType
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
import org.jetbrains.skia.paragraph.Paragraph
import org.jetbrains.skia.paragraph.RectHeightMode
import org.jetbrains.skia.paragraph.RectWidthMode
import org.jetbrains.skia.paragraph.TextStyle
import java.lang.AutoCloseable
import java.net.URI
import java.text.BreakIterator
import kotlin.io.path.Path
import kotlin.math.PI
import kotlin.math.ceil
import kotlin.math.cos
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlin.time.Duration.Companion.seconds
import kotlin.time.TimeSource

const val TEXT_MIME_TYPE = "text/plain;charset=utf-8"
const val URI_LIST_MIME_TYPE = "text/uri-list"
const val PNG_MIME_TYPE = "image/png"
const val TEXT_TOP_OFFSET = 200f

// TODO
val EXAMPLE_FILES: List<String> = listOf(
    "/home/nikola/Pictures/Screenshots/Screenshot From 2025-01-15 12-08-34.png",
    "/home/nikola/Pictures/Screenshots/Screenshot From 2025-01-15 13-55-25.png",
//    "/home/nikola/Pictures/Screenshots/Screenshot From 2025-01-15 14-02-45.png",
//    "/etc/hosts",
//    "/boot/efi/",
)

sealed class DataTransferContentType {
    data class Text(val text: String) : DataTransferContentType()
    data class UriList(val files: List<String>) : DataTransferContentType()

    fun mimeTypes(): List<String> {
        return when (this) {
            is Text -> listOf(TEXT_MIME_TYPE)
            is UriList -> listOf(URI_LIST_MIME_TYPE, TEXT_MIME_TYPE)
        }
    }
}

fun KeyCode.isModifierKey(): Boolean {
    return when (this.value) {
        KeyCode.Alt_L, KeyCode.Alt_R,
        KeyCode.Control_L, KeyCode.Control_R,
        KeyCode.Shift_L, KeyCode.Shift_R,
        KeyCode.Super_L, KeyCode.Super_R,
        -> true

        else -> false
    }
}

fun Set<KeyModifiers>.shortcutModifiers(): Set<KeyModifiers> = toMutableSet().also {
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
    fun paste(supportedMimeTypes: List<String>)
    fun pasteFromPrimarySelection(supportedMimeTypes: List<String>)
    fun startDrag(content: DataTransferContentType, params: StartDragAndDropParams, draw: (Canvas, PhysicalSize, Double) -> Unit)
    fun stopDrag()
}

private data class PreeditData(
    val cursorOffset: Int,
    val text: List<Pair<String, TextStyle?>>,
)

private class EditorState {
    private var textInputEnabled: Boolean = false
    private var preedit: PreeditData? = null
    private var text: StringBuilder = StringBuilder()
    private var cursorOffset: Int = 0
    private var cursorRectangle = Rect(0f, 0f, 0f, 0f)
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
        const val FONT_SIZE = 50
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
            cursorRectangle = LogicalRect(
                x = cursorRectangle.left.roundToInt(),
                y = cursorRectangle.top.roundToInt(),
                width = (cursorRectangle.right - cursorRectangle.left).roundToInt(),
                height = (cursorRectangle.bottom - cursorRectangle.top).roundToInt(),
            ),
        )
    }

    private fun getTextLineStatsString(selectionRange: Pair<Int, Int>?): String {
        val s = StringBuilder()
        s.append("Cursor pos: $cursorOffset")
        if (selectionRange != null) {
            s.append(", selection: ${selectionRange.first} - ${selectionRange.second}")
        }
        preedit?.cursorOffset.let {
            s.append(", compose: $it")
        }
        return s.toString()
    }

    fun onTextChanged(windowState: WindowState) {
        val scale = windowState.scale.toFloat()
        val selectionRange = getSelectionRange()
        textLineStats = statsTextLineCreator.makeTextLine(getTextLineStatsString(selectionRange), 20 * scale, Color.WHITE)
        val cursorOffset = cursorOffset
        val fontSize = FONT_SIZE * scale
        val preedit = preedit
        val stringLine: List<Pair<String, TextStyle?>> = if (selectionRange != null) {
            listOf(
                Pair(text.substring(0, selectionRange.first), null),
                Pair(text.substring(selectionRange.first, selectionRange.second), TextLineCreator.selectionStyle(fontSize, Color.WHITE)),
                Pair(text.substring(selectionRange.second, text.length), null),
            )
        } else if (preedit != null) {
            println(preedit)
            buildList {
                if (cursorOffset != 0) {
                    add(Pair(text.substring(0, cursorOffset), null))
                }
                preedit.text.forEach { it.second?.fontSize = fontSize }
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
        val x = if (stringLine.isEmpty() || offset == 0) {
            0f
        } else {
            if (preedit?.text.isNullOrEmpty() || preedit.cursorOffset == 0) {
                val start = getPreviousGlyphOffset(text.toString(), cursorOffset)
                val end = getNextGlyphOffset(text.toString(), start)
                textLine.getRectsForRange(start, end, RectHeightMode.MAX, RectWidthMode.MAX).single().rect.right
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

        cursorRectangle = Rect.makeXYWH(
            l = x / scale,
            t = TEXT_TOP_OFFSET / scale,
            w = 2f,
            h = textLine.ideographicBaseline / scale,
        )
    }

    fun draw(canvas: Canvas, y: Float, w: Float, scale: Float) {
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
                    dst = Rect(0f, 0f, imageRect.width * scale, imageRect.height * scale),
                    paint,
                )
            }
        }

        // Draw text caret
        Paint().use { paint ->
            paint.color = Color.GREEN
            paint.strokeWidth = cursorRectangle.width * scale

            canvas.drawLine(
                x0 = cursorRectangle.left * scale,
                y0 = cursorRectangle.top * scale,
                x1 = cursorRectangle.left * scale,
                y1 = cursorRectangle.bottom * scale,
                paint = paint,
            )
        }

        val textLineFps = fpsTextLineCreator.makeTextLine("$lastFps FPS", 20 * scale, Color.WHITE)
        textLineStats.paint(canvas, 10f * scale, FONT_SIZE * scale)
        textLineFps.paint(canvas, w - (100f * scale), FONT_SIZE * scale)
        textLine.paint(canvas, 0f, y)
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
    ): EventHandlerResult {
        val shortcutModifiers = modifiers.shortcutModifiers()
        when (shortcutModifiers) {
            setOf(KeyModifiers.Control, KeyModifiers.Shift) -> when (event.keyCode.value) {
                KeyCode.V -> {
                    clipboardHandler.paste(listOf(PNG_MIME_TYPE, URI_LIST_MIME_TYPE, TEXT_MIME_TYPE))
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
                    clipboardHandler.paste(listOf("application/fleet-multi-caret"))
                    clipboardHandler.paste(listOf(PNG_MIME_TYPE, TEXT_MIME_TYPE, URI_LIST_MIME_TYPE))
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

        onTextChanged(windowState)
        if (textInputEnabled) {
            window.textInputUpdate(createTextInputContext())
        }
        return EventHandlerResult.Stop
    }

    fun onDataTransfer(content: DataTransferContent, window: Window, windowState: WindowState): EventHandlerResult {
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
                onTextChanged(windowState)
                if (textInputEnabled) {
                    window.textInputUpdate(createTextInputContext())
                }
            }

            PNG_MIME_TYPE -> {
                pastedImage = Image.makeFromEncoded(content.data)
            }
        }
        return EventHandlerResult.Stop
    }

    fun onDragAndDropFinished(action: DragAndDropAction?): EventHandlerResult {
        if (action == DragAndDropAction.Move) {
            deleteSelection()
        }
        return EventHandlerResult.Stop
    }

    fun onTextInputAvailability(available: Boolean, window: Window): EventHandlerResult {
        if (available) {
            window.textInputEnable(createTextInputContext())
            textInputEnabled = true
        } else {
            window.textInputDisable()
            textInputEnabled = false
        }
        return EventHandlerResult.Stop
    }

    fun onTextInput(event: Event.TextInput, window: Window, windowState: WindowState): EventHandlerResult {
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
                val utf8Len = preeditText.encodeToByteArray().size
                val attributes = preeditStringData.attributes + listOf(
                    TextInputPreeditAttribute(
                        beginBytePos = utf8Len.toUInt(),
                        endBytePos = utf8Len.toUInt(),
                        underline = TextInputPreeditUnderlineType.None,
                        foregroundHighlight = false,
                        backgroundHighlight = false,
                        strikethrough = false,
                        bold = false,
                        italic = false,
                    ),
                )
                attributes.forEach {
                    val startOffset = utf8OffsetToUtf16Offset(preeditText, it.beginBytePos)
                    if ((previousEndOffset == null && it.beginBytePos != 0U) ||
                        (previousEndOffset != null && startOffset > previousEndOffset)
                    ) {
                        val substring = preeditText.substring(previousEndOffset ?: 0, startOffset)
                        add(Pair(substring, null))
                    }
                    val endOffset = utf8OffsetToUtf16Offset(preeditText, it.endBytePos)
                    if (startOffset != endOffset) {
                        val textStyle = TextLineCreator.preeditStyle(it)
                        val substring = preeditText.substring(startOffset, endOffset)
                        Logger.info { "Preedit style from $startOffset to $endOffset ($substring): $it" }
                        add(Pair(substring, textStyle))
                    }
                    previousEndOffset = endOffset
                }
            }

            preedit = PreeditData(
                text = textWithStyles,
                cursorOffset = preeditStringData.text?.let { preeditString ->
                    check(preeditStringData.cursorBytePos >= 0)
                    utf8OffsetToUtf16Offset(preeditString, preeditStringData.cursorBytePos.toUInt())
                } ?: 0,
            )
        }
        onTextChanged(windowState)
        if (preedit != null || event.deleteSurroundingTextData != null || event.commitStringData != null) {
            window.textInputUpdate(createTextInputContext())
        }
        return EventHandlerResult.Stop
    }
}

internal class WindowState {
    var active: Boolean = false
    var maximized: Boolean = false
    var fullscreen: Boolean = false
    var pointerShape: PointerShape = PointerShape.Default
    var scale: Double = 1.0
    var size: LogicalSize = LogicalSize(width = 0, height = 0)
    var insetStart: LogicalSize = LogicalSize(width = 0, height = 0)
    var insetEnd: LogicalSize = LogicalSize(width = 0, height = 0)

    fun configure(event: Event.WindowConfigure) {
        active = event.active
        maximized = event.maximized
        fullscreen = event.fullscreen
        size = event.size
        insetStart = event.insetStart
        insetEnd = event.insetEnd
    }
}

fun Paragraph.toLogicalSize(): LogicalSize {
    return LogicalSize(width = ceil(maxIntrinsicWidth).roundToInt(), height = ceil(height).roundToInt())
}

private class ContentArea(
    var origin: LogicalPoint,
    var size: LogicalSize,
) {
    private var markerPosition: LogicalPoint? = null
    private var dragIconTextLineCreator = TextLineCreator()

    fun onMouseMoved(locationInWindow: LogicalPoint): EventHandlerResult {
        markerPosition = LogicalPoint(
            locationInWindow.x - origin.x,
            locationInWindow.y - origin.y,
        )
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
        return if (event.locationInWindow.y <= windowState.insetStart.height &&
            (x <= windowState.insetStart.width || x >= windowState.size.width - windowState.insetEnd.width)
        ) {
            EventHandlerResult.Continue
        } else {
            when (event.button) {
                MouseButton.LEFT -> when (modifiers.shortcutModifiers()) {
                    setOf(KeyModifiers.Alt) -> {
                        val skikoTextLine = dragIconTextLineCreator.makeTextLine(
                            EXAMPLE_FILES.joinToString("\n"),
                            10f,
                            Color.BLACK,
                        )
                        val dragIconParams = DragIconParams(
                            renderingMode = RenderingMode.Auto,
                            size = skikoTextLine.toLogicalSize(),
                        )
                        println(dragIconParams)
                        val content = DataTransferContentType.UriList(EXAMPLE_FILES)
                        val startDragAndDropParams = StartDragAndDropParams(
                            mimeTypes = content.mimeTypes(),
                            actions = setOf(DragAndDropAction.Copy),
                            dragIconParams,
                        )
                        clipboardHandler.startDrag(content, startDragAndDropParams) { canvas, physicalSize, scale ->
                            Paint().use { paint ->
                                paint.color = Color.GREEN
                                canvas.drawRect(
                                    Rect.makeXYWH(0f, 0f, physicalSize.width.toFloat(), physicalSize.height.toFloat()),
                                    paint,
                                )
                            }
                            val skikoTextLine = dragIconTextLineCreator.makeTextLine(
                                EXAMPLE_FILES.joinToString("\n"),
                                (10f * scale).toFloat(),
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
                                EditorState.FONT_SIZE.toFloat(),
                                Color.WHITE,
                            )
                            val dragIconParams = DragIconParams(
                                renderingMode = RenderingMode.Auto,
                                size = skikoTextLine.toLogicalSize(),
                            )
                            val content = DataTransferContentType.Text(it)
                            val startDragAndDropParams = StartDragAndDropParams(
                                mimeTypes = content.mimeTypes(),
                                actions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                                dragIconParams,
                            )
                            clipboardHandler.startDrag(content, startDragAndDropParams) { canvas, physicalSize, scale ->
                                Paint().use { paint ->
                                    paint.color = Color.BLUE
                                    canvas.drawRect(
                                        Rect.makeXYWH(0f, 0f, physicalSize.width.toFloat(), physicalSize.height.toFloat()),
                                        paint,
                                    )
                                }
                                val skikoTextLine = dragIconTextLineCreator.makeTextLine(
                                    it,
                                    (EditorState.FONT_SIZE * scale).toFloat(),
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

    fun draw(canvas: Canvas, time: Long, scale: Float, editorState: EditorState, accentColor: Int) {
        val contentOrigin = origin.toPhysical(scale)
        val contentSize = size.toPhysical(scale)
        Paint().use { paint ->
            paint.color = 0x77264653
            canvas.drawRect(
                Rect.makeXYWH(
                    contentOrigin.x.toFloat(),
                    contentOrigin.y.toFloat(),
                    contentSize.width.toFloat(),
                    contentSize.height.toFloat(),
                ),
                paint,
            )
        }
        canvas.withTranslated(contentOrigin) {
            Paint().use { paint ->
                paint.color = Color.WHITE
                paint.strokeWidth = scale
                canvas.drawLine(
                    contentSize.width.toFloat(),
                    0f,
                    0f,
                    contentSize.height.toFloat(),
                    paint,
                )
            }
        }
        canvas.drawSpinningCircle(contentOrigin, contentSize, time)
        editorState.draw(canvas, TEXT_TOP_OFFSET, contentSize.width.toFloat(), scale)
        canvas.drawWindowBorders(contentOrigin, contentSize, scale, accentColor)
        canvas.drawCursor(contentOrigin, contentSize, scale)
    }

    private fun Canvas.drawSpinningCircle(origin: PhysicalPoint, size: PhysicalSize, t: Long) = withTranslated(origin) {
        val width = size.width.toFloat()
        val height = size.height.toFloat()
        val angle = (t / 2000f) * 2f * PI
        val r = width / 4
        val x = r * sin(angle).toFloat() + width / 2f
        val y = r * cos(angle).toFloat() + height / 2f
        Paint().use { paint ->
            paint.color = Color.GREEN
            drawCircle(x, y, 30f, paint)
        }
    }

    private fun Canvas.drawWindowBorders(origin: PhysicalPoint, size: PhysicalSize, scale: Float, accentColor: Int) =
        withTranslated(origin) {
            val width = size.width.toFloat()
            val height = size.height.toFloat()
            Paint().use { paint ->
                val barSize = 3 * scale

                // left
                paint.color = 0xFFe76f51.toInt()
                drawRect(Rect.makeXYWH(0f, 0f, barSize, 100 * scale), paint)
                drawRect(Rect.makeXYWH(0f, height / 2 - 50 * scale, barSize, 100 * scale), paint)
                drawRect(Rect.makeXYWH(0f, height - 100 * scale, barSize, 100 * scale), paint)

                // top
                paint.color = 0xFF2a9d8f.toInt()
                drawRect(Rect.makeXYWH(0f, 0f, 100 * scale, barSize), paint)
                drawRect(Rect.makeXYWH(width / 2 - 50 * scale, 0f, 100 * scale, barSize), paint)
                drawRect(Rect.makeXYWH(width - 100 * scale, 0f, 100 * scale, barSize), paint)

                // right
                paint.color = 0xFFe9c46a.toInt()
                drawRect(Rect.makeXYWH(width - barSize, 0f, barSize, 100 * scale), paint)
                drawRect(Rect.makeXYWH(width - barSize, height / 2 - 50 * scale, barSize, 100 * scale), paint)
                drawRect(Rect.makeXYWH(width - barSize, height - 100 * scale, barSize, 100 * scale), paint)

                // bottom
                paint.color = accentColor
                drawRect(Rect.makeXYWH(0f, height - barSize, 100 * scale, barSize), paint)
                drawRect(Rect.makeXYWH(width / 2 - 50 * scale, height - barSize, 100 * scale, barSize), paint)
                drawRect(Rect.makeXYWH(width - 100 * scale, height - barSize, 100 * scale, barSize), paint)
            }
        }

    private fun Canvas.drawCursor(origin: PhysicalPoint, size: PhysicalSize, scale: Float) = withTranslated(origin) {
        val canvas = this
        markerPosition?.let { curs ->
            val positive = curs.x > 0 && curs.y > 0
            val inBox = curs.x < size.width && curs.y < size.height
            if (positive && inBox) {
                val x = curs.x
                val y = curs.y
                val width = size.width.toFloat()
                val height = size.height.toFloat()

                Paint().use { paint ->
                    paint.color = 0x40FFFFFF
                    canvas.drawRect(Rect.makeXYWH(0f, y * scale, width, 2 * scale), paint)
                    canvas.drawRect(Rect.makeXYWH(x * scale, 0f, 2 * scale, height), paint)
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

        fun create(windowContentSize: LogicalSize, desktopSettings: DesktopSettings, requestClose: () -> Unit): WindowContainer {
            val contentArea = ContentArea(LogicalPoint.Zero, windowContentSize)
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

    fun configure(event: Event.WindowConfigure, title: String, scale: Double) {
        val decorationMode = event.decorationMode
        customTitle = when (decorationMode) {
            WindowDecorationMode.Server -> {
                null
            }
            is WindowDecorationMode.CustomTitlebar -> {
                val font = FontMgr.default.matchFamilyStyle("sans-serif", FontStyle.BOLD)?.let { typeface ->
                    Font(typeface, (decorationMode.height * scale).toFloat())
                }
                TextLine.make(title, font)
            }
        }
        contentArea.origin = LogicalPoint(x = 0f, y = 0f)
        contentArea.size = event.size
    }

    fun onScaleChange(windowState: WindowState, title: String) {
        val font = FontMgr.default.matchFamilyStyle("sans-serif", FontStyle.BOLD)?.let { typeface ->
            Font(typeface, (windowState.insetStart.height * windowState.scale).toFloat())
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
                        clipboardHandler.pasteFromPrimarySelection(listOf(PNG_MIME_TYPE, URI_LIST_MIME_TYPE, TEXT_MIME_TYPE))
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

    fun draw(canvas: Canvas, time: Long, scale: Float, editorState: EditorState, windowState: WindowState) {
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

            canvas.drawRect(Rect.makeXYWH(0f, 0f, windowState.insetStart.width * scale, windowState.insetStart.height * scale), paint)
            canvas.drawRect(
                Rect.makeXYWH(
                    (windowState.size.width - windowState.insetEnd.width) * scale,
                    0f,
                    windowState.insetEnd.width * scale,
                    windowState.insetEnd.height * scale,
                ),
                paint,
            )
        }

        customTitle?.let {
            Paint().use { paint ->
                paint.color = if (windowState.active) Color.WHITE else COLOR_LIGHT_GRAY
                canvas.drawTextLine(
                    it,
                    windowState.insetStart.width.toFloat() * scale,
                    windowState.insetStart.height.toFloat() * scale,
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
            val container = WindowContainer.create(windowParams.size, desktopSettings, requestClose)
            return RotatingBallWindow(container, app, windowParams, glProcFunc)
        }
    }

    fun settingsChanged(desktopSettings: DesktopSettings) {
        windowContainer.settingsChanged(desktopSettings)
    }

    fun queryDragAndDropTarget(locationInWindow: LogicalPoint): DragAndDropQueryResponse {
        val response = if (locationInWindow.x < windowContainer.contentArea.size.width / 2) {
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

    override fun Canvas.draw(size: PhysicalSize, scale: Double, time: Long) {
        val canvas = this
        windowContainer.draw(canvas, time, scale.toFloat(), editorState, windowState)
    }

    fun onWindowDraw(event: Event.WindowDraw): EventHandlerResult {
        return if (performDrawing(event, windowState.scale)) {
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
        return content?.let {
            editorState.onDataTransfer(it, window, windowState)
        } ?: EventHandlerResult.Stop
    }

    fun onDragAndDropFinished(action: DragAndDropAction?): EventHandlerResult {
        return editorState.onDragAndDropFinished(action)
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
        return editorState.onKeyDown(event, window, windowState, modifiers, clipboardHandler)
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

    fun onTextInputAvailability(available: Boolean): EventHandlerResult {
        return editorState.onTextInputAvailability(available, window)
    }

    fun onTextInput(event: Event.TextInput): EventHandlerResult {
        return editorState.onTextInput(event, window, windowState)
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
    private var keyWindowId: WindowId? = null
    private var modifiers = setOf<KeyModifiers>()
    private val desktopSettings = DesktopSettings()
    private val windowClipboardHandlers = mutableMapOf<WindowId, ClipboardHandler>()
    private var currentClipboard: DataTransferContentType? = null
    private var currentClipboardPasteSerial = 0
    private val clipboardPasteSerialToWindow = mutableMapOf<Int, WindowId>()
    private var currentDragContent: DataTransferContentType? = null
    private var currentPrimarySelectionContent: DataTransferContentType? = null
    private var currentDragIconDraw: ((Canvas, PhysicalSize, Double) -> Unit)? = null
    private var dragIconDirectContext: DirectContext? = null
    private val requestSources = mutableMapOf<RequestId, WindowId>()
    private val notificationSources = mutableMapOf<UInt, WindowId>()

    fun createWindow(decorationMode: WindowDecorationMode) {
        val windowId = nextWindowId
        nextWindowId += 1
        val windowParams = WindowParams(
            windowId = windowId,
            size = LogicalSize(width = 640, height = 480),
            minSize = LogicalSize(320, 240),
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

            override fun paste(supportedMimeTypes: List<String>) {
                currentClipboardPasteSerial += 1
                clipboardPasteSerialToWindow[currentClipboardPasteSerial] = keyWindowId!!
                app.clipboardPaste(currentClipboardPasteSerial, supportedMimeTypes)
            }

            override fun pasteFromPrimarySelection(supportedMimeTypes: List<String>) {
                currentClipboardPasteSerial += 1
                clipboardPasteSerialToWindow[currentClipboardPasteSerial] = keyWindowId!!
                app.primarySelectionPaste(currentClipboardPasteSerial, supportedMimeTypes)
            }

            override fun startDrag(
                content: DataTransferContentType,
                params: StartDragAndDropParams,
                draw: (Canvas, PhysicalSize, Double) -> Unit,
            ) {
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
            Event.ApplicationStarted -> {
                createWindow(decorationMode = WindowDecorationMode.CustomTitlebar(50))
                EventHandlerResult.Stop
            }
            is Event.DisplayConfigurationChange -> EventHandlerResult.Continue
            is Event.DesktopSettingChange -> {
                settingChanged(event.setting)
                EventHandlerResult.Stop
            }
            is Event.WindowDraw -> windows[event.windowId]?.onWindowDraw(event) ?: EventHandlerResult.Continue
            is Event.WindowConfigure -> {
                windows[event.windowId]?.configure(event).also {
                    if (event.active) {
                        keyWindowId = event.windowId
                    }
                } ?: EventHandlerResult.Continue
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
                        draw(surface.canvas, event.size, event.scale)
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
                if (modifiers.shortcutModifiers() == setOf(KeyModifiers.Control) && event.keyCode.value == KeyCode.N) {
                    createWindow(WindowDecorationMode.CustomTitlebar(50))
                    EventHandlerResult.Stop
                } else if (modifiers.shortcutModifiers() == setOf(KeyModifiers.Control) && event.keyCode.value == KeyCode.P) {
                    val windowId = keyWindowId!!
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
                    windows.firstNotNullOfOrNull { if (it.key == keyWindowId) null else it.value }?.window?.activate(null)
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
                keyWindowId = event.windowId
                windows[event.windowId]?.onTextInputAvailability(true) ?: EventHandlerResult.Continue
            }
            is Event.WindowKeyboardLeave -> {
                if (keyWindowId == event.windowId) {
                    keyWindowId = null
                }
                windows[event.windowId]?.onTextInputAvailability(false) ?: EventHandlerResult.Continue
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

            Event.DragIconFrameTick -> {
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
        Logger.info { "args = $args" }
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
