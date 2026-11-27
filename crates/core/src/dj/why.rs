//! Why the DJ picked a song, in a few words: from the biggest part of its score.

use crate::dj::Parts;
use crate::dj::index::Index;
use crate::dj::session::Tag;

/// A tag as a listener would name it, in English or Turkish.
pub fn tag_label(t: Tag, ix: &Index, en: bool) -> String {
    match t {
        Tag::Genre(w) | Tag::Artist(w) | Tag::Mood(w) => ix.word(w).to_string(),
        Tag::Lang(w) => match ix.word(w) {
            "tr" if en => "Turkish".into(),
            "tr" => "Türkçe".into(),
            other => other.to_string(),
        },
        Tag::Decade(d) => {
            let year = u32::from(d) * 10;
            if en {
                format!("{year}s")
            } else {
                // seksenler, doksanlar: the suffix follows the spoken tens
                let lar = matches!((year / 10) % 10, 1 | 3 | 4 | 6 | 9);
                format!("{year}'{}", if lar { "lar" } else { "ler" })
            }
        }
        Tag::Bpm(b) => format!("{} BPM", u32::from(b) * 10),
    }
}

/// The reason under a picked song, in English or Turkish. `kind` is the draw (1 = wildcard); the rest say what made
/// up its score.
#[allow(clippy::too_many_arguments)]
pub fn sentence(
    en: bool,
    kind: u8,
    p: &Parts,
    fit_tag: Option<String>,
    taste_name: Option<String>,
    seed_title: &str,
    age_days: u32,
    last_heard_days: Option<u32>,
    new_days: u32,
) -> String {
    if kind == 1 {
        return if en { "Surprise" } else { "Sürpriz" }.into();
    }
    if p.feedback > 0.0 {
        return if en { "You asked for more like this" } else { "Daha çok istediğin bir şarkı" }.into();
    }
    let parts = [(p.fit, 0), (p.taste, 1), (p.similar, 2), (p.fresh, 3)];
    let best = parts.iter().filter(|(v, _)| *v > 0.0).max_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, i)| *i);
    match best {
        Some(0) if let Some(t) = fit_tag => {
            if en { format!("In line with the {t} you just heard") } else { format!("Az önce dinlediğin {t} çizgisinde") }
        }
        Some(1) if let Some(x) = taste_name => {
            if en { format!("You often play {x}") } else { format!("Sık dinlediğin {x}") }
        }
        Some(2) if !seed_title.is_empty() => {
            if en { format!("Similar to {seed_title}") } else { format!("{seed_title} şarkısına benzer") }
        }
        Some(3) if age_days < new_days => if en { "New in the library" } else { "Kütüphaneye yeni eklendi" }.into(),
        Some(3) if let Some(d) = last_heard_days.filter(|d| *d >= 60) => {
            let n = d / 30;
            if en { format!("Not played for {n} months") } else { format!("{n} aydır çalmadın") }
        }
        Some(3) => if en { "One you rarely play" } else { "Az dinlediğin bir şarkı" }.into(),
        _ => if en { "Fits this sitting" } else { "Bu oturuma uygun" }.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(fit: f64, similar: f64, fresh: f64) -> Parts {
        Parts { fit, similar, fresh, ..Parts::default() }
    }

    #[test]
    fn why_surprise_and_similar() {
        assert_eq!(sentence(false, 1, &parts(2.0, 0.0, 0.0), None, None, "Song A", 400, None, 30), "Sürpriz");
        assert_eq!(sentence(false, 2, &parts(0.2, 0.8, 0.1), None, None, "Song A", 400, None, 30), "Song A şarkısına benzer");
        assert_eq!(sentence(false, 2, &parts(0.9, 0.8, 0.1), Some("Shoegaze".into()), None, "Song A", 400, None, 30), "Az önce dinlediğin Shoegaze çizgisinde");
    }

    #[test]
    fn why_months() {
        let p = parts(0.1, 0.0, 0.6);
        assert_eq!(sentence(false, 2, &p, None, None, "", 400, Some(95), 30), "3 aydır çalmadın");
        assert_eq!(sentence(false, 2, &p, None, None, "", 5, Some(95), 30), "Kütüphaneye yeni eklendi");
        assert_eq!(sentence(true, 2, &p, None, None, "", 400, Some(95), 30), "Not played for 3 months");
    }

    #[test]
    fn decade_suffix() {
        let ix = Index::open(std::env::temp_dir().join("re-la-why-none.json"));
        assert_eq!(tag_label(Tag::Decade(198), &ix, false), "1980'ler");
        assert_eq!(tag_label(Tag::Decade(199), &ix, false), "1990'lar");
    }
}
