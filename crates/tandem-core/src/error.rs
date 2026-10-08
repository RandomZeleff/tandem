#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("could not locate the user data directory")]
    NoDataDir,
    #[error("logging setup failed: {0}")]
    Logging(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
