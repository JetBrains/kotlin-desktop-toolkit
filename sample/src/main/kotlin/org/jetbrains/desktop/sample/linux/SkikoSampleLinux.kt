@file:Suppress("DuplicatedCode")

package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.ApplicationConfig
import org.jetbrains.desktop.linux.ColorSchemeValue
import org.jetbrains.desktop.linux.DataSource
import org.jetbrains.desktop.linux.DataTransferContent
import org.jetbrains.desktop.linux.DesktopSetting
import org.jetbrains.desktop.linux.DesktopTitlebarAction
import org.jetbrains.desktop.linux.DragAndDropAction
import org.jetbrains.desktop.linux.DragAndDropQueryData
import org.jetbrains.desktop.linux.DragAndDropQueryResponse
import org.jetbrains.desktop.linux.DragIconParams
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.FileDialog
import org.jetbrains.desktop.linux.KeyCode
import org.jetbrains.desktop.linux.KeyModifiers
import org.jetbrains.desktop.linux.KotlinDesktopToolkit
import org.jetbrains.desktop.linux.LogLevel
import org.jetbrains.desktop.linux.Logger
import org.jetbrains.desktop.linux.LogicalPixels
import org.jetbrains.desktop.linux.LogicalPoint
import org.jetbrains.desktop.linux.LogicalSize
import org.jetbrains.desktop.linux.MouseButton
import org.jetbrains.desktop.linux.PhysicalSize
import org.jetbrains.desktop.linux.PointerShape
import org.jetbrains.desktop.linux.RenderingMode
import org.jetbrains.desktop.linux.RequestId
import org.jetbrains.desktop.linux.ShowNotificationParams
import org.jetbrains.desktop.linux.StartDragAndDropParams
import org.jetbrains.desktop.linux.SupportedActionsForMime
import org.jetbrains.desktop.linux.TextInputContentHint
import org.jetbrains.desktop.linux.TextInputContentPurpose
import org.jetbrains.desktop.linux.TextInputContext
import org.jetbrains.desktop.linux.Window
import org.jetbrains.desktop.linux.WindowCapabilities
import org.jetbrains.desktop.linux.WindowDecorationMode
import org.jetbrains.desktop.linux.WindowId
import org.jetbrains.desktop.linux.WindowParams
import org.jetbrains.desktop.linux.WindowResizeEdge
import org.jetbrains.desktop.linux.utf8OffsetToUtf16Offset
import org.jetbrains.desktop.sample.common.runtimeInfo
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.DirectContext
import org.jetbrains.skia.GLAssembledInterface
import org.jetbrains.skia.Image
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import org.jetbrains.skia.Surface
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
import kotlin.Double
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
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlin.run
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Duration.Companion.seconds
import kotlin.time.TimeSource
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

private fun Set<KeyModifiers>.shortcutModifiers(): Set<KeyModifiers> = toMutableSet().also {
    it.remove(KeyModifiers.CapsLock)
    it.remove(KeyModifiers.NumLock)
}

internal data class XdgDesktopSettings(
    var titlebarLayout: String? = null,
    var doubleClickInterval: Duration = 500.milliseconds,
    var colorScheme: ColorSchemeValue = ColorSchemeValue.NoPreference,
    var accentColor: Int = Color.BLUE,
    var cursorSize: UInt? = null,
    var cursorTheme: String? = null,
    var actionDoubleClickTitlebar: DesktopTitlebarAction = DesktopTitlebarAction.ToggleMaximize,
    var actionMiddleClickTitlebar: DesktopTitlebarAction = DesktopTitlebarAction.None,
    var actionRightClickTitlebar: DesktopTitlebarAction = DesktopTitlebarAction.Menu,
    var middleClickPaste: Boolean = true,
) {
    private fun colorDoubleToInt(v: Double): Int = (v * 255).roundToInt()

    fun update(s: DesktopSetting) {
        when (s) {
            is DesktopSetting.TitlebarLayout -> titlebarLayout = s.value
            is DesktopSetting.DoubleClickInterval -> doubleClickInterval = s.value
            is DesktopSetting.ColorScheme -> colorScheme = s.value
            is DesktopSetting.AccentColor -> accentColor = Color.makeARGB(
                a = colorDoubleToInt(s.value.alpha),
                r = colorDoubleToInt(s.value.red),
                g = colorDoubleToInt(s.value.green),
                b = colorDoubleToInt(s.value.blue),
            )

            is DesktopSetting.CursorSize -> cursorSize = s.value
            is DesktopSetting.CursorTheme -> cursorTheme = s.value
            is DesktopSetting.ActionDoubleClickTitlebar -> actionDoubleClickTitlebar = s.value
            is DesktopSetting.ActionMiddleClickTitlebar -> actionMiddleClickTitlebar = s.value
            is DesktopSetting.ActionRightClickTitlebar -> actionRightClickTitlebar = s.value
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
    fun startDrag(content: DataTransferContentType, params: StartDragAndDropParams, draw: (Canvas, Double) -> Unit)
}

private data class PreeditData(
    val cursorOffset: Int?,
    val text: List<Pair<String, TextStyle?>>,
)

private class EditorState(val app: Application) {
    private var scale: Double = 1.0
    private var container = LogicalDoubleRect.Zero
    private var textInputAvailable: Boolean = false
    private var textInputEnabled: Boolean = false
    private var preedit: PreeditData? = null
    private var text: StringBuilder = StringBuilder()
    private var cursorOffset: Int = 0
    private var cursorRectangle = LogicalDoubleRect.Zero
    private var selectionStartOffset: Int? = null
    private var selectionEndOffset: Int? = null
    private var textLineCreator = TextLineCreator()
    private var textLine = textLineCreator.makeTextLine("", 0f, Color.WHITE, 0f)
    private var statsTextLineCreator = TextLineCreator()
    private var textLineStats = statsTextLineCreator.makeTextLine("", 0f, Color.WHITE, 0f)
    private var fpsTextLineCreator = TextLineCreator()
    private var drawCallCount = 0
    private var lastFps = 0
    private var lastDrawMeasureTime = TimeSource.Monotonic.markNow()
    private var pastedImage: Image? = null
    private var modifiers: Set<KeyModifiers> = emptySet()
    private var previousTextInputContext: TextInputContext? = null

    companion object {
        const val FONT_SIZE: LogicalPixels = 50.0

        private fun codepointFromOffset(sb: StringBuilder, offset: Int): UShort {
            if (offset == 0) {
                return 0U
            }
            return sb.codePointCount(0, offset).toUShort()
        }

        private fun getPreviousGlyphOffset(sb: StringBuilder, offset: Int): Int {
            if (offset == 0) {
                return 0
            }
            val bi = BreakIterator.getCharacterInstance()
            bi.text = StringBuilderCharacterIterator(sb)
            return bi.preceding(offset)
        }

        private fun getNextGlyphOffset(sb: StringBuilder, offset: Int): Int {
            val bi = BreakIterator.getCharacterInstance()
            bi.text = StringBuilderCharacterIterator(sb)
            return bi.following(offset).let {
                if (it == BreakIterator.DONE) {
                    sb.length
                } else {
                    it
                }
            }
        }
    }

    private fun createTextInputContext(changeCausedByInputMethod: Boolean): TextInputContext {
        Logger.info { "createTextInputContext: $text" }
        val cursorCodepoint = codepointFromOffset(text, cursorOffset)
        return TextInputContext(
            surroundingText = text.toString(),
            cursorCodepointOffset = cursorCodepoint,
            selectionStartCodepointOffset = selectionStartOffset?.let { codepointFromOffset(text, it) } ?: cursorCodepoint,
            hints = setOf(TextInputContentHint.Multiline),
            contentPurpose = TextInputContentPurpose.Normal,
            cursorRectangle = cursorRectangle.round(),
            changeCausedByInputMethod = changeCausedByInputMethod,
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
        val modifiers = modifiers
        if (modifiers.isNotEmpty()) {
            s.append(", modifiers: ${modifiers.joinToString(", ")}")
        }
        return s.toString()
    }

    private fun update() {
        val selectionRange = getSelectionRange()
        textLineStats = statsTextLineCreator.makeTextLine(
            getTextLineStatsString(selectionRange),
            20.0.toSkiko(scale),
            Color.WHITE,
            container.width.toSkiko(scale),
        )

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

        textLine = textLineCreator.makeTextLine(stringLine, fontSize, Color.WHITE, container.width.toSkiko(scale))

        val offsetInPreedit = cursorOffset + (preedit?.cursorOffset ?: 0)
        val lineMetrics = textLine.lineMetrics
        val lastLineMetric = lineMetrics.lastOrNull()
//        for (lineMetric in lineMetrics) {
//            println(lineMetric)
//        }
        val nestedPhysicalXYH: Triple<Float, Float, Float>? = if (lastLineMetric == null) {
            Triple(0f, 0f, textLine.height)
        } else if (preedit != null && preedit.text.isNotEmpty()) {
            if (preedit.cursorOffset == null) {
                null
            } else {
                val preeditText = StringBuilder()
                preedit.text.forEach { preeditText.append(it.first) }

                when (preedit.cursorOffset) {
                    preeditText.length -> {
                        val rect = textLine.getRectsForRange(
                            offsetInPreedit - 1,
                            offsetInPreedit,
                            RectHeightMode.MAX,
                            RectWidthMode.MAX,
                        ).first().rect

                        Triple(rect.right, rect.top, rect.height)
                    }

                    else -> {
                        val preeditStart = getNextGlyphOffset(preeditText, preedit.cursorOffset)
                        val rect = textLine.getRectsForRange(
                            cursorOffset + preeditStart - 1,
                            cursorOffset + preeditStart,
                            RectHeightMode.MAX,
                            RectWidthMode.MAX,
                        ).first().rect

                        Triple(rect.left, rect.top, rect.height)
                    }
                }
            }
        } else if (cursorOffset == text.length && lastLineMetric.width == 0.0) {
            val lineHeight = lastLineMetric.height.toFloat()
            Triple(0f, textLine.height - lineHeight, lineHeight)
        } else {
            val lineMetric = lineMetrics.first { it.startIndex <= offsetInPreedit && it.endIndex >= offsetInPreedit }
            if (offsetInPreedit == lineMetric.startIndex) {
                val lineHeight = lineMetric.height.toFloat()
                Triple(0f, (lineMetric.baseline + lineMetric.descent).toFloat() - lineHeight, lineHeight)
            } else {
                val start = getPreviousGlyphOffset(text, cursorOffset)
                val end = getNextGlyphOffset(text, start)
                val rects = textLine.getRectsForRange(start, end, RectHeightMode.MAX, RectWidthMode.MAX)
//                for (textBox in rects) {
//                    println("textBox: ${textBox.rect}")
//                }
                val rect = rects.first().rect
                Triple(rect.right, rect.top, rect.height)
            }
        }

        if (nestedPhysicalXYH != null) {
            cursorRectangle = LogicalDoubleRect(
                x = container.x + nestedPhysicalXYH.first.toLogicalPixels(scale),
                y = container.y + (textLineStats.height + nestedPhysicalXYH.second).toLogicalPixels(scale),
                width = 2.0,
                height = nestedPhysicalXYH.third.toLogicalPixels(scale),
            )
        }
    }

    fun draw(canvas: Canvas) {
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
                    dst = Rect(0f, 0f, imageRect.width * scale.toFloat(), imageRect.height * scale.toFloat()),
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

        val fpsLineWidth: LogicalPixels = 100.0
        val textLineFps = fpsTextLineCreator.makeTextLine(
            "$lastFps FPS",
            20.0.toSkiko(scale),
            Color.WHITE,
            fpsLineWidth.toSkiko(scale),
        )
        textLineStats.paint(canvas, container.x.toSkiko(scale), container.y.toSkiko(scale))
        textLineFps.paint(canvas, (container.x + container.width - fpsLineWidth).toSkiko(scale), container.y.toSkiko(scale))
        textLine.paint(canvas, container.x.toSkiko(scale), container.y.toSkiko(scale) + textLineStats.height)
        Paint().use { paint ->
            paint.color = Color.WHITE
        }
    }

    private fun getSelectionRange(): Pair<Int, Int>? {
        val selectionStartOffset = selectionStartOffset
        val selectionEndOffset = selectionEndOffset
        if (selectionStartOffset != null && selectionEndOffset != null) {
            val range = Pair(selectionStartOffset, selectionEndOffset)
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
            selectionEndOffset = null
            return true
        }
        return false
    }

    private fun typeIn(characters: String) {
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
    ) {
        val shortcutModifiers = modifiers.shortcutModifiers()
        val changesText = when (shortcutModifiers) {
            setOf(KeyModifiers.Logo) -> false
            setOf(KeyModifiers.Control, KeyModifiers.Shift) -> when (event.keyCode.value) {
                KeyCode.V -> {
                    clipboardHandler.paste(listOf(PNG_MIME_TYPE, URI_LIST_MIME_TYPE, TEXT_MIME_TYPE))
                    false
                }

                KeyCode.C -> {
                    clipboardHandler.copy(DataTransferContentType.UriList(EXAMPLE_FILES))
                    false
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
                    false
                }

                else -> false
            }

            setOf(KeyModifiers.Control) -> when (event.keyCode.value) {
                KeyCode.V -> {
                    clipboardHandler.paste(listOf(PNG_MIME_TYPE, TEXT_MIME_TYPE, URI_LIST_MIME_TYPE))
                    false
                }

                KeyCode.C -> {
                    getCurrentSelection()?.let { selection ->
                        clipboardHandler.copy(DataTransferContentType.Text(selection))
                    }
                    false
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
                    false
                }

                KeyCode.M -> {
                    window.startMove()
                    false
                }

                KeyCode.Tab -> {
                    window.requestInternalActivationToken()
                    false
                }

                else -> false
            }

            setOf(KeyModifiers.Shift) -> when (event.keyCode.value) {
                KeyCode.Up -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    selectionEndOffset = 0
                    cursorOffset = 0
                    true
                }

                KeyCode.Down -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    val end = text.length
                    selectionEndOffset = end
                    cursorOffset = end
                    true
                }

                KeyCode.Left -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    cursorOffset = getPreviousGlyphOffset(text, cursorOffset)
                    selectionEndOffset = cursorOffset
                    true
                }

                KeyCode.Right -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    cursorOffset = getNextGlyphOffset(text, cursorOffset)
                    selectionEndOffset = cursorOffset
                    true
                }

                else -> {
                    val characters = event.characters
                    if (characters != null) {
                        typeIn(characters)
                        true
                    } else {
                        false
                    }
                }
            }

            else -> when (event.keyCode.value) {
                KeyCode.Shift_R -> {
                    val disableTextInput = modifiers.contains(KeyModifiers.Alt)
                    resetTextInput(app, reenable = !disableTextInput, clear = modifiers.contains(KeyModifiers.Control))
                    textInputEnabled = !disableTextInput
                    false
                }

                KeyCode.Tab -> {
                    if (!textInputEnabled) {
                        textInputEnabled = true
                        app.textInputEnable(createTextInputContext(changeCausedByInputMethod = false))
                    }
                    false
                }

                KeyCode.BackSpace -> {
                    if (!deleteSelection() && cursorOffset > 0) {
                        val newCursorOffset = getPreviousGlyphOffset(text, cursorOffset)
                        text.delete(newCursorOffset, cursorOffset)
                        cursorOffset = newCursorOffset
                    }
                    true
                }

                KeyCode.F11 -> {
                    if (windowState.fullscreen) {
                        window.unsetFullScreen()
                    } else {
                        window.setFullScreen()
                    }
                    false
                }

                KeyCode.Up -> {
                    cursorOffset = 0
                    true
                }

                KeyCode.Down -> {
                    cursorOffset = text.length
                    true
                }

                KeyCode.Left -> {
                    cursorOffset = getPreviousGlyphOffset(text, cursorOffset)
                    true
                }

                KeyCode.Right -> {
                    cursorOffset = getNextGlyphOffset(text, cursorOffset)
                    true
                }

                KeyCode.Return, KeyCode.KP_Enter -> {
                    typeIn("\n")
                    true
                }

                else -> {
                    val characters = event.characters
                    if (characters != null) {
                        typeIn(characters)
                        true
                    } else {
                        false
                    }
                }
            }
        }

        if (changesText) {
            preedit = null
            if (shortcutModifiers != setOf(KeyModifiers.Shift)) {
                selectionStartOffset = null
                selectionEndOffset = null
            }
            updateInputMethod(changeCausedByInputMethod = false)
        }
    }

    fun onDataTransfer(content: DataTransferContent) {
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
                updateInputMethod(changeCausedByInputMethod = false)
            }

            PNG_MIME_TYPE -> {
                pastedImage = Image.makeFromEncoded(content.data)
            }
        }
    }

    fun onDragAndDropFinished(action: DragAndDropAction?) {
        if (action == DragAndDropAction.Move) {
            deleteSelection()
            selectionStartOffset = null
            selectionEndOffset = null
        }
    }

    fun onTextInputAvailability(event: Event.TextInputAvailability) {
        if (event.available && textInputEnabled) {
            app.textInputEnable(createTextInputContext(changeCausedByInputMethod = false))
        } else {
            resetTextInput(app, reenable = false, clear = false)
        }
        textInputAvailable = event.available
        update()
    }

    fun resetTextInput(app: Application, reenable: Boolean = true, clear: Boolean = false) {
        if (!textInputAvailable) {
            return
        }

        app.textInputDisable()

        if (clear) {
            cursorOffset = 0
            text.clear()
        }

        if (textInputEnabled && reenable) {
            app.textInputEnable(createTextInputContext(changeCausedByInputMethod = false))
        }
    }

    fun onTextInput(event: Event.TextInput) {
        val deleteSurroundingTextData = event.deleteSurroundingTextData
        val commitStringData = event.commitStringData
        val preeditStringData = event.preeditStringData

        preedit = null
        if (deleteSurroundingTextData != null || commitStringData != null || preeditStringData != null) {
            deleteSelection()
        }

        if (deleteSurroundingTextData != null) {
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

        commitStringData?.text?.let { commitString ->
            this.text.insert(cursorOffset, commitString)
            cursorOffset += commitString.length
        }

        preeditStringData?.text?.let { preeditString ->
            preedit = PreeditData(
                text = listOf(Pair(preeditString, TextLineCreator.preeditStyle())),
                cursorOffset = if (preeditStringData.cursorBeginBytePos == -1) {
                    null
                } else {
                    check(preeditStringData.cursorBeginBytePos >= 0)
                    check(preeditStringData.cursorEndBytePos >= 0)
                    utf8OffsetToUtf16Offset(preeditString, preeditStringData.cursorBeginBytePos.toUInt())
                },
            )
        }

        updateInputMethod(true)
    }

    private fun updateInputMethod(changeCausedByInputMethod: Boolean) {
        update()
        if (textInputAvailable && textInputEnabled) {
            val textInputContext = createTextInputContext(changeCausedByInputMethod)
            if (previousTextInputContext != textInputContext) {
                previousTextInputContext = textInputContext
                app.textInputUpdate(textInputContext)
            }
        }
    }

    fun setContainerRect(rect: LogicalDoubleRect) {
        container = rect
        updateInputMethod(changeCausedByInputMethod = false)
    }

    fun onScaleChanged(newScale: Double) {
        scale = newScale
        updateInputMethod(changeCausedByInputMethod = false)
    }

    fun onModifiersChanged(modifiers: Set<KeyModifiers>) {
        this.modifiers = modifiers
        update()
    }
}

private class WindowState {
    var size: LogicalSize? = null
    var fullscreen: Boolean = false
    var capabilities: WindowCapabilities? = null
    var scale: Double = 1.0

    fun configure(event: Event.WindowConfigure) {
        size = event.size
        fullscreen = event.fullscreen
        capabilities = event.capabilities
    }
}

private class ContentArea {
    var contentRect = LogicalDoubleRect.Zero
    private var markerPosition: LogicalPoint? = null
    private var dragIconTextLineCreator = TextLineCreator()

    companion object {
        const val CONTENT_BORDER_THICKNESS: LogicalPixels = 3.0
    }

    fun onMouseMoved(locationInWindow: LogicalPoint): EventHandlerResult {
        markerPosition = locationInWindow
        return EventHandlerResult.Continue
    }

    fun onMouseDown(
        event: Event.MouseDown,
        clipboardHandler: ClipboardHandler,
        modifiers: Set<KeyModifiers>,
        editorState: EditorState,
    ): EventHandlerResult {
        return when (event.button) {
            MouseButton.LEFT -> when (modifiers.shortcutModifiers()) {
                setOf(KeyModifiers.Alt) -> {
                    val scale = 1.0
                    val fontSize: LogicalPixels = 10.0
                    val skikoTextLine = dragIconTextLineCreator.makeTextLine(
                        EXAMPLE_FILES.joinToString("\n"),
                        fontSize.toSkiko(scale),
                        Color.BLACK,
                        Float.MAX_VALUE,
                    )
                    val dragIconParams = DragIconParams(
                        renderingMode = RenderingMode.Auto,
                        size = skikoTextLine.toLogicalSize(scale),
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
                            fontSize.toSkiko(scale),
                            Color.BLACK,
                            Float.MAX_VALUE,
                        )
                        skikoTextLine.paint(canvas, 0f, 0f)
                    }
                    EventHandlerResult.Stop
                }

                else -> {
                    editorState.getCurrentSelection()?.let { text ->
                        val scale = 1.0
                        val fontSize: LogicalPixels = 12.0
                        val skikoTextLine = dragIconTextLineCreator.makeTextLine(
                            text,
                            fontSize.toSkiko(scale),
                            Color.WHITE,
                            Float.MAX_VALUE,
                        )
                        val dragIconParams = DragIconParams(
                            renderingMode = RenderingMode.Auto,
                            size = skikoTextLine.toLogicalSize(scale),
                        )
                        val content = DataTransferContentType.Text(text)
                        val startDragAndDropParams = StartDragAndDropParams(
                            mimeTypes = content.mimeTypes(),
                            actions = setOf(DragAndDropAction.Copy, DragAndDropAction.Move),
                            dragIconParams,
                        )
                        clipboardHandler.startDrag(content, startDragAndDropParams) { canvas, scale ->
                            canvas.clear(Color.BLUE)
                            val skikoTextLine = dragIconTextLineCreator.makeTextLine(
                                text,
                                fontSize.toSkiko(scale),
                                Color.WHITE,
                                Float.MAX_VALUE,
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

    fun draw(canvas: Canvas, time: Long, scale: Double, editorState: EditorState) {
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
        editorState.draw(canvas)
        drawWindowBorders(canvas, contentRect, scale, CONTENT_BORDER_THICKNESS)
        drawCursor(canvas, contentRect, scale)
    }

    private fun drawSpinningCircle(canvas: Canvas, container: LogicalDoubleRect, scale: Double, t: Long) {
        val width = container.width.toSkiko(scale)
        val height = container.height.toSkiko(scale)
        val angle = (t / 2000f) * 2f * PI
        val r = width / 4
        val x = r * sin(angle).toFloat() + width / 2f
        val y = r * cos(angle).toFloat() + height / 2f
        Paint().use { paint ->
            paint.color = Color.GREEN
            canvas.drawCircle(container.x.toSkiko(scale) + x, container.y.toSkiko(scale) + y, 30f * scale.toFloat(), paint)
        }
    }

    private fun drawWindowBorders(canvas: Canvas, container: LogicalDoubleRect, scale: Double, thickness: LogicalPixels) {
        Paint().use { paint ->
            val len: LogicalPixels = 100.0

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
            paint.color = 0xFFFFFFFF.toInt()
            val bottomY = container.y + (container.height - thickness)
            canvas.drawRect(LogicalDoubleRect(container.x, bottomY, len, thickness).toSkiko(scale), paint)
            canvas.drawRect(LogicalDoubleRect(horizontalCenterX, bottomY, len, thickness).toSkiko(scale), paint)
            canvas.drawRect(LogicalDoubleRect(horizontalRightX, bottomY, len, thickness).toSkiko(scale), paint)
        }
    }

    private fun drawCursor(canvas: Canvas, container: LogicalDoubleRect, scale: Double) {
        markerPosition?.let { curs ->
            if (container.contains(curs)) {
                Paint().use { paint ->
                    paint.color = 0x40FFFFFF
                    canvas.drawRect(
                        LogicalDoubleRect(
                            container.x,
                            curs.y,
                            container.width,
                            2.0,
                        ).toSkiko(scale),
                        paint,
                    )
                    canvas.drawRect(
                        LogicalDoubleRect(
                            curs.x,
                            container.y,
                            2.0,
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
    private var customTitlebar: SkikoCustomTitlebarLinux?,
    var customBorders: SkikoCustomBordersLinux?,
    val contentArea: ContentArea,
    private var xdgDesktopSettings: XdgDesktopSettings,
) {
    companion object {
        fun create(xdgDesktopSettings: XdgDesktopSettings): WindowContainer {
            val contentArea = ContentArea()
            return WindowContainer(null, customBorders = null, contentArea, xdgDesktopSettings)
        }
    }

    fun settingsChanged(xdgDesktopSettings: XdgDesktopSettings, s: DesktopSetting, windowState: WindowState) {
        this.xdgDesktopSettings = xdgDesktopSettings
        if (s is DesktopSetting.TitlebarLayout) {
            customTitlebar?.setLayout(windowState.capabilities, layoutString = s.value)
        }
    }

    fun configure(event: Event.WindowConfigure, window: Window) {
        val shouldUseCustomTitlebar = when (event.decorationMode) {
            WindowDecorationMode.Client -> !event.fullscreen
            WindowDecorationMode.Server -> false
        }
        if (shouldUseCustomTitlebar) {
            val headerRect = LogicalDoubleRect(0.0, 0.0, event.size.width.toDouble(), 55.0)
            val titlebar = customTitlebar ?: SkikoCustomTitlebarLinux(
                headerRect = headerRect,
            ) { window.close() }.also {
                customTitlebar = it
            }
            titlebar.configure(event, headerRect, xdgDesktopSettings.titlebarLayout)
            val customBorders = customBorders ?: SkikoCustomBordersLinux().also { customBorders = it }
            customBorders.configure(event)
            contentArea.contentRect =
                LogicalDoubleRect(0.0, headerRect.height, event.size.width.toDouble(), event.size.height - headerRect.height)
        } else {
            customTitlebar = null
            customBorders = null
            contentArea.contentRect = LogicalDoubleRect(0.0, 0.0, event.size.width.toDouble(), event.size.height.toDouble())
        }
    }

    fun onMouseEntered(): EventHandlerResult {
        if (customTitlebar?.onMouseEntered() == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        return EventHandlerResult.Continue
    }

    fun onMouseExited(): EventHandlerResult {
        if (customTitlebar?.onMouseExited() == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        return EventHandlerResult.Continue
    }

    fun onMouseMoved(locationInWindow: LogicalPoint, window: Window): EventHandlerResult {
        if (customTitlebar?.onMouseMoved(locationInWindow, window) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        return contentArea.onMouseMoved(locationInWindow)
    }

    fun onMouseDown(
        event: Event.MouseDown,
        window: Window,
        editorState: EditorState,
        modifiers: Set<KeyModifiers>,
        clipboardHandler: ClipboardHandler,
        xdgDesktopSettings: XdgDesktopSettings,
    ): EventHandlerResult {
        if (customBorders?.onMouseDown(event, window) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        if (customTitlebar?.onMouseDown(event) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        if (contentArea.onMouseDown(event, clipboardHandler, modifiers, editorState) == EventHandlerResult.Stop) {
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
                    if (xdgDesktopSettings.middleClickPaste) {
                        clipboardHandler.pasteFromPrimarySelection(listOf(PNG_MIME_TYPE, URI_LIST_MIME_TYPE, TEXT_MIME_TYPE))
                        EventHandlerResult.Stop
                    } else {
                        EventHandlerResult.Continue
                    }
                }

                emptySet<KeyModifiers>() -> {
                    window.startResize(WindowResizeEdge.Top)
                    EventHandlerResult.Stop
                }

                else -> EventHandlerResult.Continue
            }
        } else {
            EventHandlerResult.Continue
        }
    }

    fun onMouseUp(event: Event.MouseUp, xdgDesktopSettings: XdgDesktopSettings, window: Window): EventHandlerResult {
        if (customTitlebar?.onMouseUp(event, xdgDesktopSettings, window) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        return EventHandlerResult.Continue
    }

    fun draw(canvas: Canvas, time: Long, title: String, editorState: EditorState, windowState: WindowState) {
        canvas.clear(Color.TRANSPARENT)
        val scale = windowState.scale
        customTitlebar?.draw(canvas, xdgDesktopSettings, title, scale)
        contentArea.draw(canvas, time, scale, editorState)
    }
}

private class RotatingBallWindow(
    private val windowContainer: WindowContainer,
    app: Application,
    windowParams: WindowParams,
) : SkikoWindowLinux(app, windowParams) {
    private val editorState = EditorState(app)
    private var title: String = windowParams.title
    private var windowState = WindowState()
    private var pointerShape: PointerShape = PointerShape.Default

    companion object {
        fun createWindow(app: Application, windowParams: WindowParams, xdgDesktopSettings: XdgDesktopSettings): RotatingBallWindow {
            val container = WindowContainer.create(xdgDesktopSettings)

            return RotatingBallWindow(container, app, windowParams)
        }
    }

    fun settingsChanged(xdgDesktopSettings: XdgDesktopSettings, s: DesktopSetting) {
        windowContainer.settingsChanged(xdgDesktopSettings, s, windowState)
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

    override fun draw(canvas: Canvas, size: PhysicalSize, time: Long) {
        windowContainer.draw(canvas, time, title, editorState, windowState)
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
        windowContainer.configure(event, window)

        val contentRect = windowContainer.contentArea.contentRect
        editorState.setContainerRect(
            contentRect.copy(
                x = contentRect.x + ContentArea.CONTENT_BORDER_THICKNESS,
                width = contentRect.width - (ContentArea.CONTENT_BORDER_THICKNESS * 2),
            ),
        )
        return EventHandlerResult.Stop
    }

    fun onScaleChange(event: Event.WindowScaleChanged): EventHandlerResult {
        windowState.scale = event.newScale
        editorState.onScaleChanged(event.newScale)
        return EventHandlerResult.Stop
    }

    private fun changePointerShape(newPointerShape: PointerShape) {
        if (pointerShape != newPointerShape) {
            pointerShape = newPointerShape
            window.setPointerShape(newPointerShape)
        }
    }

    fun onDataTransfer(content: DataTransferContent?): EventHandlerResult {
        content?.let {
            editorState.onDataTransfer(it)
        }
        return EventHandlerResult.Stop
    }

    fun onDragAndDropFinished(action: DragAndDropAction?): EventHandlerResult {
        editorState.onDragAndDropFinished(action)
        return EventHandlerResult.Stop
    }

    fun onMouseMoved(locationInWindow: LogicalPoint): EventHandlerResult {
        val borderEdge = windowContainer.customBorders?.toEdge(locationInWindow)
        return if (borderEdge != null) {
            changePointerShape(SkikoCustomBordersLinux.edgeToPointerShape(borderEdge))
            EventHandlerResult.Stop
        } else {
            changePointerShape(PointerShape.Default)
            windowContainer.onMouseMoved(locationInWindow, window)
        }
    }

    fun onKeyDown(event: Event.KeyDown, modifiers: Set<KeyModifiers>, clipboardHandler: ClipboardHandler): EventHandlerResult {
        if (modifiers.shortcutModifiers() == setOf(KeyModifiers.Control) && event.keyCode.value == KeyCode.H) {
            changePointerShape(PointerShape.Hidden)
        } else {
            editorState.onKeyDown(event, window, windowState, modifiers, clipboardHandler)
        }
        return EventHandlerResult.Stop
    }

    fun onTextInputAvailability(event: Event.TextInputAvailability): EventHandlerResult {
        editorState.onTextInputAvailability(event)
        return EventHandlerResult.Stop
    }

    fun onTextInput(event: Event.TextInput): EventHandlerResult {
        editorState.onTextInput(event)
        return EventHandlerResult.Stop
    }

    fun onModifiersChanged(modifiers: Set<KeyModifiers>): EventHandlerResult {
        editorState.onModifiersChanged(modifiers)
        return EventHandlerResult.Stop
    }

    fun onMouseEntered(locationInWindow: LogicalPoint): EventHandlerResult {
        if (onMouseMoved(locationInWindow) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        if (windowContainer.onMouseEntered() == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        return windowContainer.onMouseMoved(locationInWindow, window)
    }

    fun onMouseExited(): EventHandlerResult {
        return windowContainer.onMouseExited()
    }

    fun onMouseDown(
        event: Event.MouseDown,
        modifiers: Set<KeyModifiers>,
        clipboardHandler: ClipboardHandler,
        xdgDesktopSettings: XdgDesktopSettings,
        app: Application,
    ): EventHandlerResult {
        editorState.resetTextInput(app, reenable = true, clear = modifiers.contains(KeyModifiers.Control))
        return windowContainer.onMouseDown(event, window, editorState, modifiers, clipboardHandler, xdgDesktopSettings)
    }

    fun onMouseUp(event: Event.MouseUp, xdgDesktopSettings: XdgDesktopSettings): EventHandlerResult {
        return windowContainer.onMouseUp(event, xdgDesktopSettings, window)
    }
}

private class ApplicationState(private val app: Application) : AutoCloseable {
    private var nextWindowId = 0L
    private val windows = mutableMapOf<WindowId, RotatingBallWindow>()
    private var keyWindowId: WindowId? = null
    private var modifiers = setOf<KeyModifiers>()
    private val xdgDesktopSettings = XdgDesktopSettings()
    private val windowClipboardHandlers = mutableMapOf<WindowId, ClipboardHandler>()
    private var currentClipboard: DataTransferContentType? = null
    private var currentClipboardPasteSerial = 0
    private val clipboardPasteSerialToWindow = mutableMapOf<Int, WindowId>()
    private var currentDragContent: DataTransferContentType? = null
    private var currentPrimarySelectionContent: DataTransferContentType? = null
    private var currentDragIconDraw: ((Canvas, Double) -> Unit)? = null
    private var dragIconDirectContext: DirectContext? = null
    private val requestSources = mutableMapOf<RequestId, WindowId>()
    private val notificationSources = mutableMapOf<UInt, WindowId>()

    fun createWindow(useCustomTitlebar: Boolean, renderingMode: RenderingMode) {
        val windowId = nextWindowId
        nextWindowId += 1
        val windowParams = WindowParams(
            windowId = windowId,
            size = LogicalSize(width = 720, height = 480),
            minSize = LogicalSize(320, 240),
            title = "Window $windowId",
            appId = "org.jetbrains.desktop.linux.skikoSample1",
            preferClientSideDecoration = useCustomTitlebar,
            renderingMode = renderingMode,
        )

        val window = RotatingBallWindow.createWindow(
            app,
            windowParams,
            xdgDesktopSettings,
        )
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

            override fun startDrag(content: DataTransferContentType, params: StartDragAndDropParams, draw: (Canvas, Double) -> Unit) {
                currentDragContent = content
                currentDragIconDraw = draw
                window.window.startDragAndDrop(params)
            }
        }
    }

    fun handleEvent(event: Event): EventHandlerResult {
        if (event !is Event.MouseMoved && event !is Event.WindowDraw) {
            Logger.info { "$event" }
        }

        return when (event) {
            Event.ApplicationStarted -> {
                createWindow(useCustomTitlebar = true, renderingMode = RenderingMode.Auto)
                EventHandlerResult.Stop
            }

            Event.ApplicationWantsToTerminate -> EventHandlerResult.Continue
            Event.ApplicationWillTerminate -> EventHandlerResult.Continue
            is Event.DisplayConfigurationChange -> EventHandlerResult.Continue
            is Event.DesktopSettingChange -> {
                settingChanged(event.setting)
                EventHandlerResult.Stop
            }

            is Event.WindowCloseRequest -> {
                val windowId = event.windowId
                val window = windows[windowId] ?: return EventHandlerResult.Continue
                window.close()
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
                    event.softwareDrawData?.let { softwareDrawData ->
                        performSoftwareDrawing(event.size, softwareDrawData, drawImpl)
                    } ?: run {
                        dragIconDirectContext?.let {
                            performOpenGlDrawing(event.size, it, drawImpl)
                        } ?: run {
                            val eglFunc = app.getEglProcFunc()!!
                            val openGlInterface = GLAssembledInterface.createFromNativePointers(
                                ctxPtr = eglFunc.ctxPtr,
                                fPtr = eglFunc.fPtr,
                            )
                            val directContext = DirectContext.makeGLWithInterface(openGlInterface)
                            dragIconDirectContext = directContext
                            performOpenGlDrawing(event.size, directContext, drawImpl)
                        }
                    }
                    EventHandlerResult.Stop
                } ?: EventHandlerResult.Continue
            }

            is Event.DragAndDropFinished -> {
                windows[event.windowId]?.onDragAndDropFinished(event.action) ?: EventHandlerResult.Continue
                currentDragIconDraw = null
                dragIconDirectContext?.close()
                dragIconDirectContext = null
                EventHandlerResult.Stop
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

            is Event.ActivationTokenResponse -> {
                windows.keys.firstOrNull { it != keyWindowId }?.let { windowIdToActivate ->
                    val w = windows[windowIdToActivate]!!
                    w.window.activate(event.token)
                }
                EventHandlerResult.Stop
            }

            is Event.KeyDown -> {
                if (modifiers.shortcutModifiers() == setOf(KeyModifiers.Control) && event.keyCode.value == KeyCode.N) {
                    createWindow(useCustomTitlebar = false, renderingMode = RenderingMode.Auto)
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
                } else {
                    windows[keyWindowId]?.onKeyDown(event, modifiers, windowClipboardHandlers[keyWindowId]!!)
                        ?: EventHandlerResult.Continue
                }
            }

            is Event.KeyUp -> EventHandlerResult.Continue
            is Event.ModifiersChanged -> {
                modifiers = event.modifiers
                windows[keyWindowId]?.onModifiersChanged(event.modifiers) ?: EventHandlerResult.Continue
            }

            is Event.MouseDown -> windows[event.windowId]?.onMouseDown(
                event,
                modifiers,
                windowClipboardHandlers[event.windowId]!!,
                xdgDesktopSettings,
                app,
            )
                ?: EventHandlerResult.Continue

            is Event.MouseEntered -> windows[event.windowId]?.onMouseEntered(event.locationInWindow) ?: EventHandlerResult.Continue
            is Event.MouseExited -> windows[event.windowId]?.onMouseExited() ?: EventHandlerResult.Continue
            is Event.MouseUp -> {
                if (event.button == MouseButton.LEFT) {
                    currentDragContent = null
                }
                windows[event.windowId]?.onMouseUp(event, xdgDesktopSettings) ?: EventHandlerResult.Continue
            }

            is Event.ScrollWheel -> EventHandlerResult.Continue
            is Event.TextInput -> windows[keyWindowId]?.onTextInput(event) ?: EventHandlerResult.Continue
            is Event.TextInputAvailability -> windows[event.windowId]?.onTextInputAvailability(event) ?: EventHandlerResult.Continue
            is Event.WindowKeyboardEnter -> {
                keyWindowId = event.windowId
                EventHandlerResult.Continue
            }

            is Event.WindowKeyboardLeave -> {
                check(keyWindowId == event.windowId)
                keyWindowId = null
                EventHandlerResult.Continue
            }

            is Event.WindowScaleChanged -> windows[event.windowId]?.onScaleChange(event) ?: EventHandlerResult.Continue
            is Event.WindowScreenChange -> EventHandlerResult.Continue
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
                    event.activationToken?.let { activationToken ->
                        val w = windows[windowIdToActivate]!!
                        w.window.activate(activationToken)
                    }
                }
                EventHandlerResult.Stop
            }
        }
    }

    fun settingChanged(s: DesktopSetting) {
        this.xdgDesktopSettings.update(s)
        xdgDesktopSettings.cursorTheme?.let { cursorTheme ->
            xdgDesktopSettings.cursorSize?.let { cursorSize ->
                app.setCursorTheme(cursorTheme, cursorSize)
            }
        }
        windows.values.forEach { it.settingsChanged(xdgDesktopSettings, s) }
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

    fun onDataTransferCancelled(dataSource: DataSource) {
        when (dataSource) {
            DataSource.Clipboard -> currentClipboard = null
            DataSource.DragAndDrop -> {
                currentDragContent = null
                currentDragIconDraw = null
                dragIconDirectContext?.close()
                dragIconDirectContext = null
            }
            DataSource.PrimarySelection -> currentPrimarySelectionContent = null
        }
    }

    override fun close() {
        windows.values.forEach(AutoCloseable::close)
        windows.clear()
        windowClipboardHandlers.clear()
        app.close()
    }
}

fun main(args: Array<String>) {
    if (args.isNotEmpty()) {
        Logger.info { "args = ${args.contentToString()}" }
    }
    Logger.info { runtimeInfo() }
    KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Debug)
    val app = Application()
    ApplicationState(app).use { state ->
        app.runEventLoop(
            ApplicationConfig(
                eventHandler = { state.handleEvent(it) },
                queryDragAndDropTarget = { queryData ->
                    state.queryDragAndDropTarget(queryData)
                },
                getDataTransferData = { dataSource, mimeType ->
                    state.getDataTransferData(dataSource, mimeType)
                },
            ),
        )
    }
}
