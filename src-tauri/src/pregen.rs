//! "Préparer pour jouer à plusieurs" (chantier 3): runs [`tandem_core::pregen`] in the
//! background and reports its progress to the Worlds tab.

use std::collections::HashMap;

use serde::Serialize;
use tandem_core::pregen::{self, Progress};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{oneshot, Mutex};

use crate::error::{CommandError, CommandResult};
use crate::AppState;

const PROGRESS_EVENT: &str = "pregen://progress";

/// Preparations running, by instance: what the page last saw, and how to cancel.
#[derive(Default)]
pub struct Pregens(Mutex<HashMap<String, Running>>);

struct Running {
    view: PregenView,
    cancel: Option<oneshot::Sender<()>>,
}

impl Pregens {
    pub async fn running(&self, instance_id: &str) -> bool {
        self.0.lock().await.contains_key(instance_id)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PregenView {
    pub instance_id: String,
    pub world: String,
    pub radius: u32,
    /// `starting`, `running`, `stopping`, `done`, `cancelled` or `failed`.
    pub status: &'static str,
    pub progress: Option<Progress>,
    pub error: Option<String>,
}

/// Starts preparing `world`; progress comes as `pregen://progress` events.
#[tauri::command]
pub async fn pregen_start(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    world: String,
    radius: u32,
) -> CommandResult<PregenView> {
    if !pregen::RADII.contains(&radius) {
        return Err(CommandError::msg("Taille de zone inconnue"));
    }
    if state.duo.serves(&instance_id).await {
        return Err(CommandError::msg(
            "Ce monde est hébergé en ce moment : arrête l'invitation d'abord.",
        ));
    }
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    if !state.games.begin(&instance_id) {
        return Err(CommandError::msg(
            "L'instance est lancée : ferme le jeu pour préparer un de ses mondes.",
        ));
    }
    let (cancel, cancelled) = oneshot::channel();
    let view = PregenView {
        instance_id: instance_id.clone(),
        world: world.clone(),
        radius,
        status: "starting",
        progress: None,
        error: None,
    };
    state.pregens.0.lock().await.insert(
        instance_id.clone(),
        Running {
            view: view.clone(),
            cancel: Some(cancel),
        },
    );
    let _ = app.emit(PROGRESS_EVENT, &view);
    let ctx = state.ctx.clone();
    let first = view.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let outcome = run(&app, &ctx, &instance, &world, radius, cancelled).await;
        state.games.end(&instance.id);
        let mut running = state.pregens.0.lock().await;
        let mut view = running.remove(&instance.id).map(|r| r.view).unwrap_or(view);
        match outcome {
            Ok(true) => view.status = "done",
            Ok(false) => view.status = "cancelled",
            Err(err) => {
                tracing::warn!(instance = %instance.id, error = %err.0, "world preparation failed");
                view.status = "failed";
                view.error = Some(err.0);
            }
        }
        drop(running);
        let _ = app.emit(PROGRESS_EVENT, &view);
    });
    Ok(first)
}

struct RunError(String);

impl From<tandem_core::Error> for RunError {
    fn from(err: tandem_core::Error) -> Self {
        Self(err.to_string())
    }
}

/// Generates until done (`true`) or cancelled (`false`). A cancel while the server is
/// still starting takes effect once it is up, so the world is always saved cleanly.
async fn run(
    app: &AppHandle,
    ctx: &tandem_core::Context,
    instance: &tandem_core::instance::Instance,
    world: &str,
    radius: u32,
    mut cancelled: oneshot::Receiver<()>,
) -> Result<bool, RunError> {
    let mut task = pregen::start(ctx, instance, world, radius).await?;
    if cancelled.try_recv().is_ok() {
        task.stop().await?;
        return Ok(false);
    }
    update(app, &instance.id, "running", None).await;
    loop {
        tokio::select! {
            next = task.next() => match next {
                Some(progress) => {
                    update(app, &instance.id, "running", Some(progress)).await;
                    if progress.done {
                        update(app, &instance.id, "stopping", Some(progress)).await;
                        task.stop().await?;
                        return Ok(true);
                    }
                }
                None => {
                    return Err(RunError(
                        "le serveur s'est arrêté avant la fin (voir le journal)".into(),
                    ))
                }
            },
            _ = &mut cancelled => {
                update(app, &instance.id, "stopping", None).await;
                task.stop().await?;
                return Ok(false);
            }
        }
    }
}

async fn update(
    app: &AppHandle,
    instance_id: &str,
    status: &'static str,
    progress: Option<Progress>,
) {
    let state = app.state::<AppState>();
    let mut running = state.pregens.0.lock().await;
    if let Some(r) = running.get_mut(instance_id) {
        r.view.status = status;
        if progress.is_some() {
            r.view.progress = progress;
        }
        let _ = app.emit(PROGRESS_EVENT, &r.view);
    }
}

/// Stops a preparation: Chunky keeps where it was, the next one picks it up quickly.
#[tauri::command]
pub async fn pregen_cancel(state: State<'_, AppState>, instance_id: String) -> CommandResult<()> {
    if let Some(r) = state.pregens.0.lock().await.get_mut(&instance_id) {
        if let Some(cancel) = r.cancel.take() {
            let _ = cancel.send(());
        }
    }
    Ok(())
}

/// The preparation running for an instance, if any (when the page opens again).
#[tauri::command]
pub async fn pregen_status(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Option<PregenView>> {
    Ok(state
        .pregens
        .0
        .lock()
        .await
        .get(&instance_id)
        .map(|r| r.view.clone()))
}

/// Whether the instance can prepare its worlds (Fabric and Quilt for now).
#[tauri::command]
pub async fn pregen_supported(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<bool> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(tandem_core::server::supports(instance.loader))
}
