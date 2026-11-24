package app.rela.core

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SongTest {
    private fun obj(text: String) = Json.parseToJsonElement(text) as JsonObject

    @Test
    fun readsTheKeysTheScreensShow() {
        val s = Song(obj("""{"id":"s1","title":"Teardrop","artist":"Massive Attack","duration":330,"starred":"2026-01-01T00:00:00Z","userRating":4,"discNumber":2,"track":7}"""))
        assertEquals("Teardrop", s.title)
        assertEquals("Massive Attack", s.artist)
        assertEquals(330.0, s.duration, 0.0)
        assertTrue(s.starred)
        assertEquals(4, s.rating)
        assertEquals(2, s.disc)
        assertEquals(7, s.track)
    }

    @Test
    fun missingKeysAreEmpty() {
        val s = Song(obj("""{"id":"s1"}"""))
        assertEquals("", s.title)
        assertEquals(0.0, s.duration, 0.0)
        assertFalse(s.starred)
        assertEquals(0, s.rating)
        assertEquals(1, s.disc) // no disc number: the first disc
    }

    @Test
    fun songsUnderAKey() {
        val album = obj("""{"song":[{"id":"a"},{"id":"b"},7]}""")
        assertEquals(listOf("a", "b"), album.songs("song").map { it.id })
        assertEquals(listOf("x"), obj("""{"entry":{"id":"x"}}""").songs("entry").map { it.id })
        assertTrue(album.songs("entry").isEmpty())
    }

    @Test
    fun albumLength() {
        assertEquals("4 dk", length(250.0))
        assertEquals("1 sa 2 dk", length(3725.0))
    }
}
