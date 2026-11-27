package app.rela.ui.library

import app.rela.R
import app.rela.plural
import app.rela.str

/**
 * A DJ setting: option key (the core's rela_core::dj::knobs keeps its default), label, hint, range, unit, default, the
 * settings page it sits on, its control kind, and the switch above it ([under]) when it has one.
 */
data class Knob(
    val key: String, val label: Int, val hint: Int, val unit: Unit, val from: Int, val to: Int, val step: Int,
    val def: Int, val group: Group, val kind: Kind, val under: String? = null, val presets: List<Int> = emptyList(),
) {
    enum class Unit { PERCENT, DAYS, SECONDS, MINUTES, SONGS, NUMBER }
    enum class Group { PICK, SKIPS, MEMORY, FINE }
    enum class Kind { SLIDER, SEG, SPIN, PRESET }
}

/** An on/off option (default on) that heads its page; the knobs under it hide while it is off. */
data class DjSwitch(val key: String, val label: Int, val hint: Int, val group: Knob.Group)

/** The DJ settings in page order; the switches are in [djSwitches]. Every default is checked against the core. */
val djKnobs = listOf(
    Knob("radioLanguage", R.string.language_affinity, R.string.dj_language_hint, Knob.Unit.PERCENT, 0, 100, 10, def = 80, group = Knob.Group.PICK, kind = Knob.Kind.SLIDER),
    Knob("radioGenre", R.string.genre_affinity, R.string.genre_affinity_hint, Knob.Unit.PERCENT, 0, 100, 10, def = 50, group = Knob.Group.PICK, kind = Knob.Kind.SLIDER),
    Knob("radioExplore", R.string.discovery, R.string.discovery_hint, Knob.Unit.PERCENT, 0, 50, 10, def = 20, group = Knob.Group.PICK, kind = Knob.Kind.SLIDER),
    Knob("djWildcard", R.string.dj_wildcard, R.string.dj_wildcard_hint, Knob.Unit.PERCENT, 5, 30, 5, def = 10, group = Knob.Group.PICK, kind = Knob.Kind.SLIDER, under = "djWildcardOn"),
    Knob("radioBatch", R.string.batch, 0, Knob.Unit.SONGS, 5, 30, 5, def = 10, group = Knob.Group.PICK, kind = Knob.Kind.SEG),
    Knob("djArtistMax", R.string.dj_artist_max, R.string.dj_artist_max_hint, Knob.Unit.SONGS, 1, 5, 1, def = 2, group = Knob.Group.PICK, kind = Knob.Kind.SEG),

    Knob("djSkipShare", R.string.dj_skip_share, R.string.dj_skip_share_hint, Knob.Unit.PERCENT, 10, 60, 5, def = 30, group = Knob.Group.SKIPS, kind = Knob.Kind.SLIDER, under = "djSkipOn"),
    Knob("djDislikeSkips", R.string.dj_dislike_skips, R.string.dj_dislike_skips_hint, Knob.Unit.NUMBER, 1, 10, 1, def = 3, group = Knob.Group.SKIPS, kind = Knob.Kind.SPIN, under = "djDislikeOn"),
    Knob("djDislikeDays", R.string.dj_dislike_days, R.string.dj_dislike_days_hint, Knob.Unit.DAYS, 1, 5, 1, def = 2, group = Knob.Group.SKIPS, kind = Knob.Kind.SEG, under = "djDislikeOn"),
    Knob("djSearchSkips", R.string.dj_search_skips, R.string.dj_search_skips_hint, Knob.Unit.NUMBER, 2, 5, 1, def = 2, group = Knob.Group.SKIPS, kind = Knob.Kind.SEG, under = "djSearchOn"),
    Knob("djSearchSec", R.string.dj_search_sec, R.string.dj_search_sec_hint, Knob.Unit.SECONDS, 30, 300, 10, def = 90, group = Knob.Group.SKIPS, kind = Knob.Kind.SPIN, under = "djSearchOn"),

    Knob("djRepeatDays", R.string.dj_repeat_days, R.string.dj_repeat_days_hint, Knob.Unit.DAYS, 1, 30, 1, def = 7, group = Knob.Group.MEMORY, kind = Knob.Kind.SPIN, under = "djRepeatOn"),
    Knob("djMemoryDays", R.string.dj_memory_days, R.string.dj_memory_days_hint, Knob.Unit.DAYS, 7, 364, 7, def = 60, group = Knob.Group.MEMORY, kind = Knob.Kind.PRESET,
        presets = listOf(14, 30, 60, 91, 182, 364)),
    Knob("djIdleMin", R.string.dj_idle_min, R.string.dj_idle_min_hint, Knob.Unit.MINUTES, 5, 240, 5, def = 30, group = Knob.Group.MEMORY, kind = Knob.Kind.SPIN),

    Knob("djSkipMaxSec", R.string.dj_skip_max_sec, R.string.dj_skip_max_sec_hint, Knob.Unit.SECONDS, 30, 600, 30, def = 180, group = Knob.Group.FINE, kind = Knob.Kind.SPIN),
    Knob("djFullShare", R.string.dj_full_share, R.string.dj_full_share_hint, Knob.Unit.PERCENT, 60, 100, 5, def = 90, group = Knob.Group.FINE, kind = Knob.Kind.SLIDER),
    Knob("djSessionFade", R.string.dj_session_fade, R.string.dj_session_fade_hint, Knob.Unit.PERCENT, 30, 95, 5, def = 70, group = Knob.Group.FINE, kind = Knob.Kind.SLIDER),
    Knob("djNewDays", R.string.dj_new_days, R.string.dj_new_days_hint, Knob.Unit.DAYS, 0, 180, 5, def = 30, group = Knob.Group.FINE, kind = Knob.Kind.SPIN),
    Knob("djSimilar", R.string.dj_similar, R.string.dj_similar_hint, Knob.Unit.NUMBER, 0, 200, 10, def = 80, group = Knob.Group.FINE, kind = Knob.Kind.SLIDER),
    Knob("djTaste", R.string.dj_taste, R.string.dj_taste_hint, Knob.Unit.NUMBER, 0, 200, 10, def = 50, group = Knob.Group.FINE, kind = Knob.Kind.SLIDER),
    Knob("djRandomness", R.string.dj_randomness, R.string.dj_randomness_hint, Knob.Unit.NUMBER, 10, 200, 10, def = 50, group = Knob.Group.FINE, kind = Knob.Kind.SLIDER),
)

val djSwitches = listOf(
    DjSwitch("djWildcardOn", R.string.dj_wildcard, R.string.dj_wildcard_on_hint, Knob.Group.PICK),
    DjSwitch("djSkipOn", R.string.dj_skip_on, R.string.dj_skip_on_hint, Knob.Group.SKIPS),
    DjSwitch("djDislikeOn", R.string.dj_dislike_on, R.string.dj_dislike_on_hint, Knob.Group.SKIPS),
    DjSwitch("djSearchOn", R.string.dj_search_on, R.string.dj_search_skips_hint, Knob.Group.SKIPS),
    DjSwitch("djRepeatOn", R.string.dj_repeat_days, R.string.dj_repeat_on_hint, Knob.Group.MEMORY),
)

/** A knob's value as the settings show it. */
fun knobText(k: Knob, v: Int): String = when (k.unit) {
    Knob.Unit.PERCENT -> str(R.string.percent, v)
    Knob.Unit.DAYS -> plural(R.plurals.n_days, v)
    Knob.Unit.SECONDS -> plural(R.plurals.n_seconds, v)
    Knob.Unit.MINUTES -> plural(R.plurals.n_minutes, v)
    Knob.Unit.SONGS -> plural(R.plurals.n_songs, v)
    Knob.Unit.NUMBER -> "$v"
}
