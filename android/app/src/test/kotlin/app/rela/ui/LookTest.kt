package app.rela.ui

import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color
import org.junit.Assert.assertEquals
import org.junit.Test

class LookTest {
    @Test
    fun defaultsWithoutOptions() = assertEquals(Look(), look("{}"))

    @Test
    fun readsTheOptions() {
        val l = look("""{"amoled":true,"gridColumns":3,"coverShape":"circle","albumView":"list","albumOrder":"recent"}""")
        assertEquals(Look(amoled = true, gridColumns = 3, coverShape = "circle", albumView = "list", albumOrder = "recent"), l)
    }

    @Test
    fun badValuesFallBackToDefaults() {
        assertEquals(2, look("""{"gridColumns":"abc"}""").gridColumns)
        assertEquals(4, look("""{"gridColumns":9}""").gridColumns)
        assertEquals(3, look("""{"gridColumns":3.0}""").gridColumns)
        assertEquals("soft", look("""{"coverShape":"star"}""").coverShape)
        assertEquals("grid", look("""{"albumView":"table"}""").albumView)
        assertEquals(Look(), look("not json"))
    }

    @Test
    fun amoledOnlyBlackensDark() {
        val dark = darkColorScheme()
        val light = lightColorScheme()
        val black = relaScheme(dark, dark = true, amoled = true)
        assertEquals(Color.Black, black.background)
        assertEquals(Color.Black, black.surface)
        assertEquals(dark.surfaceContainer, black.surfaceContainer) // cards stay visible on black
        assertEquals(light, relaScheme(light, dark = false, amoled = true))
        assertEquals(dark, relaScheme(dark, dark = true, amoled = false))
    }

    @Test
    fun readsTheNewOptions() {
        val l = look("""{"coverTap":"queue","tabs":"radio,library","coverQuality":"low"}""")
        assertEquals("queue", l.coverTap)
        assertEquals(listOf("library", "radio"), l.tabs)
        assertEquals("low", l.coverQuality)
    }

    @Test
    fun badNewValuesFallBack() {
        assertEquals("lyrics", look("""{"coverTap":"x"}""").coverTap)
        assertEquals("high", look("""{"coverQuality":"x"}""").coverQuality)
        assertEquals(tabIds, look("{}").tabs)
        assertEquals(listOf("library"), look("""{"tabs":""}""").tabs)
        assertEquals(listOf("library"), look("""{"tabs":"foo"}""").tabs)
        assertEquals(listOf("library", "radio"), look("""{"tabs":"radio,foo"}""").tabs)
    }

    @Test
    fun coverPxScales() {
        assertEquals(50, coverPx(100, "low"))
        assertEquals(75, coverPx(100, "medium"))
        assertEquals(100, coverPx(100, "high"))
        assertEquals(1, coverPx(1, "low"))
    }
}
