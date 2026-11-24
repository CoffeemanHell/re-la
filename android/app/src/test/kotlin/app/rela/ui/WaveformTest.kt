package app.rela.ui

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Test

class WaveformTest {
    @Test
    fun eachBarIsTheLoudestOfItsRange() {
        assertArrayEquals(floatArrayOf(1f, 0.5f), bars(listOf(0f, 1f, 0.5f, 0.2f), 2), 0f)
    }

    @Test
    fun noBarsIsEmpty() {
        assertEquals(0, bars(List(4096) { 1f }, 0).size)
        assertEquals(0, bars(emptyList(), 10).size)
    }

    @Test
    fun moreBarsThanValuesRepeatValues() {
        assertArrayEquals(floatArrayOf(0.1f, 0.1f, 0.9f, 0.9f), bars(listOf(0.1f, 0.9f), 4), 0f)
    }
}
