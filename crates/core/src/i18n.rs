//! The language of the core's own text (status line, login errors): Turkish when the UI is Turkish, English otherwise.
//! The front end sets it once at start, from the language its own UI uses.
use std::sync::atomic::{AtomicBool, Ordering};

static ENGLISH: AtomicBool = AtomicBool::new(false);

/// "tr", "tr-TR", "tr_TR": Turkish; any other language: English.
pub fn is_english(lang: &str) -> bool {
    !lang.trim().to_ascii_lowercase().starts_with("tr")
}

pub fn set_language(lang: &str) {
    ENGLISH.store(is_english(lang), Ordering::Relaxed);
}

pub fn english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

/// The text in the current language.
pub fn tr(tr: &'static str, en: &'static str) -> &'static str {
    if english() { en } else { tr }
}

#[cfg(test)]
mod tests {
    #[test]
    fn turkish_only_for_turkish() {
        for l in ["tr", "tr-TR", "tr_TR", "TR"] {
            assert!(!super::is_english(l), "{l}");
        }
        for l in ["en", "en-US", "de-DE", "", "C"] {
            assert!(super::is_english(l), "{l}");
        }
    }
}
