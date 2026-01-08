package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Robot
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.BeforeAll
import kotlin.test.Test
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
}
