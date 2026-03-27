package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h

@JvmInline
public value class KeyboardType internal constructor(internal val value: Int) {
    public companion object {
        public val Ansi: KeyboardType = KeyboardType(40)
        public val Iso: KeyboardType = KeyboardType(41)
        public val Jis: KeyboardType = KeyboardType(42)
    }
}

public class Robot : AutoCloseable {

    init {
        ffiDownCall {
            desktop_macos_h.robot_initialize()
        }
    }

    /**
     * Returns `true` if accessibility is allowed on the current process.
     * Accessibility permissions are required to use the [Robot].
     * To enable accessibility, go to System Preferences > Security & Privacy > Accessibility.
     * Or use the tool like https://github.com/jacobsalmela/tccutil
     * Accessibility permissions are inherited from parent processes.
     * E.g., you might grant the permissions to IJ if you run tests from its terminal
     */
    public fun isAccessibilityAllowed(): Boolean {
        return ffiDownCall {
            desktop_macos_h.robot_is_accessibility_allowed()
        }
    }

    public fun emulateKeyboardEvent(key: KeyCode, isKeyDown: Boolean) {
        ffiDownCall {
            desktop_macos_h.emulate_keyboard_event(key.value, isKeyDown)
        }
    }

    public fun setKeyboardType(type: KeyboardType) {
        ffiDownCall {
            desktop_macos_h.robot_set_keyboard_type(type.value)
        }
    }

    override fun close() {
        ffiDownCall {
            desktop_macos_h.robot_deinitialize()
        }
    }
}
