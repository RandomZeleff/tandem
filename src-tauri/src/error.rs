use serde::Serialize;

/// Error returned by Tauri commands, serialized as its message for the frontend.
#[derive(Debug)]
pub struct CommandError(String);

impl CommandError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl From<tandem_core::Error> for CommandError {
    fn from(err: tandem_core::Error) -> Self {
        Self(err.to_string())
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
