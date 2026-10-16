//! Settings strings: a short, copy-pasteable form of a full settings set.
//!
//! Format: `MMRN.` + base64url(deflate(JSON of non-default values)).
//! Only values that differ from defaults are stored, so strings stay short
//! and keep working when new settings (with defaults) are added.
//!
//! Item lists are stored as bitsets over the pool in game-data order: the
//! JSON holds `"#<pool size>:<offset>"` and the bitset bytes follow the JSON
//! after a NUL byte, so the whole payload compresses together. New items are
//! only ever appended to the pool, so older strings keep decoding.
//!
//! Enabled tricks work the same way over `data/tricks.json` (append-only):
//! `["#<list size>:<offset>", <tricks not in the list>...]`. Strings made
//! before this stored plain arrays of names, which still decode.
//!
//! Lists of locations in other settings (hint priorities) store each known
//! location as its index in the game data instead of its name.

use crate::schema::{schema, ItemPool, SettingKind};
use crate::{changed, with_defaults, Values};
use base64::Engine;
use serde_json::Value;

const PREFIX: &str = "MMRN.";
const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

#[derive(Debug, thiserror::Error)]
pub enum SettingsStringError {
    #[error("this isn't an MMR Next settings string (it should start with \"{PREFIX}\")")]
    Prefix,
    #[error("the settings string is damaged or incomplete")]
    Corrupt,
}

/// Encode settings. Cosmetic settings are left out unless `include_cosmetics`.
pub fn encode_settings_string(values: &Values, include_cosmetics: bool) -> String {
    let mut diff = changed(values);
    if !include_cosmetics {
        diff.retain(|k, _| !crate::schema::is_cosmetic(k));
    }
    let mut tail = Vec::new();
    for (key, value) in diff.iter_mut() {
        if let (Some(pool), Some(list)) = (item_pool(key), value.as_array()) {
            *value = Value::String(pack(pool, list, &mut tail));
        } else if key == TRICKS {
            if let Some(list) = value.as_array() {
                *value = pack_tricks(list, &mut tail);
            }
        } else if ID_LISTS.contains(&key.as_str()) {
            ids_to_indexes(value);
        }
    }
    let mut payload = serde_json::to_vec(&diff).expect("settings serialize");
    if !tail.is_empty() {
        payload.push(0);
        payload.extend(tail);
    }
    let packed = miniz_oxide::deflate::compress_to_vec(&payload, 10);
    format!("{PREFIX}{}", B64.encode(packed))
}

/// Decode a settings string into a full set of values.
pub fn decode_settings_string(s: &str) -> Result<Values, SettingsStringError> {
    let body = s.trim().strip_prefix(PREFIX).ok_or(SettingsStringError::Prefix)?;
    let packed = B64.decode(body).map_err(|_| SettingsStringError::Corrupt)?;
    let payload = miniz_oxide::inflate::decompress_to_vec_with_limit(&packed, 1 << 22).map_err(|_| SettingsStringError::Corrupt)?;
    let (json, tail) = match payload.iter().position(|b| *b == 0) {
        Some(i) => (&payload[..i], &payload[i + 1..]),
        None => (&payload[..], &[][..]),
    };
    let mut partial: Values = serde_json::from_slice(json).map_err(|_| SettingsStringError::Corrupt)?;
    for (key, value) in partial.iter_mut() {
        if let (Some(pool), Some(packed)) = (item_pool(key), value.as_str()) {
            *value = unpack(pool, packed, tail).ok_or(SettingsStringError::Corrupt)?.into();
        } else if key == TRICKS {
            if let Some(list) = unpack_tricks(value, tail) {
                *value = list.ok_or(SettingsStringError::Corrupt)?.into();
            }
        } else if ID_LISTS.contains(&key.as_str()) {
            indexes_to_ids(value).ok_or(SettingsStringError::Corrupt)?;
        }
    }
    Ok(with_defaults(&partial))
}

const TRICKS: &str = "EnabledTricks";

/// Settings holding (nested) lists of location ids.
const ID_LISTS: [&str; 1] = ["OverrideHintPriorities"];

fn ids_to_indexes(value: &mut Value) {
    match value {
        Value::Array(list) => list.iter_mut().for_each(ids_to_indexes),
        Value::String(id) => {
            if let Some(i) = mmr_data::game().items.iter().position(|it| it.id == *id) {
                *value = Value::from(i);
            }
        }
        _ => {}
    }
}

fn indexes_to_ids(value: &mut Value) -> Option<()> {
    match value {
        Value::Array(list) => list.iter_mut().try_for_each(indexes_to_ids),
        Value::Number(n) => {
            let id = &mmr_data::game().items.get(n.as_u64()? as usize)?.id;
            *value = Value::String(id.clone());
            Some(())
        }
        _ => Some(()),
    }
}

/// Every trick of the bundled logic files, append-only.
fn trick_list() -> &'static [String] {
    static LIST: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    LIST.get_or_init(|| serde_json::from_str(include_str!("../../../data/tricks.json")).expect("bundled trick list"))
}

fn pack_tricks(list: &[Value], tail: &mut Vec<u8>) -> Value {
    let known = trick_list();
    let offset = tail.len();
    tail.resize(offset + known.len().div_ceil(8), 0);
    let mut out = vec![Value::String(format!("#{}:{offset}", known.len()))];
    for v in list {
        let Some(name) = v.as_str() else { continue };
        match known.iter().position(|k| k == name) {
            Some(i) => tail[offset + i / 8] |= 1 << (i % 8),
            None => out.push(Value::String(name.to_string())),
        }
    }
    Value::Array(out)
}

/// `None`: not packed (an older string's plain list). `Some(None)`: damaged.
fn unpack_tricks(value: &Value, tail: &[u8]) -> Option<Option<Vec<String>>> {
    let list = value.as_array()?;
    let packed = list.first()?.as_str()?.strip_prefix('#')?;
    let known = trick_list();
    let decode = || {
        let (size, offset) = packed.split_once(':')?;
        let (size, offset): (usize, usize) = (size.parse().ok()?, offset.parse().ok()?);
        if size > known.len() {
            return None; // made by a newer version with more tricks
        }
        let bits = tail.get(offset..offset + size.div_ceil(8))?;
        let mut out: Vec<String> = (0..size).filter(|i| bits[i / 8] >> (i % 8) & 1 == 1).map(|i| known[i].clone()).collect();
        out.extend(list[1..].iter().filter_map(|v| v.as_str().map(String::from)));
        Some(out)
    };
    Some(decode())
}

fn item_pool(key: &str) -> Option<ItemPool> {
    schema().get(key).filter(|d| d.kind == SettingKind::ItemList).and_then(|d| d.pool)
}

fn pool_ids(pool: ItemPool) -> Vec<&'static str> {
    let game = mmr_data::game();
    match pool {
        ItemPool::Locations => game.locations().map(|i| i.id.as_str()).collect(),
        ItemPool::StartingItems => game.starting_items().map(|i| i.id.as_str()).collect(),
    }
}

fn pack(pool: ItemPool, list: &[Value], tail: &mut Vec<u8>) -> String {
    let ids = pool_ids(pool);
    let offset = tail.len();
    tail.resize(offset + ids.len().div_ceil(8), 0);
    for v in list {
        if let Some(i) = v.as_str().and_then(|s| ids.iter().position(|id| *id == s)) {
            tail[offset + i / 8] |= 1 << (i % 8);
        }
    }
    format!("#{}:{offset}", ids.len())
}

fn unpack(pool: ItemPool, packed: &str, tail: &[u8]) -> Option<Vec<String>> {
    let ids = pool_ids(pool);
    let (size, offset) = packed.strip_prefix('#')?.split_once(':')?;
    let (size, offset): (usize, usize) = (size.parse().ok()?, offset.parse().ok()?);
    if size > ids.len() {
        return None; // made by a newer version with more items
    }
    let bits = tail.get(offset..offset + size.div_ceil(8))?;
    Some((0..size).filter(|i| bits[i / 8] >> (i % 8) & 1 == 1).map(|i| ids[i].to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{defaults, presets, apply_preset};

    /// Tricks are a set: compare them sorted.
    fn normalized(values: &Values) -> Values {
        let mut v = changed(values);
        if let Some(Value::Array(t)) = v.get_mut(TRICKS) {
            t.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        }
        v
    }

    #[test]
    fn round_trips_presets() {
        for p in presets() {
            let values = apply_preset(&p, None);
            let s = encode_settings_string(&values, true);
            assert!(s.len() < 1500, "{} is {} chars", p.name, s.len());
            let back = decode_settings_string(&s).unwrap();
            assert!(normalized(&back) == normalized(&values), "{}", p.name);
        }
    }

    #[test]
    fn tricks_from_custom_logic_survive() {
        let mut values = defaults();
        values.insert(TRICKS.into(), serde_json::json!(["Lensless Chests", "A Trick Only My Logic Has"]));
        let back = decode_settings_string(&encode_settings_string(&values, false)).unwrap();
        assert_eq!(normalized(&back)[TRICKS], serde_json::json!(["A Trick Only My Logic Has", "Lensless Chests"]));
    }

    /// New tricks must be appended to data/tricks.json
    /// (`python tools/gen_tricks_list.py`), or strings get longer.
    #[test]
    fn trick_list_covers_bundled_logic() {
        for text in [include_str!("../../../data/logic/casual.json"), include_str!("../../../data/logic/glitched.json")] {
            let logic: Value = serde_json::from_str(text.trim_start_matches('\u{FEFF}')).unwrap();
            let entries = logic.get("entries").unwrap_or(&logic).as_array().unwrap();
            for e in entries.iter().filter(|e| e.get("trick").is_some()) {
                let id = e["id"].as_str().unwrap();
                assert!(trick_list().iter().any(|t| t == id), "{id} is missing from data/tricks.json: run tools/gen_tricks_list.py");
            }
        }
    }

    #[test]
    fn old_strings_with_plain_trick_lists_decode() {
        let payload = br#"{"EnabledTricks":["Lensless Chests"]}"#;
        let s = format!("{PREFIX}{}", B64.encode(miniz_oxide::deflate::compress_to_vec(payload, 10)));
        assert_eq!(decode_settings_string(&s).unwrap()[TRICKS], serde_json::json!(["Lensless Chests"]));
    }

    #[test]
    fn defaults_are_tiny() {
        assert!(encode_settings_string(&defaults(), true).len() < 20);
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(decode_settings_string("hello"), Err(SettingsStringError::Prefix)));
        assert!(matches!(decode_settings_string("MMRN.!!!"), Err(SettingsStringError::Corrupt)));
    }
}
