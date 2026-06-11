package org.jetbrains.desktop.win32.tests

import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.jetbrains.desktop.win32.Window
import org.jetbrains.desktop.win32.WindowParams
import org.jetbrains.desktop.win32.WindowStyle
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.nio.file.Path
import java.util.concurrent.atomic.AtomicReference

@EnabledOnOs(OS.WINDOWS)
class WindowStyleTests {
    private fun runStyleTest(initialStyle: WindowStyle, body: (Window) -> Unit) {
        KotlinDesktopToolkit.init(
            libraryFolderPath = Path.of(System.getProperty("kdt.win32.library.folder.path")!!),
        )
        val failure = AtomicReference<Throwable>()
        Application().use { app ->
            app.onStartup {
                try {
                    val window = app.newWindow()
                    window.create(WindowParams(title = "Style Test", style = initialStyle))
                    body(window)
                    window.requestClose()
                } catch (t: Throwable) {
                    failure.set(t)
                    app.stopEventLoop()
                }
            }
            app.runEventLoop { _, event ->
                if (event is Event.WindowCloseRequest) {
                    app.stopEventLoop()
                }
                EventHandlerResult.Continue
            }
        }
        failure.get()?.let { throw it }
    }

    @Test
    fun `default style flags are all true`() {
        runStyleTest(WindowStyle()) { window ->
            assertTrue(window.isResizable(), "isResizable should default to true")
            assertTrue(window.isMinimizable(), "isMinimizable should default to true")
            assertTrue(window.isMaximizable(), "isMaximizable should default to true")
        }
    }

    @Test
    fun `creation-time style flags are reflected by getters`() {
        runStyleTest(
            WindowStyle(
                isResizable = false,
                isMinimizable = false,
                isMaximizable = false,
            ),
        ) { window ->
            assertFalse(window.isResizable())
            assertFalse(window.isMinimizable())
            assertFalse(window.isMaximizable())
        }
    }

    @Test
    fun `setResizable toggles the flag without affecting others`() {
        runStyleTest(WindowStyle()) { window ->
            window.setResizable(false)
            assertFalse(window.isResizable())
            assertTrue(window.isMinimizable())
            assertTrue(window.isMaximizable())

            window.setResizable(true)
            assertTrue(window.isResizable())
            assertTrue(window.isMinimizable())
            assertTrue(window.isMaximizable())
        }
    }

    @Test
    fun `setMinimizable toggles the flag without affecting others`() {
        runStyleTest(WindowStyle()) { window ->
            window.setMinimizable(false)
            assertTrue(window.isResizable())
            assertFalse(window.isMinimizable())
            assertTrue(window.isMaximizable())

            window.setMinimizable(true)
            assertTrue(window.isResizable())
            assertTrue(window.isMinimizable())
            assertTrue(window.isMaximizable())
        }
    }

    @Test
    fun `setMaximizable toggles the flag without affecting others`() {
        runStyleTest(WindowStyle()) { window ->
            window.setMaximizable(false)
            assertTrue(window.isResizable())
            assertTrue(window.isMinimizable())
            assertFalse(window.isMaximizable())

            window.setMaximizable(true)
            assertTrue(window.isResizable())
            assertTrue(window.isMinimizable())
            assertTrue(window.isMaximizable())
        }
    }

    @Test
    fun `repeated setter calls with the same value are idempotent`() {
        runStyleTest(WindowStyle()) { window ->
            window.setResizable(false)
            window.setResizable(false)
            assertFalse(window.isResizable())

            window.setMinimizable(false)
            window.setMinimizable(false)
            assertFalse(window.isMinimizable())

            window.setMaximizable(false)
            window.setMaximizable(false)
            assertFalse(window.isMaximizable())
        }
    }

    @Test
    fun `all setters combined produce expected state`() {
        runStyleTest(WindowStyle()) { window ->
            window.setResizable(false)
            window.setMinimizable(false)
            window.setMaximizable(false)

            assertEquals(
                listOf(false, false, false),
                listOf(window.isResizable(), window.isMinimizable(), window.isMaximizable()),
            )

            window.setMinimizable(true)
            assertEquals(
                listOf(false, true, false),
                listOf(window.isResizable(), window.isMinimizable(), window.isMaximizable()),
            )
        }
    }
}
