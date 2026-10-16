//! The browser interface builds the same ROM as everything else.

use serde_json::json;
use std::path::PathBuf;

/// SHA-256 (first 8 bytes, hex) of the ROM the wasm build made for these
/// inputs (`node` + crates/mmr-web, Oct 2026). Native must match it exactly.
const WASM_ROM: &str = "4c494c2cf21d8662";

#[test]
fn generate_matches_the_wasm_build() {
    let rom = std::env::var_os("MMR_TEST_ROM").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Zelda - Majora's Mask.z64"));
    let Ok(rom) = std::fs::read(&rom) else {
        eprintln!("skipping: no ROM");
        return;
    };
    let input = json!({ "values": {}, "seed": "2026", "outputs": { "rom": true, "tracker": true, "spoiler": true, "patch": true } });
    let (out, blobs) = mmr_web::call("generate", &input, vec![rom.clone()]).unwrap();
    let files = out["files"].as_array().unwrap();
    let i = files.iter().position(|f| f["kind"] == "rom").unwrap();
    let digest = sha256_hex8(&blobs[i]);
    assert_eq!(digest, WASM_ROM, "native and wasm builds differ");
    let p = files.iter().position(|f| f["kind"] == "patch").unwrap();
    let (_, back) = mmr_web::call("apply_patch", &serde_json::Value::Null, vec![rom, blobs[p].clone()]).unwrap();
    assert_eq!(back[0], blobs[i]);
}

fn sha256_hex8(data: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(data)[..8].iter().map(|b| format!("{b:02x}")).collect()
}
