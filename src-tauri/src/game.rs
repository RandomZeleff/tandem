//! Running game processes: preparation, output streaming, stop and exit reporting.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;
use tandem_core::crash::{self, CrashAnalysis};
use tandem_core::install::{self, InstallProgress, Stage};
use tandem_core::jvm;
use tandem_core::launch::{self, LaunchSpec};
use tandem_core::stats::{ProcessSampler, ProcessStats};
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
pub const STATS_EVENT: &str = "game://stats";

const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
const STATS_INTERVAL: Duration = Duration::from_secs(2);

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

    /// Marks an instance busy for a non-game task (e.g. a modpack install).
    /// Returns false if it is already busy.
    pub fn begin(&self, id: &str) -> bool {
        let mut slots = self.lock();
        if slots.contains_key(id) {
            return false;
        }
        slots.insert(id.to_owned(), Slot::Preparing);
        true
    }

    /// Ends a task started with [`Games::begin`].
    pub fn end(&self, id: &str) {
        let mut slots = self.lock();
        if matches!(slots.get(id), Some(Slot::Preparing)) {
            slots.remove(id);
        }
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
struct StatsPayload<'a> {
    instance_id: &'a str,
    #[serde(flatten)]
    stats: ProcessStats,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExitedPayload {
    instance_id: String,
    code: Option<i32>,
    stopped: bool,
    crash_report: Option<String>,
    analysis: Option<CrashAnalysis>,
}

/// Forwards install progress to the UI, throttled except on stage changes and completion.
pub fn progress_emitter<'a>(
    app: &'a AppHandle,
    id: &'a str,
) -> impl Fn(InstallProgress) + Send + Sync + 'a {
    let last_emit = Mutex::new((Instant::now() - PROGRESS_INTERVAL, Stage::Metadata));
    move |progress| {
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
    }
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
    tracing::info!(
        instance = %id,
        version = %instance.game_version,
        loader = %instance.loader,
        "preparing launch"
    );

    let target = install::Target::of(&instance);
    let mut prepared = install::prepare(ctx, target, &game_dir, progress_emitter(app, id)).await?;
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
        memory_mb: instance.memory_mb.unwrap_or_else(|| {
            jvm::auto_memory_mb(jvm::total_memory_mb(), jvm::count_mods(&game_dir))
        }),
        extra_jvm_args: instance
            .jvm_args
            .as_deref()
            .map(|a| a.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default(),
    });
    tracing::info!(
        instance = %id,
        java = %command.program.display(),
        memory = %command.args[0],
        "starting game"
    );
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

    let sampler = child
        .id()
        .map(|pid| tauri::async_runtime::spawn(report_stats(app.clone(), id.clone(), pid)));

    let (status, stopped) = tokio::select! {
        status = child.wait() => (status, false),
        _ = stop_rx => {
            tracing::info!(instance = %id, "stopping game");
            let _ = child.kill().await;
            (child.wait().await, true)
        }
    };
    if let Some(sampler) = sampler {
        sampler.abort();
    }
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
    let analysis = if crashed {
        let dir = game_dir.clone();
        let report = crash_report.clone();
        tauri::async_runtime::spawn_blocking(move || crash::analyze_exit(&dir, report.as_deref()))
            .await
            .ok()
    } else {
        None
    };
    if crashed {
        tracing::warn!(
            instance = %id,
            ?code,
            crash_report = ?crash_report,
            suspects = ?analysis.as_ref().map(|a| &a.suspects),
            "game crashed"
        );
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
            analysis,
        },
    );
}

/// Emits the game's memory and CPU use every few seconds until the process is gone.
async fn report_stats(app: AppHandle, id: String, pid: u32) {
    let mut sampler = ProcessSampler::new(pid);
    let mut ticks = tokio::time::interval(STATS_INTERVAL);
    ticks.tick().await;
    loop {
        ticks.tick().await;
        let Some(stats) = sampler.sample() else {
            break;
        };
        let _ = app.emit(
            STATS_EVENT,
            StatsPayload {
                instance_id: &id,
                stats,
            },
        );
    }
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
