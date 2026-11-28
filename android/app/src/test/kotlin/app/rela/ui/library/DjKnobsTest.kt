package app.rela.ui.library

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class DjKnobsTest {
    // the core's rela_core::dj::knobs::KEYS (the knobs and the five switches): "back to defaults" resets exactly these
    @Test
    fun sameKeysAsTheCore() = assertEquals(
        listOf(
            "radioLanguage", "radioGenre", "radioExplore", "radioBatch", "djSkipShare", "djDislikeSkips", "djDislikeDays",
            "djMemoryDays", "djRepeatDays", "djArtistMax", "djWildcard", "djSkipMaxSec", "djFullShare", "djSearchSkips",
            "djSearchSec", "djIdleMin", "djSessionFade", "djNewDays", "djSimilar", "djTaste", "djRandomness",
            "djSkipOn", "djSearchOn", "djDislikeOn", "djRepeatOn", "djWildcardOn",
        ).sorted(),
        (djKnobs.map { it.key } + djSwitches.map { it.key }).sorted(),
    )

    // a slider lands on both ends only when its range is whole steps
    @Test
    fun rangesAreWholeSteps() = djKnobs.forEach { assertTrue(it.key, (it.to - it.from) % it.step == 0 && it.from < it.to) }

    // a stored value that is no preset (63 days) has no index: the page shows no preset as chosen
    @Test
    fun presetIndexFindsThePreset() {
        val presets = listOf(14, 30, 60, 91, 182, 364)
        assertEquals(2, presetIndex(presets, 60))
        assertEquals(-1, presetIndex(presets, 63))
    }

    // a number box moves one step and stays in its range
    @Test
    fun nudgedStepsWithinTheRange() {
        val k = djKnobs.first { it.key == "djSearchSec" } // 30..300 by 10
        assertEquals(100, nudged(k, 90, 1))
        assertEquals(300, nudged(k, 300, 1))
        assertEquals(30, nudged(k, 30, -1))
    }
}
