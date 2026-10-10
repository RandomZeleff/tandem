use serde::Serialize;

/// Error returned by Tauri commands, serialized as its message for the frontend.
#[derive(Debug)]
pub struct CommandError(String);

impl CommandError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

/// Sent instead of the message so the frontend can offer to install Rosetta.
pub const ROSETTA_MISSING: &str = "rosetta-missing";

/// Sent when Modrinth does not know a project, so the page can say so.
pub const PROJECT_NOT_FOUND: &str = "project-not-found";

impl From<tandem_core::Error> for CommandError {
    fn from(err: tandem_core::Error) -> Self {
        match err {
            tandem_core::Error::RosettaMissing => Self::msg(ROSETTA_MISSING),
            tandem_core::Error::Http(ref detail) => {
                // The player gets the short message; the log keeps what failed.
                tracing::warn!(error = ?detail, "network request failed");
                Self(err.to_string())
            }
            err => Self(err.to_string()),
        }
    }
}

impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        Self(err.to_string())
    }
}

impl Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
