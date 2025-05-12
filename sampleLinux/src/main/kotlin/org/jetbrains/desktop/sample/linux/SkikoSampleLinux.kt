package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.ApplicationConfig
import org.jetbrains.desktop.linux.ColorSchemeValue
import org.jetbrains.desktop.linux.DataSource
import org.jetbrains.desktop.linux.DragAction
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.FontAntialiasingValue
import org.jetbrains.desktop.linux.FontHintingValue
import org.jetbrains.desktop.linux.FontRgbaOrderValue
import org.jetbrains.desktop.linux.KeyModifiers
import org.jetbrains.desktop.linux.KeySym
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
import org.jetbrains.desktop.linux.Timestamp
import org.jetbrains.desktop.linux.Window
import org.jetbrains.desktop.linux.WindowCapabilities
import org.jetbrains.desktop.linux.WindowId
import org.jetbrains.desktop.linux.WindowParams
import org.jetbrains.desktop.linux.WindowResizeEdge
import org.jetbrains.desktop.linux.XdgDesktopSetting
import org.jetbrains.desktop.linux.utf8OffsetToUtf16Offset
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
import java.nio.file.Files
import java.nio.file.Path
import java.text.BreakIterator
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.DurationUnit
import kotlin.time.toDuration

const val TEXT_MIME_TYPE = "text/plain;charset=utf-8"
const val URI_LIST_MIME_TYPE = "text/uri-list"

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
                "appmenu" -> AppMenu
                "icon" -> Icon
                "spacer" -> Spacer
                "minimize" -> Minimize
                "maximize" -> Maximize
                "close" -> Close
                else -> error("Unknown button name {button_name}")
            }
        }
    }
}

data class TitlebarLayout(val layoutLeft: List<WindowButtonType>, val layoutRight: List<WindowButtonType>) {
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

data class XdgDesktopSettings(
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
        }
    }
}

class EditorState() {
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
    var modifiers = KeyModifiers(
        capsLock = false,
        shift = false,
        control = false,
        alt = false,
        logo = false,
        numLock = false,
    )
    private var textLineCreator = TextLineCreator(cachedFontSize = 0f, cachedText = "")
    private var statsTextLineCreator = TextLineCreator(cachedFontSize = 0f, cachedText = "")
    var currentClipboard: DataTransferContentType? = null

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
        val composedTextStartOffset = this.composedTextStartOffset
        val cursorOffset = cursorOffset
        val stringLine = if (composedText.isEmpty()) {
            text
        } else {
            text.substring(0, cursorOffset) + composedText + text.substring(cursorOffset, text.length)
        }
        val textLine = textLineCreator.makeTextLine(stringLine.toString(), CustomTitlebar.CUSTOM_TITLEBAR_HEIGHT * scale)
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
            canvas.drawTextLine(textLineStats, 0f, (CustomTitlebar.CUSTOM_TITLEBAR_HEIGHT * scale) + textLineStats.height, paint)
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

    fun handleEvent(event: Event, app: Application, window: Window): EventHandlerResult {
        return when (event) {
            is Event.ModifiersChanged -> {
                modifiers = event.modifiers
                EventHandlerResult.Stop
            }
            is Event.KeyDown -> {
                if (modifiers.logo) {
                    EventHandlerResult.Continue
                } else if (modifiers.control) {
                    when (event.key.value) {
                        KeySym.V -> {
                            window.clipboardPaste(0, listOf(URI_LIST_MIME_TYPE, TEXT_MIME_TYPE))
                            EventHandlerResult.Stop
                        }
                        KeySym.v -> {
                            window.clipboardPaste(0, listOf(TEXT_MIME_TYPE, URI_LIST_MIME_TYPE))
                            EventHandlerResult.Stop
                        }
                        KeySym.C -> {
                            app.clipboardPut(listOf(URI_LIST_MIME_TYPE, TEXT_MIME_TYPE))
                            currentClipboard = DataTransferContentType.UriList(EXAMPLE_FILES)
                            EventHandlerResult.Stop
                        }
                        KeySym.c -> {
                            getCurrentSelection()?.let { selection ->
                                app.clipboardPut(listOf(TEXT_MIME_TYPE))
                                currentClipboard = DataTransferContentType.Text(selection)
                                EventHandlerResult.Stop
                            } ?: EventHandlerResult.Continue
                        }
                        else -> EventHandlerResult.Continue
                    }
                } else {
                    when (event.key.value) {
                        KeySym.BackSpace -> {
                            if (!deleteSelection() && cursorOffset > 0) {
                                val newCursorOffset = getPreviousGlyphOffset(text.toString(), cursorOffset)
                                text.delete(newCursorOffset, cursorOffset)
                                cursorOffset = newCursorOffset
                            }
                        }

                        KeySym.Up -> {
                            if (modifiers.shift) {
                                if (selectionStartOffset == null) {
                                    selectionStartOffset = cursorOffset
                                }
                                selectionEndOffset = 0
                                cursorOffset = 0
                            } else {
                                cursorOffset = 0
                            }
                        }

                        KeySym.Down -> {
                            if (modifiers.shift) {
                                if (selectionStartOffset == null) {
                                    selectionStartOffset = cursorOffset
                                }
                                val end = text.length
                                selectionEndOffset = end
                                cursorOffset = end
                            } else {
                                cursorOffset = text.length
                            }
                        }

                        KeySym.Left -> {
                            if (modifiers.shift) {
                                if (selectionStartOffset == null) {
                                    selectionStartOffset = cursorOffset
                                }
                                cursorOffset = getPreviousGlyphOffset(text.toString(), cursorOffset)
                                selectionEndOffset = cursorOffset
                            } else {
                                cursorOffset = getPreviousGlyphOffset(text.toString(), cursorOffset)
                            }
                        }

                        KeySym.Right -> {
                            if (modifiers.shift) {
                                if (selectionStartOffset == null) {
                                    selectionStartOffset = cursorOffset
                                }
                                cursorOffset = getNextGlyphOffset(text.toString(), cursorOffset)
                                selectionEndOffset = cursorOffset
                            } else {
                                cursorOffset = getNextGlyphOffset(text.toString(), cursorOffset)
                            }
                        }

                        else -> {
                            event.characters?.let { characters ->
                                deleteSelection()
                                text.insert(cursorOffset, characters)
                                cursorOffset += characters.length
                            }
                        }
                    }

                    if (!modifiers.shift && !modifiers.control && !modifiers.logo && !event.key.isModifierKey()) {
                        selectionStartOffset = null
                        selectionEndOffset = null
                    }
                    if (textInputEnabled) {
                        app.textInputUpdate(createTextInputContext(changeCausedByInputMethod = false))
                    }
                    EventHandlerResult.Stop
                }
            }
            is Event.DataTransfer -> {
                val data = event.data
                if (data.mimeTypes.contains(URI_LIST_MIME_TYPE)) {
                    val files = data.data.decodeToString().trimEnd().split("\n")
                    Logger.info { "Pasted ${files.size} files:" }
                    for (file in files) {
                        Logger.info { file }
                    }
                } else if (data.mimeTypes.contains(TEXT_MIME_TYPE)) {
                    deleteSelection()
                    val pastedText = data.data.decodeToString()
                    text.insert(cursorOffset, pastedText)
                    cursorOffset += pastedText.length
                    if (textInputEnabled) {
                        app.textInputUpdate(createTextInputContext(changeCausedByInputMethod = false))
                    }
                }
                EventHandlerResult.Stop
            }
            is Event.TextInputAvailability -> {
                if (event.available) {
                    app.textInputEnable(createTextInputContext(changeCausedByInputMethod = false))
                    textInputEnabled = true
                } else {
                    app.textInputDisable()
                    textInputEnabled = false
                }
                EventHandlerResult.Stop
            }
            is Event.TextInput -> {
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
                EventHandlerResult.Stop
            }
            else -> EventHandlerResult.Continue
        }
    }
}

private fun LogicalPoint.isInsideCircle(center: LogicalPoint, radius: LogicalPixels): Boolean {
    val xDiff = this.x - center.x
    val yDiff = this.y - center.y
    return xDiff.pow(2) + yDiff.pow(2) <= radius.pow(2)
}

private data class TextLineCreator(private var cachedFontSize: Float, private var cachedText: String) {
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

class CustomTitlebar(
    private var origin: LogicalPoint,
    var size: LogicalSize,
    var buttonLayout: TitlebarLayout,
    val requestClose: () -> Unit,
) {
    private var rectangles = ArrayList<Pair<LogicalRect, WindowButtonType>>()
    private var active: Boolean = false
    private var maximized: Boolean = false
    private var fullscreen: Boolean = false
    private var lastHeaderMouseDownTime: Timestamp? = null
    private var lastMouseLocation: LogicalPoint? = null
    private var leftClickStartLocation: LogicalPoint? = null
    private var isDragging: Boolean = false

    private var titleTextLineCreator = TextLineCreator(cachedFontSize = 0f, cachedText = "")

    companion object {
        const val CUSTOM_TITLEBAR_HEIGHT: LogicalPixels = 55f
        const val BUTTON_LINE_WIDTH: LogicalPixels = 5f
        const val MOVE_RADIUS: LogicalPixels = 3f
        val COLOR_DARK_GRAY = Color.makeRGB(128, 128, 128)
        val COLOR_LIGHT_GRAY = Color.makeRGB(211, 211, 211)
        val BUTTON_SIZE = LogicalSize(CUSTOM_TITLEBAR_HEIGHT, CUSTOM_TITLEBAR_HEIGHT)

        val APP_ICON = Image.makeFromEncoded(Files.readAllBytes(Path.of("resources/jb-logo.png")))
    }

    fun configure(event: Event.WindowConfigure, layout: TitlebarLayout) {
        size = LogicalSize(width = event.size.width, height = CUSTOM_TITLEBAR_HEIGHT)
        active = event.active
        maximized = event.maximized
        fullscreen = event.fullscreen
        setLayout(layout)
    }

    fun toggleMaximize(window: Window) {
        if (maximized) {
            window.unmaximize()
        } else {
            window.maximize()
        }
    }

    fun setLayout(layout: TitlebarLayout) {
        buttonLayout = layout
        rectangles.clear()
        buttonLayout.let {
            val buttonsLeftWidth = origin.x + (it.layoutLeft.size * BUTTON_SIZE.width)
            val buttonsRightWidth = it.layoutRight.size * BUTTON_SIZE.width
            val rect = LogicalRect(
                LogicalPoint(buttonsLeftWidth, origin.y),
                LogicalSize(size.width - buttonsRightWidth - buttonsLeftWidth, CUSTOM_TITLEBAR_HEIGHT),
            )
            rectangles.add(Pair(rect, WindowButtonType.Title))
            for ((i, button) in it.layoutLeft.withIndex()) {
                val rect = LogicalRect(LogicalPoint(i * BUTTON_SIZE.height, origin.y), BUTTON_SIZE)
                rectangles.add(Pair(rect, button))
            }
            for ((i, button) in it.layoutRight.withIndex()) {
                val rect = LogicalRect(
                    LogicalPoint(origin.x + size.width - ((it.layoutRight.size - i) * BUTTON_SIZE.width), origin.y),
                    BUTTON_SIZE,
                )
                rectangles.add(Pair(rect, button))
            }
        }
    }

    private fun executeWindowAction(
        windowButton: WindowButtonType,
        mouseButton: MouseButton,
        locationInWindow: LogicalPoint,
        window: Window,
    ): Boolean {
        return when (windowButton) {
            WindowButtonType.AppMenu, WindowButtonType.Icon -> {
                window.showMenu(locationInWindow)
                true
            }
            WindowButtonType.Spacer,
            WindowButtonType.Title,
            -> if (mouseButton == MouseButton.RIGHT) {
                window.showMenu(locationInWindow)
                true
            } else {
                false
            }
            WindowButtonType.Minimize -> {
                window.minimize()
                true
            }
            WindowButtonType.Maximize -> {
                toggleMaximize(window)
                true
            }
            WindowButtonType.Close -> {
                requestClose()
                true
            }
        }
    }

    private fun handlePotentialDoubleClick(timestamp: Timestamp, doubleClickInterval: Duration): Boolean {
        val prevTime = lastHeaderMouseDownTime
        if (prevTime != null) {
            val timeDiff = (timestamp.toDuration() - prevTime.toDuration())
            Logger.info { "timeDiff: $timeDiff" }
            if (timeDiff <= doubleClickInterval) {
                lastHeaderMouseDownTime = timestamp
                return true
            }
        }
        lastHeaderMouseDownTime = timestamp
        return false
    }

    fun handleEvent(event: Event, xdgDesktopSettings: XdgDesktopSettings, window: Window): EventHandlerResult {
        val headerRect = LogicalRect(origin, size)
        val handled: Boolean = when (event) {
            is Event.MouseDown -> {
                if (headerRect.contains(event.locationInWindow) && event.button == MouseButton.LEFT) {
                    leftClickStartLocation = event.locationInWindow
                    isDragging = false
                    true
                } else {
                    false
                }
            }
            is Event.MouseUp -> {
                if (event.button == MouseButton.LEFT) {
                    leftClickStartLocation = null
                    isDragging = false
                    if (headerRect.contains(event.locationInWindow)) {
                        rectangles.firstOrNull { it.first.contains(event.locationInWindow) }?.second?.let { windowButton ->
                            if ((windowButton == WindowButtonType.Title || windowButton == WindowButtonType.Spacer) &&
                                handlePotentialDoubleClick(event.timestamp, xdgDesktopSettings.doubleClickInterval)
                            ) {
                                toggleMaximize(window)
                                true
                            } else {
                                executeWindowAction(windowButton, event.button, event.locationInWindow, window)
                            }
                        } ?: false
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            is Event.MouseMoved -> {
                lastMouseLocation = event.locationInWindow
                if (headerRect.contains(event.locationInWindow) &&
                    !isDragging &&
                    (leftClickStartLocation?.isInsideCircle(event.locationInWindow, MOVE_RADIUS) == false)
                ) {
                    isDragging = true
                    leftClickStartLocation = null
                    window.startMove()
                    true
                } else {
                    false
                }
            }
            is Event.MouseExited -> {
                leftClickStartLocation = null
                false
            }
            is Event.MouseEntered -> {
                isDragging = false
                false
            }
            else -> false
        }
        return if (handled) EventHandlerResult.Stop else EventHandlerResult.Continue
    }

    private fun drawButton(
        canvas: Canvas,
        button: WindowButtonType,
        rect: LogicalRect,
        highlighted: Boolean,
        hovered: Boolean,
        scale: Float,
        title: String,
    ) {
        val w = rect.size.width * scale
        val h = rect.size.height * scale
        val xOffset = rect.point.x * scale
        val yOffset = rect.point.y * scale

        when (button) {
            WindowButtonType.Minimize, WindowButtonType.Maximize, WindowButtonType.Close -> {
                Paint().use { paint ->
                    paint.color = if (highlighted) COLOR_LIGHT_GRAY else if (hovered) COLOR_DARK_GRAY else Color.BLACK
                    canvas.drawRect(Rect.makeXYWH(xOffset, yOffset, w, h), paint)
                }
            }
            else -> {}
        }

        Paint().use { paint ->
            paint.color = Color.WHITE
            paint.strokeWidth = BUTTON_LINE_WIDTH * scale

            val yTop = yOffset + (paint.strokeWidth / 2)
            val yBottom = (yOffset + h) - (paint.strokeWidth / 2)
            val xLeft = xOffset + (paint.strokeWidth / 2)
            val xRight = (xOffset + w) - (paint.strokeWidth / 2)
            when (button) {
                WindowButtonType.AppMenu -> {
                    canvas.drawLine(xOffset, yOffset, xOffset + w, yOffset, paint)
                    canvas.drawLine(xOffset, yOffset + (h / 2), xOffset + w, yOffset + (h / 2), paint)
                    canvas.drawLine(xOffset, yBottom, xOffset + w, yBottom, paint)
                }
                WindowButtonType.Icon -> {
                    canvas.drawImageRect(
                        APP_ICON,
                        src = APP_ICON.imageInfo.bounds.toRect(),
                        dst = Rect(xLeft, yTop, xRight, yBottom),
                        paint,
                    )
                }
                WindowButtonType.Spacer -> {}
                WindowButtonType.Minimize -> {
                    canvas.drawLine(xOffset, yBottom, xOffset + w, yBottom, paint)
                }
                WindowButtonType.Maximize -> {
                    canvas.drawLine(xLeft, yTop, xLeft, yBottom, paint)
                    canvas.drawLine(xLeft, yTop, xRight, yTop, paint)
                    canvas.drawLine(xRight, yTop, xRight, yBottom, paint)
                    canvas.drawLine(xLeft, yBottom, xRight, yBottom, paint)
                }
                WindowButtonType.Close -> {
                    canvas.drawLine(xOffset, yOffset, xOffset + w, yBottom, paint)
                    canvas.drawLine(xOffset + w, yOffset, xOffset, yBottom, paint)
                }
                WindowButtonType.Title -> {
                    paint.color = if (active) Color.WHITE else COLOR_LIGHT_GRAY
                    canvas.drawTextLine(titleTextLineCreator.makeTextLine(title, CUSTOM_TITLEBAR_HEIGHT * scale), xOffset, yBottom, paint)
                }
            }
        }
    }

    fun draw(canvas: Canvas, scale: Float, xdgDesktopSettings: XdgDesktopSettings, title: String) {
        val physicalOrigin = origin.toPhysical(scale)
        val physicalSize = size.toPhysical(scale)
        val l = physicalOrigin.x.toFloat()
        val t = physicalOrigin.y.toFloat()
        val w = physicalSize.width.toFloat()
        val h = physicalSize.height.toFloat()
        Paint().use { paint ->
            paint.color = xdgDesktopSettings.accentColor
            canvas.drawRect(Rect.makeXYWH(l, t, w, h), paint)
        }
        for ((rect, button) in rectangles) {
            val hovered = !isDragging && (lastMouseLocation?.let { rect.contains(it) } == true)
            val highlighted = hovered && (leftClickStartLocation?.let { rect.contains(it) } == true)
            drawButton(canvas, button, rect, highlighted = highlighted, hovered = hovered, scale, title)
        }
    }
}

class ContentArea(
    var origin: LogicalPoint,
    var size: LogicalSize,
) {
    private var markerPosition: LogicalPoint? = null
    var currentDragContent: DataTransferContentType? = null

    fun handleEvent(event: Event, window: Window, editorState: EditorState): EventHandlerResult {
        return when (event) {
            is Event.MouseMoved -> {
                markerPosition = LogicalPoint(
                    event.locationInWindow.x - origin.x,
                    event.locationInWindow.y - origin.y,
                )
                EventHandlerResult.Continue
            }
            is Event.MouseDown -> {
                if (editorState.modifiers.shift) {
                    window.startDrag(listOf(URI_LIST_MIME_TYPE, TEXT_MIME_TYPE), DragAction.Move)
                    currentDragContent = DataTransferContentType.UriList(EXAMPLE_FILES)
                } else if (editorState.modifiers.control) {
                    window.startDrag(listOf(URI_LIST_MIME_TYPE, TEXT_MIME_TYPE), DragAction.Copy)
                    currentDragContent = DataTransferContentType.UriList(EXAMPLE_FILES)
                } else {
                    currentDragContent = editorState.getCurrentSelection()?.let {
                        window.startDrag(listOf(TEXT_MIME_TYPE), DragAction.Copy)
                        DataTransferContentType.Text(it)
                    }
                }
                EventHandlerResult.Stop
            }
            is Event.MouseUp -> {
                currentDragContent = null
                Logger.info { "$window: currentDragContent = null" }
                EventHandlerResult.Stop
            }
            is Event.TextInput -> {
                EventHandlerResult.Stop
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

class CustomBorders {
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

    fun handleEvent(event: Event, window: Window): EventHandlerResult {
        when (event) {
            is Event.MouseDown -> {
                val edge = toEdge(event.locationInWindow)
                if (edge != null) {
                    window.startResize(edge)
                    return EventHandlerResult.Stop
                }
            }
            is Event.MouseMoved -> {
            }
            else -> {}
        }
        return EventHandlerResult.Continue
    }
}

class WindowContainer(
    var customTitlebar: CustomTitlebar?,
    var customBorders: CustomBorders?,
    val contentArea: ContentArea,
    private var xdgDesktopSettings: XdgDesktopSettings,
    val requestClose: () -> Unit,
) {
    private var capabilities: WindowCapabilities? = null

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

    fun settingsChanged(xdgDesktopSettings: XdgDesktopSettings) {
        this.xdgDesktopSettings = xdgDesktopSettings
        capabilities?.let { capabilities ->
            customTitlebar?.setLayout(
                TitlebarLayout(
                    layoutLeft = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutLeft, capabilities),
                    layoutRight = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutRight, capabilities),
                ),
            )
        }
    }

    fun configure(event: Event.WindowConfigure) {
        capabilities = event.capabilities
        if (event.clientSideDecorations) {
            val titlebarLayout = TitlebarLayout(
                layoutLeft = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutLeft, event.capabilities),
                layoutRight = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutRight, event.capabilities),
            )
            val titlebarSize = LogicalSize(width = event.size.width, height = CustomTitlebar.CUSTOM_TITLEBAR_HEIGHT)
            val titlebar = customTitlebar ?: CustomTitlebar(
                origin = LogicalPoint.Zero, size = titlebarSize, titlebarLayout, requestClose,
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
            contentArea.size = event.size
        }
    }

    fun handleEvent(event: Event, window: Window, editorState: EditorState): EventHandlerResult {
        return when {
            customBorders?.handleEvent(event, window) == EventHandlerResult.Stop -> EventHandlerResult.Stop
            customTitlebar?.handleEvent(event, xdgDesktopSettings, window) == EventHandlerResult.Stop -> EventHandlerResult.Stop
            contentArea.handleEvent(event, window, editorState) == EventHandlerResult.Stop -> EventHandlerResult.Stop
            else -> EventHandlerResult.Continue
        }
    }

    fun draw(canvas: Canvas, time: Long, scale: Float, title: String, editorState: EditorState) {
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
        customTitlebar?.draw(canvas, scale, xdgDesktopSettings, title)
        contentArea.draw(canvas, time, scale, editorState)
    }
}

class RotatingBallWindow(
    val windowContainer: WindowContainer,
    app: Application,
    val editorState: EditorState,
    windowParams: WindowParams,
) : SkikoWindowLinux(app, windowParams) {
    private var title: String = windowParams.title

    companion object {
        fun createWindow(
            app: Application,
            editorState: EditorState,
            windowParams: WindowParams,
            xdgDesktopSettings: XdgDesktopSettings,
            requestClose: () -> Unit,
        ): RotatingBallWindow {
            val windowSize = LogicalSize(640f, 480f)
            val windowContentSize = windowSize // todo it's incorrect
            val container = WindowContainer.create(windowContentSize, xdgDesktopSettings, requestClose)

            return RotatingBallWindow(container, app, editorState, windowParams)
        }
    }

    fun settingsChanged(xdgDesktopSettings: XdgDesktopSettings) {
        windowContainer.settingsChanged(xdgDesktopSettings)
    }

    fun getDragAndDropSupportedMimeTypes(point: LogicalPoint): List<String> {
        return if (point.x < windowContainer.contentArea.size.width / 2) {
            listOf(URI_LIST_MIME_TYPE, TEXT_MIME_TYPE)
        } else {
            listOf(TEXT_MIME_TYPE, URI_LIST_MIME_TYPE)
        }
    }

    override fun handleEvent(event: Event): EventHandlerResult {
        return if (super.handleEvent(event) == EventHandlerResult.Continue) {
            when (event) {
                is Event.WindowConfigure -> {
                    windowContainer.configure(event)
                    // performDrawing(syncWithCA = true)
                    EventHandlerResult.Stop
                }
                is Event.MouseMoved -> {
                    val borderEdge = windowContainer.customBorders?.toEdge(event.locationInWindow)
                    if (borderEdge != null) {
                        window.setPointerShape(CustomBorders.edgeToPointerShape(borderEdge))
                        EventHandlerResult.Stop
                    } else {
                        window.setPointerShape(PointerShape.Default)
                        windowContainer.handleEvent(event, window, editorState)
                    }
                }
                else -> {
                    windowContainer.handleEvent(event, window, editorState)
                }
            }
        } else {
            EventHandlerResult.Stop
        }
    }

    override fun Canvas.draw(size: PhysicalSize, scale: Double, time: Long) {
        val canvas = this
        windowContainer.draw(canvas, time, scale.toFloat(), title, editorState)
    }
}

class ApplicationState(private val app: Application) : AutoCloseable {
    val windows = mutableMapOf<WindowId, RotatingBallWindow>()
    private val xdgDesktopSettings = XdgDesktopSettings()
    private val editorState = EditorState()

    fun createWindow(useCustomTitlebar: Boolean, forceSoftwareRendering: Boolean = false) {
        val windowId = windows.count().toLong()
        val windowParams = WindowParams(
            windowId = windowId,
            size = LogicalSize(width = 640f, height = 480f),
            title = "Window ${windows.count()}",
            appId = "org.jetbrains.desktop.linux.skikoSample1",
            forceClientSideDecoration = useCustomTitlebar,
            forceSoftwareRendering = forceSoftwareRendering,
        )

        windows[windowId] = RotatingBallWindow.createWindow(
            app,
            editorState,
            windowParams,
            xdgDesktopSettings,
        ) {
            handleEvent(Event.WindowCloseRequest, windowId)
        }
    }

    private fun logEvents(event: Event) {
        when (event) {
            is Event.MouseMoved, is Event.WindowDraw -> {}
            else -> {
                Logger.info { "$event" }
            }
        }
    }

    fun handleEvent(event: Event, windowId: WindowId): EventHandlerResult {
        logEvents(event)
        val window = windows[windowId] ?: return EventHandlerResult.Continue
        if (window.editorState.handleEvent(event, app, window.window) == EventHandlerResult.Stop) {
            return EventHandlerResult.Stop
        }
        return when (event) {
            is Event.WindowCloseRequest -> {
                window.close()
                windows.remove(windowId)
                if (windows.isEmpty()) {
                    app.stopEventLoop()
                }
                EventHandlerResult.Stop
            }
            else -> window.handleEvent(event)
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

    fun getDataTransferData(dataSource: DataSource, mimeType: String): ByteArray {
        val content = when (dataSource) {
            DataSource.Clipboard -> editorState.currentClipboard
            DataSource.DragAndDrop -> {
                windows.values.firstNotNullOf { it.windowContainer.contentArea.currentDragContent }
            }
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
                        content.files.joinToString("\n") { "file://$it" }.encodeToByteArray()
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

    override fun close() {
        windows.values.forEach(AutoCloseable::close)
        windows.clear()
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
                onApplicationStarted = {
                    state.createWindow(useCustomTitlebar = true, forceSoftwareRendering = false)
                },
                onXdgDesktopSettingsChange = {
                    state.settingChanged(it)
                },
                eventHandler = { event, windowId -> state.handleEvent(event, windowId) },
                getDragAndDropSupportedMimeTypes = { queryData ->
                    state.windows[queryData.windowId]!!.getDragAndDropSupportedMimeTypes(queryData.point)
                },
                getDataTransferData = { dataSource, mimeType ->
                    state.getDataTransferData(dataSource, mimeType)
                },
            ),
        )
    }
}
