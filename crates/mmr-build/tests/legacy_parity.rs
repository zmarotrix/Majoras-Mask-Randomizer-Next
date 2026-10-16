//! Stage-by-stage comparison with the old builder.
//!
//! Reference ROMs and plans come from `tools/legacy-harness` (`build`
//! command), in `MMR_LEGACY_OUT` or `legacy-out` next to the repository, named
//! `<case>-<stage>.z64` / `.plan.json`. Missing references are skipped.

use mmr_build::{build, Plan, Stage};
use std::path::PathBuf;

fn beside_repo(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..").join(name)
}

fn check(case: &str, stage: Stage) {
    let out = std::env::var_os("MMR_LEGACY_OUT").map(PathBuf::from).unwrap_or_else(|| beside_repo("legacy-out"));
    let rom_path = std::env::var_os("MMR_TEST_ROM").map(PathBuf::from).unwrap_or_else(|| beside_repo("Zelda - Majora's Mask.z64"));
    let reference = out.join(format!("{case}-{}.z64", stage.name()));
    let (Ok(mut input), Ok(theirs), Ok(plan)) = (std::fs::read(&rom_path), std::fs::read(&reference), std::fs::read_to_string(reference.with_extension("plan.json"))) else {
        eprintln!("skipping {case} {}: reference files missing", stage.name());
        return;
    };
    mmr_rom::prepare(&mut input).unwrap();
    let plan: Plan = serde_json::from_str(&plan).unwrap();
    let ours = build(&input, &plan, stage).unwrap();
    assert_eq!(ours.len(), theirs.len(), "ROM sizes differ");
    // The old builder signs ROMs with the build time; the checksum covers it.
    let ignored = |i: usize| (0x10..0x18).contains(&i) || (0x1A4D0..0x1A500).contains(&i);
    let diffs: Vec<usize> = (0..ours.len()).filter(|&i| !ignored(i) && ours[i] != theirs[i]).collect();
    if !diffs.is_empty() {
        let mut ours_rom = mmr_rom::Rom::read(&ours).unwrap();
        let mut theirs_rom = mmr_rom::Rom::read(&theirs).unwrap();
        let mut report = Vec::new();
        for i in 0..ours_rom.files.len().max(theirs_rom.files.len()) {
            let a = ours_rom.files.get(i).map(|f| (f.addr, f.end));
            let b = theirs_rom.files.get(i).map(|f| (f.addr, f.end));
            if a != b {
                report.push(format!("file {i}: ranges {a:x?} vs {b:x?}"));
                continue;
            }
            if ours_rom.files[i].cmp_addr.is_none() || i < 2 {
                continue;
            }
            let (x, y) = (ours_rom.file(i).unwrap().clone(), theirs_rom.file(i).unwrap().clone());
            if x != y {
                let count = (0..x.len().min(y.len())).filter(|&k| x[k] != y[k]).count();
                let samples: Vec<String> = (0..x.len().min(y.len())).filter(|&k| x[k] != y[k]).take(12).map(|k| format!("+{k:#X}: {:02X} vs {:02X}", x[k], y[k])).collect();
                report.push(format!("file {i} ({:#X}): {count} bytes differ, lengths {} vs {}; {}", ours_rom.files[i].addr, x.len(), y.len(), samples.join(", ")));
            }
        }
        panic!("{case} {}: {} ROM bytes differ, first at {:#X}\n{}", stage.name(), diffs.len(), diffs[0], report.join("\n"));
    }
}

#[test]
fn casual_player_model() {
    check("casual1", Stage::PlayerModel);
}

#[test]
fn message_table_round_trip() {
    let out = std::env::var_os("MMR_LEGACY_OUT").map(PathBuf::from).unwrap_or_else(|| beside_repo("legacy-out"));
    let rom_path = std::env::var_os("MMR_TEST_ROM").map(PathBuf::from).unwrap_or_else(|| beside_repo("Zelda - Majora's Mask.z64"));
    let (Ok(mut input), Ok(theirs)) = (std::fs::read(&rom_path), std::fs::read(out.join("messages.z64"))) else {
        eprintln!("skipping message round trip: reference files missing");
        return;
    };
    mmr_rom::prepare(&mut input).unwrap();
    let mut rom = mmr_rom::Rom::read(&input).unwrap();
    let mut table = mmr_build::message::MessageTable::read_default(&mut rom).unwrap();
    table.write_default(&mut rom, true).unwrap();
    let ours = rom.build(b"").unwrap();
    let ignored = |i: usize| (0x10..0x18).contains(&i) || (0x1A4D0..0x1A500).contains(&i);
    let diffs = (0..ours.len()).filter(|&i| !ignored(i) && ours[i] != theirs[i]).count();
    assert_eq!(diffs, 0);
}

#[test]
fn casual_hacks() {
    check("casual1", Stage::Hacks);
}

#[test]
fn casual_enemies() {
    check("casual1", Stage::Enemies);
}

#[test]
fn casual_items() {
    check("casual1", Stage::Items);
}

#[test]
fn casual_cutscenes() {
    check("casual1", Stage::Cutscenes);
}

#[test]
fn casual_gimmicks() {
    check("casual1", Stage::Gimmicks);
}

#[test]
fn casual_messages() {
    check("casual1", Stage::Messages);
}

#[test]
fn clock_town_fairy_icon() {
    let out = std::env::var_os("MMR_LEGACY_OUT").map(PathBuf::from).unwrap_or_else(|| beside_repo("legacy-out"));
    let rom_path = std::env::var_os("MMR_TEST_ROM").map(PathBuf::from).unwrap_or_else(|| beside_repo("Zelda - Majora's Mask.z64"));
    let (Ok(mut input), Ok(reference)) = (std::fs::read(&rom_path), std::fs::read_to_string(out.join("fairy-icon.txt"))) else {
        eprintln!("skipping fairy icon: reference files missing");
        return;
    };
    mmr_rom::prepare(&mut input).unwrap();
    let mut rom = mmr_rom::Rom::read(&input).unwrap();
    let data = rom.file(11).unwrap();
    let ours = mmr_build::fairy_icon::clock_town_fairy_icon(&data[0x1B80 + 0xC00..0x1B80 + 0x1800]);
    let hex = reference.lines().find_map(|l| l.strip_prefix("icon ")).unwrap();
    let theirs: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap()).collect();
    let diffs: Vec<String> = (0..ours.len()).filter(|&i| ours[i] != theirs[i]).take(10).map(|i| format!("{i}: {} vs {}", ours[i], theirs[i])).collect();
    assert!(diffs.is_empty(), "{} differ: {}", (0..ours.len()).filter(|&i| ours[i] != theirs[i]).count(), diffs.join(", "));
}

#[test]
fn casual_startup() {
    check("casual1", Stage::Startup);
}

#[test]
fn casual_asm() {
    check("casual1", Stage::Asm);
}

#[test]
fn casual_asm_config() {
    check("casual1", Stage::AsmConfig);
}

#[test]
fn casual_cosmetics() {
    check("casual1", Stage::Cosmetics);
}

#[test]
fn casual_audio() {
    check("casual1", Stage::Audio);
}

/// A seed with MMR Next's default settings builds, and the same seed always
/// gives the same ROM.
#[test]
fn default_settings_seed_builds_reproducibly() {
    let rom_path = std::env::var_os("MMR_TEST_ROM").map(PathBuf::from).unwrap_or_else(|| beside_repo("Zelda - Majora's Mask.z64"));
    let Ok(mut input) = std::fs::read(&rom_path) else {
        eprintln!("skipping: no ROM");
        return;
    };
    mmr_rom::prepare(&mut input).unwrap();
    let values = mmr_settings::defaults();
    let logic = mmr_logic::bundled("casual").unwrap();
    let seed = mmr_randomizer::Seed::parse("parity");
    let a = mmr_build::seed::build_seed(&input, &values, &logic, seed, mmr_randomizer::Style::Random, &Default::default()).unwrap();
    let b = mmr_build::seed::build_seed(&input, &values, &logic, seed, mmr_randomizer::Style::Random, &Default::default()).unwrap();
    assert!(a.rom == b.rom, "same seed, different ROM");
    assert_eq!(mmr_rom::crc::compute(&a.rom), (u32::from_be_bytes(a.rom[0x10..0x14].try_into().unwrap()), u32::from_be_bytes(a.rom[0x14..0x18].try_into().unwrap())));
}

/// MMR Next's own seeds with the features its randomizer adds (prices, Gibdo
/// requests, traps, random music, enemies, entrances) build, reproducibly.
#[test]
fn own_feature_seeds_build_reproducibly() {
    let rom_path = std::env::var_os("MMR_TEST_ROM").map(PathBuf::from).unwrap_or_else(|| beside_repo("Zelda - Majora's Mask.z64"));
    let Ok(mut input) = std::fs::read(&rom_path) else {
        eprintln!("skipping: no ROM");
        return;
    };
    mmr_rom::prepare(&mut input).unwrap();
    let mut values = mmr_settings::defaults();
    for (k, v) in [
        ("PriceMode", serde_json::json!(["Purchases", "Minigames", "Misc"])),
        ("RandomizeGibdoRequirements", serde_json::json!(true)),
        ("TrapAmount", serde_json::json!("Mayhem")),
        ("TrapQuirks", serde_json::json!(true)),
        ("UpdateChests", serde_json::json!(true)),
        ("EnemyMode", serde_json::json!(["Randomized"])),
        ("Cosmetic.Music", serde_json::json!("Random")),
        ("EntranceMode", serde_json::json!(["DungeonEntrances", "BossRooms", "Grottos", "SimpleInteriors"])),
        ("GossipHintStyle", serde_json::json!("Competitive")),
        ("GaroHintStyle", serde_json::json!("Relevant")),
    ] {
        values.insert(k.into(), v);
    }
    let logic = mmr_logic::bundled("casual").unwrap();
    for text in ["features-1", "features-2"] {
        let seed = mmr_randomizer::Seed::parse(text);
        let a = mmr_build::seed::build_seed(&input, &values, &logic, seed, mmr_randomizer::Style::Random, &Default::default()).unwrap();
        let b = mmr_build::seed::build_seed(&input, &values, &logic, seed, mmr_randomizer::Style::Random, &Default::default()).unwrap();
        assert!(a.rom == b.rom, "{text}: same seed, different ROM");
        assert!(a.outcome.prices.iter().all(Option::is_some));
        assert!(!a.music.is_empty());
    }
    // Blitz, random starting items and starting slots.
    let p = mmr_settings::presets().into_iter().find(|p| p.id == "blitz").unwrap();
    let mut values = mmr_settings::apply_preset(&p, None);
    values.insert("StartingItemMode".into(), serde_json::json!("Random"));
    values.insert("RandomStartingItemGroups".into(), serde_json::json!([{ "Items": ["ItemBow", "ItemHookshot", "ItemLens"], "Amount": 1 }]));
    let seed = mmr_randomizer::Seed::parse("blitz");
    let a = mmr_build::seed::build_seed(&input, &values, &logic, seed, mmr_randomizer::Style::Random, &Default::default()).unwrap();
    let b = mmr_build::seed::build_seed(&input, &values, &logic, seed, mmr_randomizer::Style::Random, &Default::default()).unwrap();
    assert!(a.rom == b.rom, "blitz: same seed, different ROM");
    assert!(!a.outcome.blitz_items.is_empty());
    assert_eq!(a.outcome.random_starting.len(), 1);
}

/// A patch rebuilds the randomized ROM exactly from the clean one.
#[test]
fn patch_round_trip() {
    let rom_path = std::env::var_os("MMR_TEST_ROM").map(PathBuf::from).unwrap_or_else(|| beside_repo("Zelda - Majora's Mask.z64"));
    let Ok(mut input) = std::fs::read(&rom_path) else {
        eprintln!("skipping: no ROM");
        return;
    };
    mmr_rom::prepare(&mut input).unwrap();
    let mut values = mmr_settings::defaults();
    values.insert("Cosmetic.Music".into(), serde_json::json!("Random"));
    let logic = mmr_logic::bundled("casual").unwrap();
    let built = mmr_build::seed::build_seed(&input, &values, &logic, mmr_randomizer::Seed(77), mmr_randomizer::Style::Random, &Default::default()).unwrap();
    let t = std::time::Instant::now();
    let patch = mmr_rom::patch::create(&input, &built.rom, b"{}");
    let made = t.elapsed();
    let (rom, _) = mmr_rom::patch::apply(&input, &patch).unwrap();
    eprintln!("patch: {} KB, made in {} ms, applied in {} ms", patch.len() / 1024, made.as_millis(), t.elapsed().as_millis() - made.as_millis());
    assert!(rom == built.rom);
    assert!(patch.len() < 4 << 20);
}

/// Every feature case in `legacy-out/cases` (made by
/// `tools/legacy-harness/cases.py`): a full build compared with the old one.
/// Prints a line per case; fails if any differ. Unsupported features are
/// reported but not failures, while they're being ported.
#[test]
fn cases() {
    let dir = std::env::var_os("MMR_LEGACY_OUT").map(PathBuf::from).unwrap_or_else(|| beside_repo("legacy-out")).join("cases");
    let rom_path = std::env::var_os("MMR_TEST_ROM").map(PathBuf::from).unwrap_or_else(|| beside_repo("Zelda - Majora's Mask.z64"));
    let (Ok(entries), Ok(mut input)) = (std::fs::read_dir(&dir), std::fs::read(&rom_path)) else {
        eprintln!("skipping cases: none found");
        return;
    };
    mmr_rom::prepare(&mut input).unwrap();
    let mut names: Vec<String> = entries.filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".plan.json").map(String::from)).collect();
    names.sort();
    let only = std::env::var("MMR_CASE").ok();
    let mut failed = Vec::new();
    for name in names {
        if only.as_deref().is_some_and(|o| o != name) {
            continue;
        }
        let plan: Plan = serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{name}.plan.json"))).unwrap()).unwrap();
        let theirs = std::fs::read(dir.join(format!("{name}.z64"))).unwrap();
        match build(&input, &plan, Stage::Audio) {
            Err(mmr_build::BuildError::Unsupported(what)) => eprintln!("{name:<18} unsupported: {what}"),
            Err(e) => {
                eprintln!("{name:<18} ERROR: {e}");
                failed.push(name);
            }
            Ok(ours) => match compare(&ours, &theirs) {
                None => eprintln!("{name:<18} identical"),
                Some(report) => {
                    eprintln!("{name:<18} DIFFERS\n{report}");
                    failed.push(name);
                }
            },
        }
    }
    assert!(failed.is_empty(), "cases differ: {failed:?}");
}

/// A description of how two ROMs differ, or `None` if they match.
fn compare(ours: &[u8], theirs: &[u8]) -> Option<String> {
    let ignored = |i: usize| (0x10..0x18).contains(&i) || (0x1A4D0..0x1A500).contains(&i);
    if ours.len() == theirs.len() && (0..ours.len()).all(|i| ignored(i) || ours[i] == theirs[i]) {
        return None;
    }
    let mut report = vec![format!("  sizes {:#X} vs {:#X}", ours.len(), theirs.len())];
    let (Ok(mut a), Ok(mut b)) = (mmr_rom::Rom::read(ours), mmr_rom::Rom::read(theirs)) else {
        return Some("  (file tables unreadable)".into());
    };
    report.push(format!("  files {} vs {}", a.files.len(), b.files.len()));
    for i in 2..a.files.len().min(b.files.len()) {
        let (ra, rb) = ((a.files[i].addr, a.files[i].end), (b.files[i].addr, b.files[i].end));
        if ra != rb {
            report.push(format!("  file {i}: range {ra:X?} vs {rb:X?}"));
        }
        if a.files[i].cmp_addr.is_none() || b.files[i].cmp_addr.is_none() {
            continue;
        }
        let (x, y) = (a.file(i).map(|d| d.clone()).unwrap_or_default(), b.file(i).map(|d| d.clone()).unwrap_or_default());
        if x != y {
            let diffs: Vec<usize> = (0..x.len().min(y.len())).filter(|&k| x[k] != y[k]).collect();
            let samples: Vec<String> = diffs.iter().take(8).map(|&k| format!("+{k:X}: {:02X}/{:02X}", x[k], y[k])).collect();
            report.push(format!("  file {i} ({:#X}): {} bytes differ, len {:#X}/{:#X}; {}", a.files[i].addr, diffs.len(), x.len(), y.len(), samples.join(" ")));
        }
        if report.len() > 25 {
            break;
        }
    }
    Some(report.join("\n"))
}
