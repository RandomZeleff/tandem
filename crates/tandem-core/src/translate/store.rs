//! Translations and glossary terms kept in SQLite.

use std::collections::HashMap;

use serde::Serialize;

use crate::db::Database;
use crate::error::Result;

pub fn source_hash(text: &str) -> String {
    sha1_smol::Sha1::from(text).digest().to_string()
}

/// Every translation into `locale`, by hash of the English text.
pub async fn load(db: &Database, locale: &str) -> Result<HashMap<String, String>> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT source_hash, target FROM translations WHERE locale = ?")
            .bind(locale)
            .fetch_all(db.pool())
            .await?;
    Ok(rows.into_iter().collect())
}

/// Saves model translations; corrections made by the player are kept.
pub async fn save_ai(
    db: &Database,
    locale: &str,
    model: &str,
    pairs: &[(String, String)],
) -> Result<()> {
    let mut tx = db.pool().begin().await?;
    for (source, target) in pairs {
        sqlx::query(
            "INSERT INTO translations (locale, source_hash, source, target, origin, model)
             VALUES (?, ?, ?, ?, 'ai', ?)
             ON CONFLICT(locale, source_hash) DO UPDATE SET
                target = excluded.target, model = excluded.model, updated_at = excluded.updated_at
             WHERE translations.origin = 'ai'",
        )
        .bind(locale)
        .bind(source_hash(source))
        .bind(source)
        .bind(target)
        .bind(model)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// A correction by the player. An empty `target` forgets the translation.
pub async fn save_manual(db: &Database, locale: &str, source: &str, target: &str) -> Result<()> {
    if target.trim().is_empty() {
        sqlx::query("DELETE FROM translations WHERE locale = ? AND source_hash = ?")
            .bind(locale)
            .bind(source_hash(source))
            .execute(db.pool())
            .await?;
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO translations (locale, source_hash, source, target, origin)
         VALUES (?, ?, ?, ?, 'manual')
         ON CONFLICT(locale, source_hash) DO UPDATE SET
            target = excluded.target, origin = 'manual', model = NULL, updated_at = excluded.updated_at",
    )
    .bind(locale)
    .bind(source_hash(source))
    .bind(source)
    .bind(target)
    .execute(db.pool())
    .await?;
    Ok(())
}

/// Hashes of the player's corrections into `locale`.
pub async fn manual_hashes(
    db: &Database,
    locale: &str,
) -> Result<std::collections::HashSet<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT source_hash FROM translations WHERE locale = ? AND origin = 'manual'",
    )
    .bind(locale)
    .fetch_all(db.pool())
    .await?;
    Ok(rows.into_iter().map(|(h,)| h).collect())
}

/// Forgets model translations of the given texts, so they are translated again.
pub async fn forget_ai(db: &Database, locale: &str, sources: &[String]) -> Result<()> {
    let mut tx = db.pool().begin().await?;
    for source in sources {
        sqlx::query(
            "DELETE FROM translations WHERE locale = ? AND source_hash = ? AND origin = 'ai'",
        )
        .bind(locale)
        .bind(source_hash(source))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryTerm {
    pub term: String,
    pub translation: String,
    /// Empty for terms shared by every instance.
    pub instance_id: String,
}

/// Terms for an instance: its own and the shared ones (its own win).
pub async fn glossary(db: &Database, locale: &str, instance_id: &str) -> Result<Vec<GlossaryTerm>> {
    Ok(sqlx::query_as(
        "SELECT term, translation, instance_id FROM glossary
         WHERE locale = ? AND instance_id IN ('', ?)
         ORDER BY term COLLATE NOCASE, instance_id DESC",
    )
    .bind(locale)
    .bind(instance_id)
    .fetch_all(db.pool())
    .await?)
}

pub async fn set_term(
    db: &Database,
    locale: &str,
    instance_id: &str,
    term: &str,
    translation: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO glossary (locale, instance_id, term, translation) VALUES (?, ?, ?, ?)
         ON CONFLICT(locale, instance_id, term) DO UPDATE SET translation = excluded.translation",
    )
    .bind(locale)
    .bind(instance_id)
    .bind(term.trim())
    .bind(translation.trim())
    .execute(db.pool())
    .await?;
    Ok(())
}

pub async fn remove_term(db: &Database, locale: &str, instance_id: &str, term: &str) -> Result<()> {
    sqlx::query("DELETE FROM glossary WHERE locale = ? AND instance_id = ? AND term = ?")
        .bind(locale)
        .bind(instance_id)
        .bind(term)
        .execute(db.pool())
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn keeps_manual_corrections() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("t.db")).await.unwrap();
        save_ai(
            &db,
            "fr_fr",
            "m",
            &[("Iron Gear".into(), "Engrenage de fer".into())],
        )
        .await
        .unwrap();
        save_manual(&db, "fr_fr", "Iron Gear", "Engrenage en fer")
            .await
            .unwrap();
        save_ai(
            &db,
            "fr_fr",
            "m",
            &[("Iron Gear".into(), "Rouage de fer".into())],
        )
        .await
        .unwrap();
        let all = load(&db, "fr_fr").await.unwrap();
        assert_eq!(
            all.get(&source_hash("Iron Gear")).map(String::as_str),
            Some("Engrenage en fer")
        );
        assert!(load(&db, "de_de").await.unwrap().is_empty());
        forget_ai(&db, "fr_fr", &["Iron Gear".into()])
            .await
            .unwrap();
        assert_eq!(load(&db, "fr_fr").await.unwrap().len(), 1);
        save_manual(&db, "fr_fr", "Iron Gear", " ").await.unwrap();
        assert!(load(&db, "fr_fr").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn instance_terms_come_with_shared_ones() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("t.db")).await.unwrap();
        set_term(&db, "fr_fr", "", "Mana", "Mana").await.unwrap();
        set_term(&db, "fr_fr", "pack", "Rune", "Rune ancienne")
            .await
            .unwrap();
        set_term(&db, "fr_fr", "other", "Void", "Néant")
            .await
            .unwrap();
        let terms: Vec<String> = glossary(&db, "fr_fr", "pack")
            .await
            .unwrap()
            .into_iter()
            .map(|t| t.term)
            .collect();
        assert_eq!(terms, ["Mana", "Rune"]);
        remove_term(&db, "fr_fr", "pack", "Rune").await.unwrap();
        assert_eq!(glossary(&db, "fr_fr", "pack").await.unwrap().len(), 1);
    }
}
