//! The hash icons: five item icons shown on the file select screen, so
//! players can check they're on the same seed. Also as images for the app,
//! spoiler log and tracker.

use crate::{BuildError, Builder};
use serde::Serialize;

/// One hash icon.
#[derive(Debug, Clone, Serialize)]
pub struct HashIcon {
    pub name: &'static str,
    /// 32x32 RGBA pixels.
    #[serde(skip)]
    pub rgba: Vec<u8>,
}

/// Icon names by hash value (0-63), as the old `OutputHashJson` lists them.
const NAMES: &[(u8, &str)] = &[
    (0, "Ocarina of Time"),
    (1, "Hero's Bow"),
    (2, "Fire Arrow"),
    (3, "Ice Arrow"),
    (4, "Light Arrow"),
    (6, "Bomb"),
    (7, "Bombchu"),
    (8, "Deku Stick"),
    (9, "Deku Nut"),
    (10, "Magic Bean"),
    (12, "Powder Keg"),
    (13, "Pictograph Box"),
    (14, "Lens of Truth"),
    (15, "Hookshot"),
    (16, "Great Fairy's Sword"),
    (18, "Empty Bottle"),
    (23, "Deku Princess"),
    (27, "Bugs"),
    (30, "Big Poe"),
    (32, "Hot Spring Water"),
    (33, "Zora Egg"),
    (34, "Gold Dust"),
    (35, "Magic Mushroom"),
    (36, "Seahorse"),
    (37, "Chateau Romani"),
    (40, "Moon's Tear"),
    (41, "Land Title Deed"),
    (45, "Room Key"),
    (46, "Letter to Mama"),
    (47, "Letter to Kafei"),
    (48, "Pendant of Memories"),
    (49, "Map"),
    (50, "Deku Mask"),
    (51, "Goron Mask"),
    (52, "Zora Mask"),
    (53, "Fierce Deity's Mask"),
    (54, "Mask of Truth"),
    (55, "Kafei's Mask"),
    (56, "All-Night Mask"),
    (57, "Bunny Hood"),
    (58, "Keaton Mask"),
    (59, "Garo's Mask"),
    (60, "Romani's Mask"),
    (61, "Circus Leader's Mask"),
    (62, "Postman's Hat"),
    (63, "Couple's Mask"),
    (64, "Great Fairy's Mask"),
    (65, "Gibdo Mask"),
    (66, "Don Gero's Mask"),
    (67, "Kamaro's Mask"),
    (68, "Captain's Hat"),
    (69, "Stone Mask"),
    (70, "Bremen Mask"),
    (71, "Blast Mask"),
    (72, "Mask of Scents"),
    (73, "Giant's Mask"),
    (77, "Kokiri Sword"),
    (79, "Gilded Sword"),
    (80, "Great Fairy's Sword"),
    (81, "Hero's Shield"),
    (82, "Mirror Shield"),
    (84, "Quiver"),
    (90, "Adult's Wallet"),
    (97, "Bombers' Notebook"),
];

/// The icon numbers a hash shows (`ImageUtils.GetIconIndices`).
pub fn icon_values(hash: &[u8]) -> Vec<usize> {
    let mut value = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]);
    (0..5)
        .map(|_| {
            let v = (value >> 26) & 0x3F;
            value <<= 6;
            v as usize
        })
        .collect()
}

impl Builder<'_> {
    /// The hash's icons, from the payload's icon table and the game's item
    /// icons. Empty if the hash or payload isn't there.
    pub(crate) fn hash_icons(&mut self) -> Result<Vec<HashIcon>, BuildError> {
        let (Some(hash), Some(symbols)) = (self.hash.clone(), self.symbols.clone()) else { return Ok(Vec::new()) };
        if hash.len() < 4 || !symbols.has("HASH_ICONS") {
            return Ok(Vec::new());
        }
        let addr = symbols.get("HASH_ICONS")?;
        let count = self.rom.read_u16(addr + 4)? as usize;
        let mut table = Vec::with_capacity(count);
        for i in 0..count as u32 {
            table.push(self.rom.read_u8(addr + 6 + i)?);
        }
        // `icon_item_static`: an archive of Yaz0 files, one per icon.
        let archive = self.rom.file(19)?.clone();
        let be32 = |o: usize| u32::from_be_bytes([archive[o], archive[o + 1], archive[o + 2], archive[o + 3]]) as usize;
        let header = be32(0);
        let icon = |n: usize| -> Option<Vec<u8>> {
            let start = header + if n == 0 { 0 } else { be32(4 * n) };
            let end = header + be32(4 * (n + 1));
            mmr_rom::yaz::decode(archive.get(start..end)?).ok().filter(|d| d.len() >= 32 * 32 * 4)
        };
        let mut out = Vec::new();
        for v in icon_values(&hash) {
            let Some(&file) = table.get(v) else { continue };
            let name = NAMES.iter().find(|(i, _)| *i == file).map_or("?", |(_, n)| n);
            let rgba = icon(file as usize).map(|d| d[..32 * 32 * 4].to_vec()).unwrap_or_else(|| vec![0; 32 * 32 * 4]);
            out.push(HashIcon { name, rgba });
        }
        Ok(out)
    }
}

/// The icons side by side as a PNG (8 pixels apart, like the old `.png`).
pub fn png(icons: &[HashIcon]) -> Vec<u8> {
    const MARGIN: usize = 8;
    let n = icons.len().max(1);
    let (w, h) = (32 * n + MARGIN * (n - 1), 32);
    let mut pixels = vec![0u8; w * h * 4];
    for (i, icon) in icons.iter().enumerate() {
        for y in 0..32 {
            let src = &icon.rgba[y * 128..y * 128 + 128];
            let at = (y * w + i * (32 + MARGIN)) * 4;
            pixels[at..at + 128].copy_from_slice(src);
        }
    }
    encode_png(w as u32, h as u32, &pixels)
}

/// One icon as a PNG.
pub fn icon_png(icon: &HashIcon) -> Vec<u8> {
    encode_png(32, 32, &icon.rgba)
}

/// 32x32 RGBA pixels as a PNG.
pub fn rgba_png(rgba: &[u8]) -> Vec<u8> {
    encode_png(32, 32, rgba)
}

/// A minimal RGBA PNG encoder.
fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut raw = Vec::with_capacity((w as usize * 4 + 1) * h as usize);
    for row in rgba.chunks(w as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    z.write_all(&raw).expect("writing to memory");
    let idat = z.finish().expect("writing to memory");
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &idat);
    chunk(b"IEND", &[]);
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

impl Builder<'_> {
    /// 32x32 RGBA icons for these items, where the game has an inventory
    /// icon for what they give (masks, equipment, bottles...).
    pub(crate) fn item_icons(&mut self, items: &[String]) -> Result<std::collections::BTreeMap<String, Vec<u8>>, BuildError> {
        let archive = self.rom.file(19)?.clone();
        let be32 = |o: usize| u32::from_be_bytes([archive[o], archive[o + 1], archive[o + 2], archive[o + 3]]) as usize;
        let header = be32(0);
        let count = header / 4 - 1;
        let icon = |n: usize| -> Option<Vec<u8>> {
            let start = header + if n == 0 { 0 } else { be32(4 * n) };
            let end = header + be32(4 * (n + 1));
            mmr_rom::yaz::decode(archive.get(start..end)?).ok().filter(|d| d.len() >= 32 * 32 * 4).map(|d| d[..32 * 32 * 4].to_vec())
        };
        let mut out = std::collections::BTreeMap::new();
        for item in items {
            if out.contains_key(item) {
                continue;
            }
            let Some(gi) = crate::items::get_item_index(item) else { continue };
            let Some(entry) = self.get_items.get(&gi) else { continue };
            let id = entry.item_gained as usize;
            if id < count {
                if let Some(rgba) = icon(id) {
                    out.insert(item.clone(), rgba);
                }
            }
        }
        Ok(out)
    }
}
