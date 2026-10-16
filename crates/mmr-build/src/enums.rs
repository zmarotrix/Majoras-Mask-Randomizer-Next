//! ROM data that the old randomizer kept in C# attributes on enum values
//! (addresses, get-item indices, patches...), dumped by
//! `tools/import_rom_data.py` into `data/rom/enums`.

use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct EnumValue {
    pub name: String,
    pub value: i64,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize)]
pub struct Attribute {
    #[serde(rename = "type")]
    pub kind: String,
    /// Constructor arguments, as written in the C# source.
    pub args: Vec<Value>,
    /// Public properties after construction.
    pub values: Map<String, Value>,
}

impl Attribute {
    pub fn get(&self, key: &str) -> &Value {
        self.values.get(key).unwrap_or(&Value::Null)
    }
}

pub struct Enum {
    pub values: Vec<EnumValue>,
    by_name: HashMap<String, usize>,
}

impl Enum {
    fn parse(json: &str) -> Enum {
        let values: Vec<EnumValue> = serde_json::from_str(json).expect("bundled enum data is valid");
        let by_name = values.iter().enumerate().map(|(i, v)| (v.name.clone(), i)).collect();
        Enum { values, by_name }
    }

    pub fn get(&self, name: &str) -> Option<&EnumValue> {
        self.by_name.get(name).map(|&i| &self.values[i])
    }

    /// Attributes of one kind on a value (empty if the value is unknown).
    pub fn attrs<'a>(&'a self, name: &'a str, kind: &'a str) -> impl Iterator<Item = &'a Attribute> + 'a {
        self.get(name).into_iter().flat_map(move |v| v.attributes.iter().filter(move |a| a.kind == kind))
    }

    pub fn attr<'a>(&'a self, name: &'a str, kind: &'a str) -> Option<&'a Attribute> {
        self.attrs(name, kind).next()
    }
}

/// A dumped enum by its C# type name.
pub fn get(name: &str) -> &'static Enum {
    static CELLS: OnceLock<std::sync::Mutex<HashMap<String, &'static Enum>>> = OnceLock::new();
    let cells = CELLS.get_or_init(Default::default);
    let mut map = cells.lock().unwrap();
    if let Some(e) = map.get(name) {
        return e;
    }
    let source = crate::enum_files::source(name).unwrap_or_else(|| panic!("enum {name} is not bundled"));
    let e: &'static Enum = Box::leak(Box::new(Enum::parse(source)));
    map.insert(name.to_string(), e);
    e
}

/// The enum type of a setting (e.g. "DamageMode" for `DamageMode`,
/// "GossipHintStyle" for `GaroHintStyle`).
pub fn setting_type(key: &str) -> Option<&'static str> {
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    let map = MAP.get_or_init(|| serde_json::from_str(include_str!("../../../data/rom/settings_enums.json")).expect("bundled settings enums"));
    map.get(key).map(String::as_str)
}

pub fn items() -> &'static Enum {
    get("Item")
}
pub fn entrances() -> &'static Enum {
    get("Entrance")
}
pub fn gibdo_items() -> &'static Enum {
    get("GibdoRequirementItem")
}
pub fn shorten_cutscene_general() -> &'static Enum {
    get("ShortenCutsceneGeneral")
}
pub fn shorten_cutscene_boss_intro() -> &'static Enum {
    get("ShortenCutsceneBossIntro")
}
pub fn boss_key_mode() -> &'static Enum {
    get("BossKeyMode")
}
pub fn small_key_mode() -> &'static Enum {
    get("SmallKeyMode")
}
