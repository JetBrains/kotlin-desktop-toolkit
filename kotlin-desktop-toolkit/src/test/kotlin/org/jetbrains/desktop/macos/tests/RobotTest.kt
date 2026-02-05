package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.Robot
import org.jetbrains.desktop.macos.TextInputSource
import org.jetbrains.desktop.macos.Window
import org.jetbrains.desktop.macos.tests.KeyboardHelpers.assertKeyDown
import org.jetbrains.desktop.macos.tests.KeyboardHelpers.assertKeyUp
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

@EnabledOnOs(OS.MAC)
class RobotTest : KDTApplicationTestBase() {

    companion object {
        lateinit var window: Window
        lateinit var robot: Robot
        lateinit var inputSourceBeforeTest: String

        @BeforeAll
        @JvmStatic
        fun init() {
            Logger.info { "RobotTest INIT STARTED" }
            inputSourceBeforeTest = ui { TextInputSource.current()!! }
            robot = ui { Robot() }
            window = createWindowAndEnsureItsFocused(name = "RobotTest Window")
            Logger.info { "RobotTest INIT FINISHED" }
        }

        @AfterAll
        @JvmStatic
        fun destroy() {
            Logger.info { "RobotTest DESTROY STARTED" }
            ui { robot.close() }
            ui {
                window.close()
            }
            ui { TextInputSource.select(inputSourceBeforeTest) }
            Logger.info { "RobotTest DESTROY FINISHED" }
        }
    }

    @Test
    fun `robot waits until the event is delivered to os`() {
        repeat(100) {
            val hadCapitalA = java.util.concurrent.atomic.AtomicBoolean(false)
            withEventHandler(handler = {
                if (it is Event.KeyDown && it.keyCode == KeyCode.ANSI_A && it.characters.text == "A") {
                    hadCapitalA.set(true)
                }
                EventHandlerResult.Continue
            }) {
                ui { robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = true) }

                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_A, isKeyDown = true) }

                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_A, isKeyDown = false) }

                ui { robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = false) }

                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_X, isKeyDown = true) }
                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_X, isKeyDown = false) }
                awaitEventOfType<Event.KeyUp> { it.keyCode == KeyCode.ANSI_X }
            }
            assertNotNull(hadCapitalA.get())
        }
    }

    @Test
    fun `modifiers are correctly stacked`() {
        repeat(100) {
            ui { robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = true) }
            ui { robot.emulateKeyboardEvent(KeyCode.Command, isKeyDown = true) }
            ui { robot.emulateKeyboardEvent(KeyCode.Option, isKeyDown = true) }
            awaitEventOfType<Event.ModifiersChanged> {
                it.keyCode == KeyCode.Option &&
                    it.modifiers.shift &&
                    it.modifiers.command &&
                    it.modifiers.option
            }

            ui { robot.emulateKeyboardEvent(KeyCode.Option, isKeyDown = false) }
            ui { robot.emulateKeyboardEvent(KeyCode.Command, isKeyDown = false) }
            ui { robot.emulateKeyboardEvent(KeyCode.Shift, isKeyDown = false) }
            awaitEventOfType<Event.ModifiersChanged> {
                it.keyCode == KeyCode.Shift &&
                    !it.modifiers.shift &&
                    !it.modifiers.command &&
                    !it.modifiers.option
            }
        }
    }

    @Test
    fun `modifiers command option`() {
        repeat(100) {
            ui { robot.emulateKeyboardEvent(KeyCode.Command, isKeyDown = true) }
            ui { robot.emulateKeyboardEvent(KeyCode.Option, isKeyDown = true) }
            awaitEventOfType<Event.ModifiersChanged> {
                it.modifiers.command &&
                    it.modifiers.option
            }

            ui { robot.emulateKeyboardEvent(KeyCode.Command, isKeyDown = false) }
            ui { robot.emulateKeyboardEvent(KeyCode.Option, isKeyDown = false) }
            awaitEventOfType<Event.ModifiersChanged> {
                !it.modifiers.shift &&
                    !it.modifiers.command &&
                    !it.modifiers.option
            }
        }
    }

    @Test
    fun `input source test`() {
        val inputSource = ui { TextInputSource.current() }
        assert(inputSource?.startsWith("com.apple.keylayout") == true) {
            "$inputSource should start with 'com.apple.keylayout'"
        }
    }

    @Test
    fun `list input sources test`() {
        val inputSources = ui { TextInputSource.list() }
        Logger.info { "Input sources: $inputSources" }
        assert(inputSources.isNotEmpty()) { "Input sources list should not be empty" }
        assert(inputSources.any { it.startsWith("com.apple.keylayout.") }) {
            "Should contain at least one keyboard layout"
        }
    }

    @Test
    fun `check that all required input sources are installed`() {
        val layouts = listOf(
            "com.apple.keylayout.ABC",
            "com.apple.keylayout.Russian",
            "com.apple.keylayout.Swedish-Pro",
            "com.apple.keylayout.USInternational-PC",
            "com.apple.keylayout.German",
            "com.apple.keylayout.Serbian-Latin",
            "com.apple.keylayout.Serbian",
            "com.apple.keylayout.Dvorak",
            "com.apple.keylayout.DVORAK-QWERTYCMD",
            "com.apple.inputmethod.Kotoeri.RomajiTyping.Japanese",
            "com.apple.inputmethod.TCIM.Pinyin",
            "com.apple.inputmethod.Korean.2SetKorean",
        )
        val inputSources = ui { TextInputSource.list(includeAll = true) }
        layouts.forEach { assertContains(inputSources, it) }
    }

    @Test
    fun `check that all required input sources might be enabled`() {
        val inputSourceNames = listOf(
            "com.apple.keylayout.ABC",
            "com.apple.keylayout.Russian",
            "com.apple.keylayout.Swedish-Pro",
            "com.apple.keylayout.USInternational-PC",
            "com.apple.keylayout.German",
            "com.apple.keylayout.Serbian-Latin",
            "com.apple.keylayout.Serbian",
            "com.apple.keylayout.Dvorak",
            "com.apple.keylayout.DVORAK-QWERTYCMD",
            "com.apple.inputmethod.Kotoeri.RomajiTyping.Japanese",
//            "com.apple.inputmethod.TCIM.Pinyin", // Fails on CI
//            "com.apple.inputmethod.Korean.2SetKorean",
        )
        inputSourceNames.forEach { inputSourceName ->
            withInputSourceEnabled(inputSourceName) {
                val inputSources = ui { TextInputSource.list(includeAll = false) }
                assertContains(inputSources, inputSourceName)
            }
        }
    }

    @Test
    fun `current input source is in the list of input sources`() {
        val currentLayout = ui { TextInputSource.current() }
        val inputSources = ui { TextInputSource.list() }
        assert(currentLayout != null) { "Current keyboard layout should not be null" }
        assertContains(inputSources, currentLayout, "Current keyboard layout should be in the list of input sources")
    }

    @Test
    fun `choose input source and restore`() {
        val originalLayout = ui { TextInputSource.current() }
        assertNotNull(originalLayout)

        val inputSources = ui { TextInputSource.list() }
        val anotherLayout = inputSources.firstOrNull { it != originalLayout && it.startsWith("com.apple.keylayout.") }

        if (anotherLayout != null) {
            val switched = ui { TextInputSource.select(anotherLayout) }
            assert(switched) { "Failed to switch to $anotherLayout" }

            val currentAfterSwitch = ui { TextInputSource.current() }
            assertEquals(anotherLayout, currentAfterSwitch)

            // Restore original layout
            val restored = ui { TextInputSource.select(originalLayout) }
            assert(restored) { "Failed to restore to $originalLayout" }

            val currentAfterRestore = ui { TextInputSource.current() }
            assertEquals(originalLayout, currentAfterRestore)
        } else {
            println("Only one keyboard layout available, skipping switch test")
        }
    }

    @Test
    fun `ascii capable test`() {
        val asciiCapableLayouts = listOf(
            "com.apple.keylayout.ABC",
            "com.apple.keylayout.Dvorak",
        )
        val nonAsciiCapableLayouts = listOf(
            "com.apple.keylayout.Russian",
            "com.apple.keylayout.Serbian",
        )
        asciiCapableLayouts.forEach { layout ->
            assert(ui { TextInputSource.isAsciiCapable(layout) }) {
                "$layout should be ASCII capable"
            }
        }
        nonAsciiCapableLayouts.forEach { layout ->
            assert(!ui { TextInputSource.isAsciiCapable(layout) }) {
                "$layout should NOT be ASCII capable"
            }
        }
    }

    @Test
    fun `swedish test`() {
        withInputSourceSelected("com.apple.keylayout.Swedish-Pro") {
            ui { robot.emulateKeyboardEvent(KeyCode.ANSI_Semicolon, true) }
            ui { robot.emulateKeyboardEvent(KeyCode.ANSI_Semicolon, false) }
            awaitEventOfType<Event.KeyDown> { it.characters.text == "ö" }
        }
    }

    @Test
    fun `switch layout works fast`() {
        repeat(10) {
            val keyCode = KeyCode.ANSI_A
            val englishLetter = "a"
            val russianLetter = "ф"
            withInputSourceSelected("com.apple.keylayout.ABC") {
                ui { robot.emulateKeyboardEvent(keyCode, true) }
                ui { robot.emulateKeyboardEvent(keyCode, false) }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = englishLetter,
                    key = englishLetter,
                    keyWithModifiers = englishLetter,
                    modifiers = emptySet(),
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = englishLetter,
                    key = englishLetter,
                    keyWithModifiers = englishLetter,
                    modifiers = emptySet(),
                )
            }

            withInputSourceSelected("com.apple.keylayout.Russian") {
                ui { robot.emulateKeyboardEvent(keyCode, true) }
                ui { robot.emulateKeyboardEvent(keyCode, false) }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    characters = russianLetter,
                    key = russianLetter,
                    keyWithModifiers = russianLetter,
                    modifiers = emptySet(),
                )
                assertKeyUp(
                    awaitEventOfType<Event.KeyUp> { it.keyCode == keyCode },
                    keyCode,
                    typed = russianLetter,
                    key = russianLetter,
                    keyWithModifiers = russianLetter,
                    modifiers = emptySet(),
                )
            }
        }
    }
}
