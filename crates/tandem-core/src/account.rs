//! Player accounts. Only metadata is stored here; tokens belong in the OS keyring (D9).

use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum AccountKind {
    Microsoft,
    Offline,
}

impl AccountKind {
    /// Value of the `${user_type}` launch placeholder.
    pub fn user_type(self) -> &'static str {
        match self {
            AccountKind::Microsoft => "msa",
            AccountKind::Offline => "legacy",
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub kind: AccountKind,
    pub username: String,
    pub mc_uuid: String,
    pub is_active: bool,
}

/// UUID the vanilla server assigns to an offline player
/// (Java's `UUID.nameUUIDFromBytes("OfflinePlayer:" + name)`).
pub fn offline_uuid(username: &str) -> uuid::Uuid {
    let digest = md5::compute(format!("OfflinePlayer:{username}"));
    uuid::Builder::from_md5_bytes(digest.0).into_uuid()
}

fn validate_username(username: &str) -> Result<()> {
    let valid_len = (3..=16).contains(&username.len());
    let valid_chars = username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_');
    if valid_len && valid_chars {
        Ok(())
    } else {
        Err(Error::InvalidInput(
            "Le pseudo doit faire 3 à 16 caractères : lettres, chiffres ou _".into(),
        ))
    }
}

const COLUMNS: &str = "id, kind, username, mc_uuid, is_active";

impl Database {
    pub async fn list_accounts(&self) -> Result<Vec<Account>> {
        Ok(sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM accounts ORDER BY created_at"
        ))
        .fetch_all(self.pool())
        .await?)
    }

    pub async fn active_account(&self) -> Result<Option<Account>> {
        Ok(sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM accounts WHERE is_active = 1"
        ))
        .fetch_optional(self.pool())
        .await?)
    }

    /// Adds an offline account (or returns the existing one with that name) and activates it.
    pub async fn add_offline_account(&self, username: &str) -> Result<Account> {
        let username = username.trim();
        validate_username(username)?;
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT id FROM accounts WHERE kind = 'offline' AND username = ? COLLATE NOCASE",
        )
        .bind(username)
        .fetch_optional(self.pool())
        .await?;
        let id = match existing {
            Some(id) => id,
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO accounts (id, kind, username, mc_uuid) VALUES (?, 'offline', ?, ?)",
                )
                .bind(&id)
                .bind(username)
                .bind(offline_uuid(username).to_string())
                .execute(self.pool())
                .await?;
                id
            }
        };
        self.set_active_account(&id).await
    }

    pub async fn set_active_account(&self, id: &str) -> Result<Account> {
        let mut tx = self.pool().begin().await?;
        sqlx::query("UPDATE accounts SET is_active = 0 WHERE is_active = 1")
            .execute(&mut *tx)
            .await?;
        let updated = sqlx::query(
            "UPDATE accounts SET is_active = 1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(Error::AccountNotFound(id.to_owned()));
        }
        tx.commit().await?;
        self.active_account()
            .await?
            .ok_or_else(|| Error::AccountNotFound(id.to_owned()))
    }

    pub async fn remove_account(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM accounts WHERE id = ?")
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
    fn offline_uuid_matches_vanilla() {
        assert_eq!(
            offline_uuid("Notch").to_string(),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
    }

    #[tokio::test]
    async fn offline_accounts_single_active() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Database::open(&tmp.path().join("t.db")).await.unwrap();

        let steve = db.add_offline_account("Steve").await.unwrap();
        let alex = db.add_offline_account("Alex").await.unwrap();
        assert!(alex.is_active);
        assert_eq!(db.active_account().await.unwrap().unwrap().id, alex.id);

        // Re-adding an existing name reuses it.
        let again = db.add_offline_account("steve").await.unwrap();
        assert_eq!(again.id, steve.id);
        assert_eq!(db.list_accounts().await.unwrap().len(), 2);

        assert!(db.add_offline_account("x").await.is_err());
        assert!(db.add_offline_account("bad name!").await.is_err());
    }
}
