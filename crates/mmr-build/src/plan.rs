//! What the builder needs from the randomizer: the settings, where every
//! item went, prices, hints and so on.
//!
//! The shape matches what `tools/legacy-harness` exports from the old
//! randomizer, so the two builders can be compared on identical input.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Plan {
    pub seed: i64,
    /// Version text for the title screen ("v1.16.0.0"); MMR Next's own when absent.
    #[serde(default)]
    pub version: Option<String>,
    /// The old randomizer's build hash, when comparing with it.
    #[serde(default)]
    pub hash: Option<Vec<u8>>,
    /// Cosmetic settings, keyed like the old settings files.
    #[serde(default)]
    pub cosmetic: serde_json::Map<String, Value>,
    /// Gameplay settings, keyed like the old settings files.
    pub settings: serde_json::Map<String, Value>,
    /// Randomized locations (decoded from `CustomItemListString`).
    #[serde(default, deserialize_with = "null_as_default")]
    pub custom_item_list: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub custom_starting_item_list: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub custom_junk_locations: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub items: Vec<PlacedItem>,
    #[serde(default)]
    pub gossip_quotes: Option<Vec<Message>>,
    /// What each hint is about, for the tracker (not part of the old plan).
    #[serde(default, skip)]
    pub hint_info: Vec<crate::hints::HintInfo>,
    #[serde(default)]
    pub message_costs: Option<Vec<Option<u16>>>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub gibdo_requirements: Vec<GibdoRequirement>,
    /// Locations the randomizer treats as junk.
    #[serde(default, deserialize_with = "null_as_default")]
    pub junk_locations: Vec<String>,
    /// Items nothing in logic needs.
    #[serde(default, deserialize_with = "null_as_default")]
    pub logically_junk_items: Vec<String>,
    /// The logic used, in the old entry form (ids as strings).
    #[serde(default, deserialize_with = "null_as_default")]
    pub logic: Vec<LogicEntry>,
    #[serde(default)]
    pub blitz_extra_items: Option<Vec<String>>,
    #[serde(default)]
    pub random_starting_items: Option<Vec<String>>,
    #[serde(default)]
    pub traps: Option<Vec<Trap>>,
    #[serde(default)]
    pub file_select_skybox: i32,
    #[serde(default)]
    pub file_select_color: i32,
    #[serde(default)]
    pub title_logo_color: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlacedItem {
    /// The item's original id (`ItemObject.ID`).
    #[serde(default)]
    pub id: String,
    pub item: String,
    /// The location it was placed at; `None` if unused.
    pub location: Option<String>,
    pub is_randomized: bool,
    pub name_override: Option<String>,
    pub mimic: Option<Mimic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GibdoRequirement {
    pub amount: u8,
    pub item_required: String,
    pub logic_entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LogicEntry {
    pub id: String,
    pub required: Option<Vec<String>>,
    pub conditional: Option<Vec<Vec<String>>>,
    pub is_fake_item: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Mimic {
    pub item: String,
    pub fake_name: Option<String>,
    pub chest_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Trap {
    pub item: String,
    pub location: Option<String>,
    pub mimic: Option<Mimic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Message {
    pub id: u16,
    pub header: Option<Vec<u8>>,
    /// Characters as numbers (messages hold raw bytes, not text).
    pub message: Vec<u16>,
}

impl Plan {
    pub fn setting(&self, key: &str) -> &Value {
        self.settings.get(key).unwrap_or(&Value::Null)
    }
    pub fn flag(&self, key: &str) -> bool {
        self.setting(key).as_bool().unwrap_or(false)
    }
    /// An enum setting's value name, e.g. `"Casual"`.
    pub fn choice(&self, key: &str) -> &str {
        self.setting(key).as_str().unwrap_or("")
    }
    pub fn number(&self, key: &str) -> i64 {
        self.setting(key).as_i64().unwrap_or(0)
    }
    /// Whether a flags setting (stored as "A, B, C") includes a value.
    pub fn has(&self, key: &str, flag: &str) -> bool {
        has_flag(self.setting(key), flag)
    }
    /// An enum setting as its number (flags settings combine their values).
    pub fn enum_value(&self, key: &str) -> i64 {
        let ty = crate::enums::setting_type(key).unwrap_or_else(|| panic!("{key} is not an enum setting"));
        crate::stages::flags_value(crate::enums::get(ty), self.setting(key))
    }

    /// A setting nested in an object, e.g. `ShortenCutsceneSettings.General`.
    pub fn nested(&self, key: &str, field: &str) -> &Value {
        self.setting(key).get(field).unwrap_or(&Value::Null)
    }
}

/// Flags enums are serialized as a comma-separated list of names.
pub fn has_flag(v: &Value, flag: &str) -> bool {
    match v {
        Value::String(s) => s.split(',').any(|x| x.trim() == flag),
        Value::Array(a) => a.iter().any(|x| x.as_str() == Some(flag)),
        _ => false,
    }
}

impl PlacedItem {
    /// What the item looks like (a trap's disguise, or itself).
    pub fn display(&self) -> &str {
        self.mimic.as_ref().map(|m| m.item.as_str()).unwrap_or(&self.item)
    }
    /// Where it was placed.
    pub fn loc(&self) -> &str {
        self.location.as_deref().unwrap_or("")
    }
    /// The old `ItemObject.DisplayName`.
    pub fn display_name(&self) -> &str {
        crate::stages::display_name(self)
    }
    /// The display name without counts ("10 Bombs" -> "Bombs").
    pub fn alternate_name(&self) -> String {
        crate::stages::alternate_name(self.display_name())
    }
}

/// `null` (the old randomizer's missing lists) reads as empty.
fn null_as_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}
