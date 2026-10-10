//! What an instance has to translate: language files in mod jars, enabled resource
//! packs and KubeJS assets, FTB Quests texts and Patchouli books. Also what is already
//! translated, so only missing texts go to the model.

use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::lang::{self, Format};
use super::snbt;
use crate::content::deps;
use crate::paths::DataDir;

/// Resource packs Tandem generates: never read back as a source.
pub const PACK_PREFIX: &str = "tandem-translation-";

/// Folder of the instance where Tandem keeps translation state (originals, manifest).
pub const STATE_DIR: &str = ".tandem";

/// Quest fields players read.
const QUEST_KEYS: &[&str] = &["title", "subtitle", "description"];

/// Patchouli fields players read.
const BOOK_KEYS: &[&str] = &[
    "name",
    "title",
    "text",
    "description",
    "subtitle",
    "landing_text",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    Mod,
    ResourcePack,
    KubeJs,
    Quests,
    Book,
}

/// One mod, pack or quest book, with the texts it brings.
#[derive(Debug, Clone)]
pub struct Source {
    /// Stable across scans: `mod:<file>`, `pack:<name>`, `kubejs`, `quests`, `book:<path>`.
    pub id: String,
    pub name: String,
    pub kind: SourceKind,
    pub units: Vec<Unit>,
}

/// Texts that end up in one output file.
#[derive(Debug, Clone)]
pub struct Unit {
    pub target: Target,
    /// Key (meaning depends on the target) and English text.
    pub entries: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// `assets/<namespace>/lang/<locale>` in the generated pack. Keys are language keys.
    Lang { namespace: String },
    /// `config/ftbquests/quests/lang/<locale>.snbt`, or one file of the
    /// `lang/<locale>/` folder (`file` relative to it). Keys are `key` or `key#index`.
    QuestLang { file: Option<String> },
    /// A quest file with texts inside (older FTB Quests), translated in place. `file` is
    /// relative to the instance; keys are the position of the string in the file.
    QuestInline { file: String, original: String },
    /// A Patchouli book page: a copy of `original` with texts replaced, written to
    /// `dest` with the locale in place of `en_us`. Keys are JSON pointers.
    Book { dest: BookDest, original: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookDest {
    /// Path inside the generated pack, with `{locale}` for the language folder.
    Pack(String),
    /// Path relative to the instance (loose books), with `{locale}`.
    Loose(String),
}

/// The instance's translatable texts and its existing translations into `locale`.
#[derive(Debug, Default)]
pub struct Scan {
    pub sources: Vec<Source>,
    /// Language key → translation already shipped by a mod or pack (last one wins).
    pub existing: HashMap<String, String>,
    /// FTB Quests language keys already translated.
    pub existing_quests: HashMap<String, String>,
    pub format: Option<Format>,
}

/// Reads everything translatable in an instance. Blocking: call from a worker thread.
pub fn scan(
    data: &DataDir,
    instance_id: &str,
    game_dir: &Path,
    game_version: &str,
    locale: &str,
) -> Scan {
    let format = lang::format_for(game_version);
    let mut scan = Scan {
        format: Some(format),
        ..Default::default()
    };
    let names: HashMap<String, String> = deps::scan_cached(data, instance_id, game_dir)
        .into_iter()
        .map(|m| (m.file_name, m.name))
        .collect();

    // Mods first, then KubeJS, then packs from lowest to highest priority: a later
    // source overrides the English text and the translations of an earlier one.
    let jars = enabled_jars(game_dir);
    for (file, found) in read_parallel(&jars, |path| read_archive(path, format, locale)) {
        let name = names
            .get(&file)
            .cloned()
            .unwrap_or_else(|| file.trim_end_matches(".jar").to_owned());
        push(
            &mut scan,
            format!("mod:{file}"),
            name,
            SourceKind::Mod,
            found,
        );
    }

    let kubejs = game_dir.join("kubejs").join("assets");
    if kubejs.is_dir() {
        let found = read_folder(&kubejs, format, locale, "");
        push(
            &mut scan,
            "kubejs".into(),
            "KubeJS".into(),
            SourceKind::KubeJs,
            found,
        );
    }

    for pack in enabled_packs(game_dir) {
        let found = if pack.is_dir() {
            read_folder(&pack.join("assets"), format, locale, "")
        } else {
            std::fs::File::open(&pack)
                .ok()
                .and_then(|f| read_zip(f, format, locale))
                .unwrap_or_default()
        };
        let name = pack
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        push(
            &mut scan,
            format!("pack:{name}"),
            name.trim_end_matches(".zip").to_owned(),
            SourceKind::ResourcePack,
            found,
        );
    }

    quests(game_dir, locale, &mut scan);
    loose_books(game_dir, &mut scan);
    scan
}

/// What one jar, pack or folder holds.
#[derive(Debug, Default)]
struct Found {
    /// English language files by namespace.
    english: Vec<(String, Vec<(String, String)>)>,
    /// Existing translations into the locale.
    translated: Vec<(String, String)>,
    books: Vec<Unit>,
}

fn push(scan: &mut Scan, id: String, name: String, kind: SourceKind, found: Found) {
    scan.existing.extend(found.translated);
    let mut units: Vec<Unit> = found
        .english
        .into_iter()
        .filter(|(_, entries)| !entries.is_empty())
        .map(|(namespace, entries)| Unit {
            target: Target::Lang { namespace },
            entries,
        })
        .collect();
    units.extend(found.books);
    if !units.is_empty() {
        scan.sources.push(Source {
            id,
            name,
            kind,
            units,
        });
    }
}

fn enabled_jars(game_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(game_dir.join("mods")) else {
        return Vec::new();
    };
    let mut jars: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jar"))
        .collect();
    jars.sort();
    jars
}

/// Packs of `resourcepacks/` listed in `options.txt`, lowest priority first.
fn enabled_packs(game_dir: &Path) -> Vec<PathBuf> {
    let options = std::fs::read_to_string(game_dir.join("options.txt")).unwrap_or_default();
    let folder = game_dir.join("resourcepacks");
    let mut seen = std::collections::HashSet::new();
    resource_packs(&options)
        .into_iter()
        .filter_map(|entry| {
            let name = entry.strip_prefix("file/").unwrap_or(&entry).to_owned();
            if name.starts_with(PACK_PREFIX) || !seen.insert(name.clone()) {
                return None;
            }
            let path = folder.join(&name);
            path.exists().then_some(path)
        })
        .collect()
}

/// The `resourcePacks` list of an `options.txt`.
pub fn resource_packs(options: &str) -> Vec<String> {
    options
        .lines()
        .find_map(|l| l.strip_prefix("resourcePacks:"))
        .and_then(|list| serde_json::from_str(list.trim()).ok())
        .unwrap_or_default()
}

/// Runs `read` on every path on all cores, keeping file names and order.
fn read_parallel(paths: &[PathBuf], read: impl Fn(&Path) -> Found + Sync) -> Vec<(String, Found)> {
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(16);
    let chunk = paths.len().div_ceil(threads).max(1);
    let read = &read;
    std::thread::scope(|scope| {
        let handles: Vec<_> = paths
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|p| {
                            let name = p
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            (name, read(p))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    })
}

fn read_archive(path: &Path, format: Format, locale: &str) -> Found {
    std::fs::File::open(path)
        .ok()
        .and_then(|f| read_zip(f, format, locale))
        .unwrap_or_default()
}

/// Language files and Patchouli books of a jar or zipped pack.
fn read_zip<R: Read + Seek>(reader: R, format: Format, locale: &str) -> Option<Found> {
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(reader)).ok()?;
    let mut found = Found::default();
    let names: Vec<String> = archive.file_names().map(str::to_owned).collect();
    for name in &names {
        let Some(kind) = classify(name, format, locale) else {
            continue;
        };
        let Ok(mut entry) = archive.by_name(name) else {
            continue;
        };
        let mut text = String::new();
        if entry.read_to_string(&mut text).is_err() {
            continue;
        }
        collect(&mut found, kind, name, &text, format, locale, &names);
    }
    Some(found)
}

/// Same as [`read_zip`] for an unpacked `assets` folder. `prefix` is the path of
/// `dir` inside the assets folder.
fn read_folder(dir: &Path, format: Format, locale: &str, prefix: &str) -> Found {
    let mut files = Vec::new();
    walk(dir, prefix, &mut files);
    let names: Vec<String> = files.iter().map(|(n, _)| format!("assets/{n}")).collect();
    let mut found = Found::default();
    for (name, path) in &files {
        let name = format!("assets/{name}");
        let Some(kind) = classify(&name, format, locale) else {
            continue;
        };
        if let Ok(text) = std::fs::read_to_string(path) {
            collect(&mut found, kind, &name, &text, format, locale, &names);
        }
    }
    found
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let path = entry.path();
        if path.is_dir() {
            walk(&path, &rel, out);
        } else {
            out.push((rel, path));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    English,
    Translated,
    Book,
}

/// What an archive path is: `assets/<ns>/lang/en_us.json`, the locale's file, or a page
/// of an assets-based Patchouli book in English.
fn classify(name: &str, format: Format, locale: &str) -> Option<EntryKind> {
    let lower = name.to_ascii_lowercase();
    let rest = lower.strip_prefix("assets/")?;
    let (_, path) = rest.split_once('/')?;
    if let Some(file) = path.strip_prefix("lang/") {
        let ext = match format {
            Format::Json => ".json",
            Format::Legacy => ".lang",
        };
        let stem = file.strip_suffix(ext)?;
        return if stem == "en_us" {
            Some(EntryKind::English)
        } else if stem == locale {
            Some(EntryKind::Translated)
        } else {
            None
        };
    }
    (path.starts_with("patchouli_books/") && path.contains("/en_us/") && path.ends_with(".json"))
        .then_some(EntryKind::Book)
}

fn collect(
    found: &mut Found,
    kind: EntryKind,
    name: &str,
    text: &str,
    format: Format,
    locale: &str,
    all: &[String],
) {
    let namespace = name.split('/').nth(1).unwrap_or_default().to_owned();
    match kind {
        EntryKind::English => found.english.push((namespace, lang::parse(text, format))),
        EntryKind::Translated => found.translated.extend(lang::parse(text, format)),
        EntryKind::Book => {
            // A book already translated into the locale is left alone.
            let translated = name.replacen("/en_us/", "/{locale}/", 1);
            let in_locale = name.replacen("/en_us/", &format!("/{locale}/"), 1);
            let has_locale = all.iter().any(|n| n.eq_ignore_ascii_case(&in_locale));
            if has_locale {
                return;
            }
            if let Some(unit) = book_unit(text, BookDest::Pack(translated)) {
                found.books.push(unit);
            }
        }
    }
}

/// Texts of a Patchouli JSON file. Values that look like language keys are left to the
/// language files.
fn book_unit(text: &str, dest: BookDest) -> Option<Unit> {
    let value: serde_json::Value =
        serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let mut entries = Vec::new();
    book_texts(&value, String::new(), &mut entries);
    (!entries.is_empty()).then(|| Unit {
        target: Target::Book {
            dest,
            original: text.to_owned(),
        },
        entries,
    })
}

fn book_texts(value: &serde_json::Value, pointer: String, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let path = format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"));
                match child {
                    serde_json::Value::String(s)
                        if BOOK_KEYS.contains(&key.as_str()) && !looks_like_key(s) =>
                    {
                        out.push((path, s.clone()));
                    }
                    _ => book_texts(child, path, out),
                }
            }
        }
        serde_json::Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                book_texts(child, format!("{pointer}/{i}"), out);
            }
        }
        _ => {}
    }
}

/// `item.mod.thing`: a reference to a language key, not a text.
fn looks_like_key(text: &str) -> bool {
    !text.contains(' ') && text.contains('.') && !text.ends_with('.')
}

/// Loose Patchouli books in the instance's `patchouli_books/` folder.
fn loose_books(game_dir: &Path, scan: &mut Scan) {
    let root = game_dir.join("patchouli_books");
    let Ok(books) = std::fs::read_dir(&root) else {
        return;
    };
    for book in books.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()) {
        let book_name = book.file_name().to_string_lossy().into_owned();
        let english = book.path().join("en_us");
        if !english.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        walk(&english, "", &mut files);
        let units: Vec<Unit> = files
            .into_iter()
            .filter(|(rel, _)| rel.ends_with(".json"))
            .filter_map(|(rel, path)| {
                let text = std::fs::read_to_string(path).ok()?;
                book_unit(
                    &text,
                    BookDest::Loose(format!("patchouli_books/{book_name}/{{locale}}/{rel}")),
                )
            })
            .collect();
        if !units.is_empty() {
            scan.sources.push(Source {
                id: format!("book:{book_name}"),
                name: book_name,
                kind: SourceKind::Book,
                units,
            });
        }
    }
}

fn quest_root(game_dir: &Path) -> PathBuf {
    game_dir.join("config").join("ftbquests").join("quests")
}

/// FTB Quests texts: from the language files when the pack has them, else from the
/// quest files themselves (their untranslated originals, see [`original_text`]).
fn quests(game_dir: &Path, locale: &str, scan: &mut Scan) {
    let root = quest_root(game_dir);
    if !root.is_dir() {
        return;
    }
    let lang_dir = root.join("lang");
    let mut units = Vec::new();
    if lang_dir.join("en_us.snbt").is_file() || lang_dir.join("en_us").is_dir() {
        let mut files = Vec::new();
        if let Ok(text) = std::fs::read_to_string(lang_dir.join("en_us.snbt")) {
            files.push((None, text));
        }
        let mut nested = Vec::new();
        walk(&lang_dir.join("en_us"), "", &mut nested);
        for (rel, path) in nested.into_iter().filter(|(r, _)| r.ends_with(".snbt")) {
            if let Ok(text) = std::fs::read_to_string(path) {
                files.push((Some(rel), text));
            }
        }
        for (file, text) in files {
            let entries = quest_lang_entries(&text);
            if !entries.is_empty() {
                units.push(Unit {
                    target: Target::QuestLang { file },
                    entries,
                });
            }
        }
        let mut translated = Vec::new();
        if let Ok(text) = std::fs::read_to_string(lang_dir.join(format!("{locale}.snbt"))) {
            translated.extend(quest_lang_entries(&text));
        }
        let mut nested = Vec::new();
        walk(&lang_dir.join(locale), "", &mut nested);
        for (_, path) in nested {
            if let Ok(text) = std::fs::read_to_string(path) {
                translated.extend(quest_lang_entries(&text));
            }
        }
        scan.existing_quests.extend(translated);
    } else {
        let mut files = Vec::new();
        walk(&root, "", &mut files);
        files.sort();
        for (rel, _) in files.into_iter().filter(|(r, _)| r.ends_with(".snbt")) {
            let file = format!("config/ftbquests/quests/{rel}");
            let Some(original) = original_text(game_dir, &file) else {
                continue;
            };
            let entries: Vec<(String, String)> = snbt::strings(&original)
                .into_iter()
                .enumerate()
                .filter(|(_, s)| QUEST_KEYS.contains(&s.key.as_str()))
                .map(|(i, s)| (i.to_string(), s.value))
                .collect();
            if !entries.is_empty() {
                units.push(Unit {
                    target: Target::QuestInline { file, original },
                    entries,
                });
            }
        }
    }
    if !units.is_empty() {
        scan.sources.push(Source {
            id: "quests".into(),
            name: "Quêtes (FTB Quests)".into(),
            kind: SourceKind::Quests,
            units,
        });
    }
}

/// Entries of an FTB Quests language file: `key` for strings, `key#i` for list items.
pub fn quest_lang_entries(text: &str) -> Vec<(String, String)> {
    snbt::strings(text)
        .into_iter()
        .map(|s| match s.index {
            Some(i) => (format!("{}#{i}", s.key), s.value),
            None => (s.key, s.value),
        })
        .collect()
}

/// The text of a file before Tandem translated it in place: the saved original when
/// the file is still the one Tandem wrote, else the file itself.
pub fn original_text(game_dir: &Path, rel: &str) -> Option<String> {
    let current = std::fs::read(game_dir.join(rel)).ok()?;
    let manifest = super::output::Manifest::load(game_dir);
    if manifest.wrote(rel, &current) {
        if let Ok(original) = std::fs::read_to_string(super::output::original_path(game_dir, rel)) {
            return Some(original);
        }
    }
    String::from_utf8(current).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip(files: &[(&str, &str)]) -> std::io::Cursor<Vec<u8>> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            for (name, content) in files {
                writer
                    .start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(content.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
        }
        buffer.set_position(0);
        buffer
    }

    #[test]
    fn reads_jars() {
        let jar = zip(&[
            (
                "assets/create/lang/en_us.json",
                r#"{"block.create.press": "Mechanical Press", "x": "Y"}"#,
            ),
            (
                "assets/create/lang/fr_fr.json",
                r#"{"block.create.press": "Presse mécanique"}"#,
            ),
            ("assets/create/lang/de_de.json", r#"{"x": "Z"}"#),
            (
                "assets/create/patchouli_books/guide/en_us/entries/a.json",
                r#"{"name": "Basics", "pages": [{"type": "text", "text": "Hello $(item)world$()"}, {"title": "item.create.cog"}]}"#,
            ),
            (
                "assets/other/patchouli_books/done/en_us/b.json",
                r#"{"name": "Done"}"#,
            ),
            (
                "assets/other/patchouli_books/done/fr_fr/b.json",
                r#"{"name": "Fini"}"#,
            ),
        ]);
        let found = read_zip(jar, Format::Json, "fr_fr").unwrap();
        assert_eq!(found.english.len(), 1);
        assert_eq!(found.english[0].0, "create");
        assert_eq!(
            found.translated,
            [(
                "block.create.press".to_owned(),
                "Presse mécanique".to_owned()
            )]
        );
        assert_eq!(found.books.len(), 1);
        let book = &found.books[0];
        assert_eq!(
            book.entries,
            [
                ("/name".to_owned(), "Basics".to_owned()),
                (
                    "/pages/0/text".to_owned(),
                    "Hello $(item)world$()".to_owned()
                )
            ]
        );
        assert!(
            matches!(&book.target, Target::Book { dest: BookDest::Pack(p), .. } if p == "assets/create/patchouli_books/guide/{locale}/entries/a.json")
        );
    }

    #[test]
    fn reads_legacy_files_case_insensitively() {
        let jar = zip(&[
            ("assets/old/lang/en_US.lang", "tile.a.name=Old Block\n"),
            ("assets/old/lang/fr_FR.lang", "tile.a.name=Vieux bloc\n"),
        ]);
        let found = read_zip(jar, Format::Legacy, "fr_fr").unwrap();
        assert_eq!(
            found.english[0].1,
            [("tile.a.name".to_owned(), "Old Block".to_owned())]
        );
        assert_eq!(found.translated.len(), 1);
    }

    #[test]
    fn lists_enabled_packs() {
        let options = "lang:en_us\nresourcePacks:[\"vanilla\",\"file/a.zip\",\"file/tandem-translation-fr_fr\"]\n";
        assert_eq!(
            resource_packs(options),
            ["vanilla", "file/a.zip", "file/tandem-translation-fr_fr"]
        );
        assert!(resource_packs("lang:en_us").is_empty());
    }

    #[test]
    fn reads_quest_language_files() {
        let entries = quest_lang_entries(
            "{\n\tchapter.1.title: \"Start\"\n\t\"quest.2.quest_desc\": [\"a\", \"b\"]\n}",
        );
        assert_eq!(
            entries,
            [
                ("chapter.1.title".to_owned(), "Start".to_owned()),
                ("quest.2.quest_desc#0".to_owned(), "a".to_owned()),
                ("quest.2.quest_desc#1".to_owned(), "b".to_owned()),
            ]
        );
    }
}
