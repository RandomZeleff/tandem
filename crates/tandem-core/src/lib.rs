//! Launcher business logic, independent of Tauri.

pub mod account;
pub mod auth;
pub mod content;
pub mod context;
pub mod crash;
pub mod datapacks;
pub mod db;
pub mod download;
pub mod duo;
pub mod error;
pub mod friends;
pub mod health;
pub mod install;
pub mod instance;
pub mod java;
pub mod java_detect;
pub mod journal;
pub mod jvm;
pub mod launch;
pub mod launchers;
pub mod logging;
pub mod meta;
pub mod optimizations;
pub mod paths;
pub mod pregen;
pub mod process;
pub mod screenshots;
pub mod secrets;
pub mod server;
pub mod skins;
pub mod stats;
pub mod store;
pub mod translate;
pub mod worlds;

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
