package org.jetbrains.desktop.sample

import org.jetbrains.desktop.macos.AppMenuItem
import org.jetbrains.desktop.macos.AppMenuManager
import org.jetbrains.desktop.macos.AppMenuStructure
import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Cursor
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.FileDialog
import org.jetbrains.desktop.macos.KeyModifiersSet
import org.jetbrains.desktop.macos.Keystroke
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.LogLevel
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.LogicalPixels
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.MetalCommandQueue
import org.jetbrains.desktop.macos.MetalDevice
import org.jetbrains.desktop.macos.PhysicalPoint
import org.jetbrains.desktop.macos.PhysicalSize
import org.jetbrains.desktop.macos.Screen
import org.jetbrains.desktop.macos.Window
import org.jetbrains.desktop.macos.WindowBackground
import org.jetbrains.desktop.macos.WindowVisualEffect
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import java.lang.AutoCloseable
import java.nio.file.Files
import java.nio.file.Path
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

class CustomTitlebar(
    private var origin: LogicalPoint,
    var size: LogicalSize,
    var startWindowDrag: (() -> Unit)? = null,
) {
    companion object {
        const val CUSTOM_TITLEBAR_HEIGHT: LogicalPixels = 55.0
    }

    fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.MouseDown -> {
                if (event.locationInWindow.x > origin.x &&
                    event.locationInWindow.x < origin.x + size.width * 0.75 &&
                    event.locationInWindow.y > origin.y &&
                    event.locationInWindow.y < origin.y + size.height
                ) {
                    startWindowDrag?.invoke()
                    EventHandlerResult.Stop
                } else {
                    EventHandlerResult.Continue
                }
            }
            else -> EventHandlerResult.Continue
        }
    }

    fun draw(canvas: Canvas, time: Long, scale: Double) {
        val physicalOrigin = origin.toPhysical(scale)
        val physicalSize = size.toPhysical(scale)
        val x = physicalOrigin.x.toFloat()
        val y = physicalOrigin.y.toFloat()
        val width = physicalSize.width.toFloat()
        val height = physicalSize.height.toFloat()
        Paint().use { paint ->
            paint.color = 0xFF404040.toInt()
            canvas.drawRect(Rect.makeXYWH(x, y, width, height), paint)
        }
        Paint().use { paint ->
            paint.color = 0xFFAAAAAA.toInt()
            canvas.drawRect(Rect.makeXYWH(width * 0.75f, y, width * 0.25f, height), paint)
        }
    }
}

class ContentArea(
    var origin: LogicalPoint,
    var size: LogicalSize,
) {

    private var markerPosition: LogicalPoint? = null

    fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.MouseMoved -> {
                markerPosition = LogicalPoint(
                    event.locationInWindow.x - origin.x,
                    event.locationInWindow.y - origin.y,
                )
                EventHandlerResult.Continue
            }

            else -> EventHandlerResult.Continue
        }
    }

    fun draw(canvas: Canvas, time: Long, scale: Double) {
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
        canvas.drawSpiningCircle(contentOrigin, contentSize, time, scale.toFloat())
        canvas.drawWindowBorders(contentOrigin, contentSize, time, scale.toFloat())
        canvas.drawCursor(contentOrigin, contentSize, time, scale.toFloat())
    }

    private fun Canvas.drawSpiningCircle(origin: PhysicalPoint, size: PhysicalSize, t: Long, scale: Float) = withTranslated(origin) {
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

    private fun Canvas.drawWindowBorders(origin: PhysicalPoint, size: PhysicalSize, t: Long, scale: Float) = withTranslated(origin) {
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

    private fun Canvas.drawCursor(origin: PhysicalPoint, size: PhysicalSize, t: Long, scale: Float) = withTranslated(origin) {
        val canvas = this
        markerPosition?.let { curs ->
            val positive = curs.x > 0 && curs.y > 0
            val inBox = curs.x < size.width && curs.y < size.height
            if (positive && inBox) {
                val x = curs.x.toFloat()
                val y = curs.y.toFloat()
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

class WindowContainer(
    val customTitlebar: CustomTitlebar?,
    private val contentArea: ContentArea,
) {
    companion object {
        fun create(windowContentSize: LogicalSize, useCustomTitlebar: Boolean): WindowContainer {
            return if (useCustomTitlebar) {
                val titlebar = CustomTitlebar(
                    LogicalPoint.Zero,
                    LogicalSize(width = windowContentSize.width, height = CustomTitlebar.CUSTOM_TITLEBAR_HEIGHT),
                )
                val contentArea = ContentArea(
                    LogicalPoint(x = 0.0, y = CustomTitlebar.CUSTOM_TITLEBAR_HEIGHT),
                    LogicalSize(
                        width = windowContentSize.width,
                        height = windowContentSize.height - titlebar.size.height,
                    ),
                )
                WindowContainer(titlebar, contentArea)
            } else {
                val contentArea = ContentArea(LogicalPoint.Zero, windowContentSize)
                WindowContainer(null, contentArea)
            }
        }
    }

    fun resize(windowSize: LogicalSize) {
        if (customTitlebar != null) {
            customTitlebar.size = LogicalSize(width = windowSize.width, height = CustomTitlebar.CUSTOM_TITLEBAR_HEIGHT)
            contentArea.origin = LogicalPoint(x = 0.0, y = customTitlebar.size.height)
            contentArea.size =
                LogicalSize(width = windowSize.width, height = windowSize.height - customTitlebar.size.height)
        } else {
            contentArea.size = windowSize
        }
    }

    fun handleEvent(event: Event): EventHandlerResult {
        return when {
            customTitlebar?.handleEvent(event) == EventHandlerResult.Stop -> EventHandlerResult.Stop
            contentArea.handleEvent(event) == EventHandlerResult.Stop -> EventHandlerResult.Stop
            else -> EventHandlerResult.Continue
        }
    }

    fun draw(canvas: Canvas, time: Long, scale: Double) {
        customTitlebar?.draw(canvas, time, scale)
        contentArea.draw(canvas, time, scale)
    }
}

class RotatingBallWindow(
    device: MetalDevice,
    queue: MetalCommandQueue,
    private val windowContainer: WindowContainer,
    windowParams: Window.WindowParams,
) : SkikoWindow(
    device,
    queue,
    windowParams,
) {

    companion object {
        fun createWindow(
            device: MetalDevice,
            queue: MetalCommandQueue,
            title: String,
            origin: LogicalPoint,
            useCustomTitlebar: Boolean,
        ): RotatingBallWindow {
            val windowSize = LogicalSize(640.0, 480.0)
            val windowContentSize = windowSize // todo it's incorrect
            val container = WindowContainer.create(windowContentSize, useCustomTitlebar)

            val windowParams = Window.WindowParams(
                size = windowSize,
                title = title,
                origin = origin,
                useCustomTitlebar = useCustomTitlebar,
                titlebarHeight = container.customTitlebar?.size?.height ?: 0.0,
            )

            return RotatingBallWindow(device, queue, container, windowParams)
        }
    }

    init {
        windowContainer.resize(view.size().toLogical(window.scaleFactor()))
        performDrawing(syncWithCA = true)
    }

    override fun handleEvent(event: Event): EventHandlerResult {
        return if (super.handleEvent(event) == EventHandlerResult.Continue) {
            when {
                event is Event.WindowResize -> {
                    val isRunning = displayLink.isRunning()
                    displayLink.setRunning(false)
                    val viewSize = view.size().toLogical(window.scaleFactor())
                    assert(event.size == viewSize)
                    windowContainer.resize(event.size)
                    performDrawing(syncWithCA = true)
                    displayLink.setRunning(isRunning)
                    EventHandlerResult.Stop
                }
            }
            windowContainer.customTitlebar?.startWindowDrag = {
                window.startDrag()
            }
            windowContainer.handleEvent(event)
        } else {
            EventHandlerResult.Stop
        }
    }

    override fun Canvas.draw(size: PhysicalSize, time: Long) {
        val canvas = this
//        canvas.clear(Color.TRANSPARENT) // use RED to debug
        windowContainer.draw(canvas, time, window.scaleFactor())
    }
}

class ApplicationState : AutoCloseable {
    private val windows = mutableListOf<RotatingBallWindow>()

    private val device: MetalDevice by lazy {
        MetalDevice.create()
    }

    private val queue by lazy {
        MetalCommandQueue.create(device)
    }

    fun createWindow(useCustomTitlebar: Boolean) {
        windows.add(
            RotatingBallWindow.createWindow(
                device,
                queue,
                "Window ${windows.count()}",
                LogicalPoint(0.0, 0.0),
                useCustomTitlebar,
            ),
        )
    }

    private fun setPaused(value: Boolean) {
        mainWindow()?.displayLink?.setRunning(!value)
    }

    private fun mainWindow(): RotatingBallWindow? {
        return windows.find { window ->
            window.window.isMain
        }
    }

    private fun changeCurrentWindowSize(delta: LogicalPixels) {
        mainWindow()?.let { window ->
            val currentOrigin = window.window.origin
            val currentSize = window.window.size
            // todo check display bounds
            // todo check min and max size
            window.window.setRect(
                origin = LogicalPoint(
                    currentOrigin.x - delta / 2.0,
                    currentOrigin.y - delta / 2.0,
                ),
                size = LogicalSize(
                    currentSize.width + delta,
                    currentSize.height + delta,
                ),
                animateTransition = true,
            )
        }
    }

    private fun makeWindowTransparent() {
        mainWindow()?.let { window ->
            window.window.setBackground(WindowBackground.Transparent)
        }
    }

    private fun makeWindowOpaque() {
        mainWindow()?.let { window ->
            window.window.setBackground(WindowBackground.SolidColor(org.jetbrains.desktop.macos.Color(1.0, 1.0, 1.0, 1.0)))
        }
    }

    private var cursors = generateSequence { Cursor.Icon.entries.asSequence() }.flatten().iterator()

    private fun cycleCursor() {
        Cursor.icon = cursors.next()
    }

    private var effect = generateSequence { WindowVisualEffect.entries.asSequence() }.flatten().iterator()

    private fun cycleWindowEffects() {
        mainWindow()?.let { window ->
            window.window.setBackground(WindowBackground.VisualEffect(effect.next()))
        }
    }

    private fun killWindow(window: RotatingBallWindow) {
        windows.remove(window)
        window.close()
    }

    private fun logEvents(event: Event) {
        when (event) {
            is Event.KeyDown -> {
                Logger.info { "$event" }
            }
            is Event.KeyUp -> {
                Logger.info { "$event" }
            }
            is Event.ModifiersChanged -> {
                Logger.info { "$event" }
            }
            is Event.MouseEntered -> {
                Logger.info { "$event" }
            }
            is Event.MouseExited -> {
                Logger.info { "$event" }
            }
            is Event.MouseMoved -> {
                Logger.info { "$event" }
            }
            is Event.MouseDragged -> {
                Logger.info { "$event" }
            }
            is Event.MouseDown -> {
                Logger.info { "$event" }
            }
            is Event.MouseUp -> {
                Logger.info { "$event" }
            }
            else -> {}
        }
    }

    fun handleEvent(event: Event): EventHandlerResult {
//        logEvents(event)
        val eventWindowId = event.windowId()

        return when (event) {
            is Event.WindowCloseRequest -> {
                windows.find {
                    it.window.windowId() == eventWindowId
                }?.let { window ->
                    killWindow(window)
                } ?: run {
                    Logger.warn { "Can't find window for $event" }
                }
                EventHandlerResult.Stop
            }
            is Event.KeyDown -> {
                EventHandlerResult.Continue
            }
            is Event.WindowFullScreenToggle -> {
                AppMenuManager.setMainMenu(buildMenu())
                EventHandlerResult.Continue
            }
            else -> {
                val window = windows.find {
                    it.window.windowId() == eventWindowId
                }
                window?.let {
                    window.handleEvent(event)
                } ?: EventHandlerResult.Continue
            }
        }
    }

    fun buildMenu(): AppMenuStructure {
        return AppMenuStructure(
            AppMenuItem.SubMenu(
                // Ignored
                title = "App",
                AppMenuItem.Action(
                    "New Window",
                    keystroke = Keystroke(key = "n", modifiers = KeyModifiersSet.create(command = true)),
                    perform = { createWindow(useCustomTitlebar = true) },
                ),
                AppMenuItem.Action(
                    "Open...",
                    keystroke = Keystroke(key = "o", modifiers = KeyModifiersSet.create(command = true, shift = true)),
                    perform = {
                        val path = FileDialog.showModal(FileDialog.DialogParams())
                        println("openFileDialog callback received $path")
                    },
                ),
                AppMenuItem.Action(
                    "New Titled Window",
                    keystroke = Keystroke(key = "n", modifiers = KeyModifiersSet.create(command = true, shift = true)),
                    perform = { createWindow(useCustomTitlebar = false) },
                ),
                AppMenuItem.Separator,
                AppMenuItem.Action(
                    "Hide ${Application.name}",
                    keystroke = Keystroke(key = "h", modifiers = KeyModifiersSet.create(command = true)),
                    perform = {
                        Application.hide()
                    },
                ),
                AppMenuItem.Action(
                    "Hide Others",
                    keystroke = Keystroke(key = "h", modifiers = KeyModifiersSet.create(command = true, option = true)),
                    perform = {
                        Application.hideOtherApplications()
                    },
                ),
                AppMenuItem.Action(
                    "Show All",
                    perform = {
                        Application.unhideAllApplications()
                    },
                ),
                AppMenuItem.Action(
                    "Quit",
                    keystroke = Keystroke(key = "q", modifiers = KeyModifiersSet.create(command = true)),
                    perform = {
                        Application.stopEventLoop()
                    },
                ),
                specialTag = AppMenuItem.SubMenu.SpecialTag.AppNameMenu,
            ),
            AppMenuItem.SubMenu(
                title = "View",
                AppMenuItem.Action(
                    title = "Set Title",
                    keystroke = Keystroke(key = "s", modifiers = KeyModifiersSet.create(command = true)),
                    perform = {
                        mainWindow()?.window?.let { window ->
                            val previousTitle = window.title
                            Logger.info { "Title was: $previousTitle" }
                            window.title = "$previousTitle[x]"
                        }
                    },
                ),
                AppMenuItem.Action(
                    title = "Toggle Full Screen",
                    state = when (mainWindow()?.window?.isFullScreen) {
                        true -> AppMenuItem.ActionItemState.On
                        else -> AppMenuItem.ActionItemState.Off
                    },
                    keystroke = Keystroke(key = "f", modifiers = KeyModifiersSet.create(command = true, control = true)),
                    perform = { mainWindow()?.window?.toggleFullScreen() },
                ),
//                specialTag = "View",
            ),
            AppMenuItem.SubMenu(
                title = "Animation",
                AppMenuItem.Action(
                    title = "Pause",
                    keystroke = Keystroke(key = "p", modifiers = KeyModifiersSet.create(command = true)),
                    perform = { setPaused(true) },
                ),
                AppMenuItem.Action(
                    title = "Run",
                    keystroke = Keystroke(key = "r", modifiers = KeyModifiersSet.create(command = true)),
                    perform = { setPaused(false) },
                ),
            ),
            AppMenuItem.SubMenu(
                title = "Displays",
                AppMenuItem.Action(
                    title = "List Displays",
                    keystroke = Keystroke(key = "d", modifiers = KeyModifiersSet.create(command = true)),
                    perform = { Logger.info { Screen.allScreens().toString() } },
                ),
            ),
            AppMenuItem.SubMenu(
                title = "Window",
                AppMenuItem.Action(
                    title = "Increase Size",
                    keystroke = Keystroke(key = "+", modifiers = KeyModifiersSet.create(command = true)),
                    perform = { changeCurrentWindowSize(50.0) },
                ),
                AppMenuItem.Action(
                    title = "Drecrease Size",
                    keystroke = Keystroke(key = "-", modifiers = KeyModifiersSet.create(command = true)),
                    perform = { changeCurrentWindowSize(-50.0) },
                ),
                AppMenuItem.Action(
                    title = "Make Window Transparent",
                    keystroke = Keystroke(key = "t", modifiers = KeyModifiersSet.create(command = true)),
                    perform = { makeWindowTransparent() },
                ),
                AppMenuItem.Action(
                    title = "Make Window Opaque",
                    keystroke = Keystroke(key = "o", modifiers = KeyModifiersSet.create(command = true)),
                    perform = { makeWindowOpaque() },
                ),
                AppMenuItem.Action(
                    title = "Cycle Window Effects",
                    keystroke = Keystroke(key = "e", modifiers = KeyModifiersSet.create(command = true)),
                    perform = { cycleWindowEffects() },
                ),
                AppMenuItem.Action(
                    title = "Log Window Position",
                    keystroke = Keystroke(key = "l", modifiers = KeyModifiersSet.create(command = true)),
                    perform = {
                        mainWindow()?.window?.let { window ->
                            Logger.info {
                                """
                                    WindowId: ${window.windowId()}
                                    origin: ${window.origin}
                                    size: ${window.size}
                                    contentOrigin: ${window.contentOrigin}
                                    contentSize: ${window.contentSize}
                                """.trimIndent()
                            }
                        }
                    },
                ),
                AppMenuItem.Action(
                    title = "Close Window",
                    keystroke = Keystroke(key = "w", modifiers = KeyModifiersSet.create(command = true)),
                    perform = {
                        mainWindow()?.let {
                            killWindow(it)
                        }
                    },
                ),
                specialTag = AppMenuItem.SubMenu.SpecialTag.Window,
            ),
            AppMenuItem.SubMenu(
                title = "Mouse",
                AppMenuItem.Action(
                    title = "Cycle Mouse Cursor",
                    keystroke = Keystroke(key = "m", modifiers = KeyModifiersSet.create(command = true)),
                    perform = { cycleCursor() },
                ),
                AppMenuItem.Action(
                    title = "Mouse Toggle Cursor",
                    keystroke = Keystroke(key = "h", modifiers = KeyModifiersSet.create(command = true)),
                    perform = {
                        Cursor.hidden = !Cursor.hidden
                    },
                ),
            ),
            AppMenuItem.SubMenu(title = "Help"),
        )
    }

    override fun close() {
        windows.forEach {
            it.close()
        }
        queue.close()
        device.close()
    }
}

fun main() {
    Logger.info { runtimeInfo() }
    KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
    Application.init(Application.ApplicationConfig())
    ApplicationState().use { state ->
        state.createWindow(useCustomTitlebar = true)
        Application.runEventLoop { event ->
            if (event is Event.ApplicationDidFinishLaunching) {
                Files.readAllBytes(Path.of("resources/jb-logo.png")).let { iconBytes ->
                    Application.setDockIcon(iconBytes)
                }
                AppMenuManager.setMainMenu(state.buildMenu())
            }
            state.handleEvent(event)
        }
    }
}
