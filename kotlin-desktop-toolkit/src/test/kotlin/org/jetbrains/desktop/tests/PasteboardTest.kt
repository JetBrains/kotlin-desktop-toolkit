package org.jetbrains.desktop.tests

import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.LogLevel
import org.jetbrains.desktop.macos.Pasteboard
import org.jetbrains.desktop.macos.Pasteboard.Element
import kotlin.io.path.absolutePathString
import kotlin.test.Test
import kotlin.test.assertEquals
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
        val strings = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.readItemsOfType(type = Pasteboard.STRING_TYPE)
        }
        assertEquals(listOf("Hello World!!!"), strings)
    }

    @Test
    fun putStringWithHTMLTest() {
        KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
        val htmlContent = """
                          <html>
                              <body>
                                  <span style = "color: green">Hello World!!!</span>
                              </body>
                          </html>
        """.trimIndent()
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
                        content = htmlContent,
                    ),
                ),
            )
        }
        assertTrue(success)
        val strings = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.readItemsOfType(type = Pasteboard.STRING_TYPE)
        }
        assertEquals(listOf("Hello World!!!"), strings)
        val htmls = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.readItemsOfType(type = Pasteboard.HTML_TYPE)
        }
        assertEquals(listOf(htmlContent), htmls)
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
        val strings = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.readItemsOfType(type = Pasteboard.STRING_TYPE)
        }
        assertEquals(listOf("String1", "String2"), strings)
    }

    @Test
    fun putTwoFilesTest() {
        KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
        val counter = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val file1 = kotlin.io.path.createTempFile(suffix = "File1.txt")
        val file2 = kotlin.io.path.createTempFile(suffix = "File2.txt")
        val content1 = "Hello1"
        val content2 = "Hello2"
        val success = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.writeObjects(
                Pasteboard.Item.Url("file://${file1.absolutePathString()}"),
                Pasteboard.Item.Url("file://${file2.absolutePathString()}"),
                Pasteboard.Item.of(type = Pasteboard.STRING_TYPE, content = content1),
                Pasteboard.Item.of(type = Pasteboard.STRING_TYPE, content = content2),
            )
        }
        assertTrue(success)
        val files = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.readFileItemPaths()
        }
        val strings = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.readItemsOfType(type = Pasteboard.STRING_TYPE)
        }
        assertEquals(listOf(file1.absolutePathString(), file2.absolutePathString()), files)
        assertEquals(listOf(content1, content2), strings)
    }

    @Test
    fun putHttpsUrl() {
        KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
        val url = "https://jetbrains.com"
        val counter = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val success = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.writeObjects(
                Pasteboard.Item.Url(url),
            )
        }
        assertTrue(success)
        val urls = GrandCentralDispatch.dispatchOnMainSync {
            Pasteboard.readItemsOfType(type = Pasteboard.URL_TYPE)
        }
        assertEquals(listOf(url), urls)
    }
}
