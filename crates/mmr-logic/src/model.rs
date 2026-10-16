//! The logic file format.
//!
//! A logic file is a list of entries. Each entry says what is needed to
//! reach a location, obtain an event, or satisfy a helper:
//!
//! ```json
//! {
//!   "id": "ItemBow",
//!   "requires": ["AreaWoodFallTempleAccess", "WFT 2nd Floor Access"],
//!   "any_of": [["Any Sword"], ["OtherArrow"], ["MaskDeku", "Magic Meter"]]
//! }
//! ```
//!
//! means "every id in `requires`, and every id in at least one `any_of`
//! group". This is the same meaning as the legacy `RequiredItems` /
//! `ConditionalItems` pair, so files convert losslessly in both directions.

use crate::condition::SettingCondition;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const FORMAT_NAME: &str = "mmr-logic";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogicFile {
    /// Always `"mmr-logic"`; distinguishes the file from legacy logic.
    pub format: String,
    pub version: u32,
    /// Free-form description shown in the editor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub entries: Vec<Entry>,
}

impl LogicFile {
    pub fn new(entries: Vec<Entry>) -> Self {
        LogicFile { format: FORMAT_NAME.into(), version: FORMAT_VERSION, description: None, entries }
    }

    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    /// Every one of these is needed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    /// At least one of these groups is needed (all ids within the group).
    /// Empty means no alternatives are needed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_of: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Time::is_empty")]
    pub time: Time,
    /// Must be done before any of these happen in the same cycle (e.g. the
    /// Goron Elder's song can't be learned once spring has come).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<String>,
    /// Also needed, but only when what's found here is lost on the Song of
    /// Time (a temporary item, or anything before time travel is possible).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub temporary_requires: Vec<String>,
    /// Present if this entry is a trick the player can opt into.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trick: Option<Trick>,
    /// Present if this entry is satisfied by a setting rather than by items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting: Option<SettingCondition>,
    /// Notes for logic maintainers. Not used by the randomizer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Entry {
    pub fn new(id: impl Into<String>) -> Self {
        Entry { id: id.into(), ..Default::default() }
    }

    /// Every id this entry references, with where it appears.
    pub fn references(&self) -> impl Iterator<Item = &str> {
        self.requires
            .iter()
            .chain(self.any_of.iter().flatten())
            .chain(&self.temporary_requires)
            .chain(&self.before)
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Trick {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Time-of-day constraints. See the editor help for what each field means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Time {
    /// When having this item matters: locations that need it must be
    /// available at one of these times.
    #[serde(default, skip_serializing_if = "TimeSet::is_empty")]
    pub needed: TimeSet,
    /// When this location or event can be done. Empty means any time.
    #[serde(default, skip_serializing_if = "TimeSet::is_empty")]
    pub available: TimeSet,
    /// When the setup for this happens, if different from `available`.
    #[serde(default, skip_serializing_if = "TimeSet::is_empty")]
    pub setup: TimeSet,
    /// There's no coming back from here within the cycle (the Moon, the
    /// Clock Tower roof): what's found here only helps after the Song of
    /// Time, except time-travel items themselves.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_return: bool,
    /// The times are opening hours (a shop door, a grotto): a check that
    /// needs this place must be collectable while it's open. Otherwise the
    /// times join the chain like any other requirement.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub opening_hours: bool,
}

impl Time {
    pub fn is_empty(&self) -> bool {
        self.needed.is_empty() && self.available.is_empty() && self.setup.is_empty() && !self.no_return && !self.opening_hours
    }
}

/// A set of the six day/night periods, as the legacy `TimeOfDay` flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct TimeSet(pub u8);

impl TimeSet {
    pub const PERIODS: [&'static str; 6] = ["Day1", "Night1", "Day2", "Night2", "Day3", "Night3"];
    pub const ALL: TimeSet = TimeSet(0b11_1111);

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn names(&self) -> Vec<&'static str> {
        Self::PERIODS.iter().enumerate().filter(|(i, _)| self.0 & (1 << i) != 0).map(|(_, n)| *n).collect()
    }

    pub fn from_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Result<Self, UnknownPeriod> {
        let mut bits = 0;
        for name in names {
            let name = name.trim();
            if name.is_empty() || name == "None" {
                continue;
            }
            if name == "All" {
                bits |= Self::ALL.0;
                continue;
            }
            let i = Self::PERIODS.iter().position(|p| *p == name).ok_or_else(|| UnknownPeriod(name.to_string()))?;
            bits |= 1 << i;
        }
        Ok(TimeSet(bits))
    }

    /// Parse the legacy comma-separated form, e.g. `"Day1, Night1"`.
    pub fn from_legacy(s: &str) -> Result<Self, UnknownPeriod> {
        Self::from_names(s.split(','))
    }

    pub fn to_legacy(&self) -> String {
        match self.0 {
            0 => "None".into(),
            b if b == Self::ALL.0 => "All".into(),
            _ => self.names().join(", "),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown time of day {0:?} (expected one of Day1, Night1, Day2, Night2, Day3, Night3)")]
pub struct UnknownPeriod(pub String);

impl fmt::Display for TimeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_legacy())
    }
}

impl Serialize for TimeSet {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.names().serialize(s)
    }
}

impl<'de> Deserialize<'de> for TimeSet {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let names = Vec::<String>::deserialize(d)?;
        TimeSet::from_names(names.iter().map(String::as_str)).map_err(serde::de::Error::custom)
    }
}

/// What kind of thing an entry describes. Derived, not stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// A place an item can be found (and the item normally found there).
    Location,
    /// An area or event from the game data that is not a location.
    Event,
    /// A trick the player can opt into.
    Trick,
    /// Satisfied by a setting.
    Setting,
    /// A named group of requirements, reused by other entries.
    Helper,
}

impl EntryKind {
    pub fn of(entry: &Entry) -> EntryKind {
        if entry.trick.is_some() {
            return EntryKind::Trick;
        }
        if entry.setting.is_some() {
            return EntryKind::Setting;
        }
        match mmr_data::game().item(&entry.id) {
            Some(item) if item.is_location() => EntryKind::Location,
            Some(_) => EntryKind::Event,
            None => EntryKind::Helper,
        }
    }
}
