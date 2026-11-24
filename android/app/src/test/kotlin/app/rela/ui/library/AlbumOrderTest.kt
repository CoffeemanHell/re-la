package app.rela.ui.library

import org.junit.Assert.assertEquals
import org.junit.Test

class AlbumOrderTest {
    @Test
    fun plainType() = assertEquals(mapOf("type" to "recent"), albumParams("recent", starred = false, descending = true))

    @Test
    fun starredWins() = assertEquals(mapOf("type" to "starred"), albumParams("byYear", starred = true, descending = false))

    @Test
    fun yearDirection() {
        assertEquals(mapOf("type" to "byYear", "fromYear" to 0, "toYear" to 9999), albumParams("byYear", starred = false, descending = false))
        assertEquals(mapOf("type" to "byYear", "fromYear" to 9999, "toYear" to 0), albumParams("byYear", starred = false, descending = true))
    }

    @Test
    fun everyOrderHasALabel() = assertEquals(8, albumOrders.toMap().size)
}
