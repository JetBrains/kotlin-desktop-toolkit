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
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.Timeout
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.util.concurrent.TimeUnit
import kotlin.test.Test
import kotlin.test.assertEquals

@EnabledOnOs(OS.MAC)
class KoreanImeTest : KDTApplicationTestBase() {

    companion object {
        lateinit var window: Window
        lateinit var robot: Robot

        @JvmStatic
        @BeforeAll
        @Timeout(value = 15, unit = TimeUnit.SECONDS)
        fun init() {
            robot = ui { Robot() }
            window = createWindowAndEnsureItsFocused(name = "Korean IME Test Window")
        }

        @JvmStatic
        @AfterAll
        @Timeout(value = 15, unit = TimeUnit.SECONDS)
        fun destroy() {
            ui { robot.close() }
            ui { window.close() }
        }
    }

    fun press(keyCode: KeyCode) {
        ui { robot.emulateKeyboardEvent(keyCode, true) }
        ui { robot.emulateKeyboardEvent(keyCode, false) }
        awaitEventOfType<Event.KeyDown> { it.keyCode == keyCode }
    }

    /**
     * Types S, U, D keys with Korean 2-Set IME selected and verifies that
     * the TextInputClient receives the composed syllable 녕 (nyeong).
     *
     * In 2-Set Korean:
     *   S → ㄴ (nieun), U → ㅕ (yeo), D → ㅇ (ieung)
     *   ㄴ + ㅕ + ㅇ = 녕
     */
    @Test
    @Timeout(value = 30, unit = TimeUnit.SECONDS)
    fun `typing S U D with Korean 2-Set IME produces 녕`() {
        val insertedTexts = mutableListOf<String>()
        val markedTexts = mutableListOf<String>()

        val imeClient = object : TextInputClient {
            @Volatile
            var committed = ""

            @Volatile
            var marked = ""

            override fun hasMarkedText(): Boolean {
                val result = marked.isNotEmpty()
                Logger.info { "hasMarkedText: $result" }
                return result
            }

            override fun markedRange(): TextRange? {
                val result = if (marked.isNotEmpty()) TextRange(committed.length.toLong(), marked.length.toLong()) else null
                Logger.info { "markedRange: $result" }
                return result
            }

            override fun selectedRange(): TextRange {
                val pos = (committed.length + marked.length).toLong()
                val result = TextRange(pos, 0)
                Logger.info { "selectedRange: $result" }
                return result
            }

            override fun insertText(text: String, replacementRange: TextRange?) {
                Logger.info { "insertText: '$text', replacementRange=$replacementRange" }
                marked = ""
                if (replacementRange != null) {
                    val start = replacementRange.location.toInt()
                    val end = start + replacementRange.length.toInt()
                    committed = committed.substring(0, start) + text + committed.substring(end.coerceAtMost(committed.length))
                } else {
                    committed += text
                }
                insertedTexts.add(text)
            }

            override fun doCommand(command: String): Boolean {
                Logger.info { "doCommand: '$command'" }
                return false
            }

            override fun unmarkText() {
                Logger.info { "unmarkText, marked was: '$marked'" }
                committed += marked
                marked = ""
            }

            override fun setMarkedText(text: String, selectedRange: TextRange?, replacementRange: TextRange?) {
                Logger.info { "setMarkedText: '$text'" }
                marked = text
                markedTexts.add(text)
            }

            override fun attributedStringForRange(range: TextRange): TextInputClient.StringAndRange {
                val full = committed + marked
                val result = TextInputClient.StringAndRange(full, range)
                Logger.info { "attributedStringForRange: range=$range, result=$result" }
                return result
            }

            override fun firstRectForCharacterRange(range: TextRange): TextInputClient.RectAndRange {
                val result = TextInputClient.RectAndRange(LogicalRect.Zero, range)
                Logger.info { "firstRectForCharacterRange: range=$range, result=$result" }
                return result
            }

            override fun characterIndexForPoint(point: LogicalPoint): Long? {
                Logger.info { "characterIndexForPoint: point=$point" }
                return null
            }
        }

        ui { window.setTextInputClient(imeClient) }

        withInputSourceSelected("com.apple.inputmethod.Korean.2SetKorean") {
            withEventHandler({ event ->
                when (event) {
                    is Event.KeyDown -> {
                        Logger.info { "Send KeyDown to ime: $event" }
                        window.textInputContext.handleCurrentEvent()
                    }
                    is Event.KeyUp -> EventHandlerResult.Stop // TODO is this stop important here?
                    else -> EventHandlerResult.Continue
                }
            }) {
                repeat(1) {
                    // S → ㄴ
                    press(KeyCode.ANSI_S)
                    // U → ㅕ (combines with ㄴ → 녀)
                    press(KeyCode.ANSI_U)
                    // D → ㅇ (combines → 녕)
                    press(KeyCode.ANSI_D)
                }

                // Give IME a moment to finish composing
                Thread.sleep(200)

                Logger.info { "markedTexts: $markedTexts" }
                Logger.info { "insertedTexts: $insertedTexts" }
                Logger.info { "current marked: '${imeClient.marked}', committed: '${imeClient.committed}'" }

                // Korean 2-Set IME composes via insertText with replacement ranges,
                // so the composed syllable is committed directly (not marked).
                assertEquals("녕", imeClient.committed, "Committed text should be 녕 after typing S, U, D")
            }
        }
    }
}
