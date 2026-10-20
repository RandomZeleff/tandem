//! Friends without a server of ours (D59). Each Tandem has a lasting iroh key: its
//! endpoint id is its address. A short friend code points at it through iroh's public
//! pkarr relay, like invite codes. Once two players are friends, each keeps the other's
//! endpoint id and their launchers talk directly: who is online, what they play, invites.

pub mod protocol;
pub mod service;

use iroh::{EndpointId, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db::Database;
use crate::duo::invite;
use crate::error::{Error, Result};
use crate::paths::DataDir;
use crate::secrets;

/// iroh application protocol between friends' launchers.
pub const ALPN: &[u8] = b"tandem/friends/1";

/// Setting holding this launcher's friend code (kept for good once made).
const CODE_SETTING: &str = "friend_code";
/// Setting: `true` hides what the player does from friends ("appear offline").
pub const HIDDEN_SETTING: &str = "friends_hidden";
/// Setting: friends may join a world the player hosts without being invited.
pub const OPEN_TO_FRIENDS_SETTING: &str = "friends_can_join";
/// How long the relay keeps the code's record; it is published again well before.
const CODE_TTL_SECS: u32 = 2 * 60 * 60;

pub(crate) fn fail(err: impl std::fmt::Display) -> Error {
    Error::Friends(err.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum FriendState {
    /// We asked, they have not answered yet.
    Outgoing,
    /// They asked, we have not answered yet.
    Incoming,
    Friend,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Friend {
    /// Their endpoint id.
    pub id: String,
    /// Their Minecraft name ("…" until their launcher was reached once).
    pub name: String,
    pub mc_uuid: String,
    /// Face of their skin (PNG data URL).
    pub avatar: Option<String>,
    pub state: FriendState,
    pub added_at: i64,
}

impl Friend {
    pub fn endpoint(&self) -> Option<EndpointId> {
        self.id.parse().ok()
    }
}

/// What a friend is doing, as they share it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Presence {
    /// In the launcher.
    Online,
    /// In game.
    #[serde(rename_all = "camelCase")]
    Playing { instance: String },
    /// Hosting a world; `code` only when friends may join without an invite.
    #[serde(rename_all = "camelCase")]
    Hosting {
        instance: Option<String>,
        world: Option<String>,
        code: Option<String>,
    },
}

const COLUMNS: &str = "id, name, mc_uuid, avatar, state, added_at";

impl Database {
    pub async fn list_friends(&self) -> Result<Vec<Friend>> {
        Ok(sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM friends ORDER BY state = 'friend' DESC, name COLLATE NOCASE"
        ))
        .fetch_all(self.pool())
        .await?)
    }

    pub async fn friend(&self, id: &str) -> Result<Option<Friend>> {
        Ok(
            sqlx::query_as(&format!("SELECT {COLUMNS} FROM friends WHERE id = ?"))
                .bind(id)
                .fetch_optional(self.pool())
                .await?,
        )
    }

    /// Adds a friend, or changes the state of one already known.
    pub async fn set_friend(&self, id: &str, name: &str, state: FriendState) -> Result<()> {
        sqlx::query(
            "INSERT INTO friends (id, name, state, added_at) VALUES (?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET state = excluded.state",
        )
        .bind(id)
        .bind(name)
        .bind(state)
        .bind(chrono_now())
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Who they are, as their launcher says.
    pub async fn set_friend_profile(
        &self,
        id: &str,
        name: &str,
        mc_uuid: &str,
        avatar: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE friends SET name = ?, mc_uuid = ?, avatar = ? WHERE id = ?")
            .bind(name)
            .bind(mc_uuid)
            .bind(avatar)
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn remove_friend(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM friends WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// This launcher's lasting key, kept in the system's credential store. One per data
/// folder, so a test folder is another player.
pub fn identity(data: &DataDir) -> Result<SecretKey> {
    let digest = Sha256::digest(data.root().to_string_lossy().as_bytes());
    let name = format!(
        "friends-key-{}",
        digest[..6]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    if let Some(hex) = secrets::get(&name)? {
        if let Some(bytes) = decode_hex(&hex) {
            return Ok(SecretKey::from_bytes(&bytes));
        }
    }
    let key = SecretKey::generate();
    let hex: String = key.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
    secrets::set(&name, &hex)?;
    Ok(key)
}

fn decode_hex(hex: &str) -> Option<[u8; 32]> {
    let mut bytes = [0u8; 32];
    if hex.len() != 64 {
        return None;
    }
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(bytes)
}

/// This launcher's friend code, made once (`XXXX-XXXX`).
pub async fn own_code(db: &Database) -> Result<String> {
    if let Some(code) = db.get_setting::<String>(CODE_SETTING).await? {
        if let Some(code) = invite::normalize(&code) {
            return Ok(code);
        }
    }
    let code = invite::normalize(&invite::new_code()).ok_or_else(|| fail("code invalide"))?;
    db.set_setting(CODE_SETTING, &code).await?;
    Ok(code)
}

fn code_key(code: &str) -> SecretKey {
    let digest: [u8; 32] = Sha256::digest(format!("tandem-friend-v1:{code}")).into();
    SecretKey::from_bytes(&digest)
}

/// Points the friend code at this launcher on the public relay.
pub async fn publish_code(http: &reqwest::Client, code: &str, me: EndpointId) -> Result<()> {
    invite::publish_record(
        http,
        &code_key(code),
        &format!("friend={me}"),
        CODE_TTL_SECS,
    )
    .await
}

/// The launcher a friend code points at, or `None` if the code is unknown.
pub async fn resolve_code(http: &reqwest::Client, code: &str) -> Result<Option<EndpointId>> {
    let Some(code) = invite::normalize(code) else {
        return Ok(None);
    };
    let value = invite::resolve_record(http, &code_key(&code).public()).await?;
    Ok(value.and_then(|v| v.strip_prefix("friend=")?.parse().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friend_codes_have_their_own_keys() {
        assert_eq!(code_key("7K2PQX9M").public(), code_key("7K2PQX9M").public());
        assert_ne!(code_key("7K2PQX9M").public(), code_key("7K2PQX9N").public());
        assert_eq!(decode_hex(&"ab".repeat(32)), Some([0xab; 32]));
        assert_eq!(decode_hex("abc"), None);
    }

    #[test]
    fn presence_is_tagged_json() {
        let hosting = Presence::Hosting {
            instance: Some("Prominence II".into()),
            world: None,
            code: Some("7K2P-QX9M".into()),
        };
        let json = serde_json::to_string(&hosting).unwrap();
        assert!(json.starts_with(r#"{"kind":"hosting""#));
        assert_eq!(serde_json::from_str::<Presence>(&json).unwrap(), hosting);
    }

    #[tokio::test]
    async fn friends_are_stored() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Database::open(&tmp.path().join("t.db")).await.unwrap();
        db.set_friend("a", "Léo", FriendState::Outgoing)
            .await
            .unwrap();
        db.set_friend("a", "ignored", FriendState::Friend)
            .await
            .unwrap();
        db.set_friend_profile("a", "Léo", "uuid", None)
            .await
            .unwrap();
        let friend = db.friend("a").await.unwrap().unwrap();
        assert_eq!(
            (friend.name.as_str(), friend.state),
            ("Léo", FriendState::Friend)
        );
        db.remove_friend("a").await.unwrap();
        assert!(db.list_friends().await.unwrap().is_empty());
    }
}
