//! Minecraft language files: JSON (1.13+) and the older `key=value` `.lang` format,
//! read leniently (comments, trailing commas, BOM) since mods ship all sorts of them.

use serde::Serialize;

/// Languages Tandem can translate into, as Minecraft locale codes with their names.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("fr_fr", "Français (France)"),
    ("fr_ca", "Français (Canada)"),
    ("es_es", "Español (España)"),
    ("es_mx", "Español (México)"),
    ("de_de", "Deutsch"),
    ("it_it", "Italiano"),
    ("pt_br", "Português (Brasil)"),
    ("pt_pt", "Português (Portugal)"),
    ("nl_nl", "Nederlands"),
    ("pl_pl", "Polski"),
    ("ru_ru", "Русский"),
    ("uk_ua", "Українська"),
    ("tr_tr", "Türkçe"),
    ("ja_jp", "日本語"),
    ("ko_kr", "한국어"),
    ("zh_cn", "简体中文"),
    ("zh_tw", "繁體中文"),
];

/// English name of a locale, for the model's instructions.
pub fn english_name(locale: &str) -> &'static str {
    match locale {
        "fr_fr" => "French (France)",
        "fr_ca" => "Canadian French",
        "es_es" => "Spanish (Spain)",
        "es_mx" => "Mexican Spanish",
        "de_de" => "German",
        "it_it" => "Italian",
        "pt_br" => "Brazilian Portuguese",
        "pt_pt" => "European Portuguese",
        "nl_nl" => "Dutch",
        "pl_pl" => "Polish",
        "ru_ru" => "Russian",
        "uk_ua" => "Ukrainian",
        "tr_tr" => "Turkish",
        "ja_jp" => "Japanese",
        "ko_kr" => "Korean",
        "zh_cn" => "Simplified Chinese",
        "zh_tw" => "Traditional Chinese",
        _ => "the target language",
    }
}

pub fn is_supported(locale: &str) -> bool {
    LANGUAGES.iter().any(|(code, _)| *code == locale)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    /// `assets/<ns>/lang/<locale>.json`, 1.13 and later.
    Json,
    /// `assets/<ns>/lang/<locale>.lang`, 1.12 and earlier.
    Legacy,
}

/// File name of a locale's language file for a game version: `fr_fr.json`, `fr_fr.lang`
/// (1.11–1.12) or `fr_FR.lang` (1.10 and earlier, region in capitals).
pub fn file_name(locale: &str, format: Format, game_version: &str) -> String {
    match format {
        Format::Json => format!("{locale}.json"),
        Format::Legacy if minor_version(game_version).is_some_and(|m| m <= 10) => {
            match locale.split_once('_') {
                Some((lang, region)) => format!("{lang}_{}.lang", region.to_uppercase()),
                None => format!("{locale}.lang"),
            }
        }
        Format::Legacy => format!("{locale}.lang"),
    }
}

/// `12` for `1.12.2`; `None` for snapshots and versions numbered by year (`26.3`).
pub fn minor_version(game_version: &str) -> Option<u32> {
    let mut parts = game_version.split('.');
    match (parts.next(), parts.next()) {
        (Some("1"), Some(minor)) => minor.parse().ok(),
        _ => None,
    }
}

/// Language files are JSON from 1.13 on.
pub fn format_for(game_version: &str) -> Format {
    match minor_version(game_version) {
        Some(minor) if minor <= 12 => Format::Legacy,
        _ => Format::Json,
    }
}

/// Entries of a language file in file order. Values that are not strings are skipped.
pub fn parse(text: &str, format: Format) -> Vec<(String, String)> {
    let text = text.trim_start_matches('\u{feff}');
    match format {
        Format::Json => parse_json(text),
        Format::Legacy => parse_legacy(text),
    }
}

fn parse_json(text: &str) -> Vec<(String, String)> {
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(_) => match serde_json::from_str(&relax_json(text)) {
            Ok(value) => value,
            Err(_) => return Vec::new(),
        },
    };
    match value {
        serde_json::Value::Object(map) => map
            .into_iter()
            .filter_map(|(k, v)| match v {
                serde_json::Value::String(s) => Some((k, s)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Removes `//` and `/* */` comments and trailing commas outside strings.
fn relax_json(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '/' if chars.peek() == Some(&'/') => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut previous = ' ';
                for next in chars.by_ref() {
                    if previous == '*' && next == '/' {
                        break;
                    }
                    previous = next;
                }
            }
            ',' => {
                // A comma followed only by whitespace and a closing bracket is dropped.
                let rest: String = chars.clone().take_while(|c| c.is_whitespace()).collect();
                let after = chars.clone().nth(rest.chars().count());
                if !matches!(after, Some('}') | Some(']')) {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

fn parse_legacy(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            (!key.is_empty()).then(|| (key.to_owned(), value.replace("\\n", "\n")))
        })
        .collect()
}

/// Serializes entries as a language file.
pub fn write(entries: &[(String, String)], format: Format) -> String {
    match format {
        Format::Json => {
            let map: serde_json::Map<String, serde_json::Value> = entries
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            serde_json::to_string_pretty(&map).unwrap_or_else(|_| "{}".to_owned())
        }
        Format::Legacy => {
            let mut out = String::new();
            for (key, value) in entries {
                out.push_str(key);
                out.push('=');
                out.push_str(&value.replace('\n', "\\n"));
                out.push('\n');
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lenient_json() {
        let text = "\u{feff}{\n  // comment\n  \"a\": \"x // not a comment\",\n  /* block */ \"b\": \"y\",\n  \"n\": 3,\n}";
        assert_eq!(
            parse(text, Format::Json),
            [
                ("a".to_owned(), "x // not a comment".to_owned()),
                ("b".to_owned(), "y".to_owned())
            ]
        );
        assert!(parse("not json", Format::Json).is_empty());
    }

    #[test]
    fn parses_and_writes_legacy() {
        let entries = parse(
            "# header\ntile.a.name=Stone Thing\nbad line\nb=Line\\nTwo\n",
            Format::Legacy,
        );
        assert_eq!(
            entries,
            [
                ("tile.a.name".to_owned(), "Stone Thing".to_owned()),
                ("b".to_owned(), "Line\nTwo".to_owned())
            ]
        );
        assert_eq!(
            write(&entries, Format::Legacy),
            "tile.a.name=Stone Thing\nb=Line\\nTwo\n"
        );
    }

    #[test]
    fn names_files_by_version() {
        assert_eq!(file_name("fr_fr", Format::Json, "1.20.1"), "fr_fr.json");
        assert_eq!(
            file_name("fr_fr", format_for("1.12.2"), "1.12.2"),
            "fr_fr.lang"
        );
        assert_eq!(
            file_name("fr_fr", format_for("1.7.10"), "1.7.10"),
            "fr_FR.lang"
        );
        assert_eq!(format_for("26.3"), Format::Json);
        assert_eq!(format_for("1.12.2"), Format::Legacy);
    }
}
