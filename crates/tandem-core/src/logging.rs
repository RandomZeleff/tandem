//! Tracing setup: rolling log files plus an in-memory buffer streamed to the UI.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

pub use tracing_appender::non_blocking::WorkerGuard;

use crate::error::{Error, Result};

const DEFAULT_FILTER: &str = "info,tandem=debug,tandem_lib=debug,tandem_core=debug";
const MAX_LOG_FILES: usize = 7;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// Monotonic sequence number; lets the UI drop duplicates between snapshot and stream.
    pub seq: u64,
    pub timestamp_ms: u64,
    pub level: String,
    pub target: String,
    pub message: String,
}

pub type LogListener = Box<dyn Fn(&LogEntry) + Send + Sync>;

/// Bounded ring buffer of the most recent log entries.
#[derive(Debug, Clone)]
pub struct LogBuffer {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    next_seq: Arc<AtomicU64>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            next_seq: Arc::new(AtomicU64::new(1)),
            capacity,
        }
    }

    fn next_seq(&self) -> u64 {
        self.next_seq.fetch_add(1, Ordering::Relaxed)
    }

    pub fn push(&self, entry: LogEntry) {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if entries.len() == self.capacity {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    pub fn snapshot(&self) -> Vec<LogEntry> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.iter().cloned().collect()
    }
}

struct UiLayer {
    buffer: LogBuffer,
    listener: Option<LogListener>,
}

impl<S: Subscriber> Layer<S> for UiLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let meta = event.metadata();
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or_default();
        let entry = LogEntry {
            seq: self.buffer.next_seq(),
            timestamp_ms,
            level: meta.level().to_string(),
            target: meta.target().to_string(),
            message: visitor.finish(),
        };
        if let Some(listener) = &self.listener {
            listener(&entry);
        }
        self.buffer.push(entry);
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
    fields: String,
}

impl MessageVisitor {
    fn finish(self) -> String {
        if self.fields.is_empty() {
            self.message
        } else {
            format!("{}{}", self.message, self.fields)
        }
    }
}

impl Visit for MessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_owned();
        } else {
            let _ = write!(self.fields, " {}={}", field.name(), value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            let _ = write!(self.fields, " {}={:?}", field.name(), value);
        }
    }
}

/// Installs the global subscriber: stderr, daily-rotated files in `logs_dir`, and the UI buffer.
/// Keep the returned guard alive for the whole program, or buffered file logs are lost.
pub fn init(
    logs_dir: &Path,
    buffer: LogBuffer,
    listener: Option<LogListener>,
) -> Result<WorkerGuard> {
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("tandem")
        .filename_suffix("log")
        .max_log_files(MAX_LOG_FILES)
        .build(logs_dir)
        .map_err(|e| Error::Logging(e.to_string()))?;
    let (file_writer, guard) = tracing_appender::non_blocking(appender);
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(fmt::layer().with_writer(file_writer).with_ansi(false))
        .with(UiLayer { buffer, listener })
        .try_init()
        .map_err(|e| Error::Logging(e.to_string()))?;
    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(buffer: &LogBuffer, message: &str) -> LogEntry {
        LogEntry {
            seq: buffer.next_seq(),
            timestamp_ms: 0,
            level: "INFO".into(),
            target: "test".into(),
            message: message.into(),
        }
    }

    #[test]
    fn buffer_keeps_most_recent_entries() {
        let buffer = LogBuffer::new(2);
        for msg in ["a", "b", "c"] {
            buffer.push(entry(&buffer, msg));
        }
        let snapshot = buffer.snapshot();
        let messages: Vec<_> = snapshot.iter().map(|e| e.message.as_str()).collect();
        assert_eq!(messages, ["b", "c"]);
        assert!(snapshot[0].seq < snapshot[1].seq);
    }
}
