package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Pasteboard
import org.jetbrains.desktop.macos.Pasteboard.Element
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import kotlin.io.path.absolutePathString
import kotlin.io.path.createTempFile
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@EnabledOnOs(OS.MAC)
class PasteboardTest : KDTApplicationTestBase() {

    private fun List<ByteArray>.toStringsList(): List<String> {
        return this.map { it.decodeToString() }
    }

    @Test
    fun putStringTest() {
        val counter = ui {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val success = ui {
            Pasteboard.writeObjects(
                Pasteboard.Item.ofString(
                    type = Pasteboard.STRING_TYPE,
                    content = "Hello World!!!",
                ),
            )
        }
        assertTrue(success)
        val strings = ui {
            Pasteboard.readItemsOfType(type = Pasteboard.STRING_TYPE).toStringsList()
        }
        assertEquals(listOf("Hello World!!!"), strings)
    }

    @Test
    fun putStringWithHTMLTest() {
        val htmlContent = """
                          <html>
                              <body>
                                  <span style = "color: green">Hello World!!!</span>
                              </body>
                          </html>
        """.trimIndent()
        val counter = ui {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val success = ui {
            Pasteboard.writeObjects(
                Pasteboard.Item.Combined(
                    Element.ofString(
                        type = Pasteboard.STRING_TYPE,
                        content = "Hello World!!!",
                    ),
                    Element.ofString(
                        type = Pasteboard.HTML_TYPE,
                        content = htmlContent,
                    ),
                ),
            )
        }
        assertTrue(success)
        val strings = ui {
            Pasteboard.readItemsOfType(type = Pasteboard.STRING_TYPE).toStringsList()
        }
        assertEquals(listOf("Hello World!!!"), strings)
        val htmls = ui {
            Pasteboard.readItemsOfType(type = Pasteboard.HTML_TYPE).toStringsList()
        }
        assertEquals(listOf(htmlContent), htmls)
    }

    @Test
    fun putTwoStringTest() {
        val counter = ui {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val success = ui {
            Pasteboard.writeObjects(
                Pasteboard.Item.ofString(
                    type = Pasteboard.STRING_TYPE,
                    content = "String1",
                ),
                Pasteboard.Item.ofString(
                    type = Pasteboard.STRING_TYPE,
                    content = "String2",
                ),
            )
        }
        assertTrue(success)
        val strings = ui {
            Pasteboard.readItemsOfType(type = Pasteboard.STRING_TYPE).toStringsList()
        }
        assertEquals(listOf("String1", "String2"), strings)
    }

    @Test
    fun putTwoFilesTest() {
        val counter = ui {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val file1 = createTempFile(suffix = "File1.txt")
        val file2 = createTempFile(suffix = "File2.txt")
        val content1 = "Hello1"
        val content2 = "Hello2"
        val success = ui {
            Pasteboard.writeObjects(
                Pasteboard.Item.File(file1.absolutePathString()),
                Pasteboard.Item.File(file2.absolutePathString()),
                Pasteboard.Item.ofString(type = Pasteboard.STRING_TYPE, content = content1),
                Pasteboard.Item.ofString(type = Pasteboard.STRING_TYPE, content = content2),
            )
        }
        assertTrue(success)
        val files = ui {
            Pasteboard.readFileItemPaths()
        }
        val strings = ui {
            Pasteboard.readItemsOfType(type = Pasteboard.STRING_TYPE).toStringsList()
        }
        assertEquals(listOf(file1.absolutePathString(), file2.absolutePathString()), files)
        assertEquals(listOf(content1, content2), strings)
    }

    @Test
    fun putFileWithSpaceInPathTest() {
        val counter = ui {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val file = createTempFile(suffix = "File name with spaces.txt")
        val success = ui {
            Pasteboard.writeObjects(
                Pasteboard.Item.File(file.absolutePathString()),
            )
        }
        assertTrue(success)
        val files = ui {
            Pasteboard.readFileItemPaths()
        }
        assertEquals(listOf(file.absolutePathString()), files)
    }

    @Test
    fun putHttpsUrl() {
        val url = "https://jetbrains.com"
        val counter = ui {
            Pasteboard.clear()
        }
        assert(counter > 0L)
        val success = ui {
            Pasteboard.writeObjects(
                Pasteboard.Item.Url(url),
            )
        }
        assertTrue(success)
        val urls = ui {
            Pasteboard.readItemsOfType(type = Pasteboard.URL_TYPE).toStringsList()
        }
        assertEquals(listOf(url), urls)
    }

    @Test
    fun putEmoji() {
        val counter = ui {
            Pasteboard.clear()
        }
        assert(counter > 0L)

        val emojiString = "😃"
        val success = ui {
            Pasteboard.writeObjects(
                Pasteboard.Item.ofString(type = Pasteboard.STRING_TYPE, content = emojiString),
            )
        }
        assertTrue(success)
        val result = ui {
            Pasteboard.readItemsOfType(type = Pasteboard.STRING_TYPE).toStringsList()
        }
        assertEquals(listOf(emojiString), result)
    }

    @Test
    fun putPngImage() {
        val counter = ui {
            Pasteboard.clear()
        }
        assert(counter > 0L)

        val imageBytes = jbIconBytes()
        val success = ui {
            Pasteboard.writeObjects(
                Pasteboard.Item.of(
                    type = Pasteboard.PNG_IMAGE_TYPE,
                    content = imageBytes,
                ),
            )
        }
        assertTrue(success)
        val result = ui {
            Pasteboard.readItemsOfType(type = Pasteboard.PNG_IMAGE_TYPE).single()
        }
        assertContentEquals(imageBytes, result)
    }
}
