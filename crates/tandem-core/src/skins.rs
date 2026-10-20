//! Skins and capes of a Microsoft account (Mojang's profile API), and the player's own
//! library of skins kept on this PC (`<data>/skins/<sha1>.png`, names in the settings).

use std::io::Read;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::account::{Account, AccountKind};
use crate::auth;
use crate::error::{Error, Result};
use crate::Context;

const PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const SKINS_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const ACTIVE_SKIN_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins/active";
const ACTIVE_CAPE_URL: &str = "https://api.minecraftservices.com/minecraft/profile/capes/active";
const NAME_LOOKUP_URL: &str = "https://api.mojang.com/users/profiles/minecraft/";
const SESSION_PROFILE_URL: &str = "https://sessionserver.mojang.com/session/minecraft/profile/";

/// Setting holding the library's names and models, in the order shown.
const LIBRARY_SETTING: &str = "skin_library";
/// Skins and capes are only ever downloaded from Mojang's texture server.
const TEXTURE_HOSTS: [&str; 2] = [
    "http://textures.minecraft.net/",
    "https://textures.minecraft.net/",
];
/// Mojang refuses bigger files; a skin is a few kilobytes.
const MAX_SKIN_BYTES: usize = 24 * 1024;

/// The nine skins every player can pick (1.19.3+), as named in the game files.
pub const DEFAULT_SKINS: [&str; 9] = [
    "steve", "alex", "ari", "efe", "kai", "makena", "noor", "sunny", "zuri",
];

/// Arm width of a skin: 4 pixels ("classic", Steve) or 3 ("slim", Alex).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Variant {
    Classic,
    Slim,
}

impl Variant {
    fn api_name(self) -> &'static str {
        match self {
            Variant::Classic => "classic",
            Variant::Slim => "slim",
        }
    }

    fn from_api(value: &str) -> Self {
        if value.eq_ignore_ascii_case("slim") {
            Variant::Slim
        } else {
            Variant::Classic
        }
    }
}

// ---------------------------------------------------------------------------------------
// The account's skin and capes

#[derive(Debug, Deserialize)]
struct ApiProfile {
    #[serde(default)]
    skins: Vec<ApiTexture>,
    #[serde(default)]
    capes: Vec<ApiTexture>,
}

#[derive(Debug, Deserialize)]
struct ApiTexture {
    id: String,
    #[serde(default)]
    state: String,
    url: String,
    #[serde(default)]
    variant: Option<String>,
    #[serde(default)]
    alias: Option<String>,
}

impl ApiTexture {
    fn active(&self) -> bool {
        self.state == "ACTIVE"
    }
}

/// What the account wears now.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wardrobe {
    /// The skin as a PNG data URL, `None` if it could not be downloaded.
    pub skin: Option<String>,
    pub variant: Variant,
    /// Its id in the library, when the player saved it there.
    pub library_id: Option<String>,
    pub capes: Vec<Cape>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cape {
    pub id: String,
    pub name: String,
    pub active: bool,
    /// The cape texture (64×32) as a PNG data URL.
    pub texture: Option<String>,
}

fn microsoft(account: &Account) -> Result<()> {
    if account.kind == AccountKind::Microsoft {
        Ok(())
    } else {
        Err(Error::InvalidInput(
            "Les skins se changent avec un compte Microsoft.".into(),
        ))
    }
}

async fn checked(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status().as_u16();
    match status {
        200..=299 => Ok(response),
        401 => Err(Error::LoginRequired),
        429 => Err(Error::InvalidInput(
            "Mojang limite les changements de skin : réessaie dans une minute.".into(),
        )),
        _ => Err(Error::HttpStatus {
            url: response.url().as_str().to_owned(),
            status,
        }),
    }
}

async fn texture(ctx: &Context, url: &str) -> Result<Vec<u8>> {
    if !TEXTURE_HOSTS.iter().any(|host| url.starts_with(host)) {
        return Err(Error::InvalidInput(
            "Texture hors du serveur de Mojang.".into(),
        ));
    }
    let url = url.replacen("http://", "https://", 1);
    let bytes = checked(ctx.http.get(url).send().await?)
        .await?
        .bytes()
        .await?;
    Ok(bytes.to_vec())
}

fn data_url(png: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    )
}

/// Readable name of a cape: Mojang gives aliases like `MIGRATOR` or `Pan`.
fn cape_name(alias: Option<&str>) -> String {
    let Some(alias) = alias.filter(|a| !a.is_empty()) else {
        return "Cape".into();
    };
    alias
        .split(['_', ' '])
        .filter(|w| !w.is_empty())
        .map(|word| {
            let lower = word.to_lowercase();
            let mut chars = lower.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

async fn wardrobe_from(ctx: &Context, profile: ApiProfile) -> Result<Wardrobe> {
    let skin = profile
        .skins
        .iter()
        .find(|s| s.active())
        .or(profile.skins.first());
    let variant = skin
        .and_then(|s| s.variant.as_deref())
        .map_or(Variant::Classic, Variant::from_api);
    let png = match skin {
        Some(skin) => texture(ctx, &skin.url).await.ok(),
        None => None,
    };
    let library_id = png.as_deref().map(skin_id);
    let library_id = match library_id {
        Some(id) if library_path(ctx, &id).exists() => Some(id),
        _ => None,
    };

    let mut capes = Vec::with_capacity(profile.capes.len());
    for cape in &profile.capes {
        capes.push(Cape {
            id: cape.id.clone(),
            name: cape_name(cape.alias.as_deref()),
            active: cape.active(),
            texture: texture(ctx, &cape.url).await.ok().map(|png| data_url(&png)),
        });
    }
    Ok(Wardrobe {
        skin: png.as_deref().map(data_url),
        variant,
        library_id,
        capes,
    })
}

async fn fetch_profile(ctx: &Context, token: &str) -> Result<ApiProfile> {
    Ok(
        checked(ctx.http.get(PROFILE_URL).bearer_auth(token).send().await?)
            .await?
            .json()
            .await?,
    )
}

/// The skin and capes of a Microsoft account.
pub async fn wardrobe(ctx: &Context, account: &Account) -> Result<Wardrobe> {
    microsoft(account)?;
    let token = auth::access_token(ctx, account).await?;
    let profile = fetch_profile(ctx, &token).await?;
    wardrobe_from(ctx, profile).await
}

/// Puts a skin from the library (or a default one) on the account.
pub async fn apply(ctx: &Context, account: &Account, skin: &str) -> Result<Wardrobe> {
    microsoft(account)?;
    let (png, variant) = match (skin.strip_prefix("default:"), skin.strip_prefix("pack:")) {
        (Some(name), _) => {
            let (name, variant) = parse_default(name)?;
            (default_skin(ctx, name, variant).await?, variant)
        }
        (_, Some(id)) => {
            let skin = PACK
                .iter()
                .find(|s| s.id == id)
                .ok_or_else(|| Error::InvalidInput("Skin du pack inconnu.".into()))?;
            (skin.png.to_vec(), skin.variant)
        }
        _ => {
            let entry = library_entry(ctx, skin).await?;
            (
                tokio::fs::read(library_path(ctx, skin)).await?,
                entry.variant,
            )
        }
    };
    let token = auth::access_token(ctx, account).await?;
    let form = reqwest::multipart::Form::new()
        .text("variant", variant.api_name())
        .part(
            "file",
            reqwest::multipart::Part::bytes(png.clone())
                .file_name("skin.png")
                .mime_str("image/png")?,
        );
    let profile: ApiProfile = checked(
        ctx.http
            .post(SKINS_URL)
            .bearer_auth(&token)
            .multipart(form)
            .send()
            .await?,
    )
    .await?
    .json()
    .await?;
    tracing::info!(username = %account.username, ?variant, "skin changed");
    refresh_avatar(ctx, account, &png).await;
    wardrobe_from(ctx, profile).await
}

/// Wears the cape (`None`: no cape).
pub async fn set_cape(ctx: &Context, account: &Account, cape: Option<&str>) -> Result<Wardrobe> {
    microsoft(account)?;
    let token = auth::access_token(ctx, account).await?;
    let request = match cape {
        Some(id) => ctx
            .http
            .put(ACTIVE_CAPE_URL)
            .json(&serde_json::json!({ "capeId": id })),
        None => ctx.http.delete(ACTIVE_CAPE_URL),
    };
    let profile: ApiProfile = checked(request.bearer_auth(&token).send().await?)
        .await?
        .json()
        .await?;
    wardrobe_from(ctx, profile).await
}

/// Goes back to the default skin Mojang picks for this account.
pub async fn reset(ctx: &Context, account: &Account) -> Result<Wardrobe> {
    microsoft(account)?;
    let token = auth::access_token(ctx, account).await?;
    checked(
        ctx.http
            .delete(ACTIVE_SKIN_URL)
            .bearer_auth(&token)
            .send()
            .await?,
    )
    .await?;
    let wardrobe = wardrobe(ctx, account).await?;
    if let Some(png) = wardrobe.skin.as_deref().and_then(decode_data_url) {
        refresh_avatar(ctx, account, &png).await;
    }
    Ok(wardrobe)
}

fn decode_data_url(url: &str) -> Option<Vec<u8>> {
    let encoded = url.strip_prefix("data:image/png;base64,")?;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()
}

/// The face in the sidebar follows the new skin at once.
async fn refresh_avatar(ctx: &Context, account: &Account, png: &[u8]) {
    let png = png.to_vec();
    let face = tokio::task::spawn_blocking(move || auth::face_from_skin(&png))
        .await
        .ok()
        .flatten();
    if let Some(face) = face {
        if let Err(err) = ctx.db.set_account_avatar(&account.id, &face).await {
            tracing::warn!(error = %err, "could not update the account picture");
        }
    }
}

// ---------------------------------------------------------------------------------------
// Library on this PC

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryEntry {
    /// SHA-1 of the PNG: the same skin is only kept once.
    id: String,
    name: String,
    variant: Variant,
}

/// A skin of the library or a default one, ready to show.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySkin {
    /// Library id, or `default:<name>:<variant>`.
    pub id: String,
    pub name: String,
    pub variant: Variant,
    /// The skin as a PNG data URL.
    pub texture: String,
}

fn skins_dir(ctx: &Context) -> PathBuf {
    ctx.data.root().join("skins")
}

fn library_path(ctx: &Context, id: &str) -> PathBuf {
    skins_dir(ctx).join(format!("{id}.png"))
}

fn skin_id(png: &[u8]) -> String {
    sha1_smol::Sha1::from(png).digest().to_string()
}

async fn entries(ctx: &Context) -> Result<Vec<LibraryEntry>> {
    Ok(ctx
        .db
        .get_setting(LIBRARY_SETTING)
        .await?
        .unwrap_or_default())
}

async fn library_entry(ctx: &Context, id: &str) -> Result<LibraryEntry> {
    entries(ctx)
        .await?
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| Error::InvalidInput("Ce skin n'est plus dans ta bibliothèque.".into()))
}

/// The saved skins, newest first.
pub async fn library(ctx: &Context) -> Result<Vec<LibrarySkin>> {
    let mut skins = Vec::new();
    for entry in entries(ctx).await? {
        match tokio::fs::read(library_path(ctx, &entry.id)).await {
            Ok(png) => skins.push(LibrarySkin {
                texture: data_url(&png),
                id: entry.id,
                name: entry.name,
                variant: entry.variant,
            }),
            Err(err) => tracing::warn!(id = %entry.id, error = %err, "skin file missing"),
        }
    }
    Ok(skins)
}

/// Checks a PNG is a Minecraft skin (64×64, or the old 64×32) and guesses its model.
pub fn inspect(png: &[u8]) -> Result<Variant> {
    let invalid = || {
        Error::InvalidInput(
            "Ce fichier n'est pas un skin Minecraft (PNG de 64 × 64 pixels).".into(),
        )
    };
    if png.len() > MAX_SKIN_BYTES {
        return Err(invalid());
    }
    let format = image::guess_format(png).map_err(|_| invalid())?;
    if format != image::ImageFormat::Png {
        return Err(invalid());
    }
    let image = image::load_from_memory(png)
        .map_err(|_| invalid())?
        .to_rgba8();
    match (image.width(), image.height()) {
        (64, 64) => {
            // A slim arm is 3 pixels wide: the 4th column of the right arm's back is empty.
            let slim = (20..32).all(|y| image.get_pixel(55, y)[3] == 0)
                && (20..32).any(|y| image.get_pixel(54, y)[3] == 0);
            Ok(if slim {
                Variant::Slim
            } else {
                Variant::Classic
            })
        }
        (64, 32) => Ok(Variant::Classic),
        _ => Err(invalid()),
    }
}

async fn add(
    ctx: &Context,
    png: Vec<u8>,
    name: String,
    variant: Option<Variant>,
) -> Result<LibrarySkin> {
    let guessed = inspect(&png)?;
    let variant = variant.unwrap_or(guessed);
    let id = skin_id(&png);
    tokio::fs::create_dir_all(skins_dir(ctx)).await?;
    tokio::fs::write(library_path(ctx, &id), &png).await?;

    let mut list = entries(ctx).await?;
    let name = match list.iter().position(|e| e.id == id) {
        // Already saved: moved to the top, keeping the name the player gave it.
        Some(i) => list.remove(i).name,
        None => name,
    };
    let entry = LibraryEntry {
        id: id.clone(),
        name,
        variant,
    };
    list.insert(0, entry.clone());
    ctx.db.set_setting(LIBRARY_SETTING, &list).await?;
    Ok(LibrarySkin {
        id,
        name: entry.name,
        variant,
        texture: data_url(&png),
    })
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Skin".into())
}

/// Saves a PNG chosen or dropped by the player.
pub async fn add_file(ctx: &Context, path: &Path) -> Result<LibrarySkin> {
    let png = tokio::fs::read(path).await?;
    add(ctx, png, file_stem(path), None).await
}

#[derive(Deserialize)]
struct NameLookup {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct SessionProfile {
    #[serde(default)]
    properties: Vec<SessionProperty>,
}

#[derive(Deserialize)]
struct SessionProperty {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct TexturesValue {
    textures: Textures,
}

#[derive(Deserialize)]
struct Textures {
    #[serde(rename = "SKIN")]
    skin: Option<SkinTexture>,
}

#[derive(Deserialize)]
struct SkinTexture {
    url: String,
    #[serde(default)]
    metadata: Option<SkinMetadata>,
}

#[derive(Deserialize)]
struct SkinMetadata {
    model: Option<String>,
}

/// Saves the skin another player wears now, found by their name.
pub async fn add_player(ctx: &Context, name: &str) -> Result<LibrarySkin> {
    let name = name.trim();
    let not_found = || Error::InvalidInput(format!("Aucun joueur ne s'appelle « {name} »."));
    if name.is_empty()
        || name.len() > 16
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(not_found());
    }
    let response = ctx
        .http
        .get(format!("{NAME_LOOKUP_URL}{name}"))
        .send()
        .await?;
    if matches!(response.status().as_u16(), 204 | 404) {
        return Err(not_found());
    }
    let player: NameLookup = checked(response).await?.json().await?;
    let session: SessionProfile = checked(
        ctx.http
            .get(format!("{SESSION_PROFILE_URL}{}", player.id))
            .send()
            .await?,
    )
    .await?
    .json()
    .await?;
    let skin = session
        .properties
        .iter()
        .find(|p| p.name == "textures")
        .and_then(|p| {
            base64::engine::general_purpose::STANDARD
                .decode(&p.value)
                .ok()
        })
        .and_then(|raw| serde_json::from_slice::<TexturesValue>(&raw).ok())
        .and_then(|t| t.textures.skin)
        .ok_or_else(|| Error::InvalidInput(format!("{} porte le skin par défaut.", player.name)))?;
    let variant = skin
        .metadata
        .and_then(|m| m.model)
        .map_or(Variant::Classic, |m| Variant::from_api(&m));
    let png = texture(ctx, &skin.url).await?;
    add(ctx, png, player.name, Some(variant)).await
}

pub async fn rename(ctx: &Context, id: &str, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::InvalidInput("Donne un nom à ce skin.".into()));
    }
    let mut list = entries(ctx).await?;
    if let Some(entry) = list.iter_mut().find(|e| e.id == id) {
        entry.name = name.chars().take(40).collect();
    }
    ctx.db.set_setting(LIBRARY_SETTING, &list).await
}

pub async fn set_variant(ctx: &Context, id: &str, variant: Variant) -> Result<()> {
    let mut list = entries(ctx).await?;
    if let Some(entry) = list.iter_mut().find(|e| e.id == id) {
        entry.variant = variant;
    }
    ctx.db.set_setting(LIBRARY_SETTING, &list).await
}

pub async fn remove(ctx: &Context, id: &str) -> Result<()> {
    let mut list = entries(ctx).await?;
    list.retain(|e| e.id != id);
    ctx.db.set_setting(LIBRARY_SETTING, &list).await?;
    match tokio::fs::remove_file(library_path(ctx, id)).await {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err.into()),
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------------------------
// Tandem's own skins, drawn for the launcher (`tools/skins/generate.py`)

struct PackSkin {
    id: &'static str,
    name: &'static str,
    variant: Variant,
    png: &'static [u8],
}

macro_rules! pack_skin {
    ($id:literal, $name:literal, $variant:expr) => {
        PackSkin {
            id: $id,
            name: $name,
            variant: $variant,
            png: include_bytes!(concat!("../assets/skins/", $id, ".png")),
        }
    };
}

const PACK: [PackSkin; 6] = [
    pack_skin!("dino", "Dino", Variant::Classic),
    pack_skin!("astronaute", "Astronaute", Variant::Classic),
    pack_skin!("pirate", "Pirate", Variant::Slim),
    pack_skin!("robot", "Robot", Variant::Classic),
    pack_skin!("mineur", "Mineur", Variant::Classic),
    pack_skin!("exploratrice", "Exploratrice", Variant::Slim),
];

/// The Tandem skin pack, shipped with the launcher.
pub fn pack() -> Vec<LibrarySkin> {
    PACK.iter()
        .map(|s| LibrarySkin {
            id: format!("pack:{}", s.id),
            name: s.name.into(),
            variant: s.variant,
            texture: data_url(s.png),
        })
        .collect()
}

// ---------------------------------------------------------------------------------------
// Default skins, read from a game jar already on this PC

fn parse_default(spec: &str) -> Result<(&'static str, Variant)> {
    let (name, variant) = spec.split_once(':').unwrap_or((spec, "classic"));
    let name = DEFAULT_SKINS
        .iter()
        .find(|n| **n == name)
        .ok_or_else(|| Error::InvalidInput("Skin par défaut inconnu.".into()))?;
    Ok((name, Variant::from_api(variant)))
}

fn defaults_dir(ctx: &Context) -> PathBuf {
    skins_dir(ctx).join("defaults")
}

fn default_file(name: &str, variant: Variant) -> String {
    let folder = match variant {
        Variant::Classic => "wide",
        Variant::Slim => "slim",
    };
    format!("{folder}-{name}.png")
}

/// Copies the default skins out of the newest game jar that has them (1.19.3+), once.
fn extract_defaults(versions: &Path, out: &Path) -> Result<bool> {
    let mut jars: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(versions)?
        .flatten()
        .filter_map(|dir| {
            let id = dir.file_name().to_string_lossy().into_owned();
            let jar = dir.path().join(format!("{id}.jar"));
            let modified = jar.metadata().and_then(|m| m.modified()).ok()?;
            Some((modified, jar))
        })
        .collect();
    jars.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));

    for (_, jar) in jars {
        let Ok(file) = std::fs::File::open(&jar) else {
            continue;
        };
        let Ok(mut archive) = zip::ZipArchive::new(file) else {
            continue;
        };
        if archive
            .by_name("assets/minecraft/textures/entity/player/wide/ari.png")
            .is_err()
        {
            continue;
        }
        std::fs::create_dir_all(out)?;
        for name in DEFAULT_SKINS {
            for (folder, variant) in [("wide", Variant::Classic), ("slim", Variant::Slim)] {
                let entry = format!("assets/minecraft/textures/entity/player/{folder}/{name}.png");
                let Ok(mut file) = archive.by_name(&entry) else {
                    continue;
                };
                let mut png = Vec::new();
                file.read_to_end(&mut png)?;
                std::fs::write(out.join(default_file(name, variant)), png)?;
            }
        }
        return Ok(true);
    }
    Ok(false)
}

async fn ensure_defaults(ctx: &Context) -> Result<bool> {
    let out = defaults_dir(ctx);
    if out.join(default_file("zuri", Variant::Slim)).exists() {
        return Ok(true);
    }
    let versions = ctx.data.versions();
    tokio::task::spawn_blocking(move || extract_defaults(&versions, &out))
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e)))?
}

async fn default_skin(ctx: &Context, name: &str, variant: Variant) -> Result<Vec<u8>> {
    if !ensure_defaults(ctx).await? {
        return Err(Error::InvalidInput(
            "Lance une fois une version 1.19.3 ou plus récente pour avoir les skins par défaut."
                .into(),
        ));
    }
    Ok(tokio::fs::read(defaults_dir(ctx).join(default_file(name, variant))).await?)
}

/// The nine default skins in both models, empty if no recent game was installed yet.
pub async fn defaults(ctx: &Context) -> Result<Vec<LibrarySkin>> {
    if !ensure_defaults(ctx).await? {
        return Ok(Vec::new());
    }
    let mut skins = Vec::new();
    for name in DEFAULT_SKINS {
        for variant in [Variant::Classic, Variant::Slim] {
            let Ok(png) =
                tokio::fs::read(defaults_dir(ctx).join(default_file(name, variant))).await
            else {
                continue;
            };
            let mut title = name.to_owned();
            title[..1].make_ascii_uppercase();
            skins.push(LibrarySkin {
                id: format!("default:{name}:{}", variant.api_name()),
                name: title,
                variant,
                texture: data_url(&png),
            });
        }
    }
    Ok(skins)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skin(width: u32, height: u32, slim: bool) -> Vec<u8> {
        let mut image = image::RgbaImage::from_pixel(width, height, image::Rgba([90, 60, 40, 255]));
        if slim && height == 64 {
            for y in 16..32 {
                for x in [47, 54, 55] {
                    image.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
                }
            }
        }
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        png
    }

    #[test]
    fn guesses_the_arm_width() {
        assert_eq!(inspect(&skin(64, 64, false)).unwrap(), Variant::Classic);
        assert_eq!(inspect(&skin(64, 64, true)).unwrap(), Variant::Slim);
        assert_eq!(inspect(&skin(64, 32, false)).unwrap(), Variant::Classic);
    }

    #[test]
    fn refuses_what_is_not_a_skin() {
        assert!(inspect(&skin(128, 128, false)).is_err());
        assert!(inspect(b"not a png").is_err());
    }

    #[test]
    fn names_capes_from_their_alias() {
        assert_eq!(cape_name(Some("MIGRATOR")), "Migrator");
        assert_eq!(cape_name(Some("15th_anniversary")), "15th Anniversary");
        assert_eq!(cape_name(None), "Cape");
    }

    #[test]
    fn pack_skins_are_valid_and_their_model_is_right() {
        for skin in &PACK {
            assert_eq!(inspect(skin.png).unwrap(), skin.variant, "{}", skin.id);
        }
    }

    #[test]
    fn reads_default_skin_ids() {
        assert_eq!(parse_default("ari:slim").unwrap(), ("ari", Variant::Slim));
        assert_eq!(parse_default("steve").unwrap(), ("steve", Variant::Classic));
        assert!(parse_default("herobrine:classic").is_err());
    }
}
