//! Shared handles every launcher operation needs.

use std::time::Duration;

use crate::db::Database;
use crate::error::Result;
use crate::paths::DataDir;

const USER_AGENT: &str = concat!(
    "Tandem/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/RandomZeleff/tandem)"
);

/// Cheap to clone: every field is reference-counted internally.
#[derive(Debug, Clone)]
pub struct Context {
    pub data: DataDir,
    pub db: Database,
    pub http: reqwest::Client,
}

impl Context {
    /// Creates the data directory layout, opens the database and builds the HTTP client.
    pub async fn init(data: DataDir) -> Result<Self> {
        data.ensure().await?;
        let db = Database::open(&data.database()).await?;
        Ok(Self {
            data,
            db,
            http: http_client()?,
        })
    }
}

pub fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(60))
        .build()?)
}
