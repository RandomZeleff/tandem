use serde::Serialize;

/// Error returned by Tauri commands, serialized as its message for the frontend.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct CommandError(#[from] tandem_core::Error);

impl Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
