package app.rela.playback

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.rela.DeckEnd
import uniffi.rela.DeckNote

class DeckLogicTest {
    private val seen = mutableListOf<DeckNote>()
    private val logic = DeckLogic { seen += it }

    @Test
    fun volumeFollowsMpvsCubicScale() {
        assertEquals(1f, exoVolume(100.0), 0f)
        assertEquals(0.125f, exoVolume(50.0), 1e-6f)
        assertEquals(0f, exoVolume(-3.0), 0f)
        assertEquals(1f, exoVolume(130.0), 0f)
    }

    @Test
    fun loadThenReadyGivesStartLoadedOnce() {
        logic.started()
        logic.ready(230_000)
        logic.ready(230_000)
        assertEquals(listOf(DeckNote.StartFile, DeckNote.Duration(230.0), DeckNote.FileLoaded), seen)
    }

    @Test
    fun gaplessMoveKeepsMpvsOrder() {
        logic.started()
        logic.ready(230_000)
        seen.clear()
        logic.gaplessMove(readyNow = true, durationMs = 251_000)
        assertEquals(
            listOf(DeckNote.EndFile(DeckEnd.EOF, ""), DeckNote.StartFile, DeckNote.Duration(251.0), DeckNote.FileLoaded),
            seen,
        )
    }

    @Test
    fun gaplessMoveIntoAnItemStillBuffering() {
        logic.gaplessMove(readyNow = false, durationMs = 0)
        logic.ready(251_000)
        assertEquals(DeckNote.FileLoaded, seen.last())
        assertEquals(1, seen.count { it == DeckNote.FileLoaded })
    }

    @Test
    fun errorEndsWithItsMessage() {
        logic.failed("Source error")
        assertEquals(listOf(DeckNote.EndFile(DeckEnd.ERROR, "Source error")), seen)
    }

    @Test
    fun currentItemFailingGoesOnWithTheAppendedOne() {
        // mpv: the current file ends with an error, the next starts; the core ignores that end while it has appended
        logic.started()
        logic.ready(230_000)
        seen.clear()
        logic.failedIntoNext("Source error")
        logic.ready(251_000)
        assertEquals(
            listOf(DeckNote.EndFile(DeckEnd.ERROR, "Source error"), DeckNote.StartFile, DeckNote.Duration(251.0), DeckNote.FileLoaded),
            seen,
        )
    }

    @Test
    fun appendedItemFailingReadsAsMpvsOrder() {
        // mpv: the current file ends, the next starts, then the next fails; the core then skips past it
        logic.started()
        logic.ready(230_000)
        seen.clear()
        logic.failedAhead("Source error")
        assertEquals(listOf(DeckNote.EndFile(DeckEnd.EOF, ""), DeckNote.StartFile, DeckNote.EndFile(DeckEnd.ERROR, "Source error")), seen)
    }

    @Test
    fun positionOnlyWhenItMoved() {
        logic.position(1000)
        logic.position(1004)
        logic.position(1020)
        logic.stopped()
        assertEquals(listOf(DeckNote.Position(1.0), DeckNote.Position(1.02), DeckNote.Position(null)), seen)
    }

    @Test
    fun whatTheTrackDoesNotSayIsCleared() {
        assertEquals(
            listOf(
                DeckNote.Prop("audio-params/format", "mp3"), DeckNote.Prop("audio-codec-name", "mp3"),
                DeckNote.Prop("audio-params/samplerate", "44100"), DeckNote.Prop("audio-params/channel-count", null), DeckNote.Bitrate(128_000.0),
            ),
            trackNotes("audio/mpeg", 44_100, -1, 128_000),
        )
        assertTrue(trackNotes(null, 0, 0, 0).all { it is DeckNote.Prop && it.value == null || it == DeckNote.Bitrate(null) })
    }

    @Test
    fun namesAsMpvSaysThem() {
        assertEquals("flac", codecName("audio/flac"))
        assertEquals("mp3", codecName("audio/mpeg"))
        assertEquals("s16", pcmName(2))
        assertEquals("float", pcmName(4))
    }
}
