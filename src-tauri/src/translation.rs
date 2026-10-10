//! Commands of the AI translation (see `tandem_core::translate`).

use serde::Serialize;
use tandem_core::content::project;
use tandem_core::translate::lang::LANGUAGES;
use tandem_core::translate::llm::{Preset, ProviderConfig, PRESETS};
use tandem_core::translate::service::{Preferences, ProviderState};
use tandem_core::translate::store::{self, GlossaryTerm};
use tandem_core::translate::{Overview, Progress, ReviewEntry, RunReport};
use tauri::{AppHandle, Emitter, State};

use crate::error::CommandResult;
use crate::AppState;

/// Progress of a running translation.
pub const PROGRESS_EVENT: &str = "translate://progress";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Language {
    code: &'static str,
    name: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationSettings {
    languages: Vec<Language>,
    presets: &'static [Preset],
    provider: ProviderState,
    preferences: Preferences,
}

#[tauri::command]
pub async fn translation_settings(
    state: State<'_, AppState>,
) -> CommandResult<TranslationSettings> {
    Ok(TranslationSettings {
        languages: LANGUAGES
            .iter()
            .map(|(code, name)| Language { code, name })
            .collect(),
        presets: PRESETS,
        provider: state.translations.provider(&state.ctx).await?,
        preferences: state.translations.preferences(&state.ctx).await?,
    })
}

#[tauri::command]
pub async fn set_translation_preferences(
    state: State<'_, AppState>,
    preferences: Preferences,
) -> CommandResult<()> {
    Ok(state
        .translations
        .set_preferences(&state.ctx, &preferences)
        .await?)
}

/// `key`: a new API key, `""` to delete it, `null` to keep the stored one.
#[tauri::command]
pub async fn set_translation_provider(
    state: State<'_, AppState>,
    config: ProviderConfig,
    key: Option<String>,
) -> CommandResult<()> {
    Ok(state
        .translations
        .set_provider(&state.ctx, &config, key.as_deref())
        .await?)
}

/// The provider's models, proving it answers.
#[tauri::command]
pub async fn test_translation_provider(
    state: State<'_, AppState>,
    config: ProviderConfig,
    key: Option<String>,
) -> CommandResult<Vec<String>> {
    Ok(state
        .translations
        .test_provider(&state.ctx, &config, key.as_deref())
        .await?)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewWithState {
    #[serde(flatten)]
    overview: Overview,
    running: bool,
}

#[tauri::command]
pub async fn translation_overview(
    state: State<'_, AppState>,
    instance_id: String,
    locale: String,
) -> CommandResult<OverviewWithState> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    let overview = state
        .translations
        .overview(&state.ctx, &instance, &locale)
        .await?;
    Ok(OverviewWithState {
        overview,
        running: state.translations.is_running(&instance_id),
    })
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    instance_id: String,
    progress: Progress,
}

/// Translates what is missing and writes the result into the instance. Resolves when
/// the run ends (finished, cancelled or stopped by errors).
#[tauri::command]
pub async fn translate_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    locale: String,
) -> CommandResult<RunReport> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    let id = instance_id.clone();
    Ok(state
        .translations
        .run(&state.ctx, &instance, &locale, move |progress| {
            let _ = app.emit(
                PROGRESS_EVENT,
                ProgressEvent {
                    instance_id: id.clone(),
                    progress,
                },
            );
        })
        .await?)
}

#[tauri::command]
pub fn cancel_translation(state: State<'_, AppState>, instance_id: String) {
    state.translations.cancel(&instance_id);
}

/// Writes (from what is translated so far) or removes the instance's translation.
#[tauri::command]
pub async fn set_translation_active(
    state: State<'_, AppState>,
    instance_id: String,
    locale: String,
    active: bool,
) -> CommandResult<()> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    if active {
        state
            .translations
            .activate(&state.ctx, &instance, &locale)
            .await?;
    } else {
        state.translations.deactivate(&state.ctx, &instance).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn translation_entries(
    state: State<'_, AppState>,
    instance_id: String,
    locale: String,
    source_id: String,
    query: String,
) -> CommandResult<Vec<ReviewEntry>> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(state
        .translations
        .entries(&state.ctx, &instance, &locale, &source_id, &query)
        .await?)
}

/// The player's own translation of a text (empty to forget it).
#[tauri::command]
pub async fn correct_translation(
    state: State<'_, AppState>,
    instance_id: String,
    locale: String,
    english: String,
    translation: String,
) -> CommandResult<()> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(state
        .translations
        .correct(&state.ctx, &instance, &locale, &english, &translation)
        .await?)
}

/// Forgets the model's translations of a mod or pack, to translate it again.
#[tauri::command]
pub async fn forget_translations(
    state: State<'_, AppState>,
    instance_id: String,
    locale: String,
    source_id: String,
) -> CommandResult<()> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(state
        .translations
        .forget(&state.ctx, &instance, &locale, &source_id)
        .await?)
}

#[tauri::command]
pub async fn set_translation_excluded(
    state: State<'_, AppState>,
    instance_id: String,
    source_id: String,
    excluded: bool,
) -> CommandResult<()> {
    Ok(state
        .translations
        .set_excluded(&state.ctx, &instance_id, &source_id, excluded)
        .await?)
}

#[tauri::command]
pub async fn glossary_terms(
    state: State<'_, AppState>,
    instance_id: String,
    locale: String,
) -> CommandResult<Vec<GlossaryTerm>> {
    Ok(store::glossary(&state.ctx.db, &locale, &instance_id).await?)
}

/// `instance_id` null: a term for every instance.
#[tauri::command]
pub async fn set_glossary_term(
    state: State<'_, AppState>,
    instance_id: Option<String>,
    locale: String,
    term: String,
    translation: String,
) -> CommandResult<()> {
    Ok(store::set_term(
        &state.ctx.db,
        &locale,
        instance_id.as_deref().unwrap_or(""),
        &term,
        &translation,
    )
    .await?)
}

#[tauri::command]
pub async fn remove_glossary_term(
    state: State<'_, AppState>,
    instance_id: Option<String>,
    locale: String,
    term: String,
) -> CommandResult<()> {
    Ok(store::remove_term(
        &state.ctx.db,
        &locale,
        instance_id.as_deref().unwrap_or(""),
        &term,
    )
    .await?)
}

/// A project's description in `locale`, sanitized.
#[tauri::command]
pub async fn translate_description(
    state: State<'_, AppState>,
    project_id: String,
    locale: String,
) -> CommandResult<String> {
    let details = project::details(&state.ctx, &state.projects, &project_id).await?;
    Ok(state
        .translations
        .translate_html(&state.ctx, &locale, &details.title, &details.body_html)
        .await?)
}

/// Rewrites an instance's translation in the background after its content changed.
pub fn refresh_later(app: &AppHandle, instance_id: &str) {
    use tauri::Manager;
    let app = app.clone();
    let instance_id = instance_id.to_owned();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let Ok(instance) = state.ctx.db.get_instance(&instance_id).await else {
            return;
        };
        if let Err(err) = state.translations.refresh(&state.ctx, &instance).await {
            tracing::warn!(instance = %instance_id, %err, "could not refresh translation");
        }
    });
}
