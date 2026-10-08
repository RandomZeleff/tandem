use std::path::PathBuf;

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
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("HTTP {status} for {url}")]
    HttpStatus { url: String, status: u16 },
    #[error("checksum mismatch for {}", path.display())]
    ChecksumMismatch { path: PathBuf },
    #[error("archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("could not locate the user data directory")]
    NoDataDir,
    #[error("logging setup failed: {0}")]
    Logging(String),
    #[error("unknown Minecraft version: {0}")]
    VersionNotFound(String),
    #[error("no Java runtime `{component}` available for {platform}")]
    JavaUnavailable { component: String, platform: String },
    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),
    #[error("instance not found: {0}")]
    InstanceNotFound(String),
    #[error("account not found: {0}")]
    AccountNotFound(String),
    #[error("no active account")]
    NoActiveAccount,
    #[error("{0} is not supported yet")]
    LoaderNotSupported(String),
    #[error("{loader} is not available for Minecraft {game_version}")]
    LoaderUnavailable {
        loader: String,
        game_version: String,
    },
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
