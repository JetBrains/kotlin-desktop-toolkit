package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public object TextInputSource {
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

    public fun list(): List<String> {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                val result = desktop_macos_h.text_input_source_list(arena)
                try {
                    listOfStringsFromNative(result)
                } finally {
                    ffiDownCall { desktop_macos_h.string_array_drop(result) }
                }
            }
        }
    }

    /**
     * macOS has race conditions in input source switching
     * immediately after you can observe some weird key events (even if [current] reports expected value), for example,
     * [Event.KeyDown.characters] corresponds to new layout [Event.KeyDown.key] and [Event.KeyDown.keyWithModifiers] to old layout.
     * This function is usually used in tests, and as a workaround you can wait for 10 ms.
     * See: [KDTApplicationTestBase.withInputSource]
     */
    public fun select(sourceId: String): Boolean {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.text_input_source_select(arena.allocateUtf8String(sourceId))
            }
        }
    }

    public fun isAsciiCapable(sourceId: String): Boolean {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.text_input_source_is_ascii_capable(arena.allocateUtf8String(sourceId))
            }
        }
    }
}
