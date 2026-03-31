package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

/**
 * Kotlin wrapper around Apple's Text Input Source Services (`TISInputSource` API from Carbon/HIToolbox).
 *
 * Text input sources on macOS fall into three categories:
 * - **Keyboard input sources** — keyboard layouts, keyboard input methods, and input modes
 *   (e.g. `com.apple.keyboardlayout.US`, `com.apple.Kotoeri.Katakana`)
 * - **Palette input sources** — character palette, keyboard viewer, private dictionary panels
 * - **Ink input sources**
 *
 * Each input source is identified by a unique reverse-DNS string (the *source ID*),
 * such as `com.apple.keylayout.US` or `com.apple.inputmethod.SCIM`.
 *
 * Exactly one keyboard input source is selected (current) at any time. Selecting a new one
 * automatically deselects the previous one. Multiple palette/ink sources may be selected simultaneously.
 *
 * Input sources can be *installed*, *enabled*, and *selected* — these are distinct states:
 * - **Installed** — present on the system but not necessarily visible to the user.
 * - **Enabled** — available for selection in the input menu / keyboard switcher.
 * - **Selected** — actively receiving input.
 *
 * For the full Apple reference, see:
 * [Text Input Source Services Reference](https://leopard-adc.pepas.com/documentation/TextFonts/Reference/TextInputSourcesReference/TextInputSourcesReference.pdf)
 */
public object TextInputSource {

    /**
     * Returns the source ID of the currently selected keyboard input source,
     * or `null` if it cannot be determined.
     *
     * Wraps `TISCopyCurrentKeyboardInputSource` + `kTISPropertyInputSourceID`.
     */
    public fun current(): String? {
        val layout = ffiDownCall {
            desktop_macos_h.text_input_source_current()
        }
        if (layout == MemorySegment.NULL) return null
        return try {
            layout.getUtf8String(0)
        } finally {
            ffiDownCall { desktop_macos_h.string_drop(layout) }
        }
    }

    /**
     * Returns a list of source IDs for keyboard input sources.
     *
     * @param includeAll when `false` (default), returns only *enabled* input sources;
     *   when `true`, returns *all installed* input sources, including disabled ones.
     *   **Note:** passing `true` may have significant memory impact if many input sources
     *   are installed, as it forces caching of data for all matching sources.
     *
     * Wraps `TISCreateInputSourceList`.
     */
    public fun list(includeAll: Boolean = false): List<String> {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                val result = desktop_macos_h.text_input_source_list(arena, includeAll)
                try {
                    listOfStringsFromNative(result)
                } finally {
                    ffiDownCall { desktop_macos_h.string_array_drop(result) }
                }
            }
        }
    }

    /**
     * Selects the input source identified by [sourceId], making it the current keyboard input source.
     *
     * For selection to succeed, the input source must be both *select-capable*
     * ([isSelectCapable] returns `true`) and *enabled*. If the input source is an
     * input mode, its parent input method must also be enabled.
     *
     * @param sourceId the reverse-DNS identifier of the input source
     *   (e.g. `"com.apple.keylayout.US"`).
     * @return `true` if the input source was successfully selected, `false` otherwise.
     *
     * **Caveat:** macOS has race conditions in input source switching — immediately after
     * selecting, you may observe inconsistent key events (even if [current] reports the
     * expected value). For example, [Event.KeyDown.characters] may correspond to the new
     * layout while [Event.KeyDown.key] and [Event.KeyDown.keyWithModifiers] still reflect
     * the old layout. As a workaround, wait ~10 ms after selecting.
     *
     * Wraps `TISSelectInputSource`.
     */
    public fun select(sourceId: String): Boolean {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.text_input_source_select(arena.allocateUtf8String(sourceId))
            }
        }
    }

    /**
     * Returns the type of the input source identified by [sourceId], or `null` if
     * the source is not found.
     *
     * Possible values (CFString constants from `TextInputSources.h`):
     * - `"TISTypeKeyboardLayout"` — a keyboard layout
     * - `"TISTypeKeyboardInputMethodWithoutModes"` — an input method without modes
     * - `"TISTypeKeyboardInputMethodModeEnabled"` — a mode-enabled input method (parent)
     * - `"TISTypeKeyboardInputMode"` — an input mode of a parent input method
     * - `"TISTypeCharacterPalette"` — a character palette
     * - `"TISTypeKeyboardViewer"` — a keyboard viewer
     * - `"TISTypeInk"` — an ink input source
     *
     * Wraps `kTISPropertyInputSourceType`.
     */
    public fun type(sourceId: String): String? {
        val typePtr = ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.text_input_source_type(arena.allocateUtf8String(sourceId))
            }
        }
        if (typePtr == MemorySegment.NULL) return null
        return try {
            typePtr.getUtf8String(0)
        } finally {
            ffiDownCall { desktop_macos_h.string_drop(typePtr) }
        }
    }

    /**
     * Returns the source ID of the parent input source for the given [sourceId],
     * or `null` if no parent exists.
     *
     * Only input modes (type `TISTypeKeyboardInputMode`) typically have a parent —
     * the parent is the mode-enabled input method whose ID is a prefix of the mode's ID.
     * For example, `"com.apple.inputmethod.Kotoeri.RomajiTyping.Japanese"` has the parent
     * `"com.apple.inputmethod.Kotoeri.RomajiTyping"`.
     *
     * Keyboard layouts and standalone input methods return `null`.
     */
    public fun getParent(sourceId: String): String? {
        val parentPtr = ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.text_input_source_get_parent(arena.allocateUtf8String(sourceId))
            }
        }
        if (parentPtr == MemorySegment.NULL) return null
        return try {
            parentPtr.getUtf8String(0)
        } finally {
            ffiDownCall { desktop_macos_h.string_drop(parentPtr) }
        }
    }

    /**
     * Returns whether the input source identified by [sourceId] is capable of ASCII input.
     *
     * Wraps `kTISPropertyInputSourceIsASCIICapable`.
     */
    public fun isAsciiCapable(sourceId: String): Boolean {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.text_input_source_is_ascii_capable(arena.allocateUtf8String(sourceId))
            }
        }
    }

    /**
     * Returns whether the input source identified by [sourceId] can ever be programmatically
     * selected via [select].
     *
     * This is a static property of the input source and does not depend on its current
     * enabled/disabled state. Input sources that are never select-capable include parent
     * input methods that have modes — only their individual modes can be selected.
     *
     * Wraps `kTISPropertyInputSourceIsSelectCapable`.
     */
    public fun isSelectCapable(sourceId: String): Boolean {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.text_input_source_is_select_capable(arena.allocateUtf8String(sourceId))
            }
        }
    }

    /**
     * Returns whether the input source identified by [sourceId] can ever be programmatically
     * enabled via [setEnabledExact].
     *
     * Most input sources are enable-capable. Exceptions include input-method-private
     * keyboard layouts (used internally via `TISSetInputMethodKeyboardLayoutOverride`),
     * which cannot be directly enabled. Input modes are enable-capable but can only
     * transition from disabled to enabled when their parent input method is already enabled.
     *
     * Wraps `kTISPropertyInputSourceIsEnableCapable`.
     */
    public fun isEnableCapable(sourceId: String): Boolean {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.text_input_source_is_enable_capable(arena.allocateUtf8String(sourceId))
            }
        }
    }

    /**
     * Enables or disables the input source identified by [sourceId], automatically
     * targeting the parent input method when [sourceId] is an input mode.
     *
     * For example, calling `setEnabled("com.apple.inputmethod.Kotoeri.RomajiTyping.Japanese", true)`
     * will enable the parent `"com.apple.inputmethod.Kotoeri.RomajiTyping"` instead,
     * since input modes cannot be enabled directly without their parent.
     *
     * Use [setEnabledExact] if you need to enable/disable a specific source ID without
     * parent resolution.
     *
     * @param sourceId the reverse-DNS identifier of the input source.
     * @param enabled `true` to enable, `false` to disable.
     * @return `true` if the operation succeeded, `false` otherwise.
     */
    public fun setEnabled(sourceId: String, enabled: Boolean): Boolean {
        val sourceIdToEnable = getParent(sourceId) ?: sourceId
        return setEnabledExact(sourceIdToEnable, enabled)
    }

    /**
     * Enables or disables the input source identified by [sourceId].
     *
     * Enabling makes the input source available for selection in the input menu.
     * Disabling removes it from the user interface and makes it unavailable for selection.
     * At least one keyboard input source must remain enabled at all times.
     *
     * For enabling to succeed, the input source must be *enable-capable*
     * ([isEnableCapable] returns `true`). If the input source is an input mode,
     * its parent input method must already be enabled.
     *
     * @param sourceId the reverse-DNS identifier of the input source.
     * @param enabled `true` to enable, `false` to disable.
     * @return `true` if the operation succeeded, `false` otherwise.
     *
     * Wraps `TISEnableInputSource` / `TISDisableInputSource`.
     */
    public fun setEnabledExact(sourceId: String, enabled: Boolean): Boolean {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.text_input_source_set_enable(arena.allocateUtf8String(sourceId), enabled)
            }
        }
    }
}
