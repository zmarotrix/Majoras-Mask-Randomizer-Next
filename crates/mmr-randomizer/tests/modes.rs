//! Vanilla and No Logic modes.

use mmr_randomizer::{generate, Seed, Style};

fn values(mode: &str) -> mmr_settings::Values {
    let p = mmr_settings::presets().into_iter().find(|p| p.id == "season-2-standard").unwrap();
    let mut v = mmr_settings::apply_preset(&p, None);
    v.insert("LogicMode".into(), mode.into());
    v
}

#[test]
fn vanilla_mode_shuffles_nothing() {
    let logic = mmr_logic::bundled("casual").unwrap();
    let out = generate(&logic, &values("Vanilla"), Seed(3), Style::Random).unwrap();
    assert!(out.placements.is_empty());
    assert!(out.layout.iter().all(|e| e.location == e.item && !e.randomized));
}

#[test]
fn no_logic_mode_places_everything() {
    let logic = mmr_logic::bundled("casual").unwrap();
    let v = values("NoLogic");
    let a = generate(&logic, &v, Seed(3), Style::Random).unwrap();
    let b = generate(&logic, &v, Seed(3), Style::Random).unwrap();
    assert!(!a.placements.is_empty());
    assert_eq!(mmr_randomizer::fingerprint(&a), mmr_randomizer::fingerprint(&b));
}

#[test]
fn entrance_shuffle_makes_beatable_seeds() {
    let logic = mmr_logic::bundled("casual").unwrap();
    let mut v = values("Casual");
    v.insert("EntranceMode".into(), serde_json::json!(["DungeonEntrances", "BossRooms", "Grottos", "SimpleInteriors"]));
    for seed in 1..=6 {
        let out = generate(&logic, &v, Seed(seed), Style::Random).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        assert!(out.entrances.len() >= 4 + 4 + 4, "seed {seed}: {} entrances", out.entrances.len());
        assert!(out.entrances.iter().any(|(e, t)| e != t));
    }
}

#[test]
fn item_plando_places_chosen_items() {
    let logic = mmr_logic::bundled("casual").unwrap();
    let v = values("Casual");
    let plando = mmr_randomizer::plando::parse(r#"[
        { "Name": "Hookshot in the bank", "ItemList": ["ItemHookshot"], "CheckList": ["HeartPieceBank", "UpgradeAdultWallet"] },
        { "Name": "Two masks", "ItemList": ["MaskBunnyHood", "MaskKeaton", "MaskPostmanHat"], "CheckList": ["ChestInnGuestRoom", "ChestInnStaffRoom", "HeartPieceNotebookMayor"], "ItemDrawCount": 2 }
    ]"#).unwrap();
    for seed in 1..=3 {
        let out = mmr_randomizer::generate_with(&logic, &v, Seed(seed), Style::Random, &plando).unwrap();
        let at = |item: &str| out.layout.iter().find(|e| e.item == item).map(|e| e.location.clone()).unwrap();
        assert!(["HeartPieceBank", "UpgradeAdultWallet"].contains(&at("ItemHookshot").as_str()));
        let masks = ["MaskBunnyHood", "MaskKeaton", "MaskPostmanHat"].iter().filter(|m| ["ChestInnGuestRoom", "ChestInnStaffRoom", "HeartPieceNotebookMayor"].contains(&at(m).as_str())).count();
        assert!(masks >= 2, "seed {seed}: {masks} masks placed");
    }
    let bad = mmr_randomizer::plando::parse(r#"[{ "Name": "Nowhere", "ItemList": ["ItemHookshot"], "CheckList": ["NotACheck"] }]"#).unwrap();
    assert!(mmr_randomizer::generate_with(&logic, &v, Seed(1), Style::Random, &bad).is_err());
}

#[test]
fn hint_caps_limit_each_tier() {
    let logic = mmr_logic::bundled("casual").unwrap();
    let groups: std::collections::BTreeMap<String, Vec<String>> =
        serde_json::from_str(include_str!("../../../data/rom/combinable_hints.json").trim_start_matches('\u{FEFF}')).unwrap();
    let group_of = |l: &str| groups.iter().find(|(_, ls)| ls.iter().any(|x| x == l)).map_or_else(|| l.to_string(), |(g, _)| g.clone());
    for preset in ["modern", "advanced"] {
        let p = mmr_settings::presets().into_iter().find(|p| p.id == preset).unwrap();
        let v = mmr_settings::apply_preset(&p, None);
        let tiers: Vec<Vec<String>> = serde_json::from_value(v["OverrideHintPriorities"].clone()).unwrap();
        let caps: Vec<usize> = serde_json::from_value(v["OverrideHintItemCaps"].clone()).unwrap();
        for seed in 1..=4 {
            let out = generate(&logic, &v, Seed(seed), Style::Random).unwrap();
            for (tier, &cap) in tiers.iter().zip(&caps).filter(|(_, &c)| c > 0) {
                let open: std::collections::BTreeSet<String> =
                    tier.iter().filter(|l| !out.junk_locations.contains(l)).map(|l| group_of(l)).collect();
                assert!(open.len() <= cap, "{preset} seed {seed}: {} open groups, cap {cap}", open.len());
            }
        }
    }
}

#[test]
fn classic_placement_style() {
    let logic = mmr_logic::bundled("casual").unwrap();
    let v = values("Casual");
    let a = generate(&logic, &v, Seed(8), Style::Classic).unwrap();
    let b = generate(&logic, &v, Seed(8), Style::Classic).unwrap();
    let r = generate(&logic, &v, Seed(8), Style::Random).unwrap();
    assert_eq!(mmr_randomizer::fingerprint(&a), mmr_randomizer::fingerprint(&b));
    assert_ne!(mmr_randomizer::fingerprint(&a), mmr_randomizer::fingerprint(&r));
}
