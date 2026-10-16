//! Assumed fill.
//!
//! Important items are placed one at a time. Each goes to a random empty
//! location that is reachable while *assuming* the player already has every
//! important item not yet placed, and where the seed can still be beaten
//! with it there. Because the assumption only shrinks, every placed item
//! stays reachable, so the result is beatable by construction (and checked
//! anyway). Unimportant items then fill the remaining spots.

use crate::reach::Graph;
use crate::rng::Rng;
use crate::world::{Id, World};
use std::collections::BTreeSet;

/// The order important items are placed in. Items placed earlier have more
/// freedom, so the order shapes where things tend to end up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    /// Random order: the least biased.
    Random,
    /// MMR 2.0's hand-tuned recipe ("Bespoke").
    Bespoke,
    /// The oldest order: category by category (quest items, trade items,
    /// dungeon items, upgrades, songs, masks, the rest), each in item order.
    Classic,
}

#[derive(Debug, thiserror::Error)]
pub enum FillError {
    #[error("couldn't find a spot for {0}")]
    Stuck(String),
    #[error("the finished seed isn't beatable")]
    NotBeatable,
    #[error("{0}")]
    Plando(String),
}

/// Placement result, indexed by location id.
pub type Placement = Vec<Option<Id>>;

/// Place items. Also returns checks hint caps made junk-only.
pub fn fill(world: &World, graph: &Graph, rng: &mut Rng, style: Style, junk_location_count: usize) -> Result<(Placement, BTreeSet<Id>), FillError> {
    let n = world.items.len();
    // Unshuffled locations keep their own item.
    let shuffled: BTreeSet<Id> = world.shuffled.iter().copied().collect();
    let mut placement: Placement = (0..n).map(|l| (world.items[l].location.is_some() && !shuffled.contains(&l)).then_some(l)).collect();
    let mut free = crate::plando::place(world, &world.plando, rng, &mut placement).map_err(FillError::Plando)?;
    // Checks hint priority caps have closed (junk only from then on).
    let mut capped: BTreeSet<Id> = BTreeSet::new();
    let caps = world.hint_caps.as_ref();
    if let Some(c) = caps {
        for l in (0..n).filter(|&l| placement[l].is_some_and(|i| free.contains(&i))) {
            c.after_place(world, &placement, l, &mut capped);
        }
    }
    free.extend(place_free_items(world, rng, &mut placement)?);
    let ok = |capped: &BTreeSet<Id>, item: Id, l: Id| world.allowed(item, l) && !(capped.contains(&l) && !world.is_junk(item) && !world.gives_junk.contains(&item));

    let progression: Vec<Id> = world.pool.iter().copied().filter(|&i| world.is_progression(i) && !free.contains(&i)).collect();
    let order = placement_order(world, rng, style, &progression, junk_location_count);

    // Assume everything important that isn't placed yet.
    let mut unplaced: BTreeSet<Id> = order.iter().copied().collect();
    let mut empty: Vec<Id> = world.shuffled.iter().copied().filter(|&l| placement[l].is_none()).collect();
    for item in order {
        unplaced.remove(&item);
        let mut owned: Vec<Id> = world.starting.clone();
        owned.extend(unplaced.iter().copied());
        // No logic: anywhere it's allowed.
        let reach = (!world.no_logic).then(|| graph.sweep(&owned, &placement, true));
        let mut candidates: Vec<Id> = empty.iter().copied().filter(|&l| reach.as_ref().is_none_or(|r| r.locations[l]) && ok(&capped, item, l)).collect();
        // What an item is worth can depend on where it is (a temporary item
        // found too late in the cycle is useless), so check the seed can
        // still be beaten with the item there, and try another spot if not.
        let spot = loop {
            let Some(&spot) = rng.choose(&candidates) else {
                return Err(FillError::Stuck(world.items[item].name.clone().unwrap_or_else(|| world.items[item].id.clone())));
            };
            placement[spot] = Some(item);
            if world.no_logic || graph.goal_reached(&graph.sweep(&owned, &placement, true)) {
                break spot;
            }
            placement[spot] = None;
            candidates.retain(|&l| l != spot);
        };
        empty.retain(|&l| l != spot);
        if let Some(c) = caps {
            c.after_place(world, &placement, spot, &mut capped);
        }
    }

    // Everything else goes anywhere it's allowed. Most constrained first.
    let mut rest: Vec<Id> = world.pool.iter().copied().filter(|i| !progression.contains(i) && !free.contains(i)).collect();
    rng.shuffle(&mut rest);
    rest.sort_by_key(|&i| empty.iter().filter(|&&l| ok(&capped, i, l)).count());
    for item in rest {
        let candidates: Vec<Id> = empty.iter().copied().filter(|&l| ok(&capped, item, l)).collect();
        let Some(&spot) = rng.choose(&candidates) else {
            return Err(FillError::Stuck(world.items[item].name.clone().unwrap_or_else(|| world.items[item].id.clone())));
        };
        placement[spot] = Some(item);
        empty.retain(|&l| l != spot);
        if let Some(c) = caps {
            c.after_place(world, &placement, spot, &mut capped);
        }
    }
    if let Some(c) = caps {
        c.finish(world, &placement, rng, &mut capped);
    }

    let reach = graph.sweep(&world.starting, &placement, true);
    if !world.no_logic && !graph.goal_reached(&reach) {
        return Err(FillError::NotBeatable);
    }
    Ok((placement, capped))
}

/// Items that can't start together (one would be lost or wasted).
const FORBIDDEN_START_TOGETHER: &[&[&str]] = &[
    &["ItemBow", "UpgradeBigQuiver", "UpgradeBiggestQuiver"],
    &["ItemBombBag", "UpgradeBigBombBag", "UpgradeBiggestBombBag"],
    &["UpgradeAdultWallet", "UpgradeGiantWallet", "UpgradeRoyalWallet"],
    &["StartingSword", "UpgradeRazorSword", "UpgradeGildedSword"],
    &["StartingShield", "ShopItemTradingPostShield", "ShopItemZoraShield", "UpgradeMirrorShield"],
    &["FairyMagic", "FairyDoubleMagic"],
    &["SongLullabyIntro", "SongLullaby"],
];

/// Shuffled starting slots are filled first, before logic (the old
/// `PlaceFreeItems`): random items that can be started with if the
/// starting item mode allows, otherwise the seed's starting items (which
/// give junk there) or a rupee. Returns the items placed.
fn place_free_items(world: &World, rng: &mut Rng, placement: &mut Placement) -> Result<BTreeSet<Id>, FillError> {
    let slots = ["MaskDeku", "SongHealing", "StartingShield", "StartingSword", "StartingHeartContainer1", "StartingHeartContainer2"];
    let in_pool: BTreeSet<Id> = world.pool.iter().copied().collect();
    let mut available: Vec<Id> = match world.starting_item_mode.as_str() {
        "Random" => (0..world.items.len()).filter(|&i| world.items[i].can_start_with && !world.items[i].temporary && world.items[i].id != "ItemPowderKeg").collect(),
        "AllowTemporaryItems" => (0..world.items.len()).filter(|&i| world.items[i].can_start_with).collect(),
        _ => Vec::new(),
    };
    available.retain(|i| in_pool.contains(i) && !(world.items[*i].downgradable && !world.prevent_downgrades) && !world.starting.contains(i));
    let mut hearts: Vec<Id> = world.starting.iter().copied().filter(|i| in_pool.contains(i) && (world.add_songs || !world.items[*i].song)).collect();
    let mut rupees: Vec<Id> = (0..world.items.len()).filter(|i| in_pool.contains(i) && world.items[*i].name.as_deref().is_some_and(|n| n.contains("Rupee"))).collect();
    let mut placed = BTreeSet::new();
    for slot in slots {
        if slot == "SongHealing" && !world.add_songs {
            continue;
        }
        let Some(l) = world.id(slot) else { continue };
        if placement[l].is_some() || world.shuffled.binary_search(&l).is_err() {
            continue;
        }
        let junk = world.junk_locations.contains(&l);
        let list = if !junk && !available.is_empty() {
            &mut available
        } else if !hearts.is_empty() {
            &mut hearts
        } else {
            &mut rupees
        };
        let Some(&item) = rng.choose(list) else {
            return Err(FillError::Stuck(format!("a starting item for {slot} (not enough items that can be started with)")));
        };
        list.retain(|&i| i != item);
        placement[l] = Some(item);
        placed.insert(item);
        if let Some(group) = FORBIDDEN_START_TOGETHER.iter().find(|g| g.contains(&world.items[item].id.as_str())) {
            available.retain(|&i| !group.contains(&world.items[i].id.as_str()));
        }
    }
    Ok(placed)
}

/// Restricted items first (fewest options), then the style's order, then
/// any remaining important items at random.
fn placement_order(world: &World, rng: &mut Rng, style: Style, progression: &[Id], junk_location_count: usize) -> Vec<Id> {
    let mut restricted: Vec<Id> = progression.iter().copied().filter(|&i| world.is_restricted(i)).collect();
    rng.shuffle(&mut restricted);
    let mut order = restricted;
    let styled = match style {
        Style::Bespoke => bespoke_order(world, rng, junk_location_count),
        Style::Classic => classic_order(world),
        Style::Random => Vec::new(),
    };
    for item in styled {
        if progression.contains(&item) && !order.contains(&item) {
            order.push(item);
        }
    }
    let mut rest: Vec<Id> = progression.iter().copied().filter(|i| !order.contains(i)).collect();
    rng.shuffle(&mut rest);
    order.extend(rest);
    order
}

/// MMR 2.0's placement without Random or Bespoke (`PlaceQuestItems` ..
/// `PlaceMoonItems`), as an ordering: fixed, no randomness.
fn classic_order(world: &World) -> Vec<Id> {
    const RANGES: [(&str, &str); 15] = [
        ("TradeItemRoomKey", "TradeItemMamaLetter"),
        ("TradeItemMoonTear", "TradeItemOceanDeed"),
        ("FrogWoodfallTemple", "FrogLaundryPool"),
        ("ItemWoodfallMap", "ItemStoneTowerKey4"),
        ("StartingSword", "StartingHeartContainer2"),
        ("UpgradeRazorSword", "UpgradeRoyalWallet"),
        ("SongHealing", "SongOath"),
        ("SongLullabyIntro", "SongLullabyIntro"),
        ("MaskPostmanHat", "MaskZora"),
        ("MaskDeku", "ItemNotebook"),
        ("CollectibleSwampSpiderToken1", "CollectibleOceanSpiderToken30"),
        ("CollectibleStrayFairyClockTown", "CollectibleStrayFairyStoneTower15"),
        ("MundaneItemLotteryPurpleRupee", "MundaneItemSeahorse"),
        ("ShopItemTradingPostRedPotion", "ShopItemZoraRedPotion"),
        ("HeartPieceDekuTrial", "ChestLinkTrialBombchu10"),
    ];
    let mut out = Vec::new();
    for (from, to) in RANGES {
        let (Some(a), Some(b)) = (world.id(from), world.id(to)) else { continue };
        out.extend(a..=b);
    }
    out
}

/// MMR 2.0 `PlaceBespokeItems`, as an ordering. Returns item ids in the
/// order the recipe places them.
fn bespoke_order(world: &World, rng: &mut Rng, junk_location_count: usize) -> Vec<Id> {
    let id = |s: &str| world.id(s).expect("bespoke item exists");
    let mut out: Vec<Id> = Vec::new();
    let pool: BTreeSet<Id> = world.pool.iter().copied().collect();
    // MMR 2.0: max(500 - floor(checks * U(0.7, 1.3)), 0). Whole numbers
    // only, in thousandths, so every platform computes exactly the same.
    let checks = world.pool.len().saturating_sub(junk_location_count) as u64;
    let factor = 700 + rng.below(600) as u64;
    let density = 500u64.saturating_sub(checks * factor / 1000);

    let wallets = [id("UpgradeRoyalWallet"), id("UpgradeGiantWallet"), id("UpgradeAdultWallet")];
    let mut can_place_songs = !world.add_songs;
    let place = |out: &mut Vec<Id>, item: Id, can_place_songs: bool| {
        // Wallets are always placed biggest first.
        let item = if wallets.contains(&item) { *wallets.iter().find(|w| !out.contains(w)).unwrap_or(&item) } else { item };
        if (!world.items[item].song || can_place_songs) && pool.contains(&item) && !out.contains(&item) {
            out.push(item);
        }
    };
    let ids = |names: &[&str]| -> Vec<Id> { names.iter().map(|n| id(n)).collect() };
    let group = |out: &mut Vec<Id>, rng: &mut Rng, items: Vec<Id>, can: bool| {
        let mut items = items;
        rng.shuffle(&mut items);
        for i in items {
            place(out, i, can);
        }
    };
    // Enum ranges from the legacy item list.
    let range = |from: &str, to: &str| -> Vec<Id> { (id(from)..=id(to)).collect() };

    if world.giant_mask_anywhere {
        place(&mut out, id("MaskGiant"), can_place_songs);
    }
    if rng.below(100) < 30 {
        place(&mut out, id("ItemNotebook"), can_place_songs);
    }
    for i in ["UpgradeBiggestBombBag", "UpgradeBigBombBag", "UpgradeBiggestQuiver", "UpgradeBigQuiver"] {
        place(&mut out, id(i), can_place_songs);
    }
    let mut trade = ids(&["TradeItemRoomKey", "TradeItemKafeiLetter", "TradeItemMamaLetter", "MaskKafei"]);
    trade.extend(ids(&["UpgradeRoyalWallet", "UpgradeGiantWallet"]));
    let n = ((density + 35) / 70) as usize;
    let picked = rng.sample(&trade, n);
    group(&mut out, rng, picked, can_place_songs);

    let tier1 = ids(&["MaskGreatFairy", "MaskScents", "TradeItemKafeiLetter", "TradeItemMamaLetter", "MaskKafei"]);
    let tier2 = ids(&[
        "MaskPostmanHat", "MaskAllNight", "MaskKeaton", "MaskBremen", "MaskTruth", "MaskBunnyHood", "MaskRomani", "MaskCircusLeader", "MaskCouple",
        "MaskKamaro",
    ]);
    let tier3 = ids(&["TradeItemMoonTear", "TradeItemLandDeed", "TradeItemSwampDeed", "TradeItemMountainDeed", "TradeItemOceanDeed"]);
    let tier4 = ids(&["MaskStone", "MaskDonGero", "TradeItemRoomKey", "TradeItemPendant", "SongLullabyIntro"]);
    let blast_bomb = ids(&["ItemBombBag", "MaskBlast"]);
    let captain_keg = ids(&["ItemPowderKeg", "MaskCaptainHat"]);
    let new_wave_elegy = ids(&["SongNewWaveBossaNova", "SongElegy"]);

    let n = ((density + 25) / 50) as usize;
    let picked = rng.sample(&tier2, n);
    group(&mut out, rng, picked, can_place_songs);
    let picked = rng.sample(&tier3, 1);
    group(&mut out, rng, picked, can_place_songs);

    let roll = rng.below(100);
    let c = can_place_songs;
    if roll < 60 {
        group(&mut out, rng, ids(&["ItemHookshot", "ItemBow", "MaskZora", "SongSonata", "SongLullaby"]), c);
        for i in ["MaskFierceDeity", "ItemLightArrow", "ItemIceArrow", "SongEpona", "SongHealing", "SongLullabyIntro", "SongSoaring"] {
            place(&mut out, id(i), c);
        }
        group(&mut out, rng, new_wave_elegy.clone(), c);
        place(&mut out, id("SongStorms"), c);
        place(&mut out, id("SongOath"), c);
        group(&mut out, rng, ids(&["ItemFireArrow", "MaskDeku", "MaskGoron"]), c);
        group(&mut out, rng, blast_bomb.clone(), c);
    } else if roll < 85 {
        place(&mut out, id("MaskGoron"), c);
        place(&mut out, id("ItemHookshot"), c);
        place(&mut out, id("SongSonata"), c);
        // MMR 2.0 skips the Lullaby here if the Goron Mask landed on the
        // Gibdo Mask's spot; that depends on placement, so it's kept unconditional.
        place(&mut out, id("SongLullaby"), c);
        for i in ["SongEpona", "SongHealing", "SongSoaring"] {
            place(&mut out, id(i), c);
        }
        group(&mut out, rng, new_wave_elegy.clone(), c);
        for i in [
            "SongStorms", "SongLullaby", "SongLullabyIntro", "SongOath", "MaskFierceDeity", "MaskZora", "ItemBow", "ItemFireArrow", "ItemLightArrow",
            "ItemIceArrow", "MaskDeku",
        ] {
            place(&mut out, id(i), c);
        }
        group(&mut out, rng, blast_bomb.clone(), c);
    } else {
        for i in ["MaskDeku", "SongLullaby", "ItemHookshot", "SongSonata", "SongEpona", "SongHealing", "SongLullabyIntro", "SongSoaring"] {
            place(&mut out, id(i), c);
        }
        group(&mut out, rng, new_wave_elegy.clone(), c);
        for i in [
            "SongStorms", "SongOath", "MaskFierceDeity", "MaskZora", "MaskGoron", "ItemBow", "ItemFireArrow", "ItemLightArrow", "ItemIceArrow",
        ] {
            place(&mut out, id(i), c);
        }
        group(&mut out, rng, blast_bomb.clone(), c);
    }

    for i in ["ItemNotebook", "ItemPictobox", "ItemLens"] {
        place(&mut out, id(i), c);
    }
    group(&mut out, rng, captain_keg, c);
    for i in [
        "UpgradeMirrorShield", "FairyDoubleMagic", "FairyMagic", "MaskFierceDeity", "MaskGiant", "MaskGibdo", "MaskGaro", "UpgradeRoyalWallet",
        "UpgradeGiantWallet", "UpgradeAdultWallet",
    ] {
        place(&mut out, id(i), c);
    }

    can_place_songs = true;
    let c = can_place_songs;
    if world.add_songs {
        place(&mut out, id("SongEpona"), c);
        group(&mut out, rng, ids(&["SongSonata", "SongLullaby", "SongNewWaveBossaNova", "SongElegy"]), c);
        for i in ["SongHealing", "SongOath", "SongSoaring"] {
            place(&mut out, id(i), c);
        }
    }
    group(&mut out, rng, tier1, c);
    group(&mut out, rng, tier2, c);
    group(&mut out, rng, tier3, c);
    group(&mut out, rng, tier4, c);
    for i in range("MaskPostmanHat", "MaskKamaro")
        .into_iter()
        .chain(range("TradeItemRoomKey", "TradeItemMamaLetter"))
        .chain(range("TradeItemMoonTear", "TradeItemOceanDeed"))
    {
        place(&mut out, i, c);
    }
    group(&mut out, rng, ids(&["ItemFairySword", "FairySpinAttack", "UpgradeGildedSword"]), c);
    for i in [
        "FairyDoubleDefense", "UpgradeRazorSword", "StartingSword", "ItemBottleAliens", "ItemBottleWitch", "ItemBottleGoronRace",
        "ItemBottleMadameAroma", "ItemBottleBeavers", "ItemBottleDampe",
    ] {
        place(&mut out, id(i), c);
    }
    if world.add_songs {
        place(&mut out, id("SongStorms"), c);
    }
    for i in ["ItemMagicBean", "ChestInvertedStoneTowerBean", "ShopItemBusinessScrubMagicBean"] {
        place(&mut out, id(i), c);
    }
    out
}
