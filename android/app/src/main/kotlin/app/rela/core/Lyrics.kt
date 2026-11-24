package app.rela.core

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull

/**
 * A song's lyrics from getLyricsBySongId (OpenSubsonic structuredLyrics): the main track, synced if there is one.
 * Word timing as the desktop's LyricsData.qml reads it: enhanced LRC `<mm:ss.xx>` tags, or `cueLine` (songLyrics v2).
 */
data class Lyrics(val lines: List<Line>, val synced: Boolean, val offsetMs: Long) {
    /** A timed word or syllable; [text] keeps the spaces after it, so the words joined give the line back. */
    data class Word(val startMs: Long, val text: String)
    data class Line(val startMs: Long, val text: String, val words: List<Word>? = null)

    /** The sung line at [seconds]; -1 before the first line or when not synced. */
    fun lineAt(seconds: Double): Int {
        if (!synced) return -1
        val ms = seconds * 1000
        var k = -1
        for ((i, l) in lines.withIndex()) if (l.startMs - offsetMs <= ms) k = i else break
        return k
    }

    /** The sung word of [line] at [seconds]; -1 before its first word, or when the line has no word timing. */
    fun wordAt(line: Int, seconds: Double): Int {
        val words = lines.getOrNull(line)?.words ?: return -1
        if (!synced) return -1
        val ms = seconds * 1000
        var k = -1
        for ((i, w) in words.withIndex()) if (w.startMs - offsetMs <= ms) k = i else break
        return k
    }

    companion object {
        private val tag = Regex("<(\\d+):(\\d+(?:\\.\\d+)?)>")

        /** Enhanced LRC: a tag before each word; whitespace between tags joins the word before it. */
        private fun split(start: Long, value: String): Line {
            if (!tag.containsMatchIn(value)) return Line(start, value.trim())
            val words = mutableListOf<Word>()
            var at = 0
            var t0 = start
            fun take(t: String) {
                if (t.isNotBlank()) words += Word(t0, t)
                else if (words.isNotEmpty()) words[words.lastIndex] = words.last().let { it.copy(text = it.text + t) }
            }
            for (m in tag.findAll(value)) {
                take(value.substring(at, m.range.first))
                t0 = Math.round((m.groupValues[1].toDouble() * 60 + m.groupValues[2].toDouble()) * 1000)
                at = m.range.last + 1
            }
            take(value.substring(at))
            return Line(start, words.joinToString("") { it.text }.trim(), words.ifEmpty { null })
        }

        /** cueLine[].cue: timed pieces of line[index], found in order in cueLine.value; a piece not found keeps the line plain. */
        private fun withCues(lines: List<Line>, cueLines: List<JsonObject>): List<Line> {
            val out = lines.toMutableList()
            for (c in cueLines) {
                val i = Song(c).num("index").toInt()
                val cues = c.listOf("cue")
                if (i !in out.indices || cues.isEmpty()) continue
                val v = c["value"].text()
                var at = 0
                val pos = cues.map { q -> v.indexOf(q["value"].text(), at).also { if (it >= 0) at = it + q["value"].text().length } }
                if (pos.any { it < 0 }) continue
                val words = cues.mapIndexed { k, q -> Word(Song(q).num("start").toLong(), v.substring(if (k == 0) 0 else pos[k], pos.getOrElse(k + 1) { v.length })) }
                out[i] = Line(out[i].startMs, v, words)
            }
            return out
        }

        fun from(reply: JsonObject): Lyrics {
            val all = reply["lyricsList"].obj().listOf("structuredLyrics")
            val main = all.filter { it["kind"].text().let { k -> k.isEmpty() || k == "main" } }.ifEmpty { all }
            val synced: (JsonObject) -> Boolean = { (it["synced"] as? JsonPrimitive)?.booleanOrNull == true }
            return of(main.firstOrNull(synced) ?: main.firstOrNull() ?: return Lyrics(emptyList(), false, 0))
        }

        /** One track: a structuredLyrics entry (`line`) or SUB/WAVE's lyrics (`lines`, same line shape). */
        fun of(track: JsonObject): Lyrics {
            val lines = (track.listOf("line") + track.listOf("lines")).map { split(Song(it).num("start").toLong(), it["value"].text()) }
            return Lyrics(withCues(lines, track.listOf("cueLine")), (track["synced"] as? JsonPrimitive)?.booleanOrNull == true, Song(track).num("offset").toLong())
        }
    }
}
