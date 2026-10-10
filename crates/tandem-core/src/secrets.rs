//! Secrets (API keys, later account tokens) in the system's credential store: Windows
//! Credential Manager, macOS Keychain. Never in the database or a file.

use crate::error::{Error, Result};

const SERVICE: &str = "dev.tandem.launcher";

fn entry(name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, name)
        .map_err(|e| Error::Translation(format!("Coffre de mots de passe inaccessible : {e}")))
}

pub fn get(name: &str) -> Result<Option<String>> {
    match entry(name)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(Error::Translation(format!(
            "Lecture du coffre impossible : {e}"
        ))),
    }
}

/// Stores `value`, or deletes the secret when it is empty.
pub fn set(name: &str, value: &str) -> Result<()> {
    let entry = entry(name)?;
    if value.trim().is_empty() {
        return match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(Error::Translation(format!(
                "Suppression dans le coffre impossible : {e}"
            ))),
        };
    }
    entry
        .set_password(value.trim())
        .map_err(|e| Error::Translation(format!("Écriture dans le coffre impossible : {e}")))
}

/// Characters per credential: Windows caps a secret at 2,560 bytes, stored as UTF-16.
const CHUNK: usize = 1000;

/// Reads a secret written by [`set_long`].
pub fn get_long(name: &str) -> Result<Option<String>> {
    let Some(count) = get(&format!("{name}#n"))? else {
        return Ok(None);
    };
    let count: usize = count.parse().unwrap_or(0);
    let mut value = String::new();
    for i in 0..count {
        match get(&format!("{name}#{i}"))? {
            Some(part) => value.push_str(&part),
            // Half-written or half-deleted: as good as absent.
            None => return Ok(None),
        }
    }
    Ok(Some(value))
}

/// Stores a secret of any length across several credentials (deleted when empty).
pub fn set_long(name: &str, value: &str) -> Result<()> {
    let previous: usize = get(&format!("{name}#n"))?
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let parts: Vec<String> = value
        .chars()
        .collect::<Vec<_>>()
        .chunks(CHUNK)
        .map(|c| c.iter().collect())
        .collect();
    for (i, part) in parts.iter().enumerate() {
        set(&format!("{name}#{i}"), part)?;
    }
    for i in parts.len()..previous {
        set(&format!("{name}#{i}"), "")?;
    }
    set(
        &format!("{name}#n"),
        &if parts.is_empty() {
            String::new()
        } else {
            parts.len().to_string()
        },
    )
}
