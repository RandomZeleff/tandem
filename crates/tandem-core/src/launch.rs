//! Builds the Java command line for a prepared version and spawns the game.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::{Child, Command};

use crate::account::Account;
use crate::error::Result;
use crate::install::PreparedVersion;
use crate::jvm;

pub struct LaunchSpec<'a> {
    pub prepared: &'a PreparedVersion,
    pub game_dir: &'a Path,
    pub assets_root: &'a Path,
    pub libraries_dir: &'a Path,
    pub account: &'a Account,
    pub access_token: &'a str,
    pub memory_mb: u32,
    pub extra_jvm_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GameCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
}

pub fn build_command(spec: &LaunchSpec<'_>) -> GameCommand {
    let env = &spec.prepared.env;
    let version = &spec.prepared.version;
    let separator = if cfg!(windows) { ";" } else { ":" };
    let classpath = spec
        .prepared
        .classpath
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(separator);
    let path = |p: &Path| p.display().to_string();

    let vars: HashMap<&str, String> = HashMap::from([
        ("auth_player_name", spec.account.username.clone()),
        ("auth_uuid", spec.account.mc_uuid.replace('-', "")),
        ("auth_access_token", spec.access_token.to_owned()),
        ("auth_session", spec.access_token.to_owned()),
        ("auth_xuid", String::new()),
        ("clientid", String::new()),
        ("user_type", spec.account.kind.user_type().to_owned()),
        ("user_properties", "{}".to_owned()),
        ("version_name", version.id.clone()),
        ("version_type", version.kind.clone()),
        ("game_directory", path(spec.game_dir)),
        ("assets_root", path(spec.assets_root)),
        ("game_assets", path(&spec.prepared.game_assets)),
        ("assets_index_name", version.assets.clone()),
        ("natives_directory", path(&spec.prepared.natives_dir)),
        ("library_directory", path(spec.libraries_dir)),
        ("classpath_separator", separator.to_owned()),
        ("classpath", classpath),
        ("launcher_name", "Tandem".to_owned()),
        ("launcher_version", crate::version().to_owned()),
    ]);

    let mut args = vec![
        format!("-Xmx{}M", spec.memory_mb),
        // Log4Shell mitigation for 1.7–1.18; harmless on newer versions.
        "-Dlog4j2.formatMsgNoLookups=true".to_owned(),
        // `System.out` otherwise uses the OS code page (cp1252 on a French Windows), which
        // the launcher console would show as `�`. `sun.*` up to Java 18, plain from 19.
        "-Dstdout.encoding=UTF-8".to_owned(),
        "-Dstderr.encoding=UTF-8".to_owned(),
        "-Dsun.stdout.encoding=UTF-8".to_owned(),
        "-Dsun.stderr.encoding=UTF-8".to_owned(),
    ];
    args.extend(jvm::gc_flags(&spec.extra_jvm_args));
    args.extend(spec.extra_jvm_args.iter().cloned());

    match &version.arguments {
        Some(arguments) if !arguments.jvm.is_empty() => {
            args.extend(
                arguments
                    .jvm
                    .iter()
                    .flat_map(|a| a.values(env))
                    .map(|v| substitute(v, &vars)),
            );
        }
        _ => {
            for raw in [
                "-Djava.library.path=${natives_directory}",
                "-cp",
                "${classpath}",
            ] {
                args.push(substitute(raw, &vars));
            }
        }
    }

    args.push(version.main_class.clone());

    match (&version.arguments, &version.minecraft_arguments) {
        (Some(arguments), _) if !arguments.game.is_empty() => args.extend(
            arguments
                .game
                .iter()
                .flat_map(|a| a.values(env))
                .map(|v| substitute(v, &vars)),
        ),
        (_, Some(legacy)) => args.extend(legacy.split_whitespace().map(|v| substitute(v, &vars))),
        _ => {}
    }

    GameCommand {
        program: spec.prepared.java.clone(),
        args,
        working_dir: spec.game_dir.to_owned(),
    }
}

/// Replaces every `${name}` with its value; unknown placeholders are left untouched.
fn substitute(raw: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                match vars.get(name) {
                    Some(value) => out.push_str(value),
                    None => out.push_str(&rest[start..start + 2 + end + 1]),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Spawns the game with piped output and no console window.
pub fn spawn(command: &GameCommand) -> Result<Child> {
    std::fs::create_dir_all(&command.working_dir)?;
    let mut cmd = Command::new(&command.program);
    cmd.args(&command.args)
        .current_dir(&command.working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    // Own process group, so `ProcessGroup::kill` reaches whatever the game starts.
    #[cfg(unix)]
    cmd.process_group(0);
    Ok(cmd.spawn()?)
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_known_and_keeps_unknown() {
        let vars = HashMap::from([("a", "1".to_owned()), ("natives_directory", "N".to_owned())]);
        assert_eq!(substitute("${a}", &vars), "1");
        assert_eq!(
            substitute("-Djava.library.path=${natives_directory}/java", &vars),
            "-Djava.library.path=N/java"
        );
        assert_eq!(substitute("${a}-${missing}-${a}", &vars), "1-${missing}-1");
        assert_eq!(substitute("x${unterminated", &vars), "x${unterminated");
    }
}
