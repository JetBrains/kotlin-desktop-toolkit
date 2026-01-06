package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.KeyModifiersSet
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Robot
import org.jetbrains.desktop.macos.Window
import kotlin.test.Test

class KeyboardTest : KDTApplicationTestBase() {

    fun Set<KeyCode>.toModifiersSet(): KeyModifiersSet {
        return KeyModifiersSet.create(
            shift = contains(KeyCode.Shift) || contains(KeyCode.RightShift),
            control = contains(KeyCode.Control) || contains(KeyCode.RightControl),
            option = contains(KeyCode.Option) || contains(KeyCode.RightOption),
            command = contains(KeyCode.Command) || contains(KeyCode.RightCommand),
            capsLock = contains(KeyCode.CapsLock),
            numericPad = false,
            help = false,
            function = false
        )
    }

    fun pressOneKeyAndAwaitEvent(
        keyCode: KeyCode,
        typed: String,
        key: String,
        keyNoModifiers: String,
        modifiers: Set<KeyCode>
    ) {
        for (modifier in modifiers) {
            ui {
                Robot.emulateKeyboardEvent(modifier, true)
            }
        }

        ui {
            Robot.emulateKeyboardEvent(keyCode, true)
        }
        ui {
            Robot.emulateKeyboardEvent(keyCode, false)
        }

        for (modifier in modifiers) {
            ui {
                Robot.emulateKeyboardEvent(modifier, false)
            }
        }

        awaitEventOfType<Event.KeyDown> {
            it.keyCode == keyCode &&
                    it.typedCharacters == typed &&
                    it.key == key &&
                    it.keyWithModifiers == keyNoModifiers &&
                    it.modifiers == modifiers.toModifiersSet()
        }
        awaitEventOfType<Event.KeyUp> {
            it.keyCode == keyCode &&
                    it.typedCharacters == typed &&
                    it.key == key &&
                    it.keyWithModifiers == keyNoModifiers &&
                    it.modifiers == modifiers.toModifiersSet()
        }
    }


    @Test
    fun latinLettersTest() {
        val window = ui {
            Window.create(origin = LogicalPoint(100.0, 200.0), title = "Main Window")
        }
        ui {
            window.makeKeyAndOrderFront()
        }
        awaitEventOfType<Event.WindowFocusChange> { it.isKeyWindow }
        pressOneKeyAndAwaitEvent(KeyCode.ANSI_A, typed = "a", key = "a", keyNoModifiers = "a", modifiers = emptySet())
        pressOneKeyAndAwaitEvent(KeyCode.ANSI_B, typed = "b", key = "b", keyNoModifiers = "b", modifiers = emptySet())
        ui {
            window.close()
        }
    }
}
