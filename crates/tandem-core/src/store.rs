//! Content-addressed file store (DECISIONS D4): one copy of each file, hardlinked
//! into every instance that uses it.

use std::io;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::paths::DataDir;

/// `store/ab/abcdef…` for a lowercase hex SHA-1.
pub fn path(data: &DataDir, sha1: &str) -> PathBuf {
    let prefix = sha1.get(..2).unwrap_or("00");
    data.store().join(prefix).join(sha1)
}

/// Places a stored file at `dest`, replacing what was there. Hardlinks when possible,
/// copies otherwise (e.g. store and instance on different volumes).
pub async fn link(stored: &Path, dest: &Path) -> Result<()> {
    let (stored, dest) = (stored.to_owned(), dest.to_owned());
    tokio::task::spawn_blocking(move || -> Result<()> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match std::fs::remove_file(&dest) {
            Err(err) if err.kind() != io::ErrorKind::NotFound => return Err(err.into()),
            _ => {}
        }
        if let Err(err) = std::fs::hard_link(&stored, &dest) {
            tracing::debug!(error = %err, dest = %dest.display(), "hardlink failed, copying");
            std::fs::copy(&stored, &dest)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| Error::Io(io::Error::other(e)))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn links_and_replaces() {
        let tmp = tempfile::tempdir().unwrap();
        let data = DataDir::new(tmp.path());
        let stored = path(&data, "abcdef");
        assert!(stored.ends_with("store/ab/abcdef") || stored.ends_with("store\\ab\\abcdef"));
        std::fs::create_dir_all(stored.parent().unwrap()).unwrap();
        std::fs::write(&stored, b"jar").unwrap();

        let dest = tmp.path().join("instances/x/mods/a.jar");
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::write(&dest, b"old").unwrap();
        link(&stored, &dest).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"jar");
    }
}
