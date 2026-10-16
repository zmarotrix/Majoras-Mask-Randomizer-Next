//! The settings schema: every setting's type, default, label and help text.
//!
//! Defined once in `data/settings.json`; the UI, CLI, validation and
//! settings strings are all driven by it.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

const SETTINGS_JSON: &str = include_str!("../../../data/settings.json");

/// How settings are presented: sections, themed groups, friendlier labels.
/// Purely presentation; the UI reads it as-is.
pub const LAYOUT_JSON: &str = include_str!("../../../data/layout.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingDef {
    pub key: String,
    pub label: String,
    pub category: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub kind: SettingKind,
    pub default: Value,
    /// For `enum` / `flags`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<OptionDef>,
    /// Legacy C# enum type name, for `enum` / `flags`.
    #[serde(default, rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_type: Option<String>,
    /// For `number`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default)]
    pub integer: bool,
    /// `number` settings that may be left empty (meaning "automatic").
    #[serde(default)]
    pub nullable: bool,
    /// For `item_list`: which items may be chosen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool: Option<ItemPool>,
    /// While this setting has a given value, other settings have no effect.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excludes: Vec<Exclude>,
    /// Stored in the game's MiscConfig rather than used by the randomizer.
    #[serde(default)]
    pub asm: bool,
    /// For `custom` settings: which editor the UI shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor: Option<String>,
    /// Edited together with another setting's editor (e.g. "use tunic
    /// colors" with the tunic colors), so not shown on its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_of: Option<String>,
    /// Transformation forms the value has an entry for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forms: Vec<String>,
    /// Editable parts of the value (e.g. HUD elements).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<OptionDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingKind {
    Bool,
    Number,
    /// One option out of several.
    Enum,
    /// Any combination of options; the value is a list.
    Flags,
    /// A list of item ids from `pool`.
    ItemList,
    /// A list of trick ids from the selected logic.
    Tricks,
    /// A file path.
    File,
    /// Structured value with a dedicated editor (colors, D-Pad, ...).
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionDef {
    pub value: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemPool {
    Locations,
    StartingItems,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exclude {
    pub when: Value,
    pub disables: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryDef {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

/// Categories in display order.
pub const CATEGORIES: &[CategoryDef] = &[
    CategoryDef { id: "logic", label: "Logic", description: "How the randomizer decides what is possible: logic rules and tricks." },
    CategoryDef { id: "items", label: "Item pool", description: "Which locations are shuffled, what you start with, and where junk goes." },
    CategoryDef { id: "randomization", label: "Randomization", description: "Keys, fairies, entrances, enemies, prices and other shuffled elements." },
    CategoryDef { id: "hints", label: "Hints", description: "Gossip stones, Garos and other in-game hints." },
    CategoryDef { id: "gimmicks", label: "Gimmicks", description: "Changes that make the game harder or stranger." },
    CategoryDef { id: "quality_of_life", label: "Quality of life", description: "Conveniences that make the game smoother without changing what's possible." },
    CategoryDef { id: "speedups", label: "Speedups", description: "Shorter minigames and tasks." },
    CategoryDef { id: "cutscenes", label: "Cutscenes", description: "Which cutscenes are shortened." },
    CategoryDef { id: "cosmetics", label: "Cosmetics", description: "Colors, music and sounds. These never affect the seed." },
    CategoryDef { id: "general", label: "General", description: "Other settings." },
];

pub struct Schema {
    pub settings: Vec<SettingDef>,
}

impl Schema {
    pub fn get(&self, key: &str) -> Option<&SettingDef> {
        self.settings.iter().find(|s| s.key == key)
    }
}

static SCHEMA: LazyLock<Schema> = LazyLock::new(|| Schema {
    settings: serde_json::from_str(SETTINGS_JSON).expect("data/settings.json is valid"),
});

pub fn schema() -> &'static Schema {
    &SCHEMA
}

/// Cosmetic settings live under this prefix and never affect the seed.
pub fn is_cosmetic(key: &str) -> bool {
    key.starts_with("Cosmetic.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn layout_covers_every_setting_once() {
        let layout: serde_json::Value = serde_json::from_str(LAYOUT_JSON).unwrap();
        let mut seen: HashMap<String, usize> = HashMap::new();
        for section in layout["sections"].as_array().unwrap() {
            for group in section["groups"].as_array().unwrap() {
                let more = group.get("more").and_then(|m| m["settings"].as_array()).cloned().unwrap_or_default();
                for key in group["settings"].as_array().unwrap().iter().chain(more.iter()) {
                    *seen.entry(key.as_str().unwrap().to_string()).or_default() += 1;
                }
            }
        }
        let schema = schema();
        for key in seen.keys() {
            assert!(schema.get(key).is_some(), "layout mentions unknown setting {key}");
        }
        for def in &schema.settings {
            if def.kind == SettingKind::ItemList {
                continue; // edited on the Item pool page
            }
            assert_eq!(seen.get(&def.key).copied().unwrap_or(0), 1, "{} must appear in exactly one layout group", def.key);
        }
        for key in layout["labels"].as_object().unwrap().keys() {
            assert!(schema.get(key).is_some(), "layout labels mention unknown setting {key}");
        }
        for (key, options) in layout["optionLabels"].as_object().unwrap() {
            let def = schema.get(key).unwrap_or_else(|| panic!("optionLabels: unknown setting {key}"));
            for value in options.as_object().unwrap().keys() {
                assert!(def.options.iter().any(|o| &o.value == value), "optionLabels: {key} has no option {value}");
            }
        }
    }
}
