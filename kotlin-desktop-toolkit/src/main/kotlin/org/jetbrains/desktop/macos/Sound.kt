package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena

/**
 * macOS System Sound API.
 *
 * Provides access to play system sounds by name.
 */
public object Sound {
    /**
     * Plays a system sound by name.
     *
     * This function plays a named system sound (e.g., "Basso", "Blow", "Bottle", "Frog", "Funk", "Glass",
     * "Hero", "Morse", "Ping", "Pop", "Purr", "Sosumi", "Submarine", "Tink").
     *
     * The sound files are typically located in `/System/Library/Sounds/`.
     *
     * @param soundName The name of the system sound to play (without the .aiff extension)
     * @return true if the sound was found and played successfully, false otherwise
     *
     * @see [Apple Developer Documentation](https://developer.apple.com/documentation/appkit/nssound)
     */
    public fun playNamed(soundName: String): Boolean {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                val soundNamePtr = arena.allocateUtf8String(soundName)
                desktop_macos_h.sound_play_named(soundNamePtr)
            }
        }
    }
}
