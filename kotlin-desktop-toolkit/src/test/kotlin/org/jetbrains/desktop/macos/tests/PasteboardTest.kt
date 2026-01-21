package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Pasteboard
import org.jetbrains.desktop.macos.Pasteboard.Element
import org.jetbrains.desktop.macos.PasteboardType
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import kotlin.io.path.absolutePathString
import kotlin.io.path.createTempFile
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertNull
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
                Pasteboard.Item(
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
                Pasteboard.Item(Element.ofString(Pasteboard.FILE_URL_TYPE, file1.toUri().toString())),
                Pasteboard.Item(Element.ofString(Pasteboard.FILE_URL_TYPE, file2.toUri().toString())),
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
                Pasteboard.Item(Element.ofString(Pasteboard.FILE_URL_TYPE, file.toUri().toString())),
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
                Pasteboard.Item(Element.ofString(Pasteboard.URL_TYPE, url)),
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

    @Test
    fun `readItemTypes returns expected types`() {
        val testPasteboard = PasteboardType.named("org.jetbrains.kdt.test-pasteboard")
        Pasteboard.clear(testPasteboard)
        Pasteboard.writeObjects(
            Pasteboard.Item(
                Element.ofString(Pasteboard.STRING_TYPE, "Hello"),
                Element.ofString(Pasteboard.HTML_TYPE, "<b>Hello</b>"),
            ),
            pasteboard = testPasteboard,
        )

        val types = Pasteboard.readItemTypes(0, testPasteboard)
        assertTrue(types.contains(Pasteboard.STRING_TYPE))
        assertTrue(types.contains(Pasteboard.HTML_TYPE))
        assertEquals(2, types.size)
    }

    @Test
    fun `readItemTypes for multiple items`() {
        val testPasteboard = PasteboardType.named("org.jetbrains.kdt.test-pasteboard2")
        Pasteboard.clear(testPasteboard)
        Pasteboard.writeObjects(
            listOf(
                Pasteboard.Item.ofString(Pasteboard.STRING_TYPE, "First"),
                Pasteboard.Item.ofString(Pasteboard.HTML_TYPE, "<b>Second</b>"),
            ),
            pasteboard = testPasteboard,
        )

        assertEquals(2, Pasteboard.itemCount(testPasteboard))

        val types0 = Pasteboard.readItemTypes(0, testPasteboard)
        assertTrue(types0.contains(Pasteboard.STRING_TYPE))

        val types1 = Pasteboard.readItemTypes(1, testPasteboard)
        assertTrue(types1.contains(Pasteboard.HTML_TYPE))
    }

    @Test
    fun `readItemData returns data for specific item and type`() {
        val testPasteboard = PasteboardType.named("org.jetbrains.kdt.test-pasteboard3")
        Pasteboard.clear(testPasteboard)
        Pasteboard.writeObjects(
            listOf(
                Pasteboard.Item(
                    Element.ofString(Pasteboard.STRING_TYPE, "First String"),
                    Element.ofString(Pasteboard.HTML_TYPE, "<b>First HTML</b>"),
                ),
                Pasteboard.Item.ofString(Pasteboard.STRING_TYPE, "Second String"),
            ),
            pasteboard = testPasteboard,
        )

        val item0String = Pasteboard.readItemData(0, Pasteboard.STRING_TYPE, testPasteboard)
        assertEquals("First String", item0String?.decodeToString())

        val item0Html = Pasteboard.readItemData(0, Pasteboard.HTML_TYPE, testPasteboard)
        assertEquals("<b>First HTML</b>", item0Html?.decodeToString())

        val item1String = Pasteboard.readItemData(1, Pasteboard.STRING_TYPE, testPasteboard)
        assertEquals("Second String", item1String?.decodeToString())

        val item1Html = Pasteboard.readItemData(1, Pasteboard.HTML_TYPE, testPasteboard)
        assertNull(item1Html)
    }
}
