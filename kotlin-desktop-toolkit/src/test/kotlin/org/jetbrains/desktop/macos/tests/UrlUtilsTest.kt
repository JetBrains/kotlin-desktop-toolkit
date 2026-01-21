package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.NativeError
import org.jetbrains.desktop.macos.UrlUtils
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.nio.file.Files
import kotlin.io.path.createTempFile
import kotlin.io.path.deleteExisting
import kotlin.io.path.writeText
import kotlin.test.Ignore
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

@EnabledOnOs(OS.MAC)
class UrlUtilsTest : KDTTestBase() {

    @Test
    fun `filePathToFileUrl converts simple path`() {
        val result = UrlUtils.filePathToFileUrl("/tmp/test.txt")
        assertEquals("file:///tmp/test.txt", result)
    }

    @Test
    fun `filePathToFileUrl handles spaces in path`() {
        val result = UrlUtils.filePathToFileUrl("/tmp/my file.txt")
        assertEquals("file:///tmp/my%20file.txt", result)
    }

    @Test
    fun `urlToFilePath converts simple file url`() {
        val result = UrlUtils.urlToFilePath("file:///tmp/test.txt")
        assertEquals("/tmp/test.txt", result)
    }

    @Test
    fun `urlToFilePath handles encoded spaces`() {
        val result = UrlUtils.urlToFilePath("file:///tmp/my%20file.txt")
        assertEquals("/tmp/my file.txt", result)
    }

    @Test
    fun `filePathToFileUrl and urlToFilePath roundtrip`() {
        val originalPath = "/Users/test/Documents/my file.txt"
        val url = UrlUtils.filePathToFileUrl(originalPath)
        assertNotNull(url)
        val roundtripPath = UrlUtils.urlToFilePath(url)
        assertEquals(originalPath, roundtripPath)
    }

    @Test
    fun `filePathToFileReferenceUrl returns reference url for existing file`() {
        val tempFile = createTempFile(suffix = ".txt")
        tempFile.writeText("test content")

        val refUrl = UrlUtils.filePathToFileReferenceUrl(tempFile.toFile().canonicalPath)
        assertNotNull(refUrl)
        assertTrue(refUrl.startsWith("file:///.file/id="), "Expected file reference URL, got: $refUrl")
    }

    @Test
    fun `filePathToFileReferenceUrl returns null for non-existing file`() {
        val refUrl = UrlUtils.filePathToFileReferenceUrl("/nonexistent/path/to/file.txt")
        assertNull(refUrl)
    }

    @Test
    fun `urlToFilePath resolves file reference url`() {
        val tempFile = createTempFile(suffix = ".txt")
        tempFile.writeText("test content")
        // Use canonical path since file reference URLs resolve symlinks (e.g., /var -> /private/var on macOS)
        val canonicalPath = tempFile.toFile().canonicalPath

        val refUrl = UrlUtils.filePathToFileReferenceUrl(canonicalPath)
        assertNotNull(refUrl)

        val resolvedPath = UrlUtils.urlToFilePath(refUrl)
        assertEquals(canonicalPath, resolvedPath)
    }

    @Test
    fun `file reference url roundtrip with spaces in path`() {
        val tempFile = createTempFile(suffix = " with spaces.txt")
        tempFile.writeText("test content")
        // Use canonical path since file reference URLs resolve symlinks
        val canonicalPath = tempFile.toFile().canonicalPath

        val refUrl = UrlUtils.filePathToFileReferenceUrl(canonicalPath)
        assertNotNull(refUrl)

        val resolvedPath = UrlUtils.urlToFilePath(refUrl)
        assertEquals(canonicalPath, resolvedPath)
    }

    @Test
    fun `file reference url roundtrip with file deletion`() {
        val tempFile = createTempFile(suffix = ".txt")
        tempFile.writeText("test content")
        val canonicalPath = tempFile.toFile().canonicalPath

        val refUrl = UrlUtils.filePathToFileReferenceUrl(canonicalPath)
        assertNotNull(refUrl)
        tempFile.deleteExisting()
        val resolvedPath = UrlUtils.urlToFilePath(refUrl)
        assertNull(resolvedPath)
    }

    @Test
    fun `file reference url roundtrip with file rename`() {
        val tempFile = createTempFile(suffix = ".txt")
        tempFile.writeText("test content")
        val canonicalPath = tempFile.toFile().canonicalPath

        val refUrl = UrlUtils.filePathToFileReferenceUrl(canonicalPath)
        assertNotNull(refUrl)

        // Rename the file
        val renamedFile = tempFile.resolveSibling("renamed_${System.nanoTime()}.txt")
        Files.move(tempFile, renamedFile)

        // File reference URL should still resolve to the renamed file
        val resolvedPath = UrlUtils.urlToFilePath(refUrl)
        assertEquals(renamedFile.toFile().canonicalPath, resolvedPath)
    }

    @Ignore("Apparently macOS 15 which is used on CI is more tolerant of invalid URLs and doesn't throw an exception")
    @Test
    fun `urlToFilePath throws for invalid URL`() {
        assertFailsWith<NativeError> {
            UrlUtils.urlToFilePath("not a valid url")
        }
    }

    @Test
    fun `urlToFilePath returns null for non-file URL`() {
        val result = UrlUtils.urlToFilePath("https://example.com/path")
        assertNull(result)
    }
}
