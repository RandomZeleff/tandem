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
