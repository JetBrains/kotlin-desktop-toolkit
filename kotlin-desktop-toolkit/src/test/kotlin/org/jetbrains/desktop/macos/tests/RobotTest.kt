package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Robot
import org.jetbrains.desktop.macos.Window
import org.jetbrains.desktop.macos.tests.KeyboardTest.Companion.window
import kotlin.test.Test
import kotlin.test.assertNotNull

class RobotTest : KDTApplicationTestBase() {
    /**
     * Otherwise when we emulate some key press e.g., Shift+A, modifier might have no effect
     */
    @Test
    fun `robot waits until the event is delivered to os`() {
        window = ui {
            Window.create(origin = LogicalPoint(100.0, 200.0), title = "Main Window")
        }
        ui {
            window.makeKeyAndOrderFront()
        }
        awaitEventOfType<Event.WindowFocusChange> { it.isKeyWindow }
        val events = mutableListOf<Event>()
        eventHandler = { event ->
            events.add(event)
            EventHandlerResult.Continue
        }
        ui { Robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = true) }

        ui { Robot.emulateKeyboardEvent(KeyCode.ANSI_A, isKeyDown = true) }

        ui { Robot.emulateKeyboardEvent(KeyCode.ANSI_A, isKeyDown = false) }

        ui { Robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = false) }

        ui { Robot.emulateKeyboardEvent(KeyCode.ANSI_X, isKeyDown = true) }
        ui { Robot.emulateKeyboardEvent(KeyCode.ANSI_X, isKeyDown = false) }
        awaitEventOfType<Event.KeyUp> { it.keyCode == KeyCode.ANSI_X }

        val keyDownEvents = events.filterIsInstance<Event.KeyDown>()
        println("Events:\n" + events.joinToString("\n"))
        assertNotNull(keyDownEvents.firstOrNull { it.typedCharacters == "A" })

    }
}