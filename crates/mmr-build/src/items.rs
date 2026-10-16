//! Item and location data for the builder (the old `ItemExtensions` and
//! `ItemUtils`), read from the dumped `Item` enum attributes.

use crate::enums::{self, Attribute, Enum};
use crate::plan::Plan;
use serde_json::Value;

pub fn table() -> &'static Enum {
    enums::items()
}

/// The enum value (order matters for some ranges).
pub fn value(item: &str) -> Option<i64> {
    table().get(item).map(|v| v.value)
}

pub fn name_of_value(value: i64) -> Option<&'static str> {
    table().values.iter().find(|v| v.value == value).map(|v| v.name.as_str())
}

fn attr(item: &str, kind: &str) -> Option<&'static Attribute> {
    let t = table();
    t.get(item)?.attributes.iter().find(|a| a.kind == kind)
}

fn attrs<'a>(item: &'a str, kind: &'a str) -> impl Iterator<Item = &'static Attribute> + 'a {
    table().get(item).into_iter().flat_map(move |v| v.attributes.iter().filter(move |a| a.kind == kind))
}

fn has(item: &str, kind: &str) -> bool {
    attr(item, kind).is_some()
}

fn num(v: &Value) -> i64 {
    v.as_i64().unwrap_or(0)
}

pub fn get_item_index(item: &str) -> Option<u16> {
    attr(item, "GetItemIndex").map(|a| num(a.get("Index")) as u16)
}

pub fn collectable_indices(item: &str) -> Vec<u16> {
    attrs(item, "CollectableIndex").map(|a| num(a.get("Index")) as u16).collect()
}

pub fn is_nullable(item: &str) -> bool {
    has(item, "NullableItem")
}

pub fn bottle_indices(item: &str) -> Option<Vec<i64>> {
    attr(item, "GetBottleItemIndices").map(|a| a.get("Indices").as_array().map(|l| l.iter().map(num).collect()).unwrap_or_default())
}

pub fn name(item: &str) -> Option<&'static str> {
    attr(item, "ItemName").and_then(|a| a.get("Name").as_str())
}

pub fn main_location(item: &str) -> Option<&'static str> {
    attr(item, "MainLocation").and_then(|a| a.get("Location").as_str())
}

pub fn item_category(item: &str) -> Option<&'static str> {
    attr(item, "ItemPool").and_then(|a| a.get("ItemCategory").as_str())
}

pub fn classic_category(item: &str) -> Option<&'static str> {
    attr(item, "ItemPool").and_then(|a| a.get("ClassicCategory").as_str())
}

pub fn chest_type(item: &str) -> Option<&'static str> {
    attr(item, "ChestType").and_then(|a| a.get("Type").as_str())
}

pub fn is_downgradable(item: &str) -> bool {
    has(item, "Downgradable")
}

pub fn is_rupee_repeatable(item: &str) -> bool {
    has(item, "RupeeRepeatable")
}

pub fn is_exclusive(item: &str) -> bool {
    has(item, "ExclusiveItem")
}

pub fn is_shop_model_visible(item: &str) -> bool {
    has(item, "ShopModelVisible")
}

/// Evaluate a settings condition from an attribute's constructor arguments
/// (`BaseSettingsConditionAttribute`): none means always; `(setting, value,
/// isEqual)` compares; `(setting, flagValue: int, hasFlag)` tests a flag.
pub fn condition(plan: &Plan, args: &[Value]) -> bool {
    let Some(key) = args.first().and_then(Value::as_str) else { return true };
    let expected = args.get(2).and_then(Value::as_bool).unwrap_or(true);
    match args.get(1) {
        Some(Value::Number(n)) => {
            let flags = settings_flags(plan, key);
            ((flags & n.as_i64().unwrap_or(0)) != 0) == expected
        }
        Some(v) => (plan.setting(key) == v) == expected,
        None => true,
    }
}

/// Numeric value of a flags setting.
pub fn settings_flags(plan: &Plan, key: &str) -> i64 {
    plan.enum_value(key)
}

/// The Overwritable condition starts after its slot argument.
fn condition_after(plan: &Plan, a: &Attribute, skip: usize) -> bool {
    condition(plan, a.args.get(skip..).unwrap_or(&[]))
}

pub fn is_temporary(plan: &Plan, item: &str) -> bool {
    attr(item, "Temporary").is_some_and(|a| condition(plan, &a.args))
}

pub fn is_repeatable(plan: &Plan, item: &str) -> bool {
    let temporary_or_cant_be = attr(item, "Temporary").is_none_or(|a| condition(plan, &a.args));
    has(item, "Repeatable") && temporary_or_cant_be
}

pub fn is_returnable(plan: &Plan, item: &str) -> bool {
    attr(item, "Returnable").is_some_and(|a| condition(plan, &a.args))
}

pub fn is_model_visible(plan: &Plan, item: &str) -> bool {
    attr(item, "ModelVisible").is_some_and(|a| condition(plan, &a.args))
}

pub fn is_text_visible(plan: &Plan, item: &str) -> bool {
    attrs(item, "TextVisible").any(|a| condition(plan, &a.args))
}

/// The slot an item can be overwritten in, if any.
pub fn overwritable_slot(plan: &Plan, item: &str) -> Option<&'static str> {
    let a = attr(item, "Overwritable")?;
    condition_after(plan, a, 1).then(|| a.args[0].as_str().unwrap_or("None"))
}

/// A get-item table entry (`GetItemEntry`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GetItemEntry {
    pub item_gained: u8,
    pub flag: u8,
    pub index: u8,
    pub kind: u8,
    pub message: u16,
    pub object: u16,
}

impl GetItemEntry {
    pub fn read(d: &[u8]) -> Self {
        GetItemEntry {
            item_gained: d[0],
            flag: d[1],
            index: d[2],
            kind: d[3],
            message: u16::from_be_bytes([d[4], d[5]]),
            object: u16::from_be_bytes([d[6], d[7]]),
        }
    }
    pub fn to_bytes(self) -> [u8; 8] {
        let [m1, m2] = self.message.to_be_bytes();
        let [o1, o2] = self.object.to_be_bytes();
        [self.item_gained, self.flag, self.index, self.kind, m1, m2, o1, o2]
    }
}

/// For items that aren't in the vanilla get-item table (traps, frogs...).
pub fn exclusive_entry(item: &str) -> Option<GetItemEntry> {
    let e = attr(item, "ExclusiveItem")?;
    let g = attr(item, "ExclusiveItemGraphic")?;
    let m = attr(item, "ExclusiveItemMessage")?;
    Some(GetItemEntry {
        item_gained: num(e.get("Item")) as u8,
        flag: num(e.get("Flags")) as u8,
        index: num(g.get("Graphic")) as u8,
        kind: num(e.get("Type")) as u8,
        message: num(m.get("Id")) as u16,
        object: num(g.get("Object")) as u16,
    })
}

pub fn exclusive_message(item: &str) -> Option<(u16, &'static str)> {
    let m = attr(item, "ExclusiveItemMessage")?;
    Some((num(m.get("Id")) as u16, m.get("Message").as_str()?))
}

/// Chest data for a location: the actor variable addresses and how the
/// chest appears.
pub fn chest(item: &str) -> Option<(Vec<u32>, i64)> {
    let a = attr(item, "Chest")?;
    let addresses = a.get("Addresses").as_array()?.iter().map(|v| num(v) as u32).collect();
    let appearance = match a.get("Type").as_str()? {
        "Normal" => 0,
        "Invisible" => 1,
        "AppearsClear" => 2,
        "AppearsSwitch" => 3,
        other => panic!("unknown chest appearance {other}"),
    };
    Some((addresses, appearance))
}

/// Chest type numbers (`ChestTypeAttribute.ChestType`).
pub fn chest_type_value(name: &str) -> u8 {
    match name {
        "SmallWooden" => 0,
        "SmallGold" => 1,
        "LargeGold" => 2,
        "BossKey" => 3,
        other => panic!("unknown chest type {other}"),
    }
}

pub fn is_grotto_chest(item: &str) -> bool {
    has(item, "GrottoChest")
}

/// The location a grotto chest's region refers to.
pub fn region_reference(item: &str) -> Option<&'static str> {
    attr(item, "Region").and_then(|a| a.get("Reference").as_str())
}

/// Patches a location applies when its item changes (`HackContent`):
/// (resource name, apply only if the item is different).
pub fn hack_contents(item: &str) -> Vec<(&'static str, bool)> {
    attrs(item, "HackContent")
        .filter_map(|a| Some((a.args.first()?.as_str()?, a.args.get(1).and_then(Value::as_bool).unwrap_or(true))))
        .collect()
}

/// Shop inventory slots selling this location's item: (address, keeper).
pub fn shop_inventories(item: &str) -> Vec<(u32, &'static str)> {
    attrs(item, "ShopInventory").map(|a| (num(a.get("ShopItemAddress")) as u32, a.get("Keeper").as_str().unwrap_or(""))).collect()
}

/// Starting item writes: (address, value, add instead of or).
pub fn starting_item_writes(item: &str) -> Vec<(u32, u8, bool)> {
    attrs(item, "StartingItem")
        .map(|a| (num(a.get("Address")) as u32, num(a.get("Value")) as u8, a.get("IsAdditional").as_bool().unwrap_or(false)))
        .collect()
}

pub fn starting_tingle_map(item: &str) -> Option<&'static str> {
    attr(item, "StartingTingleMap").and_then(|a| a.get("TingleMap").as_str())
}

pub fn has_starting_item_skull(item: &str) -> bool {
    has(item, "StartingItemSkull")
}

pub fn starting_item_ids(item: &str) -> Vec<u8> {
    attrs(item, "StartingItemId").map(|a| num(a.get("ItemId")) as u8).collect()
}

/// `StartingItemAttribute.Value` of the first one (mask draw config).
pub fn starting_item_value(item: &str) -> Option<u8> {
    attr(item, "StartingItem").map(|a| num(a.get("Value")) as u8)
}

/// Items with this enum value range, inclusive (the old `>=`/`<=` checks).
pub fn in_range(item: &str, first: &str, last: &str) -> bool {
    match (value(item), value(first), value(last)) {
        (Some(v), Some(a), Some(b)) => v >= a && v <= b,
        _ => false,
    }
}

/// `count` consecutive enum values starting at `first`.
pub fn range_from(first: &str, count: i64) -> Vec<&'static str> {
    let start = value(first).expect("range start exists");
    (start..start + count).filter_map(name_of_value).collect()
}

pub fn is_bottle_catch_content(item: &str) -> bool {
    in_range(item, "BottleCatchFairy", "BottleCatchMushroom")
}

/// Shop descriptions for an item (`ShopTextAttribute`).
#[derive(Debug, Clone, Default)]
pub struct ShopText {
    pub default: String,
    pub by_keeper: Vec<(&'static str, String)>,
    pub is_multiple: bool,
    pub is_definite: bool,
}

impl ShopText {
    pub fn for_keeper(&self, keeper: &str) -> Option<&str> {
        self.by_keeper.iter().find(|(k, _)| *k == keeper).map(|(_, t)| t.as_str())
    }
}

pub fn shop_text(item: &str) -> Option<ShopText> {
    let a = attr(item, "ShopText")?;
    const KEEPERS: [&str; 8] = ["WitchShop", "TradingPostMain", "TradingPostPartTimer", "CuriosityShop", "BombShop", "ZoraShop", "GoronShop", "GoronShopSpring"];
    Some(ShopText {
        default: a.get("Default").as_str().unwrap_or("").to_string(),
        by_keeper: KEEPERS.iter().filter_map(|k| Some((*k, a.get(k).as_str()?.to_string()))).collect(),
        is_multiple: a.get("IsMultiple").as_bool().unwrap_or(false),
        is_definite: a.get("IsDefinite").as_bool().unwrap_or(false),
    })
}

/// Whether an "item" is really an entrance (entrance randomization).
pub fn is_entrance(item: &str) -> bool {
    has(item, "Entrance")
}

/// The entrance (and its pair) an entrance item stands for.
pub fn entrances(item: &str) -> Vec<String> {
    let Some(a) = attr(item, "Entrance") else { return Vec::new() };
    [a.get("Entrance"), a.get("Pair")].iter().filter_map(|v| v.as_str().map(String::from)).collect()
}

/// Actors that act as an entrance's exit: (scene, setup, room, actor).
pub fn exit_actor_params(entrance: &str) -> Vec<(u8, usize, usize, usize)> {
    enums::entrances()
        .attrs(entrance, "ExitActorParams")
        .map(|a| (num(a.get("SceneId")) as u8, num(a.get("SetupIndex")) as usize, num(a.get("RoomIndex")) as usize, num(a.get("ActorIndex")) as usize))
        .collect()
}

/// `MessageUtils.NumberToWords`.
pub fn number_to_words(number: i64) -> String {
    const UNITS: [&str; 20] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen",
    ];
    const TENS: [&str; 10] = ["zero", "ten", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
    if number == 0 {
        return "zero".into();
    }
    if number < 0 {
        return format!("minus {}", number_to_words(-number));
    }
    let mut n = number;
    let mut words = String::new();
    for (size, name) in [(1_000_000, "million"), (1000, "thousand"), (100, "hundred")] {
        if n / size > 0 {
            words += &format!("{} {name} ", number_to_words(n / size));
            n %= size;
        }
    }
    if n > 0 {
        if !words.is_empty() {
            words += "and ";
        }
        if n < 20 {
            words += UNITS[n as usize];
        } else {
            words += TENS[(n / 10) as usize];
            if n % 10 > 0 {
                words += &format!("-{}", UNITS[(n % 10) as usize]);
            }
        }
    }
    words
}

/// Display name of an entrance item (`Item.Entrance()`).
pub fn entrance_name(item: &str) -> Option<&'static str> {
    attr(item, "EntranceName").and_then(|a| a.get("Name").as_str())
}
