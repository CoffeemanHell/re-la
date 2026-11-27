package app.rela.ui.library

import app.rela.R
import app.rela.plural
import app.rela.str

/** A DJ setting: option key (the core's rela_core::dj::knobs keeps its default), label, hint, range and unit. */
data class Knob(val key: String, val label: Int, val hint: Int, val unit: Unit, val from: Int, val to: Int, val step: Int) {
    enum class Unit { PERCENT, DAYS, SECONDS, MINUTES, SONGS, NUMBER }
}

/** The DJ settings shown first; [djAdvanced] sits under a fold. */
val djMain = listOf(
    Knob("radioLanguage", R.string.language_affinity, R.string.dj_language_hint, Knob.Unit.PERCENT, 0, 100, 10),
    Knob("radioGenre", R.string.genre_affinity, R.string.genre_affinity_hint, Knob.Unit.PERCENT, 0, 100, 10),
    Knob("radioExplore", R.string.discovery, R.string.discovery_hint, Knob.Unit.PERCENT, 0, 50, 10),
    Knob("radioBatch", R.string.batch, 0, Knob.Unit.SONGS, 5, 30, 5),
    Knob("djSkipShare", R.string.dj_skip_share, R.string.dj_skip_share_hint, Knob.Unit.PERCENT, 10, 60, 5),
    Knob("djDislikeSkips", R.string.dj_dislike_skips, R.string.dj_dislike_skips_hint, Knob.Unit.NUMBER, 1, 10, 1),
    Knob("djDislikeDays", R.string.dj_dislike_days, R.string.dj_dislike_days_hint, Knob.Unit.DAYS, 1, 5, 1),
    Knob("djMemoryDays", R.string.dj_memory_days, R.string.dj_memory_days_hint, Knob.Unit.DAYS, 7, 364, 7),
    Knob("djRepeatDays", R.string.dj_repeat_days, R.string.dj_repeat_days_hint, Knob.Unit.DAYS, 0, 30, 1),
    Knob("djArtistMax", R.string.dj_artist_max, R.string.dj_artist_max_hint, Knob.Unit.SONGS, 1, 5, 1),
    Knob("djWildcard", R.string.dj_wildcard, R.string.dj_wildcard_hint, Knob.Unit.PERCENT, 0, 30, 5),
)

val djAdvanced = listOf(
    Knob("djSkipMaxSec", R.string.dj_skip_max_sec, R.string.dj_skip_max_sec_hint, Knob.Unit.SECONDS, 30, 600, 30),
    Knob("djFullShare", R.string.dj_full_share, R.string.dj_full_share_hint, Knob.Unit.PERCENT, 60, 100, 5),
    Knob("djSearchSkips", R.string.dj_search_skips, R.string.dj_search_skips_hint, Knob.Unit.NUMBER, 2, 5, 1),
    Knob("djSearchSec", R.string.dj_search_sec, R.string.dj_search_sec_hint, Knob.Unit.SECONDS, 30, 300, 10),
    Knob("djIdleMin", R.string.dj_idle_min, R.string.dj_idle_min_hint, Knob.Unit.MINUTES, 5, 240, 5),
    Knob("djSessionFade", R.string.dj_session_fade, R.string.dj_session_fade_hint, Knob.Unit.PERCENT, 30, 95, 5),
    Knob("djNewDays", R.string.dj_new_days, R.string.dj_new_days_hint, Knob.Unit.DAYS, 0, 180, 5),
    Knob("djSimilar", R.string.dj_similar, R.string.dj_similar_hint, Knob.Unit.NUMBER, 0, 200, 10),
    Knob("djTaste", R.string.dj_taste, R.string.dj_taste_hint, Knob.Unit.NUMBER, 0, 200, 10),
    Knob("djRandomness", R.string.dj_randomness, R.string.dj_randomness_hint, Knob.Unit.NUMBER, 10, 200, 10),
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
