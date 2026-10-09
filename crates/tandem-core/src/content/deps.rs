//! Mod-to-mod dependencies, read from the jars' own metadata (`fabric.mod.json`,
//! `quilt.mod.json`, `mods.toml`, `neoforge.mods.toml`), so disabling a library that
//! other mods need can be flagged before the game refuses to start.

use std::collections::{BTreeSet, HashMap};
use std::io::{Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::DISABLED_SUFFIX;
use crate::paths::DataDir;

/// What one jar in `mods/` provides and requires.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModDeps {
    /// File name on disk, `.disabled` included when the mod is off.
    pub file_name: String,
    pub name: String,
    pub enabled: bool,
    /// Mod ids this jar provides: its own, its `provides`, and those of jars nested in it.
    pub provides: BTreeSet<String>,
    /// Mod ids it cannot start without (loader, game and Java excluded).
    pub requires: BTreeSet<String>,
}

/// A mod that needs another one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependent {
    pub name: String,
    pub file_name: String,
}

/// The jar behind a mod id, when there is one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub mod_id: String,
    pub name: String,
    /// File name without `.disabled`, as tracked in the content list.
    pub file_name: String,
    pub enabled: bool,
}

/// Ids every instance has without a jar in `mods/`.
const PLATFORM: [&str; 9] = [
    "minecraft",
    "java",
    "fabricloader",
    "fabric-loader",
    "quilt_loader",
    "forge",
    "neoforge",
    "fml",
    "javafml",
];

/// A jar seen in a previous scan, identified by its size and modification time.
#[derive(Serialize, Deserialize)]
struct Cached {
    size: u64,
    modified: u128,
    /// `None` when the jar has no readable mod metadata.
    deps: Option<ModDeps>,
}

/// Every mod jar of the instance, enabled or not. Unreadable jars are skipped. Jars
/// unchanged since the last scan come from `cache/deps/<instance>.json`: reading 400+
/// jars takes seconds (tens of seconds on a cold disk), the cached scan milliseconds.
pub fn scan_cached(data: &DataDir, instance_id: &str, game_dir: &Path) -> Vec<ModDeps> {
    let cache_path = data
        .cache()
        .join("deps")
        .join(format!("{instance_id}.json"));
    let previous: HashMap<String, Cached> = std::fs::read(&cache_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let (mods, cache) = scan_with(game_dir, previous);
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec(&cache) {
        let _ = std::fs::write(&cache_path, bytes);
    }
    mods
}

/// Every mod jar of the instance, read from disk.
pub fn scan(game_dir: &Path) -> Vec<ModDeps> {
    scan_with(game_dir, HashMap::new()).0
}

fn scan_with(
    game_dir: &Path,
    mut previous: HashMap<String, Cached>,
) -> (Vec<ModDeps>, HashMap<String, Cached>) {
    let Ok(entries) = std::fs::read_dir(game_dir.join("mods")) else {
        return (Vec::new(), HashMap::new());
    };
    let mut cache = HashMap::new();
    let mut to_read: Vec<(String, PathBuf, u64, u128)> = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !file_name.ends_with(".jar") && !file_name.ends_with(&format!(".jar{DISABLED_SUFFIX}")) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let size = meta.len();
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        // Toggling a mod renames it: the cache is keyed by the name without `.disabled`.
        let key = file_name.trim_end_matches(DISABLED_SUFFIX).to_owned();
        match previous.remove(&key) {
            Some(hit) if hit.size == size && hit.modified == modified => {
                cache.insert(key, with_name(hit, &file_name));
            }
            _ => to_read.push((file_name, entry.path(), size, modified)),
        }
    }

    // Jars are independent: read them on every core.
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(16);
    let chunk = to_read.len().div_ceil(threads).max(1);
    let read: Vec<(String, Cached)> = std::thread::scope(|scope| {
        let handles: Vec<_> = to_read
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|(file_name, path, size, modified)| {
                            let deps = std::fs::File::open(path)
                                .ok()
                                .and_then(|f| read_jar(f, 0))
                                .map(|mut d| {
                                    if d.name.is_empty() {
                                        d.name = file_name
                                            .trim_end_matches(DISABLED_SUFFIX)
                                            .trim_end_matches(".jar")
                                            .to_owned();
                                    }
                                    d
                                });
                            let key = file_name.trim_end_matches(DISABLED_SUFFIX).to_owned();
                            (
                                key,
                                with_name(
                                    Cached {
                                        size: *size,
                                        modified: *modified,
                                        deps,
                                    },
                                    file_name,
                                ),
                            )
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });
    cache.extend(read);

    let mut mods: Vec<ModDeps> = cache.values().filter_map(|c| c.deps.clone()).collect();
    mods.sort_by_key(|m| m.name.to_lowercase());
    (mods, cache)
}

/// Stamps the current file name (and so the enabled state) on a cached entry.
fn with_name(mut cached: Cached, file_name: &str) -> Cached {
    if let Some(deps) = cached.deps.as_mut() {
        deps.file_name = file_name.to_owned();
        deps.enabled = file_name.ends_with(".jar");
    }
    cached
}

/// Enabled mods that would lose a requirement if `file_name` were turned off: they need
/// one of its ids and no other enabled jar provides it.
pub fn dependents(mods: &[ModDeps], file_name: &str) -> Vec<Dependent> {
    let base = file_name.trim_end_matches(DISABLED_SUFFIX);
    let Some(target) = mods
        .iter()
        .find(|m| m.file_name.trim_end_matches(DISABLED_SUFFIX) == base)
    else {
        return Vec::new();
    };
    let others = || {
        mods.iter()
            .filter(|m| m.enabled && m.file_name != target.file_name)
    };
    let lost: BTreeSet<&String> = target
        .provides
        .iter()
        .filter(|id| !others().any(|m| m.provides.contains(*id)))
        .collect();
    others()
        .filter(|m| m.requires.iter().any(|id| lost.contains(id)))
        .map(|m| Dependent {
            name: m.name.clone(),
            file_name: m.file_name.clone(),
        })
        .collect()
}

/// The jar providing each id, enabled ones first.
pub fn providers(mods: &[ModDeps], ids: &[String]) -> Vec<Provider> {
    ids.iter()
        .filter_map(|id| {
            let found = mods
                .iter()
                .filter(|m| m.provides.contains(id))
                .max_by_key(|m| m.enabled)?;
            Some(Provider {
                mod_id: id.clone(),
                name: found.name.clone(),
                file_name: found.file_name.trim_end_matches(DISABLED_SUFFIX).to_owned(),
                enabled: found.enabled,
            })
        })
        .collect()
}

/// Nested jars are read one level deep: enough for bundled libraries.
fn read_jar<R: Read + Seek>(reader: R, depth: u8) -> Option<ModDeps> {
    let mut archive = zip::ZipArchive::new(reader).ok()?;
    let mut deps = ModDeps::default();
    let mut nested = Vec::new();

    if let Some(json) = entry_text(&mut archive, "fabric.mod.json") {
        let meta: FabricMeta = serde_json::from_str(&json).ok()?;
        deps.name = meta.name.unwrap_or_else(|| meta.id.clone());
        deps.provides.insert(meta.id);
        deps.provides.extend(meta.provides);
        deps.requires.extend(meta.depends.into_keys());
        nested.extend(meta.jars.into_iter().map(|j| j.file));
    } else if let Some(json) = entry_text(&mut archive, "quilt.mod.json") {
        let meta: QuiltMeta = serde_json::from_str(&json).ok()?;
        let loader = meta.quilt_loader;
        deps.name = loader
            .metadata
            .and_then(|m| m.name)
            .unwrap_or_else(|| loader.id.clone());
        deps.provides.insert(loader.id);
        deps.provides
            .extend(loader.provides.into_iter().map(QuiltRef::id));
        deps.requires.extend(
            loader
                .depends
                .into_iter()
                .filter(|d| !d.optional())
                .map(QuiltRef::id),
        );
        nested.extend(loader.jars);
    } else {
        let toml = ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"]
            .iter()
            .find_map(|name| entry_text(&mut archive, name))?;
        let parsed = parse_mods_toml(&toml);
        deps.name = parsed.name.unwrap_or_default();
        deps.provides.extend(parsed.ids);
        deps.requires.extend(parsed.requires);
        nested.extend(
            archive
                .file_names()
                .filter(|n| n.starts_with("META-INF/jarjar/") && n.ends_with(".jar"))
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        );
    }

    if depth == 0 {
        for path in nested {
            let mut bytes = Vec::new();
            let read = archive
                .by_name(&path)
                .ok()
                .and_then(|mut e| e.read_to_end(&mut bytes).ok());
            if read.is_some() {
                if let Some(inner) = read_jar(Cursor::new(bytes), 1) {
                    deps.provides.extend(inner.provides);
                }
            }
        }
    }
    for id in PLATFORM {
        deps.requires.remove(id);
    }
    // A jar never depends on itself (some list their own id).
    let own = deps.provides.clone();
    deps.requires.retain(|id| !own.contains(id));
    Some(deps)
}

fn entry_text<R: Read + Seek>(archive: &mut zip::ZipArchive<R>, name: &str) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}

#[derive(Deserialize)]
struct FabricMeta {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    provides: Vec<String>,
    #[serde(default)]
    depends: std::collections::BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    jars: Vec<FabricJar>,
}

#[derive(Deserialize)]
struct FabricJar {
    file: String,
}

#[derive(Deserialize)]
struct QuiltMeta {
    quilt_loader: QuiltLoader,
}

#[derive(Deserialize)]
struct QuiltLoader {
    id: String,
    #[serde(default)]
    metadata: Option<QuiltMetadata>,
    #[serde(default)]
    provides: Vec<QuiltRef>,
    #[serde(default)]
    depends: Vec<QuiltRef>,
    #[serde(default)]
    jars: Vec<String>,
}

#[derive(Deserialize)]
struct QuiltMetadata {
    name: Option<String>,
}

/// `"id"` or `{ "id": …, "optional": … }`.
#[derive(Deserialize)]
#[serde(untagged)]
enum QuiltRef {
    Id(String),
    Full {
        id: String,
        #[serde(default)]
        optional: bool,
    },
}

impl QuiltRef {
    fn id(self) -> String {
        match self {
            QuiltRef::Id(id) | QuiltRef::Full { id, .. } => id,
        }
    }

    fn optional(&self) -> bool {
        matches!(self, QuiltRef::Full { optional: true, .. })
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ModsToml {
    name: Option<String>,
    ids: Vec<String>,
    requires: Vec<String>,
}

/// Just what is needed from a `mods.toml`: the `[[mods]]` ids and display name, and the
/// `[[dependencies.*]]` that are mandatory (Forge) or required (NeoForge).
fn parse_mods_toml(toml: &str) -> ModsToml {
    #[derive(PartialEq)]
    enum Section {
        Other,
        Mod,
        Dependency,
    }
    let mut parsed = ModsToml::default();
    let mut section = Section::Other;
    // Current dependency: (modId, required).
    let mut dep: Option<(Option<String>, bool)> = None;
    let flush = |dep: &mut Option<(Option<String>, bool)>, parsed: &mut ModsToml| {
        if let Some((Some(id), true)) = dep.take() {
            parsed.requires.push(id);
        }
    };

    for line in toml.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            flush(&mut dep, &mut parsed);
            section = if line == "[[mods]]" {
                Section::Mod
            } else if line.starts_with("[[dependencies.") {
                dep = Some((None, false));
                Section::Dependency
            } else {
                Section::Other
            };
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'').trim();
        match section {
            Section::Mod => match key {
                "modId" if !value.is_empty() => parsed.ids.push(value.to_owned()),
                "displayName" if parsed.name.is_none() && !value.starts_with("${") => {
                    parsed.name = Some(value.to_owned());
                }
                _ => {}
            },
            Section::Dependency => {
                if let Some((id, required)) = dep.as_mut() {
                    match key {
                        "modId" => *id = Some(value.to_owned()),
                        "mandatory" => *required = value == "true",
                        "type" => *required = value.eq_ignore_ascii_case("required"),
                        _ => {}
                    }
                }
            }
            Section::Other => {}
        }
    }
    flush(&mut dep, &mut parsed);
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn jar(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, content) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    fn fabric(id: &str, depends: &[&str]) -> Vec<u8> {
        let mut deps = vec![
            "\"minecraft\": \"1.20.1\"".to_owned(),
            "\"fabricloader\": \"*\"".to_owned(),
        ];
        deps.extend(depends.iter().map(|d| format!("\"{d}\": \"*\"")));
        let json = format!(
            r#"{{"id": "{id}", "name": "{id} mod", "depends": {{{}}}}}"#,
            deps.join(", ")
        );
        jar(&[("fabric.mod.json", json.as_bytes())])
    }

    #[test]
    fn reads_forge_and_neoforge_tomls() {
        let forge = r#"
modLoader="javafml"
[[mods]]
modId="pandalib"
displayName="PandaLib" # comment
[[dependencies.pandalib]]
    modId="forge"
    mandatory=true
[[dependencies.pandalib]]
    modId="architectury"
    mandatory=true
[[dependencies.pandalib]]
    modId="jei"
    mandatory=false
"#;
        let parsed = parse_mods_toml(forge);
        assert_eq!(parsed.ids, ["pandalib"]);
        assert_eq!(parsed.name.as_deref(), Some("PandaLib"));
        assert_eq!(parsed.requires, ["forge", "architectury"]);

        let neo = "[[mods]]\nmodId='a'\n[[dependencies.a]]\nmodId='b'\ntype=\"required\"\n[[dependencies.a]]\nmodId='c'\ntype=\"optional\"\n";
        assert_eq!(parse_mods_toml(neo).requires, ["b"]);
    }

    #[test]
    fn finds_dependents_and_providers() {
        let tmp = tempfile::tempdir().unwrap();
        let mods = tmp.path().join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        // A library bundling a second library inside it.
        let bundled = fabric("cloth-basic-math", &[]);
        let library = jar(&[
            (
                "fabric.mod.json",
                br#"{"id": "cloth-config", "name": "Cloth Config", "jars": [{"file": "META-INF/jars/math.jar"}]}"#,
            ),
            ("META-INF/jars/math.jar", &bundled),
        ]);
        std::fs::write(mods.join("cloth.jar"), library).unwrap();
        std::fs::write(mods.join("a.jar"), fabric("a", &["cloth-config"])).unwrap();
        std::fs::write(mods.join("b.jar"), fabric("b", &["cloth-basic-math"])).unwrap();
        std::fs::write(mods.join("c.jar.disabled"), fabric("c", &["cloth-config"])).unwrap();
        std::fs::write(mods.join("d.jar"), fabric("d", &[])).unwrap();

        let scanned = scan(tmp.path());
        assert_eq!(scanned.len(), 5);
        let cloth = scanned.iter().find(|m| m.file_name == "cloth.jar").unwrap();
        assert!(cloth.provides.contains("cloth-basic-math"));
        assert!(cloth.requires.is_empty());

        // c is disabled, d needs nothing: only a and b break.
        let names: Vec<_> = dependents(&scanned, "cloth.jar")
            .into_iter()
            .map(|d| d.file_name)
            .collect();
        assert_eq!(names, ["a.jar", "b.jar"]);
        assert!(dependents(&scanned, "d.jar").is_empty());

        let found = providers(
            &scanned,
            &["cloth-config".into(), "c".into(), "nope".into()],
        );
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].file_name, "c.jar");
        assert!(!found[1].enabled);
    }

    #[test]
    fn another_provider_keeps_dependents_safe() {
        let tmp = tempfile::tempdir().unwrap();
        let mods = tmp.path().join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("lib1.jar"), fabric("lib", &[])).unwrap();
        std::fs::write(
            mods.join("lib2.jar"),
            jar(&[(
                "fabric.mod.json",
                br#"{"id": "other", "provides": ["lib"]}"#,
            )]),
        )
        .unwrap();
        std::fs::write(mods.join("user.jar"), fabric("user", &["lib"])).unwrap();
        assert!(dependents(&scan(tmp.path()), "lib1.jar").is_empty());
    }
}
