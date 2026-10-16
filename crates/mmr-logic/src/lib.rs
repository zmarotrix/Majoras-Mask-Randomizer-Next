//! Logic for the Majora's Mask Randomizer.
//!
//! * [`model`]: the logic file format
//! * [`legacy`]: conversion to/from MMR 2.0 logic files
//! * [`condition`]: setting conditions
//! * [`validate`]: problems a logic editor should show
//! * [`eval`]: what is reachable with a given inventory, and a vanilla playthrough

pub mod condition;
pub mod convert;
pub mod equivalence;
pub mod eval;
pub mod legacy;
pub mod legacy_settings;
pub mod model;
pub mod region;
pub mod regionize;
pub mod settings_rules;
pub mod validate;

pub use condition::SettingCondition;
pub use model::{Entry, EntryKind, LogicFile, Time, TimeSet, Trick};

/// The entry that represents beating the game.
pub const GOAL: &str = "OtherCredits";

#[derive(Debug, thiserror::Error)]
pub enum LogicError {
    #[error("not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("this is not a logic file (expected an MMR Next logic file or one from an older MMR)")]
    UnknownFormat,
    #[error("this logic file uses format version {0}, but this version of the randomizer only reads versions 1 and 2")]
    FormatVersion(u32),
    #[error("this logic file is from an older version of MMR (logic version {0}). Open and save it in a newer version of the old randomizer first, then import it here")]
    LegacyVersion(u32),
    #[error("entry {0:?}: {1}")]
    Entry(String, String),
}

/// Which format a file was loaded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFormat {
    /// MMR Next entry format (version 1).
    Current,
    /// MMR Next region format (version 2), compiled to entries on load.
    Regions,
    /// MMR 2.0.
    Legacy,
}

/// Load a logic file in either the current or the legacy format.
pub fn load(json: &str) -> Result<(LogicFile, SourceFormat), LogicError> {
    let json = json.trim_start_matches('\u{feff}');
    let value: serde_json::Value = serde_json::from_str(json)?;
    if value.get("format").and_then(|f| f.as_str()) == Some(model::FORMAT_NAME) {
        if value.get("version").and_then(|v| v.as_u64()) == Some(2) {
            let regions: region::RegionLogic = serde_json::from_value(value)?;
            let mut file = regions.compile();
            apply_multi_locations(&mut file);
            return Ok((file, SourceFormat::Regions));
        }
        let mut file: LogicFile = serde_json::from_value(value)?;
        if file.version > 2 {
            return Err(LogicError::FormatVersion(file.version));
        }
        apply_multi_locations(&mut file);
        return Ok((file, SourceFormat::Current));
    }
    if value.get("Version").is_some() && value.get("Logic").is_some() {
        let legacy: legacy::LegacyFile = serde_json::from_value(value)?;
        // MMR 2.0 rewrote logic per settings in code; build those rules in.
        return Ok((settings_rules::bake(&legacy::from_legacy(legacy)?), SourceFormat::Legacy));
    }
    Err(LogicError::UnknownFormat)
}

/// Load an MMR 2.0 logic file exactly as written, without building in the
/// settings rules (for comparing against MMR 2.0 behaviour).
pub fn load_legacy_raw(json: &str) -> Result<LogicFile, LogicError> {
    let legacy: legacy::LegacyFile = serde_json::from_str(json.trim_start_matches('\u{feff}'))?;
    legacy::from_legacy(legacy)
}

/// Serialize in the current format.
pub fn to_json(file: &LogicFile) -> String {
    serde_json::to_string_pretty(file).expect("logic serializes")
}

/// Load one of the logic files bundled with the randomizer.
pub fn bundled(id: &str) -> Option<LogicFile> {
    let (_, _, json) = mmr_data::BUNDLED_LOGIC.iter().find(|(i, _, _)| *i == id)?;
    Some(load(json).expect("bundled logic is valid").0)
}

/// One of the bundled logic files in the region format.
pub fn bundled_regions(id: &str) -> Option<region::RegionLogic> {
    let (_, json) = mmr_data::BUNDLED_REGIONS.iter().find(|(i, _)| *i == id)?;
    Some(serde_json::from_str(json).expect("bundled region logic is valid"))
}

/// A region-format logic file, or `None` if the JSON is another format.
pub fn parse_regions(json: &str) -> Result<Option<region::RegionLogic>, LogicError> {
    let value: serde_json::Value = serde_json::from_str(json.trim_start_matches('\u{feff}'))?;
    if value.get("version").and_then(|v| v.as_u64()) == Some(2) && value.get("regions").is_some() {
        return Ok(Some(serde_json::from_value(value)?));
    }
    Ok(None)
}

/// Items found at one of several locations have derived logic: any one of
/// those locations. Matches what MMR 2.0 does when loading logic.
pub fn apply_multi_locations(file: &mut LogicFile) {
    let game = mmr_data::game();
    for entry in &mut file.entries {
        if let Some(locations) = game.item(&entry.id).and_then(|i| i.multi_location.as_ref()) {
            entry.requires.clear();
            entry.any_of = locations.iter().map(|l| vec![l.clone()]).collect();
            entry.time = Time::default();
            entry.trick = None;
        }
    }
}

/// Whether an entry's logic is derived from game data and not editable.
pub fn is_derived(id: &str) -> bool {
    mmr_data::game().item(id).is_some_and(|i| i.multi_location.is_some())
}

/// Whether a reference to `id` means "has the item" (true) or "the entry
/// with this id is satisfied" (false).
pub fn is_item_ref(id: &str) -> bool {
    mmr_data::game().item(id).is_some_and(|i| !i.is_fake())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY_CASUAL: &str = include_str!("../../../data/logic/legacy-casual.json");
    const LEGACY_GLITCHED: &str = include_str!("../../../data/logic/legacy-glitched.json");

    #[test]
    fn imports_bundled_legacy_logic() {
        for src in [LEGACY_CASUAL, LEGACY_GLITCHED] {
            let (file, format) = load(src).unwrap();
            assert_eq!(format, SourceFormat::Legacy);
            assert!(file.entries.len() > 1500);
            assert!(file.entries.iter().any(|e| e.trick.is_some()));
            assert!(file.entries.iter().any(|e| e.setting.is_some()));
        }
    }

    #[test]
    fn legacy_round_trip_is_lossless() {
        for src in [LEGACY_CASUAL, LEGACY_GLITCHED] {
            let file = load_legacy_raw(src).unwrap();
            let enum_type = |k: &str| Some(k.rsplit('.').next().unwrap().to_string());
            let exported = legacy::to_legacy(&file, &enum_type);
            let json = serde_json::to_string(&exported).unwrap();
            assert_eq!(file, load_legacy_raw(&json).unwrap());
            // Importing builds in the settings rules, exactly once.
            let (baked, _) = load(&json).unwrap();
            assert!(settings_rules::is_baked(&baked));
            assert_eq!(baked, settings_rules::bake(&baked));
        }
    }

    #[test]
    fn current_format_round_trip() {
        let (file, _) = load(LEGACY_CASUAL).unwrap();
        let (again, format) = load(&to_json(&file)).unwrap();
        assert_eq!(format, SourceFormat::Current);
        assert_eq!(file, again);
    }

    #[test]
    fn rejects_old_legacy_versions() {
        let err = load(r#"{"Version": 12, "Logic": []}"#).unwrap_err();
        assert!(matches!(err, LogicError::LegacyVersion(12)));
    }
}
