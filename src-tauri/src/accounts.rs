//! Accounts: Microsoft sign-in in the browser, offline accounts (only once a Microsoft
//! account that owns the game was added, D36), switching and removal.

use std::sync::Mutex;

use tandem_core::account::{Account, AccountKind};
use tandem_core::auth::{self, PendingLogin, Step};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::watch;

use crate::error::{CommandError, CommandResult};
use crate::AppState;

/// Sign-in progress (payload: [`Step`]).
pub const LOGIN_STEP_EVENT: &str = "auth://step";

/// The sign-in waiting for the browser, and a way to cancel it.
#[derive(Default)]
pub struct Logins {
    pending: Mutex<Option<PendingLogin>>,
    cancel: Mutex<Option<watch::Sender<bool>>>,
}

impl Logins {
    fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        m.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Starts a Microsoft sign-in and opens its page in the browser. Returns the page URL,
/// for players whose browser did not open.
#[tauri::command]
pub async fn begin_microsoft_login(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<String> {
    let login = auth::start_login().await?;
    let url = login.url.clone();
    *Logins::lock(&state.logins.pending) = Some(login);
    if let Err(err) = app.opener().open_url(&url, None::<&str>) {
        tracing::warn!(error = %err, "could not open the browser");
    }
    Ok(url)
}

/// Waits for the sign-in started by [`begin_microsoft_login`] to complete.
#[tauri::command]
pub async fn finish_microsoft_login(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Account> {
    let login = Logins::lock(&state.logins.pending)
        .take()
        .ok_or_else(|| CommandError::msg("Aucune connexion en cours"))?;
    let (cancel, mut cancelled) = watch::channel(false);
    *Logins::lock(&state.logins.cancel) = Some(cancel);
    let emit = |step: Step| {
        let _ = app.emit(LOGIN_STEP_EVENT, step);
    };
    let result = tokio::select! {
        result = login.finish(&state.ctx, emit) => result.map_err(CommandError::from),
        _ = cancelled.wait_for(|c| *c) => Err(CommandError::msg("Connexion annulée")),
        // A sign-in left open in the browser does not hold the port forever.
        _ = tokio::time::sleep(std::time::Duration::from_secs(15 * 60)) => {
            Err(CommandError::msg("La connexion a pris trop de temps : recommence"))
        }
    };
    Logins::lock(&state.logins.cancel).take();
    if let Err(err) = &result {
        tracing::warn!(error = ?err, "Microsoft sign-in failed");
    }
    result
}

#[tauri::command]
pub fn cancel_microsoft_login(state: State<'_, AppState>) {
    Logins::lock(&state.logins.pending).take();
    if let Some(cancel) = Logins::lock(&state.logins.cancel).take() {
        let _ = cancel.send(true);
    }
}

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> CommandResult<Vec<Account>> {
    Ok(state.ctx.db.list_accounts().await?)
}

/// Offline accounts are for playing without network or under another name, once the
/// player has shown they own the game with a Microsoft account (as Prism does).
#[tauri::command]
pub async fn add_offline_account(
    state: State<'_, AppState>,
    username: String,
) -> CommandResult<Account> {
    if !state.ctx.db.has_microsoft_account().await? {
        return Err(CommandError::msg(
            "Connecte d'abord un compte Microsoft qui possède Minecraft",
        ));
    }
    Ok(state.ctx.db.add_offline_account(&username).await?)
}

#[tauri::command]
pub async fn set_active_account(state: State<'_, AppState>, id: String) -> CommandResult<Account> {
    Ok(state.ctx.db.set_active_account(&id).await?)
}

#[tauri::command]
pub async fn remove_account(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    let accounts = state.ctx.db.list_accounts().await?;
    let removed = accounts.iter().find(|a| a.id == id);
    if removed.is_some_and(|a| a.kind == AccountKind::Microsoft) {
        let account_id = id.clone();
        tokio::task::spawn_blocking(move || auth::forget(&account_id))
            .await
            .map_err(|e| CommandError::msg(e.to_string()))??;
    }
    state.ctx.db.remove_account(&id).await?;
    // The next account takes over, so the play button keeps working.
    if removed.is_some_and(|a| a.is_active) {
        if let Some(next) = accounts.iter().find(|a| a.id != id) {
            state.ctx.db.set_active_account(&next.id).await?;
        }
    }
    Ok(())
}
