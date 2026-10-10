use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Erreur de fichier : {0}")]
    Io(#[from] std::io::Error),
    #[error("Erreur de la base de données : {0}")]
    Database(#[from] sqlx::Error),
    #[error("Mise à jour de la base de données impossible : {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("Données illisibles : {0}")]
    Json(#[from] serde_json::Error),
    #[error("{}", http_message(.0))]
    Http(#[from] reqwest::Error),
    #[error("Pas de connexion à Internet pour l'instant")]
    Offline,
    #[error("Le serveur a répondu {status} pour {url}")]
    HttpStatus { url: String, status: u16 },
    #[error("Fichier abîmé pendant le téléchargement : {}", path.display())]
    ChecksumMismatch { path: PathBuf },
    #[error("Archive illisible : {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Image illisible : {0}")]
    Image(#[from] image::ImageError),
    #[error("Dossier des données introuvable")]
    NoDataDir,
    #[error("Journal impossible à créer : {0}")]
    Logging(String),
    #[error("Version de Minecraft inconnue : {0}")]
    VersionNotFound(String),
    #[error("Aucun Java `{component}` disponible pour {platform}")]
    JavaUnavailable { component: String, platform: String },
    #[error(
        "Cette version a besoin de Rosetta 2, qui n'est pas installé. \
         Installe-le avec : softwareupdate --install-rosetta --agree-to-license"
    )]
    RosettaMissing,
    #[error("Système non pris en charge : {0}")]
    UnsupportedPlatform(String),
    #[error("Instance introuvable : {0}")]
    InstanceNotFound(String),
    #[error("Compte introuvable : {0}")]
    AccountNotFound(String),
    #[error("Aucun compte actif : ajoute un compte pour jouer")]
    NoActiveAccount,
    #[error("La session Microsoft a expiré : reconnecte ce compte")]
    LoginRequired,
    #[error("Connecte un compte Microsoft qui possède Minecraft pour jouer, même hors ligne")]
    OwnershipRequired,
    #[error("{0}")]
    Auth(String),
    #[error("Jeu à deux : {0}")]
    Duo(String),
    #[error("{0} n'est pas encore pris en charge")]
    LoaderNotSupported(String),
    #[error("{loader} n'existe pas pour Minecraft {game_version}")]
    LoaderUnavailable {
        loader: String,
        game_version: String,
    },
    #[error("Une étape de l'installateur a échoué ({processor}) :\n{output}")]
    ProcessorFailed { processor: String, output: String },
    #[error(
        "Les mods ont besoin d'une instance avec un loader (Fabric, Quilt, Forge ou NeoForge)"
    )]
    ModLoaderRequired,
    #[error("Aucune version de {title} ne marche avec Minecraft {game_version}")]
    ContentUnavailable { title: String, game_version: String },
    #[error("Contenu introuvable : {0}")]
    ContentNotFound(String),
    #[error("{0}")]
    Translation(String),
    #[error("{0}")]
    InvalidInput(String),
}

/// What the player sees for a request that failed: no URL or library jargon (the full
/// error goes to the log where it is caught).
fn http_message(err: &reqwest::Error) -> String {
    if err.is_timeout() {
        "Le serveur met trop de temps à répondre, réessaie dans un instant".into()
    } else if err.is_connect() || err.is_request() {
        "Pas de connexion à Internet (ou le serveur ne répond pas)".into()
    } else if err.is_decode() || err.is_body() {
        "Réponse du serveur interrompue ou illisible, réessaie".into()
    } else {
        "Erreur réseau, réessaie dans un instant".into()
    }
}

impl Error {
    /// The network itself failed (no route, DNS, timeout), as opposed to a server answer.
    pub fn is_network(&self) -> bool {
        match self {
            Error::Offline => true,
            Error::Http(err) => err.is_connect() || err.is_timeout() || err.is_request(),
            _ => false,
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
