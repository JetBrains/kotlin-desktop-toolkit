package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.KeyModifiersSet
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Robot
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.Timeout
import java.util.Locale.getDefault
import java.util.concurrent.TimeUnit
import kotlin.test.Test

class KeyboardTest : KDTApplicationTestBase() {

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

    fun pressOneKeyAndAwaitEvent(keyCode: KeyCode, typed: String, key: String, keyWithModifiers: String, modifiers: Set<KeyCode>) {
        val modifiersSet = modifiers.toModifiersSet()

        try {
            for (modifier in modifiers) {
                ui {
                    robot.emulateKeyboardEvent(modifier, true)
                }
            }

            ui {
                robot.emulateKeyboardEvent(keyCode, true)
            }
            ui {
                robot.emulateKeyboardEvent(keyCode, false)
            }

            awaitEventOfType<Event.KeyDown> {
                it.keyCode == keyCode &&
                    it.typedCharacters == typed &&
                    it.key == key &&
                    it.keyWithModifiers == keyWithModifiers &&
                    it.modifiers == modifiersSet
            }
            awaitEventOfType<Event.KeyUp> {
                it.keyCode == keyCode &&
                    it.typedCharacters == typed &&
                    it.key == key &&
                    it.keyWithModifiers == keyWithModifiers &&
                    it.modifiers == modifiersSet
            }
        } finally {
            for (modifier in modifiers) {
                ui {
                    robot.emulateKeyboardEvent(modifier, false)
                }
            }
        }
    }

    companion object {
        lateinit var window: Window
        lateinit var robot: Robot

        @JvmStatic
        @BeforeAll
        fun createWindow() {
            robot = ui { Robot() }
            window = ui {
                Window.create(origin = LogicalPoint(100.0, 200.0), title = "Main Window")
            }
            ui {
                window.makeKeyAndOrderFront()
            }
            awaitEventOfType<Event.WindowFocusChange> { it.isKeyWindow }
            eventHandler = { event ->
                println("Event: $event")
                EventHandlerResult.Continue
            }
        }

        @JvmStatic
        @AfterAll
        fun destroyWindow() {
            ui {
                window.close()
            }
            eventHandler = null
            ui { robot.close() }
        }
    }

    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    @Test
    fun latinLettersNoModifiersTest() {
        ansiLetters.forEach { (keyCode, letter) ->
            pressOneKeyAndAwaitEvent(keyCode, typed = letter, key = letter, keyWithModifiers = letter, modifiers = emptySet())
        }
    }

    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    @Test
    fun latinLettersWithShiftTest() {
        val modifiers = setOf(KeyCode.Shift)
        ansiLetters.forEach { (keyCode, letter) ->
            val uppercaseLetter = letter.uppercase(getDefault())
            pressOneKeyAndAwaitEvent(
                keyCode,
                typed = uppercaseLetter,
                key = letter,
                keyWithModifiers = uppercaseLetter,
                modifiers = modifiers,
            )
        }
    }

    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    @Test
    fun latinLettersWithCommandTest() {
        val modifiers = setOf(KeyCode.Command)
        ansiLetters.forEach { (keyCode, letter) ->
            pressOneKeyAndAwaitEvent(keyCode, typed = letter, key = letter, keyWithModifiers = letter, modifiers = modifiers)
        }
    }

    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    @Test
    fun latinLettersWithControlTest() {
        val modifiers = setOf(KeyCode.Control)
        ansiLetters.forEach { (keyCode, letter) ->
            val typed: String = controlLayer[keyCode]!!
            pressOneKeyAndAwaitEvent(keyCode, typed = typed, key = letter, keyWithModifiers = typed, modifiers = modifiers)
        }
    }

    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    @Test
    fun latinLettersWithOptionTest() {
        val modifiers = setOf(KeyCode.Option)

        ansiLetters.forEach { (keyCode, letter) ->
            val keyData = optionLayer[keyCode]!!
            val optionLayerLetter = keyData.letter
            val typed = if (keyData.isDeadKey) {
                ""
            } else {
                optionLayerLetter
            }
            pressOneKeyAndAwaitEvent(keyCode, typed = typed, key = letter, keyWithModifiers = optionLayerLetter, modifiers = modifiers)
        }
    }

    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    @Test
    fun latinLettersWithOptionShiftTest() {
        val modifiers = setOf(KeyCode.Option, KeyCode.Shift)

        ansiLetters.forEach { (keyCode, letter) ->
            val keyData = optionLayerShifted[keyCode]!!
            val optionLayerLetter = keyData.letter
            val typed = if (keyData.isDeadKey) {
                ""
            } else {
                optionLayerLetter
            }
            pressOneKeyAndAwaitEvent(keyCode, typed = typed, key = letter, keyWithModifiers = optionLayerLetter, modifiers = modifiers)
        }
    }

    data class KeyData(
        val keyCode: KeyCode,
        val letter: String,
    )

    val ansiLetters = listOf(
        KeyData(KeyCode.ANSI_A, "a"),
        KeyData(KeyCode.ANSI_B, "b"),
        KeyData(KeyCode.ANSI_C, "c"),
        KeyData(KeyCode.ANSI_D, "d"),
        KeyData(KeyCode.ANSI_E, "e"),
        KeyData(KeyCode.ANSI_F, "f"),
        KeyData(KeyCode.ANSI_G, "g"),
        KeyData(KeyCode.ANSI_H, "h"),
        KeyData(KeyCode.ANSI_I, "i"),
        KeyData(KeyCode.ANSI_J, "j"),
        KeyData(KeyCode.ANSI_K, "k"),
        KeyData(KeyCode.ANSI_L, "l"),
        KeyData(KeyCode.ANSI_M, "m"),
        KeyData(KeyCode.ANSI_N, "n"),
        KeyData(KeyCode.ANSI_O, "o"),
        KeyData(KeyCode.ANSI_P, "p"),
        KeyData(KeyCode.ANSI_Q, "q"),
        KeyData(KeyCode.ANSI_R, "r"),
        KeyData(KeyCode.ANSI_S, "s"),
        KeyData(KeyCode.ANSI_T, "t"),
        KeyData(KeyCode.ANSI_U, "u"),
        KeyData(KeyCode.ANSI_V, "v"),
        KeyData(KeyCode.ANSI_W, "w"),
        KeyData(KeyCode.ANSI_X, "x"),
        KeyData(KeyCode.ANSI_Y, "y"),
        KeyData(KeyCode.ANSI_Z, "z"),
    )

    data class OptionLayerKeyData(
        val letter: String,
        val isDeadKey: Boolean,
    )

    val optionLayer = mapOf(
        Pair(KeyCode.ANSI_A, OptionLayerKeyData("å", isDeadKey = false)),
        Pair(KeyCode.ANSI_B, OptionLayerKeyData("∫", isDeadKey = false)),
        Pair(KeyCode.ANSI_C, OptionLayerKeyData("ç", isDeadKey = false)),
        Pair(KeyCode.ANSI_D, OptionLayerKeyData("∂", isDeadKey = false)),
        Pair(KeyCode.ANSI_E, OptionLayerKeyData("´", isDeadKey = true)),
        Pair(KeyCode.ANSI_F, OptionLayerKeyData("ƒ", isDeadKey = false)),
        Pair(KeyCode.ANSI_G, OptionLayerKeyData("©", isDeadKey = false)),
        Pair(KeyCode.ANSI_H, OptionLayerKeyData("˙", isDeadKey = false)),
        Pair(KeyCode.ANSI_I, OptionLayerKeyData("ˆ", isDeadKey = true)),
        Pair(KeyCode.ANSI_J, OptionLayerKeyData("∆", isDeadKey = false)),
        Pair(KeyCode.ANSI_K, OptionLayerKeyData("˚", isDeadKey = false)),
        Pair(KeyCode.ANSI_L, OptionLayerKeyData("¬", isDeadKey = false)),
        Pair(KeyCode.ANSI_M, OptionLayerKeyData("µ", isDeadKey = false)),
        Pair(KeyCode.ANSI_N, OptionLayerKeyData("˜", isDeadKey = true)),
        Pair(KeyCode.ANSI_O, OptionLayerKeyData("ø", isDeadKey = false)),
        Pair(KeyCode.ANSI_P, OptionLayerKeyData("π", isDeadKey = false)),
        Pair(KeyCode.ANSI_Q, OptionLayerKeyData("œ", isDeadKey = false)),
        Pair(KeyCode.ANSI_R, OptionLayerKeyData("®", isDeadKey = false)),
        Pair(KeyCode.ANSI_S, OptionLayerKeyData("ß", isDeadKey = false)),
        Pair(KeyCode.ANSI_T, OptionLayerKeyData("†", isDeadKey = false)),
        Pair(KeyCode.ANSI_U, OptionLayerKeyData("¨", isDeadKey = true)),
        Pair(KeyCode.ANSI_V, OptionLayerKeyData("√", isDeadKey = false)),
        Pair(KeyCode.ANSI_W, OptionLayerKeyData("∑", isDeadKey = false)),
        Pair(KeyCode.ANSI_X, OptionLayerKeyData("≈", isDeadKey = false)),
        Pair(KeyCode.ANSI_Y, OptionLayerKeyData("¥", isDeadKey = false)),
        Pair(KeyCode.ANSI_Z, OptionLayerKeyData("Ω", isDeadKey = false)),
    )

val optionLayerShifted = mapOf(
        Pair(KeyCode.ANSI_A, OptionLayerKeyData("Å", isDeadKey = false)),
        Pair(KeyCode.ANSI_B, OptionLayerKeyData("ı", isDeadKey = false)),
        Pair(KeyCode.ANSI_C, OptionLayerKeyData("Ç", isDeadKey = false)),
        Pair(KeyCode.ANSI_D, OptionLayerKeyData("Î", isDeadKey = false)),
        Pair(KeyCode.ANSI_E, OptionLayerKeyData("´", isDeadKey = false)),
        Pair(KeyCode.ANSI_F, OptionLayerKeyData("Ï", isDeadKey = false)),
        Pair(KeyCode.ANSI_G, OptionLayerKeyData("˝", isDeadKey = false)),
        Pair(KeyCode.ANSI_H, OptionLayerKeyData("Ó", isDeadKey = false)),
        Pair(KeyCode.ANSI_I, OptionLayerKeyData("ˆ", isDeadKey = false)),
        Pair(KeyCode.ANSI_J, OptionLayerKeyData("Ô", isDeadKey = false)),
        Pair(KeyCode.ANSI_K, OptionLayerKeyData("\uF8FF", isDeadKey = false)), // Apple logo
        Pair(KeyCode.ANSI_L, OptionLayerKeyData("Ò", isDeadKey = false)),
        Pair(KeyCode.ANSI_M, OptionLayerKeyData("Â", isDeadKey = false)),
        Pair(KeyCode.ANSI_N, OptionLayerKeyData("˜", isDeadKey = false)),
        Pair(KeyCode.ANSI_O, OptionLayerKeyData("Ø", isDeadKey = false)),
        Pair(KeyCode.ANSI_P, OptionLayerKeyData("∏", isDeadKey = false)),
        Pair(KeyCode.ANSI_Q, OptionLayerKeyData("Œ", isDeadKey = false)),
        Pair(KeyCode.ANSI_R, OptionLayerKeyData("‰", isDeadKey = false)),
        Pair(KeyCode.ANSI_S, OptionLayerKeyData("Í", isDeadKey = false)),
        Pair(KeyCode.ANSI_T, OptionLayerKeyData("ˇ", isDeadKey = false)),
        Pair(KeyCode.ANSI_U, OptionLayerKeyData("¨", isDeadKey = false)),
        Pair(KeyCode.ANSI_V, OptionLayerKeyData("◊", isDeadKey = false)),
        Pair(KeyCode.ANSI_W, OptionLayerKeyData("„", isDeadKey = false)),
        Pair(KeyCode.ANSI_X, OptionLayerKeyData("˛", isDeadKey = false)),
        Pair(KeyCode.ANSI_Y, OptionLayerKeyData("Á", isDeadKey = false)),
        Pair(KeyCode.ANSI_Z, OptionLayerKeyData("¸", isDeadKey = false)),
    )

    // https://chatgpt.com/share/695d443f-4260-8005-8992-3a13a00a575c
    // Historically Ctrl+A or other letters used for entering control characters
    val controlLayer = mapOf(
        Pair(KeyCode.ANSI_A, String(intArrayOf(1), 0, 1)), // Start of Heading
        Pair(KeyCode.ANSI_B, String(intArrayOf(2), 0, 1)), // Start of Text
        Pair(KeyCode.ANSI_C, String(intArrayOf(3), 0, 1)), // End of Text
        Pair(KeyCode.ANSI_D, String(intArrayOf(4), 0, 1)), // End of Transmission
        Pair(KeyCode.ANSI_E, String(intArrayOf(5), 0, 1)), // Enquiry
        Pair(KeyCode.ANSI_F, String(intArrayOf(6), 0, 1)), // Acknowledge
        Pair(KeyCode.ANSI_G, String(intArrayOf(7), 0, 1)), // Bell
        Pair(KeyCode.ANSI_H, String(intArrayOf(8), 0, 1)), // Backspace
        Pair(KeyCode.ANSI_I, String(intArrayOf(9), 0, 1)), // Horizontal Tab
        Pair(KeyCode.ANSI_J, String(intArrayOf(10), 0, 1)), // Line Feed
        Pair(KeyCode.ANSI_K, String(intArrayOf(11), 0, 1)), // Vertical Tab
        Pair(KeyCode.ANSI_L, String(intArrayOf(12), 0, 1)), // Form Feed
        Pair(KeyCode.ANSI_M, String(intArrayOf(13), 0, 1)), // Carriage Return
        Pair(KeyCode.ANSI_N, String(intArrayOf(14), 0, 1)), // Shift Out
        Pair(KeyCode.ANSI_O, String(intArrayOf(15), 0, 1)), // Shift In
        Pair(KeyCode.ANSI_P, String(intArrayOf(16), 0, 1)), // Data Link Escape
        Pair(KeyCode.ANSI_Q, String(intArrayOf(17), 0, 1)), // Device Control 1
        Pair(KeyCode.ANSI_R, String(intArrayOf(18), 0, 1)), // Device Control 2
        Pair(KeyCode.ANSI_S, String(intArrayOf(19), 0, 1)), // Device Control 3
        Pair(KeyCode.ANSI_T, String(intArrayOf(20), 0, 1)), // Device Control 4
        Pair(KeyCode.ANSI_U, String(intArrayOf(21), 0, 1)), // Negative Acknowledge
        Pair(KeyCode.ANSI_V, String(intArrayOf(22), 0, 1)), // Synchronous Idle
        Pair(KeyCode.ANSI_W, String(intArrayOf(23), 0, 1)), // End of Transmission Block
        Pair(KeyCode.ANSI_X, String(intArrayOf(24), 0, 1)), // Cancel
        Pair(KeyCode.ANSI_Y, String(intArrayOf(25), 0, 1)), // End of Medium
        Pair(KeyCode.ANSI_Z, String(intArrayOf(26), 0, 1)), // Substitute
    )
}
