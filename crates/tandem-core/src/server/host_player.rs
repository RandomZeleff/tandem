//! The host's own player between solo and the world server.
//!
//! In solo, the world's owner is stored in `level.dat` (`Data.Player`); a server reads
//! every player, the host included, from `playerdata/<uuid>.dat`. Before the server
//! starts, the solo player is copied there; once it stops, it goes back into `level.dat`,
//! so inventory, position and progress follow the host both ways.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use fastnbt::Value;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

use crate::error::{Error, Result};

fn invalid(message: impl Into<String>) -> Error {
    Error::WorldServer(message.into())
}

fn read_nbt(path: &Path) -> Result<Value> {
    let mut bytes = Vec::new();
    GzDecoder::new(std::fs::File::open(path)?).read_to_end(&mut bytes)?;
    fastnbt::from_bytes(&bytes).map_err(|e| invalid(format!("{}: {e}", path.display())))
}

/// Written next to the file then renamed: never a half-written `level.dat`.
fn write_nbt(path: &Path, value: &Value) -> Result<()> {
    let bytes = fastnbt::to_bytes(value).map_err(|e| invalid(e.to_string()))?;
    let tmp = path.with_extension("tandem-tmp");
    {
        let mut gz = GzEncoder::new(std::fs::File::create(&tmp)?, Compression::default());
        gz.write_all(&bytes)?;
        gz.finish()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn player_file(world: &Path, uuid: &str) -> PathBuf {
    world
        .join("playerdata")
        .join(format!("{}.dat", hyphenated(uuid)))
}

/// `playerdata` names files with the dashed form of the UUID.
pub(crate) fn hyphenated(uuid: &str) -> String {
    let raw: String = uuid
        .chars()
        .filter(|c| *c != '-')
        .collect::<String>()
        .to_lowercase();
    if raw.len() != 32 {
        return uuid.to_lowercase();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &raw[..8],
        &raw[8..12],
        &raw[12..16],
        &raw[16..20],
        &raw[20..]
    )
}

fn data_mut(level: &mut Value) -> Result<&mut std::collections::HashMap<String, Value>> {
    match level {
        Value::Compound(root) => match root.get_mut("Data") {
            Some(Value::Compound(data)) => Ok(data),
            _ => Err(invalid("level.dat sans « Data »")),
        },
        _ => Err(invalid("level.dat illisible")),
    }
}

/// Copies the solo player into `playerdata` before the server starts. A server session
/// that never handed the player back (Tandem closed abruptly) is recovered first.
pub fn give_to_server(world: &Path, uuid: &str) -> Result<()> {
    take_back_if_newer(world, uuid)?;
    let level_path = world.join("level.dat");
    let mut level = read_nbt(&level_path)?;
    let Some(player) = data_mut(&mut level)?.get("Player").cloned() else {
        // Never played in solo: the server creates the player.
        return Ok(());
    };
    let path = player_file(world, uuid);
    std::fs::create_dir_all(path.parent().unwrap_or(world))?;
    write_nbt(&path, &player)
}

/// Puts the player the server saved back into `level.dat`, for solo.
pub fn take_back(world: &Path, uuid: &str) -> Result<()> {
    let path = player_file(world, uuid);
    if !path.exists() {
        return Ok(());
    }
    let player = read_nbt(&path)?;
    let level_path = world.join("level.dat");
    let mut level = read_nbt(&level_path)?;
    data_mut(&mut level)?.insert("Player".to_owned(), player);
    write_nbt(&level_path, &level)
}

/// What the solo world says about how it is played, for the server's properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelSettings {
    /// `survival`, `creative`, `adventure` or `spectator`.
    pub game_mode: &'static str,
    /// `peaceful`, `easy`, `normal` or `hard`.
    pub difficulty: &'static str,
    pub hardcore: bool,
    /// Cheats allowed: the host gets operator rights, as the owner of a LAN world does.
    pub allow_commands: bool,
}

pub fn level_settings(world: &Path) -> Result<LevelSettings> {
    let mut level = read_nbt(&world.join("level.dat"))?;
    let data = data_mut(&mut level)?;
    let int = |key: &str| match data.get(key) {
        Some(Value::Int(v)) => i64::from(*v),
        Some(Value::Byte(v)) => i64::from(*v),
        _ => 0,
    };
    Ok(LevelSettings {
        game_mode: match int("GameType") {
            1 => "creative",
            2 => "adventure",
            3 => "spectator",
            _ => "survival",
        },
        difficulty: match int("Difficulty") {
            0 => "peaceful",
            1 => "easy",
            3 => "hard",
            _ => "normal",
        },
        hardcore: int("hardcore") != 0,
        allow_commands: int("allowCommands") != 0,
    })
}

fn take_back_if_newer(world: &Path, uuid: &str) -> Result<()> {
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    match (
        modified(&player_file(world, uuid)),
        modified(&world.join("level.dat")),
    ) {
        (Some(player), Some(level)) if player > level => take_back(world, uuid),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn compound(entries: &[(&str, Value)]) -> Value {
        Value::Compound(
            entries
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect::<HashMap<_, _>>(),
        )
    }

    fn player(level: &Path) -> Option<Value> {
        match read_nbt(level).unwrap() {
            Value::Compound(root) => match root.get("Data") {
                Some(Value::Compound(data)) => data.get("Player").cloned(),
                _ => None,
            },
            _ => None,
        }
    }

    #[test]
    fn player_goes_to_the_server_and_back() {
        let dir = tempfile::tempdir().unwrap();
        let world = dir.path();
        let solo = compound(&[
            ("XpLevel", Value::Int(7)),
            ("Pos", Value::List(vec![Value::Double(1.5)])),
        ]);
        let level = compound(&[(
            "Data",
            compound(&[
                ("LevelName", Value::String("Monde".into())),
                ("Player", solo.clone()),
            ]),
        )]);
        write_nbt(&world.join("level.dat"), &level).unwrap();
        let uuid = "0123456789abcdef0123456789ABCDEF";

        give_to_server(world, uuid).unwrap();
        let file = world.join("playerdata/01234567-89ab-cdef-0123-456789abcdef.dat");
        assert_eq!(read_nbt(&file).unwrap(), solo);

        // The server saves the player at level 12.
        let played = compound(&[
            ("XpLevel", Value::Int(12)),
            ("Pos", Value::List(vec![Value::Double(9.0)])),
        ]);
        write_nbt(&file, &played).unwrap();
        take_back(world, uuid).unwrap();
        assert_eq!(player(&world.join("level.dat")), Some(played));
        // The rest of level.dat is untouched.
        match read_nbt(&world.join("level.dat")).unwrap() {
            Value::Compound(root) => match &root["Data"] {
                Value::Compound(data) => {
                    assert_eq!(data["LevelName"], Value::String("Monde".into()))
                }
                _ => panic!("no Data"),
            },
            _ => panic!("no root"),
        }
    }

    #[test]
    fn reads_how_the_world_is_played() {
        let dir = tempfile::tempdir().unwrap();
        let level = compound(&[(
            "Data",
            compound(&[
                ("GameType", Value::Int(1)),
                ("Difficulty", Value::Byte(3)),
                ("allowCommands", Value::Byte(1)),
            ]),
        )]);
        write_nbt(&dir.path().join("level.dat"), &level).unwrap();
        assert_eq!(
            level_settings(dir.path()).unwrap(),
            LevelSettings {
                game_mode: "creative",
                difficulty: "hard",
                hardcore: false,
                allow_commands: true
            }
        );
    }

    #[test]
    fn a_world_never_played_in_solo_has_no_player_to_give() {
        let dir = tempfile::tempdir().unwrap();
        let level = compound(&[(
            "Data",
            compound(&[("LevelName", Value::String("Neuf".into()))]),
        )]);
        write_nbt(&dir.path().join("level.dat"), &level).unwrap();
        give_to_server(dir.path(), "0123456789abcdef0123456789abcdef").unwrap();
        assert!(!dir.path().join("playerdata").exists());
        take_back(dir.path(), "0123456789abcdef0123456789abcdef").unwrap();
        assert_eq!(player(&dir.path().join("level.dat")), None);
    }
}
