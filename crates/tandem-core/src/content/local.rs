//! Files in the content folders that Tandem does not track: mods added by hand, files
//! from CurseForge packs or other launchers that Modrinth does not know. They can still
//! be listed, switched off (same `.disabled` renaming) and removed.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{deps, mrpack, ContentKind, DISABLED_SUFFIX};
use crate::context::Context;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFile {
    pub kind: ContentKind,
    /// Name on disk without the `.disabled` suffix.
    pub file_name: String,
    /// The mod's own name when its jar says, else the file name without extension.
    pub name: String,
    pub enabled: bool,
    /// 0 for a folder (resource or shader pack left unzipped).
    pub size: u64,
}

const KINDS: [ContentKind; 3] = [
    ContentKind::Mod,
    ContentKind::ResourcePack,
    ContentKind::Shader,
];

fn accepted(kind: ContentKind, name: &str, is_dir: bool) -> bool {
    match kind {
        ContentKind::Mod => !is_dir && name.ends_with(".jar"),
        // Packs can be zips or plain folders.
        _ => is_dir || name.ends_with(".zip"),
    }
}

/// Untracked files of an instance, sorted by name.
pub async fn list(ctx: &Context, instance_id: &str) -> Result<Vec<LocalFile>> {
    let known: HashSet<(ContentKind, String)> = ctx
        .db
        .list_content(instance_id)
        .await?
        .into_iter()
        .map(|c| (c.kind, c.file_name))
        .collect();
    let (data, id, game_dir) = (
        ctx.data.clone(),
        instance_id.to_owned(),
        ctx.data.instance_dir(instance_id),
    );
    mrpack::blocking(move || {
        let mut files = Vec::new();
        for kind in KINDS {
            let Ok(entries) = std::fs::read_dir(game_dir.join(kind.folder())) else {
                continue;
            };
            for entry in entries.filter_map(|e| e.ok()) {
                let on_disk = entry.file_name().to_string_lossy().into_owned();
                let (file_name, enabled) = match on_disk.strip_suffix(DISABLED_SUFFIX) {
                    Some(name) => (name.to_owned(), false),
                    None => (on_disk.clone(), true),
                };
                let Ok(meta) = entry.metadata() else { continue };
                if !accepted(kind, &file_name, meta.is_dir())
                    || known.contains(&(kind, file_name.clone()))
                {
                    continue;
                }
                let name = Path::new(&file_name)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| file_name.clone());
                files.push(LocalFile {
                    kind,
                    name,
                    file_name,
                    enabled,
                    size: if meta.is_dir() { 0 } else { meta.len() },
                });
            }
        }
        // Mod names come from the jars, through the dependency scan and its cache.
        if files.iter().any(|f| f.kind == ContentKind::Mod) {
            let names: HashMap<String, String> = deps::scan_cached(&data, &id, &game_dir)
                .into_iter()
                .filter(|m| !m.name.trim().is_empty())
                .map(|m| {
                    let file = m
                        .file_name
                        .strip_suffix(DISABLED_SUFFIX)
                        .unwrap_or(&m.file_name)
                        .to_owned();
                    (file, m.name)
                })
                .collect();
            for file in files.iter_mut().filter(|f| f.kind == ContentKind::Mod) {
                if let Some(name) = names.get(&file.file_name) {
                    file.name = name.clone();
                }
            }
        }
        files.sort_by_key(|f| f.name.to_lowercase());
        Ok(files)
    })
    .await
}

/// Path of an untracked file, refusing names that would leave its folder.
fn path_of(
    ctx: &Context,
    instance_id: &str,
    kind: ContentKind,
    file_name: &str,
) -> Result<PathBuf> {
    if file_name.is_empty()
        || file_name.contains(['/', '\\', ':'])
        || file_name == "."
        || file_name == ".."
    {
        return Err(Error::InvalidInput(format!(
            "Nom de fichier invalide : {file_name}"
        )));
    }
    Ok(ctx
        .data
        .instance_dir(instance_id)
        .join(kind.folder())
        .join(file_name))
}

fn disabled(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(DISABLED_SUFFIX);
    PathBuf::from(name)
}

/// Switches an untracked file on or off by renaming it, like tracked content.
pub async fn set_enabled(
    ctx: &Context,
    instance_id: &str,
    kind: ContentKind,
    file_name: &str,
    enabled: bool,
) -> Result<()> {
    let on = path_of(ctx, instance_id, kind, file_name)?;
    let off = disabled(&on);
    let (from, to) = if enabled { (off, on) } else { (on, off) };
    if tokio::fs::try_exists(&from).await.unwrap_or(false) {
        tokio::fs::rename(&from, &to).await?;
        tracing::info!(instance = %instance_id, file = %file_name, enabled, "local file toggled");
    }
    Ok(())
}

/// Deletes an untracked file (or pack folder), whether enabled or not.
pub async fn remove(
    ctx: &Context,
    instance_id: &str,
    kind: ContentKind,
    file_name: &str,
) -> Result<()> {
    let on = path_of(ctx, instance_id, kind, file_name)?;
    for path in [disabled(&on), on] {
        let removed = match tokio::fs::metadata(&path).await {
            Ok(meta) if meta.is_dir() => tokio::fs::remove_dir_all(&path).await,
            Ok(_) => tokio::fs::remove_file(&path).await,
            Err(_) => continue,
        };
        removed?;
    }
    tracing::info!(instance = %instance_id, file = %file_name, "local file removed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests_support::add_row;
    use crate::instance::{self, NewInstance};
    use crate::paths::DataDir;

    #[tokio::test]
    async fn lists_toggles_and_removes_untracked_files() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Context::init(DataDir::new(dir.path())).await.unwrap();
        let inst = ctx
            .db
            .create_instance(&NewInstance {
                name: "T".into(),
                game_version: "1.21.1".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        let game = ctx.data.instance_dir(&inst.id);
        for (path, body) in [
            ("mods/tracked.jar", "x"),
            ("mods/hand-made.jar", "abc"),
            ("mods/off.jar.disabled", "y"),
            ("mods/notes.txt", "z"),
            ("resourcepacks/pack.zip", "zip"),
            ("resourcepacks/Folder Pack/pack.mcmeta", "{}"),
            ("shaderpacks/shader.zip", "s"),
        ] {
            let p = game.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        add_row(&ctx, &inst.id, "mods", "tracked.jar", "abc").await;

        let files = list(&ctx, &inst.id).await.unwrap();
        let names: Vec<(&str, bool)> = files
            .iter()
            .map(|f| (f.file_name.as_str(), f.enabled))
            .collect();
        assert_eq!(
            names,
            vec![
                ("Folder Pack", true),
                ("hand-made.jar", true),
                ("off.jar", false),
                ("pack.zip", true),
                ("shader.zip", true),
            ]
        );

        set_enabled(&ctx, &inst.id, ContentKind::Mod, "hand-made.jar", false)
            .await
            .unwrap();
        assert!(game.join("mods/hand-made.jar.disabled").is_file());
        set_enabled(&ctx, &inst.id, ContentKind::Mod, "off.jar", true)
            .await
            .unwrap();
        assert!(game.join("mods/off.jar").is_file());

        remove(&ctx, &inst.id, ContentKind::ResourcePack, "Folder Pack")
            .await
            .unwrap();
        remove(&ctx, &inst.id, ContentKind::Mod, "hand-made.jar")
            .await
            .unwrap();
        assert!(!game.join("resourcepacks/Folder Pack").exists());
        assert!(!game.join("mods/hand-made.jar.disabled").exists());
        assert!(remove(&ctx, &inst.id, ContentKind::Mod, "../x.jar")
            .await
            .is_err());
        let _ = instance::delete(&ctx, &inst.id).await;
    }
}
