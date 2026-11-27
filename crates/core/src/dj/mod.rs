//! The DJ: picks for the queue end from what this listener plays, skips and keeps.
pub mod listen;
pub mod genres;
pub mod index;
pub mod memory;
pub mod session;

use genres::{Genres, UserGenres};
use index::{Index, Song};
use memory::Memory;
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
    user: UserGenres,
}

impl Brain {
    /// The index (rebuildable) under `cache`, the memory and the listener's genre file under `data`.
    pub fn open(cache: &Path, data: &Path, day: u32) -> Brain {
        let index = Index::open(cache.join("dj-index.json"));
        let memory = Memory::open(data.join("dj-memory.json"));
        let user = UserGenres::load(&data.join("dj-genres.json"));
        let genres = Genres::build(&index, &memory.pairs(day), &user);
        Brain { index, memory, genres, session: Session::default(), user }
    }

    pub fn rebuild_genres(&mut self, day: u32) {
        self.genres = Genres::build(&self.index, &self.memory.pairs(day), &self.user);
    }
}

impl Drop for Brain {
    fn drop(&mut self) {
        let _ = self.memory.save();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weights {
    /// % of each batch in the seed's language (when one is known)
    pub language: i32,
    /// how strongly to stay close to what plays, 0-100
    pub genre: i32,
    /// % of each batch kept for new or rarely played songs
    pub explore: i32,
    pub batch: i32,
}

/// Slots of a batch: (exploring, wildcard, normal). Searching doubles the exploring share. Fractions are rounded by
/// chance, so one-song refills explore as often on average as whole batches (and one song in ten is a wildcard).
pub fn slots(batch: i32, explore: i32, searching: bool, rng: &mut fastrand::Rng) -> (usize, usize, usize) {
    let batch = batch.max(0) as usize;
    let share = f64::from(explore.clamp(0, 50)) / 100.0 * if searching { 2.0 } else { 1.0 };
    let mut by_chance = |x: f64| (x + rng.f64()).floor() as usize;
    let joker = by_chance(batch as f64 * 0.1).min(batch);
    let exp = by_chance(batch as f64 * share).min(batch - joker);
    (exp, joker, batch - joker - exp)
}

/// A disliked song keeps a chance: a good score shrinks, a poor one never grows.
fn disliked_score(s: f64) -> f64 {
    if s > 0.0 { s * 0.3 } else { s - 1.0 }
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

/// New (under 30 days in the library) or barely played.
fn fresh(s: &Song, day: u32) -> bool {
    s.play_count <= 1 || day.saturating_sub(s.created_day) < 30
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

    fn score(&self, s: &Song, index: &Index, genres: &Genres) -> f32 {
        let a = s.artists.iter().filter_map(|a| self.artist.get(a)).copied().fold(0.0f32, f32::max);
        let g = s
            .genres
            .iter()
            .map(|&g| {
                let own = self.genre.get(&g).copied().unwrap_or(0.0);
                let near = genres
                    .neighbors(index.word(g))
                    .iter()
                    .filter_map(|(n, sim)| Some(sim * self.genre.get(&index.word_id(n)?)?))
                    .fold(0.0f32, f32::max);
                own.max(near)
            })
            .fold(0.0f32, f32::max);
        let mut t = 0.5 * a + 0.5 * g;
        if s.starred {
            t += 0.5;
        }
        if s.rating > 0 {
            t += (f32::from(s.rating) - 3.0) / 2.0;
        }
        t
    }
}

/// The sitting's taste, or one made from the last songs when nothing was heard yet.
fn taste_now<'a>(brain: &'a Brain, history: &[Value], tmp: &'a mut Session) -> &'a Session {
    if !brain.session.is_empty() {
        return &brain.session;
    }
    let recent: Vec<&Song> = history.iter().rev().take(5).rev().filter_map(|m| brain.index.get(text(&m["id"]))).collect();
    tmp.seed(&recent, &brain.genres, &brain.index);
    tmp
}

/// Picks up to w.batch new songs from `pool` to continue `history` (the queue up to the current song) after
/// `upcoming` (the rest of it). Songs in `similar` came from getSimilarSongs. Pool and history go into the index first.
/// An artist is picked at most twice counting the last 10 songs and those coming up.
#[allow(clippy::too_many_arguments)]
pub fn pick(
    brain: &mut Brain,
    pool: &[Value],
    history: &[Value],
    upcoming: &[Value],
    similar: &HashSet<String>,
    w: &Weights,
    searching: bool,
    day: u32,
    rng: &mut fastrand::Rng,
) -> Vec<Value> {
    for m in history.iter().chain(pool) {
        brain.index.upsert(m, day);
    }
    let brain = &*brain;
    let ix = &brain.index;
    let mut tmp = Session::default();
    let session = taste_now(brain, history, &mut tmp);
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
        artist: String,
        score: f64,
        fresh: bool,
        same_lang: bool,
    }
    let fit_k = 0.5 + f64::from(w.genre.clamp(0, 100)) / 100.0;
    let mut seen = HashSet::new();
    let mut cands: Vec<C> = vec![];
    for m in pool {
        let id = text(&m["id"]);
        if have.contains(id) || !seen.insert(id) {
            continue;
        }
        let Some(s) = ix.get(id) else { continue };
        let age = day.saturating_sub(s.created_day) as f32;
        let mut score = fit_k * f64::from(session.fit(s, ix)) + 0.5 * f64::from(taste.score(s, ix, &brain.genres));
        if similar.contains(id) {
            score += 0.8;
        }
        score += f64::from(0.6 / (1.0 + s.play_count as f32).sqrt() + 0.5 * (1.0 - age / 30.0).max(0.0));
        match brain.memory.last_heard(id).map(|d| day.saturating_sub(d)) {
            Some(d) if d < 3 => score -= 1.0,
            Some(d) if d < 7 => score -= 0.4,
            _ => {}
        }
        if brain.memory.disliked(id, day) {
            score = disliked_score(score);
        }
        cands.push(C { song: m, artist: text(&m["artist"]).to_string(), score, fresh: fresh(s, day), same_lang: seed_lang != 0 && ix.lang(s) == seed_lang });
    }

    let (n_explore, n_joker, _) = slots(w.batch, w.explore, searching, rng); // normal picks fill the rest
    let quota = if seed_lang == 0 { 0 } else { (f64::from(w.batch.max(0) * w.language.clamp(0, 100)) / 100.0).round() as usize };
    struct Draw {
        used: Vec<bool>,
        per_artist: HashMap<String, i32>,
        same: usize,
        quota: usize,
    }
    // one draw: 0 = exploring (fresh songs only), 1 = wildcard (evenly), 2 = normal; softmax(score / 0.5) but the wildcard
    fn draw(kind: u8, cands: &[C], d: &mut Draw, rng: &mut fastrand::Rng) -> Option<usize> {
        let ok = |i: usize, lang_only: bool| {
            let c = &cands[i];
            !d.used[i] && (kind != 0 || c.fresh) && (!lang_only || c.same_lang) && *d.per_artist.get(&c.artist).unwrap_or(&0) < 2
        };
        let lang_only = d.same < d.quota;
        let mut idx: Vec<usize> = (0..cands.len()).filter(|&i| ok(i, lang_only)).collect();
        if idx.is_empty() && lang_only {
            idx = (0..cands.len()).filter(|&i| ok(i, false)).collect(); // the quota cannot be met
        }
        if idx.is_empty() {
            return None;
        }
        let chosen = if kind == 1 {
            idx[rng.usize(..idx.len())]
        } else {
            let max = idx.iter().map(|&i| cands[i].score).fold(f64::MIN, f64::max);
            let ws: Vec<f64> = idx.iter().map(|&i| ((cands[i].score - max) / 0.5).exp()).collect();
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
        *d.per_artist.entry(cands[chosen].artist.clone()).or_default() += 1;
        if cands[chosen].same_lang {
            d.same += 1;
        }
        Some(chosen)
    }
    let mut per_artist: HashMap<String, i32> = HashMap::new();
    for m in history.iter().rev().take(10).chain(upcoming) {
        *per_artist.entry(text(&m["artist"]).to_string()).or_default() += 1;
    }
    let mut d = Draw { used: vec![false; cands.len()], per_artist, same: 0, quota };
    let mut chosen = vec![];
    for _ in 0..n_explore {
        match draw(0, &cands, &mut d, rng) {
            Some(i) => chosen.push(i),
            None => break, // nothing new or barely played: normal picks fill in
        }
    }
    for _ in 0..n_joker {
        chosen.extend(draw(1, &cands, &mut d, rng));
    }
    while chosen.len() < w.batch.max(0) as usize {
        match draw(2, &cands, &mut d, rng) {
            Some(i) => chosen.push(i),
            None => break,
        }
    }
    let mut out: Vec<Value> = chosen.iter().map(|&i| cands[i].song.clone()).collect();
    rng.shuffle(&mut out);
    out
}

/// Ids from the index worth adding to the pool: up to 2·batch new or barely played songs and 2·batch others, those
/// that fit this sitting best (with some chance). None when the index is empty.
pub fn extra_ids(brain: &Brain, history: &[Value], w: &Weights, _searching: bool, day: u32, rng: &mut fastrand::Rng) -> Vec<String> {
    let ix = &brain.index;
    if ix.is_empty() {
        return vec![];
    }
    let mut tmp = Session::default();
    let session = taste_now(brain, history, &mut tmp);
    let have: HashSet<&str> = history.iter().map(|m| text(&m["id"])).collect();
    let n = 2 * w.batch.max(0) as usize;
    let mut new: Vec<(f32, &str)> = vec![];
    let mut old: Vec<(f32, &str)> = vec![];
    for s in &ix.songs {
        if have.contains(s.id.as_str()) {
            continue;
        }
        let v = session.fit(s, ix) + 0.3 * rng.f32();
        if fresh(s, day) { new.push((v, &s.id)) } else { old.push((v, &s.id)) }
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
        Brain::open(&d.join("cache"), &d.join("data"), DAY)
    }

    fn song(id: &str, title: &str, artist: &str, genre: &str) -> Value {
        json!({"id": id, "title": title, "artist": artist, "genre": genre, "playCount": 5, "created": "2020-01-01T00:00:00Z"})
    }

    fn w(language: i32, explore: i32, batch: i32) -> Weights {
        Weights { language, genre: 50, explore, batch }
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
                    b.memory.record("x", listen::Heard::EarlySkip, d);
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
        b.memory.record("a", listen::Heard::Full, DAY - 1);
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
        assert_eq!(slots(10, 20, false, r), (2, 1, 7));
        assert_eq!(slots(10, 20, true, r), (4, 1, 5));
        assert_eq!(slots(0, 20, false, r), (0, 0, 0));
        // one-song refills (the usual case once the queue is full) explore at the same rate on average
        let n = 2000;
        let (e, j) = (0..n).map(|_| slots(1, 20, false, r)).fold((0, 0), |a, s| (a.0 + s.0, a.1 + s.1));
        assert!((300..500).contains(&e) && (100..300).contains(&j), "{e} {j}");
        let e2: usize = (0..n).map(|_| slots(1, 20, true, r).0).sum();
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
            b.memory.record(&format!("song-{i:08}-abcdef"), if i % 3 == 0 { listen::Heard::EarlySkip } else { listen::Heard::Full }, DAY - (i % 90) as u32);
        }
        for i in 0..2000 {
            b.memory.record_pair(&genre(i), &genre(i + 1), DAY);
        }
        let t = Instant::now();
        b.rebuild_genres(DAY);
        let build = t.elapsed();
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
}
