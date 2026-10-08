//! Running game processes: preparation, output streaming, stop and exit reporting.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;
use tandem_core::install::{self, InstallProgress, Stage};
use tandem_core::launch::{self, LaunchSpec, DEFAULT_MEMORY_MB};
use tandem_core::{Context, Error};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Child;
use tokio::sync::oneshot;

use crate::error::{CommandError, CommandResult};

pub const PROGRESS_EVENT: &str = "install://progress";
pub const OUTPUT_EVENT: &str = "game://output";
pub const STARTED_EVENT: &str = "game://started";
pub const EXITED_EVENT: &str = "game://exited";

const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

enum Slot {
    Preparing,
    Running(oneshot::Sender<()>),
}

/// Instances currently preparing or running, keyed by instance id.
#[derive(Clone, Default)]
pub struct Games(Arc<Mutex<HashMap<String, Slot>>>);

impl Games {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Slot>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn is_busy(&self, id: &str) -> bool {
        self.lock().contains_key(id)
    }

    pub fn running_ids(&self) -> Vec<String> {
        self.lock()
            .iter()
            .filter(|(_, slot)| matches!(slot, Slot::Running(_)))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Asks a running game to stop. Returns false if it was not running.
    pub fn stop(&self, id: &str) -> bool {
        match self.lock().remove(id) {
            Some(Slot::Running(stop)) => {
                let _ = stop.send(());
                true
            }
            Some(slot @ Slot::Preparing) => {
                self.lock().insert(id.to_owned(), slot);
                false
            }
            None => false,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload<'a> {
    instance_id: &'a str,
    #[serde(flatten)]
    progress: InstallProgress,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutputPayload<'a> {
    instance_id: &'a str,
    stream: &'static str,
    line: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExitedPayload {
    instance_id: String,
    code: Option<i32>,
    stopped: bool,
    crash_report: Option<String>,
}

pub async fn launch(app: AppHandle, ctx: Context, games: Games, id: String) -> CommandResult<()> {
    {
        let mut slots = games.lock();
        if slots.contains_key(&id) {
            return Err(CommandError::msg("this instance is already running"));
        }
        slots.insert(id.clone(), Slot::Preparing);
    }
    match start(&app, &ctx, &id).await {
        Ok((child, game_dir)) => {
            let (stop_tx, stop_rx) = oneshot::channel();
            games.lock().insert(id.clone(), Slot::Running(stop_tx));
            ctx.db.mark_instance_played(&id).await?;
            let _ = app.emit(STARTED_EVENT, &id);
            tauri::async_runtime::spawn(supervise(app, games, id, child, game_dir, stop_rx));
            Ok(())
        }
        Err(err) => {
            games.lock().remove(&id);
            tracing::error!(instance = %id, error = %err, "launch failed");
            Err(err.into())
        }
    }
}

async fn start(app: &AppHandle, ctx: &Context, id: &str) -> Result<(Child, PathBuf), Error> {
    let instance = ctx.db.get_instance(id).await?;
    let account = ctx
        .db
        .active_account()
        .await?
        .ok_or(Error::NoActiveAccount)?;
    let game_dir = ctx.data.instance_dir(id);
    tracing::info!(instance = %id, version = %instance.game_version, "preparing launch");

    let last_emit = Mutex::new((Instant::now() - PROGRESS_INTERVAL, Stage::Metadata));
    let mut prepared = install::prepare(ctx, &instance.game_version, &game_dir, |progress| {
        let mut last = last_emit.lock().unwrap_or_else(|e| e.into_inner());
        let finished = progress.download.done_files == progress.download.total_files;
        if progress.stage != last.1 || finished || last.0.elapsed() >= PROGRESS_INTERVAL {
            *last = (Instant::now(), progress.stage);
            let _ = app.emit(
                PROGRESS_EVENT,
                ProgressPayload {
                    instance_id: id,
                    progress,
                },
            );
        }
    })
    .await?;
    if let Some(java) = &instance.java_path {
        prepared.java = PathBuf::from(java);
    }

    let command = launch::build_command(&LaunchSpec {
        prepared: &prepared,
        game_dir: &game_dir,
        assets_root: &ctx.data.assets(),
        libraries_dir: &ctx.data.libraries(),
        account: &account,
        // Offline accounts have no token; Microsoft sign-in arrives in Phase 2.
        access_token: "0",
        memory_mb: instance.memory_mb.unwrap_or(DEFAULT_MEMORY_MB),
        extra_jvm_args: instance
            .jvm_args
            .as_deref()
            .map(|a| a.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default(),
    });
    tracing::info!(instance = %id, java = %command.program.display(), "starting game");
    Ok((launch::spawn(&command)?, game_dir))
}

async fn supervise(
    app: AppHandle,
    games: Games,
    id: String,
    mut child: Child,
    game_dir: PathBuf,
    stop_rx: oneshot::Receiver<()>,
) {
    let started = SystemTime::now();
    let mut readers = Vec::new();
    if let Some(out) = child.stdout.take() {
        readers.push(tauri::async_runtime::spawn(forward(
            app.clone(),
            id.clone(),
            "stdout",
            out,
        )));
    }
    if let Some(err) = child.stderr.take() {
        readers.push(tauri::async_runtime::spawn(forward(
            app.clone(),
            id.clone(),
            "stderr",
            err,
        )));
    }

    let (status, stopped) = tokio::select! {
        status = child.wait() => (status, false),
        _ = stop_rx => {
            tracing::info!(instance = %id, "stopping game");
            let _ = child.kill().await;
            (child.wait().await, true)
        }
    };
    for reader in readers {
        let _ = reader.await;
    }
    games.lock().remove(&id);

    let code = status.as_ref().ok().and_then(|s| s.code());
    let crashed = !stopped && code != Some(0);
    let crash_report = if crashed {
        latest_crash_report(&game_dir, started).await
    } else {
        None
    };
    if crashed {
        tracing::warn!(instance = %id, ?code, crash_report = ?crash_report, "game crashed");
    } else {
        tracing::info!(instance = %id, ?code, "game exited");
    }
    let _ = app.emit(
        EXITED_EVENT,
        ExitedPayload {
            instance_id: id,
            code,
            stopped,
            crash_report: crash_report.map(|p| p.display().to_string()),
        },
    );
}

/// Streams lines to the UI. Reads raw bytes: the game may print non-UTF-8 text, and
/// a reader that stops on bad input would let the pipe fill up and freeze the game.
async fn forward(app: AppHandle, id: String, stream: &'static str, output: impl AsyncRead + Unpin) {
    let mut reader = BufReader::new(output);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let line = String::from_utf8_lossy(&buf).trim_end().to_owned();
                let _ = app.emit(
                    OUTPUT_EVENT,
                    OutputPayload {
                        instance_id: &id,
                        stream,
                        line,
                    },
                );
            }
        }
    }
}

async fn latest_crash_report(game_dir: &Path, since: SystemTime) -> Option<PathBuf> {
    let mut entries = tokio::fs::read_dir(game_dir.join("crash-reports"))
        .await
        .ok()?;
    let mut latest: Option<(SystemTime, PathBuf)> = None;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let Ok(modified) = entry.metadata().await.and_then(|m| m.modified()) else {
            continue;
        };
        if modified >= since && latest.as_ref().is_none_or(|(t, _)| modified > *t) {
            latest = Some((modified, entry.path()));
        }
    }
    latest.map(|(_, path)| path)
}
