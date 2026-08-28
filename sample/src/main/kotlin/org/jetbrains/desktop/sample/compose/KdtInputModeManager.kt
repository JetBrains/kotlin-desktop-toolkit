package org.jetbrains.desktop.sample.compose

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.input.InputMode
import androidx.compose.ui.input.InputModeManager

/**
 * Tracks whether the user is currently driving the UI with the pointer or the keyboard, which is what
 * Compose keys focus rings off.
 *
 * Compose ships an implementation of this, but it is `internal` to compose-ui, so applications hosting
 * their own scene have to bring their own.
 *
 * [onInputModeChanged] is where a platform reacts to the switch — macOS, for instance, hides the cursor
 * while the user is typing.
 */
internal class KdtInputModeManager(
    initialInputMode: InputMode,
    private val onInputModeChanged: (InputMode) -> Unit = {},
) : InputModeManager {
    override var inputMode: InputMode by mutableStateOf(initialInputMode)
        private set

    @ExperimentalComposeUiApi
    override fun requestInputMode(inputMode: InputMode): Boolean {
        if (inputMode != InputMode.Touch && inputMode != InputMode.Keyboard) {
            return false
        }
        if (inputMode != this.inputMode) {
            this.inputMode = inputMode
            onInputModeChanged(inputMode)
        }
        return true
    }
}
