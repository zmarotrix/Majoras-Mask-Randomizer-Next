//! What the Gibdos under Ikana Castle ask for (the old
//! `RandomizeGibdoRequirements`), and the logic their new requests need.

use crate::rng::Rng;
use mmr_logic::{Entry, LogicFile};
use mmr_settings::Values;
use serde::Deserialize;
use serde_json::Value;
use std::sync::OnceLock;

/// A Gibdo's request: an amount of an item, and the logic entry it guards.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GibdoRequest {
    pub item: String,
    pub amount: u8,
    pub logic_entry: Option<String>,
}

/// The Gibdos, in the game's order, with what they ask for normally.
const GIBDOS: [(&str, Option<&str>); 16] = [
    ("BluePotion", Some("GibdoEntranceLeft")),
    ("MagicBeans", Some("GibdoEntranceRight")),
    ("SpringWater", Some("GibdoToBombPots")),
    ("Fish", Some("GibdoToHotWater")),
    ("Bugs", Some("GibdoToFairyFountain")),
    ("DekuNuts", Some("GibdoToCowAndMiniboss")),
    ("Bombs", Some("GibdoToMiniBoss")),
    ("HotSpringWater", Some("GibdoToCow")),
    ("BigPoe", Some("GibdoToFinalWallmaster")),
    ("Milk", Some("GibdoToMirrorShield")),
    ("Bugs", Some("GibdoToLeftChest")),
    ("Bugs", Some("GibdoToRightChest")),
    ("Fish", Some("GibdoToBlackBoes")),
    ("Bugs", None),
    ("Bugs", None),
    ("Bugs", None),
];

#[derive(Debug, Deserialize)]
struct EnumValue {
    name: String,
    attributes: Vec<Attr>,
}

#[derive(Debug, Deserialize)]
struct Attr {
    #[serde(rename = "type")]
    kind: String,
    values: serde_json::Map<String, Value>,
}

fn items() -> &'static [EnumValue] {
    static TABLE: OnceLock<Vec<EnumValue>> = OnceLock::new();
    TABLE.get_or_init(|| serde_json::from_str(include_str!("../../../data/rom/enums/GibdoRequirementItem.json")).expect("bundled Gibdo items"))
}

fn num(v: Option<&Value>) -> i64 {
    v.and_then(Value::as_i64).unwrap_or(0)
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default()
}

/// (default, min, max) amounts an item can be asked for in.
fn amounts(item: &EnumValue) -> Option<(i64, i64, i64)> {
    item.attributes.iter().find(|a| a.kind == "ItemGibdoAmount").map(|a| (num(a.values.get("DefaultAmount")), num(a.values.get("Min")), num(a.values.get("Max"))))
}

fn flag(values: &Values, key: &str) -> bool {
    values.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// The Gibdos' requests. Randomized ones change `logic` (unless there's
/// no logic) so their doors need what they ask for.
pub fn randomize(logic: &mut LogicFile, values: &Values, rng: &mut Rng) -> Vec<GibdoRequest> {
    let all = items();
    let get = |name: &str| all.iter().find(|i| i.name == name).expect("known Gibdo item");
    if !flag(values, "RandomizeGibdoRequirements") {
        return GIBDOS
            .iter()
            .map(|&(item, entry)| GibdoRequest { item: item.into(), amount: amounts(get(item)).map_or(1, |a| a.0) as u8, logic_entry: entry.map(String::from) })
            .collect();
    }

    let mut candidates: Vec<&EnumValue> = all.iter().filter(|i| i.name != "None").filter(|i| i.name != "Bombchu" || flag(values, "BombchuDrops")).collect();
    let mut out = Vec::new();
    for &(_, entry) in &GIBDOS {
        let i = rng.below(candidates.len());
        let chosen = candidates.remove(i);
        let amount = match amounts(chosen) {
            Some((_, min, max)) => min + rng.below((max - min + 1) as usize) as i64,
            None => 1,
        };
        out.push(GibdoRequest { item: chosen.name.clone(), amount: amount as u8, logic_entry: entry.map(String::from) });
    }

    if values.get("LogicMode").and_then(Value::as_str) != Some("NoLogic") {
        for (n, r) in out.iter().enumerate() {
            let Some(entry) = &r.logic_entry else { continue };
            let item = get(&r.item);
            let in_range = |a: &Attr| {
                let (min, max) = (num(a.values.get("MinAmount")), num(a.values.get("MaxAmount")));
                (min == 0 || min <= r.amount as i64) && (max == 0 || max >= r.amount as i64)
            };
            let mut requires: Vec<String> = Vec::new();
            // Some requests need what another entry needs (a bottle of ...).
            if let Some(reference) = item.attributes.iter().find(|a| a.kind == "ItemGibdoLogicReference").and_then(|a| a.values.get("Reference")).and_then(Value::as_str) {
                if let Some(original) = logic.entry(reference).cloned() {
                    let id = format!("MMRN Gibdo {} request", n + 1);
                    let mut clone = Entry::new(id.clone());
                    clone.requires = original.requires;
                    clone.any_of = original.any_of;
                    logic.entries.push(clone);
                    requires.push(id);
                }
            }
            requires.extend(item.attributes.iter().filter(|a| a.kind == "ItemGibdoLogicRequirements" && in_range(a)).flat_map(|a| strings(a.values.get("Items"))));
            let any_of: Vec<Vec<String>> = item.attributes.iter().filter(|a| a.kind == "ItemGibdoLogicConditional" && in_range(a)).map(|a| strings(a.values.get("Items"))).collect();
            if let Some(e) = logic.entries.iter_mut().find(|e| e.id == *entry) {
                e.requires = requires;
                e.any_of = any_of;
            }
        }
    }
    out
}
