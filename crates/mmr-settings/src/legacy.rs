//! Import settings from MMR 2.0: `settings.json` files and the compact
//! item-list strings they contain.

use crate::schema::{schema, ItemPool, SettingKind};
use crate::{with_defaults, Values};
use serde_json::{Map, Value};

/// Legacy keys that hold item lists as bit strings, with their new keys.
const ITEM_LIST_KEYS: &[(&str, &str)] = &[
    ("CustomItemListString", "CustomItemList"),
    ("CustomStartingItemListString", "CustomStartingItemList"),
    ("CustomJunkLocationsString", "CustomJunkLocations"),
];

#[derive(Debug, thiserror::Error)]
pub enum LegacyError {
    #[error("not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("this doesn't look like an MMR settings file")]
    NotSettings,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct Imported {
    pub values: Values,
    /// Legacy settings that have no equivalent and were skipped.
    pub skipped: Vec<String>,
}

/// Import an MMR 2.0 `settings.json` (a `Configuration` object).
pub fn import_settings_file(json: &str) -> Result<Imported, LegacyError> {
    let root: Value = serde_json::from_str(json.trim_start_matches('\u{feff}'))?;
    let root = root.as_object().ok_or(LegacyError::NotSettings)?;
    let gameplay = get_ci(root, "GameplaySettings").and_then(Value::as_object);
    let cosmetic = get_ci(root, "CosmeticSettings").and_then(Value::as_object);
    if gameplay.is_none() && cosmetic.is_none() {
        return Err(LegacyError::NotSettings);
    }

    let mut flat = Map::new();
    if let Some(g) = gameplay {
        flatten("", g, &mut flat);
    }
    if let Some(c) = cosmetic {
        flatten("Cosmetic.", c, &mut flat);
    }
    Ok(import_flat(&flat))
}

/// Import already-flattened legacy key/value pairs (as used by presets).
pub fn import_flat(flat: &Map<String, Value>) -> Imported {
    let schema = schema();
    let mut partial = Map::new();
    let mut skipped = Vec::new();
    for (key, value) in flat {
        if let Some((_, new_key)) = ITEM_LIST_KEYS.iter().find(|(old, _)| old.eq_ignore_ascii_case(key)) {
            let pool = schema.get(new_key).and_then(|d| d.pool).unwrap_or(ItemPool::Locations);
            match value.as_str().map(|s| decode_item_list(pool, s)) {
                Some(Some(list)) => {
                    partial.insert(new_key.to_string(), list.into());
                }
                _ => skipped.push(key.clone()),
            }
            continue;
        }
        let Some(def) = schema.settings.iter().find(|d| d.key.eq_ignore_ascii_case(key)) else {
            skipped.push(key.clone());
            continue;
        };
        let value = match (def.kind, value) {
            (SettingKind::Flags, Value::String(s)) => {
                Value::Array(s.split(',').map(str::trim).filter(|f| !f.is_empty() && *f != "None").map(|f| Value::String(f.to_string())).collect())
            }
            (SettingKind::Flags, Value::Number(_)) => {
                skipped.push(key.clone());
                continue;
            }
            (SettingKind::Custom, v) if def.editor.is_some() => match crate::structured::normalize(def, v) {
                Some(v) => v,
                None => {
                    skipped.push(key.clone());
                    continue;
                }
            },
            _ => value.clone(),
        };
        partial.insert(def.key.clone(), value);
    }
    Imported { values: with_defaults(&partial), skipped }
}

fn flatten(prefix: &str, obj: &Map<String, Value>, out: &mut Map<String, Value>) {
    for (k, v) in obj {
        match v {
            // Nested settings groups (e.g. ShortenCutsceneSettings) flatten to
            // dotted keys; other objects are structured values.
            Value::Object(inner) if k.eq_ignore_ascii_case("ShortenCutsceneSettings") => flatten(&format!("{prefix}{k}."), inner, out),
            _ => {
                out.insert(format!("{prefix}{k}"), v.clone());
            }
        }
    }
}

fn get_ci<'a>(obj: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    obj.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
}

fn pool_items(pool: ItemPool) -> Vec<&'static str> {
    let game = mmr_data::game();
    match pool {
        ItemPool::Locations => game.locations().map(|i| i.id.as_str()).collect(),
        ItemPool::StartingItems => game.starting_items().map(|i| i.id.as_str()).collect(),
    }
}

/// Decode a legacy item-list string (`ItemUtils.ConvertStringToItemList`).
/// Returns `None` if the string doesn't match this version's item list.
pub fn decode_item_list(pool: ItemPool, s: &str) -> Option<Vec<String>> {
    let base = pool_items(pool);
    if s.trim().is_empty() {
        return Some(Vec::new());
    }
    let sections = base.len().div_ceil(32);
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != sections {
        return None;
    }
    let mut out = Vec::new();
    for (word_index, part) in parts.iter().rev().enumerate() {
        let word = if part.is_empty() { 0 } else { u32::from_str_radix(part, 16).ok()? };
        for bit in 0..32 {
            if word >> bit & 1 == 1 {
                out.push(base.get(word_index * 32 + bit)?.to_string());
            }
        }
    }
    Some(out)
}

/// Encode an item list in the legacy format (`ItemUtils.ConvertItemListToString`).
pub fn encode_item_list(pool: ItemPool, items: &[String]) -> String {
    let base = pool_items(pool);
    let mut words = vec![0u32; base.len().div_ceil(32)];
    for item in items {
        if let Some(i) = base.iter().position(|b| b == item) {
            words[i / 32] |= 1 << (i % 32);
        }
    }
    words.iter().rev().map(|w| if *w == 0 { String::new() } else { format!("{w:x}") }).collect::<Vec<_>>().join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_list_round_trip() {
        // Default CustomItemListString from the legacy GameplaySettings.
        let legacy = "-------------------------40c-80000000----21ffff-ffffffff-ffffffff-f0000000-7bbeeffa-7fffffff-e6f1fffe-ffffffff";
        let items = decode_item_list(ItemPool::Locations, legacy).unwrap();
        assert!(items.contains(&"ItemBow".to_string()));
        assert_eq!(encode_item_list(ItemPool::Locations, &items), legacy);
        assert!(decode_item_list(ItemPool::Locations, "1-2-3").is_none());
    }

    #[test]
    fn imports_a_settings_file() {
        let json = r#"{
            "GameplaySettings": {
                "LogicMode": "Glitched",
                "EnemyMode": "Randomized, Hyper",
                "CustomStartingItemListString": "----1fbfc-5800000-",
                "ShortenCutsceneSettings": { "General": "BoatArchery", "BossIntros": "Odolwa" },
                "SomethingRemoved": 3
            },
            "CosmeticSettings": { "Music": "Random" },
            "OutputSettings": {}
        }"#;
        let imported = import_settings_file(json).unwrap();
        let v = &imported.values;
        assert_eq!(v["LogicMode"], "Glitched");
        assert_eq!(v["EnemyMode"], serde_json::json!(["Randomized", "Hyper"]));
        assert_eq!(v["ShortenCutsceneSettings.General"], serde_json::json!(["BoatArchery"]));
        assert_eq!(v["Cosmetic.Music"], "Random");
        assert!(v["CustomStartingItemList"].as_array().unwrap().len() > 5);
        assert_eq!(imported.skipped, vec!["SomethingRemoved"]);
        assert!(crate::validate(v).is_empty(), "{:?}", crate::validate(v));
    }

    #[test]
    fn imports_structured_cosmetics() {
        // Shapes as MMR 2.0 writes them (JsonColorConverter: "R, G, B").
        let json = r#"{
            "GameplaySettings": {
                "TrapWeights": { "Ice": 2, "Bomb": 1 },
                "RandomStartingItemGroups": [{ "Items": ["ItemBow", "ItemHookshot"], "Amount": 1 }],
                "OverrideHintPriorities": [["HeartPieceNotebookMayor"], ["MaskBlast", "MaskAllNight"]],
                "OverrideImportanceIndicatorTiers": [1],
                "OverrideHintItemCaps": [0, 1]
            },
            "CosmeticSettings": {
                "HeartsSelection": "Random Choice",
                "UseTunicColors": { "Human": true },
                "TunicColors": { "Human": "255, 0, 128", "Deku": "30, 105, 27" },
                "EnergyColors": { "Goron": ["1, 2, 3", "4, 5, 6", "7, 8, 9"] },
                "Instruments": { "Human": "Harp", "Deku": "DekuPipes", "Goron": "Drums", "Zora": "Guitar" },
                "DPad": { "Pad": { "Values": ["None", "ZoraMask", "GoronMask", "DekuMask"] }, "State": "Enabled", "Display": "Right" },
                "HudColors": { "ButtonA": "1, 2, 3" },
                "EnergyColorsBroken": 3
            }
        }"#;
        let imported = import_settings_file(json).unwrap();
        let v = &imported.values;
        assert!(crate::validate(v).is_empty(), "{:?}", crate::validate(v));
        assert_eq!(v["Cosmetic.TunicColors"]["Human"], "255, 0, 128");
        assert_eq!(v["Cosmetic.TunicColors"]["FierceDeity"], "189, 181, 173", "missing forms keep defaults");
        assert_eq!(v["Cosmetic.UseTunicColors"]["Human"], true);
        assert_eq!(v["Cosmetic.UseTunicColors"]["Zora"], false);
        assert_eq!(v["Cosmetic.EnergyColors"]["Goron"][2], "7, 8, 9");
        assert_eq!(v["Cosmetic.Instruments"]["Human"], "Harp");
        assert_eq!(v["Cosmetic.DPad"]["Display"], "Right");
        assert_eq!(v["Cosmetic.HudColors"]["ButtonA"], "1, 2, 3");
        assert_eq!(v["Cosmetic.HudColors"]["ButtonB"], "100, 255, 120", "other HUD colours keep defaults");
        assert_eq!(v["Cosmetic.HeartsSelection"], "Random Choice");
        assert_eq!(v["TrapWeights"]["Ice"], 2);
        assert_eq!(v["RandomStartingItemGroups"][0]["Amount"], 1);
        assert_eq!(v["OverrideHintItemCaps"], serde_json::json!([0, 1]));
        assert_eq!(imported.skipped, vec!["Cosmetic.EnergyColorsBroken"]);
    }
}
