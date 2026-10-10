//! Datapacks live in a world (`saves/<world>/datapacks/`), not in the instance. They are
//! listed from disk; those Modrinth knows are recognised by their SHA-1, so the launcher
//! can show their name and icon and offer updates.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use crate::content::modrinth;
use crate::context::Context;
use crate::download::{self, DownloadTask};
use crate::error::{Error, Result};
use crate::instance::Instance;
use crate::store;

/// Modrinth's loader name for datapack files.
const LOADER: &str = "datapack";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Datapack {
    /// Zip file or folder in `datapacks/`.
    pub file_name: String,
    /// Project title when Modrinth knows the file, else the file name.
    pub title: String,
    /// `pack.mcmeta` description.
    pub description: Option<String>,
    pub project_id: Option<String>,
    pub version_number: Option<String>,
    pub icon_url: Option<String>,
    pub size_bytes: u64,
}

fn folder(game_dir: &Path, world: &str) -> Result<PathBuf> {
    let mut components = Path::new(world).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => {
            Ok(game_dir.join("saves").join(world).join("datapacks"))
        }
        _ => Err(Error::InvalidInput(format!(
            "Nom de monde invalide : {world}"
        ))),
    }
}

fn plain_name(file_name: &str) -> Result<()> {
    let mut components = Path::new(file_name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => Err(Error::InvalidInput(format!(
            "Nom de fichier invalide : {file_name}"
        ))),
    }
}

/// Description from `pack.mcmeta` (plain string or the text of a JSON component).
fn description(mcmeta: &str) -> Option<String> {
    let value: serde_json::Value =
        serde_json::from_str(mcmeta.trim_start_matches('\u{feff}')).ok()?;
    let description = value.pointer("/pack/description")?;
    let text = match description {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(o) => o.get("text")?.as_str()?.to_owned(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .filter_map(|p| {
                p.as_str()
                    .map(str::to_owned)
                    .or_else(|| p.get("text")?.as_str().map(str::to_owned))
            })
            .collect::<String>(),
        _ => return None,
    };
    // Formatting codes are noise in a list.
    let mut clean = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            clean.push(c);
        }
    }
    let clean = clean.trim().to_owned();
    (!clean.is_empty()).then_some(clean)
}

struct Local {
    file_name: String,
    sha1: Option<String>,
    description: Option<String>,
    size_bytes: u64,
}

fn read_local(dir: &Path) -> Vec<Local> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<Local> = entries
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                let mcmeta = std::fs::read_to_string(path.join("pack.mcmeta")).ok()?;
                return Some(Local {
                    file_name,
                    sha1: None,
                    description: description(&mcmeta),
                    size_bytes: 0,
                });
            }
            if !file_name.ends_with(".zip") {
                return None;
            }
            let bytes = std::fs::read(&path).ok()?;
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).ok()?;
            let mut mcmeta = String::new();
            archive
                .by_name("pack.mcmeta")
                .ok()?
                .read_to_string(&mut mcmeta)
                .ok()?;
            Some(Local {
                file_name,
                sha1: Some(sha1_smol::Sha1::from(&bytes).digest().to_string()),
                description: description(&mcmeta),
                size_bytes: bytes.len() as u64,
            })
        })
        .collect();
    found.sort_by_key(|l| l.file_name.to_lowercase());
    found
}

/// Datapacks of a world, with what Modrinth knows about them.
pub async fn list(ctx: &Context, instance_id: &str, world: &str) -> Result<Vec<Datapack>> {
    let dir = folder(&ctx.data.instance_dir(instance_id), world)?;
    let local = tokio::task::spawn_blocking(move || read_local(&dir))
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;
    let hashes: Vec<String> = local.iter().filter_map(|l| l.sha1.clone()).collect();
    // Offline, the list still shows with file names.
    let versions = modrinth::versions_by_hash(ctx, &hashes)
        .await
        .unwrap_or_default();
    let ids: Vec<String> = versions.values().map(|v| v.project_id.clone()).collect();
    let projects: HashMap<String, modrinth::Project> = modrinth::projects(ctx, &ids)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();
    Ok(local
        .into_iter()
        .map(|l| {
            let version = l.sha1.as_ref().and_then(|s| versions.get(s));
            let project = version.and_then(|v| projects.get(&v.project_id));
            Datapack {
                title: project.map_or_else(
                    || l.file_name.trim_end_matches(".zip").to_owned(),
                    |p| p.title.clone(),
                ),
                description: l.description,
                project_id: project.map(|p| p.id.clone()),
                version_number: version.map(|v| v.version_number.clone()),
                icon_url: project.and_then(|p| p.icon_url.clone()),
                size_bytes: l.size_bytes,
                file_name: l.file_name,
            }
        })
        .collect())
}

/// Installs the datapack version of a Modrinth project that fits the instance into a
/// world. Replaces an older file of the same project.
pub async fn install(
    ctx: &Context,
    instance: &Instance,
    world: &str,
    project_id: &str,
) -> Result<Datapack> {
    let game_dir = ctx.data.instance_dir(&instance.id);
    let dir = folder(&game_dir, world)?;
    if !dir.parent().is_some_and(|w| w.join("level.dat").is_file()) {
        return Err(Error::InvalidInput(format!("Monde introuvable : {world}")));
    }
    let project = modrinth::project(ctx, project_id).await?;
    let versions =
        modrinth::project_versions(ctx, &project.id, &instance.game_version, &[LOADER]).await?;
    let release = versions.iter().position(|v| v.version_type == "release");
    let version = versions
        .into_iter()
        .nth(release.unwrap_or(0))
        .ok_or_else(|| Error::ContentUnavailable {
            title: project.title.clone(),
            game_version: instance.game_version.clone(),
        })?;
    let file = version
        .primary_file()
        .ok_or_else(|| {
            Error::InvalidInput(format!(
                "{} n'a pas de fichier à télécharger",
                project.title
            ))
        })?
        .clone();
    plain_name(&file.filename)?;
    let stored = store::path(&ctx.data, &file.hashes.sha1);
    download::download_all(
        &ctx.http,
        vec![DownloadTask {
            url: file.url.clone(),
            dest: stored.clone(),
            sha1: Some(file.hashes.sha1.clone()),
            size: Some(file.size),
        }],
        1,
        |_| {},
    )
    .await?;

    // An older version of the same project goes away.
    for previous in list(ctx, &instance.id, world).await? {
        if previous.project_id.as_deref() == Some(project.id.as_str())
            && previous.file_name != file.filename
        {
            remove(ctx, &instance.id, world, &previous.file_name).await?;
        }
    }
    // Copied, not hardlinked: the game may rewrite files in a world.
    tokio::fs::create_dir_all(&dir).await?;
    tokio::fs::copy(&stored, dir.join(&file.filename)).await?;
    tracing::info!(instance = %instance.id, world, project = %project.title, version = %version.version_number, "datapack installed");
    Ok(Datapack {
        file_name: file.filename,
        title: project.title,
        description: None,
        project_id: Some(project.id),
        version_number: Some(version.version_number),
        icon_url: project.icon_url,
        size_bytes: file.size,
    })
}

pub async fn remove(ctx: &Context, instance_id: &str, world: &str, file_name: &str) -> Result<()> {
    plain_name(file_name)?;
    let path = folder(&ctx.data.instance_dir(instance_id), world)?.join(file_name);
    if path.is_dir() {
        tokio::fs::remove_dir_all(&path).await?;
    } else {
        tokio::fs::remove_file(&path).await?;
    }
    tracing::info!(instance = %instance_id, world, file_name, "datapack removed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_descriptions() {
        assert_eq!(
            description(r#"{"pack": {"pack_format": 48, "description": "§aTerralith §7v2"}}"#)
                .as_deref(),
            Some("Terralith v2")
        );
        assert_eq!(
            description(r#"{"pack": {"description": {"text": "Hi"}}}"#).as_deref(),
            Some("Hi")
        );
        assert_eq!(
            description(r#"{"pack": {"description": ["A", {"text": "B"}]}}"#).as_deref(),
            Some("AB")
        );
        assert_eq!(description(r#"{"pack": {}}"#), None);
    }

    #[test]
    fn lists_zip_and_folder_packs() {
        let dir = tempfile::tempdir().unwrap();
        let packs = dir.path();
        std::fs::create_dir_all(packs.join("folder_pack")).unwrap();
        std::fs::write(
            packs.join("folder_pack/pack.mcmeta"),
            r#"{"pack": {"description": "Folder"}}"#,
        )
        .unwrap();
        let mut writer = zip::ZipWriter::new(std::fs::File::create(packs.join("b.zip")).unwrap());
        writer
            .start_file("pack.mcmeta", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, br#"{"pack": {"description": "Zipped"}}"#).unwrap();
        writer.finish().unwrap();
        std::fs::write(packs.join("notes.txt"), "x").unwrap();
        let found = read_local(packs);
        let names: Vec<&str> = found.iter().map(|l| l.file_name.as_str()).collect();
        assert_eq!(names, ["b.zip", "folder_pack"]);
        assert!(found[0].sha1.is_some());
        assert_eq!(found[1].description.as_deref(), Some("Folder"));
    }

    #[test]
    fn refuses_odd_names() {
        assert!(folder(Path::new("g"), "../x").is_err());
        assert!(plain_name("a/b.zip").is_err());
        assert!(plain_name("ok.zip").is_ok());
    }
}
