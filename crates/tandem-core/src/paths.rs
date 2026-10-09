//! Layout of the launcher data directory (`%APPDATA%/Tandem` on Windows).

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Overrides the data directory, e.g. to keep dev data apart from a real install.
pub const DATA_DIR_ENV: &str = "TANDEM_DATA_DIR";
const APP_DIR_NAME: &str = "Tandem";

#[derive(Debug, Clone)]
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// `$TANDEM_DATA_DIR` if set, otherwise `<platform data dir>/Tandem`.
    pub fn from_env() -> Result<Self> {
        if let Some(dir) = std::env::var_os(DATA_DIR_ENV) {
            return Ok(Self::new(dir));
        }
        let base = dirs::data_dir().ok_or(Error::NoDataDir)?;
        Ok(Self::new(base.join(APP_DIR_NAME)))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database(&self) -> PathBuf {
        self.root.join("launcher.db")
    }

    /// Content-addressed file store shared by all instances (DECISIONS D4).
    pub fn store(&self) -> PathBuf {
        self.root.join("store")
    }

    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }

    /// Maven-layout library jars shared by every version.
    pub fn libraries(&self) -> PathBuf {
        self.root.join("libraries")
    }

    /// One folder per game version: `<id>.json`, `<id>.jar`, extracted natives.
    pub fn versions(&self) -> PathBuf {
        self.root.join("versions")
    }

    pub fn version_dir(&self, id: &str) -> PathBuf {
        self.versions().join(id)
    }

    pub fn instances(&self) -> PathBuf {
        self.root.join("instances")
    }

    /// Game directory (`.minecraft` equivalent) of an instance.
    pub fn instance_dir(&self, id: &str) -> PathBuf {
        self.instances().join(id)
    }

    pub fn java(&self) -> PathBuf {
        self.root.join("java")
    }

    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    /// World backups, one folder per instance.
    pub fn backups(&self) -> PathBuf {
        self.root.join("backups")
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Creates the root and every subdirectory if missing.
    pub async fn ensure(&self) -> Result<()> {
        for dir in [
            self.store(),
            self.assets(),
            self.libraries(),
            self.versions(),
            self.instances(),
            self.java(),
            self.cache(),
            self.logs(),
        ] {
            tokio::fs::create_dir_all(dir).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ensure_creates_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let data = DataDir::new(tmp.path().join("Tandem"));
        data.ensure().await.unwrap();
        for dir in [data.store(), data.instances(), data.java(), data.logs()] {
            assert!(dir.is_dir(), "{} missing", dir.display());
        }
        // Idempotent.
        data.ensure().await.unwrap();
    }
}
