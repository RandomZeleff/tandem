//! Dev tool: install a version of a Modrinth modpack, or update / roll back an
//! instance made from one.
//!
//! cargo run -p tandem-core --example pack_update -- install <slug> <version-id>
//! cargo run -p tandem-core --example pack_update -- update <instance-id> [version-id]
//! cargo run -p tandem-core --example pack_update -- rollback <instance-id>
//!
//! `update` without a version takes the newest one. Uses `TANDEM_DATA_DIR` if set.

use tandem_core::content::{mrpack, pack_update};
use tandem_core::instance::{self, PackOrigin};
use tandem_core::paths::DataDir;
use tandem_core::Context;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let ctx = Context::init(DataDir::from_env()?).await?;
    match args.first().map(String::as_str) {
        Some("install") => {
            let (slug, version) = (args.get(1).ok_or("slug")?, args.get(2).ok_or("version id")?);
            let pack = mrpack::download(&ctx, slug, Some(version)).await?;
            let index = mrpack::read_index(&pack.path).await?;
            let created =
                instance::create(&ctx, index.new_instance(Some(&pack.project.title))?).await?;
            ctx.db
                .set_instance_pack(
                    &created.id,
                    &PackOrigin {
                        project_id: pack.project.id.clone(),
                        version_id: pack.version.id.clone(),
                        version: pack.version.version_number.clone(),
                        icon: pack.project.icon_url.clone(),
                    },
                )
                .await?;
            mrpack::install(&ctx, &created, &pack.path, &index, |_| {}).await?;
            println!(
                "installed {} {} as {}",
                index.name, index.version_id, created.id
            );
        }
        Some("update") => {
            let instance = ctx
                .db
                .get_instance(args.get(1).ok_or("instance id")?)
                .await?;
            let newer = pack_update::newer_versions(&ctx, &instance).await?;
            println!("{} newer versions", newer.len());
            let version = match args.get(2) {
                Some(v) => v.clone(),
                None => newer.first().ok_or("already up to date")?.id.clone(),
            };
            let project = instance
                .pack_project_id
                .clone()
                .ok_or("not a Modrinth pack")?;
            let pack = mrpack::download(&ctx, &project, Some(&version)).await?;
            let origin = PackOrigin {
                project_id: pack.project.id.clone(),
                version_id: pack.version.id.clone(),
                version: pack.version.version_number.clone(),
                icon: pack.project.icon_url.clone(),
            };
            let started = std::time::Instant::now();
            let report =
                pack_update::update(&ctx, &instance, &pack.path, Some(origin), |_| {}).await?;
            println!("{report:?} in {:?}", started.elapsed());
            let after = ctx.db.get_instance(&instance.id).await?;
            println!(
                "now {} {:?} {:?} pack {:?}",
                after.game_version, after.loader, after.loader_version, after.pack_version
            );
        }
        Some("rollback") => {
            let instance = ctx
                .db
                .get_instance(args.get(1).ok_or("instance id")?)
                .await?;
            pack_update::rollback(&ctx, &instance).await?;
            let after = ctx.db.get_instance(&instance.id).await?;
            println!(
                "rolled back to {} {:?} pack {:?}",
                after.game_version, after.loader_version, after.pack_version
            );
        }
        _ => return Err("usage: pack_update install|update|rollback …".into()),
    }
    Ok(())
}
