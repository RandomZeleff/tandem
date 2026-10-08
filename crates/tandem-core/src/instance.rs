//! Instances: isolated game directories with their own version and settings.

use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub game_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub java_path: Option<String>,
    pub memory_mb: Option<u32>,
    pub jvm_args: Option<String>,
    pub icon: Option<String>,
    pub created_at: String,
    pub last_played_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewInstance {
    pub name: String,
    pub game_version: String,
}

const COLUMNS: &str = "id, name, game_version, loader, loader_version, java_path, memory_mb, \
                       jvm_args, icon, created_at, last_played_at";

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
                "instance name must be 1–64 characters".into(),
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
        sqlx::query("INSERT INTO instances (id, name, game_version) VALUES (?, ?, ?)")
            .bind(&id)
            .bind(name)
            .bind(&new.game_version)
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
        };
        let a = db.create_instance(&new).await.unwrap();
        let b = db.create_instance(&new).await.unwrap();
        assert_eq!(a.id, "survie");
        assert_eq!(b.id, "survie-2");
        assert_eq!(a.loader, "vanilla");

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
