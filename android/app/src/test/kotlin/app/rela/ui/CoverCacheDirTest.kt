package app.rela.ui

import java.io.File
import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class CoverCacheDirTest {
    private fun dir() = Files.createTempDirectory("covers").toFile().apply {
        File(this, "a.img").writeBytes(ByteArray(3))
        File(this, "a.meta").writeBytes(ByteArray(5))
    }

    @Test
    fun sizeCountsFiles() = assertEquals(8L, coverCacheBytes(dir()))

    @Test
    fun missingDirIsEmpty() = assertEquals(0L, coverCacheBytes(File("/nonexistent/covers-rs")))

    @Test
    fun clearRemovesFilesKeepsDir() {
        val d = dir()
        clearCoverCache(d)
        assertEquals(0L, coverCacheBytes(d))
        assertTrue(d.isDirectory)
        clearCoverCache(File("/nonexistent/covers-rs")) // no throw
    }
}
