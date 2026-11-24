package app.rela.playback

import app.rela.core.Song
import kotlinx.serialization.json.Json
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.Assert.assertNull
import uniffi.rela.PlayerState
import uniffi.rela.SyncState

class FacadeViewTest {
    private fun state(index: Long = 0, playing: Boolean = true, station: String = "{}", position: Double = 12.5, duration: Double = 0.0) =
        PlayerState(
            online = true, signedIn = true, loginFailed = false, status = "", user = "u", connecting = false, queueLength = 3u,
            index = index, currentJson = "{}", playing = playing, crossfade = 0, dj = false, stationJson = station,
            shuffle = false, repeat = 0, volume = 100.0, position = position, duration = duration, signalPath = "",
            bitrate = 0, streamInfo = "", optionsJson = "{}", theme = "",
        )

    private val queue = Song.list(
        Json.parseToJsonElement(
            """[{"id":"a","title":"Şımarık","artist":"Tarkan","duration":230},
                {"id":"b","title":"Gidiyorum","artist":"Sezen Aksu","duration":251},
                {"id":"c","title":"Teardrop","artist":"Massive Attack","duration":330}]""",
        ),
    )

    @Test
    fun middleSongHasNeighbours() {
        val v = facadeView(state(index = 1, duration = 250.5), queue)
        assertEquals(listOf("p:0:a", "c:1:b", "n:2:c"), v.items.map { it.uid })
        assertEquals(1, v.current)
        assertEquals("Gidiyorum", v.items[1].title)
        assertEquals(250_500L, v.items[1].durationMs)
        assertEquals(12_500L, v.positionMs)
        assertTrue(v.playing)
    }

    @Test
    fun edgesWrapSoPrevAndNextStayAvailable() {
        assertEquals(listOf("p:2:c", "c:0:a", "n:1:b"), facadeView(state(index = 0), queue).items.map { it.uid })
        assertEquals(listOf("p:1:b", "c:2:c", "n:0:a"), facadeView(state(index = 2), queue).items.map { it.uid })
    }

    @Test
    fun twoSongsStillHaveUniqueIds() {
        val ids = facadeView(state(index = 0), queue.take(2)).items.map { it.uid }
        assertEquals(ids.size, ids.toSet().size)
    }

    @Test
    fun stationIsOneLiveItem() {
        val v = facadeView(state(station = """{"id":"7","name":"SUB/WAVE"}"""), queue, subwaveTitle = "Now On Air")
        assertEquals(1, v.items.size)
        assertEquals("Now On Air", v.items[0].title)
        assertFalse(v.seekable)
    }

    @Test
    fun nothingWithoutAQueue() {
        assertTrue(facadeView(state(index = -1), emptyList()).items.isEmpty())
        assertTrue(facadeView(null, queue).items.isEmpty())
    }

    @Test
    fun theQueueSongIsTheIndexedOne() {
        assertEquals("b", queueSong(state(index = 1), queue)?.id)
        assertNull(queueSong(state(index = 7), queue))
    }

    @Test
    fun noQueueSongWhileAStationPlays() {
        // the core keeps the queue's index while a station plays; the station is what plays
        assertNull(queueSong(state(index = 1, station = """{"id":"7","name":"SUB/WAVE"}"""), queue))
    }

    private fun sync(following: String = "", listeners: List<String> = emptyList()) =
        SyncState(available = true, usersJson = "[]", listeners = listeners, following = following, canControl = true, allowControl = false, hidden = false)

    @Test
    fun keptAliveWhileFollowingOrListenedTo() {
        assertTrue(keepAlive(sync(following = "ali")))
        assertTrue(keepAlive(sync(listeners = listOf("veli"))))
        assertFalse(keepAlive(sync()))
        assertFalse(keepAlive(null))
    }
}
