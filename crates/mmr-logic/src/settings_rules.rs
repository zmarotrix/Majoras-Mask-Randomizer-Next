//! Build MMR 2.0's settings-dependent logic rewrites into the logic itself.
//!
//! `legacy_settings::apply` rewrites logic per settings, the way MMR 2.0 did
//! in code. [`bake`] instead produces one logic file that already contains
//! every rewrite as a visible rule depending on settings, e.g.
//!
//! ```text
//! "Moon: Boss Remains Required" = (Required Boss Remains is 0)
//!     or (is 1 and any 1 of the remains) or ... or (all 4 remains)
//! ```
//!
//! so logic you edit is the logic that runs. Tests check that `bake(logic)`
//! under any settings behaves exactly like `apply(logic, settings)`.

use crate::condition::SettingCondition;
use crate::legacy_settings::{boss_remains, combinations, item_range, item_span};
use crate::model::{Entry, LogicFile};
use serde_json::Value;
use std::collections::BTreeMap;

struct Baker {
    file: LogicFile,
    /// Generated helper ids (never rewritten by the progressive pass).
    generated: Vec<String>,
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

impl Baker {
    fn get(&mut self, id: &str) -> Option<&mut Entry> {
        self.file.entries.iter_mut().find(|e| e.id == id)
    }

    fn has(&self, id: &str) -> bool {
        self.file.entries.iter().any(|e| e.id == id)
    }

    /// An entry satisfied when a setting condition holds.
    fn cond(&mut self, name: &str, c: SettingCondition) -> String {
        let id = format!("Setting: {name}");
        if !self.has(&id) {
            let mut e = Entry::new(id.clone());
            e.setting = Some(c);
            e.note = Some(GENERATED_NOTE.into());
            self.file.entries.push(e);
            self.generated.push(id.clone());
        }
        id
    }

    fn is_on(&mut self, key: &str) -> String {
        self.cond(&format!("{key} is on"), SettingCondition::Setting { setting: key.into(), is: None, has: None })
    }

    fn is_off(&mut self, key: &str) -> String {
        self.cond(&format!("{key} is off"), SettingCondition::not(SettingCondition::Setting { setting: key.into(), is: None, has: None }))
    }

    fn equals(&mut self, key: &str, n: i64) -> String {
        self.cond(&format!("{key} is {n}"), SettingCondition::Setting { setting: key.into(), is: Some(Value::from(n)), has: None })
    }

    fn not_equals(&mut self, key: &str, n: i64) -> String {
        self.cond(&format!("{key} is not {n}"), SettingCondition::not(SettingCondition::Setting { setting: key.into(), is: Some(Value::from(n)), has: None }))
    }

    fn has_flag(&mut self, key: &str, flag: &str) -> String {
        self.cond(&format!("{key} includes {flag}"), SettingCondition::Setting { setting: key.into(), is: None, has: Some(flag.into()) })
    }

    fn lacks_flag(&mut self, key: &str, flag: &str) -> String {
        self.cond(&format!("{key} doesn't include {flag}"), SettingCondition::not(SettingCondition::Setting { setting: key.into(), is: None, has: Some(flag.into()) }))
    }

    fn randomized(&mut self, item: &str) -> String {
        self.cond(&format!("{item} is randomized"), SettingCondition::Randomized { randomized: item.into() })
    }

    fn not_randomized(&mut self, item: &str) -> String {
        self.cond(&format!("{item} is not randomized"), SettingCondition::not(SettingCondition::Randomized { randomized: item.into() }))
    }

    /// A generated helper with alternatives (created once).
    fn helper(&mut self, id: &str, requires: Vec<String>, any_of: Vec<Vec<String>>) -> String {
        if !self.has(id) {
            let mut e = Entry::new(id.to_string());
            e.requires = requires;
            e.any_of = any_of;
            e.note = Some(GENERATED_NOTE.into());
            self.file.entries.push(e);
            self.generated.push(id.to_string());
        }
        id.to_string()
    }

    /// Add an alternative that only applies when `conds` hold. If the entry
    /// had no alternatives, the original "anything goes" must stay possible
    /// when the conditions don't hold.
    fn append_group(&mut self, id: &str, conds: &[String], group: Vec<String>, negations: Vec<String>) {
        let e = self.get(id).unwrap_or_else(|| panic!("no entry {id}"));
        if e.any_of.is_empty() {
            for n in negations {
                e.any_of.push(vec![n]);
            }
        }
        let mut g = conds.to_vec();
        g.extend(group);
        e.any_of.push(g);
    }

    fn find_alternatives(&self, items: &[&str]) -> Option<String> {
        let game = mmr_data::game();
        self.file
            .entries
            .iter()
            .find(|e| {
                game.item(&e.id).is_none_or(|i| i.is_fake())
                    && e.requires.is_empty()
                    && e.any_of.len() == items.len()
                    && items.iter().all(|it| e.any_of.iter().any(|g| g.len() == 1 && g[0] == *it))
            })
            .map(|e| e.id.clone())
    }

    fn replace_first_in_requires(&mut self, id: &str, from: &str, to: &str) -> bool {
        match self.get(id) {
            Some(e) => match e.requires.iter().position(|r| r == from) {
                Some(i) => {
                    e.requires[i] = to.to_string();
                    true
                }
                None => false,
            },
            None => false,
        }
    }
}

/// A helper only built-in settings rules create; its presence means a file
/// already has them.
pub const MARKER: &str = "Victory: Boss Remains";

pub fn is_baked(file: &LogicFile) -> bool {
    file.entries.iter().any(|e| e.id == MARKER)
}

/// Logic with every MMR 2.0 settings rewrite built in as rules. Files that
/// already have them are returned unchanged.
pub fn bake(v1: &LogicFile) -> LogicFile {
    if is_baked(v1) {
        return v1.clone();
    }
    let mut b = Baker { file: v1.clone(), generated: Vec::new() };
    let remains = boss_remains();

    // Fewer boss remains needed for the Moon.
    if b.has("AreaMoonAccess") {
        let present: Vec<String> = b.get("AreaMoonAccess").unwrap().requires.iter().filter(|r| remains.contains(r)).cloned().collect();
        let mut alts: Vec<Vec<String>> = Vec::new();
        for n in 0..4 {
            let eq = b.equals("RequiredBossRemains", n);
            for c in combinations(&remains, n as usize) {
                let mut g = vec![eq.clone()];
                g.extend(c);
                alts.push(g);
            }
        }
        let mut default: Vec<String> = (0..4).map(|n| b.not_equals("RequiredBossRemains", n)).collect();
        default.extend(present);
        alts.push(default);
        let h = b.helper("Moon: Boss Remains Required", vec![], alts);
        let e = b.get("AreaMoonAccess").unwrap();
        e.requires.retain(|r| !remains.contains(r));
        e.requires.push(h);
    }

    // Victory conditions.
    if b.has("OtherCredits") {
        let direct = b.has_flag("VictoryMode", "DirectToCredits");
        let e = b.get("OtherCredits").unwrap();
        let had: Vec<String> = e.requires.iter().filter(|r| *r == "AreaMoonAccess" || *r == "OtherKillMajora").cloned().collect();
        if !had.is_empty() {
            e.requires.retain(|r| r != "AreaMoonAccess" && r != "OtherKillMajora");
            let h = b.helper("Victory: Defeat Majora", vec![], vec![vec![direct], had]);
            b.get("OtherCredits").unwrap().requires.push(h);
        }
        let hearts: Vec<String> = mmr_data::game()
            .items
            .iter()
            .filter(|i| matches!(i.item_category.as_deref(), Some("PiecesOfHeart") | Some("HeartContainers")))
            .map(|i| i.id.clone())
            .collect();
        let mut tokens = item_range("CollectibleOceanSpiderToken1", 30);
        tokens.extend(item_range("CollectibleSwampSpiderToken1", 30));
        let goals: Vec<(&str, &str, Vec<String>)> = vec![
            ("Fairies", "All Stray Fairies", item_range("CollectibleStrayFairyWoodfall1", 60)),
            ("SkullTokens", "All Skulltula Tokens", tokens),
            ("NonTransformationMasks", "All Masks", item_range("MaskPostmanHat", 20)),
            ("TransformationMasks", "All Transformation Masks", s(&["MaskDeku", "MaskGoron", "MaskZora", "MaskFierceDeity"])),
            ("Notebook", "All Notebook Entries", item_range("NotebookMeetBombers", 51)),
            ("Hearts", "All Hearts", hearts),
        ];
        for (flag, name, items) in goals {
            let off = b.lacks_flag("VictoryMode", flag);
            let on = b.has_flag("VictoryMode", flag);
            let mut g = vec![on];
            g.extend(items);
            let h = b.helper(&format!("Victory: {name}"), vec![], vec![vec![off], g]);
            b.get("OtherCredits").unwrap().requires.push(h);
        }
        // Boss remains goal: the highest one chosen wins.
        let flags = ["FourBossRemains", "ThreeBossRemains", "TwoBossRemains", "OneBossRemains"];
        let mut alts = Vec::new();
        for (i, flag) in flags.iter().enumerate() {
            let mut prefix: Vec<String> = flags[..i].iter().map(|f| b.lacks_flag("VictoryMode", f)).collect();
            prefix.push(b.has_flag("VictoryMode", flag));
            for c in combinations(&remains, 4 - i) {
                let mut g = prefix.clone();
                g.extend(c);
                alts.push(g);
            }
        }
        alts.push(flags.iter().map(|f| b.lacks_flag("VictoryMode", f)).collect());
        let h = b.helper("Victory: Boss Remains", vec![], alts);
        b.get("OtherCredits").unwrap().requires.push(h);
    }

    // Free hints.
    for (key, mask, first, last, name) in [
        ("FreeHints", "MaskTruth", "GossipTerminaSouth", "GossipTerminaGossipDrums", "Hints: Mask of Truth"),
        ("FreeGaroHints", "MaskGaro", "HintGaroCanyonLower1", "HintGaroCastleUpper", "Hints: Garo's Mask"),
    ] {
        let free = b.is_on(key);
        let h = b.helper(name, vec![], vec![vec![free], vec![mask.to_string()]]);
        for id in item_span(first, last) {
            b.replace_first_in_requires(&id, mask, &h);
        }
    }

    // Business Scrub bean.
    {
        let rand = b.randomized("ShopItemBusinessScrubMagicBean");
        let h = b.helper("Business Scrub: Magic Bean", vec![], vec![vec![rand], vec!["OtherMagicBean".into()]]);
        for id in ["ShopItemBusinessScrubMagicBeanInSwamp", "ShopItemBusinessScrubMagicBeanInTown"] {
            b.replace_first_in_requires(id, "OtherMagicBean", &h);
        }
    }

    // Princess needs two bottles when scoops are shuffled (Casual only).
    if b.has("Any Bottle") && b.has("2 Bottles") && b.has("BottleCatchPrincess") {
        let scoops: Vec<String> = mmr_data::game().items.iter().filter(|i| i.item_category.as_deref() == Some("ScoopedItems")).map(|i| i.id.clone()).collect();
        let not_casual = b.cond("LogicMode is not Casual", SettingCondition::not(SettingCondition::Setting { setting: "LogicMode".into(), is: Some("Casual".into()), has: None }));
        let casual = b.cond("LogicMode is Casual", SettingCondition::Setting { setting: "LogicMode".into(), is: Some("Casual".into()), has: None });
        let mut alts = vec![vec![not_casual, "Any Bottle".to_string()]];
        let none: Vec<String> = scoops.iter().map(|sc| b.not_randomized(sc)).collect();
        let mut g = none;
        g.push("Any Bottle".into());
        alts.push(g);
        for sc in &scoops {
            alts.push(vec![casual.clone(), b.randomized(sc), "2 Bottles".into()]);
        }
        let h = b.helper("Princess: Bottles", vec![], alts);
        b.replace_first_in_requires("BottleCatchPrincess", "Any Bottle", &h);
    }

    // Ammo helpers (found or created, as in MMR 2.0).
    let arrows40 = b.find_alternatives(&["UpgradeBigQuiver", "UpgradeBiggestQuiver"]).unwrap_or_else(|| {
        b.helper("40 Arrows", vec![], vec![s(&["UpgradeBigQuiver"]), s(&["UpgradeBiggestQuiver"])])
    });
    let bombchu10 = b.find_alternatives(&["ChestInvertedStoneTowerBombchu10", "ChestLinkTrialBombchu10", "ShopItemBombsBombchu10"]).unwrap_or_else(|| {
        b.helper(
            "10 Bombchu",
            vec![],
            vec![s(&["ChestInvertedStoneTowerBombchu10"]), s(&["ChestLinkTrialBombchu10"]), s(&["ShopItemBombsBombchu10"])],
        )
    });

    // Bring your own ammo. (MMR 2.0 also marks the Bombchu shops/chests as
    // needed on Day 1. That approximated Honey & Darling's "set up on Day 1"
    // time, which the time-of-day model now checks exactly.)
    {
        let off = b.is_off("ByoAmmo");
        let arrows = b.helper("Bring Your Own Ammo: 40 Arrows", vec![], vec![vec![off.clone()], vec![arrows40.clone()]]);
        for id in ["UpgradeBigQuiver", "UpgradeBiggestQuiver", "HeartPieceSwampArchery"] {
            if let Some(e) = b.get(id) {
                e.requires.push(arrows.clone());
            }
        }
        let biggest = b.helper("Bring Your Own Ammo: Biggest Quiver", vec![], vec![vec![off.clone()], s(&["UpgradeBiggestQuiver"])]);
        if let Some(e) = b.get("HeartPieceTownArchery") {
            e.requires.push(biggest);
        }
        let chus = b.helper("Bring Your Own Ammo: 10 Bombchu", vec![], vec![vec![off.clone()], vec![bombchu10.clone()]]);
        if let Some(e) = b.get("HeartPieceHoneyAndDarling") {
            e.requires.push(chus);
        }
        let escort = b.helper("Bring Your Own Ammo: Escort Cremia", vec![], vec![vec![off], s(&["OtherArrow"]), s(&["MaskCircusLeader"])]);
        if let Some(e) = b.get("MaskRomani") {
            e.requires.push(escort);
        }
    }

    progressive_upgrades(&mut b, &arrows40);

    // Bombchu drops add sources of Bombchu.
    {
        let drops = b.is_on("BombchuDrops");
        for id in [
            "ChestIkanaSecretShrineGrotto",
            "ChestTerminaGrottoBombchu",
            "ChestGreatBayCapeGrotto",
            "ChestGraveyardGrotto",
            "ChestToIkanaGrotto",
            "ChestToGoronRaceGrotto",
        ] {
            b.get(&bombchu10).unwrap().any_of.push(vec![drops.clone(), id.to_string()]);
        }
    }

    // Unshuffled Link Trial Bombchu chest: Bombchu aren't needed there.
    {
        let vanilla = b.not_randomized("ChestLinkTrialBombchu10");
        let h = b.helper("Link Trial: 10 Bombchu", vec![], vec![vec![vanilla], vec![bombchu10.clone()]]);
        for id in ["HeartPieceLinkTrial", "Complete Link Trial"] {
            if let Some(e) = b.get(id) {
                for g in &mut e.any_of {
                    if let Some(i) = g.iter().position(|r| *r == bombchu10) {
                        g[i] = h.clone();
                    }
                }
            }
        }
    }

    // Fewer Zora eggs needed.
    if b.has("ZoraEggAll") {
        let all = b.get("ZoraEggAll").unwrap().requires.clone();
        let mut alts = Vec::new();
        for n in 0..7 {
            let eq = b.equals("RequiredZoraEggs", n);
            for c in combinations(&all, n as usize) {
                let mut g = vec![eq.clone()];
                g.extend(c);
                alts.push(g);
            }
        }
        let mut default: Vec<String> = (0..7).map(|n| b.not_equals("RequiredZoraEggs", n)).collect();
        default.extend(all.clone());
        alts.push(default);
        let e = b.get("ZoraEggAll").unwrap();
        e.requires.clear();
        e.any_of = alts;

        if b.has("BottleCatchEgg") {
            let for_fish = b.helper("All Zora Eggs For Fish", all, vec![]);
            let egg = b.randomized("BottleCatchEgg");
            let fish = b.randomized("BottleCatchFish");
            let negations: Vec<String> = vec![]; // BottleCatchEgg has alternatives already in bundled logic
            for n in 0..7 {
                let eq = b.equals("RequiredZoraEggs", n);
                let conds = vec![eq, egg.clone(), fish.clone()];
                b.append_group("BottleCatchEgg", &conds, vec!["BottleCatchEgg".into(), for_fish.clone()], negations.clone());
            }
        }
    }

    time_rules(&mut b.file);
    b.file
}

/// MMR 2.0's time rules that lived in its placement code, as logic.
fn time_rules(file: &mut LogicFile) {
    let game = mmr_data::game();
    for e in &mut file.entries {
        // The Moon and the Clock Tower roof: no way back within the cycle.
        let region = game.item(&e.id).filter(|i| i.is_location()).and_then(|i| i.region.as_deref());
        if matches!(region, Some("TheMoon") | Some("ClockTowerRoof")) {
            e.time.no_return = true;
        }
        // Doors and grottos with times: those are opening hours.
        if game.item(&e.id).is_some_and(|i| i.entrance) && !e.time.available.is_empty() {
            e.time.opening_hours = true;
        }
        match e.id.as_str() {
            // The Goron Elder can't teach the lullaby once spring has come.
            "SongLullabyIntro" => e.before = s(&["AreaSnowheadTempleClear"]),
            // Saving the old lady: the Kafei quest items only count when
            // what's found here doesn't last (MMR 2.0 ignored them for
            // permanent items).
            "MaskBlast" | "NotebookSaveOldLady" | "UpgradeBigBombBagInBombShop" => {
                for kafei in ["TradeItemKafeiLetter", "TradeItemPendant"] {
                    if let Some(i) = e.requires.iter().position(|r| r == kafei) {
                        e.requires.remove(i);
                        e.temporary_requires.push(kafei.into());
                    }
                }
            }
            _ => {}
        }
    }
}

/// Progressive upgrades as rules: every place MMR 2.0 would swap an upgrade
/// for a tier count gets a helper "X (progressive)" = X when progressive
/// upgrades are off, the tier count when on.
fn progressive_upgrades(b: &mut Baker, arrows40: &str) {
    let off = b.is_off("ProgressiveUpgrades");
    let on = b.is_on("ProgressiveUpgrades");

    // Redefine an existing "any of" helper as: its old alternatives when
    // off, `prog` when on.
    let conditional = |b: &mut Baker, id: &str, prog: Vec<Vec<String>>| {
        let (off, on) = (off.clone(), on.clone());
        let e = b.get(id).unwrap();
        let mut alts: Vec<Vec<String>> = e.any_of.iter().map(|g| [vec![off.clone()], g.clone()].concat()).collect();
        alts.extend(prog.into_iter().map(|g| [vec![on.clone()], g].concat()));
        e.any_of = alts;
    };

    let quivers = s(&["ItemBow", "UpgradeBigQuiver", "UpgradeBiggestQuiver"]);
    conditional(b, arrows40, combinations(&quivers, 2));
    let arrows50 = b.helper("50 Arrows", quivers.clone(), vec![]);

    let bags = s(&["ItemBombBag", "UpgradeBigBombBag", "UpgradeBiggestBombBag"]);
    let bombs20 = b.find_alternatives(&["ItemBombBag", "UpgradeBigBombBag", "UpgradeBiggestBombBag"]);
    let bombs30 = match b.find_alternatives(&["UpgradeBigBombBag", "UpgradeBiggestBombBag"]) {
        Some(id) => {
            conditional(b, &id, combinations(&bags, 2));
            id
        }
        None => b.helper("30 Bombs", vec![], combinations(&bags, 2)),
    };
    let bombs40 = b.helper("40 Bombs", bags.clone(), vec![]);

    let swords = s(&["StartingSword", "UpgradeRazorSword", "UpgradeGildedSword"]);
    let sword1 = b.find_alternatives(&["StartingSword", "UpgradeRazorSword", "UpgradeGildedSword"]);
    let sword2 = match b.find_alternatives(&["UpgradeRazorSword", "UpgradeGildedSword"]) {
        Some(id) => {
            conditional(b, &id, combinations(&swords, 2));
            id
        }
        None => b.helper("Sword Level 2", vec![], combinations(&swords, 2)),
    };
    let sword3 = b.helper("Sword Level 3", swords.clone(), vec![]);

    let wallets = s(&["UpgradeAdultWallet", "UpgradeGiantWallet", "UpgradeRoyalWallet"]);
    let wallets200 = b.find_alternatives(&["UpgradeAdultWallet", "UpgradeGiantWallet", "UpgradeRoyalWallet"]);
    let wallets500 = match b.find_alternatives(&["UpgradeGiantWallet", "UpgradeRoyalWallet"]) {
        Some(id) => {
            conditional(b, &id, combinations(&wallets, 2));
            id
        }
        None => b.helper("Wallet Level 2", vec![], combinations(&wallets, 2)),
    };
    let wallets999 = b.helper("Wallet Level 3", wallets.clone(), vec![]);

    let magic_any = b.find_alternatives(&["FairyMagic", "FairyDoubleMagic"]);
    let magic_large = b.helper("Large Magic", s(&["FairyMagic", "FairyDoubleMagic"]), vec![]);
    let lullaby_any = b.find_alternatives(&["SongLullaby", "SongLullabyIntro"]);
    let lullaby_full = b.helper("Full Lullaby", s(&["SongLullaby", "SongLullabyIntro"]), vec![]);

    // "X (progressive)": X when off, the tier when on.
    let mut wrappers: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut wrap = |b: &mut Baker, item: &str, tier: &str| -> String {
        wrappers
            .entry((item.to_string(), tier.to_string()))
            .or_insert_with(|| {
                let id = format!("{item} (progressive)");
                b.helper(&id, vec![], vec![vec![off.clone(), item.to_string()], vec![on.clone(), tier.to_string()]])
            })
            .clone()
    };

    // Same swaps, same exclusions as MMR 2.0; generated helpers are never
    // rewritten (they're either tiers themselves or already account for it).
    let is = |id: &str, other: &Option<String>| other.as_deref() == Some(id);
    let ids: Vec<String> = b.file.entries.iter().map(|e| e.id.clone()).filter(|id| !b.generated.contains(id) || is_byo(id)).collect();
    for id in ids {
        let mut req_swaps: Vec<(&str, String)> = Vec::new();
        let mut grp_swaps: Vec<(&str, String)> = Vec::new();
        if id != lullaby_full {
            req_swaps.push(("SongLullaby", lullaby_full.clone()));
        }
        if !is(&id, &lullaby_any) {
            grp_swaps.push(("SongLullaby", lullaby_full.clone()));
        }
        if id != magic_large {
            req_swaps.push(("FairyDoubleMagic", magic_large.clone()));
        }
        if !is(&id, &magic_any) {
            grp_swaps.push(("FairyDoubleMagic", magic_large.clone()));
        }
        if id != wallets999 {
            req_swaps.push(("UpgradeRoyalWallet", wallets999.clone()));
        }
        if !is(&id, &wallets200) && id != wallets500 {
            grp_swaps.push(("UpgradeRoyalWallet", wallets999.clone()));
            grp_swaps.push(("UpgradeGiantWallet", wallets500.clone()));
        }
        if id != sword3 {
            req_swaps.push(("UpgradeGildedSword", sword3.clone()));
        }
        if !is(&id, &sword1) && id != sword2 {
            grp_swaps.push(("UpgradeGildedSword", sword3.clone()));
            grp_swaps.push(("UpgradeRazorSword", sword2.clone()));
        }
        if id != bombs40 {
            req_swaps.push(("UpgradeBiggestBombBag", bombs40.clone()));
        }
        if !is(&id, &bombs20) && id != bombs30 && id != "OtherExplosive" {
            grp_swaps.push(("UpgradeBiggestBombBag", bombs40.clone()));
            grp_swaps.push(("UpgradeBigBombBag", bombs30.clone()));
        }
        if id != arrows50 {
            req_swaps.push(("UpgradeBiggestQuiver", arrows50.clone()));
        }
        if id != arrows40 && id != "OtherArrow" {
            grp_swaps.push(("UpgradeBiggestQuiver", arrows50.clone()));
            grp_swaps.push(("UpgradeBigQuiver", arrows40.to_string()));
        }
        let req_w: Vec<(&str, String)> = req_swaps.iter().map(|(x, t)| (*x, wrap(b, x, t))).collect();
        let grp_w: Vec<(&str, String)> = grp_swaps.iter().map(|(x, t)| (*x, wrap(b, x, t))).collect();
        let e = b.get(&id).unwrap();
        for (x, w) in &req_w {
            if let Some(i) = e.requires.iter().position(|r| r == x) {
                e.requires[i] = w.clone();
            }
        }
        for g in &mut e.any_of {
            for (x, w) in &grp_w {
                if let Some(i) = g.iter().position(|r| r == x) {
                    g[i] = w.clone();
                }
            }
        }
    }
}

/// Bring-your-own-ammo helpers stand in for references MMR 2.0 added
/// directly to entries, so the progressive pass must still see them.
fn is_byo(id: &str) -> bool {
    id.starts_with("Bring Your Own Ammo:")
}

/// Note on every entry [`bake`] creates.
pub const GENERATED_NOTE: &str = "Built-in settings rule (older MMR versions applied this in code).";

/// Logic specialized for one set of settings: alternatives whose setting
/// conditions are false are dropped, true conditions are removed, entries
/// that can't be satisfied are emptied, and built-in rule helpers nothing
/// uses any more are removed. The result references exactly the items MMR
/// 2.0's rewritten logic referenced, which matters for placement (items the
/// logic never mentions are treated as junk).
/// What [`specialize`] makes an entry require when it can never be met.
pub const IMPOSSIBLE: &str = "(impossible with these settings)";

pub fn specialize(file: &LogicFile, values: &serde_json::Map<String, Value>) -> LogicFile {
    use std::collections::BTreeSet;
    let truth: BTreeMap<&str, bool> =
        file.entries.iter().filter_map(|e| e.setting.as_ref().map(|c| (e.id.as_str(), c.eval(values)))).collect();
    let mut out = file.clone();
    for e in &mut out.entries {
        if e.setting.is_some() {
            continue;
        }
        if e.requires.iter().any(|r| truth.get(r.as_str()) == Some(&false)) {
            // Can never be satisfied with these settings.
            e.requires = vec![IMPOSSIBLE.into()];
            e.any_of.clear();
            continue;
        }
        e.requires.retain(|r| truth.get(r.as_str()) != Some(&true));
        if !e.any_of.is_empty() {
            let groups: Vec<Vec<String>> = e
                .any_of
                .iter()
                .filter(|g| !g.iter().any(|r| truth.get(r.as_str()) == Some(&false)))
                .map(|g| g.iter().filter(|r| truth.get(r.as_str()) != Some(&true)).cloned().collect())
                .collect();
            if groups.is_empty() {
                e.requires = vec![IMPOSSIBLE.into()];
                e.any_of.clear();
            } else if groups.iter().any(|g: &Vec<String>| g.is_empty()) {
                e.any_of.clear();
            } else {
                e.any_of = groups;
            }
        }
    }
    // Drop setting entries and unused built-in helpers.
    loop {
        let used: BTreeSet<&str> = out.entries.iter().flat_map(|e| e.references()).collect();
        let before = out.entries.len();
        let keep: Vec<bool> = out
            .entries
            .iter()
            .map(|e| {
                let generated = e.setting.is_some() && e.id.starts_with("Setting: ") || e.note.as_deref() == Some(GENERATED_NOTE);
                !generated || used.contains(e.id.as_str())
            })
            .collect();
        let mut i = 0;
        out.entries.retain(|_| {
            i += 1;
            keep[i - 1]
        });
        if out.entries.len() == before {
            break;
        }
    }
    out
}
