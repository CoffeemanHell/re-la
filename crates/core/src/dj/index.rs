//! The whole library in a compact form, on disk: what the DJ scores songs by. Names live once in a word table; songs
//! hold their numbers (0 = none). No titles or covers: a picked song is fetched whole with getSong.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use unicode_script::{Script, UnicodeScript};

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

/// Best-effort language of a song: "tr" for Turkish-only letters or a Turkish genre tag, a script name for non-Latin
/// scripts (Cyrillic, Arabic, Han, ...), "" when unknown.
/// shortcut: letter heuristic; Latin-script languages other than Turkish are not told apart.
pub fn guess_language(song: &Value) -> String {
    let names = song["genres"].as_array().into_iter().flatten().map(|g| text(&g["name"]));
    let genre = std::iter::once(text(&song["genre"])).chain(names).collect::<Vec<_>>().join(" ").to_lowercase();
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

/// Navidrome's names for no artist in particular: they say nothing about the artist's other songs.
pub(crate) fn nobody(artist: &str) -> bool {
    matches!(artist, "" | "Various Artists" | "[Unknown Artist]")
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Song {
    pub id: String,
    pub artists: Vec<u32>,
    pub album: u32, // albumId
    pub genres: Vec<u32>,
    pub moods: Vec<u32>,
    pub year: u16,
    pub bpm: u16,
    pub created_day: u32,
    pub lang: u32, // guessed from this song alone; Index::lang adds the artist's
    pub play_count: u32,
    pub starred: bool,
    pub rating: u8,
}

#[derive(Serialize, Deserialize, Default)]
struct File {
    v: u32,
    words: Vec<String>,
    songs: Vec<Song>,
    full_day: u32,
}

pub struct Index {
    path: PathBuf,
    words: Vec<String>,
    word_ids: HashMap<String, u32>,
    pub songs: Vec<Song>,
    by_id: HashMap<String, usize>,
    albums: HashSet<u32>,
    artist_lang: HashMap<u32, u32>,
    /// Day of the last complete library walk.
    pub full_day: u32,
}

fn day_of(iso: &str) -> Option<u32> {
    let t = time::OffsetDateTime::parse(iso, &time::format_description::well_known::Rfc3339).ok()?;
    u32::try_from(t.unix_timestamp() / 86_400).ok()
}

impl Index {
    /// From disk; empty when the file is missing, broken or of another version.
    pub fn open(path: PathBuf) -> Index {
        let f = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<File>(&b).ok())
            .filter(|f| f.v == 1 && !f.words.is_empty())
            .unwrap_or(File { v: 1, words: vec![String::new()], ..Default::default() });
        let mut ix = Index {
            path,
            word_ids: f.words.iter().enumerate().map(|(i, w)| (w.clone(), i as u32)).collect(),
            words: f.words,
            songs: vec![],
            by_id: HashMap::new(),
            albums: HashSet::new(),
            artist_lang: HashMap::new(),
            full_day: f.full_day,
        };
        for s in f.songs {
            ix.put(s);
        }
        ix
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(d) = self.path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let f = File { v: 1, words: self.words.clone(), songs: self.songs.clone(), full_day: self.full_day };
        crate::disk::write_atomic_private(&self.path, &serde_json::to_vec(&f).map_err(std::io::Error::other)?)
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn words_len(&self) -> usize {
        self.words.len()
    }

    pub fn word(&self, i: u32) -> &str {
        self.words.get(i as usize).map_or("", String::as_str)
    }

    pub fn word_id(&self, s: &str) -> Option<u32> {
        self.word_ids.get(s).copied()
    }

    fn intern(&mut self, s: &str) -> u32 {
        if s.is_empty() {
            return 0;
        }
        if let Some(&i) = self.word_ids.get(s) {
            return i;
        }
        let i = self.words.len() as u32;
        self.words.push(s.to_string());
        self.word_ids.insert(s.to_string(), i);
        i
    }

    fn put(&mut self, s: Song) {
        self.albums.insert(s.album);
        if s.lang != 0 {
            for &a in &s.artists {
                if !nobody(self.word(a)) {
                    self.artist_lang.insert(a, s.lang);
                }
            }
        }
        match self.by_id.get(&s.id) {
            Some(&i) => self.songs[i] = s,
            None => {
                self.by_id.insert(s.id.clone(), self.songs.len());
                self.songs.push(s);
            }
        }
    }

    /// Adds or refreshes a song from its Subsonic JSON (OpenSubsonic fields when there). No date: `today`.
    pub fn upsert(&mut self, song: &Value, today: u32) {
        let id = text(&song["id"]);
        if id.is_empty() {
            return;
        }
        let names = |list: &Value, key: Option<&str>| -> Vec<String> {
            list.as_array()
                .map(|a| a.iter().map(|v| text(key.map_or(v, |k| &v[k])).to_string()).filter(|s| !s.is_empty()).collect())
                .unwrap_or_default()
        };
        let mut genres = names(&song["genres"], Some("name"));
        if genres.is_empty() && !text(&song["genre"]).is_empty() {
            genres.push(text(&song["genre"]).into());
        }
        let mut artists = names(&song["artists"], Some("name"));
        if artists.is_empty() && !text(&song["artist"]).is_empty() {
            artists.push(text(&song["artist"]).into());
        }
        let moods = names(&song["moods"], None);
        let lang = guess_language(song);
        let num = |k: &str| song[k].as_u64().unwrap_or(0);
        let s = Song {
            id: id.into(),
            artists: artists.iter().map(|a| self.intern(a)).collect(),
            album: self.intern(text(&song["albumId"])),
            genres: genres.iter().map(|g| self.intern(g)).collect(),
            moods: moods.iter().map(|m| self.intern(m)).collect(),
            year: num("year").min(u16::MAX as u64) as u16,
            bpm: num("bpm").min(u16::MAX as u64) as u16,
            created_day: day_of(text(&song["created"])).unwrap_or(today),
            lang: self.intern(&lang),
            play_count: num("playCount").min(u32::MAX as u64) as u32,
            starred: !song["starred"].is_null(),
            rating: num("userRating").min(5) as u8,
        };
        self.put(s);
    }

    /// Keeps only these songs (after a complete library walk).
    pub fn retain(&mut self, ids: &HashSet<String>) {
        let songs = std::mem::take(&mut self.songs);
        self.by_id.clear();
        self.albums.clear();
        self.artist_lang.clear(); // as reopening would leave it
        for s in songs.into_iter().filter(|s| ids.contains(&s.id)) {
            self.put(s);
        }
    }

    /// Just the songs and words (what genre closeness reads), to build from on another thread.
    pub fn genre_view(&self) -> Index {
        Index {
            path: self.path.clone(),
            words: self.words.clone(),
            word_ids: HashMap::new(),
            songs: self.songs.clone(),
            by_id: HashMap::new(),
            albums: HashSet::new(),
            artist_lang: HashMap::new(),
            full_day: self.full_day,
        }
    }

    pub fn get(&self, id: &str) -> Option<&Song> {
        self.by_id.get(id).map(|&i| &self.songs[i])
    }

    pub fn has_album(&self, album_id: &str) -> bool {
        self.word_id(album_id).is_some_and(|w| self.albums.contains(&w))
    }

    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }

    /// The song's language, or else its artist's from their other songs; 0 when unknown.
    pub fn lang(&self, s: &Song) -> u32 {
        if s.lang != 0 {
            return s.lang;
        }
        s.artists.iter().find_map(|a| self.artist_lang.get(a).copied()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("re-la-djidx-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("dj-index.json")
    }

    fn song(id: &str, title: &str, artist: &str, genre: &str) -> Value {
        json!({"id": id, "title": title, "artist": artist, "genre": genre, "albumId": "al1"})
    }

    // play counts, stars and ratings: the owner's alone
    #[cfg(unix)]
    #[test]
    fn saved_private() {
        use std::os::unix::fs::PermissionsExt;
        let p = tmp("private");
        let ix = Index::open(p.clone());
        ix.save().unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn language_guess() {
        assert_eq!(guess_language(&song("a", "Şımarık", "Tarkan", "Pop")), "tr");
        assert!(guess_language(&song("a", "Yesterday", "The Beatles", "Pop")).is_empty());
        assert_eq!(guess_language(&song("a", "Yesterday", "X", "Türkçe Pop")), "tr");
        assert!(!guess_language(&song("a", "Кино", "Цой", "Pop")).is_empty());
    }

    #[test]
    fn upsert_reads_opensubsonic() {
        let mut ix = Index::open(tmp("os"));
        ix.upsert(
            &json!({"id": "s1", "title": "T", "artists": [{"id": "a", "name": "Iron"}], "artist": "Iron", "albumId": "al9",
                "genres": [{"name": "Heavy Metal"}, {"name": "Hard Rock"}], "genre": "Heavy Metal", "moods": ["dark"],
                "bpm": 120, "year": 1982, "created": "2026-11-01T10:00:00Z", "playCount": 3, "starred": "2026-01-01T00:00:00Z",
                "userRating": 4}),
            20_000,
        );
        let s = ix.get("s1").unwrap();
        let names = |v: &[u32]| v.iter().map(|&w| ix.word(w).to_string()).collect::<Vec<_>>();
        assert_eq!(names(&s.genres), ["Heavy Metal", "Hard Rock"]);
        assert_eq!(names(&s.artists), ["Iron"]);
        assert_eq!(names(&s.moods), ["dark"]);
        assert_eq!((s.bpm, s.year, s.play_count, s.starred, s.rating), (120, 1982, 3, true, 4));
        assert_eq!(s.created_day, 20_758); // 2026-11-01
        assert!(ix.has_album("al9") && !ix.has_album("al1"));
    }

    #[test]
    fn upsert_plain_genre() {
        let mut ix = Index::open(tmp("plain"));
        ix.upsert(&song("a", "T", "X", "Pop"), 1);
        assert_eq!(ix.get("a").unwrap().genres.len(), 1);
        ix.upsert(&json!({"id": "b", "genre": "", "genres": []}), 1);
        assert!(ix.get("b").unwrap().genres.is_empty() && ix.get("b").unwrap().artists.is_empty());
        assert_eq!(ix.get("b").unwrap().created_day, 1); // no date: today
    }

    #[test]
    fn upsert_updates() {
        let mut ix = Index::open(tmp("upd"));
        let mut s = song("a", "T", "X", "Pop");
        s["playCount"] = 1.into();
        ix.upsert(&s, 1);
        s["playCount"] = 5.into();
        ix.upsert(&s, 1);
        assert_eq!(ix.songs.len(), 1);
        assert_eq!(ix.get("a").unwrap().play_count, 5);
    }

    // a Latin-letter song of an artist who sings in Turkish elsewhere is Turkish too, whichever came first
    #[test]
    fn artist_language() {
        let mut ix = Index::open(tmp("lang"));
        ix.upsert(&song("y", "Gel", "TR Artist", "Pop"), 1);
        ix.upsert(&song("x", "Şarkı", "TR Artist", "Pop"), 1);
        let tr = ix.word_id("tr").unwrap();
        assert_eq!(ix.lang(ix.get("y").unwrap()), tr);
        for nobody in ["", "Various Artists", "[Unknown Artist]"] {
            let mut ix = Index::open(tmp("nobody"));
            ix.upsert(&song("x", "Şarkı", nobody, "Pop"), 1);
            ix.upsert(&song("y", "Song", nobody, "Pop"), 1);
            assert_eq!(ix.lang(ix.get("y").unwrap()), 0, "{nobody}");
        }
    }

    #[test]
    fn retain_drops() {
        let mut ix = Index::open(tmp("retain"));
        for id in ["a", "b", "c"] {
            ix.upsert(&song(id, "T", "X", "Pop"), 1);
        }
        ix.retain(&["a".to_string(), "c".to_string()].into());
        assert!(ix.get("b").is_none() && ix.get("c").is_some() && ix.songs.len() == 2);
    }

    #[test]
    fn roundtrip() {
        let p = tmp("round");
        let mut ix = Index::open(p.clone());
        ix.upsert(&song("x", "Şarkı", "TR Artist", "Pop"), 1);
        ix.upsert(&song("y", "Gel", "TR Artist", "Rock"), 1);
        ix.full_day = 9;
        ix.save().unwrap();
        let ix = Index::open(p);
        assert_eq!((ix.songs.len(), ix.full_day, ix.is_empty()), (2, 9, false));
        assert_eq!(ix.word(ix.get("y").unwrap().genres[0]), "Rock");
        assert_eq!(ix.lang(ix.get("y").unwrap()), ix.word_id("tr").unwrap());
        assert!(ix.has_album("al1"));
    }

    #[test]
    fn corrupt_file() {
        let p = tmp("corrupt");
        std::fs::write(&p, "{nope").unwrap();
        let mut ix = Index::open(p.clone());
        assert!(ix.is_empty());
        ix.upsert(&song("a", "T", "X", "Pop"), 1);
        ix.save().unwrap();
        assert_eq!(Index::open(p).songs.len(), 1);
    }
}
