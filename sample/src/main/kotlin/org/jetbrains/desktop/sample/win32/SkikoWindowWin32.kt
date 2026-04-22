package org.jetbrains.desktop.sample.win32

import org.jetbrains.desktop.win32.AngleRenderer
import org.jetbrains.desktop.win32.Appearance
import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.CursorIcon
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
import org.jetbrains.desktop.win32.OleClipboard
import org.jetbrains.desktop.win32.PhysicalPoint
import org.jetbrains.desktop.win32.PhysicalSize
import org.jetbrains.desktop.win32.PointerButton
import org.jetbrains.desktop.win32.Screen
import org.jetbrains.desktop.win32.SurfaceParams
import org.jetbrains.desktop.win32.VirtualKey
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

abstract class SkikoWindowWin32(app: Application) : AutoCloseable {
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

    private fun isSizeChanged(size: PhysicalSize): Boolean {
        return (size.width != currentSize.width || size.height != currentSize.height)
    }

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
                        val html = dataObject.readHtmlFragment()
                        val text = dataObject.readTextItem()
                        Logger.debug { "html: $html -- text: $text" }
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

                    VirtualKey.C -> {
                        if (Keyboard.getKeyState(VirtualKey.Control).isDown) {
                            DataObject.build {
                                addTextItem("Hello OLE clipboard!")
                                addHtmlFragment("Hello <b>OLE clipboard</b>!")
                            }.use { clipboardData ->
                                OleClipboard.writeToClipboard(clipboardData)
                            }
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

                    VirtualKey.V -> {
                        if (Keyboard.getKeyState(VirtualKey.Control).isDown) {
                            val clipboardData = OleClipboard.readClipboard()
                            val textItem = clipboardData.readTextItem()
                            val htmlFragment = clipboardData.readHtmlFragment()
                            Logger.debug { "OLE clipboard text: $textItem" }
                            Logger.debug { "OLE clipboard HTML fragment: $htmlFragment" }
                        }
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
                val enableImmersiveDarkMode = newAppearance == Appearance.Dark
                window.setImmersiveDarkMode(enableImmersiveDarkMode)
                EventHandlerResult.Stop
            }

            is Event.WindowTitleChanged -> with(event) {
                Logger.debug { "New window title: $title" }
                EventHandlerResult.Continue
            }

            is Event.PointerUpdated -> with(event) {
                if (!nonClientArea && state.pressedButtons.hasFlag(PointerButton.Left)) {
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

            else -> EventHandlerResult.Continue
        }
    }

    private fun performDrawing(size: PhysicalSize, scale: Float) {
        angleRenderer.makeCurrent()
        if (isSizeChanged(size)) {
            currentSize = size
            surfaceParams = angleRenderer.resizeSurface(size.width, size.height)
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
