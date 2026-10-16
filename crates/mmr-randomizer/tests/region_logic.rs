//! Converting logic to regions must not change a single seed, nor which
//! items are required (what Way of the Hero hints are built from).

use mmr_randomizer::{fingerprint, generate, required_items, Seed, Style};

#[test]
fn region_logic_gives_identical_seeds_and_hints() {
    for logic_id in ["casual", "glitched"] {
        let old = mmr_logic::bundled(logic_id).unwrap();
        let new = mmr_logic::regionize::to_regions_auto(&old).0.compile();
        for preset in ["season-1-standard", "season-2-standard", "season-3-standard", "modern", "classic", "blitz", "sgl-2026"] {
            let p = mmr_settings::presets().into_iter().find(|p| p.id == preset).unwrap();
            let mut values = mmr_settings::apply_preset(&p, None);
            if logic_id == "glitched" {
                values.insert("LogicMode".into(), "Glitched".into());
            }
            for seed in 1..=8u64 {
                for style in [Style::Random, Style::Bespoke] {
                    let a = generate(&old, &values, Seed(seed), style);
                    let b = generate(&new, &values, Seed(seed), style);
                    match (a, b) {
                        (Ok(a), Ok(b)) => {
                            assert_eq!(fingerprint(&a), fingerprint(&b), "{logic_id}/{preset}/{seed}/{style:?}: placements differ");
                            assert_eq!(
                                required_items(&old, &values, &a),
                                required_items(&new, &values, &b),
                                "{logic_id}/{preset}/{seed}/{style:?}: required items differ"
                            );
                        }
                        (Err(a), Err(b)) => assert_eq!(a.to_string(), b.to_string()),
                        (a, b) => panic!("{logic_id}/{preset}/{seed}/{style:?}: one failed: {:?} / {:?}", a.err(), b.err()),
                    }
                }
            }
        }
    }
}

/// With entrance randomization, logic with regions still makes seeds (its
/// ways back out of moved dungeons aren't rewired yet, so seeds can differ
/// from the item logic's).
#[test]
fn region_logic_handles_entrance_randomization() {
    let old = mmr_logic::bundled("casual").unwrap();
    let new = mmr_logic::regionize::to_regions_auto(&old).0.compile();
    let p = mmr_settings::presets().into_iter().find(|p| p.id == "blitz-s2").unwrap();
    let values = mmr_settings::apply_preset(&p, None);
    for seed in 1..=4u64 {
        generate(&new, &values, Seed(seed), Style::Random).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
    }
}
