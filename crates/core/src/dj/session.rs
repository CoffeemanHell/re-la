//! This sitting's taste: tags of what was just heard (+) or left early (−), fading with each song. A few early skips
//! close together (`Knobs::search_skips` in `search_ms`) mean the listener is looking for something else: until a song
//! holds them, nothing goes to long memory and the DJ explores more. The song that holds them becomes the new direction.

use crate::dj::Knobs;
use crate::dj::genres::Genres;
use crate::dj::index::{Index, Song};
use crate::dj::listen::Heard;
use std::collections::HashMap;

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

#[derive(Default, Clone, PartialEq)]
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

    /// Early skips close together turn it on; a song that holds the listener (or a long while of nothing) turns it off.
    pub fn searching(&self, at_ms: f64, k: &Knobs) -> bool {
        self.search && k.search_skips != usize::MAX && !self.idle(at_ms, k) // MAX: the search switch is off
    }

    /// The search switch went off: no search, and no skips kept towards one.
    pub fn end_search(&mut self) {
        self.search = false;
        self.skips.clear();
    }

    fn idle(&self, at_ms: f64, k: &Knobs) -> bool {
        self.last_ms > 0.0 && at_ms - self.last_ms > k.idle_ms
    }

    fn add(&mut self, s: &Song, v: f32, genres: &Genres, index: &Index) {
        for t in tags(s, index) {
            *self.w.entry(t).or_default() += v;
            if let Tag::Genre(g) = t {
                for (n, sim) in genres.neighbors(index.word(g)) {
                    if let Some(id) = index.word_id(n).filter(|&id| id != g) {
                        *self.w.entry(Tag::Genre(id)).or_default() += v * sim;
                    }
                }
            }
        }
    }

    /// A strong push towards (v > 0) or away from (v < 0) songs like this one: "more / less like this".
    pub fn nudge(&mut self, s: &Song, v: f32, genres: &Genres, index: &Index) {
        self.add(s, v, genres, index);
    }

    /// A song ended this way at `ratio` of it. True when it caught a searching listener: the DJ's upcoming picks
    /// should make way for ones like it.
    #[allow(clippy::too_many_arguments)]
    pub fn hear(&mut self, s: &Song, h: Heard, at_ms: f64, ratio: f64, genres: &Genres, index: &Index, k: &Knobs) -> bool {
        if self.idle(at_ms, k) {
            *self = Session::default();
        }
        if k.search_skips == usize::MAX {
            self.end_search(); // the search switch is off
        }
        self.last_ms = at_ms;
        if self.search && (h == Heard::Full || ratio >= 0.5) {
            self.w.clear();
            self.skips.clear();
            self.search = false;
            self.add(s, 3.0, genres, index);
            return true;
        }
        self.w.values_mut().for_each(|v| *v *= k.fade);
        let v = match h {
            Heard::Full => 1.0,
            Heard::EarlySkip => -1.0,
            Heard::Neutral => 0.0,
        };
        if v != 0.0 {
            self.add(s, v, genres, index);
        }
        if h == Heard::EarlySkip && k.search_skips != usize::MAX {
            self.skips.retain(|&t| (0.0..=k.search_ms).contains(&(at_ms - t))); // the clock may step back
            self.skips.push(at_ms);
            if self.skips.len() >= k.search_skips {
                self.search = true;
            }
        }
        false
    }

    /// Starts from the last songs when nothing was heard yet this sitting (oldest first).
    pub fn seed(&mut self, recent: &[&Song], genres: &Genres, index: &Index, k: &Knobs) {
        self.w.clear();
        for s in recent {
            self.w.values_mut().for_each(|v| *v *= k.fade);
            self.add(s, 1.0, genres, index);
        }
    }

    /// How well a song fits this sitting, -1..1.
    pub fn fit(&self, s: &Song, index: &Index) -> f32 {
        self.fit_best(s, index).0
    }

    /// The fit, and the tag that helps it most that a listener would name (not a BPM or mood).
    pub fn fit_best(&self, s: &Song, index: &Index) -> (f32, Option<Tag>) {
        let (mut sum, mut best) = (0.0f32, None::<(f32, Tag)>);
        for t in tags(s, index) {
            let w = self.weight(t);
            sum += w;
            if w > 0.0 && !matches!(t, Tag::Bpm(_) | Tag::Mood(_)) && best.is_none_or(|(b, _)| w > b) {
                best = Some((w, t));
            }
        }
        (sum.tanh(), best.map(|(_, t)| t))
    }

    /// The strongest tags of this sitting, strongest first.
    pub fn top(&self, n: usize) -> Vec<(Tag, f32)> {
        let mut v: Vec<(Tag, f32)> = self.w.iter().map(|(t, w)| (*t, *w)).filter(|(_, w)| *w != 0.0).collect();
        v.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
        v.truncate(n);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dj::genres::UserGenres;
    use crate::dj::listen::Heard::*;
    use serde_json::json;

    fn kn() -> Knobs {
        Knobs::default()
    }

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
        s.hear(ix.get("a").unwrap(), Full, 1000.0, 1.0, &g, &ix, &kn());
        assert!((s.weight(genre(&ix, "Qqa")) - 1.0).abs() < 1e-5);
        s.hear(ix.get("b").unwrap(), Full, 2000.0, 1.0, &g, &ix, &kn());
        assert!((s.weight(genre(&ix, "Qqa")) - 0.7).abs() < 1e-5);
        assert!(s.fit(ix.get("b").unwrap(), &ix) > s.fit(ix.get("a").unwrap(), &ix));
    }

    #[test]
    fn search_switch_off_ends_search() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1000.0, 0.1, &g, &ix, &kn());
        s.hear(ix.get("b").unwrap(), EarlySkip, 2000.0, 0.1, &g, &ix, &kn());
        let off = Knobs { search_skips: usize::MAX, ..kn() };
        assert!(!s.searching(2000.0, &off));
        assert!(!s.hear(ix.get("a").unwrap(), Full, 3000.0, 1.0, &g, &ix, &off), "no search left to end");
        assert!(!s.searching(3000.0, &kn()));
    }

    #[test]
    fn search_switch_off_and_on_starts_over() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1000.0, 0.1, &g, &ix, &kn());
        s.hear(ix.get("b").unwrap(), EarlySkip, 2000.0, 0.1, &g, &ix, &kn());
        s.end_search();
        assert!(!s.searching(2000.0, &kn()), "off then on with no song between");
        let off = Knobs { search_skips: usize::MAX, ..kn() };
        s.hear(ix.get("a").unwrap(), EarlySkip, 3000.0, 0.1, &g, &ix, &off);
        s.hear(ix.get("b").unwrap(), EarlySkip, 4000.0, 0.1, &g, &ix, &kn());
        assert!(!s.searching(4000.0, &kn()), "a skip while off does not count towards a search");
    }

    #[test]
    fn search_mode() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1000.0, 0.1, &g, &ix, &kn());
        s.hear(ix.get("b").unwrap(), EarlySkip, 60_000.0, 0.1, &g, &ix, &kn());
        assert!(s.searching(60_000.0, &kn()));
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1000.0, 0.1, &g, &ix, &kn());
        s.hear(ix.get("b").unwrap(), EarlySkip, 101_000.0, 0.1, &g, &ix, &kn());
        assert!(!s.searching(101_000.0, &kn()));
    }

    #[test]
    fn catch() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1000.0, 0.1, &g, &ix, &kn());
        s.hear(ix.get("a").unwrap(), EarlySkip, 2000.0, 0.1, &g, &ix, &kn());
        assert!(!s.hear(ix.get("hm").unwrap(), Neutral, 3000.0, 0.4, &g, &ix, &kn()), "not held yet");
        assert!(s.hear(ix.get("b").unwrap(), Neutral, 4000.0, 0.5, &g, &ix, &kn()));
        assert!(!s.searching(4000.0, &kn()));
        assert!((s.weight(genre(&ix, "Qqb")) - 3.0).abs() < 1e-5);
        assert_eq!(s.weight(genre(&ix, "Qqa")), 0.0);
    }

    #[test]
    fn genre_spread() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("hm").unwrap(), Full, 1000.0, 1.0, &g, &ix, &kn());
        assert!(s.weight(genre(&ix, "Hard Rock")) > 0.0);
    }

    #[test]
    fn idle_reset() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), Full, 1000.0, 1.0, &g, &ix, &kn());
        s.hear(ix.get("b").unwrap(), Full, 1000.0 + 31.0 * 60_000.0, 1.0, &g, &ix, &kn());
        assert_eq!(s.weight(genre(&ix, "Qqa")), 0.0);
        assert!(!s.is_empty());
    }

    #[test]
    fn search_ends_when_idle_or_clock_steps_back() {
        let (ix, g) = setup();
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1000.0, 0.1, &g, &ix, &kn());
        s.hear(ix.get("b").unwrap(), EarlySkip, 2000.0, 0.1, &g, &ix, &kn());
        assert!(s.searching(2000.0, &kn()) && !s.searching(2000.0 + 31.0 * 60_000.0, &kn()));
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1_000_000.0, 0.1, &g, &ix, &kn());
        s.hear(ix.get("b").unwrap(), EarlySkip, 500_000.0, 0.1, &g, &ix, &kn());
        assert!(!s.searching(500_000.0, &kn()));
    }

    #[test]
    fn seed_from_recent() {
        let (ix, g) = setup();
        let mut s = Session::default();
        assert!(s.is_empty());
        s.seed(&[ix.get("a").unwrap(), ix.get("b").unwrap()], &g, &ix, &kn());
        assert!(s.weight(genre(&ix, "Qqb")) > s.weight(genre(&ix, "Qqa")) && s.weight(genre(&ix, "Qqa")) > 0.0);
    }

    #[test]
    fn three_skips_to_search() {
        let (ix, g) = setup();
        let k = Knobs { search_skips: 3, ..Knobs::default() };
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1000.0, 0.1, &g, &ix, &k);
        s.hear(ix.get("b").unwrap(), EarlySkip, 2000.0, 0.1, &g, &ix, &k);
        assert!(!s.searching(2000.0, &k));
        s.hear(ix.get("a").unwrap(), EarlySkip, 3000.0, 0.1, &g, &ix, &k);
        assert!(s.searching(3000.0, &k));
    }

    #[test]
    fn search_sec_idle_and_fade() {
        let (ix, g) = setup();
        let short = Knobs { search_ms: 30_000.0, ..Knobs::default() };
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), EarlySkip, 1000.0, 0.1, &g, &ix, &short);
        s.hear(ix.get("b").unwrap(), EarlySkip, 61_000.0, 0.1, &g, &ix, &short);
        assert!(!s.searching(61_000.0, &short), "60 s apart is not close with a 30 s window");

        let idle = Knobs { idle_ms: 10.0 * 60_000.0, ..Knobs::default() };
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), Full, 1000.0, 1.0, &g, &ix, &idle);
        s.hear(ix.get("b").unwrap(), Full, 1000.0 + 11.0 * 60_000.0, 1.0, &g, &ix, &idle);
        assert_eq!(s.weight(genre(&ix, "Qqa")), 0.0);

        let fade = Knobs { fade: 0.5, ..Knobs::default() };
        let mut s = Session::default();
        s.hear(ix.get("a").unwrap(), Full, 1000.0, 1.0, &g, &ix, &fade);
        s.hear(ix.get("b").unwrap(), Full, 2000.0, 1.0, &g, &ix, &fade);
        assert!((s.weight(genre(&ix, "Qqa")) - 0.5).abs() < 1e-5);
    }
}
