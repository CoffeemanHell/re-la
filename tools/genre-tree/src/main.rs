//! Builds crates/core/data/genre_tree.json from Wikidata (CC0): music genres, their English/Turkish names and aliases,
//! "subclass of" (P279) and "influenced by" (P737) links between them. Run by hand now and then:
//!     cargo run -p genre-tree --release -- crates/core/data/genre_tree.json

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

const URL: &str = "https://query.wikidata.org/sparql";
const GENRE: &str = "?g wdt:P31 wd:Q188451 .";

async fn ask(http: &reqwest::Client, q: &str) -> Result<Vec<Value>, String> {
    let url = reqwest::Url::parse_with_params(URL, [("query", q), ("format", "json")]).map_err(|e| e.to_string())?;
    for _ in 0..6 {
        let r = http.get(url.clone()).send().await.map_err(|e| e.to_string())?;
        if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let wait = r.headers().get("retry-after").and_then(|v| v.to_str().ok()?.parse().ok()).unwrap_or(61);
            eprintln!("429, waiting");
            tokio::time::sleep(Duration::from_secs(wait + 1)).await;
            continue;
        }
        let v: Value = r.error_for_status().map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
        let rows = v["results"]["bindings"].as_array().cloned().unwrap_or_default();
        eprintln!("{} rows", rows.len());
        tokio::time::sleep(Duration::from_secs(61)).await; // the service may allow one query a minute
        return Ok(rows);
    }
    Err("Wikidata kept refusing (429)".into())
}

/// "Q123" from a row's entity URL.
fn qid(row: &Value, k: &str) -> String {
    row[k]["value"].as_str().unwrap_or("").rsplit('/').next().unwrap_or("").to_string()
}

#[derive(Serialize, Debug, PartialEq)]
struct Genre {
    n: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    p: Vec<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    i: Vec<usize>,
}

#[derive(Serialize)]
struct Doc<'a> {
    license: &'static str,
    made: String,
    g: &'a [Genre],
}

/// Genres with at least one link, in QID order; links point at their place in the list.
fn build(names: &BTreeMap<String, BTreeSet<String>>, parents: &[(String, String)], influences: &[(String, String)]) -> Vec<Genre> {
    // a genre with no link to any other says nothing about similarity: left out to keep the file small
    let linked: BTreeSet<&String> = parents.iter().chain(influences).flat_map(|(a, b)| [a, b]).collect();
    let keep: Vec<&String> = names.keys().filter(|q| linked.contains(q)).collect();
    let at: HashMap<&String, usize> = keep.iter().enumerate().map(|(i, q)| (*q, i)).collect();
    let mut g: Vec<Genre> = keep.iter().map(|q| Genre { n: names[*q].iter().cloned().collect(), p: vec![], i: vec![] }).collect();
    for (pairs, parent) in [(parents, true), (influences, false)] {
        for (a, b) in pairs {
            if let (Some(&x), Some(&y)) = (at.get(a), at.get(b))
                && x != y
            {
                if parent { g[x].p.push(y) } else { g[x].i.push(y) }
            }
        }
    }
    for e in &mut g {
        for l in [&mut e.p, &mut e.i] {
            l.sort_unstable();
            l.dedup();
        }
    }
    g
}

async fn run(out: &str) -> Result<(), String> {
    let http = reqwest::Client::builder()
        .user_agent("re-la-genre-tree/1.0")
        .timeout(Duration::from_secs(300))
        .default_headers([(reqwest::header::ACCEPT, "application/sparql-results+json".parse().unwrap())].into_iter().collect())
        .build()
        .map_err(|e| e.to_string())?;
    let mut names: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for prop in ["rdfs:label", "skos:altLabel"] {
        for row in ask(&http, &format!(r#"SELECT ?g ?l WHERE {{ {GENRE} ?g {prop} ?l FILTER(LANG(?l) IN ("en", "tr")) }}"#)).await? {
            let n = row["l"]["value"].as_str().unwrap_or("").trim().to_lowercase();
            if !n.is_empty() && n.chars().count() <= 60 {
                names.entry(qid(&row, "g")).or_default().insert(n);
            }
        }
    }
    let mut links: Vec<Vec<(String, String)>> = vec![];
    for prop in ["P279", "P737"] {
        let rows = ask(&http, &format!("SELECT ?g ?o WHERE {{ {GENRE} ?g wdt:{prop} ?o . ?o wdt:P31 wd:Q188451 . }}")).await?;
        links.push(rows.iter().map(|r| (qid(r, "g"), qid(r, "o"))).collect());
    }
    let g = build(&names, &links[0], &links[1]);
    let made = time::OffsetDateTime::now_utc().date().to_string();
    let doc = Doc { license: "CC0 (Wikidata)", made, g: &g };
    let tmp = format!("{out}.tmp");
    std::fs::write(&tmp, serde_json::to_vec(&doc).map_err(|e| e.to_string())?).map_err(|e| format!("{tmp}: {e}"))?;
    std::fs::rename(&tmp, out).map_err(|e| format!("{out}: {e}"))?; // a failed write leaves the old catalog
    let count = |f: fn(&Genre) -> usize| g.iter().map(f).sum::<usize>();
    println!("{} genres, {} parent links, {} influence links", g.len(), count(|e| e.p.len()), count(|e| e.i.len()));
    Ok(())
}

fn main() {
    let Some(out) = std::env::args().nth(1) else {
        eprintln!("usage: genre-tree <out.json>");
        std::process::exit(2);
    };
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio");
    if let Err(e) = rt.block_on(run(&out)) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(a: &str, b: &str) -> (String, String) {
        (a.into(), b.into())
    }

    #[test]
    fn links_point_into_the_kept_list() {
        let names: BTreeMap<String, BTreeSet<String>> = [("Q1", "rock"), ("Q2", "hard rock"), ("Q3", "lonely"), ("Q4", "blues")]
            .into_iter()
            .map(|(q, n)| (q.to_string(), BTreeSet::from([n.to_string()])))
            .collect();
        let g = build(&names, &[s("Q2", "Q1"), s("Q2", "Q1"), s("Q1", "Q1"), s("Q9", "Q1")], &[s("Q1", "Q4")]);
        // Q3 has no link: left out; Q9 has no name: its link goes too
        assert_eq!(g.iter().map(|e| e.n[0].as_str()).collect::<Vec<_>>(), ["rock", "hard rock", "blues"]);
        assert_eq!((g[1].p.clone(), g[0].p.clone(), g[0].i.clone()), (vec![0], vec![], vec![2]));
        let json = serde_json::to_string(&g[2]).unwrap();
        assert_eq!(json, r#"{"n":["blues"]}"#);
    }
}
