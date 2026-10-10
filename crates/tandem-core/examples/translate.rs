//! Dev tool: translate an instance the way the launcher does.
//!
//! cargo run -p tandem-core --example translate -- <instance-id> [locale] [max-texts] [apply]
//!
//! Scans and prints what there is to translate. With `max-texts` > 0, translates that
//! many missing texts with the provider saved in the launcher's settings (defaults to
//! Ollama with qwen2.5:7b). With `apply`, writes the result into the instance.
//! Uses `TANDEM_DATA_DIR` if set.

use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use tandem_core::paths::DataDir;
use tandem_core::translate::{self, glossary, llm, output, sources, store};
use tandem_core::Context;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("warn").init();
    let mut args = std::env::args().skip(1);
    let instance_id = args
        .next()
        .ok_or("usage: translate <instance-id> [locale] [max-texts] [apply]")?;
    let locale = args.next().unwrap_or_else(|| "fr_fr".into());
    let max: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(0);
    let apply = args.next().as_deref() == Some("apply");

    let ctx = Context::init(DataDir::from_env()?).await?;
    let instance = ctx.db.get_instance(&instance_id).await?;
    let game_dir = ctx.data.instance_dir(&instance.id);

    let started = Instant::now();
    let (data, id, dir, version, loc) = (
        ctx.data.clone(),
        instance.id.clone(),
        game_dir.clone(),
        instance.game_version.clone(),
        locale.clone(),
    );
    let scan = tokio::task::spawn_blocking(move || sources::scan(&data, &id, &dir, &version, &loc))
        .await?;
    println!(
        "scan: {} sources in {:?}",
        scan.sources.len(),
        started.elapsed()
    );

    let cache = store::load(&ctx.db, &locale).await?;
    let plan = translate::Plan::new(
        Arc::new(scan),
        &cache,
        HashSet::new(),
        &instance.game_version,
        &locale,
    );
    let overview = plan.overview(None, translate::game_language(&game_dir));
    println!("totals: {:?}", overview.totals);
    println!("estimate: {:?}", overview.estimate);
    println!("quests: {:?}", overview.quests);
    let mut sources = overview.sources.clone();
    sources.sort_by_key(|s| std::cmp::Reverse(s.counts.missing));
    for s in sources.iter().take(12) {
        println!(
            "  {:>6} missing / {:>6}  {:?} {}",
            s.counts.missing, s.counts.total, s.kind, s.name
        );
    }

    if max > 0 {
        let config = ctx
            .db
            .get_setting::<llm::ProviderConfig>("translation.provider")
            .await?
            .unwrap_or(llm::ProviderConfig {
                preset: "ollama".into(),
                base_url: "http://localhost:11434/v1".into(),
                model: "qwen2.5:7b".into(),
                concurrency: 1,
            });
        let client = llm::Client::new(ctx.http.clone(), &config, None)?;
        let started = Instant::now();
        let vanilla = glossary::vanilla(&ctx, &instance.game_version, &locale).await;
        let authors = plan.author_terms();
        println!("author terms: {}", authors.len());
        let glossary = glossary::Glossary::new(vanilla.into_iter().chain(authors));
        println!(
            "glossary: {} terms in {:?}",
            glossary.len(),
            started.elapsed()
        );

        let jobs: Vec<_> = plan.jobs().into_iter().take(max).collect();
        println!("translating {} texts with {} …", jobs.len(), config.model);
        let started = Instant::now();
        let translator = translate::Translator {
            db: &ctx.db,
            client: &client,
            locale: &locale,
            glossary: &glossary,
            concurrency: config.concurrency.max(1) as usize,
        };
        let report =
            translate::translate(&translator, jobs, Arc::new(AtomicBool::new(false)), |p| {
                eprint!(
                    "\r  {}/{} done, {} failed, {} tok out, {:.0}s",
                    p.done,
                    p.total,
                    p.failed,
                    p.completion_tokens,
                    p.elapsed_ms as f64 / 1000.0
                );
            })
            .await;
        eprintln!();
        println!("report: {:?} in {:?}", report, started.elapsed());
        let tok_s = report.progress.completion_tokens as f64 / started.elapsed().as_secs_f64();
        println!("{tok_s:.1} output tokens/s");
    }

    if apply {
        let cache = store::load(&ctx.db, &locale).await?;
        let (data, id, dir, version, loc) = (
            ctx.data.clone(),
            instance.id.clone(),
            game_dir.clone(),
            instance.game_version.clone(),
            locale.clone(),
        );
        let scan =
            tokio::task::spawn_blocking(move || sources::scan(&data, &id, &dir, &version, &loc))
                .await?;
        let plan = translate::Plan::new(
            Arc::new(scan),
            &cache,
            HashSet::new(),
            &instance.game_version,
            &locale,
        );
        let outputs = plan.outputs();
        println!(
            "writing {} pack files, {} instance files",
            outputs.pack.len(),
            outputs.files.len()
        );
        output::apply(&game_dir, &instance.game_version, &locale, &outputs, true)?;
        println!("applied");
    }
    Ok(())
}
