//! Updates of Tandem itself: signed installers published with each GitHub Release
//! (`latest.json`), checked at startup, installed only when the player asks.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::{CommandError, CommandResult};
use crate::AppState;

/// Download progress of the update (payload: [`Progress`]).
pub const PROGRESS_EVENT: &str = "update://progress";

/// The update found by the last check, kept until it is installed.
#[derive(Default)]
pub struct Updates {
    pending: Mutex<Option<Update>>,
}

impl Updates {
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Update>> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    /// Release notes, in Markdown.
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

/// Asks GitHub whether a newer Tandem exists. `None` when this one is the latest.
#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Option<UpdateInfo>> {
    let updater = app.updater().map_err(|err| {
        tracing::warn!(error = %err, "update check unavailable");
        CommandError::msg("Impossible de vérifier les mises à jour.")
    })?;
    let update = match updater.check().await {
        Ok(update) => update,
        // GitHub answered, but the latest release has no `latest.json` (none published
        // since self-updates exist): nothing newer to offer.
        Err(tauri_plugin_updater::Error::ReleaseNotFound) => {
            tracing::info!("no update manifest published yet");
            None
        }
        Err(err) => {
            tracing::warn!(error = %err, "update check failed");
            return Err(CommandError::msg(
                "Impossible de vérifier les mises à jour. Vérifie ta connexion.",
            ));
        }
    };
    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        notes: u.body.clone(),
    });
    if let Some(info) = &info {
        tracing::info!(version = %info.version, "update available");
    }
    *state.updates.lock() = update;
    Ok(info)
}

/// Downloads and installs the update found by [`check_update`], then restarts Tandem.
/// Refused while a game runs or installs, or during a session with friends: the
/// installer closes the launcher.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    if state.games.any_busy() {
        return Err(CommandError::msg(
            "Ferme le jeu avant de mettre Tandem à jour.",
        ));
    }
    if state.duo.active().await {
        return Err(CommandError::msg(
            "Termine la partie avec tes amis avant de mettre Tandem à jour.",
        ));
    }
    let update = state
        .updates
        .lock()
        .take()
        .ok_or_else(|| CommandError::msg("Aucune mise à jour à installer."))?;

    let mut downloaded = 0u64;
    let result = update
        .download_and_install(
            |chunk, total| {
                downloaded += chunk as u64;
                let _ = app.emit(PROGRESS_EVENT, Progress { downloaded, total });
            },
            || tracing::info!(version = %update.version, "update downloaded, installing"),
        )
        .await;
    if let Err(err) = result {
        tracing::warn!(error = %err, "update install failed");
        // Kept so the player can try again without a new check.
        *state.updates.lock() = Some(update);
        return Err(CommandError::msg(
            "La mise à jour n'a pas pu être installée. Réessaie plus tard.",
        ));
    }
    // Windows: the installer has already closed Tandem. Elsewhere, start the new one.
    app.restart();
}
