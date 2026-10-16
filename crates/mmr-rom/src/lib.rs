//! N64 ROM handling: detect byte order, normalize to big-endian (`.z64`),
//! and check that the ROM is an unmodified Majora's Mask (USA).
//!
//! Works on bytes only (no file I/O) so it runs in the browser too.

pub mod crc;
pub mod fs;
pub mod patch;
pub mod yaz;

pub use fs::{Rom, RomFile};
use serde::Serialize;

/// Size of the Majora's Mask (USA) ROM.
pub const MM_US_SIZE: usize = 0x200_0000;
/// Header checksums of the unmodified Majora's Mask (USA) ROM.
pub const MM_US_CRC: (u32, u32) = (0x5354_631C, 0x03A2_DEF0);

/// Byte order of an N64 ROM image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ByteOrder {
    /// `.z64`: native big-endian. What the randomizer needs.
    BigEndian,
    /// `.v64`: 16-bit words swapped.
    ByteSwapped,
    /// `.n64`: 32-bit little-endian.
    LittleEndian,
}

impl ByteOrder {
    pub fn detect(rom: &[u8]) -> Option<ByteOrder> {
        match rom.get(..4)? {
            [0x80, 0x37, 0x12, 0x40] => Some(ByteOrder::BigEndian),
            [0x37, 0x80, 0x40, 0x12] => Some(ByteOrder::ByteSwapped),
            [0x40, 0x12, 0x37, 0x80] => Some(ByteOrder::LittleEndian),
            _ => None,
        }
    }
}

/// Convert a ROM to big-endian in place.
pub fn to_big_endian(rom: &mut [u8], from: ByteOrder) {
    match from {
        ByteOrder::BigEndian => {}
        ByteOrder::ByteSwapped => rom.chunks_exact_mut(2).for_each(|w| w.swap(0, 1)),
        ByteOrder::LittleEndian => rom.chunks_exact_mut(4).for_each(|w| w.reverse()),
    }
}

/// Header information, read from a big-endian ROM.
#[derive(Debug, Clone, Serialize)]
pub struct Header {
    pub crc1: u32,
    pub crc2: u32,
    /// Internal name, e.g. "ZELDA MAJORA'S MASK".
    pub name: String,
    /// Four-character game code, e.g. "NZSE".
    pub game_code: String,
    pub revision: u8,
}

impl Header {
    pub fn read(rom: &[u8]) -> Option<Header> {
        let be32 = |o: usize| u32::from_be_bytes(rom[o..o + 4].try_into().unwrap());
        if rom.len() < 0x40 {
            return None;
        }
        Some(Header {
            crc1: be32(0x10),
            crc2: be32(0x14),
            name: String::from_utf8_lossy(&rom[0x20..0x34]).trim_end_matches(['\0', ' ']).to_string(),
            game_code: String::from_utf8_lossy(&rom[0x3B..0x3F]).to_string(),
            revision: rom[0x3F],
        })
    }

    pub fn region(&self) -> &'static str {
        match self.game_code.as_bytes().get(3) {
            Some(b'E') => "USA",
            Some(b'J') => "Japan",
            Some(b'P') => "Europe",
            _ => "unknown region",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RomError {
    #[error("This file isn't an N64 ROM.")]
    NotARom,
    #[error("This is an N64 ROM, but not Majora's Mask (it's \"{name}\").")]
    WrongGame { name: String },
    #[error("This is the {region} version of Majora's Mask. Only the USA version is supported.")]
    WrongRegion { region: String },
    #[error("This Majora's Mask ROM has been modified, or was already randomized. Please use an unmodified USA ROM.")]
    Modified,
    #[error("This ROM is the wrong size ({size} bytes); it may be incomplete or modified.")]
    WrongSize { size: usize },
}

/// A checked, big-endian Majora's Mask (USA) ROM.
#[derive(Debug, Clone, Serialize)]
pub struct RomInfo {
    pub header: Header,
    /// The byte order the file was in. If not big-endian, it was converted.
    pub original_order: ByteOrder,
}

/// Check a ROM and convert it to big-endian in place.
pub fn prepare(rom: &mut [u8]) -> Result<RomInfo, RomError> {
    let order = ByteOrder::detect(rom).ok_or(RomError::NotARom)?;
    if rom.len() % 4 != 0 {
        return Err(RomError::NotARom);
    }
    to_big_endian(rom, order);
    let header = Header::read(rom).ok_or(RomError::NotARom)?;
    if !header.game_code.starts_with("NZS") && !header.name.contains("MAJORA") {
        return Err(RomError::WrongGame { name: header.name });
    }
    if header.region() != "USA" {
        return Err(RomError::WrongRegion { region: header.region().into() });
    }
    if (header.crc1, header.crc2) != MM_US_CRC {
        return Err(RomError::Modified);
    }
    if rom.len() != MM_US_SIZE {
        return Err(RomError::WrongSize { size: rom.len() });
    }
    Ok(RomInfo { header, original_order: order })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_rom(name: &str, code: &str, crc: (u32, u32), size: usize) -> Vec<u8> {
        let mut rom = vec![0u8; size];
        rom[..4].copy_from_slice(&[0x80, 0x37, 0x12, 0x40]);
        rom[0x10..0x14].copy_from_slice(&crc.0.to_be_bytes());
        rom[0x14..0x18].copy_from_slice(&crc.1.to_be_bytes());
        rom[0x20..0x20 + name.len()].copy_from_slice(name.as_bytes());
        rom[0x3B..0x3F].copy_from_slice(code.as_bytes());
        rom
    }

    fn mm() -> Vec<u8> {
        fake_rom("ZELDA MAJORA'S MASK", "NZSE", MM_US_CRC, MM_US_SIZE)
    }

    #[test]
    fn accepts_all_byte_orders() {
        let reference = mm();
        for order in [ByteOrder::BigEndian, ByteOrder::ByteSwapped, ByteOrder::LittleEndian] {
            let mut rom = reference.clone();
            match order {
                ByteOrder::ByteSwapped => rom.chunks_exact_mut(2).for_each(|w| w.swap(0, 1)),
                ByteOrder::LittleEndian => rom.chunks_exact_mut(4).for_each(|w| w.reverse()),
                ByteOrder::BigEndian => {}
            }
            let info = prepare(&mut rom).unwrap();
            assert_eq!(info.original_order, order);
            assert_eq!(info.header.name, "ZELDA MAJORA'S MASK");
            assert!(rom == reference);
        }
    }

    #[test]
    fn explains_common_mistakes() {
        assert_eq!(prepare(&mut vec![1, 2, 3, 4]).unwrap_err(), RomError::NotARom);
        let mut oot = fake_rom("THE LEGEND OF ZELDA", "CZLE", (1, 2), 0x40);
        assert!(matches!(prepare(&mut oot), Err(RomError::WrongGame { .. })));
        let mut jp = fake_rom("ZELDA MAJORA'S MASK", "NZSJ", (1, 2), 0x40);
        assert_eq!(prepare(&mut jp).unwrap_err(), RomError::WrongRegion { region: "Japan".into() });
        let mut randomized = fake_rom("ZELDA MAJORA'S MASK", "NZSE", (1, 2), MM_US_SIZE);
        assert_eq!(prepare(&mut randomized).unwrap_err(), RomError::Modified);
    }
}
