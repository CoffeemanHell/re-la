//! The DJ: picks for the queue end from what this listener plays, skips and keeps.
pub mod knobs;
pub mod listen;
pub mod genres;
pub mod index;
pub mod memory;
pub mod session;
pub mod why;

pub use knobs::Knobs;
use genres::{Genres, UserGenres};
use index::{Index, Song};
use memory::Memory;
use serde::Serialize;
use serde_json::Value;
use session::Session;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Everything the DJ knows about one account: the library, the long memory, genre closeness, this sitting.
pub struct Brain {
    pub index: Index,
    pub memory: Memory,
    pub genres: Genres,
    pub session: Session,
    /// what the last pick weighed, for a state page
    pub last_pick: Option<Weighed>,
    user: UserGenres,
}

impl Brain {
    /// The index (rebuildable) under `cache`, the memory and the listener's genre file under `data`.
    pub fn open(cache: &Path, data: &Path, day: u32, k: &Knobs) -> Brain {
        let mut b = Brain::open_lazy(cache, data);
        b.rebuild_genres(day, k);
        b
    }

    /// Opens with no genre closeness yet: the caller builds it (`genres_later`) where it does not hold anything up.
    pub fn open_lazy(cache: &Path, data: &Path) -> Brain {
        let index = Index::open(cache.join("dj-index.json"));
        let memory = Memory::open(data.join("dj-memory.json"));
        let user = UserGenres::load(&data.join("dj-genres.json"));
        Brain { index, memory, genres: Genres::default(), session: Session::default(), last_pick: None, user }
    }

    pub fn rebuild_genres(&mut self, day: u32, k: &Knobs) {
        self.genres = self.genres_later(day, k)();
    }

    /// Builds genre closeness from a copy of what it needs now, on whatever thread calls it.
    pub fn genres_later(&self, day: u32, k: &Knobs) -> impl FnOnce() -> Genres + Send + 'static {
        let (index, pairs, user) = (self.index.genre_view(), self.memory.pairs(day, k), self.user.clone());
        move || Genres::build(&index, &pairs, &user)
    }
}

impl Drop for Brain {
    fn drop(&mut self) {
        let _ = self.memory.save();
    }
}

/// Slots of a batch: (exploring, wildcard, normal). Searching doubles the exploring share. Fractions are rounded by
/// chance, so one-song refills explore (and draw wildcards) as often on average as whole batches.
pub fn slots(batch: i32, explore: i32, wildcard: f64, searching: bool, rng: &mut fastrand::Rng) -> (usize, usize, usize) {
    let batch = batch.max(0) as usize;
    let share = f64::from(explore.clamp(0, 50)) / 100.0 * if searching { 2.0 } else { 1.0 };
    // one draw for both, so the wildcard does not eat into the exploring share
    let r = rng.f64();
    let joker = ((batch as f64 * wildcard + r).floor() as usize).min(batch);
    let both = ((batch as f64 * (wildcard + share) + r).floor() as usize).min(batch);
    (both - joker, joker, batch - both)
}

/// What a candidate's score is made of.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Parts {
    /// fits this sitting
    pub fit: f64,
    /// long-term taste for its artist or genre
    pub taste: f64,
    /// Navidrome calls it similar to the song playing
    pub similar: f64,
    /// new in the library or barely played
    pub fresh: f64,
    /// heard lately (≤ 0)
    pub recent: f64,
    /// "more like this"
    pub feedback: f64,
    pub disliked: bool,
}

impl Parts {
    pub fn total(&self) -> f64 {
        let s = self.fit + self.taste + self.similar + self.fresh + self.recent + self.feedback;
        if self.disliked { disliked_score(s) } else { s }
    }
}

/// What a pick weighed: the best candidates and every pick, by score (JSON for the themes).
#[derive(Serialize, Clone, Debug, Default)]
pub struct Weighed {
    pub searching: bool,
    pub seed: Seed,
    /// the sitting's strongest tags
    pub session: Vec<SessionTag>,
    pub rows: Vec<Row>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Seed {
    pub title: String,
    pub artist: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct SessionTag {
    pub tag: String,
    pub weight: f32,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub total: f64,
    pub fit: f64,
    pub taste: f64,
    pub similar: f64,
    pub fresh: f64,
    pub recent: f64,
    pub feedback: f64,
    pub disliked: bool,
    /// faded early skips
    pub skip: f32,
    pub last_heard_days: Option<u32>,
    pub why: Option<String>,
    /// "explore" | "wildcard" | "normal"; None when not picked
    pub picked: Option<&'static str>,
}

/// A disliked song keeps a chance: a good score shrinks, a poor one never grows.
fn disliked_score(s: f64) -> f64 {
    if s > 0.0 { s * 0.3 } else { s - 1.0 }
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

/// Who the per-artist cap counts a song for: its artists (as the index reads them), not "Various Artists" and such.
fn performers(m: &Value) -> Vec<String> {
    let mut v: Vec<String> = m["artists"].as_array().into_iter().flatten().map(|a| text(&a["name"]).to_string()).collect();
    if v.iter().all(String::is_empty) {
        v = vec![text(&m["artist"]).to_string()];
    }
    v.retain(|a| !index::nobody(a));
    v
}

/// New (under `new_days` in the library) or barely played.
fn fresh(s: &Song, day: u32, k: &Knobs) -> bool {
    s.play_count <= 1 || day.saturating_sub(s.created_day) < k.new_days
}

/// Held back when heard lately: more so in the first half of `repeat_days`.
fn repeat_penalty(days_ago: Option<u32>, k: &Knobs) -> f64 {
    match days_ago {
        Some(d) if d < k.repeat_days / 2 => -1.0,
        Some(d) if d < k.repeat_days => -0.4,
        _ => 0.0,
    }
}

/// Long taste per artist and genre: their songs' plays, log-scaled to 0..1.
struct Taste {
    artist: HashMap<u32, f32>,
    genre: HashMap<u32, f32>,
}

impl Taste {
    fn of(index: &Index) -> Taste {
        let (mut artist, mut genre): (HashMap<u32, f32>, HashMap<u32, f32>) = (HashMap::new(), HashMap::new());
        for s in &index.songs {
            for &a in &s.artists {
                *artist.entry(a).or_default() += s.play_count as f32;
            }
            for &g in &s.genres {
                *genre.entry(g).or_default() += s.play_count as f32;
            }
        }
        let scale = |m: &mut HashMap<u32, f32>| {
            let max = m.values().copied().fold(0.0f32, f32::max);
            m.values_mut().for_each(|v| *v = if max > 0.0 { (1.0 + *v).ln() / (1.0 + max).ln() } else { 0.0 });
        };
        scale(&mut artist);
        scale(&mut genre);
        Taste { artist, genre }
    }

    /// The score, and the word id of the artist or genre that gives most of it.
    fn score_best(&self, s: &Song, index: &Index, genres: &Genres) -> (f32, Option<u32>) {
        let best = |v: &mut Option<(f32, u32)>, x: f32, id: u32| {
            if x > 0.0 && v.is_none_or(|(b, _)| x > b) {
                *v = Some((x, id));
            }
        };
        let (mut a, mut g) = (None, None);
        for &id in &s.artists {
            best(&mut a, self.artist.get(&id).copied().unwrap_or(0.0), id);
        }
        for &id in &s.genres {
            let own = self.genre.get(&id).copied().unwrap_or(0.0);
            let near = genres
                .neighbors(index.word(id))
                .iter()
                .filter_map(|(n, sim)| Some(sim * self.genre.get(&index.word_id(n)?)?))
                .fold(0.0f32, f32::max);
            best(&mut g, own.max(near), id);
        }
        let (av, gv) = (a.map_or(0.0, |x| x.0), g.map_or(0.0, |x| x.0));
        let name = if av >= gv { a } else { g }.map(|x| x.1);
        let mut t = 0.5 * av + 0.5 * gv;
        if s.starred {
            t += 0.5;
        }
        if s.rating > 0 {
            t += (f32::from(s.rating) - 3.0) / 2.0;
        }
        (t, name)
    }
}

/// The sitting's taste, or one made from the last songs when nothing was heard yet.
fn taste_now<'a>(brain: &'a Brain, history: &[Value], tmp: &'a mut Session, k: &Knobs) -> &'a Session {
    if !brain.session.is_empty() {
        return &brain.session;
    }
    let recent: Vec<&Song> = history.iter().rev().take(5).rev().filter_map(|m| brain.index.get(text(&m["id"]))).collect();
    tmp.seed(&recent, &brain.genres, &brain.index, k);
    tmp
}

/// Picks up to k.batch new songs from `pool` to continue `history` (the queue up to the current song) after
/// `upcoming` (the rest of it). Songs in `similar` came from getSimilarSongs. Pool and history go into the index first.
/// An artist is picked at most `artist_max` times counting the last 10 songs and those coming up.
#[allow(clippy::too_many_arguments)]
pub fn pick(
    brain: &mut Brain,
    pool: &[Value],
    history: &[Value],
    upcoming: &[Value],
    similar: &HashSet<String>,
    k: &Knobs,
    searching: bool,
    day: u32,
    rng: &mut fastrand::Rng,
) -> Vec<Value> {
    for m in history {
        // the copy taken when it was queued: older than what the index may know
        if brain.index.get(text(&m["id"])).is_none() {
            brain.index.upsert(m, day);
        }
    }
    for m in pool {
        brain.index.upsert(m, day);
    }
    let (out, state) = weigh(brain, pool, history, upcoming, similar, k, searching, day, rng);
    brain.last_pick = Some(state);
    out
}

/// `pick` once the index knows every song: the picks, and what was weighed.
#[allow(clippy::too_many_arguments)]
fn weigh(
    brain: &Brain,
    pool: &[Value],
    history: &[Value],
    upcoming: &[Value],
    similar: &HashSet<String>,
    k: &Knobs,
    searching: bool,
    day: u32,
    rng: &mut fastrand::Rng,
) -> (Vec<Value>, Weighed) {
    let ix = &brain.index;
    let mut tmp = Session::default();
    let session = taste_now(brain, history, &mut tmp, k);
    let taste = Taste::of(ix);

    // seed language: the most common among the last 5 songs
    let mut langs: HashMap<u32, i32> = HashMap::new();
    for m in history.iter().rev().take(5) {
        if let Some(s) = ix.get(text(&m["id"])) {
            let l = ix.lang(s);
            if l != 0 {
                *langs.entry(l).or_default() += 1;
            }
        }
    }
    let seed_lang = langs.iter().max_by_key(|(l, n)| (**n, std::cmp::Reverse(**l))).map_or(0, |(l, _)| *l);

    let have: HashSet<&str> = history.iter().chain(upcoming).map(|m| text(&m["id"])).collect();
    struct C<'a> {
        song: &'a Value,
        s: &'a Song,
        artists: Vec<String>,
        parts: Parts,
        score: f64,
        fit_tag: Option<session::Tag>,
        taste_of: Option<u32>,
        fresh: bool,
        same_lang: bool,
    }
    let fit_k = 0.5 + f64::from(k.genre.clamp(0, 100)) / 100.0;
    let mut seen = HashSet::new();
    let mut cands: Vec<C> = vec![];
    for m in pool {
        let id = text(&m["id"]);
        if have.contains(id) || !seen.insert(id) {
            continue;
        }
        let Some(s) = ix.get(id) else { continue };
        let age = day.saturating_sub(s.created_day) as f32;
        let (fit, fit_tag) = session.fit_best(s, ix);
        let (taste_score, taste_of) = taste.score_best(s, ix, &brain.genres);
        let new = if k.new_days == 0 { 0.0 } else { (1.0 - age / k.new_days as f32).max(0.0) };
        let parts = Parts {
            fit: fit_k * f64::from(fit),
            taste: k.taste * f64::from(taste_score),
            similar: if similar.contains(id) { k.similar } else { 0.0 },
            fresh: f64::from(0.6 / (1.0 + s.play_count as f32).sqrt() + 0.5 * new),
            recent: repeat_penalty(brain.memory.last_heard(id).map(|d| day.saturating_sub(d)), k),
            feedback: 0.0,
            disliked: brain.memory.disliked(id, day, k),
        };
        let score = parts.total();
        cands.push(C {
            song: m,
            s,
            artists: performers(m),
            parts,
            score,
            fit_tag,
            taste_of,
            fresh: fresh(s, day, k),
            same_lang: seed_lang != 0 && ix.lang(s) == seed_lang,
        });
    }

    let (n_explore, n_joker, _) = slots(k.batch, k.explore, k.wildcard, searching, rng); // normal picks fill the rest
    let quota = if seed_lang == 0 { 0 } else { (f64::from(k.batch.max(0) * k.language.clamp(0, 100)) / 100.0).round() as usize };
    struct Draw {
        used: Vec<bool>,
        per_artist: HashMap<String, i32>,
        same: usize,
        quota: usize,
        artist_max: i32,
        temperature: f64,
    }
    // one draw: 0 = exploring (fresh songs only), 1 = wildcard (evenly), 2 = normal; softmax(score / temperature) but the wildcard
    fn draw(kind: u8, cands: &[C], d: &mut Draw, rng: &mut fastrand::Rng) -> Option<usize> {
        let ok = |i: usize, lang_only: bool| {
            let c = &cands[i];
            !d.used[i]
                && (kind != 0 || c.fresh)
                && (!lang_only || c.same_lang)
                && c.artists.iter().all(|a| *d.per_artist.get(a).unwrap_or(&0) < d.artist_max)
        };
        let lang_only = d.same < d.quota;
        let mut idx: Vec<usize> = (0..cands.len()).filter(|&i| ok(i, lang_only)).collect();
        if idx.is_empty() && lang_only {
            if kind == 0 && cands.iter().enumerate().any(|(i, c)| c.same_lang && !c.fresh && !d.used[i]) {
                return None; // normal picks can still meet the quota: leave the slot to them
            }
            idx = (0..cands.len()).filter(|&i| ok(i, false)).collect(); // the quota cannot be met
        }
        if idx.is_empty() {
            return None;
        }
        let chosen = if kind == 1 {
            idx[rng.usize(..idx.len())]
        } else {
            let max = idx.iter().map(|&i| cands[i].score).fold(f64::MIN, f64::max);
            let ws: Vec<f64> = idx.iter().map(|&i| ((cands[i].score - max) / d.temperature).exp()).collect();
            let mut r = rng.f64() * ws.iter().sum::<f64>();
            let mut pick = idx[idx.len() - 1];
            for (k, wt) in ws.iter().enumerate() {
                if r < *wt {
                    pick = idx[k];
                    break;
                }
                r -= wt;
            }
            pick
        };
        d.used[chosen] = true;
        for a in &cands[chosen].artists {
            *d.per_artist.entry(a.clone()).or_default() += 1;
        }
        if cands[chosen].same_lang {
            d.same += 1;
        }
        Some(chosen)
    }
    let mut per_artist: HashMap<String, i32> = HashMap::new();
    for m in history.iter().rev().take(10).chain(upcoming) {
        for a in performers(m) {
            *per_artist.entry(a).or_default() += 1;
        }
    }
    let mut d = Draw { used: vec![false; cands.len()], per_artist, same: 0, quota, artist_max: k.artist_max, temperature: k.temperature };
    let mut chosen: Vec<(usize, u8)> = vec![];
    for _ in 0..n_explore {
        match draw(0, &cands, &mut d, rng) {
            Some(i) => chosen.push((i, 0)),
            None => break, // nothing new or barely played: normal picks fill in
        }
    }
    for _ in 0..n_joker {
        chosen.extend(draw(1, &cands, &mut d, rng).map(|i| (i, 1)));
    }
    while chosen.len() < k.batch.max(0) as usize {
        match draw(2, &cands, &mut d, rng) {
            Some(i) => chosen.push((i, 2)),
            None => break,
        }
    }

    let en = crate::i18n::english();
    let seed = history.last();
    let seed_title = seed.map_or("", |m| text(&m["title"]));
    let mut kinds: Vec<Option<u8>> = vec![None; cands.len()];
    for &(i, kind) in &chosen {
        kinds[i] = Some(kind);
    }
    // the state page shows the best ROWS and every pick: enough to see why, cheap enough for every pick
    const ROWS: usize = 200;
    let mut order: Vec<usize> = (0..cands.len()).collect();
    order.sort_by(|&a, &b| cands[b].score.total_cmp(&cands[a].score));
    let mut rows = Vec::with_capacity(ROWS.min(cands.len()) + chosen.len());
    let mut out = vec![];
    for (rank, &i) in order.iter().enumerate() {
        let c = &cands[i];
        if rank >= ROWS && kinds[i].is_none() {
            continue;
        }
        let last = brain.memory.last_heard(&c.s.id).map(|d| day.saturating_sub(d));
        let why = kinds[i].map(|kind| {
            let fit_tag = c.fit_tag.map(|t| why::tag_label(t, ix, en));
            let taste_name = c.taste_of.map(|w| ix.word(w).to_string());
            let age = day.saturating_sub(c.s.created_day);
            why::sentence(en, kind, &c.parts, fit_tag, taste_name, seed_title, age, last, k.new_days)
        });
        if let Some(w) = &why {
            let mut song = c.song.clone();
            song["djWhy"] = w.clone().into();
            out.push(song);
        }
        let p = &c.parts;
        rows.push(Row {
            id: c.s.id.clone(),
            title: text(&c.song["title"]).to_string(),
            artist: text(&c.song["artist"]).to_string(),
            total: c.score,
            fit: p.fit,
            taste: p.taste,
            similar: p.similar,
            fresh: p.fresh,
            recent: p.recent,
            feedback: p.feedback,
            disliked: p.disliked,
            skip: brain.memory.skip(&c.s.id, day, k),
            last_heard_days: last,
            why,
            picked: kinds[i].map(|kind| ["explore", "wildcard", "normal"][kind as usize]),
        });
    }
    let state = Weighed {
        searching,
        seed: Seed { title: seed_title.to_string(), artist: seed.map_or("", |m| text(&m["artist"])).to_string() },
        session: session.top(10).into_iter().map(|(t, weight)| SessionTag { tag: why::tag_label(t, ix, en), weight }).collect(),
        rows,
    };
    rng.shuffle(&mut out);
    (out, state)
}

/// Ids from the index worth adding to the pool: up to 2·batch new or barely played songs and 2·batch others, those
/// that fit this sitting best (with some chance). None when the index is empty.
pub fn extra_ids(brain: &Brain, history: &[Value], k: &Knobs, _searching: bool, day: u32, rng: &mut fastrand::Rng) -> Vec<String> {
    let ix = &brain.index;
    if ix.is_empty() {
        return vec![];
    }
    let mut tmp = Session::default();
    let session = taste_now(brain, history, &mut tmp, k);
    let have: HashSet<&str> = history.iter().map(|m| text(&m["id"])).collect();
    let n = 2 * k.batch.max(0) as usize;
    let mut new: Vec<(f32, &str)> = vec![];
    let mut old: Vec<(f32, &str)> = vec![];
    for s in &ix.songs {
        if have.contains(s.id.as_str()) {
            continue;
        }
        let v = session.fit(s, ix) + 0.3 * rng.f32();
        if fresh(s, day, k) { new.push((v, &s.id)) } else { old.push((v, &s.id)) }
    }
    let mut out = vec![];
    for mut l in [new, old] {
        l.sort_by(|a, b| b.0.total_cmp(&a.0));
        out.extend(l.iter().take(n).map(|(_, id)| id.to_string()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DAY: u32 = 20_000;

    fn brain() -> Brain {
        let d = std::env::temp_dir().join(format!("re-la-djpick-{}-{}", std::process::id(), fastrand::u64(..)));
        Brain::open(&d.join("cache"), &d.join("data"), DAY, &Knobs::default())
    }

    fn song(id: &str, title: &str, artist: &str, genre: &str) -> Value {
        json!({"id": id, "title": title, "artist": artist, "genre": genre, "playCount": 5, "created": "2020-01-01T00:00:00Z"})
    }

    fn w(language: i32, explore: i32, batch: i32) -> Knobs {
        Knobs { language, explore, batch, ..Knobs::default() }
    }

    fn ids(v: &[Value]) -> Vec<String> {
        v.iter().map(|m| m["id"].as_str().unwrap().to_string()).collect()
    }

    // 8 Turkish artists: songs without Turkish letters are recognised through the artist's other song.
    #[test]
    fn picks() {
        let mut pool = vec![];
        for a in 0..8 {
            let artist = format!("TR Artist {a}");
            pool.push(song(&format!("tr{a}x"), "Şarkı", &artist, "Pop"));
            pool.push(song(&format!("tr{a}y"), "Gel", &artist, "Pop"));
        }
        for i in 0..40 {
            pool.push(song(&format!("en{i}"), "Song", &format!("EN Artist {i}"), "Pop"));
        }
        let history = vec![song("seed", "Şımarık", "Tarkan", "Pop")];
        let mut rng = fastrand::Rng::with_seed(42);
        let mut b = brain();
        let picked = pick(&mut b, &pool, &history, &[], &HashSet::new(), &w(100, 20, 10), false, DAY, &mut rng);
        assert_eq!(picked.len(), 10);
        let mut per_artist: HashMap<String, i32> = HashMap::new();
        for m in &picked {
            assert!(m["id"].as_str().unwrap().starts_with("tr"), "{m}");
            let n = per_artist.entry(m["artist"].as_str().unwrap().into()).or_default();
            *n += 1;
            assert!(*n <= 2);
        }
        assert_eq!(ids(&picked).iter().collect::<HashSet<_>>().len(), 10);

        let tr = ids(&pick(&mut b, &pool, &history, &[], &HashSet::new(), &w(50, 20, 10), false, DAY, &mut rng))
            .iter()
            .filter(|id| id.starts_with("tr"))
            .count();
        assert!(tr >= 5);

        // one Turkish song of nobody in particular does not make the others Turkish
        for nobody in ["", "Various Artists", "[Unknown Artist]"] {
            let mut b = brain();
            let pool = [song("x", "Şarkı", nobody, "Pop"), song("y", "Song", nobody, "Pop"), song("z", "Gel", "TR Artist 0", "Pop")];
            let known = [song("tr0x", "Şarkı", "TR Artist 0", "Pop")];
            let picked = pick(&mut b, &pool, &known, &[], &HashSet::from(["y".into()]), &w(100, 0, 2), false, DAY, &mut rng);
            assert!(picked.iter().all(|v| v["id"] != "y"), "{nobody}: {picked:?}");
        }

        // already queued songs are never picked again
        assert!(pick(&mut b, &history, &history, &[], &HashSet::new(), &w(100, 20, 10), false, DAY, &mut rng).is_empty());
    }

    // a song that fits well but is disliked still comes now and then, far less often
    #[test]
    fn disliked_rare_not_banned() {
        let history: Vec<Value> = (0..5).map(|i| song(&format!("h{i}"), "S", &format!("H{i}"), "Qqfav")).collect();
        let mut pool: Vec<Value> = (0..20).map(|i| song(&format!("o{i}"), "S", &format!("O{i}"), "Qqother")).collect();
        pool.push(song("x", "S", "X", "Qqfav"));
        let count = |disliked: bool| {
            let mut b = brain();
            if disliked {
                for d in [DAY - 1, DAY - 1, DAY, DAY] {
                    b.memory.record("x", listen::Heard::EarlySkip, d, &Knobs::default());
                }
            }
            let mut rng = fastrand::Rng::with_seed(7);
            (0..1000)
                .filter(|_| pick(&mut b, &pool, &history, &[], &HashSet::new(), &w(0, 0, 1), false, DAY, &mut rng)[0]["id"] == "x")
                .count()
        };
        let (liked, disliked) = (count(false), count(true));
        assert!(disliked >= 1, "{disliked}");
        assert!(disliked * 3 <= liked, "{liked} vs {disliked}");
    }

    #[test]
    fn explore_share() {
        let fav = |id: String, a: String| json!({"id": id, "artist": a, "genre": "Rock", "playCount": 50, "created": "2020-01-01T00:00:00Z"});
        let history: Vec<Value> = (0..20).map(|i| fav(format!("h{i}"), format!("F{i}"))).collect();
        let mut pool: Vec<Value> = (0..30).map(|i| fav(format!("f{i}"), format!("F{i}"))).collect();
        let created = "2024-09-29T00:00:00Z"; // 5 days before DAY
        pool.extend((0..30).map(|i| json!({"id": format!("n{i}"), "artist": format!("N{i}"), "genre": "Rock", "playCount": 0, "created": created})));
        let mut b = brain();
        let mut rng = fastrand::Rng::with_seed(3);
        let (mut new, mut all) = (0, 0);
        for _ in 0..100 {
            let p = pick(&mut b, &pool, &history, &[], &HashSet::new(), &w(0, 20, 10), false, DAY, &mut rng);
            all += p.len();
            new += p.iter().filter(|m| m["id"].as_str().unwrap().starts_with('n')).count();
        }
        assert!(new * 100 > all * 15, "{new}/{all}");
    }

    #[test]
    fn recent_penalty() {
        let history = vec![song("h", "S", "H", "Qqg")];
        let pool = vec![song("a", "S", "A", "Qqg"), song("b", "S", "B", "Qqg")];
        let mut b = brain();
        b.memory.record("a", listen::Heard::Full, DAY - 1, &Knobs::default());
        let mut rng = fastrand::Rng::with_seed(5);
        let (mut a, mut bb) = (0, 0);
        for _ in 0..1000 {
            match pick(&mut b, &pool, &history, &[], &HashSet::new(), &w(0, 0, 1), false, DAY, &mut rng)[0]["id"].as_str().unwrap() {
                "a" => a += 1,
                _ => bb += 1,
            }
        }
        assert!(bb >= 2 * a, "{a} vs {bb}");
    }

    #[test]
    fn searching_doubles_explore() {
        let r = &mut fastrand::Rng::with_seed(4);
        assert_eq!(slots(10, 20, 0.1, false, r), (2, 1, 7));
        assert_eq!(slots(10, 20, 0.1, true, r), (4, 1, 5));
        assert_eq!(slots(0, 20, 0.1, false, r), (0, 0, 0));
        // one-song refills (the usual case once the queue is full) explore at the same rate on average
        let n = 2000;
        let (e, j) = (0..n).map(|_| slots(1, 20, 0.1, false, r)).fold((0, 0), |a, s| (a.0 + s.0, a.1 + s.1));
        assert!((375..460).contains(&e) && (150..250).contains(&j), "{e} {j}");
        let e2: usize = (0..n).map(|_| slots(1, 20, 0.1, true, r).0).sum();
        assert!(e2 > e + 300, "searching: {e2} vs {e}");
    }

    // an artist already twice among the recent and upcoming songs gets no third
    #[test]
    fn recent_artist_capped() {
        let history: Vec<Value> = (0..8).map(|i| song(&format!("h{i}"), "S", &format!("H{i}"), "Qqg")).chain([song("z1", "S", "Z", "Qqg")]).collect();
        let upcoming = vec![song("z2", "S", "Z", "Qqg")];
        let pool: Vec<Value> = (0..5).map(|i| song(&format!("z{}", i + 3), "S", "Z", "Qqg")).chain((0..5).map(|i| song(&format!("o{i}"), "S", &format!("O{i}"), "Qqx"))).collect();
        let mut b = brain();
        let mut rng = fastrand::Rng::with_seed(8);
        for _ in 0..50 {
            let p = pick(&mut b, &pool, &history, &upcoming, &HashSet::new(), &w(0, 0, 3), false, DAY, &mut rng);
            assert!(p.iter().all(|m| m["artist"] != "Z"), "{p:?}");
        }
    }

    // "Various Artists" is nobody in particular; "A" counts through the artists list whatever the label says
    #[test]
    fn cap_by_performer() {
        let mut b = brain();
        let va: Vec<Value> = (0..6).map(|i| song(&format!("v{i}"), "S", "Various Artists", "Qqg")).collect();
        let p = pick(&mut b, &va, &[], &[], &HashSet::new(), &w(0, 0, 6), false, DAY, &mut fastrand::Rng::with_seed(2));
        assert_eq!(p.len(), 6);
        let feat = |id: &str, label: &str| json!({"id": id, "artist": label, "artists": [{"name": "A"}, {"name": "B"}], "genre": "Qqg"});
        let history = vec![feat("h1", "A"), feat("h2", "A feat. B")];
        let pool = vec![feat("c1", "B feat. A"), song("o1", "S", "O", "Qqg")];
        for seed in 0..20 {
            let p = pick(&mut b, &pool, &history, &[], &HashSet::new(), &w(0, 0, 2), false, DAY, &mut fastrand::Rng::with_seed(seed));
            assert_eq!(ids(&p), ["o1"]);
        }
    }

    // the copy in the queue is older than what the index knows
    #[test]
    fn history_keeps_index() {
        let mut b = brain();
        b.index.upsert(&json!({"id": "h", "playCount": 9, "created": "2020-01-01T00:00:00Z"}), DAY);
        pick(&mut b, &[], &[json!({"id": "h", "playCount": 1})], &[], &HashSet::new(), &w(0, 0, 1), false, DAY, &mut fastrand::Rng::new());
        let s = b.index.get("h").unwrap();
        assert_eq!((s.play_count, s.created_day < DAY - 100), (9, true));
    }

    #[test]
    fn no_genre_songs() {
        let pool: Vec<Value> = (0..30).map(|i| json!({"id": format!("p{i}"), "artist": format!("A{i}")})).collect();
        let mut b = brain();
        let p = pick(&mut b, &pool, &[json!({"id": "h"})], &[], &HashSet::new(), &w(80, 20, 10), false, DAY, &mut fastrand::Rng::with_seed(1));
        assert_eq!(p.len(), 10);
    }

    #[test]
    fn extra_ids_reach_new() {
        let mut b = brain();
        for i in 0..5000 {
            b.index.upsert(&song(&format!("o{i}"), "S", &format!("A{}", i % 300), "Rock"), DAY);
        }
        for i in 0..10 {
            b.index.upsert(&json!({"id": format!("n{i}"), "artist": "N", "genre": "Rock", "playCount": 0, "created": "2024-09-29T00:00:00Z"}), DAY);
        }
        let history: Vec<Value> = (0..5).map(|i| song(&format!("o{i}"), "S", &format!("A{i}"), "Rock")).collect();
        let x = extra_ids(&b, &history, &w(0, 20, 10), false, DAY, &mut fastrand::Rng::with_seed(2));
        assert!(x.iter().filter(|id| id.starts_with('n')).count() >= 5, "{x:?}");
        assert!(x.iter().all(|id| !["o0", "o1", "o2", "o3", "o4"].contains(&id.as_str())));
        assert!(extra_ids(&brain(), &history, &w(0, 20, 10), false, DAY, &mut fastrand::Rng::with_seed(2)).is_empty());
    }

    #[test]
    fn negative_score_disliked() {
        assert_eq!(disliked_score(2.0), 2.0 * 0.3);
        assert!(disliked_score(-0.5) < -0.5); // never climbs
    }

    // 50 000 songs: picking stays under 10 ms, the index file under 10 MB, the genre build under 500 ms.
    // cargo test --offline -p rela-core --release budget_50k -- --ignored --nocapture
    #[test]
    #[ignore]
    fn budget_50k() {
        use std::time::Instant;
        let mut b = brain();
        let genre = |i: usize| format!("Genre {}", i % 300);
        for i in 0..50_000 {
            let gs: Vec<Value> = (0..1 + i % 3).map(|k| json!({"name": genre(i + k * 7)})).collect();
            b.index.upsert(
                &json!({"id": format!("song-{i:08}-abcdef"), "artist": format!("Artist {}", i % 5000), "albumId": format!("al{}", i / 10),
                    "genres": gs, "year": 1960 + i % 60, "playCount": i % 40, "created": "2023-05-01T00:00:00Z"}),
                DAY,
            );
        }
        for i in 0..5000 {
            b.memory.record(&format!("song-{i:08}-abcdef"), if i % 3 == 0 { listen::Heard::EarlySkip } else { listen::Heard::Full }, DAY - (i % 90) as u32, &Knobs::default());
        }
        for i in 0..2000 {
            b.memory.record_pair(&genre(i), &genre(i + 1), DAY, &Knobs::default());
        }
        let t = Instant::now();
        let later = b.genres_later(DAY, &Knobs::default());
        let copy = t.elapsed();
        b.genres = later();
        let build = t.elapsed() - copy;
        println!("genre copy on the player thread {copy:?}");
        b.index.save().unwrap();
        let size = std::fs::metadata(b.index.path()).unwrap().len();
        let history: Vec<Value> = (0..5).map(|i| json!({"id": format!("song-{i:08}-abcdef")})).collect();
        let pool: Vec<Value> = (100..650).map(|i| json!({"id": format!("song-{i:08}-abcdef"), "artist": format!("Artist {}", i % 5000)})).collect();
        let mut rng = fastrand::Rng::with_seed(1);
        let mut times: Vec<f64> = (0..20)
            .map(|_| {
                let t = Instant::now();
                let x = extra_ids(&b, &history, &w(80, 20, 10), false, DAY, &mut rng);
                let p = pick(&mut b, &pool, &history, &[], &HashSet::new(), &w(80, 20, 10), false, DAY, &mut rng);
                assert!(p.len() == 10 && !x.is_empty());
                t.elapsed().as_secs_f64() * 1000.0
            })
            .collect();
        times.sort_by(f64::total_cmp);
        // shortcut: a rough count from sizes, not the real RSS; heaptrack tells the truth when it matters
        let ram = b.index.songs.len() * (std::mem::size_of::<Song>() + 22 + 4 * 6) + 5000 * 64 + b.index.words_len() * 32;
        println!("genres build {build:?}, index file {:.1} MB, pick+extra median {:.2} ms (max {:.2}), ~RAM {:.1} MB", size as f64 / 1e6, times[10], times[19], ram as f64 / 1e6);
        assert!(times[10] < 10.0, "pick median {} ms", times[10]);
        assert!(size < 10_000_000, "index {size} bytes");
        assert!(build.as_millis() < 500, "genres {build:?}");
        assert!(ram < 20_000_000, "ram ~{ram}");
    }

    #[test]
    fn repeat_days_tiers() {
        let k = |repeat_days| Knobs { repeat_days, ..Knobs::default() };
        assert_eq!(repeat_penalty(Some(2), &k(7)), -1.0);
        assert_eq!(repeat_penalty(Some(3), &k(7)), -0.4);
        assert_eq!(repeat_penalty(Some(7), &k(7)), 0.0);
        assert_eq!(repeat_penalty(Some(4), &k(10)), -1.0);
        assert_eq!(repeat_penalty(Some(8), &k(10)), -0.4);
        assert_eq!(repeat_penalty(None, &k(10)), 0.0);
        assert_eq!(repeat_penalty(Some(0), &k(0)), 0.0);
    }

    #[test]
    fn artist_max_one() {
        let history = vec![song("h", "S", "H", "Qqg")];
        let pool: Vec<Value> = (0..5).map(|i| song(&format!("z{i}"), "S", "Z", "Qqg")).chain((0..5).map(|i| song(&format!("o{i}"), "S", &format!("O{i}"), "Qqg"))).collect();
        let mut b = brain();
        let k = Knobs { artist_max: 1, ..w(0, 0, 4) };
        let mut rng = fastrand::Rng::with_seed(9);
        for _ in 0..50 {
            let p = pick(&mut b, &pool, &history, &[], &HashSet::new(), &k, false, DAY, &mut rng);
            assert_eq!(p.len(), 4);
            assert!(p.iter().filter(|m| m["artist"] == "Z").count() <= 1, "{p:?}");
        }
    }

    #[test]
    fn wildcard_zero() {
        let r = &mut fastrand::Rng::with_seed(4);
        assert!((0..500).all(|_| slots(10, 20, 0.0, false, r).1 == 0));
        assert!((0..500).any(|_| slots(10, 20, 0.3, false, r).1 == 3));
    }

    #[test]
    fn new_days_zero() {
        let mut b = brain();
        b.index.upsert(&json!({"id": "n", "playCount": 50, "created": "2024-09-29T00:00:00Z"}), DAY); // 5 days before DAY
        let s = b.index.get("n").unwrap();
        assert!(fresh(s, DAY, &Knobs::default()));
        assert!(!fresh(s, DAY, &Knobs { new_days: 0, ..Knobs::default() }));
    }

    #[test]
    fn parts_total() {
        let p = Parts { fit: 0.5, taste: 0.25, similar: 0.8, fresh: 0.3, recent: -0.4, feedback: 1.0, disliked: false };
        assert!((p.total() - 2.45).abs() < 1e-9);
        assert!((Parts { disliked: true, ..p }.total() - 2.45 * 0.3).abs() < 1e-9);
        let poor = Parts { recent: -1.0, disliked: true, ..Parts::default() };
        assert_eq!(poor.total(), -2.0);
    }

    #[test]
    fn picked_songs_carry_why() {
        let pool: Vec<Value> = (0..20).map(|i| song(&format!("p{i}"), "S", &format!("A{i}"), "Qqg")).collect();
        let history = vec![song("h", "Seed Song", "H", "Qqg")];
        let mut b = brain();
        let similar: HashSet<String> = HashSet::from(["p0".into(), "p1".into()]);
        let p = pick(&mut b, &pool, &history, &[], &similar, &w(0, 20, 5), false, DAY, &mut fastrand::Rng::with_seed(6));
        assert_eq!(p.len(), 5);
        assert!(p.iter().all(|m| !m["djWhy"].as_str().unwrap_or("").is_empty()), "{p:?}");
        let state = serde_json::to_value(b.last_pick.as_ref().unwrap()).unwrap();
        let rows = state["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 20);
        assert_eq!(rows.iter().filter(|r| !r["picked"].is_null()).count(), 5);
        assert_eq!(state["seed"]["title"], "Seed Song");
        for r in rows {
            let sum: f64 = ["fit", "taste", "similar", "fresh", "recent", "feedback"].iter().map(|k| r[k].as_f64().unwrap()).sum();
            assert!((sum - r["total"].as_f64().unwrap()).abs() < 1e-9, "{r}");
        }
    }
}
