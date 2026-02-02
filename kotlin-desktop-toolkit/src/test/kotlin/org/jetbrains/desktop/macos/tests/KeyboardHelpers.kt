package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.KeyModifiersSet
import org.jetbrains.desktop.macos.tests.KDTApplicationTestBase.Companion.ui
import org.jetbrains.desktop.macos.tests.KeyboardTest.Companion.robot
import kotlin.collections.forEach

object KeyboardHelpers {
    fun Set<KeyCode>.toModifiersSet(): KeyModifiersSet {
        return KeyModifiersSet.create(
            shift = contains(KeyCode.Shift) || contains(KeyCode.RightShift),
            control = contains(KeyCode.Control) || contains(KeyCode.RightControl),
            option = contains(KeyCode.Option) || contains(KeyCode.RightOption),
            command = contains(KeyCode.Command) || contains(KeyCode.RightCommand),
            capsLock = contains(KeyCode.CapsLock),
            numericPad = false,
            help = false,
            function = false,
        )
    }

    fun withModifiersPressed(modifiers: Set<KeyCode>, block: () -> Unit) {
        try {
            modifiers.forEach { ui { robot.emulateKeyboardEvent(it, true) } }
            block()
        } finally {
            modifiers.forEach { ui { robot.emulateKeyboardEvent(it, false) } }
        }
    }

    fun assertKeyDown(
        event: Event.KeyDown,
        keyCode: KeyCode,
        typed: String,
        key: String,
        keyWithModifiers: String,
        modifiers: Set<KeyCode>,
    ) {
        val isKeyDownExpected = event.characters == typed &&
            event.key == key &&
            event.keyWithModifiers == keyWithModifiers &&
            event.modifiers == modifiers.toModifiersSet()
        assert(isKeyDownExpected) {
            "Expected keyDown event with keyCode=$keyCode, typed=$typed, key=$key, keyWithModifiers=$keyWithModifiers, modifiers=$modifiers, but got $event"
        }
    }

    fun assertKeyUp(event: Event.KeyUp, keyCode: KeyCode, typed: String, key: String, keyWithModifiers: String, modifiers: Set<KeyCode>) {
        val isKeyDownExpected = event.characters == typed &&
            event.key == key &&
            event.keyWithModifiers == keyWithModifiers &&
            event.modifiers == modifiers.toModifiersSet()
        assert(isKeyDownExpected) {
            "Expected keyUp event with keyCode=$keyCode, typed=$typed, key=$key, keyWithModifiers=$keyWithModifiers, modifiers=$modifiers, but got $event"
        }
    }
}
