package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.Robot
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
            inputSourceBeforeTest = ui { Application.currentInputSource()!! }
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
            ui { Application.chooseInputSource(inputSourceBeforeTest) }
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
        val inputSource = ui { Application.currentInputSource() }
        assert(inputSource?.startsWith("com.apple.keylayout") == true) {
            "$inputSource should start with 'com.apple.keylayout'"
        }
    }

    @Test
    fun `list input sources test`() {
        val inputSources = ui { Application.listInputSources() }
        Logger.info { "Input sources: $inputSources" }
        assert(inputSources.isNotEmpty()) { "Input sources list should not be empty" }
        assert(inputSources.any { it.startsWith("com.apple.keylayout.") }) {
            "Should contain at least one keyboard layout"
        }
    }

    @Test
    fun `check that all required input sources are installed`() {
        val inputSources = ui { Application.listInputSources() }
        assertContains(inputSources, "com.apple.keylayout.ABC")
        assertContains(inputSources, "com.apple.keylayout.Russian")
        assertContains(inputSources, "com.apple.keylayout.Swedish-Pro")
        assertContains(inputSources, "com.apple.keylayout.USInternational-PC")
        assertContains(inputSources, "com.apple.keylayout.German")
        assertContains(inputSources, "com.apple.keylayout.Serbian-Latin")
        assertContains(inputSources, "com.apple.keylayout.Serbian")
        assertContains(inputSources, "com.apple.keylayout.Dvorak")
        assertContains(inputSources, "com.apple.keylayout.DVORAK-QWERTYCMD")
        assertContains(inputSources, "com.apple.inputmethod.Kotoeri.RomajiTyping.Japanese")
        assertContains(inputSources, "com.apple.inputmethod.TCIM.Pinyin")
        assertContains(inputSources, "com.apple.inputmethod.Korean.2SetKorean")
    }

    @Test
    fun `current input source is in the list of input sources`() {
        val currentLayout = ui { Application.currentInputSource() }
        val inputSources = ui { Application.listInputSources() }
        assert(currentLayout != null) { "Current keyboard layout should not be null" }
        assertContains(inputSources, currentLayout, "Current keyboard layout should be in the list of input sources")
    }

    @Test
    fun `choose input source and restore`() {
        val originalLayout = ui { Application.currentInputSource() }
        assertNotNull(originalLayout)

        val inputSources = ui { Application.listInputSources() }
        val anotherLayout = inputSources.firstOrNull { it != originalLayout && it.startsWith("com.apple.keylayout.") }

        if (anotherLayout != null) {
            val switched = ui { Application.chooseInputSource(anotherLayout) }
            assert(switched) { "Failed to switch to $anotherLayout" }

            val currentAfterSwitch = ui { Application.currentInputSource() }
            assertEquals(anotherLayout, currentAfterSwitch)

            // Restore original layout
            val restored = ui { Application.chooseInputSource(originalLayout) }
            assert(restored) { "Failed to restore to $originalLayout" }

            val currentAfterRestore = ui { Application.currentInputSource() }
            assertEquals(originalLayout, currentAfterRestore)
        } else {
            println("Only one keyboard layout available, skipping switch test")
        }
    }

    @Test
    fun `swedish test`() {
        withInputSource("com.apple.keylayout.Swedish-Pro") {
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
            withInputSource("com.apple.keylayout.ABC") {
                ui { robot.emulateKeyboardEvent(keyCode, true) }
                ui { robot.emulateKeyboardEvent(keyCode, false) }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    typed = englishLetter,
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

            withInputSource("com.apple.keylayout.Russian") {
                ui { robot.emulateKeyboardEvent(keyCode, true) }
                ui { robot.emulateKeyboardEvent(keyCode, false) }
                assertKeyDown(
                    awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode },
                    keyCode,
                    typed = russianLetter,
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
