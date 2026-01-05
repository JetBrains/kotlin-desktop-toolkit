package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Robot
import org.jetbrains.desktop.macos.Window
import java.lang.Thread.sleep
import kotlin.test.Test

class KeyboardTest : KDTApplicationTestBase() {
    @Test
    fun smokeTest() {
        val events = mutableListOf<Event>()
        withEventHandler(handler = {
            events.add(it)
            return@withEventHandler EventHandlerResult.Continue
        }) {
            val window = ui {
                Window.create(origin = LogicalPoint(100.0, 200.0), title = "Hello1")
            }
            ui {
                window.makeKeyAndOrderFront()
            }
            sleep(1000)
            ui {
                Robot.emulateKeyboardEvent(KeyCode.ANSI_A, true)
                Robot.emulateKeyboardEvent(KeyCode.ANSI_A, false)
            }
            sleep(1000)
            ui {
                window.close()
            }
        }
        val keyUpEvents = events.filter { it is Event.KeyUp && it.keyCode == KeyCode.ANSI_A }
        val keyDownEvents = events.filter { it is Event.KeyDown && it.keyCode == KeyCode.ANSI_A }
        assert(keyUpEvents.isNotEmpty() && keyDownEvents.isNotEmpty()) {
            "No key events found for KeyCode.ANSI_A in $events"
        }
    }
}