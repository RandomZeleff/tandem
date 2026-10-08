//! SQLite storage: connection pool, migrations and settings.

use std::path::Path;

use serde::{de::DeserializeOwned, Serialize};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Opens (creating if needed) the database and applies pending migrations.
    pub async fn open(path: &Path) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        tracing::debug!(path = %path.display(), "database ready");
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn get_setting<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let raw: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        raw.map(|v| serde_json::from_str(&v).map_err(Into::into))
            .transpose()
    }

    pub async fn set_setting<T: Serialize + ?Sized>(&self, key: &str, value: &T) -> Result<()> {
        let raw = serde_json::to_string(value)?;
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(raw)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn temp_db() -> (tempfile::TempDir, Database) {
        let tmp = tempfile::tempdir().unwrap();
        let db = Database::open(&tmp.path().join("test.db")).await.unwrap();
        (tmp, db)
    }

    #[tokio::test]
    async fn migrations_create_tables() {
        let (_tmp, db) = temp_db().await;
        let tables: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
                .fetch_all(db.pool())
                .await
                .unwrap();
        for t in ["accounts", "instances", "settings"] {
            assert!(tables.iter().any(|n| n == t), "table {t} missing");
        }
    }

    #[tokio::test]
    async fn settings_roundtrip_and_overwrite() {
        let (_tmp, db) = temp_db().await;
        assert_eq!(db.get_setting::<u32>("memory_mb").await.unwrap(), None);
        db.set_setting("memory_mb", &4096u32).await.unwrap();
        db.set_setting("memory_mb", &6144u32).await.unwrap();
        assert_eq!(
            db.get_setting::<u32>("memory_mb").await.unwrap(),
            Some(6144)
        );
    }

    #[tokio::test]
    async fn reopen_keeps_data() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.db");
        Database::open(&path)
            .await
            .unwrap()
            .set_setting("lang", "fr")
            .await
            .unwrap();
        let db = Database::open(&path).await.unwrap();
        assert_eq!(
            db.get_setting::<String>("lang").await.unwrap().as_deref(),
            Some("fr")
        );
    }
}
