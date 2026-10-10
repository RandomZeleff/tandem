//! Shared handles every launcher operation needs.

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::db::Database;
use crate::error::{Error, Result};
use crate::paths::DataDir;

const USER_AGENT: &str = concat!(
    "Tandem/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/RandomZeleff/tandem)"
);

/// An optional refresh (one with a local copy to fall back on) gives up after this.
const OPTIONAL_TIMEOUT: Duration = Duration::from_secs(5);
/// After a network failure, optional refreshes are skipped for this long: on a network
/// without Internet, each would otherwise wait for its connection timeout.
const OFFLINE_GRACE: Duration = Duration::from_secs(120);

/// Cheap to clone: every field is reference-counted internally.
#[derive(Debug, Clone)]
pub struct Context {
    pub data: DataDir,
    pub db: Database,
    pub http: reqwest::Client,
    /// When the network last failed.
    offline_since: Arc<Mutex<Option<Instant>>>,
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
            offline_since: Arc::default(),
        })
    }

    /// Runs a request whose result has a local fallback: skipped right away while the
    /// network is known to be down, abandoned after a few seconds otherwise.
    pub async fn optional<T>(&self, request: impl Future<Output = Result<T>>) -> Result<T> {
        if self.recently_offline() {
            return Err(Error::Offline);
        }
        let result = tokio::time::timeout(OPTIONAL_TIMEOUT, request)
            .await
            .unwrap_or(Err(Error::Offline));
        match &result {
            Err(err) if err.is_network() => {
                tracing::info!("network unreachable, using local copies for a while");
                *self.lock_offline() = Some(Instant::now());
            }
            Ok(_) => *self.lock_offline() = None,
            Err(_) => {}
        }
        result
    }

    fn recently_offline(&self) -> bool {
        self.lock_offline()
            .is_some_and(|since| since.elapsed() < OFFLINE_GRACE)
    }

    fn lock_offline(&self) -> std::sync::MutexGuard<'_, Option<Instant>> {
        self.offline_since.lock().unwrap_or_else(|e| e.into_inner())
    }
}

pub fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(60))
        // The default HTTP/2 window (64 KB) caps a download at 64 KB per round trip:
        // ~7 MB/s from Modrinth's CDN on a connection curl fills at 40+ MB/s.
        .http2_adaptive_window(true)
        .build()?)
}
