package org.jetbrains.desktop.sample.win32

import org.jetbrains.desktop.win32.AngleRenderer
import org.jetbrains.desktop.win32.Appearance
import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.Clipboard
import org.jetbrains.desktop.win32.ClipboardResult
import org.jetbrains.desktop.win32.Cursor
import org.jetbrains.desktop.win32.CursorIcon
import org.jetbrains.desktop.win32.DataFormat
import org.jetbrains.desktop.win32.DataObject
import org.jetbrains.desktop.win32.DragDropContinueResult
import org.jetbrains.desktop.win32.DragDropEffect
import org.jetbrains.desktop.win32.DragDropManager
import org.jetbrains.desktop.win32.DragDropModifier
import org.jetbrains.desktop.win32.DragDropModifiers
import org.jetbrains.desktop.win32.DragSource
import org.jetbrains.desktop.win32.DropTarget
import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.FileDialog
import org.jetbrains.desktop.win32.Keyboard
import org.jetbrains.desktop.win32.Logger
import org.jetbrains.desktop.win32.NCHitTestResult
import org.jetbrains.desktop.win32.PhysicalPoint
import org.jetbrains.desktop.win32.PhysicalSize
import org.jetbrains.desktop.win32.PointerButton
import org.jetbrains.desktop.win32.Screen
import org.jetbrains.desktop.win32.SurfaceParams
import org.jetbrains.desktop.win32.VirtualKey
import org.jetbrains.desktop.win32.isBusy
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.ColorSpace
import org.jetbrains.skia.DirectContext
import org.jetbrains.skia.FramebufferFormat
import org.jetbrains.skia.GLAssembledInterface
import org.jetbrains.skia.Surface
import org.jetbrains.skia.SurfaceColorFormat
import org.jetbrains.skia.SurfaceOrigin
import org.jetbrains.skia.makeGLWithInterface
import kotlin.time.TimeSource

abstract class SkikoWindowWin32(private val app: Application) : AutoCloseable {
    private val angleRenderer: AngleRenderer by lazy {
        app.createAngleRenderer(window)
    }

    private val directContext: DirectContext by lazy {
        val eglFunc = angleRenderer.getEglGetProcFunc()
        val glInterface = GLAssembledInterface.createFromNativePointers(ctxPtr = eglFunc.ctxPtr, fPtr = eglFunc.fPtr)
        DirectContext.makeGLWithInterface(glInterface)
    }

    val window = app.newWindow()
    private val creationTime = TimeSource.Monotonic.markNow()

    private var currentSize = PhysicalSize(0, 0)
    private var surfaceParams: SurfaceParams? = null

    private var dragDropManager: DragDropManager? = null

    private val captionButtons = CaptionButtonsBar()
    private val counterButton = CounterButton()
    private var immersiveDark = Appearance.getCurrent() == Appearance.Dark

    fun setImmersiveDarkMode(enabled: Boolean) {
        immersiveDark = enabled
        window.setImmersiveDarkMode(enabled)
    }

    private fun isSizeChanged(size: PhysicalSize): Boolean {
        return (size.width != currentSize.width || size.height != currentSize.height)
    }

    /** Snapshot of which caption buttons to show / enable, from the window's current capabilities. */
    private fun captionButtonModel(): CaptionButtonModel = CaptionButtonModel(window.isMinimizable(), window.isMaximizable())

    /** Logical-pixel height of the custom title-bar band; window content should start below it. */
    protected val titleBarHeight: Float get() = captionButtons.titleBarHeight

    fun initializeDropManager() {
        dragDropManager = DragDropManager(window).apply {
            registerDropTarget(
                object : DropTarget {
                    override fun onDragEnter(
                        dataObject: DataObject,
                        modifiers: DragDropModifiers,
                        point: PhysicalPoint,
                        effect: DragDropEffect,
                    ): DragDropEffect {
                        Logger.debug { "Drag enter" }
                        return effect
                    }

                    override fun onDragOver(modifiers: DragDropModifiers, point: PhysicalPoint, effect: DragDropEffect): DragDropEffect {
                        return effect
                    }

                    override fun onDragLeave() {
                        Logger.debug { "Drag leave" }
                    }

                    override fun onDrop(
                        dataObject: DataObject,
                        modifiers: DragDropModifiers,
                        point: PhysicalPoint,
                        effect: DragDropEffect,
                    ): DragDropEffect {
                        Logger.debug { "Drop" }
                        if (dataObject.isFormatAvailable(DataFormat.Html)) {
                            val html = dataObject.readHtmlFragment()
                            Logger.debug { "html: $html" }
                        }
                        if (dataObject.isFormatAvailable(DataFormat.Text)) {
                            val text = dataObject.readTextItem()
                            Logger.debug { "text: $text" }
                        }
                        return effect
                    }
                },
            )
        }
    }

    open fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.WindowDraw -> with(event) {
                performDrawing(size, scale)
                EventHandlerResult.Stop
            }

            is Event.NCCalcSize -> with(event) {
                performDrawing(size, scale)
                EventHandlerResult.Stop
            }

            is Event.KeyDown -> {
                when (event.virtualKey) {
                    VirtualKey.S -> {
                        if (Keyboard.getKeyState(VirtualKey.Control).isDown) {
                            val result = FileDialog.showSaveFileDialog(window)
                            Logger.debug { "Save file dialog result: $result" }
                        } else {
                            val screens = Screen.allScreens()
                            for (screen in screens) {
                                Logger.debug { "Screen: $screen" }
                            }
                            val currentScreen = window.getScreen()
                            Logger.debug { "Current screen: $currentScreen" }
                        }
                    }

                    VirtualKey.T -> {
                        window.setTitle("New Title")
                        window.setBackdropTint(0x5FFF7F00, 1.0f)
                    }

                    VirtualKey.H -> {
                        window.removeBackdropTint()
                    }

                    VirtualKey.D -> {
                        setImmersiveDarkMode(!immersiveDark)
                        Logger.debug { "Immersive dark mode: $immersiveDark" }
                    }

                    VirtualKey.C -> {
                        if (Keyboard.getKeyState(VirtualKey.Control).isDown) {
                            copyToClipboard()
                        } else {
                            window.setCursor(CursorIcon.Hand)
                        }
                    }

                    VirtualKey.O -> {
                        if (Keyboard.getKeyState(VirtualKey.Control).isDown) {
                            val results = FileDialog.showOpenFileDialog(window)
                            Logger.debug { "Open file dialog results: $results" }
                        }
                    }

                    VirtualKey.R -> {
                        window.setResizable(!window.isResizable())
                        Logger.debug { "Resizable: ${window.isResizable()}" }
                    }

                    VirtualKey.N -> {
                        window.setMinimizable(!window.isMinimizable())
                        Logger.debug { "Minimizable: ${window.isMinimizable()}" }
                        window.requestRedraw() // reflect the minimize button's new enabled/visible state
                    }

                    VirtualKey.M -> {
                        window.setMaximizable(!window.isMaximizable())
                        Logger.debug { "Maximizable: ${window.isMaximizable()}" }
                        window.requestRedraw() // reflect the maximize button's new enabled/visible state
                    }

                    VirtualKey.V -> {
                        if (Keyboard.getKeyState(VirtualKey.Control).isDown) {
                            pasteFromClipboard()
                        }
                    }

                    VirtualKey.U -> {
                        // Shift+U hides the cursor, U shows it. ShowCursor maintains an internal display
                        // counter: hide() decrements it, show() increments it, and the cursor is visible
                        // only while the counter is >= 0. Log the returned counter to observe this.
                        val hide = Keyboard.getKeyState(VirtualKey.Shift).isDown
                        val displayCounter = if (hide) Cursor.hide() else Cursor.show()
                        Logger.debug { "Cursor ${if (hide) "hidden" else "shown"}, display counter: $displayCounter" }
                    }

                    else -> {
                        val unicode = event.toUnicode()
                        val translated = event.translate()
                        Logger.debug { "WM_KEYDOWN translated: $translated, ToUnicode: $unicode" }
                    }
                }
                EventHandlerResult.Continue
            }

            is Event.CharacterReceived -> {
                Logger.debug { "CharacterReceived event, character: ${event.character}" }
                EventHandlerResult.Continue
            }

            is Event.SystemAppearanceChange -> with(event) {
                Logger.debug { "Setting change: new appearance: $newAppearance" }
                setImmersiveDarkMode(newAppearance == Appearance.Dark)
                window.requestRedraw()
                EventHandlerResult.Stop
            }

            is Event.SystemHighContrastChange -> {
                Logger.debug { "Setting change: new high contrast: ${event.newHighContrast}" }
                EventHandlerResult.Stop
            }

            is Event.WindowTitleChanged -> with(event) {
                Logger.debug { "New window title: $title" }
                EventHandlerResult.Continue
            }

            is Event.PointerUpdated -> with(event) {
                val clientSize = window.getClientSize()
                if (captionButtons.onPointerMove(locationInWindow, clientSize.width, captionButtonModel())) {
                    window.requestRedraw()
                }
                if (counterButton.onPointerMove(locationInWindow, clientSize.height)) {
                    window.requestRedraw()
                }
                if (captionButtons.pressed == null &&
                    !counterButton.pressed &&
                    dragDropManager != null &&
                    !nonClientArea &&
                    state.pressedButtons.hasFlag(PointerButton.Left)
                ) {
                    DataObject.build {
                        addHtmlFragment("<b>HTML</b> <i>fragment</i>")
                        addTextItem("Hello drag and drop!")
                    }.use { dataObject ->
                        val dragSource = object : DragSource {
                            override fun onQueryContinueDrag(escapePressed: Boolean, modifiers: DragDropModifiers): DragDropContinueResult {
                                return when {
                                    escapePressed -> DragDropContinueResult.Cancel

                                    !modifiers.hasFlag(DragDropModifier.LeftButton) -> {
                                        Logger.debug { "Dropping (left button depressed)" }
                                        DragDropContinueResult.Drop
                                    }

                                    else -> DragDropContinueResult.Continue
                                }
                            }
                        }
                        dragDropManager?.doDragDrop(dataObject, DragDropEffect.Copy, dragSource)
                        Logger.debug { "Drag finished" }
                    }
                }
                EventHandlerResult.Continue
            }

            is Event.PointerDown -> with(event) {
                // The caption buttons are reported as non-client (see the NCHitTest handler), so the
                // press arrives here with nonClientArea = true and `locationInWindow` in logical
                // client space. Consume it (Stop) so it never reaches DefWindowProc: its legacy
                // NC-button modal loop wedges under EnableMouseInPointer + Snap Layouts, stalling
                // the render loop. The button is activated on release, in PointerUp.
                val clientSize = window.getClientSize()
                val pressedCaption = button == PointerButton.Left &&
                    captionButtons.onPointerDown(locationInWindow, clientSize.width, captionButtonModel())
                // The counter button lives in the client area, so its press arrives here too.
                // Consume it (Stop) so it activates on release without reaching DefWindowProc.
                val pressedCounter = !pressedCaption &&
                    button == PointerButton.Left &&
                    counterButton.onPointerDown(locationInWindow, clientSize.height)
                if (pressedCaption || pressedCounter) {
                    window.requestRedraw()
                    EventHandlerResult.Stop
                } else {
                    EventHandlerResult.Continue
                }
            }

            is Event.PointerUp -> with(event) {
                val clientSize = window.getClientSize()
                when {
                    captionButtons.pressed != null -> {
                        when (captionButtons.onPointerUp(locationInWindow, clientSize.width, captionButtonModel())) {
                            CaptionButtonKind.Minimize -> window.minimize()
                            CaptionButtonKind.Maximize -> if (window.isMaximized()) window.restore() else window.maximize()
                            CaptionButtonKind.Close -> window.requestClose()
                            null -> {}
                        }
                        window.requestRedraw()
                        EventHandlerResult.Stop
                    }
                    counterButton.pressed -> {
                        if (counterButton.onPointerUp(locationInWindow, clientSize.height)) {
                            Logger.debug { "Counter incremented to ${counterButton.count}" }
                        }
                        window.requestRedraw()
                        EventHandlerResult.Stop
                    }
                    else -> EventHandlerResult.Continue
                }
            }

            is Event.PointerExited -> {
                var redraw = captionButtons.onPointerExit()
                redraw = counterButton.onPointerExit() || redraw
                if (redraw) {
                    window.requestRedraw()
                }
                EventHandlerResult.Continue
            }

            is Event.NCHitTest -> with(event) {
                // WM_NCHITTEST coordinates are physical screen pixels; map them into logical
                // client space to reuse the caption-button layout. An enabled button reports its
                // HT* code, handing the Snap Layouts flyover (on Maximize) to Windows; a visible
                // but disabled button reports Caption so its area still drags the window (matching
                // the system title bar); the rest of the title-bar band reports Caption so it drags
                // too. Anything below the title bar is left to the client: returning Continue lets
                // the toolkit fall back to its HTCLIENT default.
                val model = captionButtonModel()
                val clientPoint = Screen.mapToClient(window, PhysicalPoint(mouseX, mouseY)).toLogical(window.getScaleFactor())
                val kind = captionButtons.hitTest(clientPoint, window.getClientSize().width)?.takeIf { model.isVisible(it) }
                when {
                    kind == null -> {
                        // Not over a button. Drag only within the title-bar band; below it is client.
                        if (clientPoint.y !in 0f..captionButtons.titleBarHeight) {
                            return@with EventHandlerResult.Continue
                        }
                        setHitTestResult(NCHitTestResult.Caption)
                    }
                    else -> setHitTestResult(
                        when (kind) {
                            CaptionButtonKind.Minimize -> NCHitTestResult.MinButton
                            CaptionButtonKind.Maximize -> NCHitTestResult.MaxButton
                            CaptionButtonKind.Close -> NCHitTestResult.Close
                        },
                    )
                }
                EventHandlerResult.Stop
            }

            else -> EventHandlerResult.Continue
        }
    }

    private fun performDrawing(size: PhysicalSize, scale: Float) {
        angleRenderer.makeCurrent()
        if (isSizeChanged(size)) {
            // Update currentSize only after resizeSurface returns Ok. If the EGL
            // call throws, currentSize stays at the old value so the next frame
            // retries the resize against the same target size instead of skipping it.
            surfaceParams = angleRenderer.resizeSurface(size.width, size.height)
            currentSize = size
        }
        BackendRenderTarget.makeGL(
            width = size.width,
            height = size.height,
            sampleCnt = 1,
            stencilBits = 8,
            fbId = surfaceParams!!.framebufferBinding,
            fbFormat = FramebufferFormat.GR_GL_RGBA8,
        ).use { renderTarget ->
            Surface.makeFromBackendRenderTarget(
                context = directContext,
                rt = renderTarget,
                origin = SurfaceOrigin.BOTTOM_LEFT,
                colorFormat = SurfaceColorFormat.RGBA_8888,
                colorSpace = ColorSpace.sRGB,
                surfaceProps = null,
            )!!.use { surface ->
                val time = creationTime.elapsedNow().inWholeMilliseconds
                surface.canvas.clear(Color.TRANSPARENT)
                surface.canvas.draw(size, scale, time)
                captionButtons.draw(surface.canvas, size, scale, immersiveDark, window.isMaximized(), captionButtonModel())
                counterButton.draw(surface.canvas, size, scale, immersiveDark)
                surface.flushAndSubmit()
                angleRenderer.swapBuffers()
            }
        }
    }

    abstract fun Canvas.draw(size: PhysicalSize, scale: Float, time: Long)

    override fun close() {
        dragDropManager?.let { manager ->
            manager.revokeDropTarget()
            manager.close()
        }
        window.destroy()
        window.close()
    }
}

// Clipboard helpers. The OLE clipboard can be momentarily locked by another process, and the
// synchronous Clipboard API leaves retrying to the caller — so every access is wrapped in a
// bounded busy-retry. These run on the dispatcher (OLE STA) thread, where the clipboard must be used.

private fun copyToClipboard() {
    // set + flush keeps the data available after this process exits.
    DataObject.build {
        addTextItem("Hello clipboard!")
        addHtmlFragment("Hello <b>clipboard</b>!")
    }.use { data ->
        when (val setResult = retryWhileClipboardBusy { Clipboard.set(data) }) {
            is ClipboardResult.Success -> retryWhileClipboardBusy { Clipboard.flush() }.logIfFailed("flush")
            is ClipboardResult.Failure -> Logger.error { "Clipboard set failed: ${setResult.status}" }
        }
    }
}

private fun pasteFromClipboard() {
    when (val readResult = retryWhileClipboardBusy { Clipboard.get() }) {
        is ClipboardResult.Success -> readResult.value.use {
            Logger.debug { "Clipboard text: ${it.tryReadTextItem()}" }
            Logger.debug { "Clipboard HTML fragment: ${it.tryReadHtmlFragment()}" }
        }
        is ClipboardResult.Failure -> Logger.error { "Clipboard read failed: ${readResult.status}" }
    }
}

private fun <T> retryWhileClipboardBusy(attempts: Int = 8, operation: () -> ClipboardResult<T>): ClipboardResult<T> {
    repeat(attempts - 1) {
        val result = operation()
        if (!result.isBusy) {
            return result
        }
        Thread.sleep(10)
    }
    return operation()
}

private fun ClipboardResult<*>.logIfFailed(action: String) {
    if (this is ClipboardResult.Failure) {
        Logger.error { "Clipboard $action failed: $status" }
    }
}
