//! Static game data, embedded at compile time from `data/`.
//!
//! The JSON files are generated from the legacy C# source by
//! `tools/extract_csharp.py` + `tools/curate.py`; edit those (or the JSON
//! directly once the legacy code is retired), not this crate.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

const ITEMS_JSON: &str = include_str!("../../../data/items.json");
const REGIONS_JSON: &str = include_str!("../../../data/regions.json");
const CATEGORIES_JSON: &str = include_str!("../../../data/categories.json");

/// Logic files that ship with the randomizer: `(id, display name, json)`.
pub const BUNDLED_LOGIC: &[(&str, &str, &str)] = &[
    ("casual", "Casual", include_str!("../../../data/logic/casual.json")),
    ("glitched", "Glitched", include_str!("../../../data/logic/glitched.json")),
];

/// The bundled logic in the region format (what the logic editor edits).
pub const BUNDLED_REGIONS: &[(&str, &str)] = &[
    ("casual", include_str!("../../../data/logic/casual.regions.json")),
    ("glitched", include_str!("../../../data/logic/glitched.regions.json")),
];

/// An entry of the legacy `Item` enum. In MMR every item is also the
/// location it is found at in the vanilla game, so this doubles as a location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    /// Position in the legacy enum; legacy settings strings depend on it.
    pub index: usize,
    /// Display name of the item. `None` for "fake" items (areas, events).
    pub name: Option<String>,
    /// Display name of the location. `None` if this is not a location.
    pub location: Option<String>,
    pub region: Option<String>,
    pub item_category: Option<String>,
    pub location_category: Option<String>,
    pub classic_category: Option<String>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub main_location: Option<Vec<String>>,
    /// The item is obtainable at any one of several locations (e.g. the
    /// Postbox heart piece). Its logic is derived: any of these locations.
    #[serde(default)]
    pub multi_location: Option<Vec<String>>,
    /// A song (MMR 2.0 `IsSong`). Songs only swap with songs unless the
    /// "shuffle songs with items" setting is on.
    #[serde(default)]
    pub song: bool,
    /// Chest the vanilla item comes in, e.g. "SmallWooden", "LargeGold".
    #[serde(default)]
    pub chest_type: Option<String>,
    /// An entrance that entrance randomization can shuffle.
    #[serde(default)]
    pub entrance: bool,
    /// For items flagged `Temporary`: a setting condition (in the logic
    /// format) under which the item is kept through the Song of Time.
    #[serde(default)]
    pub temporary_unless: Option<serde_json::Value>,
    /// Can be chosen as a custom starting item.
    #[serde(default)]
    pub starting_item: bool,
    #[serde(default)]
    pub shop_text: Option<String>,
    #[serde(default)]
    pub gossip_location_hints: Vec<String>,
    #[serde(default)]
    pub gossip_item_hints: Vec<String>,
}

impl Item {
    /// Fake items have no name and are never placed (areas, events, ...).
    pub fn is_fake(&self) -> bool {
        self.name.is_none()
    }

    pub fn is_location(&self) -> bool {
        self.location.is_some()
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub name: String,
    pub preposition: Option<String>,
    pub area: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub value: i64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Categories {
    #[serde(rename = "ItemCategory")]
    pub item: Vec<Category>,
    #[serde(rename = "LocationCategory")]
    pub location: Vec<Category>,
    #[serde(rename = "ClassicCategory")]
    pub classic: Vec<Category>,
}

/// All static game data, with lookup indexes.
pub struct GameData {
    pub items: Vec<Item>,
    pub regions: Vec<Region>,
    pub categories: Categories,
    item_index: HashMap<String, usize>,
    region_index: HashMap<String, usize>,
}

impl GameData {
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.item_index.get(id).map(|&i| &self.items[i])
    }

    pub fn region(&self, id: &str) -> Option<&Region> {
        self.region_index.get(id).map(|&i| &self.regions[i])
    }

    /// Items that are locations, in legacy enum order (`ItemUtils.AllLocations`).
    pub fn locations(&self) -> impl Iterator<Item = &Item> {
        self.items.iter().filter(|i| i.is_location())
    }

    /// Items that can be chosen as starting items, in legacy enum order
    /// (`ItemUtils.CustomStartingItems`).
    pub fn starting_items(&self) -> impl Iterator<Item = &Item> {
        self.items.iter().filter(|i| i.starting_item)
    }
}

static GAME_DATA: LazyLock<GameData> = LazyLock::new(|| {
    let items: Vec<Item> = serde_json::from_str(ITEMS_JSON).expect("data/items.json is valid");
    let regions: Vec<Region> = serde_json::from_str(REGIONS_JSON).expect("data/regions.json is valid");
    let categories: Categories = serde_json::from_str(CATEGORIES_JSON).expect("data/categories.json is valid");
    let item_index = items.iter().enumerate().map(|(i, it)| (it.id.clone(), i)).collect();
    let region_index = regions.iter().enumerate().map(|(i, r)| (r.id.clone(), i)).collect();
    GameData { items, regions, categories, item_index, region_index }
});

/// The embedded game data. Parsed on first access.
pub fn game() -> &'static GameData {
    &GAME_DATA
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_items() {
        let g = game();
        assert!(g.items.len() > 1000);
        let bow = g.item("ItemBow").unwrap();
        assert_eq!(bow.name.as_deref(), Some("Hero's Bow"));
        assert_eq!(bow.location.as_deref(), Some("Hero's Bow Chest"));
        assert_eq!(g.region(bow.region.as_deref().unwrap()).unwrap().name, "Woodfall Temple");
        assert_eq!(g.items.iter().enumerate().filter(|(i, it)| *i != it.index).count(), 0);
    }

    #[test]
    fn bundled_logic_is_json() {
        for (id, _, json) in BUNDLED_LOGIC {
            serde_json::from_str::<serde_json::Value>(json).unwrap_or_else(|e| panic!("{id}: {e}"));
        }
    }
}
