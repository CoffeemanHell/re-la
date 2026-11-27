//! Every number the DJ goes by, read from the options (the settings windows write them). The defaults are the ones the
//! DJ was built with.

use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct Knobs {
    /// % of each batch in the seed's language (when one is known)
    pub language: i32,
    /// how strongly to stay close to what plays, 0-100
    pub genre: i32,
    /// % of each batch kept for new or rarely played songs
    pub explore: i32,
    pub batch: i32,
    /// leaving before this share of a song (and before skip_max_sec) counts against it
    pub skip_share: f64,
    pub skip_max_sec: f64,
    /// from this share on, a song counts as heard in full
    pub full_share: f64,
    /// faded early skips, on dislike_days different days at least, make a song disliked
    pub dislike_skips: f32,
    pub dislike_days: u8,
    /// half-life of skips and genre pairs
    pub memory_days: f32,
    /// a song heard this many days ago or sooner is held back (more so in the first half)
    pub repeat_days: u32,
    /// songs per artist counting the last 10 and those coming up
    pub artist_max: i32,
    /// share of each batch drawn evenly
    pub wildcard: f64,
    /// this many early skips within search_ms: the listener is looking for something else
    pub search_skips: usize,
    pub search_ms: f64,
    /// a sitting ends after this long without a song ending
    pub idle_ms: f64,
    /// share of the sitting's taste left after each song
    pub fade: f32,
    /// a song this many days in the library is new
    pub new_days: u32,
    pub similar: f64,
    pub taste: f64,
    /// softmax temperature of the draw
    pub temperature: f64,
}

impl Default for Knobs {
    fn default() -> Knobs {
        Knobs {
            language: 80,
            genre: 50,
            explore: 20,
            batch: 10,
            skip_share: 0.3,
            skip_max_sec: 180.0,
            full_share: 0.9,
            dislike_skips: 3.0,
            dislike_days: 2,
            memory_days: 60.0,
            repeat_days: 7,
            artist_max: 2,
            wildcard: 0.1,
            search_skips: 2,
            search_ms: 90_000.0,
            idle_ms: 30.0 * 60_000.0,
            fade: 0.7,
            new_days: 30,
            similar: 0.8,
            taste: 0.5,
            temperature: 0.5,
        }
    }
}

/// (key, default, min, max) of the DJ's own options; radioLanguage/radioGenre/radioExplore/radioBatch are older.
pub const RANGES: &[(&str, i64, i64, i64)] = &[
    ("djSkipShare", 30, 10, 60),
    ("djDislikeSkips", 3, 1, 10),
    ("djDislikeDays", 2, 1, 5),
    ("djMemoryDays", 60, 7, 364), // 52 weeks: whole steps of 7
    ("djRepeatDays", 7, 0, 30),
    ("djArtistMax", 2, 1, 5),
    ("djWildcard", 10, 0, 30),
    ("djSkipMaxSec", 180, 30, 600),
    ("djFullShare", 90, 60, 100),
    ("djSearchSkips", 2, 2, 5),
    ("djSearchSec", 90, 30, 300),
    ("djIdleMin", 30, 5, 240),
    ("djSessionFade", 70, 30, 95),
    ("djNewDays", 30, 0, 180),
    ("djSimilar", 80, 0, 200),
    ("djTaste", 50, 0, 200),
    ("djRandomness", 50, 10, 200),
];

/// The DJ options the settings windows show, older ones first; "back to defaults" resets these.
pub const KEYS: &[&str] = &[
    "radioLanguage", "radioGenre", "radioExplore", "radioBatch", "djSkipShare", "djDislikeSkips", "djDislikeDays",
    "djMemoryDays", "djRepeatDays", "djArtistMax", "djWildcard", "djSkipMaxSec", "djFullShare", "djSearchSkips",
    "djSearchSec", "djIdleMin", "djSessionFade", "djNewDays", "djSimilar", "djTaste", "djRandomness",
];

impl Knobs {
    /// From the options; a missing or non-integer value takes the default, one out of range the nearest end.
    pub fn from_options(o: &Map<String, Value>) -> Knobs {
        let get = |k: &str, def: i64, lo: i64, hi: i64| o.get(k).and_then(Value::as_i64).unwrap_or(def).clamp(lo, hi);
        let dj = |k: &str| {
            let (_, def, lo, hi) = RANGES.iter().find(|r| r.0 == k).copied().unwrap_or((k, 0, 0, 0));
            get(k, def, lo, hi)
        };
        let pct = |k: &str| dj(k) as f64 / 100.0;
        Knobs {
            language: get("radioLanguage", 80, 0, 100) as i32,
            genre: get("radioGenre", 50, 0, 100) as i32,
            explore: get("radioExplore", 20, 0, 50) as i32,
            batch: get("radioBatch", 10, 1, 50) as i32,
            skip_share: pct("djSkipShare"),
            skip_max_sec: dj("djSkipMaxSec") as f64,
            full_share: pct("djFullShare"),
            dislike_skips: dj("djDislikeSkips") as f32,
            dislike_days: dj("djDislikeDays") as u8,
            memory_days: dj("djMemoryDays") as f32,
            repeat_days: dj("djRepeatDays") as u32,
            artist_max: dj("djArtistMax") as i32,
            wildcard: pct("djWildcard"),
            search_skips: dj("djSearchSkips") as usize,
            search_ms: dj("djSearchSec") as f64 * 1000.0,
            idle_ms: dj("djIdleMin") as f64 * 60_000.0,
            fade: pct("djSessionFade") as f32,
            new_days: dj("djNewDays") as u32,
            similar: pct("djSimilar"),
            taste: pct("djTaste"),
            temperature: pct("djRandomness"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_match_constants() {
        let o = crate::player::default_options();
        assert_eq!(Knobs::from_options(&o), Knobs::default());
        assert_eq!(Knobs::from_options(&Map::new()), Knobs::default());
        for (k, def, ..) in RANGES {
            assert_eq!(o.get(*k).and_then(Value::as_i64), Some(*def), "{k} in the default options");
        }
    }

    #[test]
    fn bad_values_fall_back() {
        let o = json!({"djSkipShare": "abc", "djArtistMax": -5, "djMemoryDays": 1000});
        let k = Knobs::from_options(o.as_object().unwrap());
        assert_eq!(k.skip_share, 0.3);
        assert_eq!(k.artist_max, 1);
        assert_eq!(k.memory_days, 364.0);
    }
}
