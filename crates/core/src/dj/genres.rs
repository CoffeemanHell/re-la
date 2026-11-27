//! How close two genres are: Wikidata's genre tree (CC0, built by tools/genre-tree) as the starting point, then the
//! library (genres sharing artists or songs) and listening (genres heard back to back) take over as evidence grows.
//! The listener's own file (`dj-genres.json`) has the last word.

use crate::dj::index::Index;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

/// Lower case with dotted and dotless i as one letter (so "HIP HOP" meets "hip hop" and "IŞIK" meets "ışık"), "&" as
/// "and", dashes, slashes and runs of spaces as one space.
pub fn norm(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.trim().chars() {
        match c {
            'I' | 'İ' | 'ı' => out.push('i'),
            '&' => out.push_str(" and "),
            '-' | '_' | '/' => out.push(' '),
            c => out.extend(c.to_lowercase()),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The listener's corrections, from `dj-genres.json`: `alias` (same genre), `link` (close), `apart` (not close).
#[derive(Default)]
pub struct UserGenres {
    alias: HashMap<String, String>,
    link: Vec<(String, String)>,
    apart: Vec<(String, String)>,
}

impl UserGenres {
    /// Empty when the file is missing; empty with a warning when it does not parse.
    pub fn load(path: &Path) -> UserGenres {
        let Ok(bytes) = std::fs::read(path) else { return UserGenres::default() };
        let Ok(v) = serde_json::from_slice::<Value>(&bytes) else {
            eprintln!("{}: not valid JSON, ignored", path.display());
            return UserGenres::default();
        };
        let pairs = |key: &str| -> Vec<(String, String)> {
            v[key]
                .as_object()
                .into_iter()
                .flatten()
                .flat_map(|(a, list)| list.as_array().into_iter().flatten().filter_map(|b| Some((norm(a), norm(b.as_str()?)))))
                .collect()
        };
        let alias = v["alias"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(a, b)| Some((norm(a), norm(b.as_str()?))))
            .collect();
        UserGenres { alias, link: pairs("link"), apart: pairs("apart") }
    }

    pub fn is_empty(&self) -> bool {
        self.alias.is_empty() && self.link.is_empty() && self.apart.is_empty()
    }

    fn canon(&self, name: &str) -> String {
        let n = norm(name);
        self.alias.get(&n).cloned().unwrap_or(n)
    }
}

struct Tree {
    by_name: HashMap<String, Vec<u32>>, // Wikidata has several items under some names
    parents: Vec<Vec<u32>>,
    influenced: Vec<Vec<u32>>,
}

fn tree() -> &'static Tree {
    static TREE: OnceLock<Tree> = OnceLock::new();
    TREE.get_or_init(|| {
        let v: Value = serde_json::from_slice(include_bytes!("../../data/genre_tree.json")).unwrap_or_default();
        let list = |e: &Value, k: &str| -> Vec<u32> { e[k].as_array().into_iter().flatten().filter_map(|x| x.as_u64()).map(|x| x as u32).collect() };
        let mut t = Tree { by_name: HashMap::new(), parents: vec![], influenced: vec![] };
        for (i, e) in v["g"].as_array().into_iter().flatten().enumerate() {
            for n in e["n"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                let v = t.by_name.entry(norm(n)).or_default();
                if !v.contains(&(i as u32)) {
                    v.push(i as u32);
                }
            }
            t.parents.push(list(e, "p"));
            t.influenced.push(list(e, "i"));
        }
        t
    })
}

impl Tree {
    /// The nodes of a name and their ancestors up to two steps, each at its shortest distance.
    fn up(&self, nodes: &[u32]) -> HashMap<u32, u8> {
        let mut out: HashMap<u32, u8> = HashMap::new();
        let mut put = |n: u32, d: u8| {
            let e = out.entry(n).or_insert(d);
            *e = (*e).min(d);
        };
        for &n in nodes {
            put(n, 0);
            for &p in &self.parents[n as usize] {
                put(p, 1);
                self.parents[p as usize].iter().for_each(|&g| put(g, 2));
            }
        }
        out
    }

    /// Same item 0.9, parent/child 0.8, siblings 0.7, three steps or less through a shared ancestor 0.5, influence 0.4.
    fn score(&self, a: &[u32], b: &[u32]) -> f32 {
        let (ua, ub) = (self.up(a), self.up(b));
        let mut best = 0.0f32;
        for (x, &da) in &ua {
            if let Some(&db) = ub.get(x) {
                best = best.max(match (da, db) {
                    (0, 0) => 0.9,
                    (1, 1) => 0.7,
                    _ if da + db == 1 => 0.8,
                    _ if da + db <= 3 => 0.5,
                    _ => 0.0,
                });
            }
        }
        let infl = |x: &[u32], y: &[u32]| x.iter().any(|&n| self.influenced[n as usize].iter().any(|m| y.contains(m)));
        if infl(a, b) || infl(b, a) {
            best = best.max(0.4);
        }
        best
    }
}

#[derive(Default)]
struct Facts {
    artists: HashSet<u32>,
    nodes: Vec<u32>,
}

pub struct Genres {
    user_alias: HashMap<String, String>,
    link: HashSet<(String, String)>,
    apart: HashSet<(String, String)>,
    facts: HashMap<String, Facts>,                // by canonical name: the library's genres
    together: HashMap<(String, String), f32>,     // songs tagged with both
    heard: HashMap<(String, String), f32>,        // heard back to back (faded count)
    names: HashMap<String, String>,               // canonical → the library's own spelling
    neighbors: HashMap<String, Vec<(String, f32)>>, // by the library's spelling
}

fn key(a: &str, b: &str) -> (String, String) {
    if a <= b { (a.to_string(), b.to_string()) } else { (b.to_string(), a.to_string()) }
}

fn tokens(n: &str) -> impl Iterator<Item = &str> {
    n.split(' ').filter(|t| t.chars().count() >= 3 && *t != "and" && *t != "music")
}

impl Genres {
    pub fn build(index: &Index, pairs: &[(String, String, f32)], user: &UserGenres) -> Genres {
        let t = tree();
        let canon = |n: &str| user.canon(n);
        let mut g = Genres {
            user_alias: user.alias.clone(),
            link: user.link.iter().map(|(a, b)| key(&canon(a), &canon(b))).collect(),
            apart: user.apart.iter().map(|(a, b)| key(&canon(a), &canon(b))).collect(),
            facts: HashMap::new(),
            together: HashMap::new(),
            heard: HashMap::new(),
            names: HashMap::new(),
            neighbors: HashMap::new(),
        };
        for s in &index.songs {
            let names: Vec<String> = s.genres.iter().map(|&w| canon(index.word(w))).collect();
            for (&w, n) in s.genres.iter().zip(&names) {
                g.names.entry(n.clone()).or_insert_with(|| index.word(w).to_string());
                let f = g.facts.entry(n.clone()).or_default();
                f.artists.extend(&s.artists);
            }
            for i in 0..names.len() {
                for j in i + 1..names.len() {
                    if names[i] != names[j] {
                        *g.together.entry(key(&names[i], &names[j])).or_default() += 1.0;
                    }
                }
            }
        }
        for (n, f) in &mut g.facts {
            f.nodes = t.by_name.get(n).cloned().unwrap_or_default();
        }
        for (a, b, n) in pairs {
            *g.heard.entry(key(&canon(a), &canon(b))).or_default() += n;
        }

        // Candidate pairs: sharing an artist, a song, a tree ancestor, an influence, a word, a listening or a user link.
        let mut cand: HashSet<(String, String)> = HashSet::new();
        let mut by_artist: HashMap<u32, Vec<&String>> = HashMap::new();
        let mut by_up: HashMap<u32, Vec<(&String, u8)>> = HashMap::new();
        let mut by_token: HashMap<&str, Vec<&String>> = HashMap::new();
        let mut by_node: HashMap<u32, &String> = HashMap::new();
        for (n, f) in &g.facts {
            for &a in &f.artists {
                by_artist.entry(a).or_default().push(n);
            }
            if f.nodes.is_empty() {
                tokens(n).for_each(|tk| by_token.entry(tk).or_default().push(n));
            }
            for &node in &f.nodes {
                by_node.insert(node, n);
            }
            for (x, d) in t.up(&f.nodes) {
                by_up.entry(x).or_default().push((n, d));
            }
        }
        let mut add_all = |list: &[&String]| {
            for i in 0..list.len() {
                for j in i + 1..list.len() {
                    if list[i] != list[j] {
                        cand.insert(key(list[i], list[j]));
                    }
                }
            }
        };
        by_artist.values().for_each(|l| add_all(l));
        by_token.values().for_each(|l| add_all(l));
        for l in by_up.values() {
            for i in 0..l.len() {
                for j in i + 1..l.len() {
                    if l[i].1 + l[j].1 <= 3 && l[i].0 != l[j].0 {
                        cand.insert(key(l[i].0, l[j].0));
                    }
                }
            }
        }
        for (n, f) in &g.facts {
            for &node in &f.nodes {
                for &o in &t.influenced[node as usize] {
                    if let Some(m) = by_node.get(&o).filter(|m| *m != &n) {
                        cand.insert(key(n, m));
                    }
                }
            }
        }
        cand.extend(g.together.keys().cloned());
        cand.extend(g.heard.keys().cloned());
        cand.extend(g.link.iter().cloned());

        let mut lists: HashMap<String, Vec<(String, f32)>> = HashMap::new();
        for (a, b) in cand {
            if !g.facts.contains_key(&a) || !g.facts.contains_key(&b) {
                continue;
            }
            let s = g.score(&a, &b);
            if s > 0.0 {
                lists.entry(a.clone()).or_default().push((b.clone(), s));
                lists.entry(b).or_default().push((a, s));
            }
        }
        for (n, mut l) in lists {
            l.sort_by(|x, y| y.1.total_cmp(&x.1));
            l.truncate(10);
            let spelled = |c: &String| g.names.get(c).cloned().unwrap_or_else(|| c.clone());
            let l = l.iter().map(|(c, s)| (spelled(c), *s)).collect();
            g.neighbors.insert(spelled(&n), l);
        }
        g
    }

    /// Both canonical (normalized, aliases followed).
    fn score(&self, a: &str, b: &str) -> f32 {
        if a == b {
            return 1.0;
        }
        let k = key(a, b);
        if self.apart.contains(&k) {
            return 0.0;
        }
        if self.link.contains(&k) {
            return 0.8;
        }
        let t = tree();
        let nodes = |n: &str| t.by_name.get(n).map_or(&[][..], Vec::as_slice);
        let tree_s = t.score(nodes(a), nodes(b));
        let empty = HashSet::new();
        let arts = |n: &str| self.facts.get(n).map_or(&empty, |f| &f.artists);
        let (xa, xb) = (arts(a), arts(b));
        let jaccard = if xa.is_empty() || xb.is_empty() { 0.0 } else { xa.intersection(xb).count() as f32 / xa.union(xb).count() as f32 };
        let c = self.together.get(&k).copied().unwrap_or(0.0);
        let lib = jaccard.max(c / (c + 3.0));
        let n = self.heard.get(&k).copied().unwrap_or(0.0);
        let listen = n / (n + 3.0);
        let evidence = xa.len().min(xb.len()) as f32 + n;
        let w1 = 0.6 - 0.4 * (evidence / 20.0).min(1.0);
        let w = (1.0 - w1) / 2.0;
        let mut s = w1 * tree_s + w * lib + w * listen;
        if tree_s == 0.0 && tokens(a).any(|x| tokens(b).any(|y| x == y)) {
            s += 0.3;
        }
        s.clamp(0.0, 1.0)
    }

    /// 1.0 for the same genre, 0 for unrelated ones. Works for names outside the library too (tree and words only).
    pub fn sim(&self, a: &str, b: &str) -> f32 {
        let canon = |n: &str| {
            let n = norm(n);
            self.user_alias.get(&n).cloned().unwrap_or(n)
        };
        self.score(&canon(a), &canon(b))
    }

    /// The closest library genres, at most 10, closest first.
    pub fn neighbors(&self, g: &str) -> &[(String, f32)] {
        self.neighbors.get(g).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn index(songs: &[(&str, &str, &[&str])]) -> Index {
        let p = std::env::temp_dir().join(format!("re-la-djgen-{}-{}", std::process::id(), fastrand::u64(..)));
        let mut ix = Index::open(p.join("i.json"));
        for (i, (artist, _, genres)) in songs.iter().enumerate() {
            let g: Vec<Value> = genres.iter().map(|g| json!({"name": g})).collect();
            ix.upsert(&json!({"id": format!("s{i}"), "artist": artist, "genres": g}), 1);
        }
        ix
    }

    fn user(v: Value) -> UserGenres {
        let p = std::env::temp_dir().join(format!("re-la-djgen-u-{}-{}.json", std::process::id(), fastrand::u64(..)));
        std::fs::write(&p, v.to_string()).unwrap();
        UserGenres::load(&p)
    }

    #[test]
    fn tree_knows_metal() {
        let g = Genres::build(&index(&[]), &[], &UserGenres::default());
        assert!(g.sim("Heavy Metal", "Hard Rock") > 0.0);
        assert_eq!(g.sim("Heavy Metal", "heavy  metal"), 1.0);
    }

    #[test]
    fn norm_variants() {
        assert_eq!(norm("Drum & Bass"), norm("drum and bass"));
        assert_eq!(norm("HIP-HOP"), norm("hip hop"));
        assert_eq!(norm("IŞIK"), norm("ışık"));
        assert_eq!(norm("İz"), norm("iz"));
        assert_eq!(norm("Türkçe ROCK"), "türkçe rock");
    }

    #[test]
    fn library_links_unknown() {
        let ix = index(&[("A1", "", &["Zeuhl X"]), ("A1", "", &["Prog Y"]), ("A2", "", &["Zeuhl X"]), ("A2", "", &["Prog Y"]),
            ("A3", "", &["Zeuhl X"]), ("A3", "", &["Prog Y"])]);
        let g = Genres::build(&ix, &[], &UserGenres::default());
        assert!(g.sim("Zeuhl X", "Prog Y") > 0.0);
        assert_eq!(g.neighbors("Zeuhl X")[0].0, "Prog Y");
    }

    #[test]
    fn token_overlap() {
        let g = Genres::build(&index(&[]), &[], &UserGenres::default());
        let s = g.sim("Fooqx Metalzz", "Barqx Metalzz");
        assert!(s > 0.0 && s <= 0.3, "{s}");
    }

    #[test]
    fn listening_link() {
        let ix = index(&[("A1", "", &["Qqa"]), ("A2", "", &["Qqb"])]);
        let g = Genres::build(&ix, &[("Qqa".into(), "Qqb".into(), 10.0)], &UserGenres::default());
        assert!(g.sim("Qqa", "Qqb") > 0.0);
        assert!(g.neighbors("Qqb").iter().any(|(n, _)| n == "Qqa"));
    }

    #[test]
    fn user_link_apart_alias() {
        let ix = index(&[("A1", "", &["Qqa"]), ("A2", "", &["Qqb"]), ("A3", "", &["Heavy Metal"]), ("A4", "", &["Hard Rock"])]);
        let u = user(json!({"alias": {"Türkçe Rock": "Anatolian rock"}, "link": {"Qqa": ["Qqb"]},
            "apart": {"Heavy Metal": ["Hard Rock"]}}));
        let g = Genres::build(&ix, &[], &u);
        assert_eq!(g.sim("Qqa", "Qqb"), 0.8);
        assert_eq!(g.sim("Hard Rock", "Heavy Metal"), 0.0);
        assert!(g.neighbors("Heavy Metal").iter().all(|(n, _)| n != "Hard Rock"));
        assert_eq!(g.sim("Türkçe Rock", "Anatolian rock"), 1.0);
    }

    #[test]
    fn user_file_corrupt_or_missing() {
        let p = std::env::temp_dir().join(format!("re-la-djgen-bad-{}.json", std::process::id()));
        std::fs::write(&p, "{nope").unwrap();
        assert!(UserGenres::load(&p).is_empty());
        assert!(UserGenres::load(&p.with_extension("missing")).is_empty());
    }

    #[test]
    fn multi_genre_song() {
        let ix = index(&[("A1", "", &["Qqa", "Qqb", "Qqc"])]);
        let g = Genres::build(&ix, &[], &UserGenres::default());
        assert!(g.sim("Qqa", "Qqc") > 0.0 && g.sim("Qqb", "Qqc") > 0.0);
    }
}
