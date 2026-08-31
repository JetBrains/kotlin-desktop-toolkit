package org.jetbrains.desktop.compose.macos

import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.KeyEventType
import org.jetbrains.desktop.macos.Event
import java.awt.Component
import java.awt.event.InputEvent
import kotlin.math.roundToInt
import java.awt.event.KeyEvent as AwtKeyEvent
import java.awt.event.MouseEvent as AwtMouseEvent
import java.awt.event.MouseWheelEvent as AwtMouseWheelEvent

/**
 * The component every synthesized AWT event is attributed to.
 *
 * Compose answers some questions about an event by inspecting the AWT event behind it — whether a key
 * press inserts a character, whether a scroll came from a trackpad. There is no way to satisfy those
 * checks with a kotlin-desktop-toolkit event, so instead a matching AWT event is synthesized and carried
 * along as the Compose event's `nativeEvent`.
 */
private object SyntheticEventSource : Component()

/**
 * The character a key press should insert, or `null` if it should not insert anything.
 *
 * Mirrors the checks Compose applies to AWT events, plus the two macOS-specific exclusions: control
 * characters, and the private-use range AppKit uses to encode arrows and function keys. Command and
 * control chords are excluded so that e.g. Cmd+A stays a shortcut rather than typing "a".
 */
private fun Event.KeyDown.typedCharOrNull(): Char? {
    if (modifiers.command || modifiers.control) {
        return null
    }
    // AWT's KEY_TYPED carries exactly one char; anything else is not a plain character insertion.
    val char = keyWithModifiers.text.singleOrNull() ?: return null
    if (Character.isISOControl(char)) {
        return null
    }
    // https://www.unicode.org/Public/MAPPINGS/VENDORS/APPLE/CORPCHAR.TXT
    if (char.code in 0xF700..0xF8FF) {
        return null
    }
    return if (char.isPrintable() || char.isWhitespace()) char else null
}

private fun Char.isPrintable(): Boolean {
    val block = Character.UnicodeBlock.of(this)
    return block != null && block != Character.UnicodeBlock.SPECIALS
}

/**
 * The extra Compose event that inserts a character, or `null` when this key press types nothing.
 *
 * Compose decides whether a key event should insert a character with its internal `isTypedEvent`, which
 * is implemented as "the native event is a `java.awt.event.KeyEvent` with `id == KEY_TYPED`, carrying a
 * printable `keyChar`".
 *
 * AWT-backed Compose delivers KEY_PRESSED, KEY_TYPED and KEY_RELEASED as three separate Compose events,
 * and only the KEY_TYPED one inserts text. This mirrors that: the event produced here is sent in
 * addition to the ordinary KeyDown rather than replacing it. Its shape matches what Compose derives from
 * a real AWT KEY_TYPED event: type [KeyEventType.Unknown] and an undefined key code, with the character
 * in `codePoint`.
 */
@OptIn(InternalComposeUiApi::class)
internal fun Event.KeyDown.typedKeyEventOrNull(): KeyEvent? {
    val char = typedCharOrNull() ?: return null
    var awtModifiers = 0
    if (modifiers.shift) {
        awtModifiers = awtModifiers or InputEvent.SHIFT_DOWN_MASK
    }
    if (modifiers.option) {
        awtModifiers = awtModifiers or InputEvent.ALT_DOWN_MASK
    }
    val awtEvent = AwtKeyEvent(
        SyntheticEventSource,
        AwtKeyEvent.KEY_TYPED,
        timestamp.toDuration().inWholeMilliseconds,
        awtModifiers,
        // KEY_TYPED rejects anything but an undefined key code.
        AwtKeyEvent.VK_UNDEFINED,
        char,
    )
    return KeyEvent(
        key = Key.Unknown,
        type = KeyEventType.Unknown,
        codePoint = char.code,
        isShiftPressed = modifiers.shift,
        isAltPressed = modifiers.option,
        nativeEvent = awtEvent,
    )
}

/**
 * The AWT event Compose reads a scroll's device from.
 *
 * `ScrollConfig.isPreciseWheelScroll` decides whether to run Compose's own smooth-scroll animation, and
 * the desktop implementation answers it from the AWT event: a wheel whose precise rotation differs from
 * its whole-notch one is a trackpad. Without one every scroll looks like a stepping wheel, and the
 * animation ends up stacked on top of the inertia macOS has already applied.
 *
 * `scrollType` and `scrollAmount` carry the values Compose falls back to when there is no AWT event, so
 * the device is the only thing this changes.
 */
internal fun Event.ScrollWheel.wheelEvent(): AwtMouseWheelEvent {
    // AppKit reports the device outright, so rather than reproducing AWT's rounding — which cannot tell
    // a precise delta that lands on a whole notch from a wheel click — the two rotations are made to
    // differ by half a notch exactly when the deltas are precise.
    val wheelRotation = -scrollingDeltaY.roundToInt()
    return AwtMouseWheelEvent(
        SyntheticEventSource,
        AwtMouseEvent.MOUSE_WHEEL,
        timestamp.toDuration().inWholeMilliseconds,
        0,
        locationInWindow.x.roundToInt(),
        locationInWindow.y.roundToInt(),
        0,
        0,
        0,
        false,
        AwtMouseWheelEvent.WHEEL_UNIT_SCROLL,
        1,
        wheelRotation,
        if (hasPreciseScrillingDeltas) wheelRotation + 0.5 else wheelRotation.toDouble(),
    )
}
