//! The in-game payload: code and data the randomizer adds to the game
//! (built separately from `mm-rando-dev/assembly`), its symbols, and the
//! config blocks it reads. Port of the old `Asm` namespace.

use crate::BuildError;
use indexmap::IndexMap;
use std::io::Read;

/// Where the symbols file goes.
pub const SYMBOLS_FILE_START: u32 = 0x03F0_0000;
/// Patch data before this address (the file table) is ignored.
const TABLE_END: u32 = 0x20700;

/// Symbol name to address, in the order they appear in `symbols.json`.
#[derive(Debug, Clone)]
pub struct Symbols(IndexMap<String, u32>);

impl Symbols {
    pub fn from_json(json: &[u8]) -> Result<Self, BuildError> {
        let raw: IndexMap<String, String> = serde_json::from_slice(json).map_err(|e| BuildError::Data(format!("bad symbols: {e}")))?;
        let mut map = IndexMap::new();
        for (k, v) in raw {
            let n = u32::from_str_radix(v.trim_start_matches("0x").trim_start_matches("0X"), 16).map_err(|_| BuildError::Data(format!("bad symbol value {v}")))?;
            map.insert(k, n);
        }
        Ok(Symbols(map))
    }

    pub fn get(&self, name: &str) -> Result<u32, BuildError> {
        self.0.get(name).copied().ok_or_else(|| BuildError::Data(format!("missing symbol {name}")))
    }

    pub fn has(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    /// The symbols file stored in the ROM (`Symbols.ToBytes`).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(self.0.len() as u32).to_le_bytes());
        for (name, value) in &self.0 {
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&value.to_le_bytes());
        }
        while out.len() % 0x10 != 0 {
            out.push(0);
        }
        out
    }
}

/// One write in the patch: an address and its bytes.
#[derive(Debug, Clone)]
pub struct PatchEntry {
    pub address: u32,
    pub data: Vec<u8>,
}

/// Read the gzipped patch (`Patcher.FromCompressedAlv`).
pub fn read_patch(compressed: &[u8]) -> Result<Vec<PatchEntry>, BuildError> {
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(compressed).read_to_end(&mut raw).map_err(|e| BuildError::Data(format!("bad asm patch: {e}")))?;
    let mut entries = Vec::new();
    let mut at = 0usize;
    while at < raw.len() {
        let address = u32::from_be_bytes(raw[at..at + 4].try_into().unwrap());
        let len = i32::from_be_bytes(raw[at + 4..at + 8].try_into().unwrap()) as usize;
        let data = raw[at + 8..at + 8 + len].to_vec();
        at += 8 + len;
        if address >= TABLE_END {
            entries.push(PatchEntry { address, data });
        }
    }
    Ok(entries)
}

/// The payload bytes between two addresses (`Patcher.GetFileData`).
pub fn file_data(entries: &[PatchEntry], start: u32, length: u32) -> Vec<u8> {
    let mut bytes = vec![0u8; length as usize];
    for e in entries {
        if start <= e.address && e.address < start + length {
            let offset = (e.address - start) as usize;
            bytes[offset..offset + e.data.len()].copy_from_slice(&e.data);
        }
    }
    bytes
}

/// Packs values into bits, most significant first (`MMR.Common.BitPacker`).
#[derive(Default)]
pub struct BitPacker {
    bit: usize,
    data: Vec<u8>,
}

impl BitPacker {
    fn write_byte(&mut self, v: u8) {
        let i = self.bit / 8;
        if i >= self.data.len() {
            self.data.push(0);
        }
        self.data[i] |= v;
    }

    pub fn bool(&mut self, v: bool) {
        let shift = 7 - self.bit % 8;
        self.write_byte(((v as u32) << shift) as u8);
        self.bit += 1;
    }

    pub fn int(&mut self, value: i32, bits: i32) {
        let mask = (1i32 << bits) - 1;
        let value = value & mask;
        let mut bits = bits;
        let mut shift = 7 - (self.bit % 8) as i32 - bits + 1;
        while shift < 0 {
            let right = -shift;
            self.write_byte((value >> right) as u8);
            let used = bits - right;
            self.bit += used as usize;
            shift += 8;
            bits -= used;
        }
        self.write_byte((value << shift) as u8);
        self.bit += bits as usize;
    }

    /// The bytes, padded like the old code: `align - len % align` extra
    /// bytes (a full `align` when already aligned).
    pub fn finish(self, align: usize) -> Vec<u8> {
        let mut out = self.data;
        if align > 1 {
            let padding = align - out.len() % align;
            out.resize(out.len() + padding, 0);
        }
        out
    }
}
