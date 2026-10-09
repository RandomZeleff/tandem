//! Crash analysis: what went wrong and which mod is the likely culprit, read from the
//! crash report (or the game log when the game died before writing one).

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A mod jar of the instance, with what is needed to recognise it in a stack trace.
#[derive(Debug, Clone, Default)]
pub struct ModInfo {
    pub id: Option<String>,
    pub name: String,
    pub file_name: String,
    /// Java packages (first three segments) of its classes.
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SuspectReason {
    /// Named by Forge / NeoForge in the report.
    NamedByLoader,
    /// A Mixin from this mod failed to apply.
    Mixin,
    /// Fabric refused to start because of this mod's requirements.
    Incompatible,
    /// First mod code found in the stack trace.
    StackTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suspect {
    pub name: String,
    /// Jar in `mods/`, when the mod could be matched to one.
    pub file_name: Option<String>,
    pub reason: SuspectReason,
}

/// Well-known causes with a fix the player can apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Hint {
    OutOfMemory,
    WrongJava,
    Graphics,
    IncompatibleMods,
    NativeCrash,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashAnalysis {
    /// The report's `Description:` line.
    pub description: Option<String>,
    /// First exception line, e.g. `java.lang.NullPointerException: …`.
    pub exception: Option<String>,
    pub suspects: Vec<Suspect>,
    pub hint: Option<Hint>,
    /// File the analysis was read from.
    pub source: Option<PathBuf>,
}

/// Frames from these packages are never blamed: the game, the JVM, the loaders.
const PLATFORM_PACKAGES: &[&str] = &[
    "java.",
    "javax.",
    "jdk.",
    "sun.",
    "com.sun.",
    "net.minecraft.",
    "com.mojang.",
    "org.lwjgl.",
    "net.fabricmc.",
    "org.quiltmc.",
    "net.minecraftforge.",
    "net.neoforged.",
    "cpw.mods.",
    "org.spongepowered.",
    "com.google.",
    "io.netty.",
    "org.apache.",
    "it.unimi.",
    "oshi.",
    "com.llamalad7.",
    "org.objectweb.",
    "kotlin.",
    "scala.",
    "org.slf4j.",
];

/// Reads what the last run left behind (crash report, else native crash log, else
/// `logs/latest.log`) and analyses it against the instance's mods.
pub fn analyze_exit(game_dir: &Path, crash_report: Option<&Path>) -> CrashAnalysis {
    let native = newest_file(game_dir, |name| name.starts_with("hs_err_pid"));
    let source = crash_report
        .map(Path::to_path_buf)
        .or(native.clone())
        .unwrap_or_else(|| game_dir.join("logs").join("latest.log"));
    let text = std::fs::read(&source)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let mut analysis = analyze(&text, &scan_mods(game_dir));
    if crash_report.is_none() && native.is_some() {
        analysis.hint = Some(Hint::NativeCrash);
    }
    if !text.is_empty() {
        analysis.source = Some(source);
    }
    analysis
}

/// Pure analysis of a crash report or log.
pub fn analyze(text: &str, mods: &[ModInfo]) -> CrashAnalysis {
    let description = text
        .lines()
        .find_map(|l| l.strip_prefix("Description: "))
        .map(|d| d.trim().to_owned());
    let exception = text
        .lines()
        .map(str::trim)
        .find(|l| is_exception_line(l))
        .map(|l| truncate(l, 300));

    let mut suspects = Vec::new();
    let push = |suspects: &mut Vec<Suspect>, suspect: Suspect| {
        if !suspects
            .iter()
            .any(|s: &Suspect| s.name.eq_ignore_ascii_case(&suspect.name))
        {
            suspects.push(suspect);
        }
    };

    for (name, id) in loader_suspects(text) {
        let found = find_mod(mods, Some(&id), &name);
        push(
            &mut suspects,
            suspect(found, &name, SuspectReason::NamedByLoader),
        );
    }
    for id in mixin_mods(text) {
        let found = find_mod(mods, Some(&id), &id);
        push(&mut suspects, suspect(found, &id, SuspectReason::Mixin));
    }
    let incompatible = fabric_incompatible(text);
    for (name, id) in &incompatible {
        if id == "minecraft" || id == "java" || id == "fabricloader" {
            continue;
        }
        let found = find_mod(mods, Some(id), name);
        push(
            &mut suspects,
            suspect(found, name, SuspectReason::Incompatible),
        );
    }
    if suspects.is_empty() {
        if let Some(found) = stack_trace_mod(text, mods) {
            let name = found.name.clone();
            push(
                &mut suspects,
                suspect(Some(found), &name, SuspectReason::StackTrace),
            );
        }
    }

    CrashAnalysis {
        hint: hint(text, !incompatible.is_empty()),
        description,
        exception,
        suspects,
        source: None,
    }
}

fn suspect(found: Option<&ModInfo>, fallback: &str, reason: SuspectReason) -> Suspect {
    Suspect {
        name: found.map_or_else(|| fallback.to_owned(), |m| m.name.clone()),
        file_name: found.map(|m| m.file_name.clone()),
        reason,
    }
}

fn is_exception_line(line: &str) -> bool {
    let head = line.split(':').next().unwrap_or_default();
    !head.contains(' ')
        && head.contains('.')
        && (head.ends_with("Exception") || head.ends_with("Error"))
}

fn truncate(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_owned(),
    }
}

fn hint(text: &str, incompatible: bool) -> Option<Hint> {
    if text.contains("java.lang.OutOfMemoryError") {
        Some(Hint::OutOfMemory)
    } else if text.contains("UnsupportedClassVersionError")
        || text.contains("has been compiled by a more recent version of the Java Runtime")
    {
        Some(Hint::WrongJava)
    } else if text.contains("Pixel format not accelerated")
        || text.contains("GLFW error 65543")
        || text.contains("No OpenGL context found")
        || text.contains("Failed to create window")
    {
        Some(Hint::Graphics)
    } else if incompatible || text.contains("Incompatible mods found") {
        Some(Hint::IncompatibleMods)
    } else if text.contains("A fatal error has been detected by the Java Runtime Environment") {
        Some(Hint::NativeCrash)
    } else {
        None
    }
}

/// Forge / NeoForge: `Suspected Mod(s):` blocks (`Name (modid), Version: …`) and frames
/// tagged `TRANSFORMER/modid@version/`.
fn loader_suspects(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("Suspected Mods:")
            .or_else(|| trimmed.strip_prefix("Suspected Mod:"))
        {
            in_block = true;
            let rest = rest.trim();
            if !rest.is_empty() && rest != "None" {
                found.extend(parse_named_mod(rest));
            }
            continue;
        }
        if in_block {
            if trimmed.is_empty() || !line.starts_with('\t') && !line.starts_with(' ') {
                in_block = false;
            } else if let Some(m) = parse_named_mod(trimmed) {
                found.push(m);
            }
        }
    }
    for line in text.lines() {
        if let Some(i) = line.find("TRANSFORMER/") {
            let rest = &line[i + "TRANSFORMER/".len()..];
            if let Some((id, _)) = rest.split_once('@') {
                if !is_platform_mod(id) && !found.iter().any(|(_, f)| f == id) {
                    found.push((id.to_owned(), id.to_owned()));
                }
            }
        }
    }
    found
}

/// `Name (modid), Version: 1.0` → (Name, modid).
fn parse_named_mod(s: &str) -> Option<(String, String)> {
    let open = s.find(" (")?;
    let close = s[open..].find(')')? + open;
    let id = &s[open + 2..close];
    (!id.is_empty() && !id.contains(' ')).then(|| (s[..open].trim().to_owned(), id.to_owned()))
}

fn is_platform_mod(id: &str) -> bool {
    matches!(
        id,
        "minecraft" | "forge" | "neoforge" | "fml" | "mixin" | "java"
    )
}

/// Mod ids of failing Mixins: `Mixin [foo.mixins.json:Bar] from mod foo failed…`.
fn mixin_mods(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for line in text.lines() {
        if !line.contains("Mixin") {
            continue;
        }
        if let Some(i) = line.find("from mod ") {
            let id: String = line[i + 9..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if !id.is_empty() && !is_platform_mod(&id) && !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    ids
}

/// Fabric's "Some of your mods are incompatible" report: `Mod 'Name' (id) …` lines.
fn fabric_incompatible(text: &str) -> Vec<(String, String)> {
    let Some(start) = text
        .find("Some of your mods are incompatible")
        .or_else(|| text.find("Incompatible mods found"))
    else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for line in text[start..].lines().take(60) {
        let mut rest = line;
        while let Some(i) = rest.find("Mod '") {
            rest = &rest[i + 5..];
            let Some(end) = rest.find('\'') else { break };
            let name = &rest[..end];
            let after = &rest[end + 1..];
            if let Some(id) = after
                .trim_start()
                .strip_prefix('(')
                .and_then(|a| a.split_once(')'))
                .map(|(id, _)| id)
            {
                if !found.iter().any(|(_, f): &(String, String)| f == id) {
                    found.push((name.to_owned(), id.to_owned()));
                }
            }
            rest = after;
        }
    }
    found
}

/// First stack frame (`at some.package.Class.method`) that belongs to a mod jar.
fn stack_trace_mod<'a>(text: &str, mods: &'a [ModInfo]) -> Option<&'a ModInfo> {
    for line in text.lines() {
        let Some(frame) = line.trim().strip_prefix("at ") else {
            continue;
        };
        // Forge frames look like `TRANSFORMER/modid@1.0/pkg.Class.method(…)`.
        let frame = frame.rsplit('/').next().unwrap_or(frame);
        if PLATFORM_PACKAGES.iter().any(|p| frame.starts_with(p)) {
            continue;
        }
        if let Some(m) = mods
            .iter()
            .find(|m| m.packages.iter().any(|p| frame.starts_with(p.as_str())))
        {
            return Some(m);
        }
    }
    None
}

fn find_mod<'a>(mods: &'a [ModInfo], id: Option<&str>, name: &str) -> Option<&'a ModInfo> {
    id.and_then(|id| mods.iter().find(|m| m.id.as_deref() == Some(id)))
        .or_else(|| mods.iter().find(|m| m.name.eq_ignore_ascii_case(name)))
}

fn newest_file(dir: &Path, matches: impl Fn(&str) -> bool) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| matches(&e.file_name().to_string_lossy()))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

/// Enabled mod jars of an instance with their id, display name and packages.
pub fn scan_mods(game_dir: &Path) -> Vec<ModInfo> {
    let Ok(entries) = std::fs::read_dir(game_dir.join("mods")) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "jar"))
        .filter_map(|p| read_mod(&p).ok())
        .collect()
}

#[derive(Deserialize)]
struct FabricModJson {
    id: String,
    #[serde(default)]
    name: Option<String>,
}

fn read_mod(jar: &Path) -> std::io::Result<ModInfo> {
    let file_name = jar
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut archive =
        zip::ZipArchive::new(std::fs::File::open(jar)?).map_err(std::io::Error::other)?;
    let mut info = ModInfo {
        name: file_name.trim_end_matches(".jar").to_owned(),
        file_name,
        ..Default::default()
    };

    for meta in ["fabric.mod.json", "quilt.mod.json"] {
        if let Ok(mut entry) = archive.by_name(meta) {
            let mut json = String::new();
            entry.read_to_string(&mut json)?;
            if let Ok(parsed) = serde_json::from_str::<FabricModJson>(&json) {
                info.name = parsed.name.unwrap_or_else(|| parsed.id.clone());
                info.id = Some(parsed.id);
            } else if let Some(id) = quilt_id(&json) {
                info.name = id.clone();
                info.id = Some(id);
            }
            break;
        }
    }
    if info.id.is_none() {
        for meta in ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"] {
            if let Ok(mut entry) = archive.by_name(meta) {
                let mut toml = String::new();
                entry.read_to_string(&mut toml)?;
                info.id = toml_value(&toml, "modId");
                if let Some(name) = toml_value(&toml, "displayName") {
                    info.name = name;
                }
                break;
            }
        }
    }

    let mut packages = HashSet::new();
    for name in archive.file_names() {
        let Some(class) = name.strip_suffix(".class") else {
            continue;
        };
        if name.starts_with("META-INF/") {
            continue;
        }
        let segments: Vec<&str> = class.split('/').collect();
        if segments.len() >= 3 {
            packages.insert(format!("{}.", segments[..3].join(".")));
        }
    }
    info.packages = packages.into_iter().collect();
    Ok(info)
}

/// `"id": "x"` inside quilt.mod.json's `quilt_loader` block.
fn quilt_id(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    value["quilt_loader"]["id"].as_str().map(str::to_owned)
}

/// First `key = "value"` in a mods.toml (enough for modId / displayName).
fn toml_value(toml: &str, key: &str) -> Option<String> {
    toml.lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        if k.trim() != key {
            return None;
        }
        let v = v.trim().trim_matches('"').trim();
        (!v.is_empty() && !v.starts_with("${")).then(|| v.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sodium() -> ModInfo {
        ModInfo {
            id: Some("sodium".into()),
            name: "Sodium".into(),
            file_name: "sodium-0.6.13.jar".into(),
            packages: vec!["net.caffeinemc.mods.".into()],
        }
    }

    #[test]
    fn blames_first_mod_frame_in_a_vanilla_style_report() {
        let report = "---- Minecraft Crash Report ----\n\
            Description: Rendering overlay\n\n\
            java.lang.NullPointerException: Cannot invoke \"Object.hashCode()\"\n\
            \tat java.base/java.util.HashMap.hash(HashMap.java:338)\n\
            \tat net.minecraft.client.renderer.LevelRenderer.renderLevel(LevelRenderer.java:1)\n\
            \tat net.caffeinemc.mods.sodium.client.render.SodiumWorldRenderer.setup(SodiumWorldRenderer.java:5)\n";
        let a = analyze(report, &[sodium()]);
        assert_eq!(a.description.as_deref(), Some("Rendering overlay"));
        assert!(a
            .exception
            .unwrap()
            .starts_with("java.lang.NullPointerException"));
        assert_eq!(
            a.suspects,
            [Suspect {
                name: "Sodium".into(),
                file_name: Some("sodium-0.6.13.jar".into()),
                reason: SuspectReason::StackTrace
            }]
        );
    }

    #[test]
    fn reads_forge_suspected_mods_and_transformer_frames() {
        let report = "Description: Ticking entity\n\
            java.lang.IllegalStateException: boom\n\
            \tat TRANSFORMER/create@0.5.1/com.simibubi.create.Foo.tick(Foo.java:1)\n\
            Suspected Mods: \n\
            \tCreate (create), Version: 0.5.1\n\
            \t\tIssue tracker URL: https://example.org\n\
            Stacktrace:\n";
        let a = analyze(report, &[]);
        assert_eq!(a.suspects.len(), 1);
        assert_eq!(a.suspects[0].name, "Create");
        assert_eq!(a.suspects[0].reason, SuspectReason::NamedByLoader);
    }

    #[test]
    fn reads_fabric_incompatible_mods() {
        let log = "[main/ERROR]: Incompatible mods found!\n\
            net.fabricmc.loader.impl.FormattedException: Some of your mods are incompatible with the game or each other!\n\
            A potential solution has been determined, this may resolve your problem:\n\
            \t - Replace mod 'Sodium' (sodium) 0.5.3 with any version that is compatible with:\n\
            More details:\n\
            \t - Mod 'Sodium' (sodium) 0.5.3 requires version 1.20.1 of 'Minecraft' (minecraft), but only the wrong version is present: 1.21.4!\n";
        let a = analyze(log, &[sodium()]);
        assert_eq!(a.hint, Some(Hint::IncompatibleMods));
        assert_eq!(a.suspects.len(), 1);
        assert_eq!(a.suspects[0].reason, SuspectReason::Incompatible);
        assert_eq!(
            a.suspects[0].file_name.as_deref(),
            Some("sodium-0.6.13.jar")
        );
    }

    #[test]
    fn reads_mixin_failures_and_hints() {
        let log = "org.spongepowered.asm.mixin.transformer.throwables.MixinTransformerError: An unexpected critical error was encountered\n\
            Caused by: org.spongepowered.asm.mixin.injection.throwables.InjectionError: Critical injection failure: Mixin [lithium.mixins.json:ai.Foo] from mod lithium failed\n";
        let a = analyze(log, &[]);
        assert_eq!(a.suspects[0].name, "lithium");
        assert_eq!(a.suspects[0].reason, SuspectReason::Mixin);

        assert_eq!(
            analyze("java.lang.OutOfMemoryError: Java heap space", &[]).hint,
            Some(Hint::OutOfMemory)
        );
        assert_eq!(
            analyze("java.lang.UnsupportedClassVersionError: Foo has been compiled by a more recent version of the Java Runtime", &[]).hint,
            Some(Hint::WrongJava)
        );
        assert!(analyze("all good", &[]).suspects.is_empty());
    }

    #[test]
    fn parses_mods_toml_values() {
        let toml = "modLoader=\"javafml\"\n[[mods]]\nmodId=\"create\"\nversion=\"${file.jarVersion}\"\ndisplayName=\"Create\"\n";
        assert_eq!(toml_value(toml, "modId").as_deref(), Some("create"));
        assert_eq!(toml_value(toml, "displayName").as_deref(), Some("Create"));
        assert_eq!(toml_value(toml, "version"), None);
    }
}
