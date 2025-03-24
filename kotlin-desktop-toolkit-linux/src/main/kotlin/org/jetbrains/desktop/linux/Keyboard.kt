package org.jetbrains.desktop.linux

@JvmInline
public value class KeyCode internal constructor(private val value: String) {
    public companion object {
        internal fun fromNative(code: Int): KeyCode {
            return when (code) {
                else -> KeyCode("") // TODO
            }
        }
    }
}

public data class KeyModifiers(
    val capsLock: Boolean,
    val shift: Boolean,
    val control: Boolean,
    val alt: Boolean,
    val logo: Boolean,
    val numLock: Boolean,
) {
    internal companion object
}
