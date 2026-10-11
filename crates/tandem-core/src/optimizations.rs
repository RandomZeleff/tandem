//! Tandem Optimizations: a small server-side mod Tandem adds at launch (D42), never
//! written into the instance. Sources and build: `mods/tandem-optimizations`.
//!
//! Fabric loads it from `-Dfabric.addMods` (Quilt from `-Dloader.addMods`); its options
//! are system properties, so the player's own JVM arguments (passed after ours) can
//! override them.

use std::path::PathBuf;

use crate::error::Result;
use crate::instance::Instance;
use crate::meta::loader::Loader;
use crate::Context;

/// Built by `mods/tandem-optimizations/build.py`, checked against 1.20 to 1.21.11.
const FABRIC_JAR: &[u8] =
    include_bytes!("../../../mods/tandem-optimizations/dist/tandem-optimizations-fabric.jar");
const FABRIC_JAR_NAME: &str = "tandem-optimizations-fabric-1.0.0.jar";

/// On for every instance unless turned off in the Settings.
pub const SETTING: &str = "tandem_optimizations";
/// Mobs far from every player tick less often (the only optimization with a visible
/// effect: idle mobs far away move slower).
pub const ACTIVATION_SETTING: &str = "tandem_optimizations_activation";

/// Per-instance override of [`SETTING`].
pub fn instance_setting(instance_id: &str) -> String {
    format!("{SETTING}.{instance_id}")
}

/// Fabric and Quilt, Minecraft 1.20 to 1.21.11 (releases only). From 26.1 the game is
/// no longer obfuscated and Fabric has no intermediary names: another jar (shared with
/// NeoForge, which also runs Mojang's names) will be needed.
pub fn supports(loader: Loader, game_version: &str) -> bool {
    if !matches!(loader, Loader::Fabric | Loader::Quilt) {
        return false;
    }
    let mut parts = game_version.split('.');
    let (Some("1"), Some(minor)) = (parts.next(), parts.next()) else {
        return false;
    };
    let patch = match parts.next() {
        None => Some(0),
        Some(p) => p.parse::<u32>().ok(),
    };
    match (minor, patch, parts.next()) {
        ("20", Some(_), None) => true,
        ("21", Some(patch), None) => patch <= 11,
        _ => false,
    }
}

/// JVM arguments that add the mod to this launch, empty when it does not apply.
pub async fn jvm_args(ctx: &Context, instance: &Instance) -> Result<Vec<String>> {
    if !supports(instance.loader, &instance.game_version) {
        return Ok(Vec::new());
    }
    let enabled = match ctx
        .db
        .get_setting::<bool>(&instance_setting(&instance.id))
        .await?
    {
        Some(on) => on,
        None => ctx.db.get_setting::<bool>(SETTING).await?.unwrap_or(true),
    };
    if !enabled {
        return Ok(Vec::new());
    }
    let activation = ctx
        .db
        .get_setting::<bool>(ACTIVATION_SETTING)
        .await?
        .unwrap_or(true);
    let jar = write_jar(ctx).await?;
    let property = match instance.loader {
        Loader::Quilt => "loader.addMods",
        _ => "fabric.addMods",
    };
    let mut args = vec![format!("-D{property}={}", jar.display())];
    if !activation {
        args.push("-Dtandem.optimizations.activationRange=0".to_owned());
    }
    tracing::info!(instance = %instance.id, activation, "Tandem Optimizations added");
    Ok(args)
}

/// The embedded jar in the data folder, rewritten if missing or different.
async fn write_jar(ctx: &Context) -> Result<PathBuf> {
    let dir = ctx.data.root().join("mods");
    let path = dir.join(FABRIC_JAR_NAME);
    let current = tokio::fs::read(&path).await.ok();
    if current.as_deref() != Some(FABRIC_JAR) {
        tokio::fs::create_dir_all(&dir).await?;
        let tmp = path.with_extension("jar.tmp");
        tokio::fs::write(&tmp, FABRIC_JAR).await?;
        tokio::fs::rename(&tmp, &path).await?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_versions() {
        for v in ["1.20", "1.20.1", "1.20.6", "1.21", "1.21.1", "1.21.11"] {
            assert!(supports(Loader::Fabric, v), "{v}");
            assert!(supports(Loader::Quilt, v), "{v}");
        }
        for v in [
            "1.19.4",
            "1.21.12",
            "26.1",
            "1.21.5-pre1",
            "24w14a",
            "1.20.1.1",
            "",
        ] {
            assert!(!supports(Loader::Fabric, v), "{v}");
        }
        for loader in [Loader::Vanilla, Loader::Forge, Loader::NeoForge] {
            assert!(!supports(loader, "1.21.1"));
        }
    }

    #[test]
    fn embedded_jar_is_a_fabric_mod() {
        // A zip with the mod's metadata.
        assert_eq!(&FABRIC_JAR[..2], b"PK");
        let text = String::from_utf8_lossy(FABRIC_JAR);
        assert!(text.contains("fabric.mod.json"));
    }

    #[tokio::test]
    async fn writes_the_jar_once() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = Context::init(crate::paths::DataDir::new(dir.path()))
            .await
            .unwrap();
        let path = write_jar(&ctx).await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), FABRIC_JAR);
        std::fs::write(&path, b"old").unwrap();
        assert_eq!(write_jar(&ctx).await.unwrap(), path);
        assert_eq!(std::fs::read(&path).unwrap(), FABRIC_JAR);
    }
}
