//! Friends (D59): the service starts with the launcher once a Microsoft account is
//! there, shares what the player does on its own, and forwards requests and invites.

use std::time::Duration;

use serde::Serialize;
use tandem_core::account::{Account, AccountKind};
use tandem_core::friends::service::{FriendView, Friends as Service, Me};
use tandem_core::friends::{self, Presence, HIDDEN_SETTING, OPEN_TO_FRIENDS_SETTING};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::error::{CommandError, CommandResult};
use crate::AppState;

/// Payload: `tandem_core::friends::service::Event`.
pub const EVENT: &str = "friends://event";
/// How often the shared presence is brought up to date.
const PRESENCE_EVERY: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct FriendsState {
    service: Mutex<Option<Service>>,
}

fn me(account: &Account) -> Me {
    Me {
        name: account.username.clone(),
        mc_uuid: account.mc_uuid.clone(),
        avatar: account.avatar.clone(),
    }
}

/// The Microsoft account friends know the player by: the active one, else the first.
async fn microsoft_account(state: &AppState) -> Option<Account> {
    let accounts = state.ctx.db.list_accounts().await.ok()?;
    let microsoft = accounts.iter().filter(|a| a.kind == AccountKind::Microsoft);
    microsoft
        .clone()
        .find(|a| a.is_active)
        .or_else(|| microsoft.clone().next())
        .cloned()
}

/// Starts the service if it is not running and there is a Microsoft account.
pub async fn ensure_started(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut slot = state.friends.service.lock().await;
    if slot.is_some() {
        return;
    }
    let Some(account) = microsoft_account(&state).await else {
        return;
    };
    let key = match friends::identity(&state.ctx.data) {
        Ok(key) => key,
        Err(err) => {
            tracing::warn!(error = %err, "no key for friends");
            return;
        }
    };
    let presence = desired_presence(&state).await;
    match Service::start(
        state.ctx.db.clone(),
        state.ctx.http.clone(),
        key,
        me(&account),
        presence,
    )
    .await
    {
        Ok(service) => {
            let mut events = service.subscribe();
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                while let Ok(event) = events.recv().await {
                    let _ = handle.emit(EVENT, &event);
                }
            });
            *slot = Some(service);
        }
        Err(err) => tracing::warn!(error = %err, "friends service not started"),
    }
}

/// Starts the service at launch, then keeps the shared presence and profile current.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        ensure_started(&app).await;
        loop {
            tokio::time::sleep(PRESENCE_EVERY).await;
            let state = app.state::<AppState>();
            let service = state.friends.service.lock().await.clone();
            match service {
                Some(service) => {
                    if let Some(account) = microsoft_account(&state).await {
                        service.set_me(me(&account));
                    }
                    service.set_presence(desired_presence(&state).await).await;
                }
                None => ensure_started(&app).await,
            }
        }
    });
}

/// What friends should see: hosting, else playing, else online; nothing when hidden.
async fn desired_presence(state: &AppState) -> Option<Presence> {
    let db = &state.ctx.db;
    if db.get_setting::<bool>(HIDDEN_SETTING).await.ok().flatten() == Some(true) {
        return None;
    }
    let name = |id: Option<String>| async move {
        match id {
            Some(id) => db.get_instance(&id).await.ok().map(|i| i.name),
            None => None,
        }
    };
    if let Some(host) = state.duo.host_view().await {
        let open = db
            .get_setting::<bool>(OPEN_TO_FRIENDS_SETTING)
            .await
            .ok()
            .flatten()
            == Some(true);
        return Some(Presence::Hosting {
            instance: name(host.instance_id).await,
            world: host.world.map(|w| w.motd),
            code: open.then_some(host.code),
        });
    }
    if let Some(id) = state.games.running_ids().into_iter().next() {
        if let Some(instance) = name(Some(id)).await {
            return Some(Presence::Playing { instance });
        }
    }
    Some(Presence::Online)
}

async fn service(state: &AppState) -> CommandResult<Service> {
    state
        .friends
        .service
        .lock()
        .await
        .clone()
        .ok_or_else(|| CommandError::msg("Connecte un compte Microsoft pour avoir des amis."))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendsOverview {
    /// The service runs (a Microsoft account is there).
    pub available: bool,
    pub code: Option<String>,
    pub friends: Vec<FriendView>,
    pub hidden: bool,
    pub open_to_friends: bool,
}

#[tauri::command]
pub async fn friends_overview(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<FriendsOverview> {
    ensure_started(&app).await;
    let db = &state.ctx.db;
    let hidden = db
        .get_setting::<bool>(HIDDEN_SETTING)
        .await?
        .unwrap_or(false);
    let open_to_friends = db
        .get_setting::<bool>(OPEN_TO_FRIENDS_SETTING)
        .await?
        .unwrap_or(false);
    let service = state.friends.service.lock().await.clone();
    Ok(match service {
        Some(service) => FriendsOverview {
            available: true,
            code: Some(service.code()),
            friends: service.list().await?,
            hidden,
            open_to_friends,
        },
        None => FriendsOverview {
            available: false,
            code: None,
            friends: Vec::new(),
            hidden,
            open_to_friends,
        },
    })
}

#[tauri::command]
pub async fn friends_add(state: State<'_, AppState>, code: String) -> CommandResult<()> {
    service(&state).await?.add(&state.ctx.http, &code).await?;
    Ok(())
}

#[tauri::command]
pub async fn friends_accept(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    Ok(service(&state).await?.accept(&id).await?)
}

/// Removes a friend, cancels a request sent or declines one received.
#[tauri::command]
pub async fn friends_remove(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    Ok(service(&state).await?.remove(&id).await?)
}

/// Invites a friend to the world the player hosts right now.
#[tauri::command]
pub async fn friends_invite(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    let host = state
        .duo
        .host_view()
        .await
        .ok_or_else(|| CommandError::msg("Ouvre d'abord une partie à rejoindre."))?;
    let instance = match host.instance_id {
        Some(id) => state.ctx.db.get_instance(&id).await.ok().map(|i| i.name),
        None => None,
    };
    let world = host.world.map(|w| w.motd);
    Ok(service(&state)
        .await?
        .invite(&id, &host.code, instance, world)
        .await?)
}

#[tauri::command]
pub async fn friends_set_hidden(state: State<'_, AppState>, hidden: bool) -> CommandResult<()> {
    state.ctx.db.set_setting(HIDDEN_SETTING, &hidden).await?;
    if let Some(service) = state.friends.service.lock().await.clone() {
        service.set_presence(desired_presence(&state).await).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn friends_set_open(state: State<'_, AppState>, open: bool) -> CommandResult<()> {
    state
        .ctx
        .db
        .set_setting(OPEN_TO_FRIENDS_SETTING, &open)
        .await?;
    if let Some(service) = state.friends.service.lock().await.clone() {
        service.set_presence(desired_presence(&state).await).await;
    }
    Ok(())
}

/// Friends see the player leave at once.
pub async fn shutdown(state: &AppState) {
    if let Some(service) = state.friends.service.lock().await.take() {
        service.shutdown().await;
    }
}
