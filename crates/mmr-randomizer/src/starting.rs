//! Extra starting items: random picks from the player's groups
//! (`RandomStartingItemGroups`) and Blitz (start with some boss remains,
//! with their dungeons left out: the old `ItemUtils.PrepareBlitz`).

use crate::rng::Rng;
use mmr_data::GameData;
use mmr_logic::LogicFile;
use mmr_settings::Values;
use serde_json::Value;

/// What Blitz changes.
#[derive(Debug, Clone, Default)]
pub struct Blitz {
    /// Items the player starts with (the remains, and what's left in their
    /// dungeons that can be started with).
    pub items: Vec<String>,
    /// Locations in the skipped dungeons: junk only.
    pub junk_locations: Vec<String>,
    /// Dungeon items in the skipped dungeons, left where they normally are.
    pub vanilla: Vec<String>,
}

fn list<'v>(values: &'v Values, key: &str) -> Vec<&'v str> {
    values.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default()
}

/// One random pick per group, as many as each group asks for.
pub fn random_groups(values: &Values, rng: &mut Rng) -> Vec<String> {
    let mut out = Vec::new();
    for g in values.get("RandomStartingItemGroups").and_then(Value::as_array).into_iter().flatten() {
        let items: Vec<String> = g.get("Items").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        let amount = g.get("Amount").and_then(Value::as_u64).unwrap_or(0) as usize;
        out.extend(rng.sample(&items, amount));
    }
    out
}

const REMAINS: [&str; 4] = ["RemainsOdolwa", "RemainsGoht", "RemainsGyorg", "RemainsTwinmold"];

fn lair(remains: &str) -> &'static str {
    match remains {
        "RemainsOdolwa" => "AreaOdolwasLair",
        "RemainsGoht" => "AreaGohtsLair",
        "RemainsGyorg" => "AreaGyorgsLair",
        _ => "AreaTwinmoldsLair",
    }
}

fn dungeon_accesses(lair: &str) -> &'static [&'static str] {
    match lair {
        "AreaOdolwasLair" => &["AreaWoodFallTempleAccess"],
        "AreaGohtsLair" => &["AreaSnowheadTempleAccess"],
        "AreaGyorgsLair" => &["AreaGreatBayTempleAccess"],
        _ => &["AreaInvertedStoneTowerTempleAccess", "AreaStoneTowerTempleAccess"],
    }
}

/// Entries that can be met with every item (except those that stay where
/// they are and can't be started with), with `closed` entries never met.
fn reachable<'l>(logic: &'l LogicFile, values: &Values, game: &GameData, shuffled: &[&str], vanilla: &[String], closed: &[String]) -> std::collections::HashSet<&'l str> {
    use std::collections::{HashMap, HashSet};
    let index: HashMap<&str, usize> = logic.entries.iter().enumerate().map(|(i, e)| (e.id.as_str(), i)).collect();
    let tricks = list(values, "EnabledTricks");
    let place_like = |id: &str| match game.item(id) {
        None => true,
        Some(i) if i.is_fake() || id.starts_with("BottleCatch") => true,
        Some(i) => i.location.is_some() && (!shuffled.contains(&id) || vanilla.iter().any(|v| v == id)) && !i.starting_item,
    };
    let mut met: HashSet<&str> = HashSet::new();
    loop {
        let mut changed = false;
        for e in &logic.entries {
            if met.contains(e.id.as_str()) || closed.contains(&e.id) {
                continue;
            }
            let ok = |r: &String| {
                if r == mmr_logic::settings_rules::IMPOSSIBLE {
                    return false;
                }
                match index.get(r.as_str()) {
                    Some(_) if place_like(r) => met.contains(r.as_str()),
                    _ => !closed.contains(r),
                }
            };
            let gate = match mmr_logic::EntryKind::of(e) {
                mmr_logic::EntryKind::Trick => tricks.contains(&e.id.as_str()),
                mmr_logic::EntryKind::Setting => e.setting.as_ref().is_some_and(|c| c.eval(values)),
                _ => true,
            };
            if gate && e.requires.iter().all(ok) && (e.any_of.is_empty() || e.any_of.iter().any(|g| g.iter().all(ok))) {
                met.insert(e.id.as_str());
                changed = true;
            }
        }
        if !changed {
            return met;
        }
    }
}

/// Blitz: pick the remains to start with, and leave their dungeons out.
/// `lair_at` gives the lair an entrance leads to (entrance randomization).
pub fn blitz(logic: &mut LogicFile, values: &Values, game: &GameData, rng: &mut Rng, lair_at: impl Fn(&str) -> String) -> Blitz {
    let mode = list(values, "BossRemainsMode");
    let amounts: Vec<usize> = (1..=4).filter(|n| mode.contains(&format!("Blitz{n}").as_str())).collect();
    if amounts.is_empty() {
        return Blitz::default();
    }
    // Fewer remains are likelier (weight 1/n).
    let weights: Vec<u32> = amounts.iter().map(|&n| (12 / n) as u32).collect();
    let amount = amounts[rng.weighted(&weights)];
    let starting = list(values, "CustomStartingItemList");
    let extra: Vec<&str> = REMAINS.iter().copied().filter(|r| starting.contains(r)).collect();
    let chosen: Vec<&str> = if amount <= extra.len() {
        rng.sample(&extra, amount)
    } else {
        let others: Vec<&str> = REMAINS.iter().copied().filter(|r| !extra.contains(r)).collect();
        let mut c = extra.clone();
        c.extend(rng.sample(&others, amount - extra.len()));
        c
    };

    let shuffled = list(values, "CustomItemList");
    let item = |id: &str| game.items.iter().find(|i| i.id == id);
    let startable = |id: &str| item(id).is_some_and(|i| i.starting_item);
    let is_location = |id: &str| item(id).is_some_and(|i| i.location.is_some());
    let fairy_mode = list(values, "StrayFairyMode");
    let mut dungeon_items: Vec<String> = game
        .items
        .iter()
        .filter(|i| {
            let id = i.id.as_str();
            ["Woodfall", "Snowhead", "GreatBay", "StoneTower"].iter().any(|t| id.starts_with(&format!("Item{t}Key")) || id == format!("Item{t}BossKey"))
                || (i.item_category.as_deref() == Some("Navigation") && ["Woodfall", "Snowhead", "GreatBay", "StoneTower"].iter().any(|t| id.starts_with(&format!("Item{t}"))))
        })
        .map(|i| i.id.clone())
        .collect();
    let within_temples = fairy_mode.contains(&"KeepWithinTemples");
    if (within_temples && fairy_mode.contains(&"KeepWithinArea")) || (chosen.len() > 1 && within_temples) {
        let first = game.items.iter().position(|i| i.id == "CollectibleStrayFairyWoodfall1").unwrap_or(0);
        dungeon_items.extend(game.items[first..first + 60].iter().map(|i| i.id.clone()));
    }

    let mut out = Blitz { items: chosen.iter().map(|s| s.to_string()).collect(), ..Default::default() };
    // The entries for getting into the dungeons: the old place entries, or
    // in logic with regions, the regions they became.
    let entry_for = |access: &str| -> String {
        if logic.entry(access).is_some() {
            return access.to_string();
        }
        mmr_logic::region::region_entry_id(&mmr_logic::regionize::region_name(access))
    };
    let accesses: Vec<String> = chosen.iter().flat_map(|r| dungeon_accesses(&lair_at(lair(r))).iter().map(|a| entry_for(a))).collect();
    if accesses.is_empty() {
        return out;
    }
    // What's lost with the dungeons: everything that can be reached with
    // every item, but not once their entrances are closed. Items count as
    // owned, except those that never move and can't be started with (they
    // must be collected where they are). The old code worked this out from
    // the shape of the logic; this gives the same answer for any logic.
    let before = reachable(logic, values, game, &shuffled, &out.vanilla, &[]);
    loop {
        let after = reachable(logic, values, game, &shuffled, &out.vanilla, &accesses);
        let lost: Vec<String> = logic.entries.iter().filter(|e| before.contains(e.id.as_str()) && !after.contains(e.id.as_str())).map(|e| e.id.clone()).collect();
        let mut changed = false;
        for id in &lost {
            if is_location(id) && shuffled.contains(&id.as_str()) && dungeon_items.contains(id) && !out.vanilla.contains(id) {
                out.vanilla.push(id.clone());
                changed = true;
            }
        }
        if !changed {
            for id in &lost {
                let is_vanilla = is_location(id) && (!shuffled.contains(&id.as_str()) || out.vanilla.contains(id));
                if is_vanilla && startable(id) && !out.items.contains(id) {
                    out.items.push(id.clone());
                }
            }
            out.junk_locations = accesses.iter().cloned().chain(lost).collect();
            break;
        }
    }
    // The dungeons are closed for logic too.
    for e in logic.entries.iter_mut().filter(|e| accesses.contains(&e.id)) {
        e.requires = vec![mmr_logic::settings_rules::IMPOSSIBLE.into()];
        e.any_of.clear();
    }
    // In game order, whatever the logic's order.
    let order = |id: &String| game.items.iter().position(|i| i.id == *id).unwrap_or(usize::MAX);
    out.vanilla.sort_by_key(order);
    let mut rest = out.items.split_off(chosen.len());
    rest.sort_by_key(order);
    out.items.extend(rest);
    out.junk_locations.sort_by_key(order);
    out.junk_locations.dedup();
    out.junk_locations.retain(|l| item(l).is_some_and(|i| (i.location.is_some() || i.main_location.is_some()) && !i.entrance));
    out
}
