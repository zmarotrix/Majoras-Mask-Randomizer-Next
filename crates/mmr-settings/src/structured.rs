//! Structured setting values (per-form colours, HUD colours, D-Pad, trap
//! weights, starting item groups, hint priorities): checking and tidying.
//!
//! Values use MMR 2.0's settings.json shapes so its files import losslessly:
//! colours are `"R, G, B"` strings, per-form values are objects keyed by
//! form name (`Human`, `Deku`, `Goron`, `Zora`, `FierceDeity`).

use crate::schema::SettingDef;
use serde_json::{Map, Value};

/// Parse an `"R, G, B"` colour.
pub fn parse_color(v: &Value) -> Option<[u8; 3]> {
    let parts: Vec<u8> = v.as_str()?.split(',').map(|p| p.trim().parse().ok()).collect::<Option<_>>()?;
    <[u8; 3]>::try_from(parts).ok()
}

pub fn color(rgb: [u8; 3]) -> Value {
    Value::String(format!("{}, {}, {}", rgb[0], rgb[1], rgb[2]))
}

fn editor(def: &SettingDef) -> &str {
    def.editor.as_deref().unwrap_or("")
}

/// Problems with a structured value, if any.
pub fn check(def: &SettingDef, v: &Value) -> Option<String> {
    let obj = || v.as_object().ok_or_else(|| "must be an object".to_string());
    let result: Result<(), String> = (|| {
        match (editor(def), def.companion_of.is_some()) {
            ("tunic-colors" | "energy-colors", true) => {
                for (form, use_) in obj()? {
                    if !use_.is_boolean() {
                        return Err(format!("{form}: must be on or off"));
                    }
                }
            }
            ("tunic-colors", false) => {
                for (form, c) in obj()? {
                    parse_color(c).ok_or(format!("{form}: {c} is not a colour"))?;
                }
            }
            ("energy-colors", false) => {
                for (form, list) in obj()? {
                    let list = list.as_array().ok_or(format!("{form}: must be a list of colours"))?;
                    let expected = def.default.get(form).and_then(Value::as_array).map(Vec::len);
                    if expected.is_some_and(|n| n != list.len()) {
                        return Err(format!("{form}: must have {} colours", expected.unwrap()));
                    }
                    for c in list {
                        parse_color(c).ok_or(format!("{form}: {c} is not a colour"))?;
                    }
                }
            }
            ("instruments", _) => {
                for (form, i) in obj()? {
                    if !i.as_str().is_some_and(|s| def.options.iter().any(|o| o.value == s)) {
                        return Err(format!("{form}: {i} is not an instrument"));
                    }
                }
            }
            ("hud-colors", _) => {
                for (part, c) in obj()? {
                    parse_color(c).ok_or(format!("{part}: {c} is not a colour"))?;
                }
            }
            ("dpad", _) => {
                let values = v.pointer("/Pad/Values").and_then(Value::as_array).ok_or("must have four D-Pad slots")?;
                if values.len() != 4 {
                    return Err("must have four D-Pad slots".into());
                }
                for x in values {
                    if !x.as_str().is_some_and(|s| def.options.iter().any(|o| o.value == s)) {
                        return Err(format!("{x} can't go on the D-Pad"));
                    }
                }
                if !matches!(v.get("State").and_then(Value::as_str), Some("Enabled" | "Disabled")) {
                    return Err("State must be Enabled or Disabled".into());
                }
                if !matches!(v.get("Display").and_then(Value::as_str), Some("None" | "Left" | "Right")) {
                    return Err("Display must be None, Left or Right".into());
                }
            }
            ("trap-weights", _) => {
                for (trap, n) in obj()? {
                    if !def.options.iter().any(|o| o.value == *trap) {
                        return Err(format!("{trap} is not a trap type"));
                    }
                    if !n.as_u64().is_some_and(|n| n <= 1000) {
                        return Err(format!("{trap}: weight must be a whole number from 0 to 1000"));
                    }
                }
            }
            ("starting-groups", _) => {
                let game = mmr_data::game();
                for (i, g) in v.as_array().ok_or("must be a list")?.iter().enumerate() {
                    let n = i + 1;
                    let items = g.get("Items").and_then(Value::as_array).ok_or(format!("group {n}: needs items"))?;
                    if items.is_empty() {
                        return Err(format!("group {n}: needs at least one item"));
                    }
                    for id in items {
                        if !id.as_str().and_then(|s| game.item(s)).is_some_and(|it| it.starting_item) {
                            return Err(format!("group {n}: {id} can't be a starting item"));
                        }
                    }
                    let amount = g.get("Amount").and_then(Value::as_u64).unwrap_or(0) as usize;
                    if amount < 1 || amount > items.len() {
                        return Err(format!("group {n}: amount must be between 1 and {}", items.len()));
                    }
                }
            }
            ("hint-priorities", false) if !v.is_null() => {
                let game = mmr_data::game();
                for tier in v.as_array().ok_or("must be a list of tiers")? {
                    for id in tier.as_array().ok_or("each tier must be a list of locations")? {
                        if !id.as_str().and_then(|s| game.item(s)).is_some_and(|it| it.is_location()) {
                            return Err(format!("{id} is not a location"));
                        }
                    }
                }
            }
            ("hint-priorities", true) if !v.is_null() => {
                for n in v.as_array().ok_or("must be a list of numbers")? {
                    if !n.is_u64() {
                        return Err(format!("{n} is not a whole number"));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    })();
    result.err()
}

/// Tidy an imported value: per-form/per-part objects get any missing
/// entries from the default (MMR 2.0 files can predate a form or HUD part).
/// Returns `None` if the value can't be used at all.
pub fn normalize(def: &SettingDef, v: &Value) -> Option<Value> {
    let fill = |v: &Value| -> Option<Value> {
        let mut out: Map<String, Value> = def.default.as_object()?.clone();
        for (k, x) in v.as_object()? {
            // Keys are case-insensitive in MMR 2.0 files.
            let key = out.keys().find(|d| d.eq_ignore_ascii_case(k)).cloned().unwrap_or_else(|| k.clone());
            out.insert(key, x.clone());
        }
        Some(Value::Object(out))
    };
    let tidy = match editor(def) {
        "tunic-colors" | "energy-colors" | "instruments" | "hud-colors" if def.default.is_object() => fill(v)?,
        _ => v.clone(),
    };
    check(def, &tidy).is_none().then_some(tidy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::schema;
    use serde_json::json;

    #[test]
    fn defaults_are_valid() {
        for def in schema().settings.iter().filter(|d| d.editor.is_some()) {
            assert_eq!(check(def, &def.default), None, "{}", def.key);
        }
    }

    #[test]
    fn colours() {
        assert_eq!(parse_color(&json!("30, 105, 27")), Some([30, 105, 27]));
        assert_eq!(parse_color(&json!("30,105")), None);
        assert_eq!(parse_color(&json!("300, 0, 0")), None);
        assert_eq!(color([1, 2, 3]), json!("1, 2, 3"));
    }

    #[test]
    fn import_fills_missing_forms() {
        let def = schema().get("Cosmetic.TunicColors").unwrap();
        let v = normalize(def, &json!({"human": "1, 2, 3"})).unwrap();
        assert_eq!(v["Human"], json!("1, 2, 3"));
        assert_eq!(v["FierceDeity"], json!("189, 181, 173"));
        assert!(normalize(def, &json!({"Human": "red-ish"})).is_none());
    }

    #[test]
    fn checks_groups_and_dpad() {
        let groups = schema().get("RandomStartingItemGroups").unwrap();
        assert!(check(groups, &json!([{"Items": ["MaskDeku", "ItemBow"], "Amount": 1}])).is_none());
        assert!(check(groups, &json!([{"Items": ["MaskDeku"], "Amount": 2}])).is_some());
        let dpad = schema().get("Cosmetic.DPad").unwrap();
        assert!(check(dpad, &json!({"Pad": {"Values": ["None", "None", "Ocarina", "None"]}, "State": "Enabled", "Display": "Right"})).is_none());
        assert!(check(dpad, &json!({"Pad": {"Values": ["Bow", "None", "Ocarina", "None"]}, "State": "Enabled", "Display": "Right"})).is_some());
    }
}
