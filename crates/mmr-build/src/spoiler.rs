//! The spoiler log: everything about a seed, as data (for the tracker and
//! JSON), as a plain-text log, and as the interactive HTML tracker.

use crate::hash_icons::HashIcon;
use crate::items;
use crate::plan::Plan;
use crate::SongPlacement;
use mmr_logic::LogicFile;
use mmr_randomizer::importance::Importance;
use mmr_randomizer::Outcome;
use mmr_settings::Values;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write;

const STARTING_LOCATIONS: [&str; 6] = ["MaskDeku", "SongHealing", "StartingSword", "StartingShield", "StartingHeartContainer1", "StartingHeartContainer2"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spoiler {
    pub version: String,
    pub seed: String,
    pub settings_string: String,
    /// The build hash, hex.
    pub hash: String,
    pub hash_icons: Vec<Icon>,
    pub logic_mode: String,
    pub placement: String,
    /// Settings worth knowing at a glance (label, value).
    pub highlights: Vec<(String, String)>,
    pub starting: Vec<Named>,
    pub areas: Vec<Area>,
    pub checks: Vec<Check>,
    /// Indexes into `checks`, sphere by sphere.
    pub spheres: Vec<Vec<usize>>,
    pub entrances: Vec<EntranceRow>,
    pub hints: Vec<HintRow>,
    pub prices: Vec<PriceRow>,
    pub gibdos: Vec<PriceRow>,
    pub music: Vec<SongPlacement>,
    pub logic: TrackerLogic,
    /// Item icons as PNG data URLs, by item id.
    pub icons: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Icon {
    pub name: String,
    /// `data:image/png;base64,...`
    pub png: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Named {
    pub id: String,
    pub name: String,
    /// Why it's there ("Starting item", "Blitz", "Random pick").
    pub source: String,
}

/// A group of regions (Clock Town, Swamp...), for the tracker's layout.
#[derive(Debug, Clone, Serialize)]
pub struct Area {
    pub name: String,
    pub regions: Vec<RegionRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegionRow {
    pub id: String,
    pub name: String,
    /// Indexes into `checks`.
    pub checks: Vec<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub id: String,
    pub name: String,
    pub region: String,
    /// The region's display name.
    pub region_name: String,
    /// What logic placed here (owned when the check is done).
    pub item: String,
    /// What the player sees.
    pub item_name: String,
    pub category: String,
    pub junk: bool,
    pub required: bool,
    pub important: bool,
    /// Sphere it's collected in (1-based), if it's in the playthrough.
    pub sphere: Option<usize>,
    /// The item whose icon shows for what's here (a disguise for traps).
    pub icon: String,
    /// When it can be done: bits for Day 1, Night 1 ... Night 3 (0: any time).
    pub times: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntranceRow {
    /// "Dungeons", "Boss rooms", "Grottos" or "Interiors".
    pub kind: String,
    pub entrance: String,
    /// Where the entrance is.
    pub region: String,
    pub leads_to: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HintRow {
    /// "Gossip stone", "Garo" or "Moon".
    pub kind: String,
    pub stone: String,
    /// Plain text.
    pub text: String,
    /// Text with colours, as safe HTML.
    pub html: String,
    /// "woth", "foolish", "song", "item" or "junk".
    pub verdict: String,
    /// The region a region hint is about.
    pub region: Option<String>,
    /// Checks the hint names outright.
    pub locations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PriceRow {
    pub name: String,
    pub value: u32,
}

/// Logic for the tracker's "what can I reach now": entries with
/// requirements as indexes into `names`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackerLogic {
    /// Ids referred to (items, entries).
    pub names: Vec<String>,
    /// Per name: 1 for an item, 2 for "never" (ruled out by the settings),
    /// else 0 (an entry; ids with no entry count as met).
    pub is_item: Vec<u8>,
    /// Entries: (name index, requires, any_of). Tricks that are off can
    /// never be met.
    pub entries: Vec<(u32, Vec<u32>, Vec<Vec<u32>>)>,
    /// Location name index -> item name index, for checks that aren't
    /// shuffled (collected as soon as they're reachable).
    pub vanilla: Vec<(u32, u32)>,
    /// Item name indexes the player starts with.
    pub start: Vec<u32>,
    /// Name index of each check's location and item.
    pub checks: Vec<(u32, u32)>,
    /// Time of day, as the randomizer's reachability sees it (see
    /// `mmr_randomizer::reach`). Per entry: temporary requirements, window
    /// (`setup` else `available`), `available`, flags (1: no return,
    /// 2: opening hours), layer its requirements are read from and layer it
    /// never happens in (0: none).
    pub tmp: Vec<Vec<u32>>,
    pub window: Vec<u8>,
    pub avail: Vec<u8>,
    pub flags: Vec<u8>,
    pub read_layer: Vec<u8>,
    pub absent: Vec<u8>,
    /// Per name: item flags (1: kept through the Song of Time, 2: a time
    /// travel item), `needed` times, layer it never happens in.
    pub item_flags: Vec<u8>,
    pub needed: Vec<u8>,
    pub item_absent: Vec<u8>,
    /// Layers: the real cycle plus one per `before` target.
    pub layers: u32,
    /// Name index of the time travel entry, if any.
    pub time_travel: Option<u32>,
}

fn humanize(id: &str) -> String {
    let mut out = String::new();
    for (i, c) in id.chars().enumerate() {
        let prev = id.chars().nth(i.wrapping_sub(1));
        if i > 0 && (c.is_uppercase() && !prev.is_some_and(|p| p.is_uppercase()) || c.is_ascii_digit() && !prev.is_some_and(|p| p.is_ascii_digit())) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// A region's name for people: grottos and interiors by where they are.
fn region_display(id: &str) -> String {
    match id {
        "VariousPlaces" => return "Here and there".into(),
        "Misc" => return "Start of the game".into(),
        _ => {}
    }
    if let Some(name) = crate::enums::items().attr(id, "ExitName").and_then(|a| a.get("Name").as_str()) {
        return name.to_string();
    }
    mmr_data::game().region(id).map(|r| r.name.clone()).unwrap_or_else(|| humanize(id))
}

/// Where checks the game data files under "various places" or "misc"
/// actually are.
fn place_of(location: &str) -> Option<&'static str> {
    Some(match location {
        "ItemBottleWitch" => "Woods of Mystery",
        "UpgradeBigBombBag" => "Bomb Shop or Curiosity Shop",
        "UpgradeBiggestBombBag" => "Goron Village business scrub",
        "HeartPieceKeatonQuiz" => "any Keaton bush",
        "HeartPiecePostBox" => "any postbox",
        "SongOath" => "after any temple boss",
        "ItemTingleMapTown" | "ItemTingleMapWoodfall" | "ItemTingleMapSnowhead" | "ItemTingleMapRanch" | "ItemTingleMapGreatBay" | "ItemTingleMapStoneTower" => "wherever Tingle is",
        "CollectibleStrayFairyClockTown" => "Laundry Pool or East Clock Town",
        _ => return None,
    })
}

/// Where a check is, for people: its region, or a better description.
pub fn place(location: &str) -> String {
    if let Some(p) = place_of(location) {
        return p.into();
    }
    let region = mmr_data::game().item(location).and_then(|i| i.region.clone()).unwrap_or_else(|| "Misc".into());
    region_display(&region)
}

fn item_name(id: &str) -> String {
    items::name(id).map(String::from).unwrap_or_else(|| humanize(id))
}

fn location_name(id: &str) -> String {
    mmr_data::game().item(id).and_then(|i| i.location.clone()).unwrap_or_else(|| humanize(id))
}

/// Dungeon exits: (exit item, the dungeon you leave, where you arrive).
const EXITS: [(&str, &str, &str); 4] = [
    ("AreaWoodFallTempleClear", "Woodfall Temple", "Southern Swamp (cleared)"),
    ("AreaSnowheadTempleClear", "Snowhead Temple", "Mountain Village (spring)"),
    ("AreaGreatBayTempleClear", "Great Bay Temple", "Great Bay (cleared)"),
    ("AreaStoneTowerClear", "Stone Tower Temple", "Ikana Canyon (cleared)"),
];

/// Where a shuffled entrance is taken from.
fn entrance_from(id: &str) -> String {
    if let Some((_, dungeon, _)) = EXITS.iter().find(|(e, _, _)| *e == id) {
        return format!("Leaving {dungeon} after its boss");
    }
    items::entrance_name(id).map(String::from).unwrap_or_else(|| humanize(id))
}

/// Where a shuffled entrance now leads.
fn entrance_to(id: &str) -> String {
    if let Some((_, _, place)) = EXITS.iter().find(|(e, _, _)| *e == id) {
        return place.to_string();
    }
    items::entrance_name(id).map(String::from).unwrap_or_else(|| humanize(id))
}

/// Standard base64 (with padding).
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i)) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A message's text: plain, and as HTML with its colours.
fn message_text(units: &[u16]) -> (String, String) {
    const COLORS: [&str; 9] = ["", "red", "green", "blue", "yellow", "cyan", "pink", "silver", "orange"];
    let mut plain = String::new();
    let mut html = String::new();
    let mut open = false;
    let mut i = 0;
    while i < units.len() {
        let c = units[i];
        match c {
            0x00..=0x08 => {
                if open {
                    html.push_str("</span>");
                    open = false;
                }
                if c != 0 {
                    let _ = write!(html, "<span class=\"t-{}\">", COLORS[c as usize]);
                    open = true;
                }
            }
            0x10 | 0x11 => {
                if !plain.ends_with(' ') {
                    plain.push(' ');
                    html.push(' ');
                }
            }
            0x1E => i += 2,
            0x1F => i += 2,
            0x19 | 0xBF => {}
            0x20..=0x7E => {
                plain.push(c as u8 as char);
                html.push_str(&escape(&(c as u8 as char).to_string()));
            }
            _ => {}
        }
        i += 1;
    }
    if open {
        html.push_str("</span>");
    }
    (plain.trim().to_string(), html.trim().to_string())
}

/// What a seed's spoiler needs besides the plan.
pub struct BuildInfo<'a> {
    pub hash: &'a [u8],
    pub icons: &'a [HashIcon],
    pub music: &'a [SongPlacement],
    /// Item icons (32x32 RGBA) by item id.
    pub item_icons: &'a BTreeMap<String, Vec<u8>>,
}

/// Gather everything about a seed.
pub fn build(outcome: &Outcome, plan: &Plan, values: &Values, logic: &LogicFile, info: BuildInfo) -> Spoiler {
    let game = mmr_data::game();
    let importance: Importance = mmr_randomizer::importance::importance(logic, values, outcome, &[]);
    let prepared = mmr_randomizer::prepare(logic, values, outcome.seed).ok();
    let mode = values.get("LogicMode").and_then(|v| v.as_str()).unwrap_or("Casual").to_string();
    let world_junk = prepared.as_ref().map(|p| p.world(game, values));
    let is_junk = |item: &str| world_junk.as_ref().is_some_and(|w| w.id(item).is_some_and(|i| w.is_junk(i)));

    // Checks: every shuffled location.
    let sphere_of: BTreeMap<&str, usize> = outcome.spheres.iter().enumerate().flat_map(|(n, s)| s.iter().map(move |p| (p.location.as_str(), n + 1))).collect();
    let mut checks: Vec<Check> = Vec::new();
    for e in outcome.layout.iter().filter(|e| e.randomized) {
        let placed = plan.items.iter().find(|p| p.location.as_deref() == Some(e.location.as_str()));
        let shown = placed.map(|p| p.name_override.clone().unwrap_or_else(|| item_name(&p.item))).unwrap_or_else(|| item_name(&e.given));
        let region = game.item(&e.location).and_then(|i| i.region.clone()).unwrap_or_else(|| "Misc".into());
        let region_name = place(&e.location);
        checks.push(Check {
            id: e.location.clone(),
            name: location_name(&e.location),
            region,
            region_name,
            item: e.item.clone(),
            item_name: shown,
            category: game.item(&e.location).and_then(|i| i.location_category.clone()).unwrap_or_default(),
            junk: is_junk(&e.given) || e.given == "RecoveryHeart",
            required: importance.required.contains(&e.location),
            important: importance.important.contains(&e.location),
            sphere: sphere_of.get(e.location.as_str()).copied(),
            icon: placed.map(|p| p.mimic.as_ref().map_or_else(|| p.item.clone(), |m| m.item.clone())).unwrap_or_else(|| e.given.clone()),
            times: prepared.as_ref().and_then(|p| p.logic.entry(&e.location)).map_or(0, |x| if x.time.available.0 & 0x3F == 0x3F { 0 } else { x.time.available.0 & 0x3F }),
        });
    }
    let index_of: BTreeMap<String, usize> = checks.iter().enumerate().map(|(i, c)| (c.id.clone(), i)).collect();

    // Areas and regions, in the game data's order.
    let mut areas: Vec<Area> = Vec::new();
    for r in &game.regions {
        let in_region: Vec<usize> = checks.iter().enumerate().filter(|(_, c)| c.region == r.id).map(|(i, _)| i).collect();
        if in_region.is_empty() {
            continue;
        }
        let area = match r.area.as_deref() {
            Some("Town") => "Clock Town",
            Some("Swamp") => "Southern Swamp",
            Some("Mountain") => "Snowhead",
            Some("Ocean") => "Great Bay",
            Some("Canyon") => "Ikana",
            Some("Ranch") => "Romani Ranch",
            _ => "Elsewhere",
        };
        let row = RegionRow { id: r.id.clone(), name: region_display(&r.id), checks: in_region };
        match areas.iter_mut().find(|a| a.name == area) {
            Some(a) => a.regions.push(row),
            None => areas.push(Area { name: area.into(), regions: vec![row] }),
        }
    }
    let area_order = ["Clock Town", "Romani Ranch", "Southern Swamp", "Snowhead", "Great Bay", "Ikana", "Elsewhere"];
    areas.sort_by_key(|a| area_order.iter().position(|n| *n == a.name));

    let spheres: Vec<Vec<usize>> = outcome.spheres.iter().map(|s| s.iter().filter_map(|p| index_of.get(&p.location).copied()).collect()).collect();

    // Starting items.
    let mut starting: Vec<Named> = Vec::new();
    let mut add_start = |id: &str, source: &str| {
        if !starting.iter().any(|s| s.id == id) {
            starting.push(Named { id: id.into(), name: item_name(id), source: source.into() });
        }
    };
    for id in &plan.custom_starting_item_list {
        add_start(id, "Starting item");
    }
    for id in &outcome.blitz_items {
        add_start(id, "Blitz");
    }
    for id in &outcome.random_starting {
        add_start(id, "Random pick");
    }
    for slot in STARTING_LOCATIONS {
        if let Some(e) = outcome.layout.iter().find(|e| e.location == slot && e.given != "RecoveryHeart" && !is_junk(&e.given)) {
            add_start(&e.given, "Starting slot");
        }
    }

    let region_of = |id: &str| game.item(id).and_then(|i| i.region.as_deref()).and_then(|r| game.region(r)).map(|r| r.name.clone()).unwrap_or_default();
    let kind_of = |id: &str| {
        let kind = mmr_randomizer::entrances::entrance_items().iter().find(|e| e.id == id).map(|e| e.kind.as_str()).unwrap_or("");
        match kind {
            "Dungeon" | "DungeonExit" => "Dungeons",
            "Boss" => "Boss rooms",
            "Grotto" => "Grottos",
            _ => "Interiors",
        }
        .to_string()
    };
    let mut entrances: Vec<EntranceRow> = outcome
        .entrances
        .iter()
        .filter(|(e, t)| e != t)
        .map(|(e, t)| {
            let kind = kind_of(t);
            let exit_name = |id: &str| crate::enums::items().attr(id, "ExitName").and_then(|a| a.get("Name").as_str()).map(String::from);
            // Grottos look alike: name the holes by where they are.
            let (entrance, leads_to) = if kind == "Grottos" {
                // Unique grottos by their name; plain ones by their hole.
                let name = entrance_to(e);
                let to = if name == "Generic Grotto" { exit_name(e).map(|at| format!("Generic Grotto ({})", at.trim_end_matches(" Grotto"))).unwrap_or(name) } else { name };
                (exit_name(t).unwrap_or_else(|| entrance_from(t)), to)
            } else {
                (entrance_from(t), entrance_to(e))
            };
            EntranceRow { kind, entrance, region: region_of(t), leads_to }
        })
        .collect();
    let kinds = ["Dungeons", "Boss rooms", "Grottos", "Interiors"];
    entrances.sort_by(|a, b| (kinds.iter().position(|k| *k == a.kind), &a.region, &a.entrance).cmp(&(kinds.iter().position(|k| *k == b.kind), &b.region, &b.entrance)));

    let quote_names: BTreeMap<u16, String> = crate::enums::get("GossipQuote").values.iter().map(|v| (v.value as u16, v.name.clone())).collect();
    let mut hints: Vec<HintRow> = plan
        .gossip_quotes
        .iter()
        .flatten()
        .map(|q| {
            let name = quote_names.get(&q.id).cloned().unwrap_or_default();
            let (kind, stone) = if let Some(s) = name.strip_prefix("Garo") {
                ("Garo", humanize(s))
            } else if let Some(s) = name.strip_prefix("Moon") {
                // Moon children: one per mask (said again after clearing).
                ("Moon", item_name(s.strip_suffix("Clear").unwrap_or(s)))
            } else {
                ("Gossip stone", humanize(&name))
            };
            let (text, html) = message_text(&q.message);
            let info = plan.hint_info.iter().find(|h| h.id == q.id);
            HintRow {
                kind: kind.into(),
                stone,
                text,
                html,
                verdict: info.map(|i| i.kind.clone()).unwrap_or_default(),
                region: info.and_then(|i| i.region.clone()),
                locations: info.map(|i| i.locations.clone()).unwrap_or_default(),
            }
        })
        .collect();
    hints.sort_by(|a, b| (a.kind.clone(), a.stone.clone()).cmp(&(b.kind.clone(), b.stone.clone())));
    hints.dedup_by(|a, b| a.kind == "Moon" && a.kind == b.kind && a.stone == b.stone);

    let prices = plan
        .message_costs
        .iter()
        .flatten()
        .zip(mmr_randomizer::prices::message_costs())
        .filter_map(|(p, c)| {
            let name = c.name.clone().or_else(|| c.locations_affected.first().map(|l| location_name(l))).or_else(|| c.items_affected.first().map(|i| item_name(i))).unwrap_or_else(|| "Other price".into());
            p.map(|v| PriceRow { name, value: v as u32 })
        })
        .collect();
    let gibdos = if values.get("RandomizeGibdoRequirements").and_then(|v| v.as_bool()) == Some(true) {
        plan.gibdo_requirements.iter().map(|g| PriceRow { name: humanize(&g.item_required), value: g.amount as u32 }).collect()
    } else {
        Vec::new()
    };

    let mut highlights = Vec::new();
    for (key, label) in [("GossipHintStyle", "Gossip hints"), ("EntranceMode", "Entrances"), ("PriceMode", "Prices"), ("TrapAmount", "Traps"), ("VictoryMode", "Goal")] {
        if let Some(v) = values.get(key) {
            let shown = match v {
                serde_json::Value::Array(a) if a.is_empty() && key == "VictoryMode" => "Defeat Majora".to_string(),
                serde_json::Value::Array(a) if a.is_empty() => "Off".to_string(),
                serde_json::Value::Array(a) => a.iter().filter_map(|x| x.as_str()).map(humanize).collect::<Vec<_>>().join(", "),
                serde_json::Value::String(s) => humanize(s),
                other => other.to_string(),
            };
            highlights.push((label.to_string(), shown));
        }
    }

    // Only icons the page can show.
    let used_icons: std::collections::BTreeSet<String> = checks.iter().map(|c| c.icon.clone()).chain(starting.iter().map(|s| s.id.clone())).collect();
    let logic_data = prepared.as_ref().zip(world_junk.as_ref()).map(|(p, w)| tracker_logic(&p.logic, w, values, outcome, &checks, &starting)).unwrap_or_default();

    Spoiler {
        version: format!("MMR Next {}", env!("CARGO_PKG_VERSION")),
        seed: outcome.seed.to_string(),
        settings_string: mmr_settings::encode_settings_string(values, false),
        hash: info.hash.iter().map(|b| format!("{b:02X}")).collect(),
        hash_icons: info.icons.iter().map(|i| Icon { name: i.name.to_string(), png: format!("data:image/png;base64,{}", base64(&crate::hash_icons::icon_png(i))) }).collect(),
        logic_mode: humanize(&mode),
        placement: format!("{:?}", outcome.style),
        highlights,
        starting,
        areas,
        checks,
        spheres,
        entrances,
        hints,
        prices,
        gibdos,
        music: info.music.to_vec(),
        logic: logic_data,
        icons: info
            .item_icons
            .iter()
            .filter(|(k, _)| used_icons.contains(k.as_str()))
            .map(|(k, rgba)| (k.clone(), format!("data:image/png;base64,{}", base64(&crate::hash_icons::rgba_png(rgba)))))
            .collect(),
    }
}

fn tracker_logic(logic: &LogicFile, world: &mmr_randomizer::world::World, values: &Values, outcome: &Outcome, checks: &[Check], starting: &[Named]) -> TrackerLogic {
    const ALL: u8 = 0b11_1111;
    let mut t = TrackerLogic::default();
    let mut index: BTreeMap<String, u32> = BTreeMap::new();
    let mut id = |t: &mut TrackerLogic, name: &str| -> u32 {
        if let Some(&i) = index.get(name) {
            return i;
        }
        let i = t.names.len() as u32;
        t.names.push(name.to_string());
        // Item refs the world doesn't know act like entries (or are free).
        let item = mmr_logic::is_item_ref(name).then(|| world.id(name)).flatten();
        t.is_item.push(u8::from(item.is_some()));
        t.item_flags.push(item.map_or(0, |i| {
            let it = &world.items[i];
            u8::from(!it.temporary && !it.entrance && it.name.is_some()) | u8::from(it.time_travel) << 1
        }));
        t.needed.push(ALL);
        t.item_absent.push(0);
        index.insert(name.to_string(), i);
        i
    };
    let targets: Vec<&str> = {
        let set: std::collections::BTreeSet<&str> = logic.entries.iter().flat_map(|e| e.before.iter().map(String::as_str)).collect();
        set.into_iter().collect()
    };
    let layer_of = |x: &str| targets.iter().position(|y| *y == x).map_or(0, |p| p as u8 + 1);
    t.layers = targets.len() as u32 + 1;
    let never = id(&mut t, mmr_logic::settings_rules::IMPOSSIBLE);
    t.is_item[never as usize] = 2;
    let tricks: Vec<&str> = values.get("EnabledTricks").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
    for e in &logic.entries {
        let gate = match mmr_logic::EntryKind::of(e) {
            mmr_logic::EntryKind::Trick => tricks.contains(&e.id.as_str()),
            mmr_logic::EntryKind::Setting => e.setting.as_ref().is_some_and(|c| c.eval(values)),
            _ => true,
        };
        let me = id(&mut t, &e.id);
        let available = if e.time.available.is_empty() { ALL } else { e.time.available.0 & ALL };
        t.avail.push(available);
        t.window.push(if e.time.setup.is_empty() { available } else { e.time.setup.0 & ALL });
        t.flags.push(u8::from(e.time.no_return) | u8::from(e.time.opening_hours) << 1);
        t.read_layer.push(e.before.first().map_or(0, |b| layer_of(b)));
        t.absent.push(layer_of(&e.id));
        if t.is_item[me as usize] == 1 {
            if !e.time.needed.is_empty() {
                t.needed[me as usize] = e.time.needed.0 & ALL;
            }
            t.item_absent[me as usize] = layer_of(&e.id);
        }
        if e.id == "OtherTimeTravel" {
            t.time_travel = Some(me);
        }
        if !gate {
            t.entries.push((me, vec![never], Vec::new()));
            t.tmp.push(Vec::new());
            continue;
        }
        let requires: Vec<u32> = e.requires.iter().map(|r| id(&mut t, r)).collect();
        let any_of: Vec<Vec<u32>> = e.any_of.iter().map(|g| g.iter().map(|r| id(&mut t, r)).collect()).collect();
        let tmp: Vec<u32> = e.temporary_requires.iter().map(|r| id(&mut t, r)).collect();
        t.entries.push((me, requires, any_of));
        t.tmp.push(tmp);
    }
    for e in outcome.layout.iter().filter(|e| !e.randomized) {
        let (l, i) = (id(&mut t, &e.location), id(&mut t, &e.item));
        t.vanilla.push((l, i));
    }
    for s in starting {
        let i = id(&mut t, &s.id);
        t.start.push(i);
    }
    for c in checks {
        let (l, i) = (id(&mut t, &c.id), id(&mut t, &c.item));
        t.checks.push((l, i));
    }
    t
}

/// The plain-text spoiler log.
pub fn text(s: &Spoiler) -> String {
    let mut out = String::new();
    let rule = |out: &mut String, title: &str| {
        let _ = writeln!(out, "\n{title}\n{}", "─".repeat(title.chars().count()));
    };
    let _ = writeln!(out, "Majora's Mask Randomizer Next · {}", s.version);
    let _ = writeln!(out, "{}", "═".repeat(72));
    let _ = writeln!(out, "{:<14}{}", "Seed", s.seed);
    if !s.hash_icons.is_empty() {
        let _ = writeln!(out, "{:<14}{}", "Hash", s.hash_icons.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(" · "));
    }
    let _ = writeln!(out, "{:<14}{} · {} placement", "Logic", s.logic_mode, s.placement);
    for (k, v) in &s.highlights {
        let _ = writeln!(out, "{:<14}{}", k, v);
    }
    let _ = writeln!(out, "{:<14}{}", "Settings", s.settings_string);

    if !s.starting.is_empty() {
        rule(&mut out, "Starting items");
        for i in &s.starting {
            let _ = writeln!(out, "  {:<36} {}", i.name, i.source);
        }
    }

    let required: Vec<&Check> = s.spheres.iter().flatten().map(|&i| &s.checks[i]).filter(|c| c.required).collect();
    if !required.is_empty() {
        rule(&mut out, "Way of the Hero (needed to win)");
        for c in required {
            let _ = writeln!(out, "  {:<36} {} ({})", c.item_name, c.name, c.region_name);
        }
    }

    if !s.spheres.is_empty() {
        rule(&mut out, "Playthrough (each step needs only what came before)");
        for (n, sphere) in s.spheres.iter().enumerate() {
            let _ = writeln!(out, "  Step {}", n + 1);
            for &i in sphere {
                let c = &s.checks[i];
                let mark = if c.required { "★" } else { " " };
                let _ = writeln!(out, "   {mark} {:<34} {} ({})", c.item_name, c.name, c.region_name);
            }
        }
    }

    if !s.entrances.is_empty() {
        rule(&mut out, "Entrances");
        let mut kind = "";
        for e in &s.entrances {
            if e.kind != kind {
                kind = &e.kind;
                let _ = writeln!(out, "  {kind}");
            }
            let from = if e.region.is_empty() || e.entrance.contains(&e.region) { e.entrance.clone() } else { format!("{} ({})", e.entrance, e.region) };
            let _ = writeln!(out, "    {from:<52} → {}", e.leads_to);
        }
    }

    if !s.hints.is_empty() {
        rule(&mut out, "Hints");
        // What the gossip stones say, in short.
        let mut said = |verdict: &str, label: &str| {
            let mut regions: Vec<&str> = Vec::new();
            for h in s.hints.iter().filter(|h| h.verdict == verdict) {
                if let Some(r) = h.region.as_deref().filter(|r| !regions.contains(r)) {
                    regions.push(r);
                }
            }
            if !regions.is_empty() {
                let _ = writeln!(out, "  {:<18}{}", label, regions.iter().map(|r| region_display(r)).collect::<Vec<_>>().join(", "));
            }
        };
        said("woth", "Way of the Hero");
        said("foolish", "Foolish");
        said("song", "Foolish but song");
        let mut named: Vec<String> = Vec::new();
        for h in s.hints.iter().filter(|h| h.kind == "Gossip stone" && h.verdict == "item") {
            for l in &h.locations {
                if let Some(c) = s.checks.iter().find(|c| &c.id == l) {
                    let line = format!("{} → {}", c.name, c.item_name);
                    if !named.contains(&line) {
                        named.push(line);
                    }
                }
            }
        }
        if !named.is_empty() {
            let _ = writeln!(out, "  {:<18}{}", "Named checks", named.join("\n                    "));
        }
        for kind in ["Gossip stone", "Garo", "Moon"] {
            let rows: Vec<&HintRow> = s.hints.iter().filter(|h| h.kind == kind).collect();
            if rows.is_empty() {
                continue;
            }
            let title = match kind {
                "Gossip stone" => "Gossip stones",
                "Garo" => "Garos",
                _ => "Moon children",
            };
            let _ = writeln!(out, "\n  {title}");
            for h in rows {
                let _ = writeln!(out, "    {:<26}{}", h.stone, h.text);
            }
        }
    }

    rule(&mut out, "Checks by area (★ needed to win, · helps)");
    for a in &s.areas {
        let _ = writeln!(out, "\n  {}", a.name.to_uppercase());
        for r in &a.regions {
            let _ = writeln!(out, "  {}", r.name);
            for &i in &r.checks {
                let c = &s.checks[i];
                let mark = if c.required {
                    "★"
                } else if c.important {
                    "·"
                } else {
                    " "
                };
                let _ = writeln!(out, "   {mark} {:<44} {}", c.name, c.item_name);
            }
        }
    }

    rule(&mut out, "Where each item is");
    let mut by_item: Vec<&Check> = s.checks.iter().filter(|c| !c.junk).collect();
    by_item.sort_by(|a, b| a.item_name.cmp(&b.item_name).then(a.name.cmp(&b.name)));
    for c in by_item {
        let _ = writeln!(out, "  {:<36} {} ({})", c.item_name, c.name, c.region_name);
    }

    if !s.prices.is_empty() {
        rule(&mut out, "Prices");
        for p in &s.prices {
            let _ = writeln!(out, "  {:<40} {:>4} rupees", p.name, p.value);
        }
    }
    if !s.gibdos.is_empty() {
        rule(&mut out, "Gibdo requests (Ikana Castle well)");
        for g in &s.gibdos {
            let _ = writeln!(out, "  {:<30} × {}", g.name, g.value);
        }
    }
    if !s.music.is_empty() {
        rule(&mut out, "Music");
        for m in &s.music {
            let _ = writeln!(out, "  {:<36} {}", m.slot.trim_start_matches("mm-"), m.song);
        }
    }
    out
}

/// The interactive, spoiler-free tracker: one self-contained HTML file.
pub fn html(s: &Spoiler) -> String {
    let data = serde_json::to_string(s).expect("spoiler data").replace("</", "<\\/");
    include_str!("tracker.html").replace("/*SPOILER_DATA*/null", &data).replace("{{TITLE}}", &escape(&format!("MMR seed {}", s.seed)))
}
