//! Parallel downloader: bounded concurrency, retries, resume via `.part` files, SHA-1 checks.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use futures_util::{stream, StreamExt, TryStreamExt};
use reqwest::{header, Client, StatusCode};
use serde::Serialize;
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::error::{Error, Result};

const MAX_ATTEMPTS: u32 = 4;
pub const DEFAULT_CONCURRENCY: usize = 16;

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    /// Lowercase hex SHA-1, verified after download when present.
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done_files: usize,
    pub total_files: usize,
    pub done_bytes: u64,
    pub total_bytes: u64,
}

struct Counters {
    done_files: AtomicUsize,
    done_bytes: AtomicU64,
    total_files: usize,
    total_bytes: u64,
}

impl Counters {
    fn snapshot(&self) -> Progress {
        Progress {
            done_files: self.done_files.load(Ordering::Relaxed),
            total_files: self.total_files,
            done_bytes: self.done_bytes.load(Ordering::Relaxed),
            total_bytes: self.total_bytes,
        }
    }
}

/// Downloads every task whose destination is missing (or has the wrong size).
/// `on_progress` is called often (per chunk); throttle on the consumer side.
pub async fn download_all<F>(
    client: &Client,
    tasks: Vec<DownloadTask>,
    concurrency: usize,
    on_progress: F,
) -> Result<()>
where
    F: Fn(Progress) + Send + Sync,
{
    let pending = pending_tasks(tasks).await;
    let counters = Counters {
        done_files: AtomicUsize::new(0),
        done_bytes: AtomicU64::new(0),
        total_files: pending.len(),
        total_bytes: pending.iter().filter_map(|t| t.size).sum(),
    };
    on_progress(counters.snapshot());
    if pending.is_empty() {
        return Ok(());
    }
    tracing::info!(
        files = counters.total_files,
        bytes = counters.total_bytes,
        "downloading"
    );

    stream::iter(pending)
        .map(|task| {
            let counters = &counters;
            let on_progress = &on_progress;
            async move {
                download_with_retry(client, &task, counters, on_progress).await?;
                counters.done_files.fetch_add(1, Ordering::Relaxed);
                on_progress(counters.snapshot());
                Ok::<_, Error>(())
            }
        })
        .buffer_unordered(concurrency.max(1))
        .try_collect::<()>()
        .await
}

/// Drops duplicate destinations and files that already exist with the expected size.
async fn pending_tasks(tasks: Vec<DownloadTask>) -> Vec<DownloadTask> {
    let mut seen = HashSet::new();
    let mut pending = Vec::new();
    for task in tasks {
        if !seen.insert(task.dest.clone()) {
            continue;
        }
        let present = match fs::metadata(&task.dest).await {
            Ok(meta) => task.size.is_none_or(|size| meta.len() == size),
            Err(_) => false,
        };
        if !present {
            pending.push(task);
        }
    }
    pending
}

async fn download_with_retry<F: Fn(Progress)>(
    client: &Client,
    task: &DownloadTask,
    counters: &Counters,
    on_progress: &F,
) -> Result<()> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        let added = AtomicU64::new(0);
        match download_one(client, task, counters, &added, on_progress).await {
            Ok(()) => return Ok(()),
            Err(err) => {
                // Bytes from a failed attempt will be counted again by the next one.
                counters
                    .done_bytes
                    .fetch_sub(added.load(Ordering::Relaxed), Ordering::Relaxed);
                if attempt >= MAX_ATTEMPTS || !is_retryable(&err) {
                    tracing::error!(url = %task.url, error = %err, "download failed");
                    return Err(err);
                }
                let delay = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                tracing::warn!(url = %task.url, error = %err, attempt, "retrying download");
                tokio::time::sleep(delay).await;
            }
        }
    }
}

fn is_retryable(err: &Error) -> bool {
    match err {
        Error::Http(_) | Error::Io(_) | Error::ChecksumMismatch { .. } => true,
        Error::HttpStatus { status, .. } => *status == 429 || *status >= 500,
        _ => false,
    }
}

fn part_path(dest: &Path) -> PathBuf {
    let mut name = OsString::from(dest.as_os_str());
    name.push(".part");
    PathBuf::from(name)
}

async fn download_one<F: Fn(Progress)>(
    client: &Client,
    task: &DownloadTask,
    counters: &Counters,
    added: &AtomicU64,
    on_progress: &F,
) -> Result<()> {
    if let Some(parent) = task.dest.parent() {
        fs::create_dir_all(parent).await?;
    }
    let part = part_path(&task.dest);
    let mut offset = fs::metadata(&part).await.map(|m| m.len()).unwrap_or(0);
    if task.size.is_some_and(|size| offset > size) {
        offset = 0;
    }

    let mut request = client.get(&task.url);
    if offset > 0 {
        request = request.header(header::RANGE, format!("bytes={offset}-"));
    }
    let response = request.send().await?;
    let status = response.status();
    let append = match status {
        StatusCode::PARTIAL_CONTENT => true,
        s if s.is_success() => false,
        StatusCode::RANGE_NOT_SATISFIABLE => {
            fs::remove_file(&part).await.ok();
            return Err(Error::HttpStatus {
                url: task.url.clone(),
                status: status.as_u16(),
            });
        }
        _ => {
            return Err(Error::HttpStatus {
                url: task.url.clone(),
                status: status.as_u16(),
            })
        }
    };
    if append {
        counters.done_bytes.fetch_add(offset, Ordering::Relaxed);
        added.fetch_add(offset, Ordering::Relaxed);
    }

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(&part)
        .await?;
    let mut writer = BufWriter::new(file);
    let mut body = response.bytes_stream();
    while let Some(chunk) = body.next().await {
        let chunk = chunk?;
        writer.write_all(&chunk).await?;
        let len = chunk.len() as u64;
        counters.done_bytes.fetch_add(len, Ordering::Relaxed);
        added.fetch_add(len, Ordering::Relaxed);
        on_progress(counters.snapshot());
    }
    writer.flush().await?;
    drop(writer);

    if let Some(expected) = &task.sha1 {
        let actual = sha1_file(&part).await?;
        if !actual.eq_ignore_ascii_case(expected) {
            fs::remove_file(&part).await.ok();
            return Err(Error::ChecksumMismatch {
                path: task.dest.clone(),
            });
        }
    }
    fs::rename(&part, &task.dest).await?;
    Ok(())
}

/// Lowercase hex SHA-1 of a file, computed off the async runtime.
pub async fn sha1_file(path: &Path) -> Result<String> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let mut file = std::fs::File::open(path)?;
        let mut hasher = sha1_smol::Sha1::new();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(hasher.digest().to_string())
    })
    .await
    .map_err(|e| Error::Io(std::io::Error::other(e)))?
}

/// Downloads a small file straight into memory, checking its SHA-1 when given.
pub async fn fetch_bytes(client: &Client, url: &str, sha1: Option<&str>) -> Result<Vec<u8>> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(Error::HttpStatus {
            url: url.to_owned(),
            status: response.status().as_u16(),
        });
    }
    let bytes = response.bytes().await?.to_vec();
    if let Some(expected) = sha1 {
        let actual = sha1_smol::Sha1::from(&bytes).digest().to_string();
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(Error::ChecksumMismatch { path: url.into() });
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sha1_of_known_content() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("hello.txt");
        std::fs::write(&path, b"hello").unwrap();
        assert_eq!(
            sha1_file(&path).await.unwrap(),
            "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
        );
    }

    #[tokio::test]
    async fn pending_skips_existing_and_duplicates() {
        let tmp = tempfile::tempdir().unwrap();
        let existing = tmp.path().join("a.bin");
        std::fs::write(&existing, [0u8; 4]).unwrap();
        let task = |dest: &Path, size| DownloadTask {
            url: "http://example.invalid".into(),
            dest: dest.to_owned(),
            sha1: None,
            size: Some(size),
        };
        let missing = tmp.path().join("b.bin");
        let pending = pending_tasks(vec![
            task(&existing, 4),
            task(&missing, 8),
            task(&missing, 8),
        ])
        .await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].dest, missing);

        // Wrong size means the file must be fetched again.
        assert_eq!(pending_tasks(vec![task(&existing, 5)]).await.len(), 1);
    }
}
