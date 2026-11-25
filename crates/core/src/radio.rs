//! DJ picks for the queue end.
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use unicode_script::{Script, UnicodeScript};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadioWeights {
    /// % of each batch that must match the seed language (when one is detected)
    pub language: i32,
    /// how strongly the seed genre is preferred, 0-100
    pub genre: i32,
    /// songs added per extension
    pub batch: i32,
}

impl Default for RadioWeights {
    fn default() -> Self {
        RadioWeights { language: 80, genre: 50, batch: 10 }
    }
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

/// Best-effort language of a song: "tr" for Turkish-only letters or a Turkish genre tag, a script name for non-Latin
/// scripts (Cyrillic, Arabic, Han, ...), "" when unknown.
/// shortcut: letter heuristic; Latin-script languages other than Turkish are not told apart.
pub fn guess_language(song: &Value) -> String {
    let genre = text(&song["genre"]).to_lowercase();
    if genre.contains("türk") || genre.contains("turk") {
        return "tr".into();
    }
    let all = format!("{}{}{}", text(&song["title"]), text(&song["artist"]), text(&song["album"]));
    for c in all.chars() {
        if "ğĞşŞıİ".contains(c) {
            return "tr".into();
        }
        let script = c.script();
        if c.is_alphabetic() && !matches!(script, Script::Latin | Script::Common | Script::Inherited) {
            return script.full_name().into();
        }
    }
    String::new()
}

fn most_common(counts: &BTreeMap<String, i32>) -> String {
    let mut best = (String::new(), 0);
    for (k, n) in counts {
        if *n > best.1 {
            best = (k.clone(), *n);
        }
    }
    best.0
}

/// Skip streak: from the 2nd skip in a row each one lowers language and genre by 25 points, at most 3 steps.
pub fn relaxed(mut w: RadioWeights, skips: i32) -> RadioWeights {
    let step = (skips - 1).clamp(0, 3) * 25;
    w.language = (w.language - step).max(0);
    w.genre = (w.genre - step).max(0);
    w
}

/// Picks up to w.batch new songs from `pool` to continue `history` (queue up to the current song). Songs whose id is
/// in `similar_ids` came from getSimilarSongs and are preferred.
pub fn radio_pick(pool: &[Value], history: &[Value], similar_ids: &HashSet<String>, w: &RadioWeights, rng: &mut fastrand::Rng) -> Vec<Value> {
    // An artist with one song in a detectable language is assumed to sing in it on the other songs too.
    let mut artist_lang: HashMap<String, String> = HashMap::new();
    for m in history.iter().chain(pool) {
        let l = guess_language(m);
        let artist = text(&m["artist"]);
        // Navidrome's names for no artist in particular say nothing about the other songs
        if !l.is_empty() && !matches!(artist, "" | "Various Artists" | "[Unknown Artist]") {
            artist_lang.insert(artist.into(), l);
        }
    }
    let lang = |m: &Value| {
        let l = guess_language(m);
        if l.is_empty() { artist_lang.get(text(&m["artist"])).cloned().unwrap_or_default() } else { l }
    };

    // Seed = what the last few songs have in common.
    let (mut lang_count, mut genre_count) = (BTreeMap::new(), BTreeMap::new());
    let mut recent_artists: HashMap<String, i32> = HashMap::new();
    let mut have: HashSet<String> = HashSet::new();
    let n = history.len();
    for (i, m) in history.iter().enumerate() {
        have.insert(text(&m["id"]).into());
        if i + 5 >= n {
            let l = lang(m);
            if !l.is_empty() {
                *lang_count.entry(l).or_insert(0) += 1;
            }
            let g = text(&m["genre"]);
            if !g.is_empty() {
                *genre_count.entry(g.to_string()).or_insert(0) += 1;
            }
        }
        if i + 10 >= n {
            *recent_artists.entry(text(&m["artist"]).into()).or_insert(0) += 1;
        }
    }
    let (seed_lang, seed_genre) = (most_common(&lang_count), most_common(&genre_count));

    struct Candidate {
        song: Value,
        score: f64,
        same_lang: bool,
    }
    let mut cands = vec![];
    for m in pool {
        let id = text(&m["id"]).to_string();
        if !have.insert(id.clone()) {
            continue;
        }
        let mut score = rng.f64();
        if similar_ids.contains(&id) {
            score += 2.0;
        }
        if !seed_genre.is_empty() && text(&m["genre"]) == seed_genre {
            score += 2.0 * f64::from(w.genre) / 100.0;
        }
        score -= 0.7 * f64::from(*recent_artists.get(text(&m["artist"])).unwrap_or(&0));
        cands.push(Candidate { song: m.clone(), score, same_lang: !seed_lang.is_empty() && lang(m) == seed_lang });
    }
    cands.sort_by(|a, b| b.score.total_cmp(&a.score));

    let mut out: Vec<Value> = vec![];
    let mut used: HashSet<String> = HashSet::new();
    let mut per_artist: HashMap<String, i32> = HashMap::new();
    let mut take = |same_lang_only: bool, limit: usize, out: &mut Vec<Value>| {
        for c in &cands {
            if out.len() >= limit {
                return;
            }
            let (id, artist) = (text(&c.song["id"]).to_string(), text(&c.song["artist"]).to_string());
            if (same_lang_only && !c.same_lang) || used.contains(&id) || *per_artist.get(&artist).unwrap_or(&0) >= 2 {
                continue;
            }
            used.insert(id);
            *per_artist.entry(artist).or_insert(0) += 1;
            out.push(c.song.clone());
        }
    };
    let lang_quota = if seed_lang.is_empty() { 0 } else { (f64::from(w.batch * w.language) / 100.0).round() as usize };
    take(true, lang_quota, &mut out);
    take(false, w.batch.max(0) as usize, &mut out);
    rng.shuffle(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn song(id: &str, title: &str, artist: &str, genre: &str) -> Value {
        json!({"id": id, "title": title, "artist": artist, "genre": genre})
    }

    #[test]
    fn language_guess() {
        assert_eq!(guess_language(&song("a", "Şımarık", "Tarkan", "Pop")), "tr");
        assert!(guess_language(&song("a", "Yesterday", "The Beatles", "Pop")).is_empty());
        assert_eq!(guess_language(&song("a", "Yesterday", "X", "Türkçe Pop")), "tr");
        assert!(!guess_language(&song("a", "Кино", "Цой", "Pop")).is_empty());
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
        let all = RadioWeights { language: 100, genre: 50, batch: 10 };
        let picked = radio_pick(&pool, &history, &HashSet::new(), &all, &mut rng);
        assert_eq!(picked.len(), 10);
        let mut per_artist: HashMap<String, i32> = HashMap::new();
        let mut ids = HashSet::new();
        for m in &picked {
            assert!(m["id"].as_str().unwrap().starts_with("tr"));
            let n = per_artist.entry(m["artist"].as_str().unwrap().into()).or_default();
            *n += 1;
            assert!(*n <= 2);
            ids.insert(m["id"].as_str().unwrap().to_string());
        }
        assert_eq!(ids.len(), 10);

        let half = RadioWeights { language: 50, genre: 50, batch: 10 };
        let tr = radio_pick(&pool, &history, &HashSet::new(), &half, &mut rng)
            .iter()
            .filter(|v| v["id"].as_str().unwrap().starts_with("tr"))
            .count();
        assert!(tr >= 5);

        assert!(relaxed(all, 1).language == 100 && relaxed(all, 1).genre == 50);
        assert!(relaxed(all, 2).language == 75 && relaxed(all, 2).genre == 25);
        assert!(relaxed(all, 9).language == 25 && relaxed(all, 9).genre == 0 && relaxed(all, 9).batch == 10);

        // one Turkish song of nobody in particular does not make the others Turkish
        for nobody in ["", "Various Artists", "[Unknown Artist]"] {
            let pool = [song("x", "Şarkı", nobody, "Pop"), song("y", "Song", nobody, "Pop"), song("z", "Gel", "TR Artist 0", "Pop")];
            let known = [song("tr0x", "Şarkı", "TR Artist 0", "Pop")];
            let two = RadioWeights { language: 100, genre: 0, batch: 2 };
            let picked = radio_pick(&pool, &known, &HashSet::from(["y".into()]), &two, &mut rng);
            assert!(picked.iter().all(|v| v["id"] != "y"), "{nobody}: {picked:?}");
        }

        // already queued songs are never picked again
        assert!(radio_pick(&history, &history, &HashSet::new(), &all, &mut rng).is_empty());
    }
}
