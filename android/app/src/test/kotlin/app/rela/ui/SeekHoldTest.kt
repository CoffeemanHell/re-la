package app.rela.ui

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SeekHoldTest {
    private val h = SeekHold(to = 99.0, atMs = 1_000, song = "a")

    @Test
    fun holdsWhileTheCoreStillReportsTheOldPlace() = assertTrue(stillHeld(h, 130.0, "a", 1_016))

    @Test
    fun endsWhenTheCoreHasIt() = assertFalse(stillHeld(h, 99.4, "a", 1_200))

    @Test
    fun endsAfterThreeSeconds() = assertFalse(stillHeld(h, 130.0, "a", 4_000))

    @Test
    fun endsWithTheSong() = assertFalse(stillHeld(h, 2.0, "b", 1_016))
}
