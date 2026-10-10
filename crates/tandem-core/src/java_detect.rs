//! Java installations already on the computer (and the runtimes Tandem downloaded),
//! for players who want a specific Java for an instance. Read from each JDK's
//! `release` file: no process is started.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::paths::DataDir;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JavaInstall {
    /// The `java` executable.
    pub path: String,
    /// `21.0.10`, `1.8.0_503`.
    pub version: String,
    /// `21`, `8`.
    pub major: u32,
    pub vendor: Option<String>,
    /// Downloaded by Tandem (used automatically when it fits).
    pub managed: bool,
}

/// `8` for `1.8.0_503`, `21` for `21.0.10`.
pub fn major_of(version: &str) -> Option<u32> {
    let mut parts = version.split(['.', '_', '+', '-']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// Reads `JAVA_VERSION` and `IMPLEMENTOR` from a Java home's `release` file.
fn read_release(home: &Path) -> Option<(String, Option<String>)> {
    let text = std::fs::read_to_string(home.join("release")).ok()?;
    let value = |key: &str| {
        text.lines().find_map(|l| {
            l.strip_prefix(key)?
                .strip_prefix('=')
                .map(|v| v.trim().trim_matches('"').to_owned())
        })
    };
    Some((value("JAVA_VERSION")?, value("IMPLEMENTOR")))
}

fn executable(home: &Path) -> PathBuf {
    if cfg!(windows) {
        home.join("bin").join("java.exe")
    } else {
        home.join("bin").join("java")
    }
}

/// Folders that usually hold Java installations, one level deep.
fn search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if cfg!(windows) {
        for base in [
            std::env::var_os("ProgramFiles"),
            std::env::var_os("ProgramFiles(x86)"),
        ]
        .into_iter()
        .flatten()
        {
            let base = PathBuf::from(base);
            for vendor in [
                "Java",
                "Eclipse Adoptium",
                "Eclipse Foundation",
                "Microsoft",
                "Zulu",
                "Amazon Corretto",
                "BellSoft",
                "Semeru",
                "OpenJDK",
                "AdoptOpenJDK",
            ] {
                roots.push(base.join(vendor));
            }
        }
    } else if cfg!(target_os = "macos") {
        roots.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("Library/Java/JavaVirtualMachines"));
        }
    } else {
        roots.push(PathBuf::from("/usr/lib/jvm"));
    }
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".jdks"));
        roots.push(home.join(".sdkman/candidates/java"));
    }
    roots
}

/// A Java home for a folder: itself, or `Contents/Home` in macOS bundles.
fn home_of(dir: &Path) -> Option<PathBuf> {
    [dir.to_owned(), dir.join("Contents").join("Home")]
        .into_iter()
        .find(|h| executable(h).is_file())
}

/// Every Java found, newest first, without duplicates.
pub fn installed(data: &DataDir) -> Vec<JavaInstall> {
    let mut homes: Vec<(PathBuf, bool)> = Vec::new();
    for root in search_roots() {
        if let Ok(entries) = std::fs::read_dir(&root) {
            homes.extend(
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| home_of(&e.path()))
                    .map(|h| (h, false)),
            );
        }
    }
    if let Some(home) = std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .and_then(|h| home_of(&h))
    {
        homes.push((home, false));
    }
    // `java` on the PATH: its home is two levels up from the executable.
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = executable(dir.parent().unwrap_or(&dir));
            if candidate.is_file() {
                if let Some(home) = candidate.parent().and_then(Path::parent) {
                    homes.push((home.to_owned(), false));
                }
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(data.java()) {
        for entry in entries.filter_map(|e| e.ok()) {
            let dir = entry.path();
            let home = if cfg!(target_os = "macos") {
                dir.join("jre.bundle/Contents/Home")
            } else {
                dir
            };
            if executable(&home).is_file() {
                homes.push((home, true));
            }
        }
    }

    let mut seen = HashSet::new();
    let mut found: Vec<JavaInstall> = homes
        .into_iter()
        .filter_map(|(home, managed)| {
            let canonical = std::fs::canonicalize(&home).unwrap_or_else(|_| home.clone());
            if !seen.insert(canonical) {
                return None;
            }
            let (version, vendor) = read_release(&home)?;
            Some(JavaInstall {
                path: executable(&home).to_string_lossy().into_owned(),
                major: major_of(&version)?,
                version,
                vendor,
                managed,
            })
        })
        .collect();
    found.sort_by(|a, b| {
        b.major
            .cmp(&a.major)
            .then_with(|| a.managed.cmp(&b.managed))
            .then_with(|| a.path.cmp(&b.path))
    });
    found
}

/// The Java major version a game version JSON asks for (8 when it says nothing).
pub fn required_major(version: &crate::meta::version::VersionJson) -> u32 {
    version.java_version.as_ref().map_or(8, |j| j.major_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_versions() {
        assert_eq!(major_of("1.8.0_503"), Some(8));
        assert_eq!(major_of("21.0.10"), Some(21));
        assert_eq!(major_of("17"), Some(17));
        assert_eq!(major_of("abc"), None);
    }

    #[test]
    fn finds_homes_with_release_files() {
        let dir = tempfile::tempdir().unwrap();
        let data = DataDir::new(dir.path());
        let runtime = data.java().join("java-runtime-delta");
        let home = if cfg!(target_os = "macos") {
            runtime.join("jre.bundle/Contents/Home")
        } else {
            runtime
        };
        std::fs::create_dir_all(home.join("bin")).unwrap();
        std::fs::write(executable(&home), "").unwrap();
        std::fs::write(
            home.join("release"),
            "IMPLEMENTOR=\"Mojang\"\nJAVA_VERSION=\"21.0.7\"\n",
        )
        .unwrap();
        let found = installed(&data);
        let ours = found.iter().find(|j| j.managed).unwrap();
        assert_eq!(ours.major, 21);
        assert_eq!(ours.vendor.as_deref(), Some("Mojang"));
    }
}
