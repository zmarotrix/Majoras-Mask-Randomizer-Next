//! Settings rules built into the logic must behave exactly like MMR 2.0's
//! code-based rewrites, for any settings.

use mmr_logic::eval::{Evaluator, Scenario};
use mmr_logic::{equivalence, legacy_settings, settings_rules};
use serde_json::{json, Map, Value};

fn random_settings(n: usize) -> Vec<Map<String, Value>> {
    let presets: Vec<_> = mmr_settings::presets().iter().map(|p| mmr_settings::apply_preset(p, None)).collect();
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let victory = [
        "DirectToCredits", "CantFightMajora", "Fairies", "SkullTokens", "NonTransformationMasks", "TransformationMasks", "Notebook", "Hearts",
        "OneBossRemains", "TwoBossRemains", "ThreeBossRemains", "FourBossRemains",
    ];
    let toggles = ["ShopItemBusinessScrubMagicBean", "ChestLinkTrialBombchu10", "BottleCatchEgg", "BottleCatchFish", "BottleCatchPrincess"];
    (0..n)
        .map(|i| {
            let mut v = presets[i % presets.len()].clone();
            for key in ["ProgressiveUpgrades", "ByoAmmo", "FreeHints", "FreeGaroHints", "BombchuDrops"] {
                v.insert(key.into(), json!(next() % 2 == 0));
            }
            v.insert("RequiredBossRemains".into(), json!(next() % 5));
            v.insert("RequiredZoraEggs".into(), json!(1 + next() % 7));
            let goals: Vec<&str> = victory.iter().copied().filter(|_| next() % 4 == 0).collect();
            v.insert("VictoryMode".into(), json!(goals));
            let mut pool: Vec<Value> = v["CustomItemList"].as_array().unwrap().clone();
            for t in toggles {
                pool.retain(|x| x.as_str() != Some(t));
                if next() % 2 == 0 {
                    pool.push(json!(t));
                }
            }
            v.insert("CustomItemList".into(), Value::Array(pool));
            v.insert("LogicMode".into(), json!(if next() % 3 == 0 { "Glitched" } else { "Casual" }));
            v
        })
        .collect()
}

#[test]
fn baked_settings_match_code_rewrites() {
    let game = mmr_data::game();
    let items: Vec<String> = game.items.iter().filter(|i| i.name.is_some()).map(|i| i.id.clone()).collect();
    for logic_id in ["casual", "glitched"] {
        let raw = std::fs::read_to_string(format!("{}/../../data/logic/legacy-{logic_id}.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let v1 = mmr_logic::load_legacy_raw(&raw).unwrap();
        let baked = settings_rules::bake(&v1);
        let tricks: Vec<String> = v1.entries.iter().filter(|e| e.trick.is_some()).map(|e| e.id.clone()).collect();
        let settings = random_settings(150);
        let baked_eval = Evaluator::new(&baked);
        let plain_eval = Evaluator::new(&v1);
        let mut mismatches = std::collections::BTreeMap::<String, usize>::new();
        let mut plain_differs = std::collections::BTreeSet::<String>::new();
        for (i, s) in settings.iter().enumerate() {
            let mut patched = v1.clone();
            legacy_settings::apply(&mut patched, s);
            let patched_eval = Evaluator::new(&patched);
            // Several inventories per settings combination.
            for sc in equivalence::scenarios(12, i as u64 + 1, &items, &tricks, &[]) {
                let sc = Scenario { settings: s.clone(), ..sc };
                let (a, b) = (patched_eval.evaluate(&sc), baked_eval.evaluate(&sc));
                let plain = plain_eval.evaluate(&sc);
                for e in &v1.entries {
                    if a.is_satisfied(&e.id) != b.is_satisfied(&e.id) {
                        *mismatches.entry(e.id.clone()).or_default() += 1;
                    }
                    if a.is_satisfied(&e.id) != plain.is_satisfied(&e.id) {
                        plain_differs.insert(e.id.clone());
                    }
                }
            }
        }
        assert!(mismatches.is_empty(), "{logic_id}: {mismatches:#?}");
        // The scenarios must actually exercise the rewrites: without them,
        // plenty of entries come out differently.
        println!("{logic_id}: rewrites change {} entries in these scenarios", plain_differs.len());
        assert!(plain_differs.len() > 50, "{logic_id}: only {} entries affected; scenarios too weak", plain_differs.len());
    }
}
