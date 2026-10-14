//! A world's logbook: what the game already records about a playthrough (statistics,
//! advancements, `level.dat`) and FTB Quests progress, read without touching anything.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use crate::error::{Error, Result};
use crate::translate::snbt::{self, Value};

/// Vanilla advancements that mark a playthrough's big steps, in the usual order.
pub const MILESTONES: &[&str] = &[
    "story/smelt_iron",
    "story/mine_diamond",
    "story/enter_the_nether",
    "nether/find_fortress",
    "story/follow_ender_eye",
    "story/enter_the_end",
    "end/kill_dragon",
    "nether/summon_wither",
    "end/elytra",
];

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Journal {
    /// In-game day, as the debug screen counts it (from 1).
    pub day: Option<u64>,
    /// `None` without a statistics file (world never opened by a player yet).
    pub stats: Option<Stats>,
    /// Empty before 1.12 (no advancements).
    pub milestones: Vec<Milestone>,
    /// Only for packs with FTB Quests.
    pub quests: Option<Quests>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub play_seconds: u64,
    pub deaths: u64,
    pub mob_kills: u64,
    pub distance_m: u64,
    pub blocks_mined: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Milestone {
    /// Advancement id without the `minecraft:` namespace.
    pub id: &'static str,
    /// Unix milliseconds; `None` while not reached.
    pub done_at: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quests {
    pub done: u32,
    pub total: u32,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub title: String,
    pub done: u32,
    pub total: u32,
}

/// The logbook of `saves/<world>` in `game_dir`.
pub fn read(game_dir: &Path, world: &str) -> Result<Journal> {
    let mut parts = Path::new(world).components();
    let single = matches!(
        (parts.next(), parts.next()),
        (Some(std::path::Component::Normal(_)), None)
    );
    let dir = game_dir.join("saves").join(world);
    if !single || !dir.join("level.dat").is_file() {
        return Err(Error::InvalidInput(format!("Monde introuvable : {world}")));
    }
    let player = main_player(&dir);
    Ok(Journal {
        day: read_day(&dir.join("level.dat")),
        stats: player.as_ref().and_then(|(_, stats)| read_stats(stats)),
        milestones: milestones(&dir, player.as_ref().map(|(uuid, _)| uuid.as_str())),
        quests: quests(game_dir, &dir),
    })
}

/// The player who played this world the most (a world can hold several accounts), with
/// their statistics file.
fn main_player(dir: &Path) -> Option<(String, Json)> {
    json_files(&dir.join("stats"))
        .into_iter()
        .filter_map(|(uuid, json)| Some((uuid, json?)))
        .max_by_key(|(_, json)| play_ticks(json))
}

fn json_files(dir: &Path) -> Vec<(String, Option<Json>)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .filter_map(|p| {
            let uuid = p.file_stem()?.to_string_lossy().into_owned();
            let json = std::fs::read(&p)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok());
            Some((uuid, json))
        })
        .collect()
}

fn number(value: Option<&Json>) -> u64 {
    value.and_then(Json::as_u64).unwrap_or(0)
}

/// `minecraft:custom` of 1.13+, or the flat `stat.*` map of older versions.
fn custom(json: &Json) -> Option<&serde_json::Map<String, Json>> {
    match json.get("stats") {
        Some(stats) => stats.get("minecraft:custom")?.as_object(),
        None => json.as_object(),
    }
}

fn play_ticks(json: &Json) -> u64 {
    let Some(custom) = custom(json) else {
        return 0;
    };
    // `play_one_minute` counted ticks too, until 1.17 renamed it.
    [
        "minecraft:play_time",
        "minecraft:play_one_minute",
        "stat.playOneMinute",
    ]
    .iter()
    .map(|k| number(custom.get(*k)))
    .max()
    .unwrap_or(0)
}

fn read_stats(json: &Json) -> Option<Stats> {
    let custom = custom(json)?;
    let get = |keys: &[&str]| {
        keys.iter()
            .map(|k| number(custom.get(*k)))
            .max()
            .unwrap_or(0)
    };
    // Every way of moving except falling.
    let distance_cm: u64 = custom
        .iter()
        .filter(|(k, _)| {
            (k.ends_with("_one_cm") || k.ends_with("OneCm"))
                && !k.contains("fall")
                && !k.contains("Fall")
        })
        .map(|(_, v)| number(Some(v)))
        .sum();
    let blocks_mined = match json.get("stats") {
        Some(stats) => stats
            .get("minecraft:mined")
            .and_then(Json::as_object)
            .map(|m| m.values().map(|v| number(Some(v))).sum())
            .unwrap_or(0),
        None => custom
            .iter()
            .filter(|(k, _)| k.starts_with("stat.mineBlock."))
            .map(|(_, v)| number(Some(v)))
            .sum(),
    };
    Some(Stats {
        play_seconds: play_ticks(json) / 20,
        deaths: get(&["minecraft:deaths", "stat.deaths"]),
        mob_kills: get(&["minecraft:mob_kills", "stat.mobKills"]),
        distance_m: distance_cm / 100,
        blocks_mined,
    })
}

#[derive(Deserialize)]
struct LevelDat {
    #[serde(rename = "Data")]
    data: LevelTime,
}

#[derive(Deserialize)]
struct LevelTime {
    #[serde(rename = "DayTime")]
    day_time: Option<i64>,
}

fn read_day(path: &Path) -> Option<u64> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(file)
        .read_to_end(&mut bytes)
        .ok()?;
    let level: LevelDat = fastnbt::from_bytes(&bytes).ok()?;
    let ticks = u64::try_from(level.data.day_time?).ok()?;
    Some(ticks / 24_000 + 1)
}

fn milestones(dir: &Path, player: Option<&str>) -> Vec<Milestone> {
    let files = json_files(&dir.join("advancements"));
    // The same player as the statistics, else whoever has the most advancements.
    let file = files
        .iter()
        .find(|(uuid, _)| Some(uuid.as_str()) == player)
        .or_else(|| {
            files
                .iter()
                .max_by_key(|(_, j)| j.as_ref().and_then(Json::as_object).map_or(0, |o| o.len()))
        })
        .and_then(|(_, json)| json.as_ref());
    let Some(advancements) = file else {
        return Vec::new();
    };
    MILESTONES
        .iter()
        .map(|id| Milestone {
            id,
            done_at: advancements
                .get(format!("minecraft:{id}"))
                .filter(|a| a.get("done").and_then(Json::as_bool) == Some(true))
                .map(|a| {
                    // When the last criterion was met; a reached step without dates still counts.
                    a.get("criteria")
                        .and_then(Json::as_object)
                        .into_iter()
                        .flat_map(|c| c.values())
                        .filter_map(|v| v.as_str().and_then(parse_timestamp))
                        .max()
                        .unwrap_or(0)
                }),
        })
        .collect()
}

/// `2026-02-22 21:10:52 +0100` (advancement criteria) as Unix milliseconds.
fn parse_timestamp(text: &str) -> Option<i64> {
    let mut parts = text.split(' ');
    let date: Vec<i64> = parts
        .next()?
        .split('-')
        .map(|n| n.parse().ok())
        .collect::<Option<_>>()?;
    let time: Vec<i64> = parts
        .next()?
        .split(':')
        .map(|n| n.parse().ok())
        .collect::<Option<_>>()?;
    let zone = parts.next()?;
    let [y, m, d] = <[i64; 3]>::try_from(date).ok()?;
    let [hh, mm, ss] = <[i64; 3]>::try_from(time).ok()?;
    let sign = if zone.starts_with('-') { -1 } else { 1 };
    let digits = zone.trim_start_matches(['+', '-']);
    let offset = sign
        * (digits.get(..2)?.parse::<i64>().ok()? * 60 + digits.get(2..4)?.parse::<i64>().ok()?);
    let seconds = days_from_civil(y, m, d) * 86_400 + hh * 3600 + mm * 60 + ss - offset * 60;
    Some(seconds * 1000)
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn quest_root(game_dir: &Path) -> PathBuf {
    game_dir.join("config").join("ftbquests").join("quests")
}

/// Quests done in this world against the pack's quest book (FTB Quests 1.16+ layout).
fn quests(game_dir: &Path, world_dir: &Path) -> Option<Quests> {
    let root = quest_root(game_dir);
    let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("chapters"))
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "snbt"))
        .collect();
    files.sort();
    let done = completed(world_dir);
    let lang = quest_lang(&root);

    let mut chapters: Vec<(i64, Chapter)> = files
        .iter()
        .filter_map(|path| {
            let tree = snbt::parse(&std::fs::read_to_string(path).ok()?)?;
            // Hidden chapters and quests hold the pack's bookkeeping, not goals.
            if is_true(&tree, "always_invisible") {
                return None;
            }
            let ids: Vec<String> = tree
                .get("quests")?
                .as_list()
                .iter()
                .filter(|q| !is_true(q, "invisible"))
                .filter_map(|q| Some(q.get("id")?.as_str()?.to_ascii_uppercase()))
                .collect();
            if ids.is_empty() {
                return None;
            }
            let id = tree
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_uppercase();
            let title = tree
                .get("title")
                .and_then(Value::as_str)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .or_else(|| lang.get(&format!("chapter.{id}.title")).cloned())
                .unwrap_or_else(|| file_title(path));
            let order = tree
                .get("order_index")
                .and_then(Value::as_str)
                .and_then(|n| n.parse().ok())
                .unwrap_or(i64::MAX);
            Some((
                order,
                Chapter {
                    title: strip_formatting(&title),
                    done: ids.iter().filter(|q| done.contains(*q)).count() as u32,
                    total: ids.len() as u32,
                },
            ))
        })
        .collect();
    if chapters.is_empty() {
        return None;
    }
    chapters.sort_by_key(|(order, _)| *order);
    let chapters: Vec<Chapter> = chapters.into_iter().map(|(_, c)| c).collect();
    Some(Quests {
        done: chapters.iter().map(|c| c.done).sum(),
        total: chapters.iter().map(|c| c.total).sum(),
        chapters,
    })
}

fn is_true(tree: &Value, key: &str) -> bool {
    tree.get(key).and_then(Value::as_str) == Some("true")
}

/// Ids (quests, tasks, chapters) completed by the team that went furthest in this world.
fn completed(world_dir: &Path) -> HashSet<String> {
    let Ok(entries) = std::fs::read_dir(world_dir.join("ftbquests")) else {
        return HashSet::new();
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "snbt"))
        .filter_map(|p| snbt::parse(&std::fs::read_to_string(p).ok()?))
        .map(|tree| {
            tree.get("completed")
                .map(|c| c.keys().map(str::to_ascii_uppercase).collect())
                .unwrap_or_default()
        })
        .max_by_key(HashSet::len)
        .unwrap_or_default()
}

/// Chapter titles kept in language files (recent FTB Quests): French first, else English.
fn quest_lang(root: &Path) -> HashMap<String, String> {
    let mut entries = HashMap::new();
    for locale in ["en_us", "fr_fr"] {
        let lang = root.join("lang");
        let mut texts: Vec<String> = std::fs::read_to_string(lang.join(format!("{locale}.snbt")))
            .into_iter()
            .collect();
        if let Ok(dir) = std::fs::read_dir(lang.join(locale)) {
            texts.extend(
                dir.filter_map(|e| e.ok())
                    .filter_map(|e| std::fs::read_to_string(e.path()).ok()),
            );
        }
        for text in texts {
            entries.extend(crate::translate::sources::quest_lang_entries(&text));
        }
    }
    entries
}

/// `bosses_and_minibosses.snbt` → `Bosses and minibosses`.
fn file_title(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().replace('_', " "))
        .unwrap_or_default();
    let mut chars = stem.trim().chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => stem,
    }
}

/// Without Minecraft colour codes (`§6`, `&l`).
fn strip_formatting(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if (c == '§' || c == '&') && chars.peek().is_some_and(|n| n.is_ascii_alphanumeric()) {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_criteria_dates() {
        // 2026-02-22 20:10:52 UTC.
        assert_eq!(
            parse_timestamp("2026-02-22 21:10:52 +0100"),
            Some(1_771_791_052_000)
        );
        assert_eq!(parse_timestamp("1970-01-01 00:00:00 +0000"), Some(0));
        assert_eq!(parse_timestamp("not a date"), None);
    }

    #[test]
    fn reads_modern_and_old_stats() {
        let modern: Json = serde_json::from_str(
            r#"{"stats":{"minecraft:custom":{"minecraft:play_time":72000,"minecraft:deaths":3,
            "minecraft:walk_one_cm":150000,"minecraft:fall_one_cm":99999,"minecraft:aviate_one_cm":50000},
            "minecraft:mined":{"minecraft:stone":40,"minecraft:dirt":2}},"DataVersion":3700}"#,
        )
        .unwrap();
        let stats = read_stats(&modern).unwrap();
        assert_eq!(stats.play_seconds, 3600);
        assert_eq!(stats.deaths, 3);
        assert_eq!(stats.distance_m, 2000);
        assert_eq!(stats.blocks_mined, 42);
        let old: Json = serde_json::from_str(
            r#"{"stat.playOneMinute":2400,"stat.mobKills":7,"stat.walkOneCm":1000,"stat.mineBlock.minecraft.stone":5}"#,
        )
        .unwrap();
        let stats = read_stats(&old).unwrap();
        assert_eq!(
            (
                stats.play_seconds,
                stats.mob_kills,
                stats.distance_m,
                stats.blocks_mined
            ),
            (120, 7, 10, 5)
        );
    }

    #[test]
    fn counts_quests_per_chapter() {
        let game = tempfile::tempdir().unwrap();
        let chapters = quest_root(game.path()).join("chapters");
        std::fs::create_dir_all(&chapters).unwrap();
        std::fs::write(
            chapters.join("getting_started.snbt"),
            "{\n\tid: \"0A\"\n\torder_index: 1\n\tquests: [\n\t\t{ id: \"q1\" tasks: [{ id: \"t1\" }] }\n\t\t{ title: \"§6Two\", id: \"Q2\" }\n\t\t{ id: \"Q5\", invisible: true }\n\t]\n}",
        )
        .unwrap();
        std::fs::write(
            chapters.join("magic.snbt"),
            "{ id: \"0B\", order_index: 0, title: \"&lMagie\", quests: [{ id: \"Q3\" }] }",
        )
        .unwrap();
        std::fs::write(chapters.join("empty.snbt"), "{ id: \"0C\", quests: [] }").unwrap();
        std::fs::write(
            chapters.join("hidden.snbt"),
            "{ id: \"0D\", always_invisible: true, quests: [{ id: \"Q4\" }] }",
        )
        .unwrap();
        let lang = quest_root(game.path()).join("lang");
        std::fs::create_dir_all(&lang).unwrap();
        std::fs::write(
            lang.join("fr_fr.snbt"),
            "{ \"chapter.0A.title\": \"Premiers pas\" }",
        )
        .unwrap();
        let world = game.path().join("saves").join("w");
        std::fs::create_dir_all(world.join("ftbquests")).unwrap();
        std::fs::write(
            world.join("ftbquests").join("team.snbt"),
            "{\n\tversion: 1\n\tcompleted: {\n\t\tQ1: 1771791052166L\n\t\tT1: 1771791052166L\n\t}\n}",
        )
        .unwrap();

        let quests = quests(game.path(), &world).unwrap();
        assert_eq!((quests.done, quests.total), (1, 3));
        let titles: Vec<_> = quests
            .chapters
            .iter()
            .map(|c| (c.title.as_str(), c.done, c.total))
            .collect();
        assert_eq!(titles, [("Magie", 0, 1), ("Premiers pas", 1, 2)]);
    }

    #[test]
    fn snbt_tree() {
        let tree =
            snbt::parse("{ a: [I; 1, 2], 'b c': { d: \"e\" } f: [\"x\"\n\"y\"] g: 3.5d }").unwrap();
        assert_eq!(tree.get("a").unwrap().as_list().len(), 2);
        assert_eq!(
            tree.get("b c").unwrap().get("d").unwrap().as_str(),
            Some("e")
        );
        assert_eq!(tree.get("f").unwrap().as_list().len(), 2);
        assert_eq!(tree.get("g").unwrap().as_str(), Some("3.5d"));
    }
}
