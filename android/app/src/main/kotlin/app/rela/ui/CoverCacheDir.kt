package app.rela.ui

import android.content.Context
import java.io.File

/** The core's cover disk cache (CoverCache in crates/core); plain files, so Kotlin can size and clear it. */
fun coverCacheDir(context: Context) = File(context.cacheDir, "covers-rs")

fun coverCacheBytes(dir: File): Long = dir.listFiles()?.sumOf { it.length() } ?: 0L

/** The core may be writing a cover meanwhile: a file it is renaming in just stays or fails to delete, both harmless. */
fun clearCoverCache(dir: File) {
    dir.listFiles()?.forEach { it.delete() }
}
