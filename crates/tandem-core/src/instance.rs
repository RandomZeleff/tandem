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

/// Deletes an instance's row (and its content rows) and its whole folder.
pub async fn delete(ctx: &Context, id: &str) -> Result<()> {
    ctx.db.delete_instance(id).await?;
    match tokio::fs::remove_dir_all(ctx.data.instance_dir(id)).await {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err.into()),
        _ => {}
    }
    crate::screenshots::clear_thumbnails(&ctx.data, id);
    let _ = tokio::fs::remove_file(ctx.data.cache().join("deps").join(format!("{id}.json"))).await;
    tracing::info!(%id, "instance deleted");
    Ok(())
}

const COLUMNS: &str = "id, name, game_version, loader, loader_version, java_path, memory_mb, \
                       jvm_args, icon, created_at, last_played_at, pack_project_id, \
                       pack_version_id, pack_version";

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
