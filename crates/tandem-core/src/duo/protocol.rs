//! What host and guest say to each other. Every QUIC stream starts with one byte saying
//! what it carries; the control stream then holds JSON messages, one per line.

use iroh::endpoint::{RecvStream, SendStream};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};

use super::fail;
use crate::error::Result;

/// Version of this protocol; a host refuses guests speaking another one.
pub const VERSION: u32 = 1;

/// First byte of the control stream (opened once by the guest).
pub const STREAM_CONTROL: u8 = b'C';
/// First byte of a stream carrying one TCP connection of the game.
pub const STREAM_GAME: u8 = b'G';

/// The world the host opened to LAN, as guests see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfo {
    pub motd: String,
}

/// A mod, identified by its file's SHA-1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModRef {
    pub file_name: String,
    pub sha1: String,
}

/// What a player plays with, to check both sides match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSummary {
    pub name: String,
    pub game_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    /// Enabled mods only.
    pub mods: Vec<ModRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Message {
    /// Guest → host, first message.
    #[serde(rename_all = "camelCase")]
    Hello {
        version: u32,
        /// Shows the guest was given the code.
        proof: String,
        player: String,
        instance: Option<InstanceSummary>,
    },
    /// Host → guest, when accepted.
    #[serde(rename_all = "camelCase")]
    Welcome {
        player: String,
        instance: Option<InstanceSummary>,
        world: Option<WorldInfo>,
    },
    /// Host → guest, then the connection closes.
    Refused { reason: String },
    /// Host → guest, whenever the world is opened or closed.
    World { world: Option<WorldInfo> },
}

pub async fn write(send: &mut SendStream, message: &Message) -> Result<()> {
    let mut line = serde_json::to_vec(message)?;
    line.push(b'\n');
    send.write_all(&line).await.map_err(fail)?;
    Ok(())
}

/// Reads messages from a control stream, one JSON object per line.
pub struct Reader {
    lines: tokio::io::Lines<BufReader<RecvStream>>,
}

impl Reader {
    pub fn new(recv: RecvStream) -> Self {
        Self {
            lines: BufReader::new(recv).lines(),
        }
    }

    /// The next message, or `None` once the other side closed the stream.
    pub async fn next(&mut self) -> Result<Option<Message>> {
        loop {
            match self.lines.next_line().await.map_err(fail)? {
                None => return Ok(None),
                Some(line) if line.trim().is_empty() => continue,
                // Unknown message types (newer peer) are skipped, not fatal.
                Some(line) => match serde_json::from_str(&line) {
                    Ok(message) => return Ok(Some(message)),
                    Err(err) => tracing::debug!(error = %err, "unknown duo message skipped"),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_tagged_json() {
        let hello = Message::Hello {
            version: VERSION,
            proof: "p".into(),
            player: "Léo".into(),
            instance: None,
        };
        let json = serde_json::to_string(&hello).unwrap();
        assert!(json.starts_with(r#"{"type":"hello""#));
        assert_eq!(serde_json::from_str::<Message>(&json).unwrap(), hello);
        let world: Message =
            serde_json::from_str(r#"{"type":"world","world":{"motd":"M"}}"#).unwrap();
        assert_eq!(
            world,
            Message::World {
                world: Some(WorldInfo { motd: "M".into() })
            }
        );
    }
}
