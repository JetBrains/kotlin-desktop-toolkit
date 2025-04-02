package org.jetbrains.desktop.tests

import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.LogLevel
import org.jetbrains.desktop.macos.Pasteboard
import org.jetbrains.desktop.macos.Pasteboard.Element
import kotlin.io.path.absolutePathString
import kotlin.test.Test
import kotlin.test.assertTrue

class PasteboardTest {
    @Test
    fun putStringTest() {
        KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
        val counter = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val success = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.writeObjects(
                Pasteboard.Item.Combined(
                    Element(
                        type = Pasteboard.STRING_TYPE,
                        content = "Hello World!!!",
                    ),
                ),
            )
        }
        assertTrue(success)
    }

    @Test
    fun putStringWithHTMLTest() {
        KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
        val counter = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val success = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.writeObjects(
                Pasteboard.Item.Combined(
                    Element(
                        type = Pasteboard.STRING_TYPE,
                        content = "Hello World!!!",
                    ),
                    Element(
                        type = Pasteboard.HTML_TYPE,
                        content = """
                          <html>
                              <body>
                                  <span style = "color: green">Hello World!!!</span>
                              </body>
                          </html>
                        """.trimIndent(),
                    ),
                ),
            )
        }
        assertTrue(success)
    }

    @Test
    fun putTwoStringTest() {
        KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
        val counter = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val success = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.writeObjects(
                Pasteboard.Item.Combined(
                    Element(
                        type = Pasteboard.STRING_TYPE,
                        content = "String1",
                    ),
                ),
                Pasteboard.Item.Combined(
                    Element(
                        type = Pasteboard.STRING_TYPE,
                        content = "String2",
                    ),
                ),
            )
        }
        assertTrue(success)
    }

    @Test
    fun putTwoFilesTest() {
        KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
        val counter = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val success = GrandCentralDispatch.dispatchOnMainSync {
            val file1 = kotlin.io.path.createTempFile(suffix = "File1.txt")
            val file2 = kotlin.io.path.createTempFile(suffix = "File2.txt")
            Pasteboard.writeObjects(
                Pasteboard.Item.Url("file://${file1.absolutePathString()}"),
                Pasteboard.Item.Url("file://${file2.absolutePathString()}"),
                Pasteboard.Item.of(type = Pasteboard.STRING_TYPE, content = "Hello1"),
                Pasteboard.Item.of(type = Pasteboard.STRING_TYPE, content = "Hello2"),
            )
        }
        assertTrue(success)
    }

    @Test
    fun putHttpsUrl() {
        KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
        val counter = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val success = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.writeObjects(
                Pasteboard.Item.Url("https://jetbrains.com"),
            )
        }
        assertTrue(success)
    }
}
