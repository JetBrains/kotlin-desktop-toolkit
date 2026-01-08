package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h

public class Robot: AutoCloseable {

    init {
        ffiDownCall {
            desktop_macos_h.robot_initialize()
        }
    }

    public fun emulateKeyboardEvent(key: KeyCode, isKeyDown: Boolean) {
        ffiDownCall {
            desktop_macos_h.emulate_keyboard_event(key.value, isKeyDown)
        }
    }

    override fun close() {
        ffiDownCall {
            desktop_macos_h.robot_deinitialize()
        }
    }
}
