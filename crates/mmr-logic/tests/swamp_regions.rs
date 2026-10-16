//! Proof of concept: the Southern Swamp and Woodfall Temple as regions must
//! behave exactly like the original logic.

use mmr_logic::{convert, equivalence};
use std::collections::BTreeSet;

#[test]
fn swamp_regions_are_equivalent() {
    let old = mmr_logic::bundled("casual").unwrap();
    let (regions, report) = convert::to_regions(&old, &convert::swamp_spec());
    assert!(report.moved.len() > 50, "only {} checks moved", report.moved.len());
    let new = regions.compile();

    let game = mmr_data::game();
    let items: Vec<String> = game.items.iter().filter(|i| i.name.is_some()).map(|i| i.id.clone()).collect();
    let tricks: Vec<String> = old.entries.iter().filter(|e| e.trick.is_some()).map(|e| e.id.clone()).collect();
    let settings: Vec<_> = mmr_settings::presets().iter().map(|p| mmr_settings::apply_preset(p, None)).collect();
    let scenarios = equivalence::scenarios(3000, 0x5eed, &items, &tricks, &settings);

    let ids: BTreeSet<String> = old.entries.iter().map(|e| e.id.clone()).collect();
    let result = equivalence::compare(&old, &new, &ids, &scenarios);
    let mut by_id: std::collections::BTreeMap<&str, usize> = Default::default();
    for m in &result.mismatches {
        *by_id.entry(m.id.as_str()).or_default() += 1;
    }
    assert!(result.is_equivalent(), "{} mismatches over {} scenarios: {by_id:#?}", result.mismatches.len(), result.scenarios);
}

/// The checker must notice a real difference: drop one requirement from the
/// exit into Woodfall Temple and it should report mismatches.
#[test]
fn equivalence_check_catches_changes() {
    let old = mmr_logic::bundled("casual").unwrap();
    let (mut regions, _) = convert::to_regions(&old, &convert::swamp_spec());
    let woodfall = regions.regions.iter_mut().find(|r| r.name == "Woodfall").unwrap();
    woodfall.exits[0].rule = mmr_logic::region::Rule::Ref("MaskDeku".into()); // forgot Sonata
    let new = regions.compile();

    let items: Vec<String> = mmr_data::game().items.iter().filter(|i| i.name.is_some()).map(|i| i.id.clone()).collect();
    let scenarios = equivalence::scenarios(300, 1, &items, &[], &[]);
    let ids: BTreeSet<String> = old.entries.iter().map(|e| e.id.clone()).collect();
    let result = equivalence::compare(&old, &new, &ids, &scenarios);
    assert!(!result.is_equivalent());
    assert!(result.mismatches.iter().any(|m| m.id == "ItemBow" && m.new && !m.old), "should find the Hero's Bow reachable too early");
}

#[test]
fn print_swamp_regions() {
    let old = mmr_logic::bundled("casual").unwrap();
    let (regions, report) = convert::to_regions(&old, &convert::swamp_spec());
    let mut counts: std::collections::BTreeMap<&str, usize> = Default::default();
    for (_, r) in &report.moved {
        *counts.entry(r.as_str()).or_default() += 1;
    }
    println!("{counts:#?}");
    std::fs::write(std::env::temp_dir().join("swamp-regions.txt"), mmr_logic::region::text::regions(&regions.regions)).unwrap();
}
