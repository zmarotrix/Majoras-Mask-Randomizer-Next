//! Settings for the Majora's Mask Randomizer.
//!
//! Setting values are a flat JSON object keyed by setting key (see
//! [`schema`]). A [`Settings`] value always contains every key; missing
//! keys are filled from defaults.

pub mod legacy;
pub mod schema;
pub mod string;
pub mod structured;

use schema::{schema, SettingDef, SettingKind};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashSet;

pub use string::{decode_settings_string, encode_settings_string};

pub type Values = Map<String, Value>;

/// Every setting at its default value.
pub fn defaults() -> Values {
    schema().settings.iter().map(|s| (s.key.clone(), s.default.clone())).collect()
}

/// Defaults overlaid with `partial`. Unknown keys are dropped.
pub fn with_defaults(partial: &Values) -> Values {
    let mut values = defaults();
    for (k, v) in partial {
        if values.contains_key(k) {
            values.insert(k.clone(), v.clone());
        }
    }
    values
}

/// Only the values that differ from the defaults.
pub fn changed(values: &Values) -> Values {
    let d = defaults();
    values.iter().filter(|(k, v)| d.get(*k).is_some_and(|dv| !same(dv, v))).map(|(k, v)| (k.clone(), v.clone())).collect()
}

/// Equality that ignores order in lists (item lists, flags, tricks).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && {
                let xs: HashSet<String> = x.iter().map(Value::to_string).collect();
                y.iter().all(|v| xs.contains(&v.to_string()))
            }
        }
        _ => a == b,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub key: String,
    pub message: String,
}

/// Check every value against the schema.
pub fn validate(values: &Values) -> Vec<Issue> {
    let game = mmr_data::game();
    let mut issues = Vec::new();
    for def in &schema().settings {
        let Some(v) = values.get(&def.key) else { continue };
        let mut bad = |message: String| issues.push(Issue { key: def.key.clone(), message });
        match def.kind {
            SettingKind::Bool if !v.is_boolean() => bad("must be on or off".into()),
            SettingKind::Number => match v.as_f64() {
                None if v.is_null() && def.nullable => {}
                None => bad("must be a number".into()),
                Some(n) => {
                    if def.integer && n.fract() != 0.0 {
                        bad("must be a whole number".into());
                    } else if def.min.is_some_and(|m| n < m) || def.max.is_some_and(|m| n > m) {
                        bad(format!("must be between {} and {}", fmt_num(def.min), fmt_num(def.max)));
                    }
                }
            },
            SettingKind::Enum => {
                if !v.as_str().is_some_and(|s| def.options.iter().any(|o| o.value == s)) {
                    bad(format!("{v} is not one of the options"));
                }
            }
            SettingKind::Flags => match v.as_array() {
                Some(list) => {
                    for f in list {
                        if !f.as_str().is_some_and(|s| def.options.iter().any(|o| o.value == s)) {
                            bad(format!("{f} is not one of the options"));
                        }
                    }
                }
                None => bad("must be a list".into()),
            },
            SettingKind::ItemList => match v.as_array() {
                Some(list) => {
                    for id in list {
                        let ok = id.as_str().and_then(|s| game.item(s)).is_some_and(|item| match def.pool {
                            Some(schema::ItemPool::Locations) => item.is_location(),
                            Some(schema::ItemPool::StartingItems) => item.starting_item,
                            None => true,
                        });
                        if !ok {
                            bad(format!("{id} can't be chosen here"));
                        }
                    }
                }
                None => bad("must be a list".into()),
            },
            SettingKind::Tricks if !v.is_array() => bad("must be a list".into()),
            SettingKind::File if !v.is_string() => bad("must be a file path".into()),
            SettingKind::Custom => {
                if let Some(message) = structured::check(def, v) {
                    bad(message);
                }
            }
            _ => {}
        }
    }
    issues
}

fn fmt_num(n: Option<f64>) -> String {
    n.map(|n| n.to_string()).unwrap_or_else(|| "∞".into())
}

#[derive(Debug, Clone, Serialize)]
pub struct Inactive {
    /// The setting that has no effect.
    pub key: String,
    /// The setting that makes it inactive.
    pub because: String,
    /// Human-readable explanation, e.g. "Has no effect when Logic Mode is No Logic".
    pub message: String,
}

/// Settings that currently have no effect because of other settings.
pub fn inactive(values: &Values) -> Vec<Inactive> {
    let mut out = Vec::new();
    for def in &schema().settings {
        let Some(v) = values.get(&def.key) else { continue };
        for ex in &def.excludes {
            if *v == ex.when {
                for key in &ex.disables {
                    out.push(Inactive { key: key.clone(), because: def.key.clone(), message: format!("Has no effect when {} is {}", def.label, describe_value(def, v)) });
                }
            }
        }
    }
    out
}

/// A short human-readable form of a value, e.g. "On", "No Logic".
pub fn describe_value(def: &SettingDef, v: &Value) -> String {
    match (def.kind, v) {
        (SettingKind::Bool, Value::Bool(b)) => if *b { "on" } else { "off" }.into(),
        (SettingKind::Enum, Value::String(s)) => def.options.iter().find(|o| o.value == *s).map(|o| o.label.clone()).unwrap_or_else(|| s.clone()),
        (_, Value::Array(a)) if a.is_empty() => "none".into(),
        (_, Value::Null) => "automatic".into(),
        _ => v.to_string(),
    }
}

/// A preset bundled with the randomizer.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Preset {
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// One plain-English sentence about what the preset is for.
    #[serde(default)]
    pub description: Option<String>,
    /// A few plain-English points about what it changes.
    #[serde(default)]
    pub highlights: Vec<String>,
    pub settings: Values,
}

macro_rules! presets {
    ($($id:literal),* $(,)?) => {
        &[$(($id, include_str!(concat!("../../../data/presets/", $id, ".json")))),*]
    };
}

const PRESETS: &[(&str, &str)] = presets!(
    "season-1-standard", "season-2-standard", "season-3-standard", "classic", "modern", "advanced", "blitz", "blitz-s2", "season-6", "sgl-2026",
    "allsanity-all-masks", "allsanity-all-ribbons", "allsanity-fairyhunt",
);

/// Bundled presets, in display order.
pub fn presets() -> Vec<Preset> {
    PRESETS
        .iter()
        .map(|(id, json)| {
            let mut p: Preset = serde_json::from_str(json).expect("bundled preset is valid");
            p.id = id.to_string();
            p
        })
        .collect()
}

/// Apply a preset: defaults, then the preset's values. Cosmetic settings in
/// `keep_cosmetics` are carried over, since presets are about gameplay.
pub fn apply_preset(preset: &Preset, keep_cosmetics: Option<&Values>) -> Values {
    let mut values = with_defaults(&preset.settings);
    if let Some(current) = keep_cosmetics {
        for (k, v) in current {
            if schema::is_cosmetic(k) {
                values.insert(k.clone(), v.clone());
            }
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        let issues = validate(&defaults());
        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn presets_are_described() {
        for p in presets() {
            assert!(p.description.as_deref().is_some_and(|d| !d.is_empty()), "{} has no description", p.name);
            assert!(!p.highlights.is_empty(), "{} has no highlights", p.name);
        }
    }

    #[test]
    fn presets_are_valid() {
        for p in presets() {
            let values = apply_preset(&p, None);
            let issues = validate(&values);
            assert!(issues.is_empty(), "{}: {issues:#?}", p.name);
        }
    }

    #[test]
    fn changed_ignores_list_order() {
        let mut v = defaults();
        let mut list = v["CustomItemList"].as_array().unwrap().clone();
        list.reverse();
        v.insert("CustomItemList".into(), list.into());
        assert!(changed(&v).is_empty());
        v.insert("CloseCows".into(), false.into());
        assert_eq!(changed(&v).keys().collect::<Vec<_>>(), vec!["CloseCows"]);
    }

    #[test]
    fn reports_inactive_settings() {
        let mut v = defaults();
        v.insert("QuestItemStorage".into(), false.into());
        let ia = inactive(&v);
        assert!(ia.iter().any(|i| i.key == "KeepQuestTradeThroughTime" && i.because == "QuestItemStorage"));
    }

    #[test]
    fn validation_catches_bad_values() {
        let mut v = defaults();
        v.insert("RequiredBossRemains".into(), 9.into());
        v.insert("DamageMode".into(), "Triple".into());
        v.insert("CustomStartingItemList".into(), serde_json::json!(["NotAnItem"]));
        let keys: Vec<_> = validate(&v).into_iter().map(|i| i.key).collect();
        assert_eq!(keys, vec!["RequiredBossRemains", "CustomStartingItemList", "DamageMode"]);
    }
}
