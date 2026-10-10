//! Translating an instance's mods, packs, quests and books with a language model.
//!
//! Texts already translated by their authors are kept; the rest is translated once per
//! English string (cached in the database), in batches that share the mod's context
//! and the glossary terms they use, with game codes protected. The result is a
//! generated resource pack plus quest files, all reversible (see [`output`]).

pub mod glossary;
pub mod lang;
pub mod llm;
pub mod mask;
pub mod output;
pub mod service;
pub mod snbt;
pub mod sources;
pub mod store;

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::stream::{self, StreamExt};
use serde::Serialize;

use crate::error::{Error, Result};
use glossary::Glossary;
use lang::Format;
use llm::Client;
use output::Outputs;
use sources::{BookDest, Scan, SourceKind, Target};

/// Entries and characters per request: small enough for local models to answer in a
/// few seconds, large enough to share context.
const BATCH_ENTRIES: usize = 30;
const BATCH_CHARS: usize = 1800;

/// Consecutive failed batches after which a run stops (credit exhausted, model gone).
const MAX_FAILED_BATCHES: usize = 3;

/// Rough characters per token for estimates.
const CHARS_PER_TOKEN: f64 = 3.5;

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub total: usize,
    /// Translated by the mod's authors or a resource pack.
    pub existing: usize,
    /// Translated by Tandem (model or player correction).
    pub translated: usize,
    pub missing: usize,
}

impl Counts {
    fn add(&mut self, other: Counts) {
        self.total += other.total;
        self.existing += other.existing;
        self.translated += other.translated;
        self.missing += other.missing;
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub id: String,
    pub name: String,
    pub kind: SourceKind,
    pub excluded: bool,
    pub counts: Counts,
}

/// What translating the missing texts would cost.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    /// Distinct English texts to send.
    pub texts: usize,
    pub characters: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuestMode {
    /// The pack has quest language files: a new one is added.
    LanguageFile,
    /// Texts live in the quest files: translated in place, originals kept.
    InPlace,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub locale: String,
    pub sources: Vec<SourceStatus>,
    pub totals: Counts,
    pub estimate: Estimate,
    /// Locale of the translation written into the instance, if any.
    pub active_locale: Option<String>,
    /// `lang` of the instance's `options.txt`.
    pub game_language: Option<String>,
    pub quests: Option<QuestMode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    /// Translated by the authors: nothing to write.
    Existing,
    /// Translated by Tandem.
    Done(String),
    Missing,
    /// No words to translate.
    Skip,
}

#[derive(Debug, Clone)]
struct Item {
    source: usize,
    unit: usize,
    key: String,
    english: String,
    state: State,
}

/// A scanned instance with the state of every text.
pub struct Plan {
    scan: Arc<Scan>,
    items: Vec<Item>,
    excluded: HashSet<String>,
    game_version: String,
    locale: String,
}

/// An existing translation identical to a sentence in English was never translated
/// (mods often copy `en_us` into other languages).
fn untranslated_copy(existing: &str, english: &str) -> bool {
    existing == english && english.split_whitespace().count() >= 3
}

impl Plan {
    pub fn new(
        scan: Arc<Scan>,
        translations: &HashMap<String, String>,
        excluded: HashSet<String>,
        game_version: &str,
        locale: &str,
    ) -> Self {
        // Language keys are global: the last source defining a key decides its text.
        let mut winner: HashMap<&str, (usize, usize)> = HashMap::new();
        for (s, source) in scan.sources.iter().enumerate() {
            for (u, unit) in source.units.iter().enumerate() {
                if matches!(unit.target, Target::Lang { .. }) {
                    for (key, _) in &unit.entries {
                        winner.insert(key.as_str(), (s, u));
                    }
                }
            }
        }

        let mut items = Vec::new();
        for (s, source) in scan.sources.iter().enumerate() {
            for (u, unit) in source.units.iter().enumerate() {
                for (key, english) in &unit.entries {
                    let existing = match &unit.target {
                        Target::Lang { .. } => {
                            if winner.get(key.as_str()) != Some(&(s, u)) {
                                continue;
                            }
                            scan.existing.get(key)
                        }
                        Target::QuestLang { .. } => scan.existing_quests.get(key),
                        Target::QuestInline { .. } | Target::Book { .. } => None,
                    };
                    let state = if !mask::has_words(english) {
                        State::Skip
                    } else if existing.is_some_and(|e| !untranslated_copy(e, english)) {
                        State::Existing
                    } else if let Some(done) = translations.get(&store::source_hash(english)) {
                        State::Done(done.clone())
                    } else {
                        State::Missing
                    };
                    items.push(Item {
                        source: s,
                        unit: u,
                        key: key.clone(),
                        english: english.clone(),
                        state,
                    });
                }
            }
        }
        Self {
            scan,
            items,
            excluded,
            game_version: game_version.to_owned(),
            locale: locale.to_owned(),
        }
    }

    pub fn overview(
        &self,
        active_locale: Option<String>,
        game_language: Option<String>,
    ) -> Overview {
        let mut per_source = vec![Counts::default(); self.scan.sources.len()];
        for item in &self.items {
            let counts = &mut per_source[item.source];
            match item.state {
                State::Skip => continue,
                State::Existing => counts.existing += 1,
                State::Done(_) => counts.translated += 1,
                State::Missing => counts.missing += 1,
            }
            counts.total += 1;
        }
        let mut totals = Counts::default();
        let sources: Vec<SourceStatus> = self
            .scan
            .sources
            .iter()
            .zip(per_source)
            .filter(|(_, counts)| counts.total > 0)
            .map(|(source, counts)| {
                let excluded = self.excluded.contains(&source.id);
                if !excluded {
                    totals.add(counts);
                }
                SourceStatus {
                    id: source.id.clone(),
                    name: source.name.clone(),
                    kind: source.kind,
                    excluded,
                    counts,
                }
            })
            .collect();
        let quests = self
            .scan
            .sources
            .iter()
            .find(|s| s.kind == SourceKind::Quests)
            .map(|s| {
                if s.units
                    .iter()
                    .any(|u| matches!(u.target, Target::QuestInline { .. }))
                {
                    QuestMode::InPlace
                } else {
                    QuestMode::LanguageFile
                }
            });
        Overview {
            locale: self.locale.clone(),
            sources,
            totals,
            estimate: estimate(&self.jobs()),
            active_locale,
            game_language,
            quests,
        }
    }

    /// Distinct missing texts of included sources, most visible first.
    pub fn jobs(&self) -> Vec<Job> {
        let mut seen = HashSet::new();
        let mut jobs: Vec<Job> = Vec::new();
        // Texts with numbers join the first job sharing their masked form.
        let mut by_template: HashMap<String, usize> = HashMap::new();
        for item in &self.items {
            if item.state != State::Missing
                || self.excluded.contains(&self.scan.sources[item.source].id)
                || !seen.insert(item.english.as_str())
            {
                continue;
            }
            let source = &self.scan.sources[item.source];
            if item.english.bytes().any(|b| b.is_ascii_digit()) {
                let template = mask::mask_numbers(&item.english).text;
                if let Some(&index) = by_template.get(&template) {
                    jobs[index].variants.push(item.english.clone());
                    continue;
                }
                by_template.insert(template, jobs.len());
            }
            jobs.push(Job {
                english: item.english.clone(),
                context: source.name.clone(),
                priority: priority(source.kind, &source.units[item.unit].target, &item.key),
                variants: Vec::new(),
                hint: match source.units[item.unit].target {
                    Target::Lang { .. } => item.key.clone(),
                    _ => String::new(),
                },
            });
        }
        jobs.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.context.cmp(&b.context))
        });
        jobs
    }

    /// Names the mods' authors already translated: the model keeps them consistent.
    pub fn author_terms(&self) -> Vec<(String, String)> {
        self.items
            .iter()
            // Short texts are names and terms (types, elements, materials) worth reusing.
            .filter(|i| i.state == State::Existing && i.english.split_whitespace().count() <= 3)
            .filter_map(|i| {
                let existing = self.scan.existing.get(&i.key)?;
                (existing != &i.english && !existing.contains('%'))
                    .then(|| (i.english.clone(), existing.clone()))
            })
            .collect()
    }

    /// Texts of one source with their translation, for review. `query` filters on the
    /// English or the translation.
    pub fn entries(&self, source_id: &str, query: &str) -> Vec<ReviewEntry> {
        let query = query.trim().to_lowercase();
        let mut seen = HashSet::new();
        self.items
            .iter()
            .filter(|i| self.scan.sources[i.source].id == source_id && i.state != State::Skip)
            .filter(|i| seen.insert(i.english.clone()))
            .filter_map(|i| {
                let translation = match &i.state {
                    State::Done(t) => Some(t.clone()),
                    State::Existing => None,
                    _ => None,
                };
                let entry = ReviewEntry {
                    key: i.key.clone(),
                    english: i.english.clone(),
                    translation,
                    by_authors: i.state == State::Existing,
                };
                let matches = query.is_empty()
                    || entry.english.to_lowercase().contains(&query)
                    || entry.key.to_lowercase().contains(&query)
                    || entry
                        .translation
                        .as_deref()
                        .is_some_and(|t| t.to_lowercase().contains(&query));
                matches.then_some(entry)
            })
            .collect()
    }

    /// Files to write for the texts translated by Tandem.
    pub fn outputs(&self) -> Outputs {
        let format = self.scan.format.unwrap_or(Format::Json);
        let mut outputs = Outputs::default();
        let mut lang: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut by_unit: HashMap<(usize, usize), Vec<&Item>> = HashMap::new();
        for item in &self.items {
            if self.excluded.contains(&self.scan.sources[item.source].id) {
                continue;
            }
            by_unit
                .entry((item.source, item.unit))
                .or_default()
                .push(item);
        }

        let mut units: Vec<_> = by_unit.into_iter().collect();
        units.sort_by_key(|((s, u), _)| (*s, *u));
        for ((s, u), items) in units {
            let unit = &self.scan.sources[s].units[u];
            let done: HashMap<&str, &str> = items
                .iter()
                .filter_map(|i| match &i.state {
                    State::Done(t) => Some((i.key.as_str(), t.as_str())),
                    _ => None,
                })
                .collect();
            if done.is_empty() {
                continue;
            }
            match &unit.target {
                Target::Lang { namespace } => {
                    let list = lang.entry(namespace.clone()).or_default();
                    list.extend(done.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())));
                }
                Target::QuestLang { file } => {
                    let compound = quest_compound(&unit.entries, &done);
                    let path = match file {
                        None => format!("config/ftbquests/quests/lang/{}.snbt", self.locale),
                        Some(rel) => format!("config/ftbquests/quests/lang/{}/{rel}", self.locale),
                    };
                    outputs.files.insert(path, snbt::write_compound(&compound));
                }
                Target::QuestInline { file, original } => {
                    let edits = snbt::strings(original)
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, s)| {
                            done.get(i.to_string().as_str())
                                .map(|t| (s.start, s.end, (*t).to_owned()))
                        })
                        .collect();
                    outputs
                        .files
                        .insert(file.clone(), snbt::replace(original, edits));
                    outputs.replacing.insert(file.clone());
                }
                Target::Book { dest, original } => {
                    let Ok(mut json) = serde_json::from_str::<serde_json::Value>(
                        original.trim_start_matches('\u{feff}'),
                    ) else {
                        continue;
                    };
                    for (pointer, text) in &done {
                        if let Some(slot) = json.pointer_mut(pointer) {
                            *slot = serde_json::Value::String((*text).to_owned());
                        }
                    }
                    let content = serde_json::to_string_pretty(&json).unwrap_or_default();
                    match dest {
                        BookDest::Pack(path) => {
                            outputs
                                .pack
                                .insert(path.replace("{locale}", &self.locale), content);
                        }
                        BookDest::Loose(path) => {
                            outputs
                                .files
                                .insert(path.replace("{locale}", &self.locale), content);
                        }
                    }
                }
            }
        }

        let file_name = lang::file_name(&self.locale, format, &self.game_version);
        for (namespace, mut entries) in lang {
            entries.sort();
            outputs.pack.insert(
                format!("assets/{namespace}/lang/{file_name}"),
                lang::write(&entries, format),
            );
        }
        outputs
    }
}

/// Keys of a quest language file in order, list items regrouped; untranslated items of
/// a translated list keep their English text so the list stays whole.
fn quest_compound(
    entries: &[(String, String)],
    done: &HashMap<&str, &str>,
) -> Vec<(String, Vec<String>)> {
    let mut order: Vec<String> = Vec::new();
    let mut values: HashMap<String, Vec<(String, bool)>> = HashMap::new();
    for (key, english) in entries {
        let base = key.rsplit_once('#').map_or(key.as_str(), |(b, i)| {
            if i.chars().all(|c| c.is_ascii_digit()) {
                b
            } else {
                key.as_str()
            }
        });
        if !values.contains_key(base) {
            order.push(base.to_owned());
        }
        let translated = done.get(key.as_str());
        values.entry(base.to_owned()).or_default().push((
            translated.map_or(english.clone(), |t| (*t).to_owned()),
            translated.is_some(),
        ));
    }
    order
        .into_iter()
        .filter_map(|key| {
            let list = values.remove(&key)?;
            list.iter()
                .any(|(_, t)| *t)
                .then(|| (key, list.into_iter().map(|(v, _)| v).collect()))
        })
        .collect()
}

/// Names first (what players see most), then quests, interface texts, books.
fn priority(kind: SourceKind, target: &Target, key: &str) -> u8 {
    const NAMES: &[&str] = &[
        "item.",
        "block.",
        "entity.",
        "effect.",
        "enchantment.",
        "biome.",
        "fluid.",
        "tile.",
    ];
    match (kind, target) {
        (_, Target::Lang { .. })
            if NAMES.iter().any(|p| key.starts_with(p))
                && !key.contains(".tooltip")
                && !key.contains(".desc") =>
        {
            0
        }
        (SourceKind::Quests, _) => 1,
        (_, Target::Lang { .. }) => 2,
        _ => 3,
    }
}

/// A distinct English text to translate.
#[derive(Debug, Clone)]
pub struct Job {
    pub english: String,
    /// Mod or pack name, given to the model as context.
    pub context: String,
    pub priority: u8,
    /// Texts that differ from `english` only by their numbers: translated along with it.
    pub variants: Vec<String>,
    /// The text's language key (`block.cobblemon.bug_gem_block`), a hint of what it is.
    pub hint: String,
}

impl Job {
    /// Texts this job translates.
    fn size(&self) -> usize {
        1 + self.variants.len()
    }

    fn masked(&self) -> mask::Masked {
        if self.variants.is_empty() {
            mask::mask(&self.english)
        } else {
            mask::mask_numbers(&self.english)
        }
    }
}

fn estimate(jobs: &[Job]) -> Estimate {
    let characters: usize = jobs.iter().map(|j| j.english.chars().count()).sum();
    let batches = jobs.len().div_ceil(BATCH_ENTRIES) as u64;
    let text_tokens = (characters as f64 / CHARS_PER_TOKEN).ceil() as u64;
    Estimate {
        texts: jobs.len(),
        characters,
        // Instructions and glossary come with every batch.
        input_tokens: text_tokens + batches * 450,
        // Translations run longer than English, plus the JSON around them.
        output_tokens: (text_tokens as f64 * 1.3) as u64 + batches * 40,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewEntry {
    pub key: String,
    pub english: String,
    pub translation: Option<String>,
    /// Already translated by the mod's authors (shown, not editable).
    pub by_authors: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done: usize,
    pub total: usize,
    pub failed: usize,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport {
    pub progress: Progress,
    pub cancelled: bool,
    /// Why the run stopped early, if it did.
    pub error: Option<String>,
}

/// Where and how a run translates.
pub struct Translator<'a> {
    pub db: &'a crate::db::Database,
    pub client: &'a Client,
    pub locale: &'a str,
    pub glossary: &'a Glossary,
    /// Requests sent at once.
    pub concurrency: usize,
}

/// Translates `jobs` and saves each batch as it comes. Stops early when cancelled or
/// when batches keep failing; what was translated is kept either way.
pub async fn translate(
    translator: &Translator<'_>,
    jobs: Vec<Job>,
    cancel: Arc<AtomicBool>,
    on_progress: impl Fn(Progress) + Send + Sync,
) -> RunReport {
    let Translator {
        db,
        client,
        locale,
        glossary,
        concurrency,
    } = *translator;
    let started = Instant::now();
    let language = lang::english_name(locale);
    let system = llm::system_prompt(language);
    let progress = std::sync::Mutex::new(Progress {
        total: jobs.iter().map(Job::size).sum(),
        ..Default::default()
    });
    let failed_in_a_row = std::sync::atomic::AtomicUsize::new(0);
    let first_error: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
    let stop = AtomicBool::new(false);

    let batches = make_batches(jobs, BATCH_ENTRIES, BATCH_CHARS);
    let run_batch = |batch: Vec<Job>| {
        let (system, progress, failed_in_a_row, first_error, stop, cancel) = (
            &system,
            &progress,
            &failed_in_a_row,
            &first_error,
            &stop,
            &cancel,
        );
        let on_progress = &on_progress;
        async move {
            if cancel.load(Ordering::Relaxed) || stop.load(Ordering::Relaxed) {
                return;
            }
            let (translated, failures, usage) =
                match translate_batch(client, system, glossary, &batch).await {
                    Ok(result) => {
                        failed_in_a_row.store(0, Ordering::Relaxed);
                        result
                    }
                    Err(err) => {
                        let mut first = first_error.lock().unwrap_or_else(|e| e.into_inner());
                        first.get_or_insert_with(|| err.to_string());
                        if failed_in_a_row.fetch_add(1, Ordering::Relaxed) + 1 >= MAX_FAILED_BATCHES
                        {
                            stop.store(true, Ordering::Relaxed);
                        }
                        (Vec::new(), batch.iter().map(Job::size).sum(), (0, 0))
                    }
                };
            if !translated.is_empty() {
                if let Err(err) = store::save_ai(db, locale, client.model(), &translated).await {
                    tracing::warn!(%err, "could not save translations");
                }
            }
            let snapshot = {
                let mut p = progress.lock().unwrap_or_else(|e| e.into_inner());
                p.done += translated.len();
                p.failed += failures;
                p.prompt_tokens += usage.0;
                p.completion_tokens += usage.1;
                p.elapsed_ms = started.elapsed().as_millis() as u64;
                *p
            };
            on_progress(snapshot);
        }
    };
    stream::iter(batches)
        .map(run_batch)
        .buffer_unordered(concurrency.max(1))
        .collect::<Vec<()>>()
        .await;

    let progress = *progress.lock().unwrap_or_else(|e| e.into_inner());
    let error = if stop.load(Ordering::Relaxed) {
        first_error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    } else {
        None
    };
    RunReport {
        progress,
        cancelled: cancel.load(Ordering::Relaxed),
        error,
    }
}

/// Groups jobs in order, never mixing contexts in one batch.
fn make_batches(jobs: Vec<Job>, max_entries: usize, max_chars: usize) -> Vec<Vec<Job>> {
    let mut batches: Vec<Vec<Job>> = Vec::new();
    let mut chars = 0;
    for job in jobs {
        let len = job.english.chars().count();
        let fits = batches.last().is_some_and(|b| {
            b.len() < max_entries
                && chars + len <= max_chars
                && b[0].context == job.context
                && b[0].priority == job.priority
        });
        if fits {
            chars += len;
            if let Some(last) = batches.last_mut() {
                last.push(job);
            }
        } else {
            chars = len;
            batches.push(vec![job]);
        }
    }
    batches
}

type BatchResult = (Vec<(String, String)>, usize, (u64, u64));

/// One request, plus a second one for texts the model got wrong. Returns the
/// translations, how many failed, and the tokens used.
async fn translate_batch(
    client: &Client,
    system: &str,
    glossary: &Glossary,
    batch: &[Job],
) -> Result<BatchResult> {
    let (mut translated, retry, mut usage) = attempt(client, system, glossary, batch).await?;
    let mut failed = 0;
    if !retry.is_empty() {
        // Smaller requests for what failed: models stumble less on fewer texts.
        for chunk in retry.chunks(5) {
            match attempt(client, system, glossary, chunk).await {
                Ok((more, still, used)) => {
                    translated.extend(more);
                    failed += still.iter().map(Job::size).sum::<usize>();
                    usage.0 += used.0;
                    usage.1 += used.1;
                }
                Err(_) => failed += chunk.iter().map(Job::size).sum::<usize>(),
            }
        }
    }
    Ok((translated, failed, usage))
}

type Attempt = (Vec<(String, String)>, Vec<Job>, (u64, u64));

async fn attempt(
    client: &Client,
    system: &str,
    glossary: &Glossary,
    batch: &[Job],
) -> Result<Attempt> {
    let masked: Vec<mask::Masked> = batch.iter().map(Job::masked).collect();
    // Language keys make the best ids: they tell the model what each text is.
    let mut ids = HashSet::new();
    let texts: Vec<(String, String)> = masked
        .iter()
        .zip(batch)
        .enumerate()
        .map(|(i, (m, job))| {
            let id =
                if !job.hint.is_empty() && job.hint.len() <= 80 && ids.insert(job.hint.as_str()) {
                    job.hint.clone()
                } else {
                    format!("#{}", i + 1)
                };
            (id, m.text.clone())
        })
        .collect();
    let hits = glossary.hits(&batch.iter().map(|j| j.english.as_str()).collect::<Vec<_>>());
    let user = llm::user_prompt(&batch[0].context, &hits, &texts);

    let mut delay = Duration::from_secs(2);
    let completion = loop {
        match client.complete(system, &user, true).await {
            Ok(c) => break c,
            Err(err) if llm::is_transient(&err) && delay <= Duration::from_secs(16) => {
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
            Err(err) => return Err(err),
        }
    };
    let answers = llm::parse_answer(&completion.text);
    let mut translated = Vec::new();
    let mut retry = Vec::new();
    for ((id, _), (job, m)) in texts.iter().zip(batch.iter().zip(&masked)) {
        let result = answers
            .get(id)
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .and_then(|t| mask::unmask(t, &m.codes))
            .filter(|t| !(t == &job.english && job.english.split_whitespace().count() >= 4));
        match result {
            Some(text) => {
                let answer = answers.get(id).map(|t| t.trim()).unwrap_or_default();
                for variant in &job.variants {
                    if let Some(own) = mask::unmask(answer, &mask::mask_numbers(variant).codes) {
                        translated.push((variant.clone(), own));
                    }
                }
                translated.push((job.english.clone(), text));
            }
            None => retry.push(job.clone()),
        }
    }
    Ok((
        translated,
        retry,
        (completion.prompt_tokens, completion.completion_tokens),
    ))
}

/// Translates a sanitized HTML description line by line, tags protected. Lines already
/// translated come from the cache. The result is sanitized again.
pub async fn translate_html(
    db: &crate::db::Database,
    client: &Client,
    locale: &str,
    context: &str,
    html: &str,
) -> Result<String> {
    let cache = store::load(db, locale).await?;
    let lines: Vec<&str> = html.lines().collect();
    let mut todo: Vec<Job> = Vec::new();
    let mut seen = HashSet::new();
    for line in &lines {
        let text = strip_tags(line);
        if mask::has_words(&text)
            && !cache.contains_key(&store::source_hash(line))
            && seen.insert(*line)
        {
            todo.push(Job {
                english: (*line).to_owned(),
                context: context.to_owned(),
                priority: 0,
                variants: Vec::new(),
                hint: String::new(),
            });
        }
    }
    if !todo.is_empty() {
        let system = llm::system_prompt(lang::english_name(locale));
        let glossary = Glossary::default();
        let mut translated = Vec::new();
        for batch in make_batches(todo, 12, 2500) {
            let masked: Vec<mask::Masked> =
                batch.iter().map(|j| mask::mask_html(&j.english)).collect();
            let texts: Vec<(String, String)> = masked
                .iter()
                .enumerate()
                .map(|(i, m)| ((i + 1).to_string(), m.text.clone()))
                .collect();
            let user = llm::user_prompt(context, &glossary.hits(&[]), &texts);
            let completion = client.complete(&system, &user, true).await?;
            let answers = llm::parse_answer(&completion.text);
            for ((id, _), (job, m)) in texts.iter().zip(batch.iter().zip(&masked)) {
                if let Some(text) = answers
                    .get(id)
                    .and_then(|t| mask::unmask(t.trim(), &m.codes))
                {
                    translated.push((job.english.clone(), text));
                }
            }
        }
        store::save_ai(db, locale, client.model(), &translated).await?;
    }
    let cache = store::load(db, locale).await?;
    let out: Vec<String> = lines
        .iter()
        .map(|line| {
            cache
                .get(&store::source_hash(line))
                .cloned()
                .unwrap_or_else(|| (*line).to_owned())
        })
        .collect();
    Ok(crate::content::markdown::sanitize(&out.join("\n")))
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// `lang` from an instance's `options.txt`.
pub fn game_language(game_dir: &Path) -> Option<String> {
    std::fs::read_to_string(game_dir.join("options.txt"))
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("lang:").map(|v| v.trim().to_lowercase()))
}

/// Model-less check that a provider answers, for the settings page.
pub async fn check_provider(client: &Client) -> Result<Vec<String>> {
    let models = client.models().await?;
    if models.is_empty() {
        return Err(Error::Translation(
            "Le service ne propose aucun modèle.".into(),
        ));
    }
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sources::{Source, Unit};

    fn source(id: &str, kind: SourceKind, units: Vec<Unit>) -> Source {
        Source {
            id: id.into(),
            name: id.into(),
            kind,
            units,
        }
    }

    fn lang_unit(ns: &str, entries: &[(&str, &str)]) -> Unit {
        Unit {
            target: Target::Lang {
                namespace: ns.into(),
            },
            entries: entries
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
        }
    }

    fn scan() -> Scan {
        let mut scan = Scan {
            format: Some(Format::Json),
            ..Default::default()
        };
        scan.sources.push(source(
            "mod:a.jar",
            SourceKind::Mod,
            vec![lang_unit(
                "a",
                &[
                    ("item.a.gear", "Iron Gear"),
                    ("a.tip", "Hold shift to see more"),
                    ("a.num", "%s / %s"),
                    ("a.copy", "This text was copied"),
                    ("a.over", "Old text"),
                ],
            )],
        ));
        // A pack overriding one key of the mod.
        scan.sources.push(source(
            "pack:p.zip",
            SourceKind::ResourcePack,
            vec![lang_unit("a", &[("a.over", "New text")])],
        ));
        scan.sources.push(source(
            "quests",
            SourceKind::Quests,
            vec![Unit {
                target: Target::QuestInline {
                    file: "config/ftbquests/quests/chapters/c.snbt".into(),
                    original: "{ title: \"Begin\" description: [\"Go\", \"Run fast\"] id: \"X\" }"
                        .into(),
                },
                entries: vec![
                    ("0".into(), "Begin".into()),
                    ("2".into(), "Run fast".into()),
                ],
            }],
        ));
        scan.existing.insert("a.tip".into(), "Maintenir Maj".into());
        scan.existing
            .insert("a.copy".into(), "This text was copied".into());
        scan
    }

    #[test]
    fn plans_and_counts() {
        let cache: HashMap<String, String> = [(
            store::source_hash("Iron Gear"),
            "Engrenage de fer".to_owned(),
        )]
        .into();
        let plan = Plan::new(Arc::new(scan()), &cache, HashSet::new(), "1.20.1", "fr_fr");
        let overview = plan.overview(None, Some("en_us".into()));
        assert_eq!(
            overview.totals,
            Counts {
                total: 6,
                existing: 1,
                translated: 1,
                missing: 4
            }
        );
        let mod_a = &overview.sources[0];
        // `a.over` belongs to the pack, `a.num` has no words.
        assert_eq!(mod_a.counts.total, 3);
        assert_eq!(overview.quests, Some(QuestMode::InPlace));
        let all = plan.jobs();
        let jobs: Vec<&str> = all.iter().map(|j| j.english.as_str()).collect();
        assert_eq!(jobs[..2], ["Begin", "Run fast"][..]);
        assert!(
            jobs.contains(&"New text")
                && jobs.contains(&"This text was copied")
                && !jobs.contains(&"Old text")
        );
        assert!(overview.estimate.input_tokens > 0);
    }

    #[test]
    fn excluded_sources_are_left_out() {
        let plan = Plan::new(
            Arc::new(scan()),
            &HashMap::new(),
            ["quests".to_owned()].into(),
            "1.20.1",
            "fr_fr",
        );
        assert!(plan.jobs().iter().all(|j| j.context != "quests"));
        let overview = plan.overview(None, None);
        assert!(
            overview
                .sources
                .iter()
                .find(|s| s.id == "quests")
                .unwrap()
                .excluded
        );
    }

    #[test]
    fn builds_outputs() {
        let cache: HashMap<String, String> = [
            (
                store::source_hash("Iron Gear"),
                "Engrenage de fer".to_owned(),
            ),
            (store::source_hash("Run fast"), "Cours vite".to_owned()),
        ]
        .into();
        let plan = Plan::new(Arc::new(scan()), &cache, HashSet::new(), "1.20.1", "fr_fr");
        let outputs = plan.outputs();
        let lang = &outputs.pack["assets/a/lang/fr_fr.json"];
        assert!(lang.contains("\"item.a.gear\": \"Engrenage de fer\""));
        let quests = &outputs.files["config/ftbquests/quests/chapters/c.snbt"];
        assert_eq!(
            quests,
            "{ title: \"Begin\" description: [\"Go\", \"Cours vite\"] id: \"X\" }"
        );
        assert!(outputs
            .replacing
            .contains("config/ftbquests/quests/chapters/c.snbt"));
    }

    #[test]
    fn regroups_quest_lists() {
        let entries: Vec<(String, String)> = vec![
            ("quest.1.title".into(), "Start".into()),
            ("quest.1.quest_desc#0".into(), "One".into()),
            ("quest.1.quest_desc#1".into(), "Two".into()),
            ("quest.2.title".into(), "Untouched".into()),
        ];
        let done: HashMap<&str, &str> = [("quest.1.quest_desc#1", "Deux")].into();
        assert_eq!(
            quest_compound(&entries, &done),
            [(
                "quest.1.quest_desc".to_owned(),
                vec!["One".to_owned(), "Deux".to_owned()]
            )]
        );
    }

    #[test]
    fn groups_numbered_variants() {
        let mut scan = Scan {
            format: Some(Format::Json),
            ..Default::default()
        };
        scan.sources.push(source(
            "mod:c.jar",
            SourceKind::Mod,
            vec![lang_unit(
                "c",
                &[
                    ("block.c.p1", "Oak Pattern 1"),
                    ("block.c.p2", "Oak Pattern 2"),
                    ("block.c.p12", "Oak Pattern 12"),
                    ("block.c.mk2", "Drill MK2"),
                ],
            )],
        ));
        let plan = Plan::new(
            Arc::new(scan),
            &HashMap::new(),
            HashSet::new(),
            "1.20.1",
            "fr_fr",
        );
        let jobs = plan.jobs();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].english, "Oak Pattern 1");
        assert_eq!(jobs[0].variants, ["Oak Pattern 2", "Oak Pattern 12"]);
        assert!(jobs[1].variants.is_empty());
        assert_eq!(estimate(&jobs).texts, 2);
    }

    #[test]
    fn batches_by_context_and_size() {
        let job = |text: &str, context: &str| Job {
            english: text.into(),
            context: context.into(),
            priority: 0,
            variants: Vec::new(),
            hint: String::new(),
        };
        let jobs = vec![
            job("a", "x"),
            job("b", "x"),
            job("c", "y"),
            job("dddd", "y"),
            job("e", "y"),
        ];
        let batches = make_batches(jobs, 2, 4);
        let sizes: Vec<usize> = batches.iter().map(Vec::len).collect();
        assert_eq!(sizes, [2, 1, 1, 1]);
    }
}
