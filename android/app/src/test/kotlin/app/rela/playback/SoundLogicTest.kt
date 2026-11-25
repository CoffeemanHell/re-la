package app.rela.playback

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SoundLogicTest {
    private val seen = mutableListOf<String>()
    private val sound = SoundLogic(pause = { seen += "pause" }, mute = { seen += "mute $it" })

    private fun lost(passing: Boolean, following: Boolean) = sound.lost(passing, following) { seen += "resume" }

    @Test
    fun aCallPausesAndResumes() {
        lost(passing = true, following = false)
        assertTrue(sound.waiting)
        assertTrue(sound.gained(playing = false, following = false))
        assertEquals(listOf("pause", "resume"), seen)
        assertFalse(sound.waiting)
    }

    @Test
    fun playedByHandMeanwhileNothingIsOwed() {
        lost(passing = true, following = false)
        sound.playing(focus = true)
        assertTrue(sound.gained(playing = true, following = false))
        assertFalse(sound.gained(playing = false, following = false))
        assertEquals(listOf("pause"), seen)
    }

    @Test
    fun aPausedRootIsNotStarted() {
        lost(passing = true, following = false)
        assertFalse(sound.gained(playing = false, following = true))
        assertEquals(listOf("pause"), seen)
    }

    @Test
    fun aFollowerIsMutedDuringACall() {
        lost(passing = true, following = true)
        sound.playing(focus = false) // the root paused and played: the call still has the focus
        assertEquals(listOf("mute true"), seen)
        assertTrue(sound.waiting)
        assertTrue(sound.gained(playing = true, following = true))
        assertEquals(listOf("mute true", "mute false"), seen)
    }

    @Test
    fun aRootPausedDuringTheCallLeavesSilence() {
        lost(passing = true, following = true)
        assertFalse(sound.gained(playing = false, following = true))
        assertEquals(listOf("mute true", "mute false"), seen)
    }

    @Test
    fun headphonesOutPausesForGood() {
        lost(passing = false, following = true)
        assertFalse(sound.waiting)
        assertFalse(sound.gained(playing = false, following = false))
        assertEquals(listOf("pause"), seen)
    }

    @Test
    fun headphonesOutDuringACallStaysSilentUntilPlayed() {
        lost(passing = true, following = true)
        lost(passing = false, following = true)
        sound.playing(focus = true)
        assertEquals(listOf("mute true", "pause", "mute false"), seen)
    }
}
