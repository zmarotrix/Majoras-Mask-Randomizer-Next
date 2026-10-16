//! Gossip stone and Garo hints for MMR Next's own seeds (the old
//! `Randomizer.MakeGossipQuotes` and `MessageUtils.MakeGossipQuotes`).
//!
//! Styles: Random (any item, sometimes a lie), Relevant (stones near a
//! check tend to hint it) and Competitive (Way of the Hero / foolish region
//! hints and hints for checks that are often skipped, each told by two
//! stones, never by one that needs the hinted check to reach).

use crate::enums;
use crate::items;
use crate::message::{units, wrap};
use crate::plan::{Message, PlacedItem, Plan};
use crate::stages::article;
use mmr_randomizer::importance::Importance;
use mmr_randomizer::rng::{Rng, Seed, Stream};
use serde_json::Value;
use std::collections::BTreeMap;

const WHITE: char = '\x00';
const YELLOW: char = '\x04';
const PINK: char = '\x06';
const SILVER: char = '\x07';
const ORANGE: char = '\x08';
const HEADER: [u8; 11] = [2, 0, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
const START_SENTENCES: [&str; 5] = ["They say", "I hear", "It seems", "Apparently,", "It appears"];
const MID_SENTENCES: [&str; 6] = ["leads to", "yields", "brings", "holds", "conceals", "possesses"];
const CLOCK_TOWN_REGIONS: [&str; 7] = ["NorthClockTown", "SouthClockTown", "EastClockTown", "WestClockTown", "LaundryPool", "StockPotInn", "ClockTowerRoof"];
const JUNK_MESSAGES: [&str; 14] = [
    "\u{1E}\u{69}\u{4F}They say that Jimmie1717's mod\u{11}lottery is \u{01}RIGGED!\u{00}\u{BF}",
    "\u{1E}\u{69}\u{4F}Real ZELDA players use HOLD targeting!\u{BF}",
    "\u{1E}\u{69}\u{4F}They say items are random...\u{BF}",
    "\u{1E}\u{69}\u{4F}They say the \u{05}blue dog\u{00} shall prevail...\u{BF}",
    "\u{1E}\u{69}\u{4F}My body craves for the touch of\u{11}\u{01}mashed potatoes\u{00}...\u{BF}",
    "\u{1E}\u{69}\u{2B}Dear Mario, please come to the \u{11}castle. I've baked a cake for you.\u{11}Yours truly, Princess Toadstool\u{11}\u{06}Peach\u{00}\u{BF}",
    "\u{1E}\u{69}\u{56}I overheard something useful:\u{11}\u{DF}\u{BF}",
    "\u{1E}\u{69}\u{56}I overheard something useful:\u{11}\u{D6}\u{BF}",
    "\u{1E}\u{69}\u{4F}They say the best button for bombchus\u{11}is \u{04}\u{B7}\u{00}...\u{BF}",
    "\u{1E}\u{69}\u{4F}They say the key to victory is\u{11}beating the game...\u{BF}",
    "\u{1E}\u{38}\u{0B}They say a certain player once stole\u{11}their items back from Takkuri...\u{BF}",
    "\u{1E}\u{69}\u{4F}They say wearing the \u{01}Bremen Mask\u{00}\u{11}increases your chances of beating the\u{11}Gorman bros...\u{BF}",
    "\u{1E}\u{69}\u{6F}Use the boost to get through!\u{BF}",
    "\u{1E}\u{69}\u{4F}They say the \u{04}gold dog\u{00} cheats...\u{BF}",
];

/// A gossip stone (or Garo) message slot (`GossipQuote`).
#[derive(Debug, Clone)]
struct Quote {
    name: String,
    id: u16,
    /// (is the restriction an item, the item or location, force a clear hint)
    restrict: Vec<(bool, String, bool)>,
    /// The logic entry for reaching the stone.
    stone: Option<String>,
}

impl Quote {
    fn moon(&self) -> bool {
        self.name.starts_with("Moon")
    }
    fn garo(&self) -> bool {
        self.name.starts_with("Garo")
    }
}

fn quotes() -> Vec<Quote> {
    let mut v: Vec<Quote> = enums::get("GossipQuote")
        .values
        .iter()
        .map(|q| Quote {
            name: q.name.clone(),
            id: q.value as u16,
            restrict: q
                .attributes
                .iter()
                .filter(|a| a.kind == "GossipRestrict")
                .map(|a| (a.get("Type").as_str() == Some("Item"), a.get("Item").as_str().unwrap_or("").to_string(), a.get("ForceClear").as_bool().unwrap_or(false)))
                .collect(),
            stone: q.attributes.iter().find(|a| a.kind == "GossipStone").and_then(|a| a.get("Item").as_str().map(String::from)),
        })
        .collect();
    v.sort_by_key(|q| q.id);
    v
}

/// The logic entries for every stone, for [`Importance::required_for`].
pub fn stone_entries() -> Vec<String> {
    quotes().into_iter().filter_map(|q| q.stone).collect()
}

/// Groups of checks hinted together, e.g. "Beaver Races".
fn combinable() -> &'static BTreeMap<String, Vec<String>> {
    static T: std::sync::OnceLock<BTreeMap<String, Vec<String>>> = std::sync::OnceLock::new();
    T.get_or_init(|| serde_json::from_str(include_str!("../../../data/rom/combinable_hints.json").trim_start_matches('\u{FEFF}')).expect("bundled hint groups"))
}

fn combine_info(location: &str) -> Option<(&'static String, &'static Vec<String>)> {
    combinable().iter().find(|(_, v)| v.iter().any(|l| l == location))
}

/// Everything hint making needs about the seed.
struct Ctx<'a> {
    plan: &'a Plan,
    imp: &'a Importance,
    is_junk: &'a dyn Fn(&str) -> bool,
    /// Plan items in the game's item order (the old `ItemList`).
    list: Vec<&'a PlacedItem>,
}

impl Ctx<'_> {
    fn obj(&self, id: &str) -> Option<&PlacedItem> {
        self.list.iter().copied().find(|p| p.id == id)
    }
    fn at(&self, location: &str) -> Option<&PlacedItem> {
        self.list.iter().copied().find(|p| p.location.as_deref() == Some(location))
    }
    fn region_of(&self, location: &str) -> Option<String> {
        let a = enums::items().attr(location, "Region")?;
        if let Some(r) = a.get("Region").as_str() {
            return Some(r.to_string());
        }
        let reference = a.get("Reference").as_str()?;
        let target = self.obj(reference).and_then(|io| io.location.clone()).unwrap_or_else(|| reference.to_string());
        self.region_of(&target)
    }
    /// A location's name, with the entrance it's behind (`Item.Location`).
    fn location_name(&self, location: &str) -> String {
        let name = enums::items().attr(location, "LocationName").and_then(|a| a.get("Name").as_str()).unwrap_or(location).to_string();
        let Some(a) = enums::items().attr(location, "Region") else { return name };
        if a.get("PassToLocation").as_bool() != Some(true) {
            return name;
        }
        let Some(reference) = a.get("Reference").as_str() else { return name };
        let at = self.obj(reference).and_then(|io| io.location.clone()).unwrap_or_else(|| reference.to_string());
        let exit = enums::items().attr(&at, "ExitName").and_then(|a| a.get("Name").as_str()).unwrap_or("");
        format!("{exit} {name}")
    }
    fn junk_location(&self, location: &str) -> bool {
        self.plan.junk_locations.iter().any(|l| l == location)
    }
    fn setting_list(&self, key: &str) -> Vec<String> {
        self.plan.setting(key).as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default()
    }
    fn is_region_restricted(&self, item: &str) -> bool {
        let p = self.plan;
        let category = items::item_category(item);
        let temples = |k: &str| p.has(k, "KeepWithinTemples");
        let first_fairy = enums::items().values.iter().position(|v| v.name == "CollectibleStrayFairyWoodfall1").unwrap_or(0);
        let pos = enums::items().values.iter().position(|v| v.name == item).unwrap_or(usize::MAX);
        let dungeon_fairy = (first_fairy..first_fairy + 60).contains(&pos);
        let key = ["Woodfall", "Snowhead", "GreatBay", "StoneTower"].iter().any(|t| item.starts_with(&format!("Item{t}Key")));
        let boss_key = ["Woodfall", "Snowhead", "GreatBay", "StoneTower"].iter().any(|t| item == format!("Item{t}BossKey"));
        let navigation = category == Some("Navigation") && ["Woodfall", "Snowhead", "GreatBay", "StoneTower"].iter().any(|t| item.starts_with(&format!("Item{t}")));
        (temples("SmallKeyMode") && key)
            || (temples("BossKeyMode") && boss_key)
            || (temples("StrayFairyMode") && dungeon_fairy)
            || (category == Some("BossRemains") && (temples("BossRemainsMode") || p.has("BossRemainsMode", "GreatFairyRewards")))
            || (temples("DungeonNavigationMode") && navigation)
    }
    /// Hinted elsewhere (by NPCs, the Giants, Tatl).
    fn is_item_hinted(&self, item: &str) -> bool {
        let p = self.plan;
        (p.flag("UpdateNPCText") && ["CollectibleStrayFairyClockTown", "ItemPowderKeg", "ItemBottleGoronRace", "ItemBottleBeavers", "MaskGaro", "SongTime", "SongSoaring"].contains(&item))
            || (p.flag("OathHint") && item == "SongOath")
            || (p.flag("RemainsHint") && items::item_category(item) == Some("BossRemains"))
    }
    fn can_be_required(item: &str) -> bool {
        !matches!(items::item_category(item), Some("StrayFairies" | "SkulltulaTokens" | "HeartContainers" | "PiecesOfHeart" | "RecoveryHearts" | "NotebookEntries"))
    }
    fn is_required(&self, item: &str, location: &str, anything: bool) -> bool {
        if anything && self.imp.important.contains(location) && is_song(item) && self.imp.required.contains(location) {
            return true;
        }
        (anything || (Self::can_be_required(item) && !self.is_item_hinted(item))) && self.imp.required.contains(location)
    }
    fn is_important(&self, location: &str) -> bool {
        self.imp.important.contains(location)
    }
    fn item_name(&self, item: &str) -> String {
        if self.plan.flag("ProgressiveUpgrades") {
            let groups: [(&[&str], &str); 6] = [
                (&["StartingSword", "UpgradeRazorSword", "UpgradeGildedSword"], "Sword Upgrade"),
                (&["FairyMagic", "FairyDoubleMagic"], "Magic Power Upgrade"),
                (&["UpgradeAdultWallet", "UpgradeGiantWallet", "UpgradeRoyalWallet"], "Wallet Upgrade"),
                (&["ItemBombBag", "UpgradeBigBombBag", "UpgradeBiggestBombBag"], "Bomb Bag Upgrade"),
                (&["ItemBow", "UpgradeBigQuiver", "UpgradeBiggestQuiver"], "Bow Upgrade"),
                (&["SongLullaby", "SongLullabyIntro"], "Goron Lullaby Upgrade"),
            ];
            if let Some((_, n)) = groups.iter().find(|(g, _)| g.contains(&item)) {
                return n.to_string();
            }
        }
        items::name(item).unwrap_or(item).to_string()
    }
}

/// A competitive hint's condition (after its priority): optionally an item
/// category that must (not) be randomized, then a settings condition.
fn competitive_condition(plan: &Plan, args: &[Value]) -> bool {
    let is_category = |v: &Value| v.as_str().is_some_and(|c| mmr_data::game().items.iter().any(|i| i.item_category.as_deref() == Some(c)));
    if args.len() >= 2 && is_category(&args[0]) && args[1].is_boolean() {
        let category = args[0].as_str().unwrap();
        let randomized = plan.custom_item_list.iter().any(|i| items::item_category(i) == Some(category));
        return randomized == args[1].as_bool().unwrap() && items::condition(plan, &args[2..]);
    }
    items::condition(plan, args)
}

fn is_song(item: &str) -> bool {
    mmr_data::game().item(item).is_some_and(|i| i.song)
}

fn item_hints(item: &str) -> Vec<String> {
    mmr_data::game().item(item).map(|i| i.gossip_item_hints.clone()).unwrap_or_default()
}

fn location_hints(location: &str) -> Vec<String> {
    mmr_data::game().item(location).map(|i| i.gossip_location_hints.clone()).unwrap_or_default()
}

fn pick<T: Clone>(rng: &mut Rng, list: &[T]) -> Option<T> {
    rng.choose(list).cloned()
}

fn sfx(id: u16) -> String {
    format!("{}{}", char::from((id >> 8) as u8), char::from(id as u8))
}

fn gossip_quote(rng: &mut Rng, sound: u16, location: &str, items: &[String]) -> String {
    let start = START_SENTENCES[rng.below(START_SENTENCES.len())];
    let mid = MID_SENTENCES[rng.below(MID_SENTENCES.len())];
    format!("\u{1E}{}{start} \u{01}{location}\u{00} {mid} {}...\u{BF}", sfx(sound), items.join(" then "))
}

#[derive(Clone, Copy, PartialEq)]
enum RegionHint {
    NoneRequired,
    SomeRequired,
    OnlyImportantSong,
}

fn region_hint(rng: &mut Rng, region: &str, kind: RegionHint, counted: bool, important: usize) -> String {
    let start = START_SENTENCES[rng.below(START_SENTENCES.len())];
    let name = crate::stages::regions::region_name(region).unwrap_or_else(|| region.to_string());
    let (mid, color, what) = if counted {
        let (what, color) = match kind {
            RegionHint::NoneRequired => ("zero important items".to_string(), SILVER),
            RegionHint::SomeRequired => (format!("{} important item{}", items::number_to_words(important as i64), if important == 1 { "" } else { "s" }), YELLOW),
            RegionHint::OnlyImportantSong => ("one important song".to_string(), ORANGE),
        };
        ("has", color, format!("{color}{what}{WHITE}"))
    } else {
        let (what, color) = match kind {
            RegionHint::NoneRequired => ("a foolish choice", SILVER),
            RegionHint::SomeRequired => ("on the Way of the Hero", YELLOW),
            RegionHint::OnlyImportantSong => ("foolish except for its song", ORANGE),
        };
        ("is", color, what.to_string())
    };
    format!("\u{1E}{}{start} {color}{name}{WHITE} {mid} {what}...\u{BF}", sfx(0x690C))
}

/// A hint about one item (or a group of checks hinted together):
/// (vague message, clear message, the items it covers).
fn item_hint(ctx: &Ctx, rng: &mut Rng, item: &PlacedItem, style: &str, combine_with: &[&PlacedItem], hintable: &[&PlacedItem], indicate: Option<&dyn Fn(&PlacedItem) -> bool>) -> (Option<String>, Option<String>, Vec<String>) {
    let p = ctx.plan;
    let loc = item.location.clone().unwrap_or_default();
    let describe = |io: &PlacedItem| -> String {
        let progressive = p.flag("ProgressiveUpgrades") && mmr_data::game().item(&io.item).is_some_and(|i| i.has_flag("Progressive"));
        let art = if progressive { "a ".to_string() } else { article(&io.item, None) };
        let mut color = PINK;
        let mut importance = String::new();
        if p.flag("HintsIndicateImportance") && indicate.is_some_and(|f| f(io)) {
            let l = io.location.clone().unwrap_or_default();
            let required = ctx.is_required(&io.item, &l, true);
            if !p.logically_junk_items.iter().any(|j| *j == io.item) {
                importance = if required { " (required)" } else { " (not required)" }.into();
            }
            color = if required { YELLOW } else { SILVER };
        }
        format!("{art}{color}{}{WHITE}{importance}", ctx.item_name(&io.item))
    };
    let mut names = vec![describe(item)];
    let mut location_name = ctx.location_name(&loc);
    let mut combined: Vec<&PlacedItem> = Vec::new();
    if let Some((group, locations)) = combine_info(&loc) {
        combined = combine_with.iter().copied().filter(|io| io.location.as_deref().is_some_and(|l| locations.contains(&l.to_string()))).collect();
        if !combined.is_empty() {
            if !combined.iter().any(|io| io.id == item.id) {
                combined.push(item);
            }
            combined.sort_by_key(|io| locations.iter().position(|l| Some(l.as_str()) == io.location.as_deref()));
            location_name = group.clone();
            names = combined.iter().map(|io| describe(io)).collect();
        }
    }
    if combined.is_empty() {
        combined.push(item);
    }
    let clear = Some(gossip_quote(rng, 0x690C, &location_name, &names));

    let mut sound = 0x690C;
    let mut names: Vec<String> = Vec::new();
    let mut location_name = None;
    if let Some(m) = &item.mimic {
        // A disguised trap always gets a lie (unless hints are clear).
        sound = 0x690A;
        names.extend(pick(rng, &item_hints(&m.item)));
        location_name = pick(rng, &location_hints(&loc));
    } else if style != "Random" || rng.below(100) >= 5 {
        names.extend(pick(rng, &item_hints(&item.item)));
        location_name = pick(rng, &location_hints(&loc));
    } else if rng.below(2) == 0 {
        // A lie: right item, wrong place.
        sound = 0x690A;
        names.extend(pick(rng, &item_hints(&item.item)));
        let other = hintable[rng.below(hintable.len())].location.clone().unwrap_or_default();
        location_name = pick(rng, &location_hints(&other));
    }
    if let Some(first) = names.first_mut() {
        *first = format!("{PINK}{first}{WHITE}");
    }
    let message = match (names.is_empty(), location_name) {
        (false, Some(l)) => Some(gossip_quote(rng, sound, &l, &names)),
        _ => None,
    };
    (message, clear, combined.iter().map(|io| io.id.clone()).collect())
}

struct CompetitiveHint {
    message: String,
    alternate: Option<String>,
    allowed: Vec<u16>,
    is_item: bool,
    info: HintInfo,
}

/// What a hint is about, so the tracker can mark the map once it's read.
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HintInfo {
    /// The message id (which stone).
    pub id: u16,
    /// "woth", "foolish", "song" (foolish but for a song), "item" or "junk".
    pub kind: String,
    /// The region a region hint is about.
    pub region: Option<String>,
    /// Checks a hint names outright (clear item hints only).
    pub locations: Vec<String>,
}

/// Make every hint the settings ask for.
pub fn make_gossip_quotes(plan: &Plan, imp: &Importance, is_junk: &dyn Fn(&str) -> bool, seed: u64) -> (Vec<Message>, Vec<HintInfo>) {
    let mut list: Vec<&PlacedItem> = plan.items.iter().collect();
    let order = |id: &str| enums::items().values.iter().position(|v| v.name == id).unwrap_or(usize::MAX);
    list.sort_by_key(|p| order(&p.id));
    let ctx = Ctx { plan, imp, is_junk, list };
    let mut rng = Rng::new(Seed(seed), Stream::Hints, 0);
    let all = quotes();
    let gossip_style = plan.choice("GossipHintStyle").to_string();
    let garo_style = plan.choice("GaroHintStyle").to_string();
    let number = |k: &str, default: usize| plan.setting(k).as_u64().map_or(default, |n| n as usize);
    let mut hinted_regions: Vec<String> = Vec::new();
    let mut hinted_items: Vec<String> = Vec::new();
    let mut out = Vec::new();
    let mut infos = Vec::new();
    let mut take = |out: &mut Vec<Message>, (m, i): (Vec<Message>, Vec<HintInfo>)| {
        out.extend(m);
        infos.extend(i);
    };

    if gossip_style != "Default" && !gossip_style.is_empty() {
        let moon: Vec<&Quote> = all.iter().filter(|q| q.moon()).collect();
        take(&mut out, make(&ctx, &mut rng, &moon, "Relevant", 0, 0, 0, &mut hinted_regions, &mut hinted_items));
    }
    let (req_garo, non_garo, ct_garo) = (number("OverrideNumberOfRequiredGaroHints", 2), number("OverrideNumberOfNonRequiredGaroHints", 2), number("OverrideMaxNumberOfClockTownGaroHints", 2));
    let (req_gossip, non_gossip, ct_gossip) =
        (number("OverrideNumberOfRequiredGossipHints", if plan.flag("AddSongs") { 4 } else { 3 }), number("OverrideNumberOfNonRequiredGossipHints", 3), number("OverrideMaxNumberOfClockTownGossipHints", 2));
    if garo_style != gossip_style || !plan.flag("MixGossipAndGaroHints") {
        if gossip_style != "Default" && !gossip_style.is_empty() {
            let q: Vec<&Quote> = all.iter().filter(|q| !q.moon() && !q.garo()).collect();
            take(&mut out, make(&ctx, &mut rng, &q, &gossip_style, req_gossip, non_gossip, ct_gossip, &mut hinted_regions, &mut hinted_items));
        }
        if garo_style != "Default" && !garo_style.is_empty() {
            let q: Vec<&Quote> = all.iter().filter(|q| q.garo()).collect();
            take(&mut out, make(&ctx, &mut rng, &q, &garo_style, req_garo, non_garo, ct_garo, &mut hinted_regions, &mut hinted_items));
        }
    } else if gossip_style != "Default" && !gossip_style.is_empty() {
        let q: Vec<&Quote> = all.iter().filter(|q| !q.moon()).collect();
        take(&mut out, make(&ctx, &mut rng, &q, &gossip_style, req_garo + req_gossip, non_garo + non_gossip, ct_garo + ct_gossip, &mut hinted_regions, &mut hinted_items));
    }
    (out, infos)
}

#[allow(clippy::too_many_arguments)]
fn make(ctx: &Ctx, rng: &mut Rng, quotes: &[&Quote], style: &str, required_hints: usize, non_required_hints: usize, max_clock_town: usize, hinted_regions: &mut Vec<String>, hinted_items: &mut Vec<String>) -> (Vec<Message>, Vec<HintInfo>) {
    let plan = ctx.plan;
    let mut prevent = vec!["BottleCatch".to_string(), "Misc".to_string()];
    let victory: Vec<String> = ctx.setting_list("VictoryMode");
    let remains_needed = ["FourBossRemains", "ThreeBossRemains", "TwoBossRemains", "OneBossRemains"].iter().position(|f| victory.iter().any(|v| v == f)).map_or(0, |i| 4 - i as i64);
    if victory.iter().all(|v| ["FourBossRemains", "ThreeBossRemains", "TwoBossRemains", "OneBossRemains", "DirectToCredits", "CantFightMajora"].contains(&v.as_str())) && plan.number("RequiredBossRemains") >= remains_needed {
        prevent.push("TheMoon".into());
    }
    let priorities: Option<Vec<Vec<String>>> = plan.setting("OverrideHintPriorities").as_array().map(|a| a.iter().map(|t| t.as_array().map(|x| x.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default()).collect());

    // What can be hinted, and (for competitive) what each region holds.
    let mut hintable: Vec<&PlacedItem> = Vec::new();
    let mut in_regions: BTreeMap<String, Vec<(&PlacedItem, String)>> = BTreeMap::new();
    let mut region_order: Vec<String> = Vec::new();
    for io in &ctx.list {
        if items::is_entrance(&io.id) {
            continue;
        }
        let Some(loc) = io.location.as_deref() else { continue };
        let main = items::main_location(&io.id);
        let main_randomized = main.and_then(|m| ctx.obj(m)).is_some_and(|m| m.is_randomized);
        if (!io.is_randomized || ctx.region_of(loc).is_none()) && (main.is_none() || !main_randomized) {
            continue;
        }
        if loc == "UpgradeRoyalWallet" {
            continue;
        }
        let item = match main {
            Some(m) => match ctx.at(m) {
                Some(x) => x,
                None => continue,
            },
            None => *io,
        };
        if main.is_none() && ["MaskDeku", "SongHealing", "StartingSword", "StartingShield", "StartingHeartContainer1", "StartingHeartContainer2"].contains(&loc) {
            continue;
        }
        if ctx.is_region_restricted(&item.item) {
            continue;
        }
        let item_loc = item.location.clone().unwrap_or_default();
        if style == "Competitive" {
            let for_importance = if main.is_some() { io.id.clone() } else { loc.to_string() };
            if let Some(region) = ctx.region_of(&for_importance) {
                if !prevent.contains(&region) && !ctx.junk_location(&item_loc) && !ctx.junk_location(&for_importance) {
                    if !region_order.contains(&region) {
                        region_order.push(region.clone());
                    }
                    in_regions.entry(region).or_default().push((item, for_importance));
                }
            }
            if hinted_items.contains(&item.id) {
                continue;
            }
            match &priorities {
                Some(p) => {
                    if !p.iter().any(|t| t.contains(&item_loc)) {
                        continue;
                    }
                }
                None => {
                    let Some(a) = enums::items().attr(&item_loc, "GossipCompetitiveHint") else { continue };
                    if a.args.len() > 1 && !competitive_condition(plan, &a.args[1..]) {
                        continue;
                    }
                }
            }
        }
        if ctx.junk_location(&item_loc) {
            continue;
        }
        if !hintable.iter().any(|h| h.id == item.id) {
            hintable.push(item);
        }
    }

    let mut unused: Vec<&PlacedItem> = hintable.clone();
    let mut combine_with: Vec<&PlacedItem> = Vec::new();
    let mut competitive: Vec<CompetitiveHint> = Vec::new();
    let stone_requirements: BTreeMap<u16, Option<std::collections::BTreeSet<String>>> =
        quotes.iter().filter_map(|q| q.stone.as_ref().map(|s| (q.id, ctx.imp.required_for.get(s).cloned().flatten()))).collect();
    let allowed_for = |locations: &[String]| -> Vec<u16> {
        quotes.iter().filter(|q| stone_requirements.get(&q.id).is_some_and(|r| r.as_ref().is_some_and(|r| locations.iter().all(|l| !r.contains(l))))).map(|q| q.id).collect()
    };

    if style == "Competitive" {
        let unique = quotes.len() / 2;
        let location_hints = unique.saturating_sub(required_hints + non_required_hints);
        let priority = |io: &PlacedItem| -> i64 {
            let l = io.location.clone().unwrap_or_default();
            match &priorities {
                Some(p) => p.iter().position(|t| t.contains(&l)).map_or(-1, |i| i as i64),
                None => -enums::items().attr(&l, "GossipCompetitiveHint").and_then(|a| a.get("Priority").as_i64()).unwrap_or(0),
            }
        };
        // One per hint group, best priority first, shuffled within a priority.
        let mut groups: Vec<(String, &PlacedItem)> = Vec::new();
        for io in &hintable {
            let l = io.location.clone().unwrap_or_default();
            let key = combine_info(&l).map(|(n, _)| n.clone()).unwrap_or(l);
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some(g) => {
                    if priority(io) < priority(g.1) {
                        g.1 = io;
                    }
                }
                None => groups.push((key, io)),
            }
        }
        let mut by_priority: BTreeMap<i64, Vec<&PlacedItem>> = BTreeMap::new();
        for (_, io) in groups {
            by_priority.entry(priority(io)).or_default().push(io);
        }
        unused = Vec::new();
        for (_, mut tier) in by_priority {
            rng.shuffle(&mut tier);
            unused.extend(tier);
        }
        unused.truncate(location_hints);
        for io in &unused {
            if let Some((_, locations)) = io.location.as_deref().and_then(combine_info) {
                for l in locations {
                    if let Some(h) = hintable.iter().find(|h| h.location.as_deref() == Some(l.as_str())) {
                        if !unused.iter().any(|u| u.id == h.id) && !combine_with.iter().any(|c| c.id == h.id) {
                            combine_with.push(h);
                        }
                    }
                }
            }
        }
        let doubled: Vec<&PlacedItem> = unused.iter().chain(unused.iter()).copied().collect();
        let tiers: Option<Vec<i64>> = plan.setting("OverrideImportanceIndicatorTiers").as_array().map(|a| a.iter().filter_map(Value::as_i64).collect());
        let indicate = |io: &PlacedItem| match (&priorities, &tiers) {
            (Some(_), Some(t)) => t.contains(&priority(io)),
            _ => true,
        };
        for io in &doubled {
            let (message, clear, combined) = item_hint(ctx, rng, io, style, &combine_with, &hintable, Some(&indicate));
            let locations: Vec<String> = combined.iter().filter_map(|id| ctx.obj(id).and_then(|o| o.location.clone())).collect();
            let info = HintInfo { kind: "item".into(), locations: locations.clone(), ..Default::default() };
            competitive.push(CompetitiveHint { message: message.unwrap_or_default(), alternate: clear, allowed: allowed_for(&locations), is_item: true, info });
            hinted_items.extend(combined);
        }

        // Regions: required (Way of the Hero), foolish, foolish except a song.
        let mut important_regions: Vec<(String, Vec<(&PlacedItem, String)>)> = Vec::new();
        let mut non_important: Vec<(String, Vec<(&PlacedItem, String)>)> = Vec::new();
        let mut song_only: Vec<(String, Vec<(&PlacedItem, String)>)> = Vec::new();
        let mut clock_town: Vec<(String, Vec<(&PlacedItem, String)>)> = Vec::new();
        let mut region_important: BTreeMap<String, usize> = BTreeMap::new();
        for region in &region_order {
            let items_here = &in_regions[region];
            let required: Vec<(&PlacedItem, String)> = items_here
                .iter()
                .filter(|(io, l)| ctx.is_required(&io.item, l, false) && !doubled.iter().any(|u| u.id == io.id) && !combine_with.iter().any(|c| c.id == io.id))
                .cloned()
                .collect();
            let mut important: Vec<(&PlacedItem, String)> = items_here.iter().filter(|(_, l)| ctx.is_important(l)).cloned().collect();
            if plan.has("BossRemainsMode", "ShuffleOnly") {
                important.retain(|(io, _)| items::item_category(&io.item) != Some("BossRemains"));
            }
            region_important.insert(region.clone(), important.len());
            let all_songs = |v: &[(&PlacedItem, String)]| v.iter().all(|(io, _)| is_song(&io.item));
            let target = if required.is_empty() && !important.is_empty() {
                if !plan.flag("AddSongs") && all_songs(&important) && !all_songs(items_here) {
                    &mut song_only
                } else {
                    continue;
                }
            } else if required.is_empty() {
                &mut non_important
            } else if CLOCK_TOWN_REGIONS.contains(&region.as_str()) {
                &mut clock_town
            } else {
                &mut important_regions
            };
            target.push((region.clone(), required.clone()));
            if !plan.flag("AddSongs") && !required.is_empty() && all_songs(&required) && all_songs(&important) && !all_songs(items_here) {
                song_only.retain(|(r, _)| r != region);
                song_only.push((region.clone(), required));
            }
        }
        for _ in 0..non_required_hints {
            let options: Vec<(String, bool)> = non_important.iter().map(|(r, _)| (r.clone(), false)).chain(song_only.iter().map(|(r, _)| (r.clone(), true))).filter(|(r, _)| !hinted_regions.contains(r)).collect();
            if options.is_empty() {
                continue;
            }
            let weights: Vec<u32> = options.iter().map(|(r, _)| in_regions[r].len() as u32).collect();
            let (region, _) = options[rng.weighted(&weights)].clone();
            let kind = if let Some(i) = song_only.iter().position(|(r, _)| *r == region) {
                song_only.remove(i);
                RegionHint::OnlyImportantSong
            } else {
                non_important.retain(|(r, _)| *r != region);
                RegionHint::NoneRequired
            };
            for _ in 0..2 {
                let message = region_hint(rng, &region, kind, false, 0);
                let alternate = region_hint(rng, &region, kind, true, 0);
                let verdict = if kind == RegionHint::OnlyImportantSong { "song" } else { "foolish" };
                let info = HintInfo { kind: verdict.into(), region: Some(region.clone()), ..Default::default() };
                competitive.push(CompetitiveHint { message, alternate: Some(alternate), allowed: Vec::new(), is_item: false, info });
            }
            hinted_regions.push(region);
        }
        let mut chosen_clock_town = 0;
        for _ in 0..required_hints {
            let mut options: Vec<&(String, Vec<(&PlacedItem, String)>)> = important_regions.iter().collect();
            if chosen_clock_town < max_clock_town || options.is_empty() {
                options.extend(clock_town.iter());
            }
            options.retain(|(r, _)| !hinted_regions.contains(r));
            if options.is_empty() {
                continue;
            }
            let (region, required) = options[rng.below(options.len())].clone();
            let locations: Vec<String> = required.iter().map(|(_, l)| l.clone()).collect();
            let allowed = allowed_for(&locations);
            let n = region_important.get(&region).copied().unwrap_or(0);
            for _ in 0..2 {
                let message = region_hint(rng, &region, RegionHint::SomeRequired, false, n);
                let alternate = region_hint(rng, &region, RegionHint::SomeRequired, true, n);
                let info = HintInfo { kind: "woth".into(), region: Some(region.clone()), ..Default::default() };
                competitive.push(CompetitiveHint { message, alternate: Some(alternate), allowed: allowed.clone(), is_item: false, info });
            }
            if let Some(i) = clock_town.iter().position(|(r, _)| *r == region) {
                clock_town.remove(i);
                chosen_clock_town += 1;
            } else {
                important_regions.retain(|(r, _)| *r != region);
            }
            hinted_regions.push(region);
        }
        unused.clear();
    } else {
        combine_with = hintable.clone();
    }

    let mut finals: Vec<Message> = Vec::new();
    let mut infos: Vec<HintInfo> = Vec::new();
    // Made hints are word-wrapped; the stock junk lines are already laid out.
    let add = |finals: &mut Vec<Message>, q: &Quote, text: &str| {
        let mut header = HEADER.to_vec();
        let mut message = if JUNK_MESSAGES.contains(&text) { units(text) } else { wrap(&units(text), 35) };
        if q.garo() {
            header[0] = 0;
            header[1] = 1;
            let mut out = Vec::with_capacity(message.len() + 1);
            for c in message {
                if c == 0xBF {
                    out.push(0x19);
                }
                out.push(c);
            }
            message = out;
        }
        finals.push(Message { id: q.id, header: Some(header), message });
    };
    let clear_for = |q: &Quote| if q.garo() { plan.flag("ClearGaroHints") } else { plan.flag("ClearHints") };
    let count_for = |q: &Quote| if q.garo() { plan.flag("ImportanceCountGaro") } else { plan.flag("ImportanceCount") };
    let text_of = |h: &CompetitiveHint, q: &Quote| -> String {
        let use_alternate = if h.is_item { clear_for(q) } else { count_for(q) };
        if use_alternate {
            h.alternate.clone().unwrap_or_else(|| h.message.clone())
        } else {
            h.message.clone()
        }
    };

    // Hints that must go on particular stones first, fewest options first.
    loop {
        let mut candidates: Vec<usize> = (0..competitive.len()).filter(|&i| !competitive[i].allowed.is_empty()).collect();
        if candidates.is_empty() {
            break;
        }
        rng.shuffle(&mut candidates);
        candidates.sort_by_key(|&i| competitive[i].allowed.len());
        let i = candidates[0];
        let allowed = competitive[i].allowed.clone();
        let id = allowed[rng.below(allowed.len())];
        let q = quotes.iter().find(|q| q.id == id).unwrap();
        let h = competitive.remove(i);
        add(&mut finals, q, &text_of(&h, q));
        infos.push(info_for(&h, q, clear_for(q)));
        for c in &mut competitive {
            c.allowed.retain(|&x| x != id);
        }
    }

    let mut order: Vec<&&Quote> = quotes.iter().collect();
    rng.shuffle(&mut order);
    for q in order {
        if finals.iter().any(|m| m.id == q.id) {
            continue;
        }
        let clear = clear_for(q);
        let mut text: Option<String> = None;
        let mut info = HintInfo { id: q.id, kind: "junk".into(), ..Default::default() };
        if !competitive.is_empty() {
            let h = competitive.remove(rng.below(competitive.len()));
            text = Some(text_of(&h, q));
            info = info_for(&h, q, clear);
        }
        if text.is_none() {
            let mut restrictions = q.restrict.clone();
            let mut candidates: Vec<&PlacedItem> = unused.clone();
            let mut chosen: Option<&PlacedItem> = None;
            loop {
                if !restrictions.is_empty() && style == "Relevant" {
                    let i = rng.below(restrictions.len());
                    let (is_item, target, _) = restrictions[i].clone();
                    let candidate = if is_item { ctx.obj(&target) } else { ctx.at(&target) };
                    match candidate {
                        Some(c) if unused.iter().any(|u| u.id == c.id) => {
                            chosen = Some(c);
                            break;
                        }
                        _ => {
                            restrictions.remove(i);
                        }
                    }
                } else if !candidates.is_empty() {
                    if style == "Competitive" {
                        let first = candidates.iter().copied().find(|io| candidates.iter().filter(|x| x.id == io.id).count() == 1);
                        chosen = first.or_else(|| Some(candidates[rng.below(candidates.len())]));
                        break;
                    }
                    let i = rng.below(candidates.len());
                    let c = candidates[i];
                    // Junk is rarely worth a stone.
                    if (ctx.is_junk)(&c.item) && (clear || rng.below(8) != 0) {
                        candidates.remove(i);
                        continue;
                    }
                    chosen = Some(c);
                    break;
                } else {
                    break;
                }
            }
            if !q.moon() {
                if let Some(c) = chosen {
                    if let Some(i) = unused.iter().position(|u| u.id == c.id) {
                        unused.remove(i);
                    }
                }
            }
            if let Some(c) = chosen {
                let (hint, clear_hint, combined) = item_hint(ctx, rng, c, style, &combine_with, &hintable, None);
                let named = clear && clear_hint.is_some();
                info.kind = "item".into();
                // Only a clear hint says where; a vague one may even lie.
                if named {
                    info.locations = combined.iter().filter_map(|id| ctx.obj(id).and_then(|o| o.location.clone())).collect();
                }
                text = if named { clear_hint } else { hint };
            }
        }
        let text = text.unwrap_or_else(|| {
            info.kind = "junk".into();
            JUNK_MESSAGES[rng.below(JUNK_MESSAGES.len())].to_string()
        });
        add(&mut finals, q, &text);
        infos.push(info);
    }
    (finals, infos)
}

/// A placed competitive hint's info: item hints only name checks when clear.
fn info_for(h: &CompetitiveHint, q: &Quote, clear: bool) -> HintInfo {
    let mut info = h.info.clone();
    info.id = q.id;
    if h.is_item && !(clear && h.alternate.is_some()) {
        info.locations.clear();
    }
    info
}
