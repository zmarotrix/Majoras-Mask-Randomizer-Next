//! The build hash: identifies a seed's content (shown as icons on the file
//! select screen, and used to seed cosmetic choices).

use mmr_rom::Rom;
use sha2::{Digest, Sha256};

/// SHA-256 over every file the build opened or added, in file order.
/// Depends only on the ROM's content, so it's the same on every OS.
pub fn content_hash(rom: &Rom, original_count: usize) -> Vec<u8> {
    let mut h = Sha256::new();
    for (i, f) in rom.files.iter().enumerate() {
        if i >= original_count || f.decompressed {
            h.update((i as u32).to_le_bytes());
            h.update(f.addr.to_le_bytes());
            h.update((f.data.len() as u32).to_le_bytes());
            h.update(&f.data);
        }
    }
    h.finalize().to_vec()
}
