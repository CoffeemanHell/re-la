//! This sitting's taste: tags of what was just heard (+) or left early (−), fading with each song. Two early skips in
//! 90 s mean the listener is looking for something else: until a song holds them, nothing goes to long memory and the
//! DJ explores more. The song that holds them becomes the new direction.

use crate::dj::genres::Genres;
use crate::dj::index::{Index, Song};
use crate::dj::listen::Heard;
use std::collections::HashMap;

const FADE: f32 = 0.7;
const SEARCH_WINDOW_MS: f64 = 90_000.0;
const IDLE_MS: f64 = 30.0 * 60_000.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Tag {
    Genre(u32),
    Artist(u32),
    Decade(u16),
    Lang(u32),
    Mood(u32),
    Bpm(u8), // bpm / 10
}

pub fn tags(s: &Song, index: &Index) -> Vec<Tag> {
    let mut t: Vec<Tag> = s.genres.iter().map(|&g| Tag::Genre(g)).collect();
    t.extend(s.artists.iter().map(|&a| Tag::Artist(a)));
    t.extend(s.moods.iter().map(|&m| Tag::Mood(m)));
    if s.year > 0 {
        t.push(Tag::Decade(s.year / 10));
    }
    if s.bpm > 0 {
        t.push(Tag::Bpm((s.bpm / 10).min(255) as u8));
    }
    match index.lang(s) {
        0 => {}
        l => t.push(Tag::Lang(l)),
    }
    t
}

#[derive(Default)]
pub struct Session {
    w: HashMap<Tag, f32>,
    skips: Vec<f64>, // early skips, clock ms
    search: bool,
    last_ms: f64,
}

impl Session {
    pub fn weight(&self, t: Tag) -> f32 {
        self.w.get(&t).copied().unwrap_or(0.0)
    }

    pub fn is_empty(&self) -> bool {
        self.w.is_empty()
    }

    /// Two early skips within 90 s turn it on; a song that holds the listener (or 30 min of nothing) turns it off.
    pub fn searching(&self, _at_ms: f64) -> bool {
        self.search
    }

    fn add(&mut self, s: &Song, v: f32, genres: &Genres, index: &Index) {
        for t in tags(s, index) {
            *self.w.entry(t).or_default() += v;
            if let Tag::Genre(g) = t {
                for (n, sim) in genres.neighbors(index.word(g)) {
                    if let Some(id) = index.word_id(n) {
                        *self.w.entry(Tag::Genre(id)).or_default() += v * sim;
                    }
                }
            }
        }
    }

    /// A song ended this way at `ratio` of it. True when it caught a searching listener: the DJ's upcoming picks
    /// should make way for ones like it.
    pub fn hear(&mut self, s: &Song, h: Heard, at_ms: f64, ratio: f64, genres: &Genres, index: &Index) -> bool {
        if self.last_ms > 0.0 && at_ms - self.last_ms > IDLE_MS {
            *self = Session::default();
        }
        self.last_ms = at_ms;
        if self.search && (h == Heard::Full || ratio >= 0.5) {
            self.w.clear();
            self.skips.clear();
            self.search = false;
            self.add(s, 3.0, genres, index);
            return true;
        }
        self.w.values_mut().for_each(|v| *v *= FADE);
        let v = match h {
            Heard::Full => 1.0,
            Heard::EarlySkip => -1.0,
            Heard::Neutral => 0.0,
        };
        if v != 0.0 {
            self.add(s, v, genres, index);
        }
        if h == Heard::EarlySkip {
            self.skips.retain(|&t| at_ms - t <= SEARCH_WINDOW_MS);
            self.skips.push(at_ms);
            if self.skips.len() >= 2 {
                self.search = true;
            }
        }
        false
    }

    /// Starts from the last songs when nothing was heard yet this sitting (oldest first).
    pub fn seed(&mut self, recent: &[&Song], genres: &Genres, index: &Index) {
        self.w.clear();
        for s in recent {
            self.w.values_mut().for_each(|v| *v *= FADE);
            self.add(s, 1.0, genres, index);
        }
    }

    /// How well a song fits this sitting, -1..1.
    pub fn fit(&self, s: &Song, index: &Index) -> f32 {
        tags(s, index).iter().map(|t| self.weight(*t)).sum::<f32>().tanh()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dj::genres::UserGenres;
    use crate::dj::listen::Heard::*;
    use serde_json::json;

    fn setup() -> (Index, Genres) {
        let p = std::env::temp_dir().join(format!("re-la-djses-{}-{}", std::process::id(), fastrand::u64(..)));
        let mut ix = Index::open(p.join("i.json"));
        for (id, artist, genre) in [("a", "Qa", "Qqa"), ("b", "Qb", "Qqb"), ("hm", "Iron", "Heavy Metal"), ("hr", "Deep", "Hard Rock")] {
            ix.upsert(&json!({"id": id, "artist": artist, "genre": genre}), 1);
        }
        let g = Genres::build(&ix, &[], &UserGenres::default());
        (ix, g)
    }

    fn genre(ix: &Index, name: &str) -> Tag {
        Tag::Genre(ix.word_id(name).unwrap())
    }

    #[test]
    fn fade() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), Full, 1000.0, 1.0, &g, &ix);
        assert!((s.weight(genre(&ix, "Qqa")) - 1.0).abs() < 1e-5);
        s.hear(ix.get("b").unwrap(), Full, 2000.0, 1.0, &g, &ix);
        assert!((s.weight(genre(&ix, "Qqa")) - 0.7).abs() < 1e-5);
        assert!(s.fit(ix.get("b").unwrap(), &ix) > s.fit(ix.get("a").unwrap(), &ix));
    }

    #[test]
    fn search_mode() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1000.0, 0.1, &g, &ix);
        s.hear(ix.get("b").unwrap(), EarlySkip, 60_000.0, 0.1, &g, &ix);
        assert!(s.searching(60_000.0));
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1000.0, 0.1, &g, &ix);
        s.hear(ix.get("b").unwrap(), EarlySkip, 101_000.0, 0.1, &g, &ix);
        assert!(!s.searching(101_000.0));
    }

    #[test]
    fn catch() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1000.0, 0.1, &g, &ix);
        s.hear(ix.get("a").unwrap(), EarlySkip, 2000.0, 0.1, &g, &ix);
        assert!(!s.hear(ix.get("hm").unwrap(), Neutral, 3000.0, 0.4, &g, &ix), "not held yet");
        assert!(s.hear(ix.get("b").unwrap(), Neutral, 4000.0, 0.5, &g, &ix));
        assert!(!s.searching(4000.0));
        assert!((s.weight(genre(&ix, "Qqb")) - 3.0).abs() < 1e-5);
        assert_eq!(s.weight(genre(&ix, "Qqa")), 0.0);
    }

    #[test]
    fn genre_spread() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("hm").unwrap(), Full, 1000.0, 1.0, &g, &ix);
        assert!(s.weight(genre(&ix, "Hard Rock")) > 0.0);
    }

    #[test]
    fn idle_reset() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), Full, 1000.0, 1.0, &g, &ix);
        s.hear(ix.get("b").unwrap(), Full, 1000.0 + 31.0 * 60_000.0, 1.0, &g, &ix);
        assert_eq!(s.weight(genre(&ix, "Qqa")), 0.0);
        assert!(!s.is_empty());
    }

    #[test]
    fn seed_from_recent() {
        let (ix, g) = setup();
        let mut s = Session::default();
        assert!(s.is_empty());
        s.seed(&[ix.get("a").unwrap(), ix.get("b").unwrap()], &g, &ix);
        assert!(s.weight(genre(&ix, "Qqb")) > s.weight(genre(&ix, "Qqa")) && s.weight(genre(&ix, "Qqa")) > 0.0);
    }
}
