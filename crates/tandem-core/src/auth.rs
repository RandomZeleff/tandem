//! Microsoft → Xbox Live → XSTS → Minecraft authentication (DECISIONS D9, D36).
//!
//! Sign-in happens in the system browser (authorization code + PKCE, answered on a
//! one-shot `http://localhost:<port>` listener). Tokens only live in the OS credential
//! store; the database keeps the profile (name, UUID, face of the skin).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::account::Account;
use crate::context::Context;
use crate::error::{Error, Result};
use crate::secrets;

/// Azure application (client) ID. Public by design: native apps cannot keep secrets.
/// Allow-listed by Mojang on 2026-10-10 to reach api.minecraftservices.com.
pub const MSA_CLIENT_ID: &str = "47ec2f94-9a22-4089-95c2-e2cbbc4afd44";

const AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";
const XBL_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

/// A Minecraft token this close to expiry is renewed before launching.
const EXPIRY_MARGIN_SECS: i64 = 10 * 60;

/// Serialises renewals: two games started together must not both spend the refresh token
/// (Microsoft rotates it on every use).
static RENEWAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

fn secret_name(account_id: &str) -> String {
    format!("msa:{account_id}")
}

/// What is kept in the credential store for one Microsoft account.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tokens {
    refresh_token: String,
    minecraft_token: String,
    /// Unix seconds.
    minecraft_expires_at: i64,
}

fn load_tokens(account_id: &str) -> Result<Option<Tokens>> {
    Ok(secrets::get_long(&secret_name(account_id))?
        .and_then(|json| serde_json::from_str(&json).ok()))
}

fn save_tokens(account_id: &str, tokens: &Tokens) -> Result<()> {
    secrets::set_long(&secret_name(account_id), &serde_json::to_string(tokens)?)
}

/// Forgets an account's tokens (when it is removed).
pub fn forget(account_id: &str) -> Result<()> {
    secrets::set_long(&secret_name(account_id), "")
}

// ---------------------------------------------------------------------------------------
// Sign-in

/// Where the sign-in is, for the progress shown while it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Step {
    /// Waiting for the player to finish in the browser.
    Browser,
    Microsoft,
    Xbox,
    Minecraft,
    Profile,
}

/// A sign-in started in the browser, waiting for Microsoft to call back.
pub struct PendingLogin {
    /// Page to open in the system browser.
    pub url: String,
    listener: TcpListener,
    redirect_uri: String,
    verifier: String,
    state: String,
}

fn random_token() -> String {
    let mut bytes = Vec::with_capacity(48);
    for _ in 0..3 {
        bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    }
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn query_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn query_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = (bytes[i] == b'%')
            .then(|| value.get(i + 1..i + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match (bytes[i], escaped) {
            (_, Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b'+', None) => {
                out.push(b' ');
                i += 1;
            }
            (b, None) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Opens the local listener and builds the Microsoft sign-in page URL.
pub async fn start_login() -> Result<PendingLogin> {
    // The browser resolves `localhost` itself; IPv4 loopback answers on every system.
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://localhost:{port}");
    let verifier = random_token();
    let state = random_token();
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    let url = format!(
        "{AUTHORIZE_URL}?client_id={MSA_CLIENT_ID}&response_type=code&response_mode=query\
         &redirect_uri={}&scope={}&state={state}&code_challenge={challenge}\
         &code_challenge_method=S256&prompt=select_account",
        query_encode(&redirect_uri),
        query_encode(SCOPE),
    );
    Ok(PendingLogin {
        url,
        listener,
        redirect_uri,
        verifier,
        state,
    })
}

/// Page the browser shows once it has handed the answer to Tandem.
fn callback_page(ok: bool) -> String {
    let (title, text) = if ok {
        (
            "Connexion réussie",
            "Tu peux fermer cet onglet et revenir dans Tandem.",
        )
    } else {
        (
            "Connexion annulée",
            "Rien n'a été enregistré. Tu peux fermer cet onglet et réessayer depuis Tandem.",
        )
    };
    format!(
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><title>Tandem · {title}</title>\
         <style>body{{margin:0;height:100vh;display:grid;place-items:center;background:#111316;color:#e8e6e3;\
         font:16px/1.5 system-ui,sans-serif}}main{{padding:32px 40px;background:#1b1e23;\
         box-shadow:inset 0 0 0 2px #2a2e35;text-align:center}}h1{{margin:0 0 8px;font-size:22px;\
         color:{}}}p{{margin:0;color:#a9adb4}}</style></head><body><main><h1>{title}</h1><p>{text}</p>\
         </main></body></html>",
        if ok { "#7fd34e" } else { "#e8b54a" }
    )
}

impl PendingLogin {
    /// Waits for Microsoft's answer in the browser, then completes the sign-in. Drop the
    /// future to cancel.
    pub async fn finish(self, ctx: &Context, on_step: impl Fn(Step)) -> Result<Account> {
        on_step(Step::Browser);
        let code = self.wait_for_code().await?;
        on_step(Step::Microsoft);
        let msa: MsaToken = form_post(
            ctx,
            TOKEN_URL,
            &[
                ("client_id", MSA_CLIENT_ID),
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", &self.redirect_uri),
                ("code_verifier", &self.verifier),
                ("scope", SCOPE),
            ],
        )
        .await?;
        let chain = minecraft_chain(ctx, &msa.access_token, &on_step).await?;
        on_step(Step::Profile);
        let profile = owned_profile(ctx, &chain.access_token).await?;
        let avatar = skin_face(ctx, &profile).await;
        let account = ctx
            .db
            .upsert_microsoft_account(&profile.name, &dashed(&profile.id), avatar.as_deref())
            .await?;
        let tokens = Tokens {
            refresh_token: msa.refresh_token.unwrap_or_default(),
            minecraft_token: chain.access_token,
            minecraft_expires_at: now() + chain.expires_in,
        };
        let id = account.id.clone();
        tokio::task::spawn_blocking(move || save_tokens(&id, &tokens))
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e)))??;
        tracing::info!(username = %account.username, "Microsoft account signed in");
        Ok(account)
    }

    async fn wait_for_code(&self) -> Result<String> {
        loop {
            let (mut socket, _) = self.listener.accept().await?;
            let mut buffer = vec![0u8; 8192];
            let read = tokio::time::timeout(Duration::from_secs(10), socket.read(&mut buffer))
                .await
                .unwrap_or(Ok(0))
                .unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]);
            let target = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("");
            let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
            let params: Vec<(String, String)> = query
                .split('&')
                .filter_map(|pair| pair.split_once('='))
                .map(|(k, v)| (k.to_owned(), query_decode(v)))
                .collect();
            let param = |name: &str| {
                params
                    .iter()
                    .find(|(k, _)| k == name)
                    .map(|(_, v)| v.clone())
            };
            // Favicon and other stray requests: not the answer.
            if param("code").is_none() && param("error").is_none() {
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await;
                continue;
            }
            let state_ok = param("state").as_deref() == Some(self.state.as_str());
            let ok = state_ok && param("code").is_some();
            let page = callback_page(ok);
            let _ = socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
                        page.len()
                    )
                    .as_bytes(),
                )
                .await;
            let _ = socket.shutdown().await;
            if !state_ok {
                return Err(Error::Auth(
                    "Réponse de connexion inattendue : recommence depuis Tandem".into(),
                ));
            }
            return match (param("code"), param("error")) {
                (Some(code), _) => Ok(code),
                (None, Some(error)) if error == "access_denied" => {
                    Err(Error::Auth("Connexion annulée dans le navigateur".into()))
                }
                (None, error) => Err(Error::Auth(format!(
                    "Microsoft a refusé la connexion : {}",
                    param("error_description").or(error).unwrap_or_default()
                ))),
            };
        }
    }
}

// ---------------------------------------------------------------------------------------
// Token chain

#[derive(Deserialize)]
struct MsaToken {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct OAuthError {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_description: String,
}

async fn form_post<T: for<'de> Deserialize<'de>>(
    ctx: &Context,
    url: &str,
    form: &[(&str, &str)],
) -> Result<T> {
    let response = ctx.http.post(url).form(form).send().await?;
    if response.status().is_success() {
        return Ok(response.json().await?);
    }
    let status = response.status().as_u16();
    let body: OAuthError = response.json().await.unwrap_or(OAuthError {
        error: String::new(),
        error_description: String::new(),
    });
    tracing::warn!(status, error = %body.error, description = %body.error_description, "Microsoft token request refused");
    if body.error == "invalid_grant" {
        return Err(Error::LoginRequired);
    }
    Err(Error::Auth(format!(
        "Microsoft a refusé la connexion ({})",
        if body.error.is_empty() {
            status.to_string()
        } else {
            body.error
        }
    )))
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxToken {
    token: String,
    display_claims: XboxClaims,
}

#[derive(Deserialize)]
struct XboxClaims {
    xui: Vec<XboxUser>,
}

#[derive(Deserialize)]
struct XboxUser {
    uhs: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XstsError {
    #[serde(default)]
    x_err: u64,
}

/// What the player can do about an XSTS refusal.
fn xsts_message(code: u64) -> String {
    match code {
        2148916233 => {
            "Ce compte Microsoft n'a pas encore de profil Xbox. Crée-le gratuitement sur \
                       https://www.xbox.com/live puis reconnecte-toi."
                .into()
        }
        2148916235 => "Xbox Live n'est pas disponible dans le pays de ce compte.".into(),
        2148916236 | 2148916237 => {
            "Ce compte doit d'abord être vérifié comme adulte sur https://account.xbox.com.".into()
        }
        2148916238 => "C'est le compte d'un enfant : un adulte doit l'ajouter à une famille \
                       Microsoft (https://account.microsoft.com/family) pour qu'il puisse jouer."
            .into(),
        code => format!("Xbox Live a refusé la connexion (code {code})."),
    }
}

#[derive(Deserialize)]
struct MinecraftLogin {
    access_token: String,
    expires_in: i64,
}

/// Microsoft access token → Xbox Live → XSTS → Minecraft access token.
async fn minecraft_chain(
    ctx: &Context,
    msa_token: &str,
    on_step: &impl Fn(Step),
) -> Result<MinecraftLogin> {
    on_step(Step::Xbox);
    let xbl: XboxToken = json_post(
        ctx,
        XBL_URL,
        &serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={msa_token}"),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }),
    )
    .await?;
    let response = ctx
        .http
        .post(XSTS_URL)
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl.token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }))
        .send()
        .await?;
    if response.status().as_u16() == 401 {
        let error: XstsError = response.json().await.unwrap_or(XstsError { x_err: 0 });
        return Err(Error::Auth(xsts_message(error.x_err)));
    }
    let xsts: XboxToken = checked(response).await?.json().await?;
    let uhs = xsts
        .display_claims
        .xui
        .first()
        .map(|u| u.uhs.clone())
        .ok_or_else(|| Error::Auth("Réponse Xbox Live incomplète".into()))?;

    on_step(Step::Minecraft);
    let response = ctx
        .http
        .post(MC_LOGIN_URL)
        .json(&serde_json::json!({ "identityToken": format!("XBL3.0 x={uhs};{}", xsts.token) }))
        .send()
        .await?;
    match response.status().as_u16() {
        403 => Err(Error::Auth(
            "Minecraft refuse cette application pour l'instant (accès non autorisé par Mojang)."
                .into(),
        )),
        429 => Err(Error::Auth(
            "Trop de connexions d'un coup : réessaie dans une minute.".into(),
        )),
        _ => Ok(checked(response).await?.json().await?),
    }
}

async fn json_post<T: for<'de> Deserialize<'de>>(
    ctx: &Context,
    url: &str,
    body: &serde_json::Value,
) -> Result<T> {
    let response = ctx
        .http
        .post(url)
        .header("Accept", "application/json")
        .json(body)
        .send()
        .await?;
    Ok(checked(response).await?.json().await?)
}

async fn checked(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status().as_u16();
    if (200..300).contains(&status) {
        Ok(response)
    } else {
        Err(Error::HttpStatus {
            url: response.url().as_str().to_owned(),
            status,
        })
    }
}

// ---------------------------------------------------------------------------------------
// Profile and ownership

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    /// UUID without dashes.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<Skin>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Skin {
    pub url: String,
    #[serde(default)]
    pub state: String,
}

#[derive(Deserialize)]
struct Entitlements {
    #[serde(default)]
    items: Vec<serde_json::Value>,
}

fn dashed(id: &str) -> String {
    uuid::Uuid::parse_str(id)
        .map(|u| u.hyphenated().to_string())
        .unwrap_or_else(|_| id.to_owned())
}

/// The Minecraft profile, refusing accounts that do not own the game (D9).
async fn owned_profile(ctx: &Context, token: &str) -> Result<Profile> {
    let entitlements: Entitlements = checked(
        ctx.http
            .get(MC_ENTITLEMENTS_URL)
            .bearer_auth(token)
            .send()
            .await?,
    )
    .await?
    .json()
    .await?;
    let response = ctx
        .http
        .get(MC_PROFILE_URL)
        .bearer_auth(token)
        .send()
        .await?;
    if response.status().as_u16() == 404 {
        return Err(Error::Auth(if entitlements.items.is_empty() {
            "Ce compte ne possède pas Minecraft: Java Edition. Il s'achète sur \
             https://www.minecraft.net (le Xbox Game Pass pour PC l'inclut aussi)."
                .into()
        } else {
            "Ce compte n'a pas encore de pseudo Minecraft. Choisis-le en lançant une fois le \
             launcher officiel ou sur https://www.minecraft.net/msaprofile, puis reconnecte-toi."
                .into()
        }));
    }
    Ok(checked(response).await?.json().await?)
}

/// Face of the active skin (head and hat layer), as a PNG data URL. `None` if unreachable.
async fn skin_face(ctx: &Context, profile: &Profile) -> Option<String> {
    let skin = profile
        .skins
        .iter()
        .find(|s| s.state == "ACTIVE")
        .or(profile.skins.first())?;
    // Skins are served only from Mojang's texture server.
    if !skin.url.starts_with("http://textures.minecraft.net/")
        && !skin.url.starts_with("https://textures.minecraft.net/")
    {
        return None;
    }
    let url = skin.url.replacen("http://", "https://", 1);
    let bytes = ctx.http.get(url).send().await.ok()?.bytes().await.ok()?;
    tokio::task::spawn_blocking(move || face_from_skin(&bytes))
        .await
        .ok()?
}

/// 8×8 face at (8, 8) with the hat layer at (40, 8) drawn over it, scaled ×8 without smoothing.
pub fn face_from_skin(png: &[u8]) -> Option<String> {
    let skin = image::load_from_memory(png).ok()?.to_rgba8();
    if skin.width() < 64 || skin.height() < 32 {
        return None;
    }
    let mut face = image::RgbaImage::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            let base = *skin.get_pixel(8 + x, 8 + y);
            let hat = *skin.get_pixel(40 + x, 8 + y);
            let pixel = if hat[3] > 0 { hat } else { base };
            face.put_pixel(x, y, image::Rgba([pixel[0], pixel[1], pixel[2], 255]));
        }
    }
    let big = image::imageops::resize(&face, 64, 64, image::imageops::FilterType::Nearest);
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(big)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ))
}

// ---------------------------------------------------------------------------------------
// Launch

/// A valid Minecraft access token for a Microsoft account, renewed when it is about to
/// expire. Without network, a stale token is returned: single player still works.
pub async fn access_token(ctx: &Context, account: &Account) -> Result<String> {
    let _renewing = RENEWAL.lock().await;
    let id = account.id.clone();
    let tokens = tokio::task::spawn_blocking(move || load_tokens(&id))
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e)))??
        .ok_or(Error::LoginRequired)?;
    if tokens.minecraft_expires_at - EXPIRY_MARGIN_SECS > now() {
        return Ok(tokens.minecraft_token);
    }
    match renew(ctx, account, &tokens).await {
        Ok(token) => Ok(token),
        Err(err) if err.is_network() && !tokens.minecraft_token.is_empty() => {
            tracing::warn!(error = %err, "could not renew the Minecraft token, using the old one");
            Ok(tokens.minecraft_token)
        }
        Err(err) => Err(err),
    }
}

async fn renew(ctx: &Context, account: &Account, tokens: &Tokens) -> Result<String> {
    let msa: MsaToken = form_post(
        ctx,
        TOKEN_URL,
        &[
            ("client_id", MSA_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", &tokens.refresh_token),
            ("scope", SCOPE),
        ],
    )
    .await?;
    let chain = minecraft_chain(ctx, &msa.access_token, &|_| {}).await?;
    let renewed = Tokens {
        refresh_token: msa
            .refresh_token
            .unwrap_or_else(|| tokens.refresh_token.clone()),
        minecraft_token: chain.access_token.clone(),
        minecraft_expires_at: now() + chain.expires_in,
    };
    let id = account.id.clone();
    tokio::task::spawn_blocking(move || save_tokens(&id, &renewed))
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e)))??;
    // The player may have changed name or skin since last time.
    if let Ok(profile) = owned_profile(ctx, &chain.access_token).await {
        let avatar = skin_face(ctx, &profile).await;
        if let Err(err) = ctx
            .db
            .upsert_microsoft_account(&profile.name, &dashed(&profile.id), avatar.as_deref())
            .await
        {
            tracing::warn!(error = %err, "could not refresh the profile");
        }
    }
    tracing::info!(username = %account.username, "Minecraft token renewed");
    Ok(chain.access_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_decodes_query_values() {
        assert_eq!(
            query_encode("http://localhost:1234"),
            "http%3A%2F%2Flocalhost%3A1234"
        );
        assert_eq!(
            query_encode("XboxLive.signin offline_access"),
            "XboxLive.signin%20offline_access"
        );
        assert_eq!(query_decode("M.C5_BAY.2%21abc%2Bd+e"), "M.C5_BAY.2!abc+d e");
        assert_eq!(query_decode("bad%zz%"), "bad%zz%");
    }

    #[test]
    fn explains_xsts_refusals() {
        assert!(xsts_message(2148916233).contains("profil Xbox"));
        assert!(xsts_message(2148916238).contains("famille"));
        assert!(xsts_message(1).contains("code 1"));
    }

    #[test]
    fn crops_the_face_with_its_hat() {
        let mut skin = image::RgbaImage::new(64, 64);
        for y in 8..16 {
            for x in 8..16 {
                skin.put_pixel(x, y, image::Rgba([200, 150, 100, 255]));
            }
        }
        // Hat pixel over the top-left corner of the face.
        skin.put_pixel(40, 8, image::Rgba([10, 20, 30, 255]));
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(skin)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let url = face_from_skin(&png).unwrap();
        let data = base64::engine::general_purpose::STANDARD
            .decode(url.trim_start_matches("data:image/png;base64,"))
            .unwrap();
        let face = image::load_from_memory(&data).unwrap().to_rgba8();
        assert_eq!(face.dimensions(), (64, 64));
        assert_eq!(*face.get_pixel(0, 0), image::Rgba([10, 20, 30, 255]));
        assert_eq!(*face.get_pixel(63, 63), image::Rgba([200, 150, 100, 255]));
        assert!(face_from_skin(b"not a png").is_none());
    }

    #[tokio::test]
    async fn login_url_uses_pkce_and_a_local_redirect() {
        let login = start_login().await.unwrap();
        assert!(login.url.starts_with(AUTHORIZE_URL));
        assert!(login.url.contains("code_challenge_method=S256"));
        assert!(login.url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A"));
        assert!(login.redirect_uri.starts_with("http://localhost:"));
        assert_ne!(login.verifier, login.state);
    }

    #[tokio::test]
    async fn reads_the_code_from_the_browser_callback() {
        let login = start_login().await.unwrap();
        let port = login.listener.local_addr().unwrap().port();
        let state = login.state.clone();
        let browser = tokio::spawn(async move {
            // A stray request first, as browsers do.
            for path in [
                "/favicon.ico".to_owned(),
                format!("/?code=M.C1%2Babc&state={state}"),
            ] {
                let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
                    .await
                    .unwrap();
                socket
                    .write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
                    .await
                    .unwrap();
                let mut page = String::new();
                socket.read_to_string(&mut page).await.unwrap();
                if path.contains("code") {
                    assert!(page.contains("Connexion réussie"));
                }
            }
        });
        assert_eq!(login.wait_for_code().await.unwrap(), "M.C1+abc");
        browser.await.unwrap();

        let login = start_login().await.unwrap();
        let port = login.listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .unwrap();
            let _ = socket
                .write_all(b"GET /?code=x&state=forged HTTP/1.1\r\n\r\n")
                .await;
        });
        assert!(login.wait_for_code().await.is_err());
    }
}
