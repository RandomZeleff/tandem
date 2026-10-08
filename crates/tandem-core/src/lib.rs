//! Launcher business logic, independent of Tauri.

pub mod auth;
pub mod db;
pub mod error;
pub mod logging;
pub mod paths;

pub use error::{Error, Result};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!super::version().is_empty());
    }
}
