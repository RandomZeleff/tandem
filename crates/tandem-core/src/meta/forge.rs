//! Forge and NeoForge (1.13+ installer format). Their installer jar carries:
//! - `version.json`: a launcher profile inheriting from vanilla, like Fabric's;
//! - `install_profile.json`: extra libraries plus "processors", Java tools that build
//!   the patched game jar from the vanilla client (deobfuscation, binary patches).
//!
//! We replay what the official installer does for the client side, without its UI:
//! download libraries, run the processors once, then launch the profile normally.

use std::collections::HashMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Deserialize;

use super::loader::{compare_versions, Loader, LoaderProfile, LoaderVersion};
use super::version::{maven_path, Library};
use crate::context::Context;
use crate::download::{self, fetch_bytes, DownloadTask};
use crate::error::{Error, Result};

const NEOFORGE_VERSIONS: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const FORGE_METADATA: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
const FORGE_PROMOTIONS: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";

/// Written next to the installer once every processor succeeded.
const PROCESSED_MARKER: &str = ".processed";
/// Lines of processor output kept in error messages.
const ERROR_TAIL_LINES: usize = 12;

// ---------------------------------------------------------------------------
// Versions
// ---------------------------------------------------------------------------

/// Loader versions for a game version, newest first. NeoForge marks pre-releases with
/// `-beta`; Forge publishes a "recommended" build per game version.
pub async fn list_versions(
    ctx: &Context,
    loader: Loader,
    game_version: &str,
) -> Result<Vec<LoaderVersion>> {
    let mut versions = match loader {
        Loader::NeoForge => {
            #[derive(Deserialize)]
            struct Listing {
                versions: Vec<String>,
            }
            let bytes = fetch_bytes(&ctx.http, NEOFORGE_VERSIONS, None).await?;
            let listing: Listing = serde_json::from_slice(&bytes)?;
            neoforge_versions(&listing.versions, game_version)
        }
        Loader::Forge => {
            let xml = fetch_bytes(&ctx.http, FORGE_METADATA, None).await?;
            let promos = fetch_bytes(&ctx.http, FORGE_PROMOTIONS, None).await?;
            forge_versions(&String::from_utf8_lossy(&xml), &promos, game_version)?
        }
        other => return Err(Error::LoaderNotSupported(other.to_string())),
    };
    versions.sort_by(|a, b| compare_versions(&b.version, &a.version));
    Ok(versions)
}

/// NeoForge versions encode the game version: `21.1.77` → 1.21.1, `21.0.3` → 1.21,
/// and since the year-based scheme `26.1.0.12` → 26.1, `26.1.1.4` → 26.1.1.
fn neoforge_game_version(version: &str) -> Option<String> {
    let core = version.split('-').next()?;
    let parts: Vec<&str> = core.split('.').collect();
    let major: u32 = parts.first()?.parse().ok()?;
    let minor: u32 = parts.get(1)?.parse().ok()?;
    if major == 0 {
        return None; // April fools snapshots (`0.25w14craftmine.3`).
    }
    if major < 25 {
        return Some(if minor == 0 {
            format!("1.{major}")
        } else {
            format!("1.{major}.{minor}")
        });
    }
    let patch: u32 = parts.get(2)?.parse().ok()?;
    Some(if patch == 0 {
        format!("{major}.{minor}")
    } else {
        format!("{major}.{minor}.{patch}")
    })
}

fn neoforge_versions(all: &[String], game_version: &str) -> Vec<LoaderVersion> {
    all.iter()
        .filter(|v| neoforge_game_version(v).as_deref() == Some(game_version))
        .map(|v| LoaderVersion {
            stable: !v.contains("beta") && !v.contains("alpha"),
            recommended: false,
            version: v.clone(),
        })
        .collect()
}

/// Forge versions are `<game>-<forge>` in the Maven metadata; old ones with an extra
/// suffix (`1.7.10-10.13.4.1614-1.7.10`) use another installer layout and are skipped.
fn forge_versions(xml: &str, promotions: &[u8], game_version: &str) -> Result<Vec<LoaderVersion>> {
    #[derive(Deserialize)]
    struct Promotions {
        promos: HashMap<String, String>,
    }
    let promos: Promotions = serde_json::from_slice(promotions)?;
    let recommended = promos.promos.get(&format!("{game_version}-recommended"));
    let prefix = format!("{game_version}-");
    Ok(xml
        .split("<version>")
        .skip(1)
        .filter_map(|chunk| chunk.split("</version>").next())
        .filter_map(|full| full.strip_prefix(&prefix))
        .filter(|forge| !forge.contains('-'))
        .map(|forge| LoaderVersion {
            stable: true,
            recommended: recommended.is_some_and(|r| r == forge),
            version: forge.to_owned(),
        })
        .collect())
}

fn installer_url(loader: Loader, game_version: &str, version: &str) -> Result<String> {
    match loader {
        Loader::NeoForge => Ok(format!(
            "{NEOFORGE_MAVEN}/{version}/neoforge-{version}-installer.jar"
        )),
        Loader::Forge => Ok(format!(
            "{FORGE_MAVEN}/{game_version}-{version}/forge-{game_version}-{version}-installer.jar"
        )),
        other => Err(Error::LoaderNotSupported(other.to_string())),
    }
}

// ---------------------------------------------------------------------------
// Installer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct InstallProfile {
    #[serde(default)]
    data: HashMap<String, SidedValue>,
    #[serde(default)]
    processors: Vec<Processor>,
    #[serde(default)]
    libraries: Vec<Library>,
}

#[derive(Debug, Clone, Deserialize)]
struct SidedValue {
    #[serde(default)]
    client: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Processor {
    jar: String,
    #[serde(default)]
    classpath: Vec<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    outputs: HashMap<String, String>,
    #[serde(default)]
    sides: Option<Vec<String>>,
}

impl Processor {
    fn runs_on_client(&self) -> bool {
        self.sides
            .as_ref()
            .is_none_or(|s| s.iter().any(|side| side == "client"))
    }
}

/// A downloaded installer, ready to provide its profile and run its processors.
#[derive(Debug, Clone)]
pub struct ForgeInstall {
    pub profile: LoaderProfile,
    /// Libraries needed by the processors (not on the game classpath).
    pub installer_libraries: Vec<Library>,
    installer: PathBuf,
    dir: PathBuf,
    install_profile: InstallProfile,
}

/// Downloads (once) and reads the installer for a Forge / NeoForge version, and
/// extracts the Maven artifacts it bundles into the libraries directory.
pub async fn load(
    ctx: &Context,
    loader: Loader,
    game_version: &str,
    version: &str,
) -> Result<ForgeInstall> {
    let dir = ctx
        .data
        .version_dir(&format!("{loader}-{game_version}-{version}"));
    let installer = dir.join("installer.jar");
    if !tokio::fs::try_exists(&installer).await.unwrap_or(false) {
        let url = installer_url(loader, game_version, version)?;
        // Both Mavens publish a `.sha1` next to every artifact.
        let sha1 = fetch_bytes(&ctx.http, &format!("{url}.sha1"), None)
            .await
            .ok()
            .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
            .filter(|s| s.len() == 40);
        download::download_all(
            &ctx.http,
            vec![DownloadTask {
                url,
                dest: installer.clone(),
                sha1,
                size: None,
            }],
            1,
            |_| {},
        )
        .await?;
    }

    let (path, libraries) = (installer.clone(), ctx.data.libraries());
    let (profile, install_profile) = blocking(move || {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&path)?)?;
        let profile: LoaderProfile =
            serde_json::from_str(&read_entry(&mut archive, "version.json")?)?;
        let install: InstallProfile =
            serde_json::from_str(&read_entry(&mut archive, "install_profile.json")?)?;
        extract_bundled_maven(&mut archive, &libraries)?;
        Ok((profile, install))
    })
    .await
    .map_err(|err| match err {
        Error::Zip(_) | Error::Json(_) => Error::InvalidInput(format!(
            "unsupported {loader} installer for {game_version}: {err}"
        )),
        other => other,
    })?;

    if profile.inherits_from != game_version {
        return Err(Error::InvalidInput(format!(
            "{loader} {version} targets Minecraft {}",
            profile.inherits_from
        )));
    }
    Ok(ForgeInstall {
        installer_libraries: install_profile.libraries.clone(),
        profile,
        installer,
        dir,
        install_profile,
    })
}

fn read_entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Result<String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|_| Error::InvalidInput(format!("{name} missing from installer")))?;
    let mut text = String::new();
    entry.read_to_string(&mut text)?;
    Ok(text)
}

/// Older installers ship some artifacts (the Forge jar itself) under `maven/`.
fn extract_bundled_maven(
    archive: &mut zip::ZipArchive<std::fs::File>,
    libraries: &Path,
) -> Result<()> {
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let Ok(relative) = name.strip_prefix("maven") else {
            continue;
        };
        let out = libraries.join(relative);
        if out.is_file() {
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        io::copy(&mut entry, &mut std::fs::File::create(&out)?)?;
    }
    Ok(())
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::Io(io::Error::other(e)))?
}

// ---------------------------------------------------------------------------
// Processors
// ---------------------------------------------------------------------------

/// Paths and values processors refer to as `{NAME}`.
struct Variables {
    values: HashMap<String, String>,
    libraries: PathBuf,
}

impl Variables {
    fn library(&self, coords: &str) -> Result<String> {
        let path = maven_path(coords)
            .ok_or_else(|| Error::InvalidInput(format!("bad Maven coordinates: {coords}")))?;
        Ok(self.libraries.join(path).display().to_string())
    }

    /// `[group:artifact:version]` is a library path; anything else may contain `{VAR}`s.
    fn resolve(&self, raw: &str) -> Result<String> {
        if let Some(coords) = raw.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            return self.library(coords);
        }
        let mut out = String::new();
        let mut rest = raw;
        while let Some(start) = rest.find('{') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            match after.find('}') {
                Some(end) => {
                    let name = &after[..end];
                    match self.values.get(name) {
                        Some(value) => out.push_str(value),
                        None => {
                            return Err(Error::InvalidInput(format!(
                                "unknown installer variable {{{name}}}"
                            )))
                        }
                    }
                    rest = &after[end + 1..];
                }
                None => {
                    out.push_str(&rest[start..]);
                    rest = "";
                }
            }
        }
        out.push_str(rest);
        Ok(out)
    }
}

/// Context the processors need from the vanilla install.
pub struct ProcessorEnv<'a> {
    pub java: &'a Path,
    pub client_jar: &'a Path,
    pub game_version: &'a str,
    pub libraries: &'a Path,
}

impl ForgeInstall {
    /// Whether the processors already ran for this version.
    pub async fn is_processed(&self) -> bool {
        tokio::fs::try_exists(self.dir.join(PROCESSED_MARKER))
            .await
            .unwrap_or(false)
    }

    /// Runs the client processors unless a previous run completed. Libraries (including
    /// [`ForgeInstall::installer_libraries`]) must already be downloaded.
    pub async fn run_processors(&self, env: &ProcessorEnv<'_>) -> Result<()> {
        if self.is_processed().await {
            return Ok(());
        }
        let marker = self.dir.join(PROCESSED_MARKER);
        let vars = self.variables(env).await?;
        let processors: Vec<&Processor> = self
            .install_profile
            .processors
            .iter()
            .filter(|p| p.runs_on_client())
            .collect();
        for (i, processor) in processors.iter().enumerate() {
            tracing::info!(step = i + 1, of = processors.len(), jar = %processor.jar, "running installer processor");
            run_processor(processor, &vars, env).await?;
        }
        tokio::fs::write(&marker, b"").await?;
        tracing::info!(profile = %self.profile.id, "installer processors done");
        Ok(())
    }

    async fn variables(&self, env: &ProcessorEnv<'_>) -> Result<Variables> {
        let data_dir = self.dir.join("data");
        let mut values = HashMap::from([
            ("SIDE".to_owned(), "client".to_owned()),
            (
                "MINECRAFT_JAR".to_owned(),
                env.client_jar.display().to_string(),
            ),
            ("MINECRAFT_VERSION".to_owned(), env.game_version.to_owned()),
            ("ROOT".to_owned(), self.dir.display().to_string()),
            ("INSTALLER".to_owned(), self.installer.display().to_string()),
            (
                "LIBRARY_DIR".to_owned(),
                env.libraries.display().to_string(),
            ),
        ]);
        let mut bundled = Vec::new();
        let partial = Variables {
            values: HashMap::new(),
            libraries: env.libraries.to_owned(),
        };
        for (name, value) in &self.install_profile.data {
            let raw = &value.client;
            let resolved = if raw.starts_with('[') {
                partial.resolve(raw)?
            } else if let Some(literal) = raw.strip_prefix('\'').and_then(|r| r.strip_suffix('\''))
            {
                literal.to_owned()
            } else if let Some(entry) = raw.strip_prefix('/') {
                // A file inside the installer (e.g. `/data/client.lzma`).
                let out = data_dir.join(entry);
                bundled.push((entry.to_owned(), out.clone()));
                out.display().to_string()
            } else {
                raw.clone()
            };
            values.insert(name.clone(), resolved);
        }

        let installer = self.installer.clone();
        blocking(move || {
            let mut archive = zip::ZipArchive::new(std::fs::File::open(&installer)?)?;
            for (entry, out) in bundled {
                let mut file = archive
                    .by_name(&entry)
                    .map_err(|_| Error::InvalidInput(format!("{entry} missing from installer")))?;
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                io::copy(&mut file, &mut std::fs::File::create(&out)?)?;
            }
            Ok(())
        })
        .await?;

        Ok(Variables {
            values,
            libraries: env.libraries.to_owned(),
        })
    }
}

/// `Main-Class` from a jar manifest.
fn main_class(jar: &Path) -> Result<String> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(jar)?)?;
    let manifest = read_entry(&mut archive, "META-INF/MANIFEST.MF")?;
    manifest
        .lines()
        .find_map(|line| line.strip_prefix("Main-Class:"))
        .map(|class| class.trim().to_owned())
        .ok_or_else(|| Error::InvalidInput(format!("no Main-Class in {}", jar.display())))
}

/// Lowercase hex SHA-1 of the files listed in `outputs` all match.
async fn outputs_match(outputs: &[(String, String)]) -> bool {
    if outputs.is_empty() {
        return false;
    }
    for (path, sha1) in outputs {
        match download::sha1_file(Path::new(path)).await {
            Ok(actual) if actual.eq_ignore_ascii_case(sha1) => {}
            _ => return false,
        }
    }
    true
}

async fn run_processor(
    processor: &Processor,
    vars: &Variables,
    env: &ProcessorEnv<'_>,
) -> Result<()> {
    let outputs: Vec<(String, String)> = processor
        .outputs
        .iter()
        .map(|(path, sha1)| {
            let sha1 = vars.resolve(sha1)?;
            Ok((vars.resolve(path)?, sha1.trim_matches('\'').to_owned()))
        })
        .collect::<Result<_>>()?;
    if outputs_match(&outputs).await {
        return Ok(());
    }

    let jar = PathBuf::from(vars.library(&processor.jar)?);
    let class = {
        let jar = jar.clone();
        blocking(move || main_class(&jar)).await?
    };
    let separator = if cfg!(windows) { ";" } else { ":" };
    let mut classpath = vec![jar.display().to_string()];
    for coords in &processor.classpath {
        classpath.push(vars.library(coords)?);
    }
    let args: Vec<String> = processor
        .args
        .iter()
        .map(|a| vars.resolve(a))
        .collect::<Result<_>>()?;

    let mut command = tokio::process::Command::new(env.java);
    command
        .arg("-cp")
        .arg(classpath.join(separator))
        .arg(&class)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command.output().await?;
    if !output.status.success() {
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let lines: Vec<&str> = text.lines().collect();
        let tail = lines[lines.len().saturating_sub(ERROR_TAIL_LINES)..].join("\n");
        tracing::error!(jar = %processor.jar, code = ?output.status.code(), output = %text, "processor failed");
        return Err(Error::ProcessorFailed {
            processor: processor.jar.clone(),
            output: tail,
        });
    }
    for (path, expected) in &outputs {
        let actual = download::sha1_file(Path::new(path)).await?;
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(Error::ChecksumMismatch {
                path: PathBuf::from(path),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_neoforge_versions_to_game_versions() {
        assert_eq!(neoforge_game_version("21.1.77").as_deref(), Some("1.21.1"));
        assert_eq!(
            neoforge_game_version("21.0.3-beta").as_deref(),
            Some("1.21")
        );
        assert_eq!(neoforge_game_version("20.2.88").as_deref(), Some("1.20.2"));
        assert_eq!(
            neoforge_game_version("26.1.0.12-beta").as_deref(),
            Some("26.1")
        );
        assert_eq!(neoforge_game_version("26.1.1.4").as_deref(), Some("26.1.1"));
        assert_eq!(neoforge_game_version("0.25w14craftmine.3-beta"), None);

        let all: Vec<String> = ["21.1.76", "21.1.77", "21.0.3", "21.1.78-beta"]
            .map(String::from)
            .to_vec();
        let found = neoforge_versions(&all, "1.21.1");
        assert_eq!(found.len(), 3);
        assert!(!found[2].stable);
    }

    #[test]
    fn reads_forge_metadata() {
        let xml = "<metadata><versioning><versions>\
            <version>1.20.1-47.4.10</version><version>1.20.1-47.4.9</version>\
            <version>1.20.10-99.0.0</version><version>1.7.10-10.13.4.1614-1.7.10</version>\
            </versions></versioning></metadata>";
        let promos = br#"{"promos":{"1.20.1-recommended":"47.4.9","1.20.1-latest":"47.4.10"}}"#;
        let versions = forge_versions(xml, promos, "1.20.1").unwrap();
        assert_eq!(versions.len(), 2);
        assert!(versions
            .iter()
            .any(|v| v.version == "47.4.9" && v.recommended));
        assert!(versions
            .iter()
            .any(|v| v.version == "47.4.10" && !v.recommended));
        assert!(forge_versions(xml, promos, "1.7.10").unwrap().is_empty());
    }

    #[test]
    fn resolves_installer_variables() {
        let vars = Variables {
            values: HashMap::from([
                ("SIDE".to_owned(), "client".to_owned()),
                ("ROOT".to_owned(), "R".to_owned()),
            ]),
            libraries: PathBuf::from("L"),
        };
        assert_eq!(vars.resolve("--side").unwrap(), "--side");
        assert_eq!(vars.resolve("{SIDE}").unwrap(), "client");
        assert_eq!(vars.resolve("{ROOT}/libraries/").unwrap(), "R/libraries/");
        assert_eq!(
            PathBuf::from(vars.resolve("[a.b:c:1:srg]").unwrap()),
            PathBuf::from("L").join("a/b/c/1/c-1-srg.jar")
        );
        assert_eq!(
            PathBuf::from(vars.resolve("[a.b:c:1@zip]").unwrap()),
            PathBuf::from("L").join("a/b/c/1/c-1.zip")
        );
        assert!(vars.resolve("{MISSING}").is_err());
    }

    #[test]
    fn filters_client_processors() {
        let server: Processor =
            serde_json::from_str(r#"{"jar":"a:b:1","sides":["server"]}"#).unwrap();
        let both: Processor = serde_json::from_str(r#"{"jar":"a:b:1"}"#).unwrap();
        assert!(!server.runs_on_client());
        assert!(both.runs_on_client());
    }
}
