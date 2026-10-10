//! Playing together (D37): one hosting session or one joined session at a time.

use serde::Serialize;
use tandem_core::account::AccountKind;
use tandem_core::duo::guest::{Guest, GuestEvent, Joined};
use tandem_core::duo::host::{GuestInfo, Host, HostConfig, HostEvent};
use tandem_core::duo::lan::LanWorld;
use tandem_core::duo::protocol::InstanceSummary;
use tandem_core::duo::{self, InstanceDiff, LinkStatus};
use tandem_core::Context;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::error::{CommandError, CommandResult};
use crate::{game, AppState};

pub const HOST_EVENT: &str = "duo://host";
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
}

struct Joining {
    /// Tells this session's events from those of an earlier one.
    session: u64,
    guest: Guest,
    joined: Joined,
    instance_id: String,
    diff: Option<InstanceDiff>,
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinView {
    #[serde(flatten)]
    pub joined: Joined,
    pub instance_id: String,
    pub diff: Option<InstanceDiff>,
    pub link: Option<LinkStatus>,
}

/// What the host's interface receives: the core event, with the guest's differences.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum HostPayload {
    World { world: Option<LanWorld> },
    GuestJoined { guest: GuestView },
    GuestLeft { id: String },
    Link { id: String, link: LinkStatus },
}

/// The name both players see: the active account, which must be a Microsoft one (the
/// LAN world checks accounts with Mojang).
async fn player(ctx: &Context) -> CommandResult<String> {
    match ctx.db.active_account().await? {
        Some(account) if account.kind == AccountKind::Microsoft => Ok(account.username),
        _ => Err(CommandError::msg(
            "Le jeu à deux demande un compte Microsoft : le monde ouvert vérifie les comptes auprès de Mojang. Connecte-le dans Comptes.",
        )),
    }
}

async fn summary_of(ctx: &Context, instance_id: &str) -> CommandResult<InstanceSummary> {
    let instance = ctx.db.get_instance(instance_id).await?;
    Ok(duo::summary(ctx, &instance).await?)
}

#[tauri::command]
pub async fn duo_host(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: Option<String>,
) -> CommandResult<HostView> {
    let mut hosting = state.duo.host.lock().await;
    if let Some(current) = hosting.as_ref() {
        return Ok(HostView {
            code: current.host.code(),
            instance_id: current.instance_id.clone(),
            world: current.host.world(),
        });
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
    let emitter = app.clone();
    let host = Host::start(
        state.ctx.http.clone(),
        HostConfig {
            player: name,
            instance: summary,
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
                HostEvent::GuestLeft { id } => HostPayload::GuestLeft { id },
                HostEvent::Link { id, link } => HostPayload::Link { id, link },
            };
            let _ = emitter.emit(HOST_EVENT, payload);
        },
    )
    .await?;
    let view = HostView {
        code: host.code(),
        instance_id: instance_id.clone(),
        world: host.world(),
    };
    *hosting = Some(Hosting { host, instance_id });
    Ok(view)
}

#[tauri::command]
pub async fn duo_stop_host(state: State<'_, AppState>) -> CommandResult<()> {
    if let Some(hosting) = state.duo.host.lock().await.take() {
        hosting.host.stop().await;
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

#[tauri::command]
pub async fn duo_join(
    app: AppHandle,
    state: State<'_, AppState>,
    code: String,
    instance_id: String,
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
    let summary = summary_of(&state.ctx, &instance_id).await?;
    let session = SESSIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let emitter = app.clone();
    let (guest, joined) = Guest::join(
        state.ctx.http.clone(),
        &code,
        &name,
        Some(summary.clone()),
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
    let diff = joined
        .host_instance
        .as_ref()
        .map(|host| duo::compare(&summary, host));
    let view = JoinView {
        joined: joined.clone(),
        instance_id: instance_id.clone(),
        diff: diff.clone(),
        link: guest.link(),
    };
    *joining = Some(Joining {
        session,
        guest,
        joined,
        instance_id,
        diff,
    });
    Ok(view)
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
        (joining.instance_id.clone(), joining.guest.port())
    };
    game::launch(
        app,
        state.ctx.clone(),
        state.games.clone(),
        instance_id,
        Some(port),
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
    let host = state.duo.host.lock().await.as_ref().map(|h| HostView {
        code: h.host.code(),
        instance_id: h.instance_id.clone(),
        world: h.host.world(),
    });
    let guest = state.duo.guest.lock().await.as_ref().map(|g| JoinView {
        joined: g.joined.clone(),
        instance_id: g.instance_id.clone(),
        diff: g.diff.clone(),
        link: g.guest.link(),
    });
    Ok(DuoState { host, guest })
}

/// Ends any session when the launcher closes, so guests are told at once.
pub async fn shutdown(state: &AppState) {
    if let Some(hosting) = state.duo.host.lock().await.take() {
        hosting.host.stop().await;
    }
    if let Some(joining) = state.duo.guest.lock().await.take() {
        joining.guest.leave().await;
    }
}
