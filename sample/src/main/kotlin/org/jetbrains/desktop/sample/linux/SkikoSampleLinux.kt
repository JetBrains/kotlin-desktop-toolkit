package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.ApplicationConfig
import org.jetbrains.desktop.linux.ColorSchemeValue
import org.jetbrains.desktop.linux.DataSource
import org.jetbrains.desktop.linux.DataTransferContent
import org.jetbrains.desktop.linux.DesktopTitlebarAction
import org.jetbrains.desktop.linux.DragAction
import org.jetbrains.desktop.linux.DragAndDropQueryData
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.FileDialog
import org.jetbrains.desktop.linux.FontAntialiasingValue
import org.jetbrains.desktop.linux.FontHintingValue
import org.jetbrains.desktop.linux.FontRgbaOrderValue
import org.jetbrains.desktop.linux.KeyCode
import org.jetbrains.desktop.linux.KeyModifiers
import org.jetbrains.desktop.linux.KotlinDesktopToolkit
import org.jetbrains.desktop.linux.LogLevel
import org.jetbrains.desktop.linux.Logger
import org.jetbrains.desktop.linux.LogicalPixels
import org.jetbrains.desktop.linux.LogicalPoint
import org.jetbrains.desktop.linux.LogicalRect
import org.jetbrains.desktop.linux.LogicalSize
import org.jetbrains.desktop.linux.MouseButton
import org.jetbrains.desktop.linux.PhysicalPoint
import org.jetbrains.desktop.linux.PhysicalSize
import org.jetbrains.desktop.linux.PointerShape
import org.jetbrains.desktop.linux.TextInputContentPurpose
import org.jetbrains.desktop.linux.TextInputContext
import org.jetbrains.desktop.linux.Window
import org.jetbrains.desktop.linux.WindowCapabilities
import org.jetbrains.desktop.linux.WindowDecorationMode
import org.jetbrains.desktop.linux.WindowId
import org.jetbrains.desktop.linux.WindowParams
import org.jetbrains.desktop.linux.WindowResizeEdge
import org.jetbrains.desktop.linux.XdgDesktopSetting
import org.jetbrains.desktop.linux.utf8OffsetToUtf16Offset
import org.jetbrains.desktop.sample.common.runtimeInfo
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.Font
import org.jetbrains.skia.FontMgr
import org.jetbrains.skia.FontStyle
import org.jetbrains.skia.Image
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import org.jetbrains.skia.TextLine
import java.lang.AutoCloseable
import java.net.URI
import java.text.BreakIterator
import kotlin.io.path.Path
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.DurationUnit
import kotlin.time.toDuration

const val TEXT_MIME_TYPE = "text/plain;charset=utf-8"
const val URI_LIST_MIME_TYPE = "text/uri-list"
const val PNG_MIME_TYPE = "image/png"

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

enum class WindowButtonType {
    AppMenu,
    Icon,
    Spacer,
    Title,
    Minimize,
    Maximize,
    Close,
    ;

    internal companion object {
        internal fun fromString(buttonName: String): WindowButtonType {
            return when (buttonName) {
                "appmenu", "menu" -> AppMenu
                "icon" -> Icon
                "spacer" -> Spacer
                "minimize" -> Minimize
                "maximize" -> Maximize
                "close" -> Close
                else -> error("Unknown button name $buttonName")
            }
        }
    }
}

internal data class TitlebarLayout(
    val layoutLeft: List<WindowButtonType>,
    val layoutRight: List<WindowButtonType>,
) {
    internal companion object {
        private fun parseOneSide(buttons: String): List<WindowButtonType> {
            return if (buttons.isEmpty()) {
                emptyList()
            } else {
                buttons.split(',').map(WindowButtonType::fromString)
            }
        }

        internal fun fromString(buttonLayout: String): TitlebarLayout {
            val (buttonsLeftStr, buttonsRightStr) = buttonLayout.split(':')
            return TitlebarLayout(parseOneSide(buttonsLeftStr), parseOneSide(buttonsRightStr))
        }
    }
}

internal data class XdgDesktopSettings(
    var titlebarLayout: TitlebarLayout = TitlebarLayout(
        layoutLeft = listOf(WindowButtonType.Icon),
        layoutRight = listOf(WindowButtonType.Minimize, WindowButtonType.Maximize, WindowButtonType.Close),
    ),
    var doubleClickInterval: Duration = 500.milliseconds,
    var colorScheme: ColorSchemeValue = ColorSchemeValue.NoPreference,
    var accentColor: Int = Color.BLUE,
    var fontAntialiasing: FontAntialiasingValue = FontAntialiasingValue.Grayscale,
    var fontHinting: FontHintingValue = FontHintingValue.Medium,
    var fontRgbaOrder: FontRgbaOrderValue = FontRgbaOrderValue.Rgb,
    var cursorSize: Int? = null,
    var cursorTheme: String? = null,
    var cursorBlink: Boolean = true,
    var cursorBlinkTime: Duration = 1200.toDuration(DurationUnit.MILLISECONDS),
    var cursorBlinkTimeout: Duration = 10.toDuration(DurationUnit.SECONDS),
    var overlayScrolling: Boolean = false,
    var audibleBell: Boolean = true,
    var actionDoubleClickTitlebar: DesktopTitlebarAction = DesktopTitlebarAction.ToggleMaximize,
    var actionMiddleClickTitlebar: DesktopTitlebarAction = DesktopTitlebarAction.None,
    var actionRightClickTitlebar: DesktopTitlebarAction = DesktopTitlebarAction.Menu,
    var middleClickPaste: Boolean = true,
) {
    private fun colorDoubleToInt(v: Double): Int = (v * 255).roundToInt()

    fun update(s: XdgDesktopSetting) {
        when (s) {
            is XdgDesktopSetting.TitlebarLayout -> titlebarLayout = TitlebarLayout.fromString(s.value)
            is XdgDesktopSetting.DoubleClickInterval -> doubleClickInterval = s.value
            is XdgDesktopSetting.ColorScheme -> colorScheme = s.value
            is XdgDesktopSetting.AccentColor -> accentColor = Color.makeARGB(
                a = colorDoubleToInt(s.value.alpha),
                r = colorDoubleToInt(s.value.red),
                g = colorDoubleToInt(s.value.green),
                b = colorDoubleToInt(s.value.blue),
            )
            is XdgDesktopSetting.FontAntialiasing -> fontAntialiasing = s.value
            is XdgDesktopSetting.FontHinting -> fontHinting = s.value
            is XdgDesktopSetting.FontRgbaOrder -> fontRgbaOrder = s.value
            is XdgDesktopSetting.AudibleBell -> audibleBell = s.value
            is XdgDesktopSetting.CursorBlink -> cursorBlink = s.value
            is XdgDesktopSetting.CursorBlinkTime -> cursorBlinkTime = s.value
            is XdgDesktopSetting.CursorBlinkTimeout -> cursorBlinkTimeout = s.value
            is XdgDesktopSetting.OverlayScrolling -> overlayScrolling = s.value
            is XdgDesktopSetting.CursorSize -> cursorSize = s.value
            is XdgDesktopSetting.CursorTheme -> cursorTheme = s.value
            is XdgDesktopSetting.ActionDoubleClickTitlebar -> actionDoubleClickTitlebar = s.value
            is XdgDesktopSetting.ActionMiddleClickTitlebar -> actionMiddleClickTitlebar = s.value
            is XdgDesktopSetting.ActionRightClickTitlebar -> actionRightClickTitlebar = s.value
            is XdgDesktopSetting.MiddleClickPaste -> middleClickPaste = s.value
        }
    }
}

private interface ClipboardHandler {
    fun copy(content: DataTransferContentType)
    fun copyToPrimarySelection(content: DataTransferContentType)
    fun paste(supportedMimeTypes: List<String>)
    fun pasteFromPrimarySelection(supportedMimeTypes: List<String>)
    fun startDrag(content: DataTransferContentType, action: DragAction)
}

private class EditorState {
    private var textInputEnabled: Boolean = false
    private var composedText: String = ""
    private var composedTextStartOffset: Int? = null
    private var composedTextEndOffset: Int? = null
    private var text: StringBuilder = StringBuilder()
    private var cursorVisible = true
    private var cursorOffset: Int = 0
    private var cursorRectangle = LogicalRect(LogicalPoint(0f, 0f), LogicalSize(0f, 0f))
    private var selectionStartOffset: Int? = null
    private var selectionEndOffset: Int? = null
    private var modifiers = setOf<KeyModifiers>()
    private var textLineCreator = TextLineCreator(cachedFontSize = 0f, cachedText = "")
    private var statsTextLineCreator = TextLineCreator(cachedFontSize = 0f, cachedText = "")
    private var pastedImage: Image? = null

    companion object {
        private fun codepointFromOffset(sb: StringBuilder, offset: Int): Short {
            if (offset == 0) {
                return 0
            }
            return sb.codePointCount(0, offset).toShort()
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

    fun createTextInputContext(changeCausedByInputMethod: Boolean): TextInputContext {
        Logger.info { "createTextInputContext: $text" }
        val cursorCodepoint = codepointFromOffset(text, cursorOffset)
        return TextInputContext(
            surroundingText = text.toString(),
            cursorCodepointOffset = cursorCodepoint,
            selectionStartCodepointOffset = selectionStartOffset?.let { codepointFromOffset(text, it) } ?: cursorCodepoint,
            isMultiline = true,
            contentPurpose = TextInputContentPurpose.Normal,
            cursorRectangle = cursorRectangle,
            changeCausedByInputMethod = changeCausedByInputMethod,
        )
    }

    private fun getTextLineStatsString(): String {
        val s = StringBuilder()
        s.append("Cursor pos: $cursorOffset")
        if (selectionStartOffset != null && selectionEndOffset != null) {
            s.append(", selection: $selectionStartOffset - $selectionEndOffset")
        }
        if (composedTextStartOffset != null && composedTextEndOffset != null) {
            s.append(", compose: $composedTextStartOffset - $composedTextEndOffset")
        }
        return s.toString()
    }

    fun draw(canvas: Canvas, y: Float, scale: Float) {
        val textLineStats = statsTextLineCreator.makeTextLine(getTextLineStatsString(), 20 * scale)
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
        val composedTextStartOffset = this.composedTextStartOffset
        val cursorOffset = cursorOffset
        val stringLine = if (composedText.isEmpty()) {
            text
        } else {
            text.substring(0, cursorOffset) + composedText + text.substring(cursorOffset, text.length)
        }
        val textLine = textLineCreator.makeTextLine(stringLine.toString(), SkikoCustomTitlebarLinux.CUSTOM_TITLEBAR_HEIGHT * scale)
        if (composedText.isNotEmpty()) {
            Paint().use { paint ->
                paint.color = Color.YELLOW
                paint.strokeWidth = 5 * scale
                val x0 = textLine.getCoordAtOffset(cursorOffset)
                val x1 = textLine.getCoordAtOffset(cursorOffset + composedText.length)
                canvas.drawLine(x0 = x0, y0 = y + (5 * scale), x1 = x1, y1 = y + (5 * scale), paint = paint)
            }
        }
        val selectionStartOffset = selectionStartOffset
        val selectionEndOffset = selectionEndOffset
        if (selectionStartOffset != null && selectionEndOffset != null) {
            Paint().use { paint ->
                paint.color = Color.BLUE
                val x0 = textLine.getCoordAtOffset(selectionStartOffset)
                val x1 = textLine.getCoordAtOffset(selectionEndOffset)
                canvas.drawRect(r = Rect(left = x0, top = y + textLine.ascent, right = x1, bottom = y + textLine.descent), paint = paint)
            }
        }
        Paint().use { paint ->
            paint.color = Color.WHITE
            canvas.drawTextLine(textLineStats, 0f, (SkikoCustomTitlebarLinux.CUSTOM_TITLEBAR_HEIGHT * scale) + textLineStats.height, paint)
            canvas.drawTextLine(textLine, 0f, y, paint)
        }
        if (cursorVisible) {
            Paint().use { paint ->
                val coord = textLine.getCoordAtOffset(cursorOffset + (composedTextStartOffset ?: 0))

                cursorRectangle = LogicalRect(
                    LogicalPoint(x = coord / scale, y = (y + textLine.ascent) / scale),
                    LogicalSize(width = 5f, height = (textLine.descent - textLine.ascent) / scale),
                )
                paint.color = Color.GREEN
                paint.strokeWidth = cursorRectangle.size.width

                canvas.drawLine(
                    x0 = cursorRectangle.point.x * scale,
                    y0 = cursorRectangle.point.y * scale,
                    x1 = cursorRectangle.point.x * scale,
                    y1 = (cursorRectangle.point.y + cursorRectangle.size.height) * scale,
                    paint = paint,
                )
            }
        }
    }

    fun shortcutModifiers(): Set<KeyModifiers> = modifiers.toMutableSet().also {
        it.remove(KeyModifiers.CapsLock)
        it.remove(KeyModifiers.NumLock)
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
            text.delete(it.first, it.second)
            cursorOffset = it.first
            return true
        }
        return false
    }

    fun typeIn(characters: String) {
        deleteSelection()
        text.insert(cursorOffset, characters)
        cursorOffset += characters.length
    }

    fun onModifiersChanged(event: Event.ModifiersChanged): EventHandlerResult {
        modifiers = event.modifiers
        return EventHandlerResult.Stop
    }

    fun onKeyDown(
        event: Event.KeyDown,
        app: Application,
        window: Window,
        windowState: WindowState,
        clipboardHandler: ClipboardHandler,
    ): EventHandlerResult {
        val shortcutModifiers = shortcutModifiers()
        when (shortcutModifiers) {
            setOf(KeyModifiers.Logo) -> EventHandlerResult.Continue
            setOf(KeyModifiers.Control, KeyModifiers.Shift) -> when (event.keyCode.value) {
                KeyCode.V -> {
                    clipboardHandler.paste(listOf(PNG_MIME_TYPE, URI_LIST_MIME_TYPE, TEXT_MIME_TYPE))
                    EventHandlerResult.Stop
                }

                KeyCode.C -> {
                    clipboardHandler.copy(DataTransferContentType.UriList(EXAMPLE_FILES))
                    EventHandlerResult.Stop
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
                    EventHandlerResult.Stop
                }

                else -> EventHandlerResult.Continue
            }

            setOf(KeyModifiers.Control) -> when (event.keyCode.value) {
                KeyCode.V -> {
                    clipboardHandler.paste(listOf(PNG_MIME_TYPE, TEXT_MIME_TYPE, URI_LIST_MIME_TYPE))
                    EventHandlerResult.Stop
                }

                KeyCode.C -> {
                    getCurrentSelection()?.let { selection ->
                        clipboardHandler.copy(DataTransferContentType.Text(selection))
                        EventHandlerResult.Stop
                    } ?: EventHandlerResult.Continue
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
                    EventHandlerResult.Stop
                }

                KeyCode.M -> {
                    window.startMove()
                    EventHandlerResult.Stop
                }

                else -> EventHandlerResult.Continue
            }
            setOf(KeyModifiers.Shift) -> when (event.keyCode.value) {
                KeyCode.Up -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    selectionEndOffset = 0
                    cursorOffset = 0
                }

                KeyCode.Down -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    val end = text.length
                    selectionEndOffset = end
                    cursorOffset = end
                }

                KeyCode.Left -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    cursorOffset = getPreviousGlyphOffset(text.toString(), cursorOffset)
                    selectionEndOffset = cursorOffset
                }

                KeyCode.Right -> {
                    if (selectionStartOffset == null) {
                        selectionStartOffset = cursorOffset
                    }
                    cursorOffset = getNextGlyphOffset(text.toString(), cursorOffset)
                    selectionEndOffset = cursorOffset
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
            selectionEndOffset = null
        }

        if (textInputEnabled) {
            app.textInputUpdate(createTextInputContext(changeCausedByInputMethod = false))
        }
        return EventHandlerResult.Stop
    }

    fun onDataTransfer(content: DataTransferContent, app: Application): EventHandlerResult {
        if (content.mimeTypes.contains(URI_LIST_MIME_TYPE)) {
            val files = content.data.decodeToString().trimEnd().split("\r\n")
            Logger.info { "Pasted ${files.size} files:" }
            for (file in files) {
                val path = URI(file).path
                Logger.info { path }
            }
        } else if (content.mimeTypes.contains(TEXT_MIME_TYPE)) {
            deleteSelection()
            val pastedText = content.data.decodeToString()
            text.insert(cursorOffset, pastedText)
            cursorOffset += pastedText.length
            selectionStartOffset = null
            selectionEndOffset = null
            if (textInputEnabled) {
                app.textInputUpdate(createTextInputContext(changeCausedByInputMethod = false))
            }
        } else if (content.mimeTypes.contains(PNG_MIME_TYPE)) {
            pastedImage = Image.makeFromEncoded(content.data)
        }
        return EventHandlerResult.Stop
    }

    fun onTextInputAvailability(event: Event.TextInputAvailability, app: Application): EventHandlerResult {
        if (event.available) {
            app.textInputEnable(createTextInputContext(changeCausedByInputMethod = false))
            textInputEnabled = true
        } else {
            app.textInputDisable()
            textInputEnabled = false
        }
        return EventHandlerResult.Stop
    }

    fun onTextInput(event: Event.TextInput, app: Application): EventHandlerResult {
        composedText = ""
        event.deleteSurroundingTextData?.let { deleteSurroundingTextData ->
            val deleteStart = cursorOffset - utf8OffsetToUtf16Offset(text, deleteSurroundingTextData.beforeLengthInBytes)
            val deleteEnd = cursorOffset + utf8OffsetToUtf16Offset(text, deleteSurroundingTextData.afterLengthInBytes)
            this.text.delete(deleteStart, deleteEnd)
        }
        event.commitStringData?.let { commitStringData ->
            commitStringData.text?.let { commitString ->
                this.text.insert(cursorOffset, commitString)
                cursorOffset += commitString.length
            }
        }
        event.preeditStringData?.let { preeditStringData ->
            deleteSelection()
            cursorVisible = !(preeditStringData.cursorBeginBytePos == -1 && preeditStringData.cursorEndBytePos == -1)
            preeditStringData.text?.let { preeditString ->
                composedText = preeditString
                composedTextStartOffset = utf8OffsetToUtf16Offset(preeditString, preeditStringData.cursorBeginBytePos)
                composedTextEndOffset = utf8OffsetToUtf16Offset(preeditString, preeditStringData.cursorEndBytePos)
            }
        } ?: run {
            composedTextStartOffset = null
            composedTextEndOffset = null
            cursorVisible = true
        }
        if (event.deleteSurroundingTextData != null || event.commitStringData != null) {
            app.textInputUpdate(createTextInputContext(changeCausedByInputMethod = true))
        }
        return EventHandlerResult.Stop
    }
}

internal data class TextLineCreator(
    private var cachedFontSize: Float,
    private var cachedText: String,
) {
    private var textLine: TextLine? = null

    init {
        Logger.info { "makeTextLine init: $cachedText" }
    }

    fun makeTextLine(text: String, fontSize: Float): TextLine {
        if (textLine == null || this.cachedText != text || this.cachedFontSize != fontSize) {
            Logger.info { "makeTextLine update: $text" }
            this.cachedText = text
            this.cachedFontSize = fontSize
            val font = FontMgr.default.matchFamilyStyle("sans-serif", FontStyle.BOLD)?.let { typeface ->
                Font(typeface, fontSize)
            }
            textLine = TextLine.make(text, font)
        } else {
//            Logger.info { "makeTextLine: $text == ${this.text}" }
        }
        return textLine!!
    }
}

fun jbIconBytes(): ByteArray {
    return object {}.javaClass.getResource("/jb-logo.png")!!.readBytes()
}

internal class WindowState {
    var active: Boolean = false
    var maximized: Boolean = false
    var fullscreen: Boolean = false
    var capabilities: WindowCapabilities? = null
    var pointerShape: PointerShape = PointerShape.Default

    fun configure(event: Event.WindowConfigure) {
        active = event.active
        maximized = event.maximized
        fullscreen = event.fullscreen
        capabilities = event.capabilities
    }
}

private class ContentArea(
    var origin: LogicalPoint,
    var size: LogicalSize,
) {
    private var markerPosition: LogicalPoint? = null

    fun onMouseMoved(event: Event.MouseMoved): EventHandlerResult {
        markerPosition = LogicalPoint(
            event.locationInWindow.x - origin.x,
            event.locationInWindow.y - origin.y,
        )
        return EventHandlerResult.Continue
    }

    fun onMouseDown(event: Event.MouseDown, clipboardHandler: ClipboardHandler, editorState: EditorState): EventHandlerResult {
        return when (event.button) {
            MouseButton.LEFT -> when (editorState.shortcutModifiers()) {
                setOf(KeyModifiers.Shift) -> {
                    clipboardHandler.startDrag(DataTransferContentType.UriList(EXAMPLE_FILES), DragAction.Move)
                    EventHandlerResult.Stop
                }

                setOf(KeyModifiers.Control) -> {
                    clipboardHandler.startDrag(DataTransferContentType.UriList(EXAMPLE_FILES), DragAction.Copy)
                    EventHandlerResult.Stop
                }

                else -> {
                    editorState.getCurrentSelection()?.let {
                        clipboardHandler.startDrag(DataTransferContentType.Text(it), DragAction.Copy)
                        EventHandlerResult.Stop
                    } ?: EventHandlerResult.Continue
                }
            }

            else -> EventHandlerResult.Continue
        }
    }

    fun draw(canvas: Canvas, time: Long, scale: Float, editorState: EditorState) {
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
        editorState.draw(canvas, contentSize.height / 2f, scale)
        canvas.drawWindowBorders(contentOrigin, contentSize, scale)
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

    private fun Canvas.drawWindowBorders(origin: PhysicalPoint, size: PhysicalSize, scale: Float) = withTranslated(origin) {
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
            paint.color = 0xFFFFFFFF.toInt()
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

private class CustomBorders {
    companion object {
        const val BORDER_SIZE: LogicalPixels = 5f

        fun edgeToPointerShape(edge: WindowResizeEdge): PointerShape {
            return when (edge) {
                WindowResizeEdge.Top -> PointerShape.NResize
                WindowResizeEdge.Bottom -> PointerShape.SResize
                WindowResizeEdge.Left -> PointerShape.WResize
                WindowResizeEdge.TopLeft -> PointerShape.NwResize
                WindowResizeEdge.BottomLeft -> PointerShape.SwResize
                WindowResizeEdge.Right -> PointerShape.EResize
                WindowResizeEdge.TopRight -> PointerShape.NeResize
                WindowResizeEdge.BottomRight -> PointerShape.SeResize
            }
        }
    }

    private var rectangles = ArrayList<Pair<LogicalRect, WindowResizeEdge>>()

    fun configure(event: Event.WindowConfigure) {
        rectangles.clear()
        val edgeSize = LogicalSize(BORDER_SIZE, BORDER_SIZE)
        rectangles.add(Pair(LogicalRect(LogicalPoint.Zero, edgeSize), WindowResizeEdge.TopLeft))
        rectangles.add(Pair(LogicalRect(LogicalPoint(event.size.width - BORDER_SIZE, 0f), edgeSize), WindowResizeEdge.TopRight))
        rectangles.add(Pair(LogicalRect(LogicalPoint(0f, event.size.height - BORDER_SIZE), edgeSize), WindowResizeEdge.BottomLeft))
        rectangles.add(
            Pair(
                LogicalRect(LogicalPoint(event.size.width - BORDER_SIZE, event.size.height - BORDER_SIZE), edgeSize),
                WindowResizeEdge.BottomRight,
            ),
        )

        rectangles.add(Pair(LogicalRect(LogicalPoint.Zero, LogicalSize(BORDER_SIZE, event.size.height)), WindowResizeEdge.Left))
        rectangles.add(
            Pair(
                LogicalRect(LogicalPoint(event.size.width - BORDER_SIZE, 0f), LogicalSize(BORDER_SIZE, event.size.height)),
                WindowResizeEdge.Right,
            ),
        )
        rectangles.add(Pair(LogicalRect(LogicalPoint.Zero, LogicalSize(event.size.width, BORDER_SIZE)), WindowResizeEdge.Top))
        rectangles.add(
            Pair(
                LogicalRect(LogicalPoint(0f, event.size.height - BORDER_SIZE), LogicalSize(event.size.width, BORDER_SIZE)),
                WindowResizeEdge.Bottom,
            ),
        )
    }

    fun toEdge(locationInWindow: LogicalPoint): WindowResizeEdge? {
        for ((rect, edge) in rectangles) {
            if (rect.contains(locationInWindow)) {
                return edge
            }
        }
        return null
    }

    fun onMouseDown(event: Event.MouseDown, window: Window): EventHandlerResult {
        val edge = toEdge(event.locationInWindow)
        return if (edge != null) {
            window.startResize(edge)
            EventHandlerResult.Stop
        } else {
            EventHandlerResult.Continue
        }
    }
}

private class WindowContainer(
    private var customTitlebar: SkikoCustomTitlebarLinux?,
    var customBorders: CustomBorders?,
    val contentArea: ContentArea,
    private var xdgDesktopSettings: XdgDesktopSettings,
    private val requestClose: () -> Unit,
) {
    companion object {
        fun create(windowContentSize: LogicalSize, xdgDesktopSettings: XdgDesktopSettings, requestClose: () -> Unit): WindowContainer {
            val contentArea = ContentArea(LogicalPoint.Zero, windowContentSize)
            return WindowContainer(null, customBorders = null, contentArea, xdgDesktopSettings, requestClose)
        }

        private fun filterUnsupportedButtons(buttons: List<WindowButtonType>, capabilities: WindowCapabilities): List<WindowButtonType> {
            return buttons
                .filter {
                    when (it) {
                        WindowButtonType.AppMenu,
                        WindowButtonType.Icon,
                        WindowButtonType.Spacer,
                        WindowButtonType.Title,
                        WindowButtonType.Close,
                        -> true
                        WindowButtonType.Minimize -> capabilities.minimize
                        WindowButtonType.Maximize -> capabilities.maximize
                    }
                }
        }
    }

    fun settingsChanged(xdgDesktopSettings: XdgDesktopSettings, windowState: WindowState) {
        this.xdgDesktopSettings = xdgDesktopSettings
        windowState.capabilities?.let { capabilities ->
            customTitlebar?.setLayout(
                TitlebarLayout(
                    layoutLeft = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutLeft, capabilities),
                    layoutRight = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutRight, capabilities),
                ),
            )
        }
    }

    fun configure(event: Event.WindowConfigure) {
        val shouldUseCustomTitlebar = when (event.decorationMode) {
            WindowDecorationMode.Client -> !event.fullscreen
            WindowDecorationMode.Server -> false
        }
        if (shouldUseCustomTitlebar) {
            val titlebarLayout = TitlebarLayout(
                layoutLeft = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutLeft, event.capabilities),
                layoutRight = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutRight, event.capabilities),
            )
            val titlebarSize = LogicalSize(width = event.size.width, height = SkikoCustomTitlebarLinux.CUSTOM_TITLEBAR_HEIGHT)
            val titlebar = customTitlebar ?: SkikoCustomTitlebarLinux(
                origin = LogicalPoint.Zero,
                size = titlebarSize,
                titlebarLayout,
                requestClose,
            ).also {
                customTitlebar = it
            }
            titlebar.configure(event, titlebarLayout)
            val customBorders = customBorders ?: CustomBorders().also { customBorders = it }
            customBorders.configure(event)
            contentArea.origin = LogicalPoint(x = 0f, y = titlebar.size.height)
            contentArea.size =
                LogicalSize(width = event.size.width, height = event.size.height - titlebar.size.height)
        } else {
            customTitlebar = null
            contentArea.origin = LogicalPoint(x = 0f, y = 0f)
            contentArea.size = event.size
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

    fun onMouseMoved(event: Event.MouseMoved, window: Window): EventHandlerResult {
        if (customTitlebar?.onMouseMoved(event, window) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        return contentArea.onMouseMoved(event)
    }

    fun onMouseDown(
        event: Event.MouseDown,
        window: Window,
        editorState: EditorState,
        clipboardHandler: ClipboardHandler,
        xdgDesktopSettings: XdgDesktopSettings,
    ): EventHandlerResult {
        if (customBorders?.onMouseDown(event, window) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        if (customTitlebar?.onMouseDown(event) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        if (contentArea.onMouseDown(event, clipboardHandler, editorState) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }

        return if (event.button == MouseButton.MIDDLE) {
            when (editorState.shortcutModifiers()) {
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

    fun onMouseUp(
        event: Event.MouseUp,
        xdgDesktopSettings: XdgDesktopSettings,
        window: Window,
        windowState: WindowState,
    ): EventHandlerResult {
        if (customTitlebar?.onMouseUp(event, xdgDesktopSettings, window, windowState) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        return EventHandlerResult.Continue
    }

    fun draw(canvas: Canvas, time: Long, scale: Float, title: String, editorState: EditorState, windowState: WindowState) {
        val backgroundColor = if (xdgDesktopSettings.colorScheme == ColorSchemeValue.PreferDark) {
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
        customTitlebar?.draw(canvas, scale, xdgDesktopSettings, title, windowState)
        contentArea.draw(canvas, time, scale, editorState)
    }
}

private class RotatingBallWindow(
    private val windowContainer: WindowContainer,
    app: Application,
    windowParams: WindowParams,
) : SkikoWindowLinux(app, windowParams) {
    private val editorState = EditorState()
    private var title: String = windowParams.title
    private var windowState = WindowState()

    companion object {
        fun createWindow(
            app: Application,
            windowParams: WindowParams,
            xdgDesktopSettings: XdgDesktopSettings,
            requestClose: () -> Unit,
        ): RotatingBallWindow {
            val windowSize = LogicalSize(640f, 480f)
            val windowContentSize = windowSize // todo it's incorrect
            val container = WindowContainer.create(windowContentSize, xdgDesktopSettings, requestClose)

            return RotatingBallWindow(container, app, windowParams)
        }
    }

    fun settingsChanged(xdgDesktopSettings: XdgDesktopSettings) {
        windowContainer.settingsChanged(xdgDesktopSettings, windowState)
    }

    fun getDragAndDropSupportedMimeTypes(point: LogicalPoint): List<String> {
        return if (point.x < windowContainer.contentArea.size.width / 2) {
            listOf(URI_LIST_MIME_TYPE, TEXT_MIME_TYPE)
        } else {
            listOf(TEXT_MIME_TYPE, URI_LIST_MIME_TYPE)
        }
    }

    override fun Canvas.draw(size: PhysicalSize, scale: Double, time: Long) {
        val canvas = this
        windowContainer.draw(canvas, time, scale.toFloat(), title, editorState, windowState)
    }

    fun configure(event: Event.WindowConfigure): EventHandlerResult {
        windowState.configure(event)
        windowContainer.configure(event)
        // performDrawing(syncWithCA = true)
        return EventHandlerResult.Stop
    }

    private fun changePointerShape(newPointerShape: PointerShape) {
        if (windowState.pointerShape != newPointerShape) {
            windowState.pointerShape = newPointerShape
            window.setPointerShape(newPointerShape)
        }
    }

    fun onDataTransfer(content: DataTransferContent?, app: Application): EventHandlerResult {
        return content?.let {
            editorState.onDataTransfer(it, app)
        } ?: EventHandlerResult.Stop
    }

    fun onMouseMoved(event: Event.MouseMoved): EventHandlerResult {
        val borderEdge = windowContainer.customBorders?.toEdge(event.locationInWindow)
        return if (borderEdge != null) {
            changePointerShape(CustomBorders.edgeToPointerShape(borderEdge))
            EventHandlerResult.Stop
        } else {
            changePointerShape(PointerShape.Default)
            windowContainer.onMouseMoved(event, window)
        }
    }

    fun onKeyDown(event: Event.KeyDown, app: Application, clipboardHandler: ClipboardHandler): EventHandlerResult {
        if (editorState.shortcutModifiers() == setOf(KeyModifiers.Control) && event.keyCode.value == KeyCode.H) {
            changePointerShape(PointerShape.Hidden)
            return EventHandlerResult.Stop
        }
        return editorState.onKeyDown(event, app, window, windowState, clipboardHandler)
    }

    fun onModifiersChanged(event: Event.ModifiersChanged): EventHandlerResult {
        return editorState.onModifiersChanged(event)
    }

    fun onTextInputAvailability(event: Event.TextInputAvailability, app: Application): EventHandlerResult {
        return editorState.onTextInputAvailability(event, app)
    }

    fun onTextInput(event: Event.TextInput, app: Application): EventHandlerResult {
        return editorState.onTextInput(event, app)
    }

    fun onMouseEntered(): EventHandlerResult {
        return windowContainer.onMouseEntered()
    }

    fun onMouseExited(): EventHandlerResult {
        return windowContainer.onMouseExited()
    }

    fun onMouseDown(
        event: Event.MouseDown,
        clipboardHandler: ClipboardHandler,
        xdgDesktopSettings: XdgDesktopSettings,
    ): EventHandlerResult {
        return windowContainer.onMouseDown(event, window, editorState, clipboardHandler, xdgDesktopSettings)
    }

    fun onMouseUp(event: Event.MouseUp, xdgDesktopSettings: XdgDesktopSettings): EventHandlerResult {
        return windowContainer.onMouseUp(event, xdgDesktopSettings, window, windowState)
    }
}

private class ApplicationState(private val app: Application) : AutoCloseable {
    private val windows = mutableMapOf<WindowId, RotatingBallWindow>()
    private var keyWindowId: WindowId? = null
    private val xdgDesktopSettings = XdgDesktopSettings()
    private val windowClipboardHandlers = mutableMapOf<WindowId, ClipboardHandler>()
    private var currentClipboard: DataTransferContentType? = null
    private var currentClipboardPasteSerial = 0
    private val clipboardPasteSerialToWindow = mutableMapOf<Int, WindowId>()
    private var currentDragContent: DataTransferContentType? = null
    private var currentPrimarySelectionContent: DataTransferContentType? = null

    fun createWindow(useCustomTitlebar: Boolean, forceSoftwareRendering: Boolean = false) {
        val windowId = windows.count().toLong()
        val windowParams = WindowParams(
            windowId = windowId,
            size = LogicalSize(width = 640f, height = 480f),
            title = "Window ${windows.count()}",
            appId = "org.jetbrains.desktop.linux.skikoSample1",
            preferClientSideDecoration = useCustomTitlebar,
            forceSoftwareRendering = forceSoftwareRendering,
        )

        val window = RotatingBallWindow.createWindow(
            app,
            windowParams,
            xdgDesktopSettings,
        ) {
            handleEvent(Event.WindowCloseRequest(windowId))
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

            override fun startDrag(content: DataTransferContentType, action: DragAction) {
                currentDragContent = content
                window.window.startDrag(content.mimeTypes(), action)
            }
        }
    }

    fun handleEvent(event: Event): EventHandlerResult {
        if (event !is Event.MouseMoved && event !is Event.WindowDraw) {
            Logger.info { "$event" }
        }

        return when (event) {
            Event.ApplicationStarted -> {
                createWindow(useCustomTitlebar = true, forceSoftwareRendering = false)
                EventHandlerResult.Stop
            }
            Event.ApplicationWantsToTerminate -> EventHandlerResult.Continue
            Event.ApplicationWillTerminate -> EventHandlerResult.Continue
            is Event.DisplayConfigurationChange -> EventHandlerResult.Continue
            is Event.XdgDesktopSettingChange -> {
                settingChanged(event.setting)
                EventHandlerResult.Stop
            }
            is Event.WindowCloseRequest -> {
                val windowId = event.windowId
                val window = windows[windowId] ?: return EventHandlerResult.Continue
                window.close()
                windows.remove(windowId)
                windowClipboardHandlers.remove(windowId)
                if (windows.isEmpty()) {
                    app.stopEventLoop()
                }
                EventHandlerResult.Stop
            }
            is Event.WindowDraw -> {
                if (windows[event.windowId]?.performDrawing(event) == true) {
                    EventHandlerResult.Stop
                } else {
                    EventHandlerResult.Continue
                }
            }
            is Event.WindowConfigure -> {
                windows[event.windowId]?.configure(event) ?: EventHandlerResult.Continue
            }
            is Event.MouseMoved -> {
                windows[event.windowId]?.onMouseMoved(event) ?: EventHandlerResult.Continue
            }
            is Event.DataTransfer -> {
                clipboardPasteSerialToWindow.remove(event.serial)?.let { windowId ->
                    windows[windowId]?.onDataTransfer(event.content, app)
                } ?: EventHandlerResult.Continue
            }
            is Event.DropPerformed -> {
                windows[event.windowId]?.onDataTransfer(event.content, app) ?: EventHandlerResult.Continue
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
            is Event.KeyDown -> windows[keyWindowId]?.onKeyDown(event, app, windowClipboardHandlers[keyWindowId]!!)
                ?: EventHandlerResult.Continue
            is Event.KeyUp -> EventHandlerResult.Continue
            is Event.ModifiersChanged -> windows[keyWindowId]?.onModifiersChanged(event) ?: EventHandlerResult.Continue
            is Event.MouseDown -> windows[event.windowId]?.onMouseDown(event, windowClipboardHandlers[event.windowId]!!, xdgDesktopSettings)
                ?: EventHandlerResult.Continue
            is Event.MouseEntered -> windows[event.windowId]?.onMouseEntered() ?: EventHandlerResult.Continue
            is Event.MouseExited -> windows[event.windowId]?.onMouseExited() ?: EventHandlerResult.Continue
            is Event.MouseUp -> {
                if (event.button == MouseButton.LEFT) {
                    currentDragContent = null
                }
                windows[event.windowId]?.onMouseUp(event, xdgDesktopSettings) ?: EventHandlerResult.Continue
            }
            is Event.ScrollWheel -> EventHandlerResult.Continue
            is Event.TextInput -> windows[keyWindowId]?.onTextInput(event, app) ?: EventHandlerResult.Continue
            is Event.TextInputAvailability -> windows[event.windowId]?.onTextInputAvailability(event, app) ?: EventHandlerResult.Continue
            is Event.WindowKeyboardEnter -> {
                keyWindowId = event.windowId
                EventHandlerResult.Continue
            }
            is Event.WindowKeyboardLeave -> {
                check(keyWindowId == event.windowId)
                keyWindowId = null
                EventHandlerResult.Continue
            }
            is Event.WindowScaleChanged, is Event.WindowScreenChange -> EventHandlerResult.Continue
        }
    }

    fun settingChanged(s: XdgDesktopSetting) {
        this.xdgDesktopSettings.update(s)
        xdgDesktopSettings.cursorTheme?.let { cursorTheme ->
            xdgDesktopSettings.cursorSize?.let { cursorSize ->
                app.setCursorTheme(cursorTheme, cursorSize)
            }
        }
        windows.values.forEach { it.settingsChanged(xdgDesktopSettings) }
    }

    fun getDragAndDropSupportedMimeTypes(queryData: DragAndDropQueryData): List<String> {
        return windows[queryData.windowId]!!.getDragAndDropSupportedMimeTypes(queryData.point)
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
            DataSource.DragAndDrop -> currentDragContent = null
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
        Logger.info { "args = $args" }
    }
    Logger.info { runtimeInfo() }
    KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Debug)
    val app = Application()
    ApplicationState(app).use { state ->
        app.runEventLoop(
            ApplicationConfig(
                eventHandler = { state.handleEvent(it) },
                getDragAndDropSupportedMimeTypes = { queryData ->
                    state.getDragAndDropSupportedMimeTypes(queryData)
                },
                getDataTransferData = { dataSource, mimeType ->
                    state.getDataTransferData(dataSource, mimeType)
                },
            ),
        )
    }
}
