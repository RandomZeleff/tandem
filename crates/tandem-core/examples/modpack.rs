//! Dev tool: install a Modrinth modpack (or a `.mrpack` file) into a new instance,
//! then export it back to `<data>/cache/<instance>.mrpack`.
//!
//! cargo run -p tandem-core --example modpack -- <project-slug | path/to/pack.mrpack>
//!
//! Launch the result with `cargo run -p tandem-core --example play -- instance:<id>`.
//! Uses `TANDEM_DATA_DIR` if set.

use std::path::PathBuf;

use tandem_core::content::mrpack;
use tandem_core::instance::{self, PackOrigin};
use tandem_core::paths::DataDir;
use tandem_core::Context;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let source = std::env::args()
        .nth(1)
        .ok_or("usage: modpack <project-slug | file.mrpack>")?;
    let ctx = Context::init(DataDir::from_env()?).await?;

    let (path, name, origin) = if source.ends_with(".mrpack") {
        (PathBuf::from(&source), None, None)
    } else {
        let pack = mrpack::download(&ctx, &source, None).await?;
        let origin = PackOrigin {
            project_id: pack.project.id.clone(),
            version_id: pack.version.id.clone(),
            version: pack.version.version_number.clone(),
            icon: pack.project.icon_url.clone(),
        };
        (pack.path, Some(pack.project.title), Some(origin))
    };
    let index = mrpack::read_index(&path).await?;
    println!(
        "{} {} ({} files, {:?})",
        index.name,
        index.version_id,
        index.files.len(),
        index.dependencies
    );

    let created = instance::create(&ctx, index.new_instance(name.as_deref())?).await?;
    if let Some(origin) = &origin {
        ctx.db.set_instance_pack(&created.id, origin).await?;
    }
    let last = std::sync::Mutex::new(0usize);
    mrpack::install(&ctx, &created, &path, &index, |p| {
        let mut last = last.lock().unwrap();
        if p.download.done_files != *last && p.download.done_files % 25 == 0 {
            *last = p.download.done_files;
            println!(
                "{:?} {}/{} files",
                p.stage, p.download.done_files, p.download.total_files
            );
        }
    })
    .await?;
    let content = ctx.db.list_content(&created.id).await?;
    println!(
        "instance {} ready, {} files tracked as content",
        created.id,
        content.len()
    );

    let export = ctx.data.cache().join(format!("{}.mrpack", created.id));
    mrpack::export(&ctx, &created, &export, "export-test").await?;
    let exported = mrpack::read_index(&export).await?;
    println!(
        "exported {} ({} files referenced, {:?})",
        export.display(),
        exported.files.len(),
        exported.dependencies
    );
    Ok(())
}
