//! The UIs keep their own copies of the DJ knob tables (shared/DjKnobs.qml, android DjKnobs.kt). These tests keep their
//! defaults equal to the core's, so a changed default shows up here and not as a reset that goes nowhere.

#[cfg(test)]
mod tests {
    use super::super::knobs::RANGES;

    const QML: &str = include_str!("../../../rela_desktop/themes/shared/DjKnobs.qml");
    const KOTLIN: &str = include_str!("../../../../android/app/src/main/kotlin/app/rela/ui/library/DjKnobs.kt");

    /// The five switches (the core reads them as bool options, default true).
    const SWITCHES: &[&str] = &["djSkipOn", "djSearchOn", "djDislikeOn", "djRepeatOn", "djWildcardOn"];

    /// The value after `marker` in the table entry that names `key`, up to the entry's end.
    fn field(src: &str, key: &str, marker: &str) -> Option<String> {
        let at = src.find(&format!("\"{key}\""))?;
        let rest = &src[at + key.len() + 2..];
        let end = ["{ key:", "Knob(\"", "DjSwitch(\""].iter().filter_map(|m| rest.find(m)).min().unwrap_or(rest.len());
        let entry = &rest[..end];
        let from = entry.find(marker)? + marker.len();
        let value: String = entry[from..].chars().take_while(|c| !matches!(c, ',' | '}' | ')' | '\n')).collect();
        Some(value.trim().to_string())
    }

    #[test]
    fn ui_tables_carry_the_cores_defaults() {
        let mut want: Vec<(&str, String)> = RANGES.iter().map(|r| (r.0, r.1.to_string())).collect();
        want.extend([("radioLanguage", "80"), ("radioGenre", "50"), ("radioExplore", "20"), ("radioBatch", "10")]
            .map(|(k, v)| (k, v.to_string())));
        for (key, def) in &want {
            assert_eq!(field(QML, key, "def: "), Some(def.clone()), "qml {key}");
            assert_eq!(field(KOTLIN, key, "def = "), Some(def.clone()), "kotlin {key}");
        }
        for s in SWITCHES {
            assert_eq!(field(QML, s, "def: "), Some("true".into()), "qml {s}");
            assert!(KOTLIN.contains(&format!("DjSwitch(\"{s}\"")), "kotlin {s}");
        }
    }

    #[test]
    fn field_reads_the_entry_that_holds_the_key() {
        assert_eq!(field("{ key: \"a\", def: 5, x: 1 }\n{ key: \"b\", def: 7 }", "b", "def: "), Some("7".into()));
        assert_eq!(field("{ key: \"a\", def: 5 }", "zz", "def: "), None);
    }
}
