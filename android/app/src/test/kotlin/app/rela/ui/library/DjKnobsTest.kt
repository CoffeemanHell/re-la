package app.rela.ui.library

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class DjKnobsTest {
    // the core's rela_core::dj::knobs::KEYS, in the same order: "back to defaults" resets exactly these
    @Test
    fun sameKeysAsTheCore() = assertEquals(
        listOf(
            "radioLanguage", "radioGenre", "radioExplore", "radioBatch", "djSkipShare", "djDislikeSkips", "djDislikeDays",
            "djMemoryDays", "djRepeatDays", "djArtistMax", "djWildcard", "djSkipMaxSec", "djFullShare", "djSearchSkips",
            "djSearchSec", "djIdleMin", "djSessionFade", "djNewDays", "djSimilar", "djTaste", "djRandomness",
        ),
        (djMain + djAdvanced).map { it.key },
    )

    // a slider lands on both ends only when its range is whole steps
    @Test
    fun rangesAreWholeSteps() = (djMain + djAdvanced).forEach { assertTrue(it.key, (it.to - it.from) % it.step == 0 && it.from < it.to) }
}
