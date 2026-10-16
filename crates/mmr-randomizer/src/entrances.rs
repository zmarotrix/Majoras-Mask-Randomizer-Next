//! Entrance randomization (the old `ShuffleEntrances`): dungeon entrances,
//! boss rooms, grottos and interiors lead somewhere else.
//!
//! An entrance placed at another means: going in there takes you to it. So
//! being through it needs what reaching the other one needed, and the logic
//! is rewritten that way (each moved entrance gets its target's original
//! requirements). That works the same for logic with or without regions.

use crate::rng::Rng;
use mmr_data::GameData;
use mmr_logic::{Entry, LogicFile};
use mmr_settings::Values;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
pub struct EntranceItem {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub additional_types: Vec<String>,
    pub entrance: String,
    pub pair: Option<String>,
    pub has_spawn: bool,
    pub has_exit_actor: bool,
}

/// Every entrance that can be shuffled, in the game's order.
pub fn entrance_items() -> &'static [EntranceItem] {
    static TABLE: OnceLock<Vec<EntranceItem>> = OnceLock::new();
    TABLE.get_or_init(|| serde_json::from_str(include_str!("../../../data/entrance_items.json")).expect("bundled entrance table"))
}

fn of_type(kind: &str) -> Vec<&'static EntranceItem> {
    entrance_items().iter().filter(|e| e.kind == kind).collect()
}

fn flags<'v>(values: &'v Values, key: &str) -> Vec<&'v str> {
    values.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default()
}

/// The entry that stands for being through an entrance: the old place
/// entry, or the region it became.
pub fn entry_id(logic: &LogicFile, id: &str) -> String {
    if logic.entry(id).is_some() {
        return id.to_string();
    }
    mmr_logic::region::region_entry_id(&mmr_logic::regionize::region_name(id))
}

const BOSS_KILLS: [&str; 4] = ["OtherKillOdolwa", "OtherKillGoht", "OtherKillGyorg", "OtherKillTwinmold"];
const REMAINS: [&str; 4] = ["RemainsOdolwa", "RemainsGoht", "RemainsGyorg", "RemainsTwinmold"];

/// Shuffle the entrances the settings ask for and rewrite `logic` to
/// match. Returns (entrance, where it now is) for every moved entrance.
pub fn shuffle(logic: &mut LogicFile, values: &Values, game: &GameData, rng: &mut Rng) -> Result<Vec<(String, String)>, String> {
    let mode = flags(values, "EntranceMode");
    if mode.is_empty() || values.get("LogicMode").and_then(Value::as_str) == Some("Vanilla") {
        return Ok(Vec::new());
    }
    let no_logic = values.get("LogicMode").and_then(Value::as_str) == Some("NoLogic");
    let original = logic.clone();
    let time = |id: &str| original.entry(&entry_id(&original, id)).map(|e| e.time).unwrap_or_default();
    let mut placed: BTreeMap<String, String> = BTreeMap::new();
    // Logic entries that move with the entrances (boss kills).
    let mut moved_events: Vec<(String, String)> = Vec::new();
    let mut remains_swap: Vec<(String, String)> = Vec::new();

    let perm = |rng: &mut Rng| {
        let mut p = [0usize, 1, 2, 3];
        rng.shuffle(&mut p);
        p
    };
    if mode.contains(&"BossRooms") {
        let bosses = of_type("Boss");
        let exits = of_type("DungeonExit");
        let p = perm(rng);
        for (from, &to) in p.iter().enumerate() {
            placed.insert(bosses[from].id.clone(), bosses[to].id.clone());
            placed.insert(exits[to].id.clone(), exits[from].id.clone());
            moved_events.push((BOSS_KILLS[to].into(), BOSS_KILLS[from].into()));
            remains_swap.push((REMAINS[to].into(), REMAINS[from].into()));
        }
    }
    if mode.contains(&"DungeonEntrances") {
        let dungeons = of_type("Dungeon");
        let exits = of_type("DungeonExit");
        let p = perm(rng);
        let mut changes = Vec::new();
        for (from, &to) in p.iter().enumerate() {
            placed.insert(dungeons[from].id.clone(), dungeons[to].id.clone());
            let target_exit = &exits[from].id;
            changes.push((exits[to].id.clone(), placed.get(target_exit).cloned().unwrap_or_else(|| target_exit.clone())));
        }
        placed.extend(changes);
    }
    for (flag, kind) in [("Grottos", "Grotto"), ("SimpleInteriors", "Interior")] {
        if !mode.contains(&flag) {
            continue;
        }
        let entrances = of_type(kind);
        let mut targets: Vec<&str> = entrances.iter().map(|e| e.id.as_str()).filter(|t| !placed.values().any(|v| v == t)).collect();
        let fits = |e: &EntranceItem, t: &str| {
            let target = entrance_items().iter().find(|x| x.id == t).unwrap();
            if !e.has_spawn && !target.has_exit_actor {
                return false;
            }
            if no_logic {
                return true;
            }
            let needed = time(&e.id).needed;
            let available = time(t).available;
            needed.is_empty() || available.is_empty() || (needed.0 & available.0) != 0
        };
        let place = |e: &EntranceItem, targets: &mut Vec<&str>, placed: &mut BTreeMap<String, String>, rng: &mut Rng, allowed: &dyn Fn(&str) -> bool| -> Result<(), String> {
            if placed.contains_key(&e.id) {
                return Ok(());
            }
            let mut remaining: Vec<&str> = targets.iter().copied().filter(|t| allowed(t)).collect();
            loop {
                if remaining.is_empty() {
                    return Err(format!("couldn't place the entrance {} anywhere", e.id));
                }
                let t = remaining.remove(rng.below(remaining.len()));
                if fits(e, t) {
                    placed.insert(e.id.clone(), t.to_string());
                    targets.retain(|x| *x != t);
                    return Ok(());
                }
            }
        };
        // Random placement can paint itself into a corner (the last
        // entrance fits nowhere left): then try again, hardest ones first.
        let (start_targets, start_placed) = (targets.clone(), placed.clone());
        let mut attempt = 0;
        loop {
            let mut order = entrances.clone();
            if attempt > 0 {
                targets = start_targets.clone();
                placed = start_placed.clone();
                rng.shuffle(&mut order);
                order.sort_by_key(|e| targets.iter().filter(|t| fits(e, t)).count());
            }
            let mut group = || -> Result<(), String> {
                // Fairy fountains stay one per area when fairies keep within areas.
                if kind == "Interior" && flags(values, "StrayFairyMode").contains(&"KeepWithinArea") {
                    let area = |id: &str| {
                        game.item(id).and_then(|i| i.region.as_deref()).and_then(|r| game.region(r)).and_then(|r| r.area.clone())
                    };
                    let fountains: Vec<&EntranceItem> = entrances.iter().copied().filter(|e| e.additional_types.iter().any(|t| t == "FairyFountain")).collect();
                    let mut areas: Vec<Option<String>> = fountains.iter().map(|f| area(&f.id)).collect();
                    for f in fountains {
                        let allowed = |t: &str| areas.contains(&area(t));
                        place(f, &mut targets, &mut placed, rng, &allowed)?;
                        let got = area(&placed[&f.id]);
                        if let Some(i) = areas.iter().position(|a| *a == got) {
                            areas.remove(i);
                        }
                    }
                }
                for &e in &order {
                    place(e, &mut targets, &mut placed, rng, &|_| true)?;
                }
                Ok(())
            };
            match group() {
                Ok(()) => break,
                Err(e) if attempt >= 50 => return Err(e),
                Err(_) => attempt += 1,
            }
        }
    }

    // Rewrite the logic from the original: each moved entrance (and boss
    // kill) needs what its target needed.
    let copy = |into: &mut Entry, from: &Entry| {
        into.requires = from.requires.clone();
        into.any_of = from.any_of.clone();
        into.time = from.time;
        into.temporary_requires = from.temporary_requires.clone();
        into.before = from.before.clone();
    };
    let moves: Vec<(String, String)> = placed.iter().map(|(e, t)| (e.clone(), t.clone())).chain(moved_events).collect();
    for (e, t) in &moves {
        if e == t {
            continue;
        }
        let (from_id, into_id) = (entry_id(&original, t), entry_id(&original, e));
        let Some(from) = original.entry(&from_id).cloned() else { continue };
        let Some(into) = logic.entries.iter_mut().find(|x| x.id == into_id) else { continue };
        if from_id.starts_with("Region: ") && into_id.starts_with("Region: ") {
            // A region's entry also lists the ways back in from its own inner
            // regions: those stay; only the ways in from outside move.
            let inside = |id: &str, region: &str| id == region || id.starts_with(&format!("{region}: "));
            let own: Vec<Vec<String>> = original.entry(&into_id).map(|x| x.any_of.clone()).unwrap_or_default().into_iter().filter(|g| g.iter().any(|r| inside(r, &into_id))).collect();
            let mut outside: Vec<Vec<String>> = if from.any_of.is_empty() { vec![from.requires.clone()] } else { from.any_of.iter().filter(|g| !g.iter().any(|r| inside(r, &from_id))).map(|g| from.requires.iter().cloned().chain(g.iter().cloned()).collect()).collect() };
            outside.extend(own);
            into.requires.clear();
            into.any_of = outside;
            into.time = from.time;
            into.temporary_requires = from.temporary_requires.clone();
            into.before = from.before.clone();
        } else {
            copy(into, &from);
        }
    }
    if !remains_swap.is_empty() {
        let swap = |r: &mut String| {
            if let Some((_, to)) = remains_swap.iter().find(|(from, _)| from == r) {
                *r = to.clone();
            }
        };
        for e in &mut logic.entries {
            e.requires.iter_mut().for_each(swap);
            e.any_of.iter_mut().flatten().for_each(swap);
        }
    }
    Ok(placed.into_iter().collect())
}
