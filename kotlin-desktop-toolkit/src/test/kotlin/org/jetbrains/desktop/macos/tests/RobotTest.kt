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
    /**
     * Otherwise when we emulate some key press e.g., Shift+A, modifier might have no effect
     */

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
            val events = mutableListOf<Event>()
            withEventHandler(handler = {
                events.add(it)
                EventHandlerResult.Continue
            }) {
                ui { robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = true) }

                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_A, isKeyDown = true) }

                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_A, isKeyDown = false) }

                ui { robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = false) }

                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_X, isKeyDown = true) }
                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_X, isKeyDown = false) }
                awaitEventOfType<Event.KeyUp> { it.keyCode == KeyCode.ANSI_X }

                val keyDownEvents = events.filterIsInstance<Event.KeyDown>()
                println("Events:\n" + events.joinToString("\n"))
                assertNotNull(
                    keyDownEvents.firstOrNull {
                        it.keyCode == KeyCode.ANSI_A &&
                            it.typedCharacters.uppercase() == it.typedCharacters
                    },
                )
            }
        }
    }
}
