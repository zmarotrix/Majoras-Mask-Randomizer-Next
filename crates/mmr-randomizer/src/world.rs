//! Everything placement needs to know, prepared once per generation:
//! which locations are shuffled, what each item is, and the rules for where
//! an item may go.
//!
//! Items and locations share ids (game data index): the item normally found
//! at location `X` is item `X`.

use mmr_data::GameData;
use mmr_settings::Values;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub type Id = usize;

/// The four temples (MMR 2.0 `RestrictedPlacementAttribute`).
const TEMPLE_REGIONS: &[&str] = &["WoodfallTemple", "SnowheadTemple", "GreatBayTemple", "StoneTowerTemple"];
/// Starting slots (`ItemUtils.IsStartingLocation`).
const STARTING_LOCATIONS: &[&str] =
    &["MaskDeku", "SongHealing", "StartingSword", "StartingShield", "StartingHeartContainer1", "StartingHeartContainer2"];
/// Dungeon Great Fairy rewards.
const GREAT_FAIRY_REWARDS: &[&str] = &["FairySpinAttack", "FairyDoubleMagic", "FairyDoubleDefense", "ItemFairySword"];
/// Checks that may not hold important items lost on the Song of Time (MMR
/// 2.0: the business scrub relocation could otherwise make seeds unbeatable).
const NO_TEMPORARY_LOCATIONS: &[&str] = &["HeartPieceNotebookHand", "NotebookToiletHandSThanks"];
/// Regions where temporary items (lost on Song of Time) may not be placed.
const NO_TEMPORARY_REGIONS: &[&str] = &["TheMoon", "ClockTowerRoof"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Town,
    Swamp,
    Mountain,
    Ocean,
    Canyon,
    Ranch,
    None,
}

impl Area {
    fn parse(s: Option<&str>) -> Area {
        match s {
            Some("Town") => Area::Town,
            Some("Swamp") => Area::Swamp,
            Some("Mountain") => Area::Mountain,
            Some("Ocean") => Area::Ocean,
            Some("Canyon") => Area::Canyon,
            Some("Ranch") => Area::Ranch,
            _ => Area::None,
        }
    }
}

/// Where an item may be placed, from the key/fairy/remains/map settings.
#[derive(Debug, Clone, Default)]
pub struct Restriction {
    temples: bool,
    area: bool,
    overworld: bool,
    great_fairies: bool,
}

#[derive(Debug, Clone)]
pub struct ItemInfo {
    pub id: String,
    pub name: Option<String>,
    pub location: Option<String>,
    pub region: Option<String>,
    pub area: Area,
    pub in_temple: bool,
    pub song: bool,
    /// Lost on the Song of Time, with these settings.
    pub temporary: bool,
    /// An entrance: being through it is a state, not something you keep.
    pub entrance: bool,
    /// The Ocarina and the Song of Time.
    pub time_travel: bool,
    pub can_start_with: bool,
    pub category: Option<String>,
    /// Can be downgraded (a better sword, shield...): never a starting item
    /// unless downgrades are prevented.
    pub downgradable: bool,
}

pub struct World<'a> {
    pub game: &'a GameData,
    pub items: Vec<ItemInfo>,
    pub index: BTreeMap<&'a str, Id>,
    /// Locations whose contents are shuffled.
    pub shuffled: Vec<Id>,
    /// Items to place: the items normally found at the shuffled locations.
    pub pool: Vec<Id>,
    /// Items the player starts with.
    pub starting: Vec<Id>,
    /// Items that give junk when found (starting items, keys for open
    /// doors, removed stray fairies). They are still placed by logic.
    pub gives_junk: BTreeSet<Id>,
    /// Locations that may only hold junk.
    pub junk_locations: BTreeSet<Id>,
    /// Items that can never be required (MMR 2.0 `JunkItems`).
    pub junk_items: BTreeSet<Id>,
    /// Items the logic refers to.
    pub logic_items: BTreeSet<Id>,
    pub add_songs: bool,
    pub remains_shuffle_only: bool,
    pub giant_mask_anywhere: bool,
    /// "None", "Random" or "AllowTemporaryItems": what shuffled starting
    /// slots get.
    pub starting_item_mode: String,
    pub prevent_downgrades: bool,
    /// Boss remains go on Great Fairy rewards.
    pub great_fairy_remains: bool,
    /// "No logic": items go anywhere they're allowed, beatable or not.
    pub no_logic: bool,
    /// Items the player chose checks for (item plando).
    pub plando: Vec<crate::plando::PlandoCombo>,
    /// Caps on hint priority tiers.
    pub hint_caps: Option<crate::hint_caps::HintCaps>,
    /// Starting items chosen for the seed (Blitz, random groups).
    pub extra_starting: Vec<Id>,
    restrictions: BTreeMap<Id, Restriction>,
}

fn list<'v>(values: &'v Values, key: &str) -> Vec<&'v str> {
    values.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default()
}

fn has_flag(values: &Values, key: &str, flag: &str) -> bool {
    list(values, key).contains(&flag)
}

impl<'a> World<'a> {
    pub fn new(game: &'a GameData, logic: &mmr_logic::LogicFile, values: &Values) -> World<'a> {
        let index: BTreeMap<&str, Id> = game.items.iter().enumerate().map(|(i, it)| (it.id.as_str(), i)).collect();
        let items: Vec<ItemInfo> = game
            .items
            .iter()
            .map(|it| {
                let region = it.region.clone();
                let r = region.as_deref().and_then(|r| game.region(r));
                ItemInfo {
                    id: it.id.clone(),
                    name: it.name.clone(),
                    location: it.location.clone(),
                    area: Area::parse(r.and_then(|r| r.area.as_deref())),
                    in_temple: region.as_deref().is_some_and(|r| TEMPLE_REGIONS.contains(&r)),
                    region,
                    song: it.song,
                    temporary: it.has_flag("Temporary")
                        && !it
                            .temporary_unless
                            .as_ref()
                            .and_then(|c| serde_json::from_value::<mmr_logic::SettingCondition>(c.clone()).ok())
                            .is_some_and(|c| c.eval(values)),
                    entrance: it.entrance,
                    time_travel: it.item_category.as_deref() == Some("TimeTravel"),
                    can_start_with: it.starting_item,
                    category: it.item_category.clone(),
                    downgradable: it.has_flag("Downgradable"),
                }
            })
            .collect();
        let id = |s: &str| index.get(s).copied();

        let mut shuffled: Vec<Id> =
            list(values, "CustomItemList").iter().filter_map(|s| id(s)).filter(|&i| items[i].location.is_some()).collect();
        shuffled.sort_unstable();
        shuffled.dedup();

        let starting: Vec<Id> = list(values, "CustomStartingItemList").iter().filter_map(|s| id(s)).collect();
        let mut gives_junk: BTreeSet<Id> = starting.iter().copied().collect();
        let small_keys: Vec<Id> = items.iter().enumerate().filter(|(_, i)| is_small_key(&i.id)).map(|(n, _)| n).collect();
        let boss_keys: Vec<Id> = items.iter().enumerate().filter(|(_, i)| is_boss_key(&i.id)).map(|(n, _)| n).collect();
        // MMR 2.0 `DungeonStrayFairies`: 60 items from Woodfall's first.
        let first_fairy = index["CollectibleStrayFairyWoodfall1"];
        let fairies: Vec<Id> = (first_fairy..first_fairy + 60).collect();
        if has_flag(values, "SmallKeyMode", "DoorsOpen") {
            gives_junk.extend(&small_keys);
        }
        if has_flag(values, "BossKeyMode", "DoorsOpen") {
            gives_junk.extend(&boss_keys);
        }
        if has_flag(values, "StrayFairyMode", "ChestsOnly") {
            gives_junk.extend(&fairies);
            // Fairies outside chests are removed: their spots stay as they are.
            let chest_fairy = |i: Id| game.items[i].location_category.as_deref() == Some("Chests");
            shuffled.retain(|&l| !fairies.contains(&l) || chest_fairy(l));
        }

        let pool = shuffled.clone();

        let mut junk_locations: BTreeSet<Id> = list(values, "CustomJunkLocations").iter().filter_map(|s| id(s)).collect();
        // A main location being junk makes its sub-locations junk too.
        for (i, it) in game.items.iter().enumerate() {
            if it.main_location.as_ref().is_some_and(|m| m.iter().any(|m| id(m).is_some_and(|m| junk_locations.contains(&m)))) {
                junk_locations.insert(i);
            }
        }

        let logic_items: BTreeSet<Id> = logic
            .entries
            .iter()
            .flat_map(|e| e.references())
            .filter(|r| mmr_logic::is_item_ref(r))
            .filter_map(id)
            .collect();

        // MMR 2.0 JunkItems: never required, not unique by name, not a heart
        // piece, and found in a small wooden chest.
        let mut name_counts: BTreeMap<&str, usize> = BTreeMap::new();
        for it in &game.items {
            if let Some(n) = it.name.as_deref() {
                *name_counts.entry(n).or_default() += 1;
            }
        }
        let junk_items: BTreeSet<Id> = game
            .items
            .iter()
            .enumerate()
            .filter(|(i, it)| {
                !logic_items.contains(i)
                    && it.name.as_deref().is_some_and(|n| name_counts[n] > 1)
                    && it.item_category.as_deref() != Some("PiecesOfHeart")
                    && it.chest_type.as_deref() == Some("SmallWooden")
            })
            .map(|(i, _)| i)
            .collect();

        let mut restrictions: BTreeMap<Id, Restriction> = BTreeMap::new();
        let mut restrict = |ids: &[Id], key: &str| {
            let r = Restriction {
                temples: has_flag(values, key, "KeepWithinTemples"),
                area: has_flag(values, key, "KeepWithinArea"),
                overworld: has_flag(values, key, "KeepWithinOverworld"),
                great_fairies: has_flag(values, key, "GreatFairyRewards"),
            };
            if r.temples || r.area || r.overworld || r.great_fairies {
                for &i in ids {
                    restrictions.insert(i, r.clone());
                }
            }
        };
        restrict(&small_keys, "SmallKeyMode");
        restrict(&boss_keys, "BossKeyMode");
        restrict(&fairies, "StrayFairyMode");
        let remains: Vec<Id> = items.iter().enumerate().filter(|(_, i)| i.category.as_deref() == Some("BossRemains")).map(|(n, _)| n).collect();
        restrict(&remains, "BossRemainsMode");
        let navigation: Vec<Id> = items
            .iter()
            .enumerate()
            .filter(|(_, i)| i.category.as_deref() == Some("Navigation") && TEMPLE_NAMES.iter().any(|t| i.id.starts_with(&format!("Item{t}"))))
            .map(|(n, _)| n)
            .collect();
        restrict(&navigation, "DungeonNavigationMode");

        let mut world = World {
            game,
            items,
            index,
            shuffled,
            pool,
            starting,
            gives_junk,
            junk_locations,
            junk_items,
            logic_items,
            add_songs: values.get("AddSongs").and_then(Value::as_bool).unwrap_or(false),
            remains_shuffle_only: has_flag(values, "BossRemainsMode", "ShuffleOnly"),
            giant_mask_anywhere: values.get("GiantMaskAnywhere").and_then(Value::as_bool).unwrap_or(false),
            starting_item_mode: values.get("StartingItemMode").and_then(Value::as_str).unwrap_or("None").to_string(),
            prevent_downgrades: values.get("PreventDowngrades").and_then(Value::as_bool).unwrap_or(false),
            great_fairy_remains: has_flag(values, "BossRemainsMode", "GreatFairyRewards"),
            no_logic: values.get("LogicMode").and_then(Value::as_str) == Some("NoLogic"),
            plando: Vec::new(),
            hint_caps: None,
            extra_starting: Vec::new(),
            restrictions,
        };
        world.hint_caps = crate::hint_caps::HintCaps::new(&world, values);
        world
    }

    /// Add the seed's extra starting items, and Blitz's skipped dungeons.
    pub fn apply_starting(&mut self, blitz: &crate::starting::Blitz, random: &[String]) {
        let ids = |v: &[String]| -> Vec<Id> { v.iter().filter_map(|s| self.index.get(s.as_str()).copied()).collect() };
        let extra: Vec<Id> = ids(&blitz.items).into_iter().chain(ids(random)).collect();
        for &i in &extra {
            if !self.starting.contains(&i) {
                self.starting.push(i);
            }
            self.gives_junk.insert(i);
            if !self.extra_starting.contains(&i) {
                self.extra_starting.push(i);
            }
        }
        self.junk_locations.extend(ids(&blitz.junk_locations));
        // Dungeon items of skipped dungeons stay where they are.
        for i in ids(&blitz.vanilla) {
            self.shuffled.retain(|&l| l != i);
            self.pool.retain(|&p| p != i);
        }
    }

    pub fn is_starting_location(&self, location: Id) -> bool {
        STARTING_LOCATIONS.contains(&self.items[location].id.as_str())
    }

    pub fn id(&self, s: &str) -> Option<Id> {
        self.index.get(s).copied()
    }

    pub fn is_junk(&self, item: Id) -> bool {
        self.junk_items.contains(&item)
    }

    /// Whether `item` may be placed at `location` (MMR 2.0 `CheckMatch`
    /// minus the logic part, which assumed fill handles).
    pub fn allowed(&self, item: Id, location: Id) -> bool {
        let it = &self.items[item];
        let loc = &self.items[location];
        let starting_slot = STARTING_LOCATIONS.contains(&loc.id.as_str());
        if starting_slot && (!it.can_start_with || (it.downgradable && !self.prevent_downgrades)) {
            return false;
        }
        if self.gives_junk.contains(&item) {
            return true;
        }
        if (self.junk_locations.contains(&location) || loc.id == "UpgradeRoyalWallet") && !self.is_junk(item) {
            return false;
        }
        if !self.add_songs && it.song != loc.song {
            return false;
        }
        if self.remains_shuffle_only {
            let remains = |i: &ItemInfo| i.category.as_deref() == Some("BossRemains");
            if remains(it) != remains(loc) {
                return false;
            }
        }
        if it.temporary && loc.region.as_deref().is_some_and(|r| NO_TEMPORARY_REGIONS.contains(&r)) {
            return false;
        }
        // MMR 2.0 places the time-travel items while treating everything as
        // temporary, so they follow this rule too.
        if (it.temporary || it.time_travel) && !self.is_junk(item) && NO_TEMPORARY_LOCATIONS.contains(&loc.id.as_str()) {
            return false;
        }
        if let Some(r) = self.restrictions.get(&item) {
            if r.great_fairies && !GREAT_FAIRY_REWARDS.contains(&loc.id.as_str()) {
                return false;
            }
            if r.temples && !(loc.in_temple || loc.id == "SongOath") {
                return false;
            }
            if r.overworld && (loc.in_temple || loc.id == "SongOath") {
                return false;
            }
            if r.area && it.area != loc.area {
                return false;
            }
        }
        true
    }

    /// Items that must be placed by logic: those the logic refers to, plus
    /// anything with a placement restriction.
    pub fn is_progression(&self, item: Id) -> bool {
        self.logic_items.contains(&item) || self.restrictions.contains_key(&item)
    }

    pub fn is_restricted(&self, item: Id) -> bool {
        self.restrictions.contains_key(&item) || (!self.add_songs && self.items[item].song)
    }
}

const TEMPLE_NAMES: &[&str] = &["Woodfall", "Snowhead", "GreatBay", "StoneTower"];

fn is_small_key(id: &str) -> bool {
    TEMPLE_NAMES.iter().any(|t| id.starts_with(&format!("Item{t}Key")))
}

fn is_boss_key(id: &str) -> bool {
    TEMPLE_NAMES.iter().any(|t| id == format!("Item{t}BossKey"))
}
