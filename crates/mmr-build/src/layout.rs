//! Turn MMR Next's own randomizer output and settings into a build [`Plan`].
//!
//! Kept free of randomizer types: the caller fills in a [`SeedLayout`].

use crate::plan::{GibdoRequirement, LogicEntry, PlacedItem, Plan};
use serde_json::{Map, Value};

/// One location's contents.
#[derive(Debug, Clone)]
pub struct LayoutItem {
    pub location: String,
    /// The item logic placed there.
    pub item: String,
    /// What the player gets (junk in place of e.g. a starting item).
    pub given: String,
    pub randomized: bool,
}

/// A logic entry in the old form: everything in `requires`, and one of each
/// `any_of` group.
#[derive(Debug, Clone)]
pub struct LogicItem {
    pub id: String,
    pub requires: Vec<String>,
    pub any_of: Vec<Vec<String>>,
    /// Not a real item (an area, event or helper).
    pub fake: bool,
}

/// Everything about a seed the ROM builder needs.
#[derive(Debug, Clone)]
pub struct SeedLayout {
    pub seed: u64,
    /// Settings as MMR Next stores them (flat keys, `Cosmetic.` prefix).
    pub settings: Map<String, Value>,
    pub items: Vec<LayoutItem>,
    pub junk_locations: Vec<String>,
    pub logic: Vec<LogicItem>,
    /// Hue rotations (0-359) for the title logo and file select screen.
    pub title_logo_color: i32,
    pub file_select_skybox: i32,
    pub file_select_color: i32,
    /// Random prices, one per price the game has (`None`: unchanged).
    pub message_costs: Option<Vec<Option<u16>>>,
    /// The Gibdos' requests: (item, amount, logic entry). Empty: vanilla.
    pub gibdos: Vec<(String, u8, Option<String>)>,
    /// Extra starting items: Blitz, and the random starting item groups.
    pub blitz_extra_items: Vec<String>,
    pub random_starting_items: Vec<String>,
    /// Shuffled entrances: (entrance, the entrance it's now behind).
    pub entrances: Vec<(String, String)>,
}

/// The Ikana well Gibdos' vanilla requests.
const VANILLA_GIBDOS: [(&str, u8, Option<&str>); 16] = [
    ("BluePotion", 1, Some("GibdoEntranceLeft")),
    ("MagicBeans", 5, Some("GibdoEntranceRight")),
    ("SpringWater", 1, Some("GibdoToBombPots")),
    ("Fish", 1, Some("GibdoToHotWater")),
    ("Bugs", 1, Some("GibdoToFairyFountain")),
    ("DekuNuts", 10, Some("GibdoToCowAndMiniboss")),
    ("Bombs", 10, Some("GibdoToMiniBoss")),
    ("HotSpringWater", 1, Some("GibdoToCow")),
    ("BigPoe", 1, Some("GibdoToFinalWallmaster")),
    ("Milk", 1, Some("GibdoToMirrorShield")),
    ("Bugs", 1, Some("GibdoToLeftChest")),
    ("Bugs", 1, Some("GibdoToRightChest")),
    ("Fish", 1, Some("GibdoToBlackBoes")),
    ("Bugs", 1, None),
    ("Bugs", 1, None),
    ("Bugs", 1, None),
];

impl Plan {
    pub fn from_layout(layout: &SeedLayout) -> Plan {
        let mut settings = Map::new();
        let mut cosmetic = Map::new();
        let list = |key: &str| layout.settings.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        let custom_item_list: Vec<String> = list("CustomItemList");
        let custom_starting_item_list: Vec<String> = list("CustomStartingItemList");
        let custom_junk_locations: Vec<String> = list("CustomJunkLocations");
        let mut cutscenes = Map::new();
        for (key, value) in &layout.settings {
            if let Some(rest) = key.strip_prefix("Cosmetic.") {
                cosmetic.insert(rest.to_string(), value.clone());
            } else if let Some(rest) = key.strip_prefix("ShortenCutsceneSettings.") {
                cutscenes.insert(rest.to_string(), value.clone());
            } else {
                settings.insert(key.clone(), value.clone());
            }
        }
        settings.insert("ShortenCutsceneSettings".into(), Value::Object(cutscenes));

        let items = layout
            .items
            .iter()
            .map(|e| PlacedItem { id: e.item.clone(), item: e.given.clone(), location: Some(e.location.clone()), is_randomized: e.randomized, name_override: None, mimic: None })
            .chain(layout.entrances.iter().map(|(e, t)| PlacedItem { id: e.clone(), item: e.clone(), location: Some(t.clone()), is_randomized: true, name_override: None, mimic: None }))
            .collect();

        // Items nothing in logic needs (MMR 2.0 `LogicallyJunkItems`, which
        // considers every item in the game).
        let used: std::collections::HashSet<&str> = layout.logic.iter().flat_map(|l| l.requires.iter().chain(l.any_of.iter().flatten())).map(String::as_str).collect();
        let mut candidates: Vec<String> = crate::items::table().values.iter().map(|v| v.name.clone()).collect();
        candidates.extend(layout.logic.iter().map(|l| l.id.clone()));
        candidates.sort();
        candidates.dedup();
        let logically_junk_items = candidates.into_iter().filter(|id| !used.contains(id.as_str())).collect();

        Plan {
            seed: layout.seed as i64,
            version: None,
            hash: None,
            cosmetic,
            settings,
            custom_item_list,
            custom_starting_item_list,
            custom_junk_locations,
            items,
            gossip_quotes: None,
            hint_info: Vec::new(),
            message_costs: layout.message_costs.clone(),
            gibdo_requirements: if layout.gibdos.is_empty() {
                VANILLA_GIBDOS.iter().map(|&(item, amount, logic)| GibdoRequirement { item_required: item.into(), amount, logic_entry: logic.map(String::from) }).collect()
            } else {
                layout.gibdos.iter().map(|(item, amount, logic)| GibdoRequirement { item_required: item.clone(), amount: *amount, logic_entry: logic.clone() }).collect()
            },
            junk_locations: layout.junk_locations.clone(),
            logically_junk_items,
            logic: layout
                .logic
                .iter()
                .map(|l| LogicEntry { id: l.id.clone(), required: Some(l.requires.clone()), conditional: Some(l.any_of.clone()), is_fake_item: l.fake })
                .collect(),
            blitz_extra_items: Some(layout.blitz_extra_items.clone()),
            random_starting_items: Some(layout.random_starting_items.clone()),
            traps: None,
            file_select_skybox: layout.file_select_skybox,
            file_select_color: layout.file_select_color,
            title_logo_color: layout.title_logo_color,
        }
    }
}
