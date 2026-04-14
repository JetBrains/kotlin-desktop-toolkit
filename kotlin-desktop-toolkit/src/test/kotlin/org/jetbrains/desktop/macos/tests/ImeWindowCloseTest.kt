package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalRect
import org.jetbrains.desktop.macos.Robot
import org.jetbrains.desktop.macos.TextInputClient
import org.jetbrains.desktop.macos.TextRange
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.Timeout
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.util.concurrent.TimeUnit
import kotlin.test.Ignore
import kotlin.test.Test
import kotlin.test.assertTrue

/**
 * Regression test for a crash when closing a window while an IME (e.g. Japanese) has active marked text.
 *
 * The crash happens because macOS's NSTextInputContext may still query the NSTextInputClient protocol
 * (e.g. selectedRange, setMarkedText) during or after window close. If the Kotlin-side callback arena
 * is already closed, this results in EXC_BAD_ACCESS.
 *
 * The original crash was observed with Fcitx5 IME, where a deferred IME menu setup block
 * (utSetupInputMethodMenuFromDeferredBlock) calls back into selectedRange after the window is destroyed.
 */
@EnabledOnOs(OS.MAC)
class ImeWindowCloseTest : KDTApplicationTestBase() {

    companion object {
        lateinit var robot: Robot

        @JvmStatic
        @BeforeAll
        @Timeout(value = 15, unit = TimeUnit.SECONDS)
        fun init() {
            robot = ui { Robot() }
        }

        @JvmStatic
        @AfterAll
        @Timeout(value = 15, unit = TimeUnit.SECONDS)
        fun destroy() {
            ui { robot.close() }
        }
    }

    @Ignore("Test crash the JVM")
    @Test
    @Timeout(value = 30, unit = TimeUnit.SECONDS)
    fun `closing window with active IME marked text should not crash`() {
        val window = createWindowAndEnsureItsFocused(name = "IME Close Test Window")

        // Set up a TextInputClient that tracks marked text state
        val imeClient = object : TextInputClient {
            @Volatile
            var markedText: String = ""

            override fun hasMarkedText(): Boolean = markedText.isNotEmpty()

            override fun markedRange(): TextRange? {
                return if (markedText.isNotEmpty()) TextRange(0, markedText.length.toLong()) else null
            }

            override fun selectedRange(): TextRange {
                val pos = markedText.length.toLong()
                return TextRange(pos, 0)
            }

            override fun insertText(text: String, replacementRange: TextRange?) {
                markedText = ""
            }

            override fun doCommand(command: String): Boolean = false

            override fun unmarkText() {
                markedText = ""
            }

            override fun setMarkedText(text: String, selectedRange: TextRange?, replacementRange: TextRange?) {
                markedText = text
            }

            override fun attributedStringForRange(range: TextRange): TextInputClient.StringAndRange {
                return TextInputClient.StringAndRange(markedText, range)
            }

            override fun firstRectForCharacterRange(range: TextRange): TextInputClient.RectAndRange {
                return TextInputClient.RectAndRange(LogicalRect.Zero, range)
            }

            override fun characterIndexForPoint(point: LogicalPoint): Long? = null
        }

        ui { window.setTextInputClient(imeClient) }

        // Switch to Japanese IME and type characters to produce marked (composing) text
        withInputSourceSelected("com.apple.inputmethod.Kotoeri.RomajiTyping.Japanese") {
            withEventHandler({ event ->
                if (event is Event.KeyDown || event is Event.KeyUp) {
                    window.textInputContext.handleCurrentEvent()
                } else {
                    EventHandlerResult.Continue
                }
            }) {
                // Type "a" which in Japanese IME produces marked text "あ"
                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_A, true) }
                ui { robot.emulateKeyboardEvent(KeyCode.ANSI_A, false) }

                // Wait for the key event to be processed by the IME
                awaitEventOfType<Event.KeyDown> { true }
                Thread.sleep(200)

                assertTrue(imeClient.hasMarkedText(), "IME should have produced marked text")
                Logger.info { "Marked text before close: '${imeClient.markedText}'" }
            }

            // Close the window while Japanese IME is still active and has marked text.
            // This is the crash scenario: macOS's NSTextInputContext may schedule deferred
            // blocks that query selectedRange/setMarkedText after the window is destroyed.
            ui { window.close() }

            // Pump the event loop to give deferred IME blocks a chance to execute.
            // The original crash happens when a deferred IME block runs after window destruction.
            Thread.sleep(500)
            ui { /* force a runloop iteration */ }
        }
    }
}
