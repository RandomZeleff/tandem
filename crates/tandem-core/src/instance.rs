//! Instances: isolated game directories with their own version and settings.

use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::db::Database;
use crate::error::{Error, Result};
use crate::meta::loader::{self, Loader};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub game_version: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    pub java_path: Option<String>,
    pub memory_mb: Option<u32>,
    pub jvm_args: Option<String>,
    /// Image URL (e.g. the modpack's icon); the UI draws a block when absent.
    pub icon: Option<String>,
    pub created_at: String,
    pub last_played_at: Option<String>,
    pub pack_project_id: Option<String>,
    pub pack_version_id: Option<String>,
    pub pack_version: Option<String>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    /// Block drawn as the icon when there is no image; `None` picks one from the id.
    pub block: Option<u32>,
}

/// Settings the player edits on an instance.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSettings {
    pub name: String,
    /// `None`: the Java Tandem picks for the game version.
    pub java_path: Option<String>,
    pub jvm_args: Option<String>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
}

/// Modrinth modpack an instance comes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackOrigin {
    pub project_id: String,
    pub version_id: String,
    pub version: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewInstance {
    pub name: String,
    pub game_version: String,
    #[serde(default)]
    pub loader: Loader,
    /// Latest stable loader version when omitted.
    #[serde(default)]
    pub loader_version: Option<String>,
}

/// Creates an instance and its folder, pinning the loader version so it never
/// changes under the player's feet.
pub async fn create(ctx: &Context, mut new: NewInstance) -> Result<Instance> {
    new.loader_version = match new.loader {
        Loader::Vanilla => None,
        loader => {
            let available = loader::list_versions(ctx, loader, &new.game_version).await?;
            let chosen = match new.loader_version.take() {
                Some(v) => available.into_iter().find(|a| a.version == v),
                None => available
                    .iter()
                    .find(|a| a.recommended)
                    .or_else(|| available.iter().find(|a| a.stable))
                    .or(available.first())
                    .cloned(),
            };
            let chosen = chosen.ok_or_else(|| Error::LoaderUnavailable {
                loader: loader.to_string(),
                game_version: new.game_version.clone(),
            })?;
            Some(chosen.version)
        }
    };
    let created = ctx.db.create_instance(&new).await?;
    tokio::fs::create_dir_all(ctx.data.instance_dir(&created.id)).await?;
    tracing::info!(
        id = %created.id,
        version = %created.game_version,
        loader = %created.loader,
        loader_version = ?created.loader_version,
        "instance created"
    );
    Ok(created)
}

/// Saves the settings the player edits (name, Java, JVM arguments, window size).
pub async fn update_settings(
    ctx: &Context,
    id: &str,
    settings: &InstanceSettings,
) -> Result<Instance> {
    let name = settings.name.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(Error::InvalidInput(
            "Le nom de l'instance doit faire 1 à 64 caractères".into(),
        ));
    }
    let java = settings
        .java_path
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty());
    if let Some(java) = java {
        if !std::path::Path::new(java).is_file() {
            return Err(Error::InvalidInput(format!("Java introuvable : {java}")));
        }
    }
    let args = settings
        .jvm_args
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty());
    if let Some(args) = args {
        if args
            .split_whitespace()
            .any(|a| a.starts_with("-Xmx") || a.starts_with("-Xms"))
        {
            return Err(Error::InvalidInput(
                "La mémoire se règle avec le curseur, pas dans les arguments (-Xmx, -Xms)".into(),
            ));
        }
    }
    let window = match (settings.window_width, settings.window_height) {
        (Some(w), Some(h)) if (320..=7680).contains(&w) && (240..=4320).contains(&h) => {
            (Some(w), Some(h))
        }
        (None, None) => (None, None),
        _ => return Err(Error::InvalidInput("Taille de fenêtre invalide".into())),
    };
    sqlx::query(
        "UPDATE instances SET name = ?, java_path = ?, jvm_args = ?, window_width = ?, window_height = ? WHERE id = ?",
    )
    .bind(name)
    .bind(java)
    .bind(args)
    .bind(window.0)
    .bind(window.1)
    .bind(id)
    .execute(ctx.db.pool())
    .await?;
    ctx.db.get_instance(id).await
}

/// Icon side length stored for custom images.
const ICON_SIZE: u32 = 128;

/// Sets the instance's icon from an image file, or back to its block (`None`).
/// `block` picks which block is drawn when there is no image.
pub async fn set_icon(
    ctx: &Context,
    id: &str,
    image: Option<&std::path::Path>,
    block: Option<u32>,
) -> Result<Instance> {
    let icon = match image {
        Some(path) => {
            let path = path.to_owned();
            Some(
                tokio::task::spawn_blocking(move || -> Result<String> {
                    use base64::Engine;
                    let img = image::open(&path)?;
                    let side = img.width().min(img.height());
                    let square = img.crop_imm(
                        (img.width() - side) / 2,
                        (img.height() - side) / 2,
                        side,
                        side,
                    );
                    // Pixel art stays sharp; photos are smoothed.
                    let filter = if side <= ICON_SIZE {
                        image::imageops::FilterType::Nearest
                    } else {
                        image::imageops::FilterType::Lanczos3
                    };
                    let resized = square.resize_exact(ICON_SIZE, ICON_SIZE, filter);
                    let mut png = Vec::new();
                    resized
                        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
                    Ok(format!(
                        "data:image/png;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(png)
                    ))
                })
                .await
                .map_err(|e| Error::Io(std::io::Error::other(e)))??,
            )
        }
        None => None,
    };
    sqlx::query("UPDATE instances SET icon = ?, block = ? WHERE id = ?")
        .bind(icon)
        .bind(block)
        .bind(id)
        .execute(ctx.db.pool())
        .await?;
    ctx.db.get_instance(id).await
}

/// Folders of an instance a copy leaves out: generated or specific to one run.
const NOT_COPIED: &[&str] = &[
    "logs",
    "crash-reports",
    ".tandem/rollback",
    "natives",
    ".cache",
];

/// Copies an instance under a new name: settings, content rows and files. Mods and other
/// files from the shared store are hardlinked (no extra disk space), the rest copied.
pub async fn duplicate(ctx: &Context, id: &str, name: &str) -> Result<Instance> {
    let source = ctx.db.get_instance(id).await?;
    let created = ctx
        .db
        .create_instance(&NewInstance {
            name: name.to_owned(),
            game_version: source.game_version.clone(),
            loader: source.loader,
            loader_version: source.loader_version.clone(),
        })
        .await?;
    sqlx::query(
        "UPDATE instances SET java_path = ?, memory_mb = ?, jvm_args = ?, icon = ?, pack_project_id = ?,
         pack_version_id = ?, pack_version = ?, window_width = ?, window_height = ?, block = ? WHERE id = ?",
    )
    .bind(&source.java_path)
    .bind(source.memory_mb)
    .bind(&source.jvm_args)
    .bind(&source.icon)
    .bind(&source.pack_project_id)
    .bind(&source.pack_version_id)
    .bind(&source.pack_version)
    .bind(source.window_width)
    .bind(source.window_height)
    .bind(source.block)
    .bind(&created.id)
    .execute(ctx.db.pool())
    .await?;

    let content = ctx.db.list_content(id).await?;
    let stored: std::collections::HashMap<String, String> = content
        .iter()
        .map(|c| {
            (
                format!("{}/{}", c.kind.folder(), c.file_name),
                c.sha1.clone(),
            )
        })
        .collect();
    let (data, from, to) = (
        ctx.data.clone(),
        ctx.data.instance_dir(id),
        ctx.data.instance_dir(&created.id),
    );
    let copy = tokio::task::spawn_blocking(move || copy_tree(&data, &from, &to, &from, &stored))
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;
    if let Err(err) = copy {
        let _ = delete(ctx, &created.id).await;
        return Err(err);
    }
    crate::content::copy_content(&ctx.db, id, &created.id).await?;
    tracing::info!(from = %id, to = %created.id, "instance duplicated");
    ctx.db.get_instance(&created.id).await
}

fn copy_tree(
    data: &crate::paths::DataDir,
    root: &std::path::Path,
    dest_root: &std::path::Path,
    dir: &std::path::Path,
    stored: &std::collections::HashMap<String, String>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if NOT_COPIED.contains(&rel.as_str()) {
            continue;
        }
        let dest = dest_root.join(&rel);
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&dest)?;
            copy_tree(data, root, dest_root, &path, stored)?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let key = rel.trim_end_matches(".disabled");
        let from_store = stored
            .get(key)
            .map(|sha| crate::store::path(data, sha))
            .filter(|p| p.is_file());
        match from_store {
            Some(stored_file) => crate::store::link_blocking(&stored_file, &dest)?,
            None => {
                std::fs::copy(&path, &dest)?;
            }
        }
    }
    Ok(())
}

/// Deletes an instance's row (and its content rows) and its whole folder.
pub async fn delete(ctx: &Context, id: &str) -> Result<()> {
    ctx.db.delete_instance(id).await?;
    match tokio::fs::remove_dir_all(ctx.data.instance_dir(id)).await {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err.into()),
        _ => {}
    }
    crate::screenshots::clear_thumbnails(&ctx.data, id);
    let _ = sqlx::query("DELETE FROM settings WHERE key LIKE ?")
        .bind(format!("translation.excluded.{id}"))
        .execute(ctx.db.pool())
        .await;
    let _ = tokio::fs::remove_file(ctx.data.cache().join("deps").join(format!("{id}.json"))).await;
    tracing::info!(%id, "instance deleted");
    Ok(())
}

const COLUMNS: &str = "id, name, game_version, loader, loader_version, java_path, memory_mb, \
                       jvm_args, icon, created_at, last_played_at, pack_project_id, \
                       pack_version_id, pack_version, window_width, window_height, block";

/// Folder-friendly id derived from the name: `My World!` → `my-world`.
fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        "instance".to_owned()
    } else {
        slug.chars().take(48).collect()
    }
}

impl Database {
    pub async fn list_instances(&self) -> Result<Vec<Instance>> {
        Ok(sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM instances
             ORDER BY last_played_at IS NULL, last_played_at DESC, created_at DESC"
        ))
        .fetch_all(self.pool())
        .await?)
    }

    pub async fn get_instance(&self, id: &str) -> Result<Instance> {
        sqlx::query_as(&format!("SELECT {COLUMNS} FROM instances WHERE id = ?"))
            .bind(id)
            .fetch_optional(self.pool())
            .await?
            .ok_or_else(|| Error::InstanceNotFound(id.to_owned()))
    }

    pub async fn create_instance(&self, new: &NewInstance) -> Result<Instance> {
        let name = new.name.trim();
        if name.is_empty() || name.chars().count() > 64 {
            return Err(Error::InvalidInput(
                "Le nom de l'instance doit faire 1 à 64 caractères".into(),
            ));
        }
        let base = slugify(name);
        let mut id = base.clone();
        let mut n = 2;
        while sqlx::query_scalar::<_, i64>("SELECT 1 FROM instances WHERE id = ?")
            .bind(&id)
            .fetch_optional(self.pool())
            .await?
            .is_some()
        {
            id = format!("{base}-{n}");
            n += 1;
        }
        sqlx::query(
            "INSERT INTO instances (id, name, game_version, loader, loader_version)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(name)
        .bind(&new.game_version)
        .bind(new.loader)
        .bind(&new.loader_version)
        .execute(self.pool())
        .await?;
        self.get_instance(&id).await
    }

    pub async fn delete_instance(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM instances WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn set_instance_pack(&self, id: &str, origin: &PackOrigin) -> Result<()> {
        sqlx::query(
            "UPDATE instances SET pack_project_id = ?, pack_version_id = ?, pack_version = ?,
             icon = COALESCE(?, icon) WHERE id = ?",
        )
        .bind(&origin.project_id)
        .bind(&origin.version_id)
        .bind(&origin.version)
        .bind(&origin.icon)
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Game version and loader, when a modpack update changes them.
    pub async fn set_instance_target(
        &self,
        id: &str,
        game_version: &str,
        loader: crate::meta::loader::Loader,
        loader_version: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE instances SET game_version = ?, loader = ?, loader_version = ? WHERE id = ?",
        )
        .bind(game_version)
        .bind(loader)
        .bind(loader_version)
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// `None` goes back to automatic memory.
    pub async fn set_instance_memory(&self, id: &str, memory_mb: Option<u32>) -> Result<()> {
        sqlx::query("UPDATE instances SET memory_mb = ? WHERE id = ?")
            .bind(memory_mb)
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn mark_instance_played(&self, id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE instances SET last_played_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
        )
        .bind(id)
        .execute(self.pool())
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn duplicates_with_store_links_and_independent_files() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Context::init(crate::paths::DataDir::new(dir.path()))
            .await
            .unwrap();
        let source = create(
            &ctx,
            NewInstance {
                name: "Original".into(),
                game_version: "1.20.1".into(),
                loader: Loader::Vanilla,
                loader_version: None,
            },
        )
        .await
        .unwrap();
        let game = ctx.data.instance_dir(&source.id);
        let stored = crate::store::path(&ctx.data, "abcd1234");
        std::fs::create_dir_all(stored.parent().unwrap()).unwrap();
        std::fs::write(&stored, "jar").unwrap();
        crate::store::link_blocking(&stored, &game.join("resourcepacks/pack.zip")).unwrap();
        std::fs::create_dir_all(game.join("config")).unwrap();
        std::fs::write(game.join("config/a.toml"), "x").unwrap();
        std::fs::create_dir_all(game.join("logs")).unwrap();
        std::fs::write(game.join("logs/latest.log"), "log").unwrap();
        crate::content::tests_support::add_row(
            &ctx,
            &source.id,
            "resourcepacks",
            "pack.zip",
            "abcd1234",
        )
        .await;

        let copy = duplicate(&ctx, &source.id, "Copie").await.unwrap();
        let copy_dir = ctx.data.instance_dir(&copy.id);
        assert_eq!(copy.name, "Copie");
        assert_eq!(
            std::fs::read_to_string(copy_dir.join("resourcepacks/pack.zip")).unwrap(),
            "jar"
        );
        assert!(!copy_dir.join("logs").exists());
        std::fs::write(copy_dir.join("config/a.toml"), "changed").unwrap();
        assert_eq!(
            std::fs::read_to_string(game.join("config/a.toml")).unwrap(),
            "x"
        );
        assert_eq!(ctx.db.list_content(&copy.id).await.unwrap().len(), 1);
    }

    #[test]
    fn slugs() {
        assert_eq!(slugify("My World!"), "my-world");
        assert_eq!(slugify("  Créa  1.21 "), "créa-1-21");
        assert_eq!(slugify("!!!"), "instance");
    }

    #[tokio::test]
    async fn create_list_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Database::open(&tmp.path().join("t.db")).await.unwrap();
        let new = NewInstance {
            name: "Survie".into(),
            game_version: "1.21.4".into(),
            ..Default::default()
        };
        let a = db.create_instance(&new).await.unwrap();
        let b = db
            .create_instance(&NewInstance {
                loader: Loader::Fabric,
                loader_version: Some("0.16.10".into()),
                ..new.clone()
            })
            .await
            .unwrap();
        assert_eq!(a.id, "survie");
        assert_eq!(b.id, "survie-2");
        assert_eq!(a.loader, Loader::Vanilla);
        assert_eq!(b.loader, Loader::Fabric);
        assert_eq!(b.loader_version.as_deref(), Some("0.16.10"));

        db.set_instance_memory(&a.id, Some(6144)).await.unwrap();
        assert_eq!(db.get_instance(&a.id).await.unwrap().memory_mb, Some(6144));
        db.set_instance_memory(&a.id, None).await.unwrap();
        assert_eq!(db.get_instance(&a.id).await.unwrap().memory_mb, None);

        db.mark_instance_played(&b.id).await.unwrap();
        let list = db.list_instances().await.unwrap();
        assert_eq!(list[0].id, "survie-2");

        db.delete_instance(&a.id).await.unwrap();
        assert!(matches!(
            db.get_instance(&a.id).await,
            Err(Error::InstanceNotFound(_))
        ));
    }
}
