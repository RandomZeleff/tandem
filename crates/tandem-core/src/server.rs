//! A world in its own process (6e, D43): while hosting, the world runs in a dedicated
//! server Tandem starts with the instance's mods, and the host's game joins it like the
//! friends do. The game crashing no longer ends the session, there is no 8-player LAN
//! limit and no "Open to LAN" step.
//!
//! Nothing is copied: `<data>/servers/<instance>/` holds links to the instance's mods
//! (client-only ones left out) and folders, and the world is read in place from the
//! instance's `saves/`. Fabric and Quilt for now: their loader runs Mojang's server jar
//! directly (`<loader>.gameJarPath`).

use std::collections::{BTreeSet, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::mpsc;

use crate::download::{self, DownloadTask};
use crate::error::{Error, Result};
use crate::install::{self, Target};
use crate::instance::Instance;
use crate::meta::{self, loader::Loader};
use crate::process::ProcessGroup;
use crate::{jvm, optimizations, Context};

pub mod host_player;

/// Instance folders never linked into the server: the world comes through `--universe`,
/// the rest is the game's own (logs, screenshots, packs, caches).
const NOT_LINKED: &[&str] = &[
    "mods",
    "saves",
    "logs",
    "crash-reports",
    "screenshots",
    "resourcepacks",
    "shaderpacks",
    "texturepacks",
    "libraries",
    "versions",
    "natives",
    ".fabric",
    ".quilt",
    "backups",
    "downloads",
];
/// Mods left out after crashing the server, one per line.
const EXCLUDED_FILE: &str = "tandem-excluded-mods.txt";
/// Starts before giving up on a server that keeps crashing on client-only mods.
const MAX_ATTEMPTS: usize = 25;
const STOP_TIMEOUT: Duration = Duration::from_secs(90);
/// What a server that failed to start writes. A mod thread can keep it alive after.
const CRASH_MARKERS: &[&str] = &[
    "Incompatible mods found",
    "Could not execute entrypoint",
    "Mixin apply failed",
    "MixinApplyError",
    "Failed to start the minecraft server",
    "Encountered an unexpected exception",
    "This crash report has been saved",
];
const CRASH_GRACE: Duration = Duration::from_secs(5);

pub fn supports(loader: Loader) -> bool {
    matches!(loader, Loader::Fabric | Loader::Quilt)
}

/// Memory for a world server, from the load tests (Cobbleverse held 18 players in 3 GB,
/// Prominence II 14 in 4 GB). Lowered when the game and the world together would leave
/// the system less than 4 GB.
pub fn memory_mb(total_mb: u64, mod_count: usize, game_mb: u32) -> u32 {
    let wanted: u64 = match mod_count {
        0..=50 => 2048,
        51..=150 => 3072,
        _ => 4096,
    };
    let room = total_mb.saturating_sub(4096 + u64::from(game_mb));
    (wanted.min(room) / 512 * 512).max(2048) as u32
}

/// A local TCP port nobody uses right now.
pub fn free_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    Ok(listener.local_addr()?.port())
}

fn fail(message: impl Into<String>) -> Error {
    Error::WorldServer(message.into())
}

/// Everything installed to run an instance's world in its own process.
#[derive(Debug, Clone)]
pub struct PreparedServer {
    pub dir: PathBuf,
    instance: Instance,
    java: PathBuf,
    classpath: Vec<PathBuf>,
    main_class: String,
    game_jar: PathBuf,
    saves: PathBuf,
}

/// How the world is served.
#[derive(Debug, Clone)]
pub struct ServerOptions {
    /// Folder of the world in the instance's `saves/`.
    pub world: String,
    pub port: u16,
    pub memory_mb: u32,
    pub max_players: u32,
    /// Accounts checked with Mojang, as a LAN world does.
    pub online_mode: bool,
    /// The hosting player, `(uuid, name)`: their solo inventory goes to the server and
    /// back, and they are operator when the world allows commands.
    pub host: Option<(String, String)>,
}

/// Installs what the server needs (server jar, the loader's server libraries, Java, as
/// the game already has) and lays out its folder.
pub async fn prepare(ctx: &Context, instance: &Instance) -> Result<PreparedServer> {
    if !supports(instance.loader) {
        return Err(Error::LoaderNotSupported(
            instance.loader.as_str().to_owned(),
        ));
    }
    let loader_version = instance
        .loader_version
        .as_deref()
        .ok_or_else(|| fail("version du loader inconnue"))?;
    let game_dir = ctx.data.instance_dir(&instance.id);
    // Java and the client's own files: already there when the game was launched once.
    let client = install::prepare(ctx, Target::of(instance), &game_dir, |_| {}).await?;
    let java = match &instance.java_path {
        Some(path) => PathBuf::from(path),
        None => client.java,
    };

    let version = meta::load_version(ctx, &instance.game_version).await?;
    let server = version.downloads.server.ok_or_else(|| {
        fail(format!(
            "Mojang ne publie pas de serveur pour {}",
            instance.game_version
        ))
    })?;
    let profile = meta::loader::load_server_profile(
        ctx,
        instance.loader,
        &instance.game_version,
        loader_version,
    )
    .await?;

    let game_jar = ctx
        .data
        .version_dir(&version.id)
        .join(format!("{}-server.jar", version.id));
    let libraries = ctx.data.libraries();
    let mut tasks = vec![DownloadTask {
        url: server.url.clone(),
        dest: game_jar.clone(),
        sha1: Some(server.sha1.clone()),
        size: Some(server.size),
    }];
    let mut classpath = Vec::new();
    for library in &profile.libraries {
        if let Some(file) = library.artifact() {
            let dest = libraries.join(&file.path);
            classpath.push(dest.clone());
            tasks.push(DownloadTask {
                url: file.url,
                dest,
                sha1: file.sha1,
                size: file.size,
            });
        }
    }
    download::download_all(&ctx.http, tasks, download::DEFAULT_CONCURRENCY, |_| {}).await?;

    let dir = ctx.data.root().join("servers").join(&instance.id);
    tokio::fs::create_dir_all(&dir).await?;
    // The server jar unpacks its own libraries next to it: shared by every instance.
    link_dir(&libraries, &dir.join("libraries")).await?;
    link_dir(
        &ctx.data.root().join("server-versions"),
        &dir.join("versions"),
    )
    .await?;
    link_instance_folders(&game_dir, &dir).await?;
    tokio::fs::write(dir.join("eula.txt"), "eula=true\n").await?;

    Ok(PreparedServer {
        dir,
        instance: instance.clone(),
        java,
        classpath,
        main_class: profile.main_class,
        game_jar,
        saves: game_dir.join("saves"),
    })
}

/// A running world server.
pub struct RunningServer {
    child: Child,
    stdin: ChildStdin,
    group: ProcessGroup,
    world: PathBuf,
    host_uuid: Option<String>,
    pub port: u16,
    /// Console lines, as the server writes them.
    pub lines: mpsc::UnboundedReceiver<String>,
}

impl RunningServer {
    /// Sends a console command (`save-all flush`, `stop`…).
    pub async fn command(&mut self, command: &str) -> Result<()> {
        self.stdin
            .write_all(format!("{command}\n").as_bytes())
            .await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// Saves and stops (killed if it does not stop in time), then gives the host's player
    /// back to the solo world.
    pub async fn stop(mut self) -> Result<()> {
        let _ = self.command("stop").await;
        if tokio::time::timeout(STOP_TIMEOUT, self.child.wait())
            .await
            .is_err()
        {
            tracing::warn!("world server did not stop, killing it");
            self.group.kill(&mut self.child).await;
        }
        self.hand_back().await
    }

    /// The host's player back into `level.dat`, once the server is gone.
    pub async fn hand_back(&self) -> Result<()> {
        let Some(uuid) = self.host_uuid.clone() else {
            return Ok(());
        };
        let world = self.world.clone();
        tokio::task::spawn_blocking(move || host_player::take_back(&world, &uuid))
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e)))?
    }

    /// The console, to read elsewhere (lines stop when the server exits).
    pub fn take_lines(&mut self) -> mpsc::UnboundedReceiver<String> {
        std::mem::replace(&mut self.lines, mpsc::unbounded_channel().1)
    }

    /// Exits on its own (crash): the exit code once it does.
    pub async fn wait(&mut self) -> Option<i32> {
        self.child.wait().await.ok().and_then(|s| s.code())
    }
}

/// Starts the world and waits until it accepts players. A client-only mod that crashes
/// the server is left out (remembered for next time) and the server started again.
pub async fn start(
    ctx: &Context,
    prepared: &PreparedServer,
    options: &ServerOptions,
) -> Result<RunningServer> {
    if !prepared
        .saves
        .join(&options.world)
        .join("level.dat")
        .exists()
    {
        return Err(fail(format!("monde introuvable : {}", options.world)));
    }
    let world = prepared.saves.join(&options.world);
    let settings = {
        let world = world.clone();
        tokio::task::spawn_blocking(move || host_player::level_settings(&world))
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e)))??
    };
    write_properties(&prepared.dir, options, &settings).await?;
    write_ops(&prepared.dir, options, &settings).await?;
    if let Some((uuid, _)) = &options.host {
        let (world, uuid) = (world.clone(), uuid.clone());
        tokio::task::spawn_blocking(move || host_player::give_to_server(&world, &uuid))
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e)))??;
    }
    for attempt in 1..=MAX_ATTEMPTS {
        let excluded = read_excluded(&prepared.dir).await;
        let mods = sync_mods(
            &ctx.data.instance_dir(&prepared.instance.id).join("mods"),
            &prepared.dir.join("mods"),
            &excluded,
        )
        .await?;
        let command = command(ctx, prepared, options).await?;
        tracing::info!(attempt, port = options.port, world = %options.world, "starting world server");
        let mut server = spawn(&command, &prepared.dir, options.port)?;
        server.world = world.clone();
        server.host_uuid = options.host.as_ref().map(|(uuid, _)| uuid.clone());
        let mut seen: Vec<String> = Vec::new();
        // A mod thread can keep a crashed server alive: stopped shortly after the crash.
        let mut crashed_at: Option<tokio::time::Instant> = None;
        let outcome = loop {
            let grace = async {
                match crashed_at {
                    Some(at) => tokio::time::sleep_until(at + CRASH_GRACE).await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                line = server.lines.recv() => match line {
                    Some(line) => {
                        let done = line.contains("]: Done (");
                        if crashed_at.is_none() && CRASH_MARKERS.iter().any(|m| line.contains(m)) {
                            crashed_at = Some(tokio::time::Instant::now());
                        }
                        seen.push(line);
                        if done && crashed_at.is_none() {
                            break Ok(());
                        }
                        if seen.len() > 20_000 {
                            seen.drain(..10_000);
                        }
                    }
                    None => break Err(server.wait().await),
                },
                () = grace => {
                    server.group.kill(&mut server.child).await;
                    break Err(None);
                }
            }
        };
        match outcome {
            Ok(()) => {
                tracing::info!(port = options.port, "world server ready");
                return Ok(server);
            }
            Err(code) => {
                let jars = culprits(&seen, &mods);
                if jars.is_empty() {
                    let tail: Vec<&str> = seen
                        .iter()
                        .rev()
                        .take(15)
                        .rev()
                        .map(String::as_str)
                        .collect();
                    tracing::warn!(?code, log = %tail.join("\n"), "world server stopped");
                    return Err(fail(
                        "le serveur s'est arrêté au démarrage (voir le journal)",
                    ));
                }
                for jar in jars {
                    tracing::warn!(attempt, %jar, ?code, "mod cannot run on the world server, left out");
                    add_excluded(&prepared.dir, &jar).await?;
                }
            }
        }
    }
    Err(fail("trop de mods incompatibles avec un serveur"))
}

async fn command(
    ctx: &Context,
    prepared: &PreparedServer,
    options: &ServerOptions,
) -> Result<Vec<String>> {
    let game_jar_property = match prepared.instance.loader {
        Loader::Quilt => "loader.gameJarPath",
        _ => "fabric.gameJarPath",
    };
    let separator = if cfg!(windows) { ";" } else { ":" };
    let mut args = vec![
        format!("-Xmx{}M", options.memory_mb),
        format!("-D{game_jar_property}={}", prepared.game_jar.display()),
        // No error window from the loader: nobody would see it and the process would hang.
        "-Djava.awt.headless=true".to_owned(),
        "-Dlog4j2.formatMsgNoLookups=true".to_owned(),
        // `sun.*` up to Java 18, plain from 19 (as for the game).
        "-Dstdout.encoding=UTF-8".to_owned(),
        "-Dstderr.encoding=UTF-8".to_owned(),
        "-Dsun.stdout.encoding=UTF-8".to_owned(),
        "-Dsun.stderr.encoding=UTF-8".to_owned(),
    ];
    args.extend(jvm::gc_flags(&[]));
    args.extend(optimizations::jvm_args(ctx, &prepared.instance).await?);
    args.extend([
        "-cp".to_owned(),
        prepared
            .classpath
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(separator),
        prepared.main_class.clone(),
        "--nogui".to_owned(),
        "--universe".to_owned(),
        prepared.saves.display().to_string(),
        "--world".to_owned(),
        options.world.clone(),
        "--port".to_owned(),
        options.port.to_string(),
    ]);
    let mut command = vec![prepared.java.display().to_string()];
    command.extend(args);
    Ok(command)
}

fn spawn(command: &[String], dir: &Path, port: u16) -> Result<RunningServer> {
    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    #[cfg(unix)]
    cmd.process_group(0);
    let mut child = cmd.spawn()?;
    // Killed with Tandem: an orphan would keep the world locked and the memory taken.
    let group = ProcessGroup::track_owned(&child);
    let stdin = child.stdin.take().ok_or_else(|| fail("pas d'entrée"))?;
    let (tx, lines) = mpsc::unbounded_channel();
    for out in [
        child
            .stdout
            .take()
            .map(|o| Box::new(o) as Box<dyn tokio::io::AsyncRead + Send + Unpin>),
        child
            .stderr
            .take()
            .map(|o| Box::new(o) as Box<dyn tokio::io::AsyncRead + Send + Unpin>),
    ]
    .into_iter()
    .flatten()
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            // Bytes, not `lines()`: one line in another encoding must not stop the reading.
            let mut reader = BufReader::new(out);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let line = String::from_utf8_lossy(&buf).trim_end().to_owned();
                        if tx.send(line).is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }
    Ok(RunningServer {
        child,
        stdin,
        group,
        world: PathBuf::new(),
        host_uuid: None,
        port,
        lines,
    })
}

/// The mods to leave out after a failed start: those the loader says need a missing mod
/// (a client-only one Tandem left out), else the last jar of the instance's mods the
/// crash's stack traces name.
fn culprits(lines: &[String], mods: &ModIndex) -> Vec<String> {
    let missing: BTreeSet<String> = lines
        .iter()
        .filter(|l| l.contains(" requires ") && l.contains("which is missing"))
        .filter_map(|l| {
            l.split_once("' (")?
                .1
                .split_once(')')
                .map(|(id, _)| id.to_owned())
        })
        .filter_map(|id| mods.by_id.get(&id).cloned())
        .collect();
    if !missing.is_empty() {
        return missing.into_iter().collect();
    }
    let from = lines
        .iter()
        .rposition(|l| l.contains("Caused by"))
        .unwrap_or(0);
    let find = |lines: &[String]| {
        lines.iter().find_map(|line| {
            line.split('[')
                .skip(1)
                .filter_map(|part| part.split([']', ':']).next())
                .map(|name| name.trim_start_matches('~').to_owned())
                .find(|name| mods.jars.contains(name))
        })
    };
    find(&lines[from..])
        .or_else(|| find(lines))
        .into_iter()
        .collect()
}

async fn write_properties(
    dir: &Path,
    options: &ServerOptions,
    settings: &host_player::LevelSettings,
) -> Result<()> {
    let text = format!(
        "# Written by Tandem for each start\n\
         server-port={}\n\
         online-mode={}\n\
         max-players={}\n\
         gamemode={}\n\
         difficulty={}\n\
         hardcore={}\n\
         enable-status=true\n\
         spawn-protection=0\n\
         motd={}\n",
        options.port,
        options.online_mode,
        options.max_players,
        settings.game_mode,
        settings.difficulty,
        settings.hardcore,
        options.world
    );
    tokio::fs::write(dir.join("server.properties"), text).await?;
    Ok(())
}

/// The host is operator when the world allows commands; nobody otherwise.
async fn write_ops(
    dir: &Path,
    options: &ServerOptions,
    settings: &host_player::LevelSettings,
) -> Result<()> {
    let ops = match &options.host {
        Some((uuid, name)) if settings.allow_commands => serde_json::json!([{
            "uuid": host_player::hyphenated(uuid),
            "name": name,
            "level": 4,
            "bypassesPlayerLimit": true,
        }]),
        _ => serde_json::json!([]),
    };
    tokio::fs::write(dir.join("ops.json"), serde_json::to_vec_pretty(&ops)?).await?;
    Ok(())
}

async fn read_excluded(dir: &Path) -> BTreeSet<String> {
    tokio::fs::read_to_string(dir.join(EXCLUDED_FILE))
        .await
        .map(|t| {
            t.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

async fn add_excluded(dir: &Path, jar: &str) -> Result<()> {
    let mut excluded = read_excluded(dir).await;
    excluded.insert(jar.to_owned());
    let text: String = excluded.into_iter().map(|j| j + "\n").collect();
    tokio::fs::write(dir.join(EXCLUDED_FILE), text).await?;
    Ok(())
}

/// The mods linked into the server, by file name and by mod id.
#[derive(Debug, Default)]
struct ModIndex {
    jars: BTreeSet<String>,
    by_id: HashMap<String, String>,
}

/// Makes the server's `mods` mirror the instance's enabled mods, without client-only ones
/// and those that could not run on it.
async fn sync_mods(from: &Path, to: &Path, excluded: &BTreeSet<String>) -> Result<ModIndex> {
    let (from, to, excluded) = (from.to_owned(), to.to_owned(), excluded.clone());
    tokio::task::spawn_blocking(move || -> Result<ModIndex> {
        std::fs::create_dir_all(&to)?;
        let mut index = ModIndex::default();
        if let Ok(entries) = std::fs::read_dir(&from) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.ends_with(".jar") || excluded.contains(&name) {
                    continue;
                }
                let meta = mod_meta(&entry.path());
                if meta.client_only {
                    continue;
                }
                if let Some(id) = meta.id {
                    index.by_id.insert(id, name.clone());
                }
                index.jars.insert(name);
            }
        }
        for entry in std::fs::read_dir(&to)?.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !index.jars.contains(&name) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        for name in &index.jars {
            let dest = to.join(name);
            let src = from.join(name);
            let same = std::fs::metadata(&dest)
                .ok()
                .zip(std::fs::metadata(&src).ok())
                .is_some_and(|(d, s)| d.len() == s.len());
            if !same {
                let _ = std::fs::remove_file(&dest);
                if std::fs::hard_link(&src, &dest).is_err() {
                    std::fs::copy(&src, &dest)?;
                }
            }
        }
        Ok(index)
    })
    .await
    .map_err(|e| Error::Io(std::io::Error::other(e)))?
}

#[derive(Deserialize)]
struct FabricMeta {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    environment: Option<String>,
}

#[derive(Deserialize)]
struct QuiltMeta {
    #[serde(default)]
    quilt_loader: Option<QuiltLoader>,
    #[serde(default)]
    minecraft: Option<FabricMeta>,
}

#[derive(Deserialize)]
struct QuiltLoader {
    #[serde(default)]
    id: Option<String>,
}

#[derive(Debug, Default, PartialEq)]
struct ModMeta {
    id: Option<String>,
    /// Declares it only runs on the client.
    client_only: bool,
}

/// What `fabric.mod.json` or `quilt.mod.json` says about a mod.
fn mod_meta(jar: &Path) -> ModMeta {
    let Ok(file) = std::fs::File::open(jar) else {
        return ModMeta::default();
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return ModMeta::default();
    };
    let mut read = |name: &str| -> Option<String> {
        let mut entry = zip.by_name(name).ok()?;
        let mut text = String::new();
        entry.read_to_string(&mut text).ok()?;
        Some(text)
    };
    if let Some(meta) =
        read("fabric.mod.json").and_then(|t| serde_json::from_str::<FabricMeta>(&t).ok())
    {
        return ModMeta {
            client_only: meta.environment.as_deref() == Some("client"),
            id: meta.id,
        };
    }
    if let Some(meta) =
        read("quilt.mod.json").and_then(|t| serde_json::from_str::<QuiltMeta>(&t).ok())
    {
        return ModMeta {
            client_only: meta.minecraft.and_then(|m| m.environment).as_deref() == Some("client"),
            id: meta.quilt_loader.and_then(|l| l.id),
        };
    }
    ModMeta::default()
}

/// Links the instance's folders (configs, scripts, global datapacks…) into the server's.
async fn link_instance_folders(game_dir: &Path, dir: &Path) -> Result<()> {
    let mut entries = tokio::fs::read_dir(game_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if NOT_LINKED.contains(&name.as_str()) || !entry.file_type().await?.is_dir() {
            continue;
        }
        link_dir(&entry.path(), &dir.join(&name)).await?;
    }
    Ok(())
}

/// A directory link: a junction on Windows (no administrator rights needed), a symlink
/// elsewhere. Left as is when it already points there.
async fn link_dir(target: &Path, link: &Path) -> Result<()> {
    tokio::fs::create_dir_all(target).await?;
    let (target, link) = (target.to_owned(), link.to_owned());
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        if let Ok(current) = std::fs::read_link(&link) {
            if strip_verbatim(&current) == strip_verbatim(&target) {
                return Ok(());
            }
        }
        match std::fs::read_link(&link) {
            // An old link: removed as a directory, without touching what it points to.
            Ok(_) => std::fs::remove_dir(&link)?,
            // A real folder the server made itself (a mod's data, before the instance had
            // one): kept, never replaced.
            Err(_) if std::fs::symlink_metadata(&link).is_ok() => {
                if std::fs::remove_dir(&link).is_err() {
                    return Ok(());
                }
            }
            Err(_) => {}
        }
        #[cfg(windows)]
        junction::create(&target, &link)?;
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link)?;
        Ok(())
    })
    .await
    .map_err(|e| Error::Io(std::io::Error::other(e)))??;
    Ok(())
}

fn strip_verbatim(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    PathBuf::from(text.trim_start_matches(r"\\?\").trim_start_matches(r"\??\"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_memory_leaves_room_for_the_system() {
        assert_eq!(memory_mb(16_384, 136, 6144), 3072);
        assert_eq!(memory_mb(16_384, 443, 8192), 4096);
        // 8 GB: what is left after the game and 4 GB for the system, never under 2 GB.
        assert_eq!(memory_mb(8192, 443, 3072), 2048);
        assert_eq!(memory_mb(12_288, 443, 5120), 3072);
    }

    #[test]
    fn finds_the_mod_a_crash_names() {
        let mods = ModIndex {
            jars: [
                "sodium-0.6.jar",
                "controlify-2.1.jar",
                "lithium.jar",
                "colorwheel-1.3.jar",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            by_id: HashMap::from([("colorwheel".to_owned(), "colorwheel-1.3.jar".to_owned())]),
        };
        let lines: Vec<String> = [
            "[main/INFO]: Loading 3 mods",
            "java.lang.RuntimeException: Could not execute entrypoint stage 'main'",
            "\tat net.fabricmc.loader.impl.FabricLoaderImpl.invokeEntrypoints(FabricLoaderImpl.java:1) ~[fabric-loader-0.19.5.jar:?]",
            "Caused by: java.lang.NoClassDefFoundError: net/minecraft/class_310",
            "\tat dev.isxander.controlify.Controlify.init(Controlify.java:42) ~[controlify-2.1.jar:?]",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert_eq!(culprits(&lines, &mods), ["controlify-2.1.jar"]);
        assert!(culprits(&lines[..2], &mods).is_empty());
        let missing = ["\t - Mod 'Colorwheel' (colorwheel) 1.3.0+mc1.20.1 requires version 1.7.0 or later of iris, which is missing!"
            .to_owned()];
        assert_eq!(culprits(&missing, &mods), ["colorwheel-1.3.jar"]);
    }

    #[test]
    fn reads_client_only_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, entry: &str, json: &str| {
            let path = dir.path().join(name);
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            zip.start_file(entry, zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut zip, json.as_bytes()).unwrap();
            zip.finish().unwrap();
            path
        };
        let client = |name, entry, json| mod_meta(&write(name, entry, json)).client_only;
        assert!(client(
            "a.jar",
            "fabric.mod.json",
            r#"{"id":"a","environment":"client"}"#
        ));
        assert!(!client(
            "b.jar",
            "fabric.mod.json",
            r#"{"id":"b","environment":"*"}"#
        ));
        assert!(!client("c.jar", "fabric.mod.json", r#"{"id":"c"}"#));
        let quilt = mod_meta(&write(
            "d.jar",
            "quilt.mod.json",
            r#"{"quilt_loader":{"id":"d"},"minecraft":{"environment":"client"}}"#,
        ));
        assert_eq!(
            quilt,
            ModMeta {
                id: Some("d".into()),
                client_only: true
            }
        );
    }

    #[tokio::test]
    async fn mirrors_mods_without_excluded_ones() {
        let dir = tempfile::tempdir().unwrap();
        let (from, to) = (dir.path().join("from"), dir.path().join("to"));
        std::fs::create_dir_all(&from).unwrap();
        std::fs::create_dir_all(&to).unwrap();
        for name in ["a.jar", "b.jar", "c.jar.disabled"] {
            std::fs::write(from.join(name), name).unwrap();
        }
        std::fs::write(to.join("old.jar"), "old").unwrap();
        let excluded = BTreeSet::from(["b.jar".to_owned()]);
        let linked = sync_mods(&from, &to, &excluded).await.unwrap();
        assert_eq!(linked.jars, BTreeSet::from(["a.jar".to_owned()]));
        let mut names: Vec<_> = std::fs::read_dir(&to)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["a.jar"]);
    }
}
