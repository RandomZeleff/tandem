//! Appearance page: the active account's skin and capes, and the skin library.

use std::path::PathBuf;

use tandem_core::account::Account;
use tandem_core::skins::{self, LibrarySkin, Variant, Wardrobe};
use tandem_core::Error;
use tauri::State;

use crate::error::CommandResult;
use crate::AppState;

async fn active(state: &AppState) -> CommandResult<Account> {
    Ok(state
        .ctx
        .db
        .active_account()
        .await?
        .ok_or(Error::NoActiveAccount)?)
}

#[tauri::command]
pub async fn skin_wardrobe(state: State<'_, AppState>) -> CommandResult<Wardrobe> {
    let account = active(&state).await?;
    Ok(skins::wardrobe(&state.ctx, &account).await?)
}

/// `skin`: a library id, or `default:<name>:<classic|slim>`.
#[tauri::command]
pub async fn skin_apply(state: State<'_, AppState>, skin: String) -> CommandResult<Wardrobe> {
    let account = active(&state).await?;
    Ok(skins::apply(&state.ctx, &account, &skin).await?)
}

#[tauri::command]
pub async fn skin_reset(state: State<'_, AppState>) -> CommandResult<Wardrobe> {
    let account = active(&state).await?;
    Ok(skins::reset(&state.ctx, &account).await?)
}

#[tauri::command]
pub async fn skin_set_cape(
    state: State<'_, AppState>,
    cape: Option<String>,
) -> CommandResult<Wardrobe> {
    let account = active(&state).await?;
    Ok(skins::set_cape(&state.ctx, &account, cape.as_deref()).await?)
}

#[tauri::command]
pub async fn skin_library(state: State<'_, AppState>) -> CommandResult<Vec<LibrarySkin>> {
    Ok(skins::library(&state.ctx).await?)
}

#[tauri::command]
pub async fn skin_defaults(state: State<'_, AppState>) -> CommandResult<Vec<LibrarySkin>> {
    Ok(skins::defaults(&state.ctx).await?)
}

#[tauri::command]
pub async fn skin_add_file(
    state: State<'_, AppState>,
    path: PathBuf,
) -> CommandResult<LibrarySkin> {
    Ok(skins::add_file(&state.ctx, &path).await?)
}

#[tauri::command]
pub async fn skin_add_player(
    state: State<'_, AppState>,
    name: String,
) -> CommandResult<LibrarySkin> {
    Ok(skins::add_player(&state.ctx, &name).await?)
}

#[tauri::command]
pub async fn skin_rename(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> CommandResult<()> {
    Ok(skins::rename(&state.ctx, &id, &name).await?)
}

#[tauri::command]
pub async fn skin_set_variant(
    state: State<'_, AppState>,
    id: String,
    variant: Variant,
) -> CommandResult<()> {
    Ok(skins::set_variant(&state.ctx, &id, variant).await?)
}

#[tauri::command]
pub async fn skin_remove(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    Ok(skins::remove(&state.ctx, &id).await?)
}
