//! Writing translations into an instance without losing anything: language files go
//! into a generated resource pack, new files (quest language files, loose books) are
//! created next to the originals, and quest files translated in place keep their
//! original in `.tandem/originals/`. A manifest lists what Tandem wrote, so turning the
//! translation off restores the instance exactly.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::lang;
use super::sources::{resource_packs, PACK_PREFIX, STATE_DIR};
use crate::error::Result;

const MANIFEST: &str = "translation.json";

/// Icon of the generated pack in the game's pack list.
const PACK_ICON: &[u8] = include_bytes!("../../../../src-tauri/icons/128x128.png");

/// What Tandem wrote into an instance.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    /// Locale of the active translation, `None` when it is off.
    pub locale: Option<String>,
    /// Files written outside the pack, relative to the instance, with their SHA-1.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// Of those, the ones that replaced an original (kept in `.tandem/originals`).
    #[serde(default)]
    pub replaced: HashSet<String>,
    /// Game language before Tandem switched it, restored when the translation is off.
    #[serde(default)]
    pub previous_language: Option<String>,
}

impl Manifest {
    pub fn load(game_dir: &Path) -> Self {
        std::fs::read(game_dir.join(STATE_DIR).join(MANIFEST))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save(&self, game_dir: &Path) -> Result<()> {
        let dir = game_dir.join(STATE_DIR);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(MANIFEST), serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    /// Whether `bytes` are exactly what Tandem last wrote at `rel`.
    pub fn wrote(&self, rel: &str, bytes: &[u8]) -> bool {
        self.files.get(rel).is_some_and(|sha| *sha == sha1(bytes))
    }
}

pub fn original_path(game_dir: &Path, rel: &str) -> PathBuf {
    game_dir.join(STATE_DIR).join("originals").join(rel)
}

fn sha1(bytes: &[u8]) -> String {
    sha1_smol::Sha1::from(bytes).digest().to_string()
}

/// Everything a translation produces.
#[derive(Debug, Default)]
pub struct Outputs {
    /// Files of the resource pack, by path inside it.
    pub pack: BTreeMap<String, String>,
    /// Files of the instance, by path relative to it.
    pub files: BTreeMap<String, String>,
    /// Of `files`, those that replace an existing file (in-place quest translation).
    pub replacing: HashSet<String>,
}

pub fn pack_name(locale: &str) -> String {
    format!("{PACK_PREFIX}{locale}")
}

/// Writes `outputs` into the instance, enables the pack and, if asked, switches the
/// game to `locale`. Files from a previous translation that are not part of this one
/// are removed or restored.
pub fn apply(
    game_dir: &Path,
    game_version: &str,
    locale: &str,
    outputs: &Outputs,
    set_language: bool,
) -> Result<()> {
    let mut manifest = Manifest::load(game_dir);
    remove_pack_folders(game_dir)?;
    if !outputs.pack.is_empty() {
        write_pack(game_dir, game_version, locale, &outputs.pack)?;
    }

    // Files no longer produced go back to how they were.
    let stale: Vec<String> = manifest
        .files
        .keys()
        .filter(|f| !outputs.files.contains_key(*f))
        .cloned()
        .collect();
    for rel in stale {
        undo_file(game_dir, &mut manifest, &rel)?;
    }

    for (rel, content) in &outputs.files {
        let path = game_dir.join(rel);
        let current = std::fs::read(&path).ok();
        let ours = current.as_deref().is_some_and(|c| manifest.wrote(rel, c));
        if outputs.replacing.contains(rel) {
            // Keep the original unless the file on disk is already Tandem's.
            if let (Some(current), false) = (&current, ours) {
                let backup = original_path(game_dir, rel);
                if let Some(parent) = backup.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(backup, current)?;
            }
            manifest.replaced.insert(rel.clone());
        } else if current.is_some() && !ours {
            // Somebody else's file (a human translation): never overwritten.
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
        manifest.files.insert(rel.clone(), sha1(content.as_bytes()));
    }

    let options_path = game_dir.join("options.txt");
    let options = std::fs::read_to_string(&options_path).unwrap_or_default();
    let current_language = language_of(&options);
    if set_language
        && current_language.as_deref() != Some(game_locale(locale, game_version).as_str())
    {
        manifest.previous_language = Some(current_language.unwrap_or_else(|| "en_us".to_owned()));
    }
    let updated = set_pack(
        &options,
        game_version,
        locale,
        !outputs.pack.is_empty(),
        set_language,
    );
    if updated != options {
        std::fs::write(&options_path, updated)?;
    }
    manifest.locale = Some(locale.to_owned());
    manifest.save(game_dir)
}

/// Turns the translation off: pack removed and disabled, files restored or deleted.
pub fn remove(game_dir: &Path, game_version: &str) -> Result<()> {
    let mut manifest = Manifest::load(game_dir);
    remove_pack_folders(game_dir)?;
    let files: Vec<String> = manifest.files.keys().cloned().collect();
    for rel in files {
        undo_file(game_dir, &mut manifest, &rel)?;
    }
    let options_path = game_dir.join("options.txt");
    if let Ok(options) = std::fs::read_to_string(&options_path) {
        let mut updated = set_pack(
            &options,
            game_version,
            manifest.locale.as_deref().unwrap_or("en_us"),
            false,
            false,
        );
        // Back to the language the player had, unless they changed it since.
        if let (Some(previous), Some(locale)) = (&manifest.previous_language, &manifest.locale) {
            if language_of(&updated).as_deref() == Some(game_locale(locale, game_version).as_str())
            {
                updated = updated
                    .lines()
                    .map(|l| {
                        if l.starts_with("lang:") {
                            format!("lang:{previous}")
                        } else {
                            l.to_owned()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n";
            }
        }
        if updated != options {
            std::fs::write(&options_path, updated)?;
        }
    }
    manifest.locale = None;
    manifest.previous_language = None;
    manifest.save(game_dir)
}

fn remove_pack_folders(game_dir: &Path) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(game_dir.join("resourcepacks")) else {
        return Ok(());
    };
    for entry in entries.filter_map(|e| e.ok()) {
        if entry.file_name().to_string_lossy().starts_with(PACK_PREFIX) {
            let path = entry.path();
            if path.is_dir() {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

/// Restores the original of a file Tandem replaced, or deletes a file it created, if
/// the file is still the one Tandem wrote.
fn undo_file(game_dir: &Path, manifest: &mut Manifest, rel: &str) -> Result<()> {
    let path = game_dir.join(rel);
    let current = std::fs::read(&path).ok();
    let ours = current.as_deref().is_some_and(|c| manifest.wrote(rel, c));
    if ours {
        let backup = original_path(game_dir, rel);
        if manifest.replaced.contains(rel) && backup.is_file() {
            std::fs::copy(&backup, &path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    let backup = original_path(game_dir, rel);
    if backup.is_file() {
        std::fs::remove_file(backup)?;
    }
    manifest.files.remove(rel);
    manifest.replaced.remove(rel);
    Ok(())
}

fn write_pack(
    game_dir: &Path,
    game_version: &str,
    locale: &str,
    files: &BTreeMap<String, String>,
) -> Result<()> {
    let root = game_dir.join("resourcepacks").join(pack_name(locale));
    std::fs::create_dir_all(&root)?;
    std::fs::write(root.join("pack.mcmeta"), pack_mcmeta(game_version, locale))?;
    std::fs::write(root.join("pack.png"), PACK_ICON)?;
    for (rel, content) in files {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
    }
    Ok(())
}

/// Resource pack format of a game version (see the Minecraft wiki's table).
pub fn pack_format(game_version: &str) -> u32 {
    let mut parts = game_version.split(['.', '-', ' ']);
    let (major, minor, patch) = (
        parts.next().and_then(|p| p.parse::<u32>().ok()),
        parts.next().and_then(|p| p.parse::<u32>().ok()),
        parts
            .next()
            .and_then(|p| p.parse::<u32>().ok())
            .unwrap_or(0),
    );
    match (major, minor) {
        (Some(1), Some(minor)) => match (minor, patch) {
            (0..=8, _) => 1,
            (9..=10, _) => 2,
            (11..=12, _) => 3,
            (13..=14, _) => 4,
            (15, _) | (16, 0..=1) => 5,
            (16, _) => 6,
            (17, _) => 7,
            (18, _) => 8,
            (19, 0..=2) => 9,
            (19, 3) => 12,
            (19, _) => 13,
            (20, 0..=1) => 15,
            (20, 2) => 18,
            (20, 3..=4) => 22,
            (20, _) => 32,
            (21, 0..=1) => 34,
            (21, 2..=3) => 42,
            (21, 4) => 46,
            (21, 5) => 55,
            (21, 6) => 63,
            (21, 7..=8) => 64,
            _ => 69,
        },
        // Snapshots and year-numbered versions: recent formats.
        _ => 69,
    }
}

/// `pack.mcmeta` accepted by the instance's version and by later ones.
fn pack_mcmeta(game_version: &str, locale: &str) -> String {
    let format = pack_format(game_version);
    let name = lang::LANGUAGES
        .iter()
        .find(|(c, _)| *c == locale)
        .map_or(locale, |(_, n)| n);
    let mut pack = serde_json::json!({
        "pack_format": format,
        "description": format!("Traduction {name} générée par Tandem"),
    });
    if format >= 18 {
        pack["supported_formats"] =
            serde_json::json!({ "min_inclusive": format, "max_inclusive": 9999 });
    }
    if format >= 65 {
        pack["min_format"] = serde_json::json!(format);
        pack["max_format"] = serde_json::json!(9999);
    }
    serde_json::to_string_pretty(&serde_json::json!({ "pack": pack })).unwrap_or_default()
}

/// `options.txt` with the translation pack enabled (or removed), right below the packs
/// the player added so their own translations win, and optionally the game language.
pub fn set_pack(
    options: &str,
    game_version: &str,
    locale: &str,
    enabled: bool,
    set_language: bool,
) -> String {
    let legacy = lang::minor_version(game_version).is_some_and(|m| m <= 12);
    let name = pack_name(locale);
    let entry = if legacy {
        name.clone()
    } else {
        format!("file/{name}")
    };
    let mut packs: Vec<String> = resource_packs(options)
        .into_iter()
        .filter(|p| !p.trim_start_matches("file/").starts_with(PACK_PREFIX))
        .collect();
    if packs.is_empty() && !legacy {
        packs.push("vanilla".to_owned());
    }
    if enabled {
        let position = packs
            .iter()
            .position(|p| p.starts_with("file/") || (legacy && p.ends_with(".zip")))
            .unwrap_or(packs.len());
        packs.insert(position, entry.clone());
    }
    let packs_line = format!(
        "resourcePacks:{}",
        serde_json::to_string(&packs).unwrap_or_else(|_| "[]".into())
    );

    // Listing the pack as accepted keeps the game from refusing it over its format.
    let mut accepted: Vec<String> = options
        .lines()
        .find_map(|l| l.strip_prefix("incompatibleResourcePacks:"))
        .and_then(|l| serde_json::from_str(l.trim()).ok())
        .unwrap_or_default();
    accepted.retain(|p: &String| !p.trim_start_matches("file/").starts_with(PACK_PREFIX));
    if enabled && !legacy {
        accepted.push(entry);
    }
    let accepted_line = format!(
        "incompatibleResourcePacks:{}",
        serde_json::to_string(&accepted).unwrap_or_else(|_| "[]".into())
    );
    let lang_line = format!("lang:{}", game_locale(locale, game_version));

    let mut out = Vec::new();
    let (mut has_packs, mut has_accepted, mut has_lang) = (false, false, false);
    for line in options.lines() {
        if line.starts_with("resourcePacks:") {
            has_packs = true;
            out.push(packs_line.clone());
        } else if line.starts_with("incompatibleResourcePacks:") {
            has_accepted = true;
            out.push(accepted_line.clone());
        } else if line.starts_with("lang:") {
            has_lang = true;
            out.push(if set_language {
                lang_line.clone()
            } else {
                line.to_owned()
            });
        } else {
            out.push(line.to_owned());
        }
    }
    if !has_packs && (enabled || !options.is_empty()) {
        out.push(packs_line);
    }
    if !has_accepted && enabled && !legacy {
        out.push(accepted_line);
    }
    if !has_lang && set_language {
        out.push(lang_line);
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

fn language_of(options: &str) -> Option<String> {
    options
        .lines()
        .find_map(|l| l.strip_prefix("lang:").map(|v| v.trim().to_owned()))
}

/// Locale as the game writes it in `options.txt`: `fr_FR` up to 1.10.
fn game_locale(locale: &str, game_version: &str) -> String {
    match (lang::minor_version(game_version), locale.split_once('_')) {
        (Some(minor), Some((language, region))) if minor <= 10 => {
            format!("{language}_{}", region.to_uppercase())
        }
        _ => locale.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_pack_formats() {
        assert_eq!(pack_format("1.12.2"), 3);
        assert_eq!(pack_format("1.16.5"), 6);
        assert_eq!(pack_format("1.20.1"), 15);
        assert_eq!(pack_format("1.21.1"), 34);
        assert_eq!(pack_format("26.3"), 69);
        let meta: serde_json::Value =
            serde_json::from_str(&pack_mcmeta("1.21.1", "fr_fr")).unwrap();
        assert_eq!(meta["pack"]["supported_formats"]["min_inclusive"], 34);
        assert!(meta["pack"]["description"]
            .as_str()
            .unwrap()
            .contains("Français"));
    }

    #[test]
    fn enables_the_pack_below_player_packs() {
        let options =
            "version:3465\nresourcePacks:[\"vanilla\",\"fabric\",\"file/Fresh.zip\"]\nlang:en_us\n";
        let on = set_pack(options, "1.20.1", "fr_fr", true, true);
        assert!(on.contains(
            r#"resourcePacks:["vanilla","fabric","file/tandem-translation-fr_fr","file/Fresh.zip"]"#
        ));
        assert!(on.contains(r#"incompatibleResourcePacks:["file/tandem-translation-fr_fr"]"#));
        assert!(on.contains("lang:fr_fr"));
        assert!(on.starts_with("version:3465\n"));
        // Applying twice changes nothing; turning off removes the pack but keeps the rest.
        assert_eq!(set_pack(&on, "1.20.1", "fr_fr", true, true), on);
        let off = set_pack(&on, "1.20.1", "fr_fr", false, false);
        assert!(off.contains(r#"resourcePacks:["vanilla","fabric","file/Fresh.zip"]"#));
        assert!(off.contains("incompatibleResourcePacks:[]"));
        assert!(off.contains("lang:fr_fr"));
    }

    #[test]
    fn creates_options_for_new_instances() {
        let on = set_pack("", "1.21.1", "de_de", true, true);
        assert!(on.contains(r#"resourcePacks:["vanilla","file/tandem-translation-de_de"]"#));
        assert!(on.contains("lang:de_de"));
        let legacy = set_pack("", "1.7.10", "fr_fr", true, true);
        assert!(legacy.contains(r#"resourcePacks:["tandem-translation-fr_fr"]"#));
        assert!(legacy.contains("lang:fr_FR"));
        assert!(!legacy.contains("incompatible"));
    }

    #[test]
    fn applies_and_restores_files() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path();
        std::fs::create_dir_all(game.join("config/q")).unwrap();
        std::fs::write(game.join("config/q/a.snbt"), "{ title: \"Start\" }").unwrap();
        std::fs::write(game.join("config/q/human.snbt"), "human").unwrap();
        std::fs::write(
            game.join("options.txt"),
            "resourcePacks:[\"vanilla\"]\nlang:en_us\n",
        )
        .unwrap();

        let mut outputs = Outputs::default();
        outputs
            .pack
            .insert("assets/x/lang/fr_fr.json".into(), "{}".into());
        outputs
            .files
            .insert("config/q/a.snbt".into(), "{ title: \"Début\" }".into());
        outputs.replacing.insert("config/q/a.snbt".into());
        outputs
            .files
            .insert("config/q/human.snbt".into(), "ai".into());
        outputs
            .files
            .insert("config/q/new.snbt".into(), "new".into());
        apply(game, "1.20.1", "fr_fr", &outputs, true).unwrap();

        assert_eq!(
            std::fs::read_to_string(game.join("config/q/a.snbt")).unwrap(),
            "{ title: \"Début\" }"
        );
        assert_eq!(
            std::fs::read_to_string(game.join("config/q/human.snbt")).unwrap(),
            "human"
        );
        assert!(game
            .join("resourcepacks/tandem-translation-fr_fr/pack.mcmeta")
            .is_file());
        assert!(game
            .join("resourcepacks/tandem-translation-fr_fr/assets/x/lang/fr_fr.json")
            .is_file());
        // The original is what a new scan reads.
        assert_eq!(
            super::super::sources::original_text(game, "config/q/a.snbt").unwrap(),
            "{ title: \"Start\" }"
        );

        // Applying again keeps the first original, not Tandem's own file.
        apply(game, "1.20.1", "fr_fr", &outputs, true).unwrap();
        assert_eq!(
            std::fs::read_to_string(original_path(game, "config/q/a.snbt")).unwrap(),
            "{ title: \"Start\" }"
        );

        remove(game, "1.20.1").unwrap();
        assert_eq!(
            std::fs::read_to_string(game.join("config/q/a.snbt")).unwrap(),
            "{ title: \"Start\" }"
        );
        assert!(!game.join("config/q/new.snbt").exists());
        assert_eq!(
            std::fs::read_to_string(game.join("config/q/human.snbt")).unwrap(),
            "human"
        );
        assert!(!game.join("resourcepacks/tandem-translation-fr_fr").exists());
        let options = std::fs::read_to_string(game.join("options.txt")).unwrap();
        assert!(options.contains(r#"resourcePacks:["vanilla"]"#));
        assert!(options.contains("lang:en_us"), "{options}");
        assert!(Manifest::load(game).files.is_empty());
    }
}
