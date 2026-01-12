package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Robot
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.BeforeAll
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class RobotTest : KDTApplicationTestBase() {

    companion object {
        lateinit var window: Window
        lateinit var robot: Robot

        @BeforeAll
        @JvmStatic
        fun init() {
            robot = ui { Robot() }
            window = ui {
                Window.create(origin = LogicalPoint(100.0, 200.0), title = "Main Window")
            }
            ui {
                window.makeKeyAndOrderFront()
            }
            awaitEventOfType<Event.WindowFocusChange> { it.isKeyWindow }
        }

        @AfterAll
        @JvmStatic
        fun destroy() {
            ui { robot.close() }
            ui {
                window.close()
            }
        }
    }

    @Test
    fun `robot waits until the event is delivered to os`() {
        repeat(100) {
            val hadCapitalA = java.util.concurrent.atomic.AtomicBoolean(false)
            withEventHandler(handler = {
                if (it is Event.KeyDown && it.keyCode == KeyCode.ANSI_A && it.typedCharacters == "A") {
                    hadCapitalA.set(true)
                }
                EventHandlerResult.Continue
            }) {
                ui { robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = true) }

                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_A, isKeyDown = true) }

                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_A, isKeyDown = false) }

                ui { robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = false) }

                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_X, isKeyDown = true) }
                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_X, isKeyDown = false) }
                awaitEventOfType<Event.KeyUp> { it.keyCode == KeyCode.ANSI_X }
            }
            assertNotNull(hadCapitalA.get())
        }
    }

    @Test
    fun `modifiers are correctly stacked`() {
        repeat(100) {
            withEventHandler(handler = {
                EventHandlerResult.Continue
            }) {
                ui { robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = true) }
                ui { robot.emulateKeyboardEvent(KeyCode.Command, isKeyDown = true) }
                ui { robot.emulateKeyboardEvent(KeyCode.Option, isKeyDown = true) }
                awaitEventOfType<Event.ModifiersChanged> {
                    it.keyCode == KeyCode.Option &&
                        it.modifiers.shift &&
                        it.modifiers.command &&
                        it.modifiers.option
                }

                ui { robot.emulateKeyboardEvent(KeyCode.Option, isKeyDown = false) }
                ui { robot.emulateKeyboardEvent(KeyCode.Command, isKeyDown = false) }
                ui { robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = false) }
                awaitEventOfType<Event.ModifiersChanged> {
                    it.keyCode == KeyCode.Shift &&
                        !it.modifiers.shift &&
                        !it.modifiers.command &&
                        !it.modifiers.option
                }
            }
        }
    }

    @Test
    fun `modifiers command option`() {
        repeat(100) {
            withEventHandler(handler = {
                println("Event: $it")
                EventHandlerResult.Continue
            }) {
                ui { robot.emulateKeyboardEvent(KeyCode.Command, isKeyDown = true) }
                ui { robot.emulateKeyboardEvent(KeyCode.Option, isKeyDown = true) }
                awaitEventOfType<Event.ModifiersChanged> {
                    it.modifiers.command &&
                        it.modifiers.option
                }

                ui { robot.emulateKeyboardEvent(KeyCode.Command, isKeyDown = false) }
                ui { robot.emulateKeyboardEvent(KeyCode.Option, isKeyDown = false) }
                awaitEventOfType<Event.ModifiersChanged> {
                    !it.modifiers.shift &&
                        !it.modifiers.command &&
                        !it.modifiers.option
                }
            }
        }
    }

    @Test
    fun `keyboard layout test`() {
        val layout = ui { Application.currentKeyboardLayout() }
        assertEquals("com.apple.keylayout.ABC", layout)
    }

    @Test
    fun `list input sources test`() {
        val inputSources = ui { Application.listInputSources() }
        println("Input sources: $inputSources")
        assert(inputSources.isNotEmpty()) { "Input sources list should not be empty" }
        assert(inputSources.any { it.startsWith("com.apple.keylayout.") }) {
            "Should contain at least one keyboard layout"
        }
    }

    @Test
    fun `current input source is in the list of input sources`() {
        val currentLayout = ui { Application.currentKeyboardLayout() }
        val inputSources = ui { Application.listInputSources() }
        assert(currentLayout != null) { "Current keyboard layout should not be null" }
        assertContains(inputSources, currentLayout, "Current keyboard layout should be in the list of input sources")
    }

    @Test
    fun `choose input source and restore`() {
        val originalLayout = ui { Application.currentKeyboardLayout() }
        assertNotNull(originalLayout)

        val inputSources = ui { Application.listInputSources() }
        val anotherLayout = inputSources.firstOrNull { it != originalLayout && it.startsWith("com.apple.keylayout.") }

        if (anotherLayout != null) {
            val switched = ui { Application.chooseInputSource(anotherLayout) }
            assert(switched) { "Failed to switch to $anotherLayout" }

            val currentAfterSwitch = ui { Application.currentKeyboardLayout() }
            assertEquals(anotherLayout, currentAfterSwitch)

            // Restore original layout
            val restored = ui { Application.chooseInputSource(originalLayout) }
            assert(restored) { "Failed to restore to $originalLayout" }

            val currentAfterRestore = ui { Application.currentKeyboardLayout() }
            assertEquals(originalLayout, currentAfterRestore)
        } else {
            println("Only one keyboard layout available, skipping switch test")
        }
    }

    // todo test CJK
}
