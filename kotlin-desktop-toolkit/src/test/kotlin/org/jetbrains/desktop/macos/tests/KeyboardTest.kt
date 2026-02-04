package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.KeyboardType
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.Robot
import org.jetbrains.desktop.macos.SpecialKey
import org.jetbrains.desktop.macos.TextInputSource
import org.jetbrains.desktop.macos.Window
import org.jetbrains.desktop.macos.tests.KeyboardHelpers.assertKeyDown
import org.jetbrains.desktop.macos.tests.KeyboardHelpers.assertKeyUp
import org.jetbrains.desktop.macos.tests.KeyboardHelpers.withModifiersPressed
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.Timeout
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.util.concurrent.TimeUnit
import kotlin.collections.emptySet
import kotlin.test.Ignore
import kotlin.test.Test
import kotlin.test.assertEquals

@EnabledOnOs(OS.MAC)
class KeyboardTest : KDTApplicationTestBase() {
    companion object {
        lateinit var window: Window
        lateinit var robot: Robot
        lateinit var inputSourceBeforeTest: String

        @JvmStatic
        @BeforeAll
        @Timeout(value = 15, unit = TimeUnit.SECONDS)
        fun init() {
            Logger.info { "KeyboardTest INIT STARTED" }
            inputSourceBeforeTest = ui { TextInputSource.current()!! }
            robot = ui { Robot() }
            window = createWindowAndEnsureItsFocused(name = "KeyboardTest Window")
            Logger.info { "KeyboardTest INIT FINISHED" }
        }

        @JvmStatic
        @AfterAll
        @Timeout(value = 15, unit = TimeUnit.SECONDS)
        fun destroy() {
            Logger.info { "KeyboardTest DESTROY STARTED" }
            ui { robot.close() }
            ui {
                window.close()
            }
            ui { TextInputSource.select(inputSourceBeforeTest) }
            Logger.info { "KeyboardTest DESTROY FINISHED" }
        }
    }

    @Test
    @Timeout(value = 10, unit = TimeUnit.SECONDS)
    fun ansiButtonsNoModifiersTest() {
        withInputSourceSelected("com.apple.keylayout.ABC") {
            ansiButtons.forEach { (keyCode, key) ->
                val modifiers = emptySet<KeyCode>()
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = key,
                    key = key,
                    keyWithModifiers = key,
                    modifiers = modifiers,
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = key,
                    key = key,
                    keyWithModifiers = key,
                    modifiers = modifiers,
                )
            }
        }
    }

    @Test
    @Timeout(value = 10, unit = TimeUnit.SECONDS)
    fun germanLayoutNoModifiersTest() {
        withInputSourceSelected("com.apple.keylayout.German") {
            // In German layout, Y and Z physical keys are swapped
            val germanLetters = listOf(
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
                KeyData(KeyCode.ANSI_Y, "z"), // Y key produces z in German layout
                KeyData(KeyCode.ANSI_Z, "y"), // Z key produces y in German layout
            )

            germanLetters.forEach { (keyCode, expectedLetter) ->
                ui { robot.emulateKeyboardEvent(keyCode, true) }
                ui { robot.emulateKeyboardEvent(keyCode, false) }

                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = expectedLetter,
                    key = expectedLetter,
                    keyWithModifiers = expectedLetter,
                    modifiers = emptySet(),
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = expectedLetter,
                    key = expectedLetter,
                    keyWithModifiers = expectedLetter,
                    modifiers = emptySet(),
                )
            }
        }
    }

    @Test
    @Timeout(value = 10, unit = TimeUnit.SECONDS)
    fun cyrillicLayoutNoModifiersTest() {
        withInputSourceSelected("com.apple.keylayout.Russian") {
            // Physical ANSI keys mapped to Cyrillic letters in Russian layout
            val cyrillicLetters = listOf(
                KeyData(KeyCode.ANSI_A, "ф"),
                KeyData(KeyCode.ANSI_B, "и"),
                KeyData(KeyCode.ANSI_C, "с"),
                KeyData(KeyCode.ANSI_D, "в"),
                KeyData(KeyCode.ANSI_E, "у"),
                KeyData(KeyCode.ANSI_F, "а"),
                KeyData(KeyCode.ANSI_G, "п"),
                KeyData(KeyCode.ANSI_H, "р"),
                KeyData(KeyCode.ANSI_I, "ш"),
                KeyData(KeyCode.ANSI_J, "о"),
                KeyData(KeyCode.ANSI_K, "л"),
                KeyData(KeyCode.ANSI_L, "д"),
                KeyData(KeyCode.ANSI_M, "ь"),
                KeyData(KeyCode.ANSI_N, "т"),
                KeyData(KeyCode.ANSI_O, "щ"),
                KeyData(KeyCode.ANSI_P, "з"),
                KeyData(KeyCode.ANSI_Q, "й"),
                KeyData(KeyCode.ANSI_R, "к"),
                KeyData(KeyCode.ANSI_S, "ы"),
                KeyData(KeyCode.ANSI_T, "е"),
                KeyData(KeyCode.ANSI_U, "г"),
                KeyData(KeyCode.ANSI_V, "м"),
                KeyData(KeyCode.ANSI_W, "ц"),
                KeyData(KeyCode.ANSI_X, "ч"),
                KeyData(KeyCode.ANSI_Y, "н"),
                KeyData(KeyCode.ANSI_Z, "я"),
            )

            cyrillicLetters.forEach { (keyCode, expectedLetter) ->
                ui { robot.emulateKeyboardEvent(keyCode, true) }
                ui { robot.emulateKeyboardEvent(keyCode, false) }

                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = expectedLetter,
                    key = expectedLetter,
                    keyWithModifiers = expectedLetter,
                    modifiers = emptySet(),
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = expectedLetter,
                    key = expectedLetter,
                    keyWithModifiers = expectedLetter,
                    modifiers = emptySet(),
                )
            }
        }
    }

    @Test
    @Timeout(value = 10, unit = TimeUnit.SECONDS)
    fun latinButtonsWithShiftTest() {
        withInputSourceSelected("com.apple.keylayout.ABC") {
            val modifiers = setOf(KeyCode.Shift)
            ansiButtons.forEach { (keyCode, key) ->
                val shiftedKey = shiftedLayer[keyCode]!!
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = shiftedKey,
                    key = key,
                    keyWithModifiers = shiftedKey,
                    modifiers = modifiers,
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = shiftedKey,
                    key = key,
                    keyWithModifiers = shiftedKey,
                    modifiers = modifiers,
                )
            }
        }
    }

    @Test
    @Timeout(value = 20, unit = TimeUnit.SECONDS)
    fun ansiButtonsWithCommandTest() {
        withInputSourceSelected("com.apple.keylayout.ABC") {
            val modifiers = setOf(KeyCode.Command)
            ansiButtons.forEach { (keyCode, key) ->
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = key,
                    key = key,
                    keyWithModifiers = key,
                    modifiers = modifiers,
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = key,
                    key = key,
                    keyWithModifiers = key,
                    modifiers = modifiers,
                )
            }
        }
    }

    @Test
    @Timeout(value = 10, unit = TimeUnit.SECONDS)
    fun latinButtonsWithCommandShiftTest() {
        withInputSourceSelected("com.apple.keylayout.ABC") {
            val modifiers = setOf(KeyCode.Command, KeyCode.Shift)
            for ((keyCode, key) in ansiButtons) {
                if (keyCode == KeyCode.ANSI_Q) {
                    continue // Close all apps and quit
                }
                if (keyCode == KeyCode.ANSI_3 || keyCode == KeyCode.ANSI_4 || keyCode == KeyCode.ANSI_5) {
                    continue // Screenshot shortcuts
                }
                if (keyCode == KeyCode.ANSI_Slash) {
                    continue // Help menu shortcut
                }
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = key,
                    key = key,
                    keyWithModifiers = key,
                    modifiers = modifiers,
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = key,
                    key = key,
                    keyWithModifiers = key,
                    modifiers = modifiers,
                )
            }
        }
    }

    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    fun latinLettersWithCommandControlTest() {
        withInputSourceSelected("com.apple.keylayout.ABC") {
            val modifiers = setOf(KeyCode.Command, KeyCode.Control)
            for ((keyCode, key) in ansiLetters) {
                if (keyCode == KeyCode.ANSI_D) {
                    continue // Reserved by Dictionary.app
                }
                if (keyCode == KeyCode.ANSI_Q) {
                    continue // Quit session
                }
                val keyWithModifiers: String = controlLayer[keyCode]!!
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = keyWithModifiers,
                    key = key,
                    keyWithModifiers = keyWithModifiers,
                    modifiers = modifiers,
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = keyWithModifiers,
                    key = key,
                    keyWithModifiers = keyWithModifiers,
                    modifiers = modifiers,
                )
            }
        }
    }

    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    fun latinLettersWithControlTest() {
        withInputSourceSelected("com.apple.keylayout.ABC") {
            val modifiers = setOf(KeyCode.Control)
            ansiLetters.forEach { (keyCode, key) ->
                val keyWithModifiers: String = controlLayer[keyCode]!!
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = keyWithModifiers,
                    key = key,
                    keyWithModifiers = keyWithModifiers,
                    modifiers = modifiers,
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = keyWithModifiers,
                    key = key,
                    keyWithModifiers = keyWithModifiers,
                    modifiers = modifiers,
                )
            }
        }
    }

    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    fun latinLettersWithControlShiftTest() {
        withInputSourceSelected("com.apple.keylayout.ABC") {
            val modifiers = setOf(KeyCode.Control, KeyCode.Shift)
            ansiLetters.forEach { (keyCode, key) ->
                val keyWithModifiers: String = controlLayer[keyCode]!!
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = keyWithModifiers,
                    key = key,
                    keyWithModifiers = keyWithModifiers,
                    modifiers = modifiers,
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = keyWithModifiers,
                    key = key,
                    keyWithModifiers = keyWithModifiers,
                    modifiers = modifiers,
                )
            }
        }
    }

    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    fun latinLettersWithOptionTest() {
        withInputSourceSelected("com.apple.keylayout.ABC") {
            val modifiers = setOf(KeyCode.Option)

            ansiLetters.forEach { (keyCode, key) ->
                val keyData = optionLayer[keyCode]!!
                val optionLayerLetter = keyData.letter
                val typed = if (keyData.isDeadKey) {
                    ""
                } else {
                    optionLayerLetter
                }
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = typed,
                    key = key,
                    keyWithModifiers = optionLayerLetter,
                    modifiers = modifiers,
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = typed,
                    key = key,
                    keyWithModifiers = optionLayerLetter,
                    modifiers = modifiers,
                )
            }
        }
    }

    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    fun latinLettersWithOptionShiftTest() {
        withInputSourceSelected("com.apple.keylayout.ABC") {
            val modifiers = setOf(KeyCode.Option, KeyCode.Shift)

            ansiLetters.forEach { (keyCode, key) ->
                val keyData = optionLayerShifted[keyCode]!!
                val optionLayerLetter = keyData.letter
                val typed = if (keyData.isDeadKey) {
                    ""
                } else {
                    optionLayerLetter
                }
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = typed,
                    key = key,
                    keyWithModifiers = optionLayerLetter,
                    modifiers = modifiers,
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = typed,
                    key = key,
                    keyWithModifiers = optionLayerLetter,
                    modifiers = modifiers,
                )
            }
        }
    }

    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    fun latinLettersWithOptionCommandTest() {
        withInputSourceSelected("com.apple.keylayout.ABC") {
            val modifiers = setOf(KeyCode.Command, KeyCode.Option)
            for ((keyCode, key) in ansiLetters) {
                if (keyCode == KeyCode.ANSI_D) {
                    continue // Is not reported on CI
                }
                if (keyCode == KeyCode.ANSI_N) {
                    continue // Global shortcut used by Arc browser
                }
                val keyData = optionLayer[keyCode]!!
                val optionLayerLetter = keyData.letter
                val keyWithModifiers = if (keyData.isDeadKey) {
                    keyData.deadKeyReplacement!!
                } else {
                    optionLayerLetter
                }
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = keyWithModifiers,
                    key = key,
                    keyWithModifiers = keyWithModifiers,
                    modifiers = modifiers,
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = keyWithModifiers,
                    key = key,
                    keyWithModifiers = keyWithModifiers,
                    modifiers = modifiers,
                )
            }
        }
    }

    // Same behavior as in Ctrl+Letter
    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    fun latinLettersWithOptionControlTest() {
        withInputSourceSelected("com.apple.keylayout.ABC") {
            val modifiers = setOf(KeyCode.Control, KeyCode.Option)
            ansiLetters.forEach { (keyCode, key) ->
                val keyWithModifiers: String = controlLayer[keyCode]!!
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = keyWithModifiers,
                    key = key,
                    keyWithModifiers = keyWithModifiers,
                    modifiers = modifiers,
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = keyWithModifiers,
                    key = key,
                    keyWithModifiers = keyWithModifiers,
                    modifiers = modifiers,
                )
            }
        }
    }

    @Ignore("Sometimes it triggers function and NumericPad Modifiers")
    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    fun `all keys ansi keyboard`() {
        val specialKeys = mapOf<KeyCode, SpecialKey>()
        withInputSourceSelected("com.apple.keylayout.ABC") {
            val modifiers = emptySet<KeyCode>()
            ui { robot.setKeyboardType(KeyboardType.Ansi) }
            allKeys.forEach { keyCode ->
                withModifiersPressed(modifiers = modifiers) {
                    ui { robot.emulateKeyboardEvent(keyCode, true) }
                    ui { robot.emulateKeyboardEvent(keyCode, false) }
                }
                val downEvent = awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode }
                val upEvent = awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode }
                assertEquals(downEvent.key, upEvent.key)
                if (downEvent.charactersIgnoringModifiers != downEvent.key) {
                    println("Key $keyCode: ${downEvent.charactersIgnoringModifiers} ... ${downEvent.key}")
                }
            }
        }
    }

    data class KeyData(
        val keyCode: KeyCode,
        val key: String,
        val isLetter: Boolean = false,
    )

    val allKeys = listOf(
        KeyCode.ANSI_A,
        KeyCode.ANSI_S,
        KeyCode.ANSI_D,
        KeyCode.ANSI_F,
        KeyCode.ANSI_H,
        KeyCode.ANSI_G,
        KeyCode.ANSI_Z,
        KeyCode.ANSI_X,
        KeyCode.ANSI_C,
        KeyCode.ANSI_V,
        KeyCode.ANSI_B,
        KeyCode.ANSI_Q,
        KeyCode.ANSI_W,
        KeyCode.ANSI_E,
        KeyCode.ANSI_R,
        KeyCode.ANSI_Y,
        KeyCode.ANSI_T,
        KeyCode.ANSI_1,
        KeyCode.ANSI_2,
        KeyCode.ANSI_3,
        KeyCode.ANSI_4,
        KeyCode.ANSI_6,
        KeyCode.ANSI_5,
        KeyCode.ANSI_Equal,
        KeyCode.ANSI_9,
        KeyCode.ANSI_7,
        KeyCode.ANSI_Minus,
        KeyCode.ANSI_8,
        KeyCode.ANSI_0,
        KeyCode.ANSI_RightBracket,
        KeyCode.ANSI_O,
        KeyCode.ANSI_U,
        KeyCode.ANSI_LeftBracket,
        KeyCode.ANSI_I,
        KeyCode.ANSI_P,
        KeyCode.ANSI_L,
        KeyCode.ANSI_J,
        KeyCode.ANSI_Quote,
        KeyCode.ANSI_K,
        KeyCode.ANSI_Semicolon,
        KeyCode.ANSI_Backslash,
        KeyCode.ANSI_Comma,
        KeyCode.ANSI_Slash,
        KeyCode.ANSI_N,
        KeyCode.ANSI_M,
        KeyCode.ANSI_Period,
        KeyCode.ANSI_Grave,
        KeyCode.ANSI_KeypadDecimal,
        KeyCode.ANSI_KeypadMultiply,
        KeyCode.ANSI_KeypadPlus,
        KeyCode.ANSI_KeypadClear,
        KeyCode.ANSI_KeypadDivide,
        KeyCode.ANSI_KeypadEnter,
        KeyCode.ANSI_KeypadMinus,
        KeyCode.ANSI_KeypadEquals,
        KeyCode.ANSI_Keypad0,
        KeyCode.ANSI_Keypad1,
        KeyCode.ANSI_Keypad2,
        KeyCode.ANSI_Keypad3,
        KeyCode.ANSI_Keypad4,
        KeyCode.ANSI_Keypad5,
        KeyCode.ANSI_Keypad6,
        KeyCode.ANSI_Keypad7,
        KeyCode.ANSI_Keypad8,
        KeyCode.ANSI_Keypad9,
        KeyCode.Return,
        KeyCode.Tab,
        KeyCode.Space,
        KeyCode.Delete,
        KeyCode.Escape,

        // Modifiers do not produce Key Down/Up events
        // KeyCode.Command,
        // KeyCode.Shift,
        // KeyCode.CapsLock,
        // KeyCode.Option,
        // KeyCode.Control,
        // KeyCode.RightCommand,
        // KeyCode.RightShift,
        // KeyCode.RightOption,
        // KeyCode.RightControl,
        // KeyCode.Function,

        KeyCode.F17,
        // Handled by OS
        // KeyCode.VolumeUp,
        // KeyCode.VolumeDown,
        // KeyCode.Mute,
        KeyCode.F18,
        KeyCode.F19,
        KeyCode.F20,
        KeyCode.F5,
        KeyCode.F6,
        KeyCode.F7,
        KeyCode.F3,
        KeyCode.F8,
        KeyCode.F9,
        // Some global shortcut
        // KeyCode.F11,
        KeyCode.F13,
        KeyCode.F16,
        // KeyCode.F14,
        KeyCode.F10,
        KeyCode.ContextualMenu,
        // KeyCode.F12,
        // KeyCode.F15,

        // Handled by OS
        // KeyCode.Help,
        KeyCode.Home,
        KeyCode.PageUp,
        KeyCode.ForwardDelete,
        KeyCode.F4,
        KeyCode.End,
        KeyCode.F2,
        KeyCode.PageDown,
        KeyCode.F1,
        KeyCode.LeftArrow,
        KeyCode.RightArrow,
        KeyCode.DownArrow,
        KeyCode.UpArrow,
    )

    val isoKeys = listOf(KeyCode.ISO_Section)
    val jisKeys = listOf(
        KeyCode.JIS_Yen,
        KeyCode.JIS_Underscore,
        KeyCode.JIS_KeypadComma,

        KeyCode.JIS_Eisu,
        KeyCode.JIS_Kana,
    )

    val ansiButtons = listOf(
        // Letters
        KeyData(KeyCode.ANSI_A, "a", isLetter = true),
        KeyData(KeyCode.ANSI_B, "b", isLetter = true),
        KeyData(KeyCode.ANSI_C, "c", isLetter = true),
        KeyData(KeyCode.ANSI_D, "d", isLetter = true),
        KeyData(KeyCode.ANSI_E, "e", isLetter = true),
        KeyData(KeyCode.ANSI_F, "f", isLetter = true),
        KeyData(KeyCode.ANSI_G, "g", isLetter = true),
        KeyData(KeyCode.ANSI_H, "h", isLetter = true),
        KeyData(KeyCode.ANSI_I, "i", isLetter = true),
        KeyData(KeyCode.ANSI_J, "j", isLetter = true),
        KeyData(KeyCode.ANSI_K, "k", isLetter = true),
        KeyData(KeyCode.ANSI_L, "l", isLetter = true),
        KeyData(KeyCode.ANSI_M, "m", isLetter = true),
        KeyData(KeyCode.ANSI_N, "n", isLetter = true),
        KeyData(KeyCode.ANSI_O, "o", isLetter = true),
        KeyData(KeyCode.ANSI_P, "p", isLetter = true),
        KeyData(KeyCode.ANSI_Q, "q", isLetter = true),
        KeyData(KeyCode.ANSI_R, "r", isLetter = true),
        KeyData(KeyCode.ANSI_S, "s", isLetter = true),
        KeyData(KeyCode.ANSI_T, "t", isLetter = true),
        KeyData(KeyCode.ANSI_U, "u", isLetter = true),
        KeyData(KeyCode.ANSI_V, "v", isLetter = true),
        KeyData(KeyCode.ANSI_W, "w", isLetter = true),
        KeyData(KeyCode.ANSI_X, "x", isLetter = true),
        KeyData(KeyCode.ANSI_Y, "y", isLetter = true),
        KeyData(KeyCode.ANSI_Z, "z", isLetter = true),
        // Digits
        KeyData(KeyCode.ANSI_1, "1"),
        KeyData(KeyCode.ANSI_2, "2"),
        KeyData(KeyCode.ANSI_3, "3"),
        KeyData(KeyCode.ANSI_4, "4"),
        KeyData(KeyCode.ANSI_5, "5"),
        KeyData(KeyCode.ANSI_6, "6"),
        KeyData(KeyCode.ANSI_7, "7"),
        KeyData(KeyCode.ANSI_8, "8"),
        KeyData(KeyCode.ANSI_9, "9"),
        KeyData(KeyCode.ANSI_0, "0"),
        // Symbols
        KeyData(KeyCode.ANSI_Minus, "-"),
        KeyData(KeyCode.ANSI_Equal, "="),
        KeyData(KeyCode.ANSI_LeftBracket, "["),
        KeyData(KeyCode.ANSI_RightBracket, "]"),
        KeyData(KeyCode.ANSI_Backslash, "\\"),
        KeyData(KeyCode.ANSI_Semicolon, ";"),
        KeyData(KeyCode.ANSI_Quote, "'"),
        KeyData(KeyCode.ANSI_Comma, ","),
        KeyData(KeyCode.ANSI_Period, "."),
        KeyData(KeyCode.ANSI_Slash, "/"),
        KeyData(KeyCode.ANSI_Grave, "`"),
    )

    val ansiLetters = ansiButtons.filter { it.isLetter }

    val shiftedLayer = mapOf(
        // Letters
        Pair(KeyCode.ANSI_A, "A"),
        Pair(KeyCode.ANSI_B, "B"),
        Pair(KeyCode.ANSI_C, "C"),
        Pair(KeyCode.ANSI_D, "D"),
        Pair(KeyCode.ANSI_E, "E"),
        Pair(KeyCode.ANSI_F, "F"),
        Pair(KeyCode.ANSI_G, "G"),
        Pair(KeyCode.ANSI_H, "H"),
        Pair(KeyCode.ANSI_I, "I"),
        Pair(KeyCode.ANSI_J, "J"),
        Pair(KeyCode.ANSI_K, "K"),
        Pair(KeyCode.ANSI_L, "L"),
        Pair(KeyCode.ANSI_M, "M"),
        Pair(KeyCode.ANSI_N, "N"),
        Pair(KeyCode.ANSI_O, "O"),
        Pair(KeyCode.ANSI_P, "P"),
        Pair(KeyCode.ANSI_Q, "Q"),
        Pair(KeyCode.ANSI_R, "R"),
        Pair(KeyCode.ANSI_S, "S"),
        Pair(KeyCode.ANSI_T, "T"),
        Pair(KeyCode.ANSI_U, "U"),
        Pair(KeyCode.ANSI_V, "V"),
        Pair(KeyCode.ANSI_W, "W"),
        Pair(KeyCode.ANSI_X, "X"),
        Pair(KeyCode.ANSI_Y, "Y"),
        Pair(KeyCode.ANSI_Z, "Z"),
        // Digits
        Pair(KeyCode.ANSI_1, "!"),
        Pair(KeyCode.ANSI_2, "@"),
        Pair(KeyCode.ANSI_3, "#"),
        Pair(KeyCode.ANSI_4, "$"),
        Pair(KeyCode.ANSI_5, "%"),
        Pair(KeyCode.ANSI_6, "^"),
        Pair(KeyCode.ANSI_7, "&"),
        Pair(KeyCode.ANSI_8, "*"),
        Pair(KeyCode.ANSI_9, "("),
        Pair(KeyCode.ANSI_0, ")"),
        // Symbols
        Pair(KeyCode.ANSI_Minus, "_"),
        Pair(KeyCode.ANSI_Equal, "+"),
        Pair(KeyCode.ANSI_LeftBracket, "{"),
        Pair(KeyCode.ANSI_RightBracket, "}"),
        Pair(KeyCode.ANSI_Backslash, "|"),
        Pair(KeyCode.ANSI_Semicolon, ":"),
        Pair(KeyCode.ANSI_Quote, "\""),
        Pair(KeyCode.ANSI_Comma, "<"),
        Pair(KeyCode.ANSI_Period, ">"),
        Pair(KeyCode.ANSI_Slash, "?"),
        Pair(KeyCode.ANSI_Grave, "~"),
    )

    data class OptionLayerKeyData(
        val letter: String,
        val isDeadKey: Boolean,
        val deadKeyReplacement: String? = null,
    )

    val optionLayer = mapOf(
        Pair(KeyCode.ANSI_A, OptionLayerKeyData("å", isDeadKey = false)),
        Pair(KeyCode.ANSI_B, OptionLayerKeyData("∫", isDeadKey = false)),
        Pair(KeyCode.ANSI_C, OptionLayerKeyData("ç", isDeadKey = false)),
        Pair(KeyCode.ANSI_D, OptionLayerKeyData("∂", isDeadKey = false)),
        Pair(KeyCode.ANSI_E, OptionLayerKeyData("´", isDeadKey = true, deadKeyReplacement = "´")),
        Pair(KeyCode.ANSI_F, OptionLayerKeyData("ƒ", isDeadKey = false)),
        Pair(KeyCode.ANSI_G, OptionLayerKeyData("©", isDeadKey = false)),
        Pair(KeyCode.ANSI_H, OptionLayerKeyData("˙", isDeadKey = false)),
        Pair(KeyCode.ANSI_I, OptionLayerKeyData("ˆ", isDeadKey = true, deadKeyReplacement = "^")),
        Pair(KeyCode.ANSI_J, OptionLayerKeyData("∆", isDeadKey = false)),
        Pair(KeyCode.ANSI_K, OptionLayerKeyData("˚", isDeadKey = false)),
        Pair(KeyCode.ANSI_L, OptionLayerKeyData("¬", isDeadKey = false)),
        Pair(KeyCode.ANSI_M, OptionLayerKeyData("µ", isDeadKey = false)),
        Pair(KeyCode.ANSI_N, OptionLayerKeyData("˜", isDeadKey = true, deadKeyReplacement = "~")),
        Pair(KeyCode.ANSI_O, OptionLayerKeyData("ø", isDeadKey = false)),
        Pair(KeyCode.ANSI_P, OptionLayerKeyData("π", isDeadKey = false)),
        Pair(KeyCode.ANSI_Q, OptionLayerKeyData("œ", isDeadKey = false)),
        Pair(KeyCode.ANSI_R, OptionLayerKeyData("®", isDeadKey = false)),
        Pair(KeyCode.ANSI_S, OptionLayerKeyData("ß", isDeadKey = false)),
        Pair(KeyCode.ANSI_T, OptionLayerKeyData("†", isDeadKey = false)),
        Pair(KeyCode.ANSI_U, OptionLayerKeyData("¨", isDeadKey = true, deadKeyReplacement = "¨")),
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
