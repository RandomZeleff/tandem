//! Terms the model must translate consistently: the official names of Minecraft's own
//! blocks, items, creatures and effects (read from the game's language files), plus
//! the player's own terms. Only the terms a batch actually uses are sent.

use std::io::Read;

use super::lang::{self, Format};
use crate::context::Context;
use crate::download::fetch_bytes;
use crate::error::Result;
use crate::meta;

/// Vanilla keys whose values are names worth keeping consistent.
const NAME_PREFIXES: &[&str] = &[
    "block.minecraft.",
    "item.minecraft.",
    "entity.minecraft.",
    "effect.minecraft.",
    "enchantment.minecraft.",
    "biome.minecraft.",
    "container.",
    "attribute.name.",
    // 1.12 and earlier.
    "tile.",
    "item.",
    "entity.",
    "potion.",
    "enchantment.",
];

/// Most terms sent with one batch.
const MAX_HITS: usize = 40;

#[derive(Debug, Default, Clone)]
pub struct Glossary {
    /// Lowercased English, English, translation; longest first so "Iron Ingot" beats "Iron".
    terms: Vec<(String, String, String)>,
}

impl Glossary {
    /// Later pairs override earlier ones with the same English term.
    pub fn new(pairs: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut map: std::collections::HashMap<String, (String, String)> =
            std::collections::HashMap::new();
        for (english, translation) in pairs {
            let english = english.trim().to_owned();
            if english.is_empty() || translation.trim().is_empty() {
                continue;
            }
            map.insert(
                english.to_lowercase(),
                (english, translation.trim().to_owned()),
            );
        }
        let mut terms: Vec<(String, String, String)> = map
            .into_iter()
            .map(|(lower, (en, tr))| (lower, en, tr))
            .collect();
        terms.sort_by(|a, b| {
            b.0.chars()
                .count()
                .cmp(&a.0.chars().count())
                .then_with(|| a.0.cmp(&b.0))
        });
        Self { terms }
    }

    pub fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Terms that appear as whole words in `texts`.
    pub fn hits<'a>(&'a self, texts: &[&str]) -> Vec<(&'a str, &'a str)> {
        let haystack = texts.join("\n").to_lowercase();
        let mut found = Vec::new();
        for (lower, english, translation) in &self.terms {
            if contains_word(&haystack, lower) {
                found.push((english.as_str(), translation.as_str()));
                if found.len() == MAX_HITS {
                    break;
                }
            }
        }
        found
    }
}

fn contains_word(haystack: &str, word: &str) -> bool {
    let mut from = 0;
    while let Some(i) = haystack[from..].find(word).map(|i| from + i) {
        let before = haystack[..i].chars().next_back();
        let after = haystack[i + word.len()..].chars().next();
        let boundary = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
        if boundary(before) && boundary(after) {
            return true;
        }
        from = i + word.len().max(1);
        while !haystack.is_char_boundary(from) {
            from += 1;
        }
    }
    false
}

/// English → `locale` names from the game's own language files, cached on disk. Empty
/// when the game files are not there yet (instance never installed).
pub async fn vanilla(ctx: &Context, game_version: &str, locale: &str) -> Vec<(String, String)> {
    let cache = ctx
        .data
        .cache()
        .join("glossary")
        .join(format!("{game_version}-{locale}.json"));
    if let Ok(bytes) = tokio::fs::read(&cache).await {
        if let Ok(pairs) = serde_json::from_slice(&bytes) {
            return pairs;
        }
    }
    match build_vanilla(ctx, game_version, locale).await {
        Ok(pairs) => {
            if !pairs.is_empty() {
                if let Ok(bytes) = serde_json::to_vec(&pairs) {
                    let _ = meta::write_file(&cache, &bytes).await;
                }
            }
            pairs
        }
        Err(err) => {
            tracing::warn!(%err, game_version, locale, "no vanilla glossary");
            Vec::new()
        }
    }
}

async fn build_vanilla(
    ctx: &Context,
    game_version: &str,
    locale: &str,
) -> Result<Vec<(String, String)>> {
    let format = lang::format_for(game_version);
    let jar = ctx
        .data
        .version_dir(game_version)
        .join(format!("{game_version}.jar"));
    let english = tokio::task::spawn_blocking(move || -> Option<String> {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(jar).ok()?).ok()?;
        let name = archive
            .file_names()
            .find(|n| {
                let lower = n.to_ascii_lowercase();
                lower == "assets/minecraft/lang/en_us.json"
                    || lower == "assets/minecraft/lang/en_us.lang"
            })?
            .to_owned();
        let mut text = String::new();
        archive
            .by_name(&name)
            .ok()?
            .read_to_string(&mut text)
            .ok()?;
        Some(text)
    })
    .await
    .ok()
    .flatten();
    let Some(english) = english else {
        return Ok(Vec::new());
    };

    let version = meta::load_version(ctx, game_version).await?;
    let index = meta::load_asset_index(ctx, &version).await?;
    let Some(object) = index.objects.iter().find_map(|(name, object)| {
        let lower = name.to_ascii_lowercase();
        let stem = lower.strip_prefix("minecraft/lang/")?;
        (stem.split('.').next() == Some(locale)).then_some(object)
    }) else {
        return Ok(Vec::new());
    };
    let path = ctx
        .data
        .assets()
        .join("objects")
        .join(object.relative_path());
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(_) => {
            let url = format!(
                "https://resources.download.minecraft.net/{}",
                object.relative_path()
            );
            let bytes = fetch_bytes(&ctx.http, &url, Some(&object.hash)).await?;
            meta::write_file(&path, &bytes).await?;
            bytes
        }
    };
    let translated: std::collections::HashMap<String, String> =
        lang::parse(&String::from_utf8_lossy(&bytes), format)
            .into_iter()
            .collect();
    Ok(pairs_from(&lang::parse(&english, format), &translated))
}

fn pairs_from(
    english: &[(String, String)],
    translated: &std::collections::HashMap<String, String>,
) -> Vec<(String, String)> {
    english
        .iter()
        .filter(|(key, _)| {
            NAME_PREFIXES.iter().any(|p| key.starts_with(p)) && !key.ends_with(".desc")
        })
        .filter_map(|(key, en)| {
            let tr = translated.get(key)?;
            let useful = tr != en
                && en.chars().any(char::is_alphabetic)
                && !en.contains('%')
                && en.len() <= 40;
            useful.then(|| (en.clone(), tr.clone()))
        })
        .collect()
}

/// For tests and tools: a glossary straight from language file contents.
pub fn from_files(english: &str, translated: &str, format: Format) -> Glossary {
    let translated: std::collections::HashMap<String, String> =
        lang::parse(translated, format).into_iter().collect();
    Glossary::new(pairs_from(&lang::parse(english, format), &translated))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_whole_words_longest_first() {
        let glossary = from_files(
            r#"{"block.minecraft.crafting_table": "Crafting Table", "item.minecraft.iron_ingot": "Iron Ingot",
                "block.minecraft.iron_block": "Block of Iron", "item.minecraft.stick": "Stick",
                "gui.done": "Done", "item.minecraft.potion.effect.x": "Potion of %s"}"#,
            r#"{"block.minecraft.crafting_table": "Établi", "item.minecraft.iron_ingot": "Lingot de fer",
                "block.minecraft.iron_block": "Bloc de fer", "item.minecraft.stick": "Bâton",
                "gui.done": "Terminé", "item.minecraft.potion.effect.x": "Potion de %s"}"#,
            Format::Json,
        );
        assert_eq!(glossary.len(), 4);
        let hits = glossary.hits(&["Place an iron ingot on the Crafting table", "Sticky piston"]);
        assert_eq!(
            hits,
            [
                ("Crafting Table", "Établi"),
                ("Iron Ingot", "Lingot de fer")
            ]
        );
    }

    #[test]
    fn player_terms_override_vanilla() {
        let glossary = Glossary::new([
            ("Mana".to_owned(), "Mana".to_owned()),
            ("mana".to_owned(), "Magie".to_owned()),
        ]);
        assert_eq!(glossary.hits(&["Mana pool"]), [("mana", "Magie")]);
    }

    #[test]
    fn matches_words_after_multibyte_text() {
        assert!(contains_word("éé stick", "stick"));
        assert!(!contains_word("sticky", "stick"));
    }
}
