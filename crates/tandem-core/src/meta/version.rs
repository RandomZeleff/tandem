//! Version JSON (`versions/<id>/<id>.json`) as published by piston-meta.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use super::loader::LoaderProfile;
use super::rules::{is_allowed, Environment, Rule};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub main_class: String,
    /// 1.13+ argument format.
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// Pre-1.13 space-separated game arguments.
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    pub asset_index: AssetIndexRef,
    pub assets: String,
    pub downloads: VersionDownloads,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub java_version: Option<JavaVersion>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional { rules: Vec<Rule>, value: ArgValue },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    One(String),
    Many(Vec<String>),
}

impl Argument {
    /// The raw (unsubstituted) values this argument contributes in `env`.
    pub fn values(&self, env: &Environment) -> Vec<&str> {
        match self {
            Argument::Plain(value) => vec![value.as_str()],
            Argument::Conditional { rules, value } if is_allowed(rules, env) => match value {
                ArgValue::One(v) => vec![v.as_str()],
                ArgValue::Many(vs) => vs.iter().map(String::as_str).collect(),
            },
            Argument::Conditional { .. } => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionDownloads {
    pub client: Artifact,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    /// Maven coordinates `group:artifact:version[:classifier]`.
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// Legacy natives: OS name → classifier (may contain `${arch}`).
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub extract: Option<Extract>,
    /// Maven repository base (loader profiles list libraries this way instead of `downloads`).
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

/// A library file to download, with its path relative to the libraries directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryFile {
    pub path: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

impl LibraryFile {
    fn from_artifact(path: String, artifact: &Artifact) -> Self {
        Self {
            path,
            url: artifact.url.clone(),
            sha1: Some(artifact.sha1.clone()),
            size: Some(artifact.size),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: HashMap<String, Artifact>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Library {
    pub fn is_allowed(&self, env: &Environment) -> bool {
        is_allowed(&self.rules, env)
    }

    /// Main jar: from `downloads` (Mojang) or from a Maven repository (loaders).
    pub fn artifact(&self) -> Option<LibraryFile> {
        if let Some(artifact) = self.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            let path = artifact.path.clone().or_else(|| maven_path(&self.name))?;
            return Some(LibraryFile::from_artifact(path, artifact));
        }
        let base = self.url.as_deref()?;
        let path = maven_path(&self.name)?;
        Some(LibraryFile {
            url: format!("{}/{path}", base.trim_end_matches('/')),
            path,
            sha1: self.sha1.clone(),
            size: self.size,
        })
    }

    /// `group:artifact[:classifier]`: identifies a library regardless of its version.
    fn key(&self) -> String {
        let coords = self.name.split('@').next().unwrap_or_default();
        let mut parts = coords.split(':');
        let group = parts.next().unwrap_or_default();
        let artifact = parts.next().unwrap_or_default();
        match parts.nth(1) {
            Some(classifier) => format!("{group}:{artifact}:{classifier}"),
            None => format!("{group}:{artifact}"),
        }
    }

    /// Legacy natives jar to extract for this platform, if any.
    pub fn native_artifact(&self, env: &Environment) -> Option<LibraryFile> {
        let classifier = self
            .natives
            .as_ref()?
            .get(env.os_name)?
            .replace("${arch}", if env.is_64bit() { "64" } else { "32" });
        let artifact = self.downloads.as_ref()?.classifiers.get(&classifier)?;
        let path = artifact
            .path
            .clone()
            .or_else(|| maven_path(&format!("{}:{classifier}", self.name)))?;
        Some(LibraryFile::from_artifact(path, artifact))
    }
}

impl VersionJson {
    /// Layers a loader profile on top of this vanilla version. Loader libraries come
    /// first on the classpath and replace vanilla ones with the same group and artifact
    /// (e.g. a newer ASM).
    pub fn apply_loader(&mut self, profile: LoaderProfile) {
        self.main_class = profile.main_class;
        if let Some(extra) = profile.arguments {
            let arguments = self.arguments.get_or_insert_with(Arguments::default);
            arguments.jvm.extend(extra.jvm);
            arguments.game.extend(extra.game);
        }
        let overridden: HashSet<String> = profile.libraries.iter().map(Library::key).collect();
        let vanilla = std::mem::take(&mut self.libraries);
        self.libraries = profile.libraries;
        self.libraries.extend(
            vanilla
                .into_iter()
                .filter(|l| !overridden.contains(&l.key())),
        );
    }
}

/// `group:artifact:version[:classifier][@ext]` → `group/path/artifact/version/artifact-version[-classifier].ext`.
pub fn maven_path(coords: &str) -> Option<String> {
    let (coords, ext) = coords.split_once('@').unwrap_or((coords, "jar"));
    let mut parts = coords.split(':');
    let group = parts.next()?;
    let artifact = parts.next()?;
    let version = parts.next()?;
    let classifier = parts.next();
    let file = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.{ext}"),
        None => format!("{artifact}-{version}.{ext}"),
    };
    Some(format!(
        "{}/{artifact}/{version}/{file}",
        group.replace('.', "/")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_paths() {
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3").as_deref(),
            Some("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar")
        );
        assert_eq!(
            maven_path("com.mojang:jtracy:1.14.38:natives-linux").as_deref(),
            Some("com/mojang/jtracy/1.14.38/jtracy-1.14.38-natives-linux.jar")
        );
        assert_eq!(
            maven_path("a.b:c:1@zip").as_deref(),
            Some("a/b/c/1/c-1.zip")
        );
        assert_eq!(maven_path("broken"), None);
    }

    #[test]
    fn parses_modern_and_legacy_arguments() {
        let json = r#"{
            "id": "x", "type": "release", "mainClass": "net.minecraft.client.main.Main",
            "assets": "1", "assetIndex": {"id":"1","sha1":"a","size":1,"url":"u"},
            "downloads": {"client": {"sha1":"b","size":2,"url":"v"}},
            "arguments": {
                "game": ["--username", "${auth_player_name}",
                         {"rules":[{"action":"allow","features":{"is_demo_user":true}}],"value":"--demo"}],
                "jvm": [{"rules":[{"action":"allow","os":{"name":"windows"}}],"value":["-a","-b"]}, "-cp", "${classpath}"]
            },
            "libraries": [{
                "name": "org.lwjgl.lwjgl:lwjgl-platform:2.9.1",
                "natives": {"windows": "natives-windows-${arch}"},
                "downloads": {"classifiers": {"natives-windows-64": {"path":"p.jar","sha1":"c","size":3,"url":"w"}}}
            }]
        }"#;
        let v: VersionJson = serde_json::from_str(json).unwrap();
        let env = Environment {
            os_name: "windows",
            arch: "x86_64",
            features: HashMap::new(),
        };
        let args = v.arguments.unwrap();
        let game: Vec<_> = args.game.iter().flat_map(|a| a.values(&env)).collect();
        assert_eq!(game, ["--username", "${auth_player_name}"]);
        let jvm: Vec<_> = args.jvm.iter().flat_map(|a| a.values(&env)).collect();
        assert_eq!(jvm, ["-a", "-b", "-cp", "${classpath}"]);

        let lib = &v.libraries[0];
        assert!(lib.artifact().is_none());
        assert_eq!(lib.native_artifact(&env).unwrap().path, "p.jar");
    }

    #[test]
    fn applies_loader_profile() {
        let vanilla = r#"{
            "id": "1.21.4", "type": "release", "mainClass": "net.minecraft.client.main.Main",
            "assets": "1", "assetIndex": {"id":"1","sha1":"a","size":1,"url":"u"},
            "downloads": {"client": {"sha1":"b","size":2,"url":"v"}},
            "arguments": {"game": ["--demo"], "jvm": ["-cp", "${classpath}"]},
            "libraries": [
                {"name": "org.ow2.asm:asm:9.6", "downloads": {"artifact": {"path":"old.jar","sha1":"c","size":3,"url":"w"}}},
                {"name": "org.lwjgl:lwjgl:3.3.3:natives-windows", "downloads": {"artifact": {"path":"n.jar","sha1":"d","size":4,"url":"x"}}}
            ]
        }"#;
        let profile = r#"{
            "id": "fabric-loader-0.16.10-1.21.4", "inheritsFrom": "1.21.4",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "arguments": {"game": [], "jvm": ["-DFabricMcEmu= net.minecraft.client.main.Main "]},
            "libraries": [
                {"name": "org.ow2.asm:asm:9.7.1", "url": "https://maven.fabricmc.net/", "sha1": "e", "size": 5},
                {"name": "net.fabricmc:intermediary:1.21.4", "url": "https://maven.fabricmc.net"}
            ]
        }"#;
        let mut v: VersionJson = serde_json::from_str(vanilla).unwrap();
        v.apply_loader(serde_json::from_str(profile).unwrap());

        assert_eq!(
            v.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        let names: Vec<_> = v.libraries.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "org.ow2.asm:asm:9.7.1",
                "net.fabricmc:intermediary:1.21.4",
                "org.lwjgl:lwjgl:3.3.3:natives-windows"
            ]
        );
        assert_eq!(
            v.libraries[0].artifact().unwrap(),
            LibraryFile {
                path: "org/ow2/asm/asm/9.7.1/asm-9.7.1.jar".into(),
                url: "https://maven.fabricmc.net/org/ow2/asm/asm/9.7.1/asm-9.7.1.jar".into(),
                sha1: Some("e".into()),
                size: Some(5),
            }
        );
        assert_eq!(
            v.libraries[1].artifact().unwrap().url,
            "https://maven.fabricmc.net/net/fabricmc/intermediary/1.21.4/intermediary-1.21.4.jar"
        );
        let args = v.arguments.unwrap();
        assert_eq!(args.jvm.len(), 3);
        assert_eq!(args.game.len(), 1);
    }
}
