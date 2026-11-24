package app.rela.core

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LyricsTest {
    private fun reply(structured: String) = Json.parseToJsonElement("""{"lyricsList":{"structuredLyrics":$structured}}""") as JsonObject

    private val synced = """{"synced":true,"line":[{"start":1000,"value":"bir"},{"start":5000,"value":"iki"},{"start":10000,"value":"üç"}]}"""

    @Test
    fun syncedLineAtAPosition() {
        val l = Lyrics.from(reply("[$synced]"))
        assertTrue(l.synced)
        assertEquals(-1, l.lineAt(0.5)) // before the first line
        assertEquals(0, l.lineAt(4.999))
        assertEquals(1, l.lineAt(5.0))
        assertEquals(2, l.lineAt(600.0))
    }

    @Test
    fun offsetShowsLinesSooner() {
        val l = Lyrics.from(reply("""[{"synced":true,"offset":500,"line":[{"start":1000,"value":"a"},{"start":5000,"value":"b"}]}]"""))
        assertEquals(1, l.lineAt(4.5))
    }

    @Test
    fun unsyncedFallback() {
        val l = Lyrics.from(reply("""[{"synced":false,"line":[{"value":"bir"},{"value":"iki"}]}]"""))
        assertFalse(l.synced)
        assertEquals(listOf("bir", "iki"), l.lines.map { it.text })
        assertEquals(-1, l.lineAt(3.0))
    }

    @Test
    fun syncedMainTrackWins() {
        val unsynced = """{"synced":false,"line":[{"value":"düz"}]}"""
        val translation = """{"kind":"translation","synced":true,"line":[{"start":0,"value":"çeviri"}]}"""
        assertEquals("bir", Lyrics.from(reply("[$unsynced,$translation,$synced]")).lines[0].text)
        assertEquals("çeviri", Lyrics.from(reply("[$translation]")).lines[0].text) // only a translation: show it
    }

    @Test
    fun wordTagsAreStripped() {
        val l = Lyrics.from(reply("""[{"synced":true,"line":[{"start":0,"value":"<00:00.10>Merhaba <00:00.80>dünya"}]}]"""))
        assertEquals("Merhaba dünya", l.lines[0].text)
    }

    @Test
    fun nothingWithoutLyrics() {
        assertTrue(Lyrics.from(JsonObject(emptyMap())).lines.isEmpty())
    }

    @Test
    fun oneTrackAsSubwaveSendsIt() {
        val t = Json.parseToJsonElement("""{"synced":true,"offset":-200,"lines":[{"start":1000,"value":"a"},{"start":5000,"value":"b"}]}""") as JsonObject
        val l = Lyrics.of(t)
        assertEquals(listOf("a", "b"), l.lines.map { it.text })
        assertEquals(-1, l.lineAt(1.1)) // offset -200: lines show later
        assertEquals(0, l.lineAt(5.1))
        assertEquals(1, l.lineAt(5.2))
    }

    @Test
    fun wordTagsBecomeWords() {
        val l = Lyrics.from(reply("""[{"synced":true,"line":[{"start":0,"value":"<00:00.10>Merhaba <00:00.80>dünya"}]}]"""))
        assertEquals("Merhaba dünya", l.lines[0].text)
        assertEquals(listOf(100L, 800L), l.lines[0].words!!.map { it.startMs })
        assertEquals("Merhaba dünya", l.lines[0].words!!.joinToString("") { it.text }) // words keep their spaces
    }

    @Test
    fun cuesSplitTheLine() {
        val l = Lyrics.from(reply("""[{"synced":true,"line":[{"start":1000,"value":"Hey there"}],
            "cueLine":[{"index":0,"value":"Hey there","cue":[{"start":1000,"value":"Hey"},{"start":1500,"value":"there"}]}]}]"""))
        assertEquals(listOf("Hey ", "there"), l.lines[0].words!!.map { it.text })
        assertEquals(listOf(1000L, 1500L), l.lines[0].words!!.map { it.startMs })
    }

    @Test
    fun cuesThatDoNotMatchKeepThePlainLine() {
        val l = Lyrics.from(reply("""[{"synced":true,"line":[{"start":0,"value":"abc"}],
            "cueLine":[{"index":0,"value":"abc","cue":[{"start":0,"value":"xyz"}]}]}]"""))
        assertEquals("abc", l.lines[0].text)
        assertEquals(null, l.lines[0].words)
    }

    @Test
    fun sungWordFollowsThePosition() {
        val l = Lyrics.from(reply("""[{"synced":true,"offset":100,"line":[{"start":0,"value":"<00:00.00>a <00:01.00>b <00:02.00>c"}]}]"""))
        assertEquals(0, l.wordAt(0, 0.5))
        assertEquals(1, l.wordAt(0, 0.95)) // offset 100 ms: words light up sooner
        assertEquals(2, l.wordAt(0, 9.0))
        assertEquals(-1, Lyrics.from(reply("[$synced]")).wordAt(0, 2.0)) // no word timing
    }

    @Test
    fun subwaveLinesCarryWordsToo() {
        val t = Json.parseToJsonElement("""{"synced":true,"lines":[{"start":0,"value":"<00:00.00>a <00:00.50>b"}]}""") as JsonObject
        assertEquals(2, Lyrics.of(t).lines[0].words!!.size)
    }
}
