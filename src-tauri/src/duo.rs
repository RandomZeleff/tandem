//! Playing together (D37): one hosting session or one joined session at a time.

use std::sync::Arc;

use serde::Serialize;
use tandem_core::account::AccountKind;
use tandem_core::duo::guest::{Guest, GuestEvent, Joined};
use tandem_core::duo::host::{GuestInfo, Host, HostConfig, HostEvent};
use tandem_core::duo::lan::LanWorld;
use tandem_core::duo::protocol::InstanceSummary;
use tandem_core::duo::sync::{self, SyncReport};
use tandem_core::duo::{self, InstanceDiff, LinkStatus};
use tandem_core::instance::{self, InstanceSettings, NewInstance};
use tandem_core::launch::QuickPlay;
use tandem_core::server::{self, RunningServer, ServerOptions};
use tandem_core::{jvm, Context};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::error::{CommandError, CommandResult};
use crate::{game, modpack, AppState};

pub const HOST_EVENT: &str = "duo://host";
/// Progress and end of the world server (6e): `{ status, attempt? }`.
pub const WORLD_SERVER_EVENT: &str = "duo://world-server";
pub const GUEST_EVENT: &str = "duo://guest";

static SESSIONS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Default)]
pub struct Duo {
    host: Mutex<Option<Hosting>>,
    guest: Mutex<Option<Joining>>,
}

struct Hosting {
    host: Host,
    instance_id: Option<String>,
    /// The world in its own process, when hosted that way (6e).
    server: Option<RunningServer>,
}

impl Hosting {
    fn view(&self) -> HostView {
        HostView {
            code: self.host.code(),
            instance_id: self.instance_id.clone(),
            world: self.host.world(),
            separate: self.server.is_some(),
        }
    }

    async fn stop(self) {
        self.host.stop().await;
        if let Some(server) = self.server {
            if let Err(err) = server.stop().await {
                tracing::warn!(error = %err, "world server stop");
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorldServerPayload {
    /// `preparing`, `starting`, `ready` or `stopped`.
    status: &'static str,
}

struct Joining {
    /// Tells this session's events from those of an earlier one.
    session: u64,
    /// Shared with a mod sync running without the lock.
    guest: Arc<Guest>,
    joined: Joined,
    /// `None` until the player picks one (none of theirs matched the host's).
    instance_id: Option<String>,
    diff: Option<InstanceDiff>,
}

impl Joining {
    fn view(&self) -> JoinView {
        JoinView {
            joined: self.joined.clone(),
            instance_id: self.instance_id.clone(),
            diff: self.diff.clone(),
            link: self.guest.link(),
        }
    }
}

/// A guest as the host sees them, with how their instance differs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestView {
    #[serde(flatten)]
    pub guest: GuestInfo,
    pub diff: Option<InstanceDiff>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostView {
    pub code: String,
    pub instance_id: Option<String>,
    pub world: Option<LanWorld>,
    /// The world runs in its own process: the host's game joins it like the friends.
    pub separate: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinView {
    #[serde(flatten)]
    pub joined: Joined,
    pub instance_id: Option<String>,
    pub diff: Option<InstanceDiff>,
    pub link: Option<LinkStatus>,
}

/// What the host's interface receives: the core event, with the guest's differences.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum HostPayload {
    World {
        world: Option<LanWorld>,
    },
    GuestJoined {
        guest: GuestView,
    },
    GuestInstance {
        id: String,
        diff: Option<InstanceDiff>,
    },
    GuestLeft {
        id: String,
    },
    Link {
        id: String,
        link: LinkStatus,
    },
}

/// The name both players see: the active account, which must be a Microsoft one (the
/// LAN world checks accounts with Mojang).
async fn player(ctx: &Context) -> CommandResult<String> {
    match ctx.db.active_account().await? {
        Some(account) if account.kind == AccountKind::Microsoft => Ok(account.username),
        _ => Err(CommandError::msg(
            "Jouer ensemble demande un compte Microsoft : le monde ouvert vérifie les comptes auprès de Mojang. Connecte-le dans Comptes.",
        )),
    }
}

async fn summary_of(ctx: &Context, instance_id: &str) -> CommandResult<InstanceSummary> {
    let instance = ctx.db.get_instance(instance_id).await?;
    Ok(duo::summary(ctx, &instance).await?)
}

/// Hosts a session. With `separate_world`, that world of `instance_id` first starts in
/// its own process (6e) and is served at once: no "Open to LAN" step.
#[tauri::command]
pub async fn duo_host(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: Option<String>,
    separate_world: Option<String>,
) -> CommandResult<HostView> {
    let mut hosting = state.duo.host.lock().await;
    if let Some(current) = hosting.as_ref() {
        return Ok(current.view());
    }
    if state.duo.guest.lock().await.is_some() {
        return Err(CommandError::msg(
            "Tu as rejoint une partie : quitte-la avant d'inviter.",
        ));
    }
    let name = player(&state.ctx).await?;
    let summary = match &instance_id {
        Some(id) => Some(summary_of(&state.ctx, id).await?),
        None => None,
    };
    let mine = summary.clone();
    let mut world_server = match (&separate_world, &instance_id) {
        (Some(world), Some(id)) => Some(start_world_server(&app, &state.ctx, id, world).await?),
        (Some(_), None) => return Err(CommandError::msg("Choisis l'instance du monde à héberger")),
        _ => None,
    };
    let fixed_world = world_server.as_ref().map(|server| LanWorld {
        motd: separate_world.clone().unwrap_or_default(),
        port: server.port,
    });
    if let Some(server) = world_server.as_mut() {
        watch_world_server(app.clone(), server);
    }
    let emitter = app.clone();
    let host = Host::start(
        state.ctx.http.clone(),
        HostConfig {
            world: fixed_world,
            player: name,
            instance: summary,
            game_dir: instance_id
                .as_ref()
                .map(|id| state.ctx.data.instance_dir(id)),
        },
        move |event| {
            let payload = match event {
                HostEvent::World { world } => HostPayload::World { world },
                HostEvent::GuestJoined { guest } => {
                    let diff = match (&guest.instance, &mine) {
                        (Some(theirs), Some(mine)) => Some(duo::compare(theirs, mine)),
                        _ => None,
                    };
                    HostPayload::GuestJoined {
                        guest: GuestView { guest, diff },
                    }
                }
                HostEvent::GuestInstance { id, instance } => HostPayload::GuestInstance {
                    id,
                    diff: instance
                        .as_ref()
                        .zip(mine.as_ref())
                        .map(|(theirs, mine)| duo::compare(theirs, mine)),
                },
                HostEvent::GuestLeft { id } => HostPayload::GuestLeft { id },
                HostEvent::Link { id, link } => HostPayload::Link { id, link },
            };
            let _ = emitter.emit(HOST_EVENT, payload);
        },
    )
    .await;
    let host = match host {
        Ok(host) => host,
        Err(err) => {
            if let Some(server) = world_server {
                let _ = server.stop().await;
            }
            return Err(err.into());
        }
    };
    let started = Hosting {
        host,
        instance_id,
        server: world_server,
    };
    let view = started.view();
    *hosting = Some(started);
    Ok(view)
}

async fn start_world_server(
    app: &AppHandle,
    ctx: &Context,
    instance_id: &str,
    world: &str,
) -> CommandResult<RunningServer> {
    let instance = ctx.db.get_instance(instance_id).await?;
    if !server::supports(instance.loader) {
        return Err(CommandError::msg(
            "Le monde à part marche pour l'instant avec Fabric et Quilt.",
        ));
    }
    let account = ctx
        .db
        .active_account()
        .await?
        .ok_or_else(|| CommandError::msg("Aucun compte actif"))?;
    let status = |status| {
        let _ = app.emit(WORLD_SERVER_EVENT, WorldServerPayload { status });
    };
    status("preparing");
    let prepared = server::prepare(ctx, &instance).await?;
    let total = jvm::total_memory_mb();
    let game_dir = ctx.data.instance_dir(instance_id);
    let game_mb = instance
        .memory_mb
        .unwrap_or_else(|| jvm::auto_memory_mb(total, jvm::count_mods(&game_dir)));
    let options = ServerOptions {
        world: world.to_owned(),
        port: server::free_port()?,
        memory_mb: server::memory_mb(total, jvm::count_mods(&prepared.dir), game_mb),
        max_players: 20,
        online_mode: true,
        host: Some((account.mc_uuid.clone(), account.username.clone())),
    };
    status("starting");
    let running = server::start(ctx, &prepared, &options).await?;
    status("ready");
    Ok(running)
}

/// Keeps the server's console flowing (to the launcher's log) and tells the page when the
/// world stops on its own.
fn watch_world_server(app: AppHandle, server: &mut RunningServer) {
    let mut lines = server.take_lines();
    tauri::async_runtime::spawn(async move {
        while let Some(line) = lines.recv().await {
            tracing::debug!(target: "world_server", "{line}");
        }
        tracing::info!("world server stopped");
        let _ = app.emit(WORLD_SERVER_EVENT, WorldServerPayload { status: "stopped" });
    });
}

/// Launches the host's game straight into the world server.
#[tauri::command]
pub async fn duo_host_play(app: AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    let (instance_id, port) = {
        let hosting = state.duo.host.lock().await;
        let hosting = hosting
            .as_ref()
            .ok_or_else(|| CommandError::msg("Aucune invitation en cours"))?;
        let port = hosting
            .server
            .as_ref()
            .map(|s| s.port)
            .ok_or_else(|| CommandError::msg("Le monde n'est pas hébergé à part"))?;
        let instance_id = hosting
            .instance_id
            .clone()
            .ok_or_else(|| CommandError::msg("Aucune instance"))?;
        (instance_id, port)
    };
    game::launch(
        app,
        state.ctx.clone(),
        state.games.clone(),
        instance_id,
        Some(QuickPlay::Server("127.0.0.1".to_owned(), port)),
    )
    .await
}

#[tauri::command]
pub async fn duo_stop_host(state: State<'_, AppState>) -> CommandResult<()> {
    if let Some(hosting) = state.duo.host.lock().await.take() {
        hosting.stop().await;
    }
    Ok(())
}

#[tauri::command]
pub async fn duo_kick(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    if let Some(hosting) = state.duo.host.lock().await.as_ref() {
        hosting.host.kick(&id);
    }
    Ok(())
}

/// Joins the host behind `code`. Without `instance_id`, the instance that best matches the
/// host's is picked once welcomed.
#[tauri::command]
pub async fn duo_join(
    app: AppHandle,
    state: State<'_, AppState>,
    code: String,
    instance_id: Option<String>,
) -> CommandResult<JoinView> {
    if state.duo.host.lock().await.is_some() {
        return Err(CommandError::msg(
            "Tu invites déjà : arrête l'invitation avant de rejoindre.",
        ));
    }
    let mut joining = state.duo.guest.lock().await;
    if let Some(previous) = joining.take() {
        previous.guest.leave().await;
    }
    let name = player(&state.ctx).await?;
    let chosen = match &instance_id {
        Some(id) => Some(summary_of(&state.ctx, id).await?),
        None => None,
    };
    let session = SESSIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let emitter = app.clone();
    let (guest, joined) = Guest::join(
        state.ctx.http.clone(),
        &code,
        &name,
        chosen.clone(),
        move |event| {
            if let GuestEvent::Closed { .. } = &event {
                // The session is over: forget it so a new one can start.
                let app = emitter.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    let ended = {
                        let mut guest = state.duo.guest.lock().await;
                        match guest.as_ref() {
                            Some(current) if current.session == session => guest.take(),
                            _ => None,
                        }
                    };
                    if let Some(ended) = ended {
                        ended.guest.leave().await;
                    }
                });
            }
            let _ = emitter.emit(GUEST_EVENT, &event);
        },
    )
    .await?;
    let (instance_id, diff) = match (instance_id, chosen, &joined.host_instance) {
        (Some(id), Some(mine), host) => {
            let diff = host.as_ref().map(|host| duo::compare(&mine, host));
            (Some(id), diff)
        }
        (_, _, Some(host)) => match duo::best_match(&state.ctx, host).await {
            Ok(Some((instance, mine, diff))) => {
                if let Err(err) = guest.set_instance(Some(mine)).await {
                    tracing::warn!(error = %err, "instance not sent to the host");
                }
                (Some(instance.id), Some(diff))
            }
            Ok(None) => (None, None),
            Err(err) => {
                tracing::warn!(error = %err, "no instance picked");
                (None, None)
            }
        },
        // An older host that says nothing of its instance: the player picks.
        _ => (None, None),
    };
    let current = Joining {
        session,
        guest: Arc::new(guest),
        joined,
        instance_id,
        diff,
    };
    let view = current.view();
    *joining = Some(current);
    Ok(view)
}

/// The guest plays with another instance: tells the host, returns the new differences.
#[tauri::command]
pub async fn duo_set_instance(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<JoinView> {
    let mine = summary_of(&state.ctx, &instance_id).await?;
    let mut joining = state.duo.guest.lock().await;
    let joining = joining
        .as_mut()
        .ok_or_else(|| CommandError::msg("Aucune partie rejointe"))?;
    joining.diff = joining
        .joined
        .host_instance
        .as_ref()
        .map(|host| duo::compare(&mine, host));
    joining.instance_id = Some(instance_id);
    joining.guest.set_instance(Some(mine)).await?;
    Ok(joining.view())
}

/// What preparing the host's instance did.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyResult {
    pub view: JoinView,
    pub report: SyncReport,
}

/// Settings key of the copy of a host's instance made on this computer.
fn copy_key(host_player: &str, host_instance: &str) -> String {
    format!("duo_copy:{host_player}:{host_instance}")
}

/// Makes the guest an instance like the host's: its Modrinth modpack when it has one
/// (else an empty instance of the same version and loader), then the host's exact mods.
/// A copy made earlier for the same host and instance is brought up to date instead.
#[tauri::command]
pub async fn duo_prepare_copy(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<CopyResult> {
    let (guest, host_player, host) = {
        let joining = state.duo.guest.lock().await;
        let joining = joining
            .as_ref()
            .ok_or_else(|| CommandError::msg("Aucune partie rejointe"))?;
        let host =
            joining.joined.host_instance.clone().ok_or_else(|| {
                CommandError::msg("L'hôte n'a pas lancé d'instance : rien à copier")
            })?;
        (
            joining.guest.clone(),
            joining.joined.host_player.clone(),
            host,
        )
    };
    let ctx = &state.ctx;
    let key = copy_key(&host_player, &host.name);
    let earlier = match ctx.db.get_setting::<String>(&key).await? {
        Some(id) => ctx.db.get_instance(&id).await.ok(),
        None => None,
    };
    let copy = match earlier {
        Some(copy)
            if copy.game_version == host.game_version && copy.loader.to_string() == host.loader =>
        {
            copy
        }
        _ => create_copy(&app, &state, &host_player, &host).await?,
    };
    ctx.db.set_setting(&key, &copy.id).await?;

    if !state.games.begin(&copy.id) {
        return Err(CommandError::msg(
            "Cette instance est en cours d'utilisation",
        ));
    }
    let synced = sync::sync_instance(
        ctx,
        &guest,
        &ctx.data.instance_dir(&copy.id),
        &host,
        game::progress_emitter(&app, &copy.id),
    )
    .await;
    state.games.end(&copy.id);
    let _ = app.emit(modpack::INSTALL_FINISHED_EVENT, &copy.id);
    let _ = app.emit(modpack::INSTANCES_CHANGED_EVENT, ());
    let report = synced?;

    let mine = summary_of(ctx, &copy.id).await?;
    let mut joining = state.duo.guest.lock().await;
    let joining = joining
        .as_mut()
        .filter(|j| Arc::ptr_eq(&j.guest, &guest))
        .ok_or_else(|| CommandError::msg("La partie s'est terminée pendant la préparation"))?;
    joining.diff = Some(duo::compare(&mine, &host));
    joining.instance_id = Some(copy.id.clone());
    joining.guest.set_instance(Some(mine)).await?;
    Ok(CopyResult {
        view: joining.view(),
        report,
    })
}

/// A new instance for the host's: their modpack version when Modrinth has it.
async fn create_copy(
    app: &AppHandle,
    state: &AppState,
    host_player: &str,
    host: &InstanceSummary,
) -> CommandResult<tandem_core::instance::Instance> {
    let name: String = format!("{} ({host_player})", host.name)
        .chars()
        .take(64)
        .collect();
    if let Some(pack) = &host.pack {
        match modpack::install_from_modrinth(
            app,
            &state.ctx,
            &state.games,
            &pack.project_id,
            Some(&pack.version_id),
        )
        .await
        {
            Ok(installed) => {
                let settings = InstanceSettings {
                    name,
                    java_path: installed.java_path.clone(),
                    jvm_args: installed.jvm_args.clone(),
                    window_width: installed.window_width,
                    window_height: installed.window_height,
                };
                return Ok(instance::update_settings(&state.ctx, &installed.id, &settings).await?);
            }
            Err(err) => {
                tracing::warn!(error = ?err, "host's modpack not installed, copying mods only")
            }
        }
    }
    let loader = serde_json::from_value(serde_json::Value::String(host.loader.clone()))
        .map_err(|_| CommandError::msg(format!("Loader inconnu : {}", host.loader)))?;
    let created = instance::create(
        &state.ctx,
        NewInstance {
            name,
            game_version: host.game_version.clone(),
            loader,
            loader_version: host.loader_version.clone(),
        },
    )
    .await?;
    let _ = app.emit(modpack::INSTANCES_CHANGED_EVENT, ());
    Ok(created)
}

#[tauri::command]
pub async fn duo_leave(state: State<'_, AppState>) -> CommandResult<()> {
    if let Some(joining) = state.duo.guest.lock().await.take() {
        joining.guest.leave().await;
    }
    Ok(())
}

/// Launches the guest's instance straight into the host's world.
#[tauri::command]
pub async fn duo_play(app: AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    let (instance_id, port) = {
        let joining = state.duo.guest.lock().await;
        let joining = joining
            .as_ref()
            .ok_or_else(|| CommandError::msg("Aucune partie rejointe"))?;
        let instance_id = joining
            .instance_id
            .clone()
            .ok_or_else(|| CommandError::msg("Choisis d'abord l'instance avec laquelle jouer"))?;
        (instance_id, joining.guest.port())
    };
    game::launch(
        app,
        state.ctx.clone(),
        state.games.clone(),
        instance_id,
        Some(QuickPlay::Server("127.0.0.1".to_owned(), port)),
    )
    .await
}

/// The current session, for a page opened after it started.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuoState {
    pub host: Option<HostView>,
    pub guest: Option<JoinView>,
}

#[tauri::command]
pub async fn duo_state(state: State<'_, AppState>) -> CommandResult<DuoState> {
    let host = state.duo.host.lock().await.as_ref().map(Hosting::view);
    let guest = state.duo.guest.lock().await.as_ref().map(Joining::view);
    Ok(DuoState { host, guest })
}

/// Ends any session when the launcher closes, so guests are told at once.
pub async fn shutdown(state: &AppState) {
    if let Some(hosting) = state.duo.host.lock().await.take() {
        hosting.stop().await;
    }
    if let Some(joining) = state.duo.guest.lock().await.take() {
        joining.guest.leave().await;
    }
}
