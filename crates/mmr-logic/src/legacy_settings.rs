//! Settings-dependent logic rewrites, ported from MMR 2.0
//! `Randomizer.UpdateLogicForSettings`.
//!
//! This is the reference implementation: it applies to entry-form logic
//! exactly what MMR 2.0 did in code. The region format expresses the same
//! rules as data (see `settings_rules`), and tests compare the two.
//!
//! Parts of the original that don't change any answer are not ported:
//! removing starting items from requirements (they're owned from the start),
//! clearing disabled tricks/settings (the evaluator already treats them as
//! unsatisfied), and the "sphere zero" and conditional simplifications.

use crate::model::{Entry, LogicFile, TimeSet};
use serde_json::{Map, Value};

type Values = Map<String, Value>;

fn flag(values: &Values, key: &str, f: &str) -> bool {
    values.get(key).and_then(Value::as_array).is_some_and(|a| a.iter().any(|v| v.as_str() == Some(f)))
}

fn on(values: &Values, key: &str) -> bool {
    values.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn num(values: &Values, key: &str, default: i64) -> i64 {
    values.get(key).and_then(Value::as_i64).unwrap_or(default)
}

fn in_pool(values: &Values, id: &str) -> bool {
    flag(values, "CustomItemList", id)
}

/// Item ids in game-data (legacy enum) order from `first`, `n` of them.
pub fn item_range(first: &str, n: usize) -> Vec<String> {
    let items = &mmr_data::game().items;
    let start = items.iter().position(|i| i.id == first).expect("range start exists");
    items[start..start + n].iter().map(|i| i.id.clone()).collect()
}

/// Item ids from `first` to `last` inclusive, in game-data order.
pub fn item_span(first: &str, last: &str) -> Vec<String> {
    let items = &mmr_data::game().items;
    let a = items.iter().position(|i| i.id == first).expect("span start exists");
    let b = items.iter().position(|i| i.id == last).expect("span end exists");
    items[a..=b].iter().map(|i| i.id.clone()).collect()
}

pub fn boss_remains() -> Vec<String> {
    item_range("RemainsOdolwa", 4)
}

/// `LogicUtils`' `Combinations(k)`, in the same order.
pub fn combinations(items: &[String], k: usize) -> Vec<Vec<String>> {
    if k == 0 {
        return vec![vec![]];
    }
    let mut out = Vec::new();
    for (i, e) in items.iter().enumerate() {
        for mut rest in combinations(&items[i + 1..], k - 1) {
            rest.insert(0, e.clone());
            out.push(rest);
        }
    }
    out
}

struct Patcher<'a> {
    file: &'a mut LogicFile,
    next: usize,
}

impl Patcher<'_> {
    fn get(&mut self, id: &str) -> Option<&mut Entry> {
        self.file.entries.iter_mut().find(|e| e.id == id)
    }

    fn entry(&mut self, id: &str) -> &mut Entry {
        self.get(id).unwrap_or_else(|| panic!("logic has no entry {id}"))
    }

    /// Add a generated helper entry (MMR 2.0 added nameless ItemObjects).
    fn add(&mut self, name: &str, requires: Vec<String>, any_of: Vec<Vec<String>>) -> String {
        self.next += 1;
        let id = format!("Setting Rule {}: {name}", self.next);
        let mut e = Entry::new(id.clone());
        e.requires = requires;
        e.any_of = any_of;
        self.file.entries.push(e);
        id
    }

    /// An existing entry with exactly this shape (no requires, these
    /// single-item alternatives in any order), like MMR 2.0's lookups.
    fn find_alternatives(&self, items: &[&str]) -> Option<String> {
        let game = mmr_data::game();
        self.file
            .entries
            .iter()
            .find(|e| {
                // MMR 2.0 `IsFake`: anything without an item name, which
                // includes every helper.
                game.item(&e.id).is_none_or(|i| i.is_fake())
                    && e.requires.is_empty()
                    && e.any_of.len() == items.len()
                    && items.iter().all(|it| e.any_of.iter().any(|g| g.len() == 1 && g[0] == *it))
            })
            .map(|e| e.id.clone())
    }
}

/// Apply MMR 2.0's settings-dependent logic rewrites to `file`.
pub fn apply(file: &mut LogicFile, values: &Values) {
    let mut p = Patcher { file, next: 0 };
    let remains = boss_remains();

    // Fewer boss remains needed for the Moon.
    let required_remains = num(values, "RequiredBossRemains", 4);
    if required_remains < 4 {
        p.entry("AreaMoonAccess").requires.retain(|r| !remains.contains(r));
        if required_remains > 0 {
            let id = p.add("Required Boss Remains", vec![], combinations(&remains, required_remains as usize));
            p.entry("AreaMoonAccess").requires.push(id);
        }
    }

    // Victory conditions.
    let victory: Vec<String> =
        values.get("VictoryMode").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
    if !victory.is_empty() {
        let has = |f: &str| victory.iter().any(|v| v == f);
        if has("DirectToCredits") {
            p.entry("OtherCredits").requires.retain(|r| r != "AreaMoonAccess" && r != "OtherKillMajora");
        }
        let need = |p: &mut Patcher, name: &str, requires: Vec<String>, any_of: Vec<Vec<String>>| {
            let id = p.add(name, requires, any_of);
            p.entry("OtherCredits").requires.push(id);
        };
        if has("Fairies") {
            need(&mut p, "All Stray Fairies", item_range("CollectibleStrayFairyWoodfall1", 60), vec![]);
        }
        if has("SkullTokens") {
            let mut tokens = item_range("CollectibleOceanSpiderToken1", 30);
            tokens.extend(item_range("CollectibleSwampSpiderToken1", 30));
            need(&mut p, "All Skulltula Tokens", tokens, vec![]);
        }
        if has("NonTransformationMasks") {
            need(&mut p, "All Masks", item_range("MaskPostmanHat", 20), vec![]);
        }
        if has("TransformationMasks") {
            need(&mut p, "All Transformation Masks", ["MaskDeku", "MaskGoron", "MaskZora", "MaskFierceDeity"].map(String::from).to_vec(), vec![]);
        }
        if has("Notebook") {
            need(&mut p, "All Notebook Entries", item_range("NotebookMeetBombers", 51), vec![]);
        }
        if has("Hearts") {
            let hearts = mmr_data::game()
                .items
                .iter()
                .filter(|i| matches!(i.item_category.as_deref(), Some("PiecesOfHeart") | Some("HeartContainers")))
                .map(|i| i.id.clone())
                .collect();
            need(&mut p, "All Hearts", hearts, vec![]);
        }
        let n = ["FourBossRemains", "ThreeBossRemains", "TwoBossRemains", "OneBossRemains"].iter().position(|f| has(f)).map(|i| 4 - i);
        if let Some(n) = n {
            if n == 4 {
                need(&mut p, "Victory Boss Remains", remains.clone(), vec![]);
            } else {
                need(&mut p, "Victory Boss Remains", vec![], combinations(&remains, n));
            }
        }
    }

    if on(values, "FreeHints") {
        for id in item_span("GossipTerminaSouth", "GossipTerminaGossipDrums") {
            if let Some(e) = p.get(&id) {
                e.requires.retain(|r| r != "MaskTruth");
            }
        }
    }
    if on(values, "FreeGaroHints") {
        for id in item_span("HintGaroCanyonLower1", "HintGaroCastleUpper") {
            if let Some(e) = p.get(&id) {
                e.requires.retain(|r| r != "MaskGaro");
            }
        }
    }

    if in_pool(values, "ShopItemBusinessScrubMagicBean") {
        for id in ["ShopItemBusinessScrubMagicBeanInSwamp", "ShopItemBusinessScrubMagicBeanInTown"] {
            if let Some(e) = p.get(id) {
                e.requires.retain(|r| r != "OtherMagicBean");
            }
        }
    }

    let scoops_shuffled = mmr_data::game()
        .items
        .iter()
        .any(|i| i.item_category.as_deref() == Some("ScoopedItems") && in_pool(values, &i.id));
    if scoops_shuffled && values.get("LogicMode").and_then(Value::as_str) == Some("Casual") && p.get("Any Bottle").is_some() && p.get("2 Bottles").is_some() {
        let e = p.entry("BottleCatchPrincess");
        e.requires.retain(|r| r != "Any Bottle");
        e.requires.push("2 Bottles".into());
    }

    let arrows40 = match p.find_alternatives(&["UpgradeBigQuiver", "UpgradeBiggestQuiver"]) {
        Some(id) => id,
        None => p.add("40 Arrows", vec![], vec![vec!["UpgradeBigQuiver".into()], vec!["UpgradeBiggestQuiver".into()]]),
    };
    let bombchu10 = match p.find_alternatives(&["ChestInvertedStoneTowerBombchu10", "ChestLinkTrialBombchu10", "ShopItemBombsBombchu10"]) {
        Some(id) => id,
        None => p.add(
            "10 Bombchu",
            vec![],
            vec![vec!["ChestInvertedStoneTowerBombchu10".into()], vec!["ChestLinkTrialBombchu10".into()], vec!["ShopItemBombsBombchu10".into()]],
        ),
    };

    if on(values, "ByoAmmo") {
        for id in ["ChestInvertedStoneTowerBombchu10", "ChestLinkTrialBombchu10", "ShopItemBombsBombchu10"] {
            p.entry(id).time.needed = TimeSet(1); // Day 1
        }
        p.entry("UpgradeBigQuiver").requires.push(arrows40.clone());
        p.entry("UpgradeBiggestQuiver").requires.push(arrows40.clone());
        p.entry("HeartPieceSwampArchery").requires.push(arrows40.clone());
        p.entry("HeartPieceTownArchery").requires.push("UpgradeBiggestQuiver".into());
        p.entry("HeartPieceHoneyAndDarling").requires.push(bombchu10.clone());
        let escort = p.add("Escort Cremia", vec![], vec![vec!["OtherArrow".into()], vec!["MaskCircusLeader".into()]]);
        p.entry("MaskRomani").requires.push(escort);
    }

    if on(values, "ProgressiveUpgrades") {
        progressive_upgrades(&mut p, &arrows40);
    }

    if on(values, "BombchuDrops") {
        let e = p.entry(&bombchu10);
        for id in [
            "ChestIkanaSecretShrineGrotto",
            "ChestTerminaGrottoBombchu",
            "ChestGreatBayCapeGrotto",
            "ChestGraveyardGrotto",
            "ChestToIkanaGrotto",
            "ChestToGoronRaceGrotto",
        ] {
            e.any_of.push(vec![id.into()]);
        }
    }

    if !in_pool(values, "ChestLinkTrialBombchu10") {
        for id in ["HeartPieceLinkTrial", "Complete Link Trial"] {
            if let Some(e) = p.get(id) {
                for g in &mut e.any_of {
                    g.retain(|r| *r != bombchu10);
                }
            }
        }
    }

    let eggs = num(values, "RequiredZoraEggs", 7);
    if eggs < 7 {
        let all = p.entry("ZoraEggAll").requires.clone();
        if in_pool(values, "BottleCatchEgg") && in_pool(values, "BottleCatchFish") {
            let for_fish = p.add("All Zora Eggs For Fish", all.clone(), vec![]);
            p.entry("BottleCatchEgg").any_of.push(vec!["BottleCatchEgg".into(), for_fish]);
        }
        let e = p.entry("ZoraEggAll");
        e.any_of = combinations(&all, eggs.max(0) as usize);
        e.requires.clear();
    }
}

/// Progressive upgrades: an upgrade item means "the next tier", so logic
/// that names a tier must count how many of that upgrade line you have.
fn progressive_upgrades(p: &mut Patcher, arrows40: &str) {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let quivers = s(&["ItemBow", "UpgradeBigQuiver", "UpgradeBiggestQuiver"]);
    p.entry(arrows40).any_of = combinations(&quivers, 2);
    let arrows50 = p.add("50 Arrows", quivers.clone(), vec![]);

    let bags = s(&["ItemBombBag", "UpgradeBigBombBag", "UpgradeBiggestBombBag"]);
    let bombs20 = p.find_alternatives(&["ItemBombBag", "UpgradeBigBombBag", "UpgradeBiggestBombBag"]);
    let bombs30 = match p.find_alternatives(&["UpgradeBigBombBag", "UpgradeBiggestBombBag"]) {
        Some(id) => {
            p.entry(&id).any_of = combinations(&bags, 2);
            id
        }
        None => p.add("30 Bombs", vec![], combinations(&bags, 2)),
    };
    let bombs40 = p.add("40 Bombs", bags.clone(), vec![]);

    let swords = s(&["StartingSword", "UpgradeRazorSword", "UpgradeGildedSword"]);
    let sword1 = p.find_alternatives(&["StartingSword", "UpgradeRazorSword", "UpgradeGildedSword"]);
    let sword2 = match p.find_alternatives(&["UpgradeRazorSword", "UpgradeGildedSword"]) {
        Some(id) => {
            p.entry(&id).any_of = combinations(&swords, 2);
            id
        }
        None => p.add("Sword Level 2", vec![], combinations(&swords, 2)),
    };
    let sword3 = p.add("Sword Level 3", swords.clone(), vec![]);

    let wallets = s(&["UpgradeAdultWallet", "UpgradeGiantWallet", "UpgradeRoyalWallet"]);
    let wallets200 = p.find_alternatives(&["UpgradeAdultWallet", "UpgradeGiantWallet", "UpgradeRoyalWallet"]);
    let wallets500 = match p.find_alternatives(&["UpgradeGiantWallet", "UpgradeRoyalWallet"]) {
        Some(id) => {
            p.entry(&id).any_of = combinations(&wallets, 2);
            id
        }
        None => p.add("Wallet Level 2", vec![], combinations(&wallets, 2)),
    };
    let wallets999 = p.add("Wallet Level 3", wallets.clone(), vec![]);

    let magic_any = p.find_alternatives(&["FairyMagic", "FairyDoubleMagic"]);
    let magic_large = p.add("Large Magic", s(&["FairyMagic", "FairyDoubleMagic"]), vec![]);

    let lullaby_any = p.find_alternatives(&["SongLullaby", "SongLullabyIntro"]);
    let lullaby_full = p.add("Full Lullaby", s(&["SongLullaby", "SongLullabyIntro"]), vec![]);

    let is = |id: &str, other: &Option<String>| other.as_deref() == Some(id);
    for e in p.file.entries.iter_mut() {
        let id = e.id.clone();
        let swap_req = |e: &mut Entry, from: &str, to: &str| {
            if let Some(i) = e.requires.iter().position(|r| r == from) {
                e.requires.remove(i);
                e.requires.push(to.to_string());
            }
        };
        let swap_groups = |e: &mut Entry, from: &str, to: &str| {
            for g in &mut e.any_of {
                if let Some(i) = g.iter().position(|r| r == from) {
                    g.remove(i);
                    g.push(to.to_string());
                }
            }
        };
        if id != lullaby_full {
            swap_req(e, "SongLullaby", &lullaby_full);
        }
        if !is(&id, &lullaby_any) {
            swap_groups(e, "SongLullaby", &lullaby_full);
        }
        if id != magic_large {
            swap_req(e, "FairyDoubleMagic", &magic_large);
        }
        if !is(&id, &magic_any) {
            swap_groups(e, "FairyDoubleMagic", &magic_large);
        }
        if id != wallets999 {
            swap_req(e, "UpgradeRoyalWallet", &wallets999);
        }
        if !is(&id, &wallets200) && id != wallets500 {
            swap_groups(e, "UpgradeRoyalWallet", &wallets999);
            swap_groups(e, "UpgradeGiantWallet", &wallets500);
        }
        if id != sword3 {
            swap_req(e, "UpgradeGildedSword", &sword3);
        }
        if !is(&id, &sword1) && id != sword2 {
            swap_groups(e, "UpgradeGildedSword", &sword3);
            swap_groups(e, "UpgradeRazorSword", &sword2);
        }
        if id != bombs40 {
            swap_req(e, "UpgradeBiggestBombBag", &bombs40);
        }
        if !is(&id, &bombs20) && id != bombs30 && id != "OtherExplosive" {
            swap_groups(e, "UpgradeBiggestBombBag", &bombs40);
            swap_groups(e, "UpgradeBigBombBag", &bombs30);
        }
        if id != arrows50 {
            swap_req(e, "UpgradeBiggestQuiver", &arrows50);
        }
        if id != arrows40 && id != "OtherArrow" {
            swap_groups(e, "UpgradeBiggestQuiver", &arrows50);
            swap_groups(e, "UpgradeBigQuiver", arrows40);
        }
    }
}
