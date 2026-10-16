//! Byte-for-byte comparison with ROMs built by the old randomizer.
//!
//! Needs a clean Majora's Mask (USA) ROM and reference outputs from
//! `tools/legacy-harness` (see `tools/legacy-harness/README.md`). Found via
//! `MMR_TEST_ROM` / `MMR_LEGACY_OUT`, or next to the repository by default.
//! Skipped (passes with a note) when they're missing, e.g. on CI.

use mmr_rom::{crc, fs::SIGNATURE_ADDRESS, Rom};
use std::path::PathBuf;

fn beside_repo(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..").join(name)
}

fn inputs(reference: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    let rom = std::env::var_os("MMR_TEST_ROM").map(PathBuf::from).unwrap_or_else(|| beside_repo("Zelda - Majora's Mask.z64"));
    let out = std::env::var_os("MMR_LEGACY_OUT").map(PathBuf::from).unwrap_or_else(|| beside_repo("legacy-out"));
    match (std::fs::read(&rom), std::fs::read(out.join(reference))) {
        (Ok(a), Ok(b)) => Some((a, b)),
        _ => {
            eprintln!("skipping: needs {} and {}", rom.display(), out.join(reference).display());
            None
        }
    }
}

/// Compare, ignoring the old builder's timestamped signature and the
/// checksum (which covers it).
fn assert_same(ours: &[u8], theirs: &[u8]) {
    assert_eq!(ours.len(), theirs.len(), "ROM sizes differ");
    let ignored = |i: usize| (0x10..0x18).contains(&i) || (SIGNATURE_ADDRESS..SIGNATURE_ADDRESS + 0x30).contains(&i);
    let diffs: Vec<usize> = (0..ours.len()).filter(|&i| !ignored(i) && ours[i] != theirs[i]).collect();
    assert!(diffs.is_empty(), "{} bytes differ, first at {:#X}", diffs.len(), diffs[0]);
    assert_eq!(crc::compute(theirs), (u32::from_be_bytes(theirs[0x10..0x14].try_into().unwrap()), u32::from_be_bytes(theirs[0x14..0x18].try_into().unwrap())));
}

#[test]
fn rebuild_matches() {
    let Some((mut input, reference)) = inputs("rebuild.z64") else { return };
    mmr_rom::prepare(&mut input).unwrap();
    let mut rom = Rom::read(&input).unwrap();
    assert_same(&rom.build(b"").unwrap(), &reference);
}

#[test]
fn recompressing_every_file_matches() {
    let Some((mut input, reference)) = inputs("recompress.z64") else { return };
    mmr_rom::prepare(&mut input).unwrap();
    let mut rom = Rom::read(&input).unwrap();
    for i in 0..rom.files.len() {
        if rom.files[i].compressed && rom.files[i].cmp_addr.is_some() {
            rom.open(i).unwrap();
        }
    }
    assert_same(&rom.build(b"").unwrap(), &reference);
}
