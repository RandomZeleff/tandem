//! Launcher business logic, independent of Tauri.

pub mod account;
pub mod auth;
pub mod content;
pub mod context;
pub mod db;
pub mod download;
pub mod error;
pub mod install;
pub mod instance;
pub mod java;
pub mod launch;
pub mod logging;
pub mod meta;
pub mod paths;
pub mod store;

pub use context::Context;
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
