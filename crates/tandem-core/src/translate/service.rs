//! What the launcher calls: plans kept in memory while the instance does not change,
//! runs that can be cancelled, turning a translation on and off, corrections, and
//! keeping the written translation in step with the instance's content.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::glossary::{self, Glossary};
use super::llm::{self, Client, ProviderConfig};
use super::output::{self, Manifest};
use super::sources::{self, Scan};
use super::{lang, store, Overview, Plan, Progress, ReviewEntry, RunReport, Translator};
use crate::context::Context;
use crate::error::{Error, Result};
use crate::instance::Instance;
use crate::secrets;

const PROVIDER: &str = "translation.provider";
const PREFERENCES: &str = "translation.preferences";

fn excluded_key(instance_id: &str) -> String {
    format!("translation.excluded.{instance_id}")
}

fn secret_name(preset: &str) -> String {
    format!("translation-key:{preset}")
}

/// Choices the player makes once for every instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub locale: String,
    /// Switch the game to the translation's language when turning it on.
    pub set_game_language: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            locale: "fr_fr".into(),
            set_game_language: true,
        }
    }
}

/// The provider as the settings page shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderState {
    pub config: Option<ProviderConfig>,
    /// A key is stored for the configured preset (the key itself never leaves Rust).
    pub has_key: bool,
}

struct CachedScan {
    fingerprint: u64,
    scan: Arc<Scan>,
}

/// One per launcher.
#[derive(Default)]
pub struct Service {
    scans: Mutex<HashMap<String, CachedScan>>,
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl Service {
    pub async fn preferences(&self, ctx: &Context) -> Result<Preferences> {
        Ok(ctx.db.get_setting(PREFERENCES).await?.unwrap_or_default())
    }

    pub async fn set_preferences(&self, ctx: &Context, preferences: &Preferences) -> Result<()> {
        if !lang::is_supported(&preferences.locale) {
            return Err(Error::InvalidInput(format!(
                "Langue inconnue : {}",
                preferences.locale
            )));
        }
        ctx.db.set_setting(PREFERENCES, preferences).await
    }

    pub async fn provider(&self, ctx: &Context) -> Result<ProviderState> {
        let config: Option<ProviderConfig> = ctx.db.get_setting(PROVIDER).await?;
        let has_key = match &config {
            Some(c) => secrets::get(&secret_name(&c.preset))
                .ok()
                .flatten()
                .is_some(),
            None => false,
        };
        Ok(ProviderState { config, has_key })
    }

    /// Saves the provider; `key` replaces the stored key (empty deletes it, `None` keeps it).
    pub async fn set_provider(
        &self,
        ctx: &Context,
        config: &ProviderConfig,
        key: Option<&str>,
    ) -> Result<()> {
        if let Some(key) = key {
            secrets::set(&secret_name(&config.preset), key)?;
        }
        ctx.db.set_setting(PROVIDER, config).await
    }

    /// Lists the models of a provider, to check it answers. `key` overrides the stored one.
    pub async fn test_provider(
        &self,
        ctx: &Context,
        config: &ProviderConfig,
        key: Option<&str>,
    ) -> Result<Vec<String>> {
        let key = match key {
            Some(k) if !k.trim().is_empty() => Some(k.to_owned()),
            _ => secrets::get(&secret_name(&config.preset)).ok().flatten(),
        };
        super::check_provider(&Client::new(ctx.http.clone(), config, key)?).await
    }

    async fn client(&self, ctx: &Context) -> Result<(Client, ProviderConfig)> {
        let config: ProviderConfig = ctx.db.get_setting(PROVIDER).await?.ok_or_else(|| {
            Error::Translation("Choisis d'abord un service de traduction dans les réglages.".into())
        })?;
        let key = secrets::get(&secret_name(&config.preset)).ok().flatten();
        if key.is_none() && llm::preset(&config.preset).is_some_and(|p| p.needs_key) {
            return Err(Error::Translation(
                "Ajoute ta clé API dans les réglages de traduction.".into(),
            ));
        }
        Ok((Client::new(ctx.http.clone(), &config, key)?, config))
    }

    async fn excluded(&self, ctx: &Context, instance_id: &str) -> Result<HashSet<String>> {
        let list: Vec<String> = ctx
            .db
            .get_setting(&excluded_key(instance_id))
            .await?
            .unwrap_or_default();
        Ok(list.into_iter().collect())
    }

    pub async fn set_excluded(
        &self,
        ctx: &Context,
        instance_id: &str,
        source_id: &str,
        excluded: bool,
    ) -> Result<()> {
        let mut list = self.excluded(ctx, instance_id).await?;
        if excluded {
            list.insert(source_id.to_owned());
        } else {
            list.remove(source_id);
        }
        let mut list: Vec<String> = list.into_iter().collect();
        list.sort();
        ctx.db.set_setting(&excluded_key(instance_id), &list).await
    }

    /// The instance's texts, rescanned only when its content changed.
    async fn scan(&self, ctx: &Context, instance: &Instance, locale: &str) -> Result<Arc<Scan>> {
        let game_dir = ctx.data.instance_dir(&instance.id);
        let fingerprint = fingerprint(&game_dir, locale);
        if let Ok(scans) = self.scans.lock() {
            if let Some(cached) = scans
                .get(&instance.id)
                .filter(|c| c.fingerprint == fingerprint)
            {
                return Ok(cached.scan.clone());
            }
        }
        let (data, id, version, loc) = (
            ctx.data.clone(),
            instance.id.clone(),
            instance.game_version.clone(),
            locale.to_owned(),
        );
        let scan = tokio::task::spawn_blocking(move || {
            sources::scan(&data, &id, &game_dir, &version, &loc)
        })
        .await
        .map_err(|e| Error::Translation(e.to_string()))?;
        let scan = Arc::new(scan);
        if let Ok(mut scans) = self.scans.lock() {
            // Scans of big packs weigh tens of megabytes: keep the latest few.
            if scans.len() >= 3 && !scans.contains_key(&instance.id) {
                if let Some(oldest) = scans.keys().next().cloned() {
                    scans.remove(&oldest);
                }
            }
            scans.insert(
                instance.id.clone(),
                CachedScan {
                    fingerprint,
                    scan: scan.clone(),
                },
            );
        }
        Ok(scan)
    }

    pub async fn plan(&self, ctx: &Context, instance: &Instance, locale: &str) -> Result<Plan> {
        let scan = self.scan(ctx, instance, locale).await?;
        let translations = store::load(&ctx.db, locale).await?;
        let excluded = self.excluded(ctx, &instance.id).await?;
        Ok(Plan::new(
            scan,
            &translations,
            excluded,
            &instance.game_version,
            locale,
        ))
    }

    pub async fn overview(
        &self,
        ctx: &Context,
        instance: &Instance,
        locale: &str,
    ) -> Result<Overview> {
        let plan = self.plan(ctx, instance, locale).await?;
        let game_dir = ctx.data.instance_dir(&instance.id);
        Ok(plan.overview(
            Manifest::load(&game_dir).locale,
            super::game_language(&game_dir),
        ))
    }

    pub fn is_running(&self, instance_id: &str) -> bool {
        self.running
            .lock()
            .is_ok_and(|r| r.contains_key(instance_id))
    }

    pub fn cancel(&self, instance_id: &str) {
        if let Ok(running) = self.running.lock() {
            if let Some(flag) = running.get(instance_id) {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    /// Translates what is missing, then writes the translation into the instance.
    pub async fn run(
        &self,
        ctx: &Context,
        instance: &Instance,
        locale: &str,
        on_progress: impl Fn(Progress) + Send + Sync,
    ) -> Result<RunReport> {
        let (client, config) = self.client(ctx).await?;
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut running = self
                .running
                .lock()
                .map_err(|_| Error::Translation("état interne".into()))?;
            if running.contains_key(&instance.id) {
                return Err(Error::Translation(
                    "Une traduction est déjà en cours pour cette instance.".into(),
                ));
            }
            running.insert(instance.id.clone(), cancel.clone());
        }
        let result = self
            .run_inner(ctx, instance, locale, &client, &config, cancel, on_progress)
            .await;
        if let Ok(mut running) = self.running.lock() {
            running.remove(&instance.id);
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_inner(
        &self,
        ctx: &Context,
        instance: &Instance,
        locale: &str,
        client: &Client,
        config: &ProviderConfig,
        cancel: Arc<AtomicBool>,
        on_progress: impl Fn(Progress) + Send + Sync,
    ) -> Result<RunReport> {
        let plan = self.plan(ctx, instance, locale).await?;
        let glossary = self.glossary(ctx, instance, locale, &plan).await?;
        let translator = Translator {
            db: &ctx.db,
            client,
            locale,
            glossary: &glossary,
            concurrency: config.concurrency.clamp(1, 16) as usize,
        };
        let report = super::translate(&translator, plan.jobs(), cancel, on_progress).await;
        tracing::info!(instance = %instance.id, locale, done = report.progress.done, failed = report.progress.failed, "translation run finished");
        if report.progress.done > 0 {
            self.activate(ctx, instance, locale).await?;
        }
        Ok(report)
    }

    /// Vanilla names, then names the mods' authors translated, then the player's terms.
    async fn glossary(
        &self,
        ctx: &Context,
        instance: &Instance,
        locale: &str,
        plan: &Plan,
    ) -> Result<Glossary> {
        let vanilla = glossary::vanilla(ctx, &instance.game_version, locale).await;
        let player = store::glossary(&ctx.db, locale, &instance.id).await?;
        // Shared terms first so the instance's own override them.
        let mut player: Vec<_> = player.into_iter().collect();
        player.sort_by_key(|t| !t.instance_id.is_empty());
        Ok(Glossary::new(
            vanilla
                .into_iter()
                .chain(plan.author_terms())
                .chain(player.into_iter().map(|t| (t.term, t.translation))),
        ))
    }

    /// Writes the translation from what is translated so far (no model involved).
    pub async fn activate(&self, ctx: &Context, instance: &Instance, locale: &str) -> Result<()> {
        let plan = self.plan(ctx, instance, locale).await?;
        let set_language = self.preferences(ctx).await?.set_game_language;
        let game_dir = ctx.data.instance_dir(&instance.id);
        let version = instance.game_version.clone();
        let locale = locale.to_owned();
        let outputs = plan.outputs();
        tokio::task::spawn_blocking(move || {
            output::apply(&game_dir, &version, &locale, &outputs, set_language)
        })
        .await
        .map_err(|e| Error::Translation(e.to_string()))?
    }

    /// Removes the translation from the instance; what was translated stays cached.
    pub async fn deactivate(&self, ctx: &Context, instance: &Instance) -> Result<()> {
        let game_dir = ctx.data.instance_dir(&instance.id);
        let version = instance.game_version.clone();
        tokio::task::spawn_blocking(move || output::remove(&game_dir, &version))
            .await
            .map_err(|e| Error::Translation(e.to_string()))?
    }

    /// Rewrites an active translation after the instance's content changed, so new or
    /// updated mods get the texts already translated.
    pub async fn refresh(&self, ctx: &Context, instance: &Instance) -> Result<()> {
        let game_dir = ctx.data.instance_dir(&instance.id);
        if let Some(locale) = Manifest::load(&game_dir).locale {
            self.activate(ctx, instance, &locale).await?;
        }
        Ok(())
    }

    pub async fn entries(
        &self,
        ctx: &Context,
        instance: &Instance,
        locale: &str,
        source_id: &str,
        query: &str,
    ) -> Result<Vec<ReviewEntry>> {
        Ok(self
            .plan(ctx, instance, locale)
            .await?
            .entries(source_id, query))
    }

    /// The player's correction of one text, written right away if the translation is on.
    pub async fn correct(
        &self,
        ctx: &Context,
        instance: &Instance,
        locale: &str,
        english: &str,
        translation: &str,
    ) -> Result<()> {
        store::save_manual(&ctx.db, locale, english, translation).await?;
        self.refresh(ctx, instance).await
    }

    /// Forgets the model's translations of one source, to translate it again.
    pub async fn forget(
        &self,
        ctx: &Context,
        instance: &Instance,
        locale: &str,
        source_id: &str,
    ) -> Result<()> {
        let plan = self.plan(ctx, instance, locale).await?;
        let texts: Vec<String> = plan
            .entries(source_id, "")
            .into_iter()
            .filter(|e| e.translation.is_some())
            .map(|e| e.english)
            .collect();
        store::forget_ai(&ctx.db, locale, &texts).await?;
        self.refresh(ctx, instance).await
    }

    /// A project description, translated (from the cache when possible).
    pub async fn translate_html(
        &self,
        ctx: &Context,
        locale: &str,
        context: &str,
        html: &str,
    ) -> Result<String> {
        let (client, _) = self.client(ctx).await?;
        super::translate_html(&ctx.db, &client, locale, context, html).await
    }
}

/// Changes when anything a scan reads changes: mods, packs and the pack list, KubeJS
/// assets, quests and loose books.
fn fingerprint(game_dir: &Path, locale: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    locale.hash(&mut hasher);
    for dir in ["mods", "resourcepacks"] {
        list_dir(&game_dir.join(dir), &mut hasher);
    }
    for tree in [
        "kubejs/assets",
        "config/ftbquests/quests",
        "patchouli_books",
    ] {
        walk_tree(&game_dir.join(tree), &mut hasher, 0);
    }
    if let Ok(options) = std::fs::read_to_string(game_dir.join("options.txt")) {
        sources::resource_packs(&options).hash(&mut hasher);
    }
    hasher.finish()
}

fn stamp(path: &Path, hasher: &mut impl Hasher) {
    if let Ok(meta) = std::fs::metadata(path) {
        meta.len().hash(hasher);
        if let Ok(modified) = meta.modified() {
            modified
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
                .hash(hasher);
        }
    }
}

fn list_dir(dir: &Path, hasher: &mut impl Hasher) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut names: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    names.sort();
    for path in names {
        path.hash(hasher);
        stamp(&path, hasher);
    }
}

fn walk_tree(dir: &Path, hasher: &mut impl Hasher, depth: u8) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk_tree(&path, hasher, depth + 1);
        } else {
            path.hash(hasher);
            stamp(&path, hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_follows_content() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path();
        std::fs::create_dir_all(game.join("mods")).unwrap();
        let empty = fingerprint(game, "fr_fr");
        assert_eq!(empty, fingerprint(game, "fr_fr"));
        assert_ne!(empty, fingerprint(game, "de_de"));
        std::fs::write(game.join("mods/a.jar"), "x").unwrap();
        let one = fingerprint(game, "fr_fr");
        assert_ne!(empty, one);
        std::fs::write(game.join("options.txt"), "resourcePacks:[\"file/a.zip\"]").unwrap();
        assert_ne!(one, fingerprint(game, "fr_fr"));
    }
}
