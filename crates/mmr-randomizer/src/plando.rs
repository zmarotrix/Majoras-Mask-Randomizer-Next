//! Item plando: put chosen items at chosen checks (the old
//! `*ItemPlando.json` files, same format).
//!
//! Each combo lists items and checks; `ItemDrawCount` of the items (all if
//! -1) go to random checks from the list, before anything else is placed.
//! Placement rules still apply unless `SkipLogic` is set, and the seed must
//! still be beatable.

use crate::fill::Placement;
use crate::rng::Rng;
use crate::world::{Id, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct PlandoCombo {
    pub name: String,
    pub notes: Option<String>,
    pub item_list: Vec<String>,
    pub check_list: Vec<String>,
    /// How many of the items to place (-1: all).
    pub item_draw_count: i64,
    /// Ignore this combo if it can't be placed, instead of failing.
    pub skip_if_error: bool,
    /// Place even where the usual rules wouldn't allow it.
    pub skip_logic: bool,
}

impl Default for PlandoCombo {
    fn default() -> Self {
        PlandoCombo { name: String::new(), notes: None, item_list: Vec::new(), check_list: Vec::new(), item_draw_count: -1, skip_if_error: false, skip_logic: false }
    }
}

/// Read a plando file: a list of combos.
pub fn parse(json: &str) -> Result<Vec<PlandoCombo>, String> {
    serde_json::from_str(json.trim_start_matches('\u{FEFF}')).map_err(|e| format!("plando file: {e}"))
}

/// Place every combo. Returns the items placed.
pub fn place(world: &World, combos: &[PlandoCombo], rng: &mut Rng, placement: &mut Placement) -> Result<BTreeSet<Id>, String> {
    let mut placed: BTreeSet<Id> = BTreeSet::new();
    let pool: BTreeSet<Id> = world.pool.iter().copied().collect();
    for combo in combos {
        let ids = |names: &[String]| -> Result<Vec<Id>, String> { names.iter().map(|n| world.id(n).ok_or_else(|| format!("plando \"{}\": no item or check called {n}", combo.name))).collect() };
        let mut items = ids(&combo.item_list)?;
        let mut checks = ids(&combo.check_list)?;
        rng.shuffle(&mut items);
        rng.shuffle(&mut checks);
        // Items already placed, and checks that aren't shuffled or are taken, drop out.
        items.retain(|i| pool.contains(i) && !placed.contains(i));
        checks.retain(|c| world.shuffled.binary_search(c).is_ok() && placement[*c].is_none());
        if items.is_empty() || checks.is_empty() {
            if combo.skip_if_error {
                continue;
            }
            let why = if items.is_empty() { "all its items are placed already or aren't shuffled" } else { "all its checks are taken or aren't shuffled" };
            return Err(format!("plando \"{}\" can't be placed: {why}", combo.name));
        }
        let want = if combo.item_draw_count < 0 { items.len() } else { combo.item_draw_count as usize };
        let mut drawn = 0;
        for &item in &items {
            if drawn >= want {
                break;
            }
            let spot = checks.iter().copied().find(|&c| {
                let forbidden_start = world.is_starting_location(c) && world.items[item].downgradable && !world.prevent_downgrades;
                combo.skip_logic || (!forbidden_start && world.allowed(item, c))
            });
            if let Some(c) = spot {
                placement[c] = Some(item);
                placed.insert(item);
                checks.retain(|&x| x != c);
                drawn += 1;
            }
        }
        if drawn < want && !combo.skip_if_error {
            return Err(format!("plando \"{}\": only {drawn} of {want} items fit its checks", combo.name));
        }
    }
    Ok(placed)
}
