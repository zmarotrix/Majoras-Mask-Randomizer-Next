//! Conversion to and from the legacy (MMR 2.0) logic JSON format.

use crate::condition::SettingCondition;
use crate::model::{Entry, LogicFile, Time, TimeSet, Trick};
use crate::LogicError;
use serde::{Deserialize, Serialize};

/// The only legacy version we read and write. Older files must first be
/// opened and saved in MMR 2.0, which migrates them.
pub const LEGACY_VERSION: u32 = 31;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LegacyFile {
    pub version: u32,
    pub logic: Vec<LegacyEntry>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LegacyEntry {
    pub id: String,
    #[serde(default)]
    pub required_items: Vec<String>,
    #[serde(default)]
    pub conditional_items: Vec<Vec<String>>,
    #[serde(default = "none")]
    pub time_needed: String,
    #[serde(default = "none")]
    pub time_available: String,
    #[serde(default = "none")]
    pub time_setup: String,
    #[serde(default)]
    pub is_trick: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trick_tooltip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trick_category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trick_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_expression: Option<String>,
}

fn none() -> String {
    "None".into()
}

pub fn from_legacy(file: LegacyFile) -> Result<LogicFile, LogicError> {
    if file.version != LEGACY_VERSION {
        return Err(LogicError::LegacyVersion(file.version));
    }
    let mut entries = Vec::with_capacity(file.logic.len());
    for e in file.logic {
        let time = |s: &str| TimeSet::from_legacy(s).map_err(|err| LogicError::Entry(e.id.clone(), err.to_string()));
        let entry = Entry {
            requires: e.required_items.clone(),
            any_of: e.conditional_items.clone(),
            before: Vec::new(),
            temporary_requires: Vec::new(),
            time: Time { needed: time(&e.time_needed)?, available: time(&e.time_available)?, setup: time(&e.time_setup)?, no_return: false, opening_hours: false },
            trick: e.is_trick.then(|| Trick {
                description: e.trick_tooltip.clone().filter(|s| !s.is_empty()),
                category: e.trick_category.clone().filter(|s| !s.is_empty()),
                url: e.trick_url.clone().filter(|s| !s.is_empty()),
            }),
            setting: match e.setting_expression.as_deref().map(str::trim) {
                Some(src) if !src.is_empty() => Some(
                    SettingCondition::from_legacy(src).map_err(|err| LogicError::Entry(e.id.clone(), err.to_string()))?,
                ),
                _ => None,
            },
            note: None,
            id: e.id,
        };
        entries.push(entry);
    }
    let mut file = LogicFile::new(entries);
    crate::apply_multi_locations(&mut file);
    Ok(file)
}

/// `enum_type` maps a setting key to its C# enum type name, for settings
/// conditions that compare against enum values.
pub fn to_legacy(file: &LogicFile, enum_type: &dyn Fn(&str) -> Option<String>) -> LegacyFile {
    LegacyFile {
        version: LEGACY_VERSION,
        logic: file
            .entries
            .iter()
            .map(|e| LegacyEntry {
                id: e.id.clone(),
                required_items: e.requires.clone(),
                conditional_items: e.any_of.clone(),
                time_needed: e.time.needed.to_legacy(),
                time_available: e.time.available.to_legacy(),
                time_setup: e.time.setup.to_legacy(),
                is_trick: e.trick.is_some(),
                trick_tooltip: e.trick.as_ref().map(|t| t.description.clone().unwrap_or_default()),
                trick_category: e.trick.as_ref().and_then(|t| t.category.clone()),
                trick_url: e.trick.as_ref().and_then(|t| t.url.clone()),
                setting_expression: e.setting.as_ref().map(|c| c.to_legacy(enum_type)),
            })
            .collect(),
    }
}
