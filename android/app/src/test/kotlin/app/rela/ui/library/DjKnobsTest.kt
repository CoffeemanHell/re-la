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
}
