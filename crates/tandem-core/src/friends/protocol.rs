//! What friends' launchers say to each other: one control stream per connection, JSON
//! messages one per line. Both sides start with [`Message::Hello`].

use iroh::endpoint::{RecvStream, SendStream};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

use super::{fail, Presence};
use crate::error::Result;

/// Version of this protocol; a launcher ignores peers speaking another one.
pub const VERSION: u32 = 1;
/// A line longer than this is not one of ours (a skin face is about 2 KB).
const MAX_LINE: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Message {
    /// First message, both ways: who is on this launcher.
    #[serde(rename_all = "camelCase")]
    Hello {
        version: u32,
        name: String,
        mc_uuid: String,
        avatar: Option<String>,
    },
    /// "Be my friend."
    Request,
    /// Answer to a request (or confirmation for a friend who lost it).
    Accept,
    /// Not friends any more, or the request is declined.
    Remove,
    /// What the player is doing; `None` when they appear offline.
    Presence { presence: Option<Presence> },
    /// Come and play: the invite code of the world being hosted.
    #[serde(rename_all = "camelCase")]
    Invite {
        code: String,
        instance: Option<String>,
        world: Option<String>,
    },
}

pub async fn write(send: &mut SendStream, message: &Message) -> Result<()> {
    let mut line = serde_json::to_vec(message)?;
    line.push(b'\n');
    send.write_all(&line).await.map_err(fail)?;
    Ok(())
}

/// Reads messages from a control stream, one JSON object per line.
pub struct Reader {
    inner: BufReader<RecvStream>,
    line: String,
}

impl Reader {
    pub fn new(recv: RecvStream) -> Self {
        Self {
            inner: BufReader::new(recv),
            line: String::new(),
        }
    }

    /// The next message, or `None` once the other side closed the stream.
    pub async fn next(&mut self) -> Result<Option<Message>> {
        loop {
            self.line.clear();
            let read = (&mut self.inner)
                .take(MAX_LINE as u64)
                .read_line(&mut self.line)
                .await
                .map_err(fail)?;
            if read == 0 {
                return Ok(None);
            }
            if !self.line.ends_with('\n') && read >= MAX_LINE {
                return Err(fail("message trop long"));
            }
            let line = self.line.trim();
            if line.is_empty() {
                continue;
            }
            // Unknown message types (newer peer) are skipped, not fatal.
            match serde_json::from_str(line) {
                Ok(message) => return Ok(Some(message)),
                Err(err) => tracing::debug!(error = %err, "unknown friends message skipped"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_tagged_json() {
        let invite = Message::Invite {
            code: "7K2P-QX9M".into(),
            instance: Some("Prominence II".into()),
            world: None,
        };
        let json = serde_json::to_string(&invite).unwrap();
        assert!(json.starts_with(r#"{"type":"invite""#));
        assert_eq!(serde_json::from_str::<Message>(&json).unwrap(), invite);
        assert_eq!(
            serde_json::from_str::<Message>(r#"{"type":"request"}"#).unwrap(),
            Message::Request
        );
        let presence: Message =
            serde_json::from_str(r#"{"type":"presence","presence":{"kind":"online"}}"#).unwrap();
        assert_eq!(
            presence,
            Message::Presence {
                presence: Some(Presence::Online)
            }
        );
    }
}
