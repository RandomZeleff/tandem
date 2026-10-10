//! Instances of other launchers (Modrinth App, Prism / MultiMC, CurseForge app, official
//! launcher), found where those launchers keep them or in a folder the player picks, and
//! copied into new Tandem instances. The originals are never modified.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use crate::content::{modrinth, mrpack};
use crate::context::Context;
use crate::download::Progress;
use crate::error::{Error, Result};
use crate::install::{InstallProgress, Stage};
use crate::instance::{self, Instance, NewInstance, PackOrigin};
use crate::meta::loader::Loader;
use crate::{store, worlds};

/// Where an instance was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    ModrinthApp,
    Prism,
    CurseForge,
    Official,
    /// A game folder picked by the player, read without any launcher metadata.
    Folder,
}

/// Game version of an official launcher profile that follows the latest release.
const LATEST_RELEASE: &str = "latest-release";
const LATEST_SNAPSHOT: &str = "latest-snapshot";

/// An instance of another launcher, ready to be imported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub source: Source,
    pub name: String,
    /// The game folder (`.minecraft`), which is what gets copied.
    pub game_dir: PathBuf,
    /// A version id, or `latest-release` / `latest-snapshot` for official profiles.
    pub game_version: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    /// Modrinth modpack it was installed from, so updates keep working in Tandem.
    pub pack_project_id: Option<String>,
    pub pack_version_id: Option<String>,
    pub icon: Option<PathBuf>,
    pub mods: usize,
    pub worlds: usize,
    /// Unix milliseconds.
    pub last_played: Option<i64>,
}

impl Found {
    fn new(source: Source, name: String, game_dir: PathBuf, game_version: String) -> Self {
        Self {
            source,
            name,
            game_dir,
            game_version,
            loader: Loader::Vanilla,
            loader_version: None,
            pack_project_id: None,
            pack_version_id: None,
            icon: None,
            mods: 0,
            worlds: 0,
            last_played: None,
        }
    }

    /// Fills in what is read from the game folder itself.
    fn counted(mut self) -> Self {
        // A vanilla instance does not take the mods folder along (see `skipped`).
        self.mods = if self.loader == Loader::Vanilla {
            0
        } else {
            count_files(&self.game_dir.join("mods"), |name| {
                name.ends_with(".jar") || name.ends_with(".jar.disabled")
            })
        };
        self.worlds = std::fs::read_dir(self.game_dir.join("saves"))
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().join("level.dat").is_file())
                    .count()
            })
            .unwrap_or(0);
        self
    }
}

fn count_files(dir: &Path, keep: impl Fn(&str) -> bool) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| keep(&e.file_name().to_string_lossy()))
                .count()
        })
        .unwrap_or(0)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

/// Reads a loader id as written by CurseForge (`forge-47.2.0`, `fabric-0.16.5-1.21.1`,
/// `neoforge-1.20.1-47.1.99`): the loader and its version, without the game version.
pub(crate) fn parse_loader_id(id: &str, game_version: &str) -> Result<(Loader, Option<String>)> {
    let (name, version) = id.split_once('-').unwrap_or((id, ""));
    let loader = match name.to_ascii_lowercase().as_str() {
        "forge" => Loader::Forge,
        "neoforge" => Loader::NeoForge,
        "fabric" => Loader::Fabric,
        "quilt" => Loader::Quilt,
        "vanilla" | "" => return Ok((Loader::Vanilla, None)),
        other => return Err(Error::LoaderNotSupported(other.to_owned())),
    };
    let version = version
        .strip_prefix(&format!("{game_version}-"))
        .unwrap_or(version);
    let version = version
        .strip_suffix(&format!("-{game_version}"))
        .unwrap_or(version);
    Ok((loader, Some(version.to_owned()).filter(|v| !v.is_empty())))
}

/// Unix milliseconds of an ISO 8601 date (`2026-01-01T12:00:00.000Z`), read as UTC: only
/// used to sort instances by last use.
fn parse_timestamp(text: &str) -> Option<i64> {
    let num = |range: std::ops::Range<usize>| text.get(range)?.parse::<i64>().ok();
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, min, s) = (num(11..13)?, num(14..16)?, num(17..19)?);
    // Days from the civil date (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 24 + h) * 60 + min) * 60_000 + s * 1000).filter(|t| *t > 0)
}

// ---------------------------------------------------------------------------------------
// Scanning

/// Instances of every launcher found at its usual place. Unreadable ones are skipped.
pub async fn scan() -> Vec<Found> {
    let mut found = Vec::new();
    for dir in modrinth_app_dirs() {
        found.extend(modrinth_app(&dir).await);
    }
    let rest = mrpack::blocking(|| {
        let mut found = Vec::new();
        for dir in prism_dirs() {
            found.extend(prism_root(&dir));
        }
        if let Some(dir) = curseforge_dir() {
            found.extend(curseforge_root(&dir));
        }
        if let Some(dir) = official_dir() {
            found.extend(official(&dir));
        }
        Ok(found)
    })
    .await
    .unwrap_or_default();
    found.extend(rest);
    found
}

/// Instances in a folder picked by the player: a launcher's folder, one instance of a
/// launcher, or a bare game folder (version and loader then guessed from its content).
pub async fn inspect_folder(ctx: &Context, dir: &Path) -> Result<Vec<Found>> {
    if dir.join("app.db").is_file() {
        return Ok(modrinth_app(dir).await);
    }
    let path = dir.to_owned();
    let known = mrpack::blocking(move || Ok(known_layout(&path))).await?;
    if let Some(found) = known {
        return Ok(found);
    }
    Ok(vec![guess_folder(ctx, dir).await?])
}

fn known_layout(dir: &Path) -> Option<Vec<Found>> {
    if dir.join("instance.cfg").is_file() {
        return prism_instance(dir, None).map(|f| vec![f]);
    }
    if dir.join("minecraftinstance.json").is_file() {
        return curseforge_instance(dir).map(|f| vec![f]);
    }
    if dir.join("launcher_profiles.json").is_file() {
        return Some(official(dir));
    }
    if dir.join("instances").is_dir() {
        let found = prism_root(dir);
        if !found.is_empty() {
            return Some(found);
        }
    }
    if dir.join("profiles").is_dir() {
        // A Modrinth App folder without its database: each profile is a bare game folder.
        return None;
    }
    let children: Vec<Found> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter_map(|p| {
            if p.join("instance.cfg").is_file() {
                prism_instance(&p, None)
            } else if p.join("minecraftinstance.json").is_file() {
                curseforge_instance(&p)
            } else {
                None
            }
        })
        .collect();
    (!children.is_empty()).then_some(children)
}

// Modrinth App ---------------------------------------------------------------------------

fn modrinth_app_dirs() -> Vec<PathBuf> {
    let Some(data) = dirs::data_dir() else {
        return Vec::new();
    };
    ["ModrinthApp", "com.modrinth.theseus"]
        .iter()
        .map(|name| data.join(name))
        .filter(|dir| dir.join("app.db").is_file())
        .collect()
}

/// Reads the app's database, read-only, without taking its lock.
async fn modrinth_app(dir: &Path) -> Vec<Found> {
    match read_modrinth_app(dir).await {
        Ok(found) => found,
        Err(err) => {
            tracing::warn!(dir = %dir.display(), error = %err, "could not read the Modrinth App database");
            Vec::new()
        }
    }
}

async fn read_modrinth_app(dir: &Path) -> Result<Vec<Found>> {
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::{ConnectOptions, Row};

    let mut conn = SqliteConnectOptions::new()
        .filename(dir.join("app.db"))
        .read_only(true)
        .immutable(true)
        .connect()
        .await?;
    let profiles_dir = dir.join("profiles");
    // Current schema (2026): the version lives in the applied content set.
    let current = sqlx::query(
        "SELECT i.path, i.name, i.icon_path, i.last_played, s.game_version, s.loader, s.loader_version,
                l.modrinth_project_id, l.modrinth_version_id
         FROM instances i
         LEFT JOIN instance_content_sets s ON s.id = i.applied_content_set_id
         LEFT JOIN instance_links l ON l.instance_id = i.id",
    )
    .fetch_all(&mut conn)
    .await;
    let rows = match current {
        Ok(rows) => rows,
        // Older schema: one `profiles` table.
        Err(_) => {
            sqlx::query(
                "SELECT path, name, icon_path, last_played, game_version, mod_loader AS loader,
                        mod_loader_version AS loader_version,
                        linked_project_id AS modrinth_project_id, linked_version_id AS modrinth_version_id
                 FROM profiles",
            )
            .fetch_all(&mut conn)
            .await?
        }
    };
    let found = rows
        .iter()
        .filter_map(|row| {
            let path: String = row.try_get("path").ok()?;
            let game_dir = if Path::new(&path).is_absolute() {
                PathBuf::from(&path)
            } else {
                profiles_dir.join(&path)
            };
            let game_version: String = row.try_get("game_version").ok()?;
            let loader: Option<String> = row.try_get("loader").ok()?;
            let (loader, _) = parse_loader_id(loader.as_deref().unwrap_or(""), "").ok()?;
            let mut found = Found::new(
                Source::ModrinthApp,
                row.try_get("name").unwrap_or(path),
                game_dir,
                game_version,
            );
            found.loader = loader;
            found.loader_version = row.try_get("loader_version").ok().flatten();
            found.pack_project_id = row.try_get("modrinth_project_id").ok().flatten();
            found.pack_version_id = row.try_get("modrinth_version_id").ok().flatten();
            found.icon = row
                .try_get::<Option<String>, _>("icon_path")
                .ok()
                .flatten()
                .map(PathBuf::from);
            // Seconds in the current schema.
            found.last_played = row
                .try_get::<Option<i64>, _>("last_played")
                .ok()
                .flatten()
                .map(|s| if s < 100_000_000_000 { s * 1000 } else { s });
            found.game_dir.is_dir().then(|| found.counted())
        })
        .collect();
    Ok(found)
}

// Prism Launcher / MultiMC ---------------------------------------------------------------

fn prism_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(data) = dirs::data_dir() {
        dirs.push(data.join("PrismLauncher"));
        dirs.push(data.join("PolyMC"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher"));
    }
    dirs.retain(|d| d.is_dir());
    dirs
}

/// `key=value` lines of an INI-like file, sections ignored.
fn read_ini(path: &Path) -> HashMap<String, String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect()
}

/// All instances of a Prism / MultiMC folder (custom instance folder honoured).
fn prism_root(dir: &Path) -> Vec<Found> {
    let config = ["prismlauncher.cfg", "polymc.cfg", "multimc.cfg"]
        .iter()
        .map(|name| read_ini(&dir.join(name)))
        .find(|c| !c.is_empty())
        .unwrap_or_default();
    let instances = config
        .get("InstanceDir")
        .filter(|d| !d.is_empty())
        .map(|d| dir.join(d))
        .unwrap_or_else(|| dir.join("instances"));
    let icons = config
        .get("IconsDir")
        .filter(|d| !d.is_empty())
        .map(|d| dir.join(d))
        .unwrap_or_else(|| dir.join("icons"));
    std::fs::read_dir(&instances)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| prism_instance(&e.path(), Some(&icons)))
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Deserialize)]
struct MmcPack {
    #[serde(default)]
    components: Vec<MmcComponent>,
}

#[derive(Deserialize)]
struct MmcComponent {
    uid: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default, rename = "cachedVersion")]
    cached_version: Option<String>,
}

fn prism_instance(dir: &Path, icons: Option<&Path>) -> Option<Found> {
    let cfg = read_ini(&dir.join("instance.cfg"));
    let pack: MmcPack = read_json(&dir.join("mmc-pack.json"))?;
    let version_of = |uid: &str| {
        pack.components
            .iter()
            .find(|c| c.uid == uid)
            .and_then(|c| c.version.clone().or(c.cached_version.clone()))
    };
    let game_version = version_of("net.minecraft")?;
    let (loader, loader_version) = [
        ("net.neoforged", Loader::NeoForge),
        ("net.minecraftforge", Loader::Forge),
        ("net.fabricmc.fabric-loader", Loader::Fabric),
        ("org.quiltmc.quilt-loader", Loader::Quilt),
    ]
    .iter()
    .find_map(|(uid, loader)| version_of(uid).map(|v| (*loader, Some(v))))
    .unwrap_or((Loader::Vanilla, None));
    let game_dir = [".minecraft", "minecraft"]
        .iter()
        .map(|d| dir.join(d))
        .find(|d| d.is_dir())
        .unwrap_or_else(|| dir.join(".minecraft"));
    let name = cfg
        .get("name")
        .cloned()
        .unwrap_or_else(|| dir.file_name().unwrap_or_default().to_string_lossy().into());
    let mut found = Found::new(Source::Prism, name, game_dir, game_version);
    found.loader = loader;
    found.loader_version = loader_version;
    if cfg.get("ManagedPack").is_some_and(|v| v == "true")
        && cfg.get("ManagedPackType").is_some_and(|v| v == "modrinth")
    {
        found.pack_project_id = cfg.get("ManagedPackID").cloned();
        found.pack_version_id = cfg.get("ManagedPackVersionID").cloned();
    }
    // Picked alone, the instance still finds the launcher's `icons/` next to `instances/`.
    let icons = icons
        .map(Path::to_owned)
        .or_else(|| Some(dir.parent()?.parent()?.join("icons")));
    let icon_dirs = [Some(dir.to_owned()), icons];
    found.icon = cfg.get("iconKey").and_then(|key| {
        icon_dirs.iter().flatten().find_map(|d| {
            ["png", "jpg", "jpeg", "ico"]
                .iter()
                .map(|ext| d.join(format!("{key}.{ext}")))
                .find(|p| p.is_file())
        })
    });
    found.last_played = cfg
        .get("lastLaunchTime")
        .and_then(|t| t.parse().ok())
        .filter(|t: &i64| *t > 0);
    Some(found.counted())
}

// CurseForge app -------------------------------------------------------------------------

fn curseforge_dir() -> Option<PathBuf> {
    let dir = dirs::home_dir()?.join("curseforge/minecraft/Instances");
    dir.is_dir().then_some(dir)
}

fn curseforge_root(dir: &Path) -> Vec<Found> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| curseforge_instance(&e.path()))
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfInstance {
    name: String,
    game_version: String,
    #[serde(default)]
    base_mod_loader: Option<CfLoader>,
    #[serde(default)]
    profile_image_path: Option<String>,
    #[serde(default)]
    last_played: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfLoader {
    name: String,
    #[serde(default)]
    minecraft_version: Option<String>,
}

fn curseforge_instance(dir: &Path) -> Option<Found> {
    let cf: CfInstance = read_json(&dir.join("minecraftinstance.json"))?;
    let game_version = cf
        .base_mod_loader
        .as_ref()
        .and_then(|l| l.minecraft_version.clone())
        .unwrap_or(cf.game_version);
    let (loader, loader_version) = match &cf.base_mod_loader {
        Some(l) => parse_loader_id(&l.name, &game_version).ok()?,
        None => (Loader::Vanilla, None),
    };
    let mut found = Found::new(Source::CurseForge, cf.name, dir.to_owned(), game_version);
    found.loader = loader;
    found.loader_version = loader_version;
    found.icon = cf
        .profile_image_path
        .map(PathBuf::from)
        .filter(|p| p.is_file());
    found.last_played = cf.last_played.and_then(|t| parse_timestamp(&t));
    Some(found.counted())
}

// Official launcher ----------------------------------------------------------------------

fn official_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let dir = dirs::data_dir()?.join("minecraft");
    #[cfg(windows)]
    let dir = dirs::data_dir()?.join(".minecraft");
    #[cfg(not(any(windows, target_os = "macos")))]
    let dir = dirs::home_dir()?.join(".minecraft");
    dir.join("launcher_profiles.json").is_file().then_some(dir)
}

#[derive(Deserialize)]
struct LauncherProfiles {
    #[serde(default)]
    profiles: HashMap<String, LauncherProfile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LauncherProfile {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    last_version_id: String,
    #[serde(default)]
    game_dir: Option<String>,
    #[serde(default)]
    last_used: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionJson {
    #[serde(default)]
    inherits_from: Option<String>,
}

fn official(root: &Path) -> Vec<Found> {
    let Some(file) = read_json::<LauncherProfiles>(&root.join("launcher_profiles.json")) else {
        return Vec::new();
    };
    let mut profiles: Vec<LauncherProfile> = file.profiles.into_values().collect();
    profiles.sort_by(|a, b| b.last_used.cmp(&a.last_used));
    profiles
        .into_iter()
        .filter_map(|p| {
            let inherits = read_json::<VersionJson>(
                &root
                    .join("versions")
                    .join(&p.last_version_id)
                    .join(format!("{}.json", p.last_version_id)),
            )
            .and_then(|v| v.inherits_from);
            let (game_version, loader, loader_version) = match p.kind.as_str() {
                "latest-release" => (LATEST_RELEASE.to_owned(), Loader::Vanilla, None),
                "latest-snapshot" => (LATEST_SNAPSHOT.to_owned(), Loader::Vanilla, None),
                _ => parse_version_id(&p.last_version_id, inherits.as_deref())?,
            };
            let name = match (p.name.trim(), p.kind.as_str()) {
                ("", "latest-release") => "Minecraft (dernière version)".to_owned(),
                ("", "latest-snapshot") => "Minecraft (dernière snapshot)".to_owned(),
                ("", _) => format!("Minecraft {game_version}"),
                (name, _) => name.to_owned(),
            };
            let game_dir = p
                .game_dir
                .filter(|d| !d.trim().is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| root.to_owned());
            let mut found = Found::new(Source::Official, name, game_dir, game_version);
            found.loader = loader;
            found.loader_version = loader_version;
            found.last_played = p.last_used.and_then(|t| parse_timestamp(&t));
            Some(found.counted())
        })
        .collect()
}

/// Game version and loader of an official launcher version id, helped by the
/// `inheritsFrom` of its JSON when the version is installed.
fn parse_version_id(id: &str, inherits: Option<&str>) -> Option<(String, Loader, Option<String>)> {
    if id.trim().is_empty() {
        return None;
    }
    let lower = id.to_ascii_lowercase();
    for (prefix, loader) in [
        ("fabric-loader-", Loader::Fabric),
        ("quilt-loader-", Loader::Quilt),
    ] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            // `fabric-loader-0.16.5-1.21.1`: the loader version has no dash.
            let (version, game) = rest.split_once('-')?;
            let game = inherits.unwrap_or(game);
            return Some((game.to_owned(), loader, Some(version.to_owned())));
        }
    }
    if let Some(version) = lower.strip_prefix("neoforge-") {
        let game = inherits
            .map(str::to_owned)
            .or_else(|| neoforge_game(version))?;
        return Some((game, Loader::NeoForge, Some(version.to_owned())));
    }
    if let Some(at) = lower.find("forge") {
        // `1.20.1-forge-47.2.0`, `1.12.2-forge1.12.2-14.23.5.2860`, `1.7.10-Forge10.13.4.1614-1.7.10`.
        let game = inherits
            .map(str::to_owned)
            .unwrap_or_else(|| id[..at].trim_end_matches('-').to_owned());
        let rest = id[at + "forge".len()..].trim_start_matches('-');
        let rest = rest.strip_prefix(&format!("{game}-")).unwrap_or(rest);
        let rest = rest.strip_suffix(&format!("-{game}")).unwrap_or(rest);
        return (!game.is_empty()).then(|| (game, Loader::Forge, Some(rest.to_owned())));
    }
    Some((inherits.unwrap_or(id).to_owned(), Loader::Vanilla, None))
}

/// NeoForge versions follow the game: `21.1.77` → `1.21.1`, `20.4.237` → `1.20.4`.
fn neoforge_game(version: &str) -> Option<String> {
    let mut parts = version.split('.');
    let (major, minor) = (parts.next()?, parts.next()?);
    let major: u32 = major.parse().ok()?;
    if major >= 25 {
        // Year-based game versions (26.1…): NeoForge `26.1.0.x` → `26.1`.
        return Some(format!("{major}.{minor}"));
    }
    Some(if minor == "0" {
        format!("1.{major}")
    } else {
        format!("1.{major}.{minor}")
    })
}

// Bare folder ----------------------------------------------------------------------------

/// Guesses the version and loader of a game folder from its mods (looked up on Modrinth)
/// and its worlds.
async fn guess_folder(ctx: &Context, dir: &Path) -> Result<Found> {
    let path = dir.to_owned();
    let (jars, world_version) = mrpack::blocking(move || {
        let jars: Vec<String> = std::fs::read_dir(path.join("mods"))
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e == "jar"))
                    .take(60)
                    .filter_map(|p| std::fs::read(p).ok())
                    .map(|bytes| sha1_smol::Sha1::from(&bytes).digest().to_string())
                    .collect()
            })
            .unwrap_or_default();
        let mut worlds = worlds::list(&path);
        worlds.sort_by_key(|w| std::cmp::Reverse(w.last_played));
        Ok((jars, worlds.into_iter().find_map(|w| w.version)))
    })
    .await?;

    let mut game_votes: HashMap<String, usize> = HashMap::new();
    let mut loader_votes: HashMap<Loader, usize> = HashMap::new();
    if !jars.is_empty() {
        for version in modrinth::versions_by_hash(ctx, &jars)
            .await
            .unwrap_or_default()
            .values()
        {
            for game in &version.game_versions {
                *game_votes.entry(game.clone()).or_default() += 1;
            }
            for loader in &version.loaders {
                if let Ok((loader, _)) = parse_loader_id(loader, "") {
                    *loader_votes.entry(loader).or_default() += 1;
                }
            }
        }
    }
    let loader = loader_votes
        .into_iter()
        .filter(|(l, _)| *l != Loader::Vanilla)
        .max_by_key(|(l, n)| (*n, *l == Loader::Fabric))
        .map(|(l, _)| l)
        .unwrap_or(Loader::Vanilla);
    // The world's version when the mods agree with it, else the one most mods support.
    let game_version = match world_version {
        Some(v) if game_votes.is_empty() || game_votes.contains_key(&v) => v,
        _ => game_votes
            .into_iter()
            .max_by_key(|(_, n)| *n)
            .map(|(v, _)| v)
            .ok_or_else(|| {
                Error::InvalidInput(
                    "Impossible de reconnaître la version du jeu de ce dossier".into(),
                )
            })?,
    };
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().trim_start_matches('.').to_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Instance importée".into());
    let mut found = Found::new(Source::Folder, name, dir.to_owned(), game_version);
    found.loader = loader;
    Ok(found.counted())
}

// ---------------------------------------------------------------------------------------
// Import

/// Top-level entries never copied: logs, caches and, in an official `.minecraft`, the
/// launcher's own files (shared versions, libraries, assets).
const SKIPPED: &[&str] = &[
    "logs",
    "crash-reports",
    ".cache",
    "cache",
    "natives",
    "jfr",
    "debug",
    "assets",
    "libraries",
    "versions",
    "bin",
    "runtime",
    "webcache",
    "webcache2",
    "launcher_accounts.json",
    "launcher_profiles.json",
    "launcher_settings.json",
    "launcher_ui_state.json",
    "launcher_log.txt",
    "launcher_cef_log.txt",
    "treatment_tags.json",
    "clientId.txt",
    "minecraftinstance.json",
    "manifest.json",
    "modlist.html",
];

/// Content folders whose files go to the shared store, like installed content.
const STORED: &[&str] = &["mods", "resourcepacks", "shaderpacks"];

fn skipped(top: &str, loader: Loader) -> bool {
    SKIPPED.contains(&top)
        || top.starts_with("launcher_")
        || (loader == Loader::Vanilla && top == "mods")
}

/// Copies `found` into a new instance. The new instance is removed if anything fails.
pub async fn import<F>(
    ctx: &Context,
    found: &Found,
    created: &Instance,
    on_progress: F,
) -> Result<()>
where
    F: Fn(InstallProgress) + Send + Sync + 'static,
{
    on_progress(InstallProgress {
        stage: Stage::Metadata,
        download: Progress::default(),
    });
    if let (Some(project), Some(version)) = (&found.pack_project_id, &found.pack_version_id) {
        // Only kept when Modrinth still knows that version (and gives its number).
        match modrinth::version(ctx, version).await {
            Ok(v) if v.project_id == *project => {
                let icon = modrinth::project(ctx, project)
                    .await
                    .ok()
                    .and_then(|p| p.icon_url);
                let origin = PackOrigin {
                    project_id: project.clone(),
                    version_id: version.clone(),
                    version: v.version_number,
                    icon,
                };
                ctx.db.set_instance_pack(&created.id, &origin).await?;
            }
            Ok(_) => {}
            Err(err) => tracing::warn!(error = %err, "modpack origin not found on Modrinth"),
        }
    }

    let (data, from, to, loader) = (
        ctx.data.clone(),
        found.game_dir.clone(),
        ctx.data.instance_dir(&created.id),
        found.loader,
    );
    let placements =
        mrpack::blocking(move || copy_game_dir(&data, &from, &to, loader, &on_progress)).await?;
    if let Err(err) = mrpack::track(ctx, created, &placements).await {
        tracing::warn!(instance = %created.id, error = %err, "could not identify imported files on Modrinth");
    }
    if let Some(icon) = found.icon.as_deref().filter(|p| p.is_file()) {
        if let Err(err) = instance::set_icon(ctx, &created.id, Some(icon), None).await {
            tracing::debug!(error = %err, "imported icon unreadable");
        }
    }
    tracing::info!(instance = %created.id, from = %found.game_dir.display(), source = ?found.source, files = placements.len(), "instance imported");
    Ok(())
}

/// The instance to create for `found`, with "latest" versions resolved.
pub async fn new_instance(ctx: &Context, found: &Found) -> Result<NewInstance> {
    let game_version = match found.game_version.as_str() {
        LATEST_RELEASE | LATEST_SNAPSHOT => {
            let manifest = crate::meta::fetch_manifest(ctx).await?;
            if found.game_version == LATEST_RELEASE {
                manifest.latest.release
            } else {
                manifest.latest.snapshot
            }
        }
        v => v.to_owned(),
    };
    Ok(NewInstance {
        name: found.name.chars().take(64).collect(),
        game_version,
        loader: found.loader,
        loader_version: found.loader_version.clone(),
    })
}

/// Walks the game folder, copying files and putting content into the store. Returns the
/// stored files (path in the instance, SHA-1) for Modrinth identification.
fn copy_game_dir(
    data: &crate::paths::DataDir,
    from: &Path,
    to: &Path,
    loader: Loader,
    on_progress: &(dyn Fn(InstallProgress) + Send + Sync),
) -> Result<Vec<(PathBuf, String)>> {
    if !from.is_dir() {
        return Err(Error::InvalidInput(format!(
            "Dossier introuvable : {}",
            from.display()
        )));
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let top = entry.file_name().to_string_lossy().into_owned();
        if skipped(&top, loader) {
            continue;
        }
        collect(&entry.path(), &mut files)?;
    }
    let total_bytes: u64 = files.iter().map(|(_, size)| size).sum();
    let (done_files, done_bytes) = (AtomicUsize::new(0), AtomicU64::new(0));
    let report = || {
        on_progress(InstallProgress {
            stage: Stage::Copying,
            download: Progress {
                done_files: done_files.load(Ordering::Relaxed),
                total_files: files.len(),
                done_bytes: done_bytes.load(Ordering::Relaxed),
                total_bytes,
            },
        })
    };
    report();
    let mut placements = Vec::new();
    let mut last_report = std::time::Instant::now();
    for (path, size) in &files {
        let Ok(relative) = path.strip_prefix(from) else {
            continue;
        };
        let dest = to.join(relative);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let in_store = relative.components().count() == 2
            && relative
                .components()
                .next()
                .is_some_and(|c| STORED.contains(&c.as_os_str().to_string_lossy().as_ref()));
        if in_store {
            let bytes = std::fs::read(path)?;
            let sha1 = sha1_smol::Sha1::from(&bytes).digest().to_string();
            let stored = store::path(data, &sha1);
            if !stored.is_file() {
                if let Some(parent) = stored.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let partial = stored.with_extension("part");
                std::fs::write(&partial, &bytes)?;
                std::fs::rename(&partial, &stored)?;
            }
            store::link_blocking(&stored, &dest)?;
            placements.push((relative.to_owned(), sha1));
        } else {
            std::fs::copy(path, &dest)?;
        }
        done_files.fetch_add(1, Ordering::Relaxed);
        done_bytes.fetch_add(*size, Ordering::Relaxed);
        if last_report.elapsed() >= std::time::Duration::from_millis(100) {
            last_report = std::time::Instant::now();
            report();
        }
    }
    report();
    Ok(placements)
}

/// Files under `path` (or `path` itself), with their sizes. Symlinks are not followed.
fn collect(path: &Path, files: &mut Vec<(PathBuf, u64)>) -> Result<()> {
    let meta = std::fs::symlink_metadata(path)?;
    if meta.is_dir() {
        for entry in std::fs::read_dir(path)? {
            collect(&entry?.path(), files)?;
        }
    } else if meta.is_file() {
        files.push((path.to_owned(), meta.len()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_loader_ids() {
        let cases = [
            ("forge-47.2.0", "1.20.1", Loader::Forge, Some("47.2.0")),
            (
                "fabric-0.18.1-1.21.1",
                "1.21.1",
                Loader::Fabric,
                Some("0.18.1"),
            ),
            (
                "neoforge-1.20.1-47.1.99",
                "1.20.1",
                Loader::NeoForge,
                Some("47.1.99"),
            ),
            (
                "neoforge-21.1.77",
                "1.21.1",
                Loader::NeoForge,
                Some("21.1.77"),
            ),
            ("quilt", "1.20.1", Loader::Quilt, None),
        ];
        for (id, game, loader, version) in cases {
            assert_eq!(
                parse_loader_id(id, game).unwrap(),
                (loader, version.map(str::to_owned)),
                "{id}"
            );
        }
        assert!(parse_loader_id("liteloader-1", "1.7.10").is_err());
    }

    #[test]
    fn parses_official_version_ids() {
        let v = |id: &str, inherits: Option<&str>| parse_version_id(id, inherits).unwrap();
        assert_eq!(v("1.21.8", None), ("1.21.8".into(), Loader::Vanilla, None));
        assert_eq!(
            v("fabric-loader-0.16.5-1.21.1", None),
            ("1.21.1".into(), Loader::Fabric, Some("0.16.5".into()))
        );
        assert_eq!(
            v("1.20.1-forge-47.2.0", None),
            ("1.20.1".into(), Loader::Forge, Some("47.2.0".into()))
        );
        assert_eq!(
            v("1.12.2-forge1.12.2-14.23.5.2860", None),
            ("1.12.2".into(), Loader::Forge, Some("14.23.5.2860".into()))
        );
        assert_eq!(
            v("1.7.10-Forge10.13.4.1614-1.7.10", None),
            ("1.7.10".into(), Loader::Forge, Some("10.13.4.1614".into()))
        );
        assert_eq!(
            v("neoforge-21.1.77", None),
            ("1.21.1".into(), Loader::NeoForge, Some("21.1.77".into()))
        );
        assert_eq!(
            v("neoforge-20.4.237", Some("1.20.4")),
            ("1.20.4".into(), Loader::NeoForge, Some("20.4.237".into()))
        );
        assert_eq!(neoforge_game("21.0.1").as_deref(), Some("1.21"));
        assert!(parse_version_id("", None).is_none());
    }

    #[test]
    fn parses_timestamps() {
        assert_eq!(parse_timestamp("1970-01-02T00:00:00Z"), Some(86_400_000));
        assert_eq!(
            parse_timestamp("2026-10-10T12:30:15.123Z"),
            Some(1_791_635_415_000)
        );
        assert_eq!(parse_timestamp("0001-01-01T00:00:00"), None);
        assert_eq!(parse_timestamp("bad"), None);
    }

    #[test]
    fn reads_prism_instances() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let inst = root.join("instances/Mon pack");
        std::fs::create_dir_all(inst.join(".minecraft/mods")).unwrap();
        std::fs::create_dir_all(inst.join(".minecraft/saves/Monde/")).unwrap();
        std::fs::write(inst.join(".minecraft/saves/Monde/level.dat"), "").unwrap();
        std::fs::write(inst.join(".minecraft/mods/a.jar"), "a").unwrap();
        std::fs::write(inst.join(".minecraft/mods/b.jar.disabled"), "b").unwrap();
        std::fs::write(
            inst.join("instance.cfg"),
            "[General]\nname=Mon pack\niconKey=pack\nManagedPack=true\nManagedPackType=modrinth\nManagedPackID=abc\nManagedPackVersionID=v1\nlastLaunchTime=1700000000000\n",
        )
        .unwrap();
        std::fs::write(
            inst.join("mmc-pack.json"),
            r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.fabricmc.fabric-loader","version":"0.16.5"}],"formatVersion":1}"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.join("icons")).unwrap();
        std::fs::write(root.join("icons/pack.png"), "png").unwrap();

        let found = prism_root(root);
        assert_eq!(found.len(), 1);
        let f = &found[0];
        assert_eq!(
            (
                f.name.as_str(),
                f.game_version.as_str(),
                f.loader,
                f.loader_version.as_deref()
            ),
            ("Mon pack", "1.20.1", Loader::Fabric, Some("0.16.5"))
        );
        assert_eq!((f.mods, f.worlds), (2, 1));
        assert_eq!(f.pack_project_id.as_deref(), Some("abc"));
        assert_eq!(
            f.icon.as_deref(),
            Some(root.join("icons/pack.png").as_path())
        );
        assert_eq!(f.last_played, Some(1_700_000_000_000));
        assert_eq!(known_layout(&inst).unwrap(), found);
    }

    #[test]
    fn reads_official_profiles() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("versions/fabric-loader-0.16.5-1.21.1")).unwrap();
        std::fs::write(
            root.join("versions/fabric-loader-0.16.5-1.21.1/fabric-loader-0.16.5-1.21.1.json"),
            r#"{"inheritsFrom": "1.21.1"}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("launcher_profiles.json"),
            r#"{"profiles": {
                "a": {"name": "", "type": "latest-release", "lastVersionId": "latest-release", "lastUsed": "2026-01-01T00:00:00.000Z"},
                "b": {"name": "Fabric", "type": "custom", "lastVersionId": "fabric-loader-0.16.5-1.21.1", "gameDir": "D:\\Jeux\\fabric", "lastUsed": "2026-02-01T00:00:00.000Z"}
            }}"#,
        )
        .unwrap();
        let found = official(root);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "Fabric");
        assert_eq!(found[0].loader, Loader::Fabric);
        assert_eq!(found[0].game_dir, PathBuf::from("D:\\Jeux\\fabric"));
        assert_eq!(found[1].game_version, LATEST_RELEASE);
        assert_eq!(found[1].game_dir, root);
    }

    #[test]
    fn copies_a_game_folder() {
        let dir = tempfile::tempdir().unwrap();
        let data = crate::paths::DataDir::new(dir.path().join("data"));
        let from = dir.path().join(".minecraft");
        for (path, body) in [
            ("mods/a.jar", "jar"),
            ("config/a.toml", "x=1"),
            ("saves/W/level.dat", "lvl"),
            ("logs/latest.log", "log"),
            ("versions/1.21/1.21.json", "{}"),
            ("launcher_profiles.json", "{}"),
            ("options.txt", "lang:fr_fr"),
        ] {
            let p = from.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        let to = dir.path().join("instance");
        let seen = std::sync::Mutex::new(Vec::new());
        let placements = copy_game_dir(&data, &from, &to, Loader::Fabric, &|p| {
            seen.lock().unwrap().push(p.download.done_files)
        })
        .unwrap();
        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].0, PathBuf::from("mods").join("a.jar"));
        assert!(store::path(&data, &placements[0].1).is_file());
        for kept in [
            "mods/a.jar",
            "config/a.toml",
            "saves/W/level.dat",
            "options.txt",
        ] {
            assert!(to.join(kept).is_file(), "{kept}");
        }
        for skipped in ["logs", "versions", "launcher_profiles.json"] {
            assert!(!to.join(skipped).exists(), "{skipped}");
        }
        assert_eq!(seen.lock().unwrap().last(), Some(&4));

        // A vanilla instance leaves the mods out.
        let vanilla = dir.path().join("vanilla");
        copy_game_dir(&data, &from, &vanilla, Loader::Vanilla, &|_| {}).unwrap();
        assert!(!vanilla.join("mods").exists());
    }
}
