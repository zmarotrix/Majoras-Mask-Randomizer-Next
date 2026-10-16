//! Every bundled logic file converts to regions automatically and behaves
//! exactly like the original on every entry.

use mmr_logic::{equivalence, regionize, validate};
use std::collections::BTreeMap;

fn check(logic_id: &str) {
    let old = mmr_logic::bundled(logic_id).unwrap();
    let (regions, report) = regionize::to_regions_auto(&old);
    let new = regions.compile();

    let errors: Vec<_> = validate::validate(&new, None).into_iter().filter(|i| i.severity == validate::Severity::Error).collect();
    assert!(errors.is_empty(), "{logic_id}: compiled logic has errors: {errors:#?}");

    let game = mmr_data::game();
    let items: Vec<String> = game.items.iter().filter(|i| i.name.is_some()).map(|i| i.id.clone()).collect();
    let tricks: Vec<String> = old.entries.iter().filter(|e| e.trick.is_some()).map(|e| e.id.clone()).collect();
    let settings: Vec<_> = mmr_settings::presets().iter().map(|p| mmr_settings::apply_preset(p, None)).collect();
    let scenarios = equivalence::scenarios(4000, 0xC0FFEE, &items, &tricks, &settings);

    let pairs: Vec<(String, String)> =
        old.entries.iter().map(|e| (e.id.clone(), report.replaced.get(&e.id).cloned().unwrap_or_else(|| e.id.clone()))).collect();
    let result = equivalence::compare_pairs(&old, &new, &pairs, &scenarios);
    let mut by_id: BTreeMap<&str, usize> = BTreeMap::new();
    for m in &result.mismatches {
        *by_id.entry(m.id.as_str()).or_default() += 1;
    }
    println!(
        "{logic_id}: {} regions ({} replace their old entry, {} inferred), {} checks + {} events in regions, {} checks unmapped",
        report.regions,
        report.equivalent_regions,
        report.inferred.len(),
        report.locations_in_regions,
        report.events_in_regions,
        report.unmapped_locations.len()
    );
    assert!(result.is_equivalent(), "{logic_id}: {} mismatches over {} scenarios: {by_id:#?}", result.mismatches.len(), result.scenarios);
}

#[test]
fn casual_converts_exactly() {
    check("casual");
}

#[test]
fn glitched_converts_exactly() {
    check("glitched");
}

/// The saved region files must stay equivalent to the entry files.
#[test]
fn saved_region_files_match() {
    for id in ["casual", "glitched"] {
        let old = mmr_logic::bundled(id).unwrap();
        let json = std::fs::read_to_string(format!("{}/../../data/logic/{id}.regions.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let (new, format) = mmr_logic::load(&json).unwrap();
        assert_eq!(format, mmr_logic::SourceFormat::Regions);
        let (_, report) = regionize::to_regions_auto(&old);
        let items: Vec<String> = mmr_data::game().items.iter().filter(|i| i.name.is_some()).map(|i| i.id.clone()).collect();
        let scenarios = equivalence::scenarios(500, 7, &items, &[], &[]);
        let pairs: Vec<(String, String)> =
            old.entries.iter().map(|e| (e.id.clone(), report.replaced.get(&e.id).cloned().unwrap_or_else(|| e.id.clone()))).collect();
        assert!(equivalence::compare_pairs(&old, &new, &pairs, &scenarios).is_equivalent(), "{id}.regions.json drifted");
    }
}
