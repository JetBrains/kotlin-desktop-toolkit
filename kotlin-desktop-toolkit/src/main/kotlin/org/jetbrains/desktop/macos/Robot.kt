package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h

public object Robot {
    public fun emulateKeyboardEvent(key: KeyCode, isKeyDown: Boolean = true) {
        ffiDownCall {
            desktop_macos_h.emulate_keyboard_event(key.value, isKeyDown)
        }
    }
}
