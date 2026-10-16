//! The spoiler log, JSON and tracker for an MMR Next seed.

use std::path::PathBuf;

#[test]
fn spoiler_log_and_tracker() {
    let rom = std::env::var_os("MMR_TEST_ROM").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Zelda - Majora's Mask.z64"));
    let Ok(mut input) = std::fs::read(&rom) else {
        eprintln!("skipping: no ROM");
        return;
    };
    mmr_rom::prepare(&mut input).unwrap();
    let p = mmr_settings::presets().into_iter().find(|p| p.id == "season-2-standard").unwrap();
    let mut values = mmr_settings::apply_preset(&p, None);
    for (k, v) in [
        ("GossipHintStyle", serde_json::json!("Competitive")),
        ("PriceMode", serde_json::json!(["Purchases"])),
        ("EntranceMode", serde_json::json!(["DungeonEntrances"])),
        ("Cosmetic.Music", serde_json::json!("Random")),
    ] {
        values.insert(k.into(), v);
    }
    let logic = mmr_logic::bundled("casual").unwrap();
    let built = mmr_build::seed::build_seed(&input, &values, &logic, mmr_randomizer::Seed(2026), mmr_randomizer::Style::Random, &Default::default()).unwrap();
    let s = mmr_build::spoiler::build(&built.outcome, &built.plan, &values, &logic, mmr_build::spoiler::BuildInfo { hash: &built.hash, icons: &built.hash_icons, music: &built.music, item_icons: &built.item_icons });
    assert_eq!(s.hash_icons.len(), 5);
    assert!(!s.hints.is_empty() && !s.entrances.is_empty() && !s.prices.is_empty() && !s.music.is_empty());
    assert!(s.checks.iter().any(|c| c.required));
    let text = mmr_build::spoiler::text(&s);
    for section in ["Way of the Hero", "Playthrough", "Entrances", "Hints", "Checks by area", "Where each item is", "Prices", "Music"] {
        assert!(text.contains(section), "no {section} section");
    }
    let html = mmr_build::spoiler::html(&s);
    assert!(html.contains("\"seed\":\"2026\"") && !html.contains("/*SPOILER_DATA*/"));
    // Same seed, same spoiler.
    let again = mmr_build::spoiler::build(&built.outcome, &built.plan, &values, &logic, mmr_build::spoiler::BuildInfo { hash: &built.hash, icons: &built.hash_icons, music: &built.music, item_icons: &built.item_icons });
    assert_eq!(mmr_build::spoiler::text(&again), text);
}

/// Music plando files in the music folder pin songs to slots.
#[test]
fn music_plando() {
    let rom = std::env::var_os("MMR_TEST_ROM").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Zelda - Majora's Mask.z64"));
    let Ok(mut input) = std::fs::read(&rom) else {
        eprintln!("skipping: no ROM");
        return;
    };
    mmr_rom::prepare(&mut input).unwrap();
    let mut values = mmr_settings::defaults();
    values.insert("Cosmetic.Music".into(), serde_json::json!("Random"));
    let logic = mmr_logic::bundled("casual").unwrap();
    let plando = r#"[{ "Name": "Theme on the title", "SongsList": ["mmr-theme.mmrs"], "SlotsList": ["mm-fileselect"] }]"#;
    let options = mmr_build::Options { music: vec![mmr_build::music::MusicFile { name: "My_MusicPlando.json".into(), data: plando.as_bytes().to_vec().into() }], ..Default::default() };
    for seed in 1..=2 {
        let built = mmr_build::seed::build_seed(&input, &values, &logic, mmr_randomizer::Seed(seed), mmr_randomizer::Style::Random, &options).unwrap();
        let at = built.music.iter().find(|p| p.slot == "mm-fileselect").map(|p| p.song.clone());
        assert_eq!(at.as_deref(), Some("mmr-theme"), "seed {seed}");
    }
}
