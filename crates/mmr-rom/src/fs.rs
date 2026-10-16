//! The game's file system: the DMA table ("dmadata") listing every file,
//! where it lives in the ROM, and whether it's Yaz0-compressed.
//!
//! [`Rom::read`] loads every file; editing a file decompresses it, and
//! [`Rom::build`] compresses edited files again and lays the ROM out. This
//! mirrors the old randomizer's `RomUtils` exactly, so ROMs built from the same
//! changes come out byte-identical.

use crate::yaz;

/// ROM address of the DMA table.
pub const FILE_TABLE: usize = 0x1A500;
/// Where the randomizer writes its signature.
pub const SIGNATURE_ADDRESS: usize = 0x1A4D0;
/// The actor overlay table (inside `code`).
pub const OVERLAY_TABLE: u32 = 0xC4_5510;
const OVERLAY_COUNT: u32 = 690;
/// Files after this many DMA entries are dropped (vanilla has none in use).
const MAX_VANILLA_FILES: usize = 0x603;
/// Size of a vanilla ROM; output grows past it in steps if needed.
const ROM_SIZE: usize = 0x200_0000;
const EXPANSION_STEP: usize = 0x4_0000;

/// One file in the ROM.
#[derive(Debug, Clone, Default)]
pub struct RomFile {
    /// Virtual ROM address range (what the game uses to find the file).
    pub addr: u32,
    pub end: u32,
    /// Physical ROM location. `None` for files the game has deleted.
    pub cmp_addr: Option<u32>,
    pub cmp_end: u32,
    pub compressed: bool,
    /// Data as stored: compressed unless [`RomFile::decompressed`].
    pub data: Vec<u8>,
    /// The data has been decompressed for editing (`WasEdited` in the old code).
    pub decompressed: bool,
    /// Lives at a fixed virtual address; appended files go before these.
    pub is_static: bool,
    /// RAM address range, for code overlays.
    pub vram_start: u32,
    pub vram_end: u32,
}

impl RomFile {
    pub fn contains(&self, vrom: u32) -> bool {
        vrom >= self.addr && vrom < self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FsError {
    #[error("the ROM's file table is damaged")]
    BadFileTable,
    #[error("no file contains address {0:#X}")]
    NoFile(u32),
    #[error("file {0} can't be read: {1}")]
    Decompress(usize, yaz::YazError),
    #[error("the ROM is larger than {0} MB, the most {1} supports")]
    TooLarge(usize, &'static str),
}

/// The ROM, as a list of files.
#[derive(Debug, Clone)]
pub struct Rom {
    pub files: Vec<RomFile>,
}

fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes(b[o..o + 4].try_into().unwrap())
}

impl Rom {
    /// Load a (checked, big-endian) ROM.
    pub fn read(rom: &[u8]) -> Result<Rom, FsError> {
        let mut files = Vec::new();
        let mut at = FILE_TABLE;
        loop {
            if at + 16 > rom.len() {
                return Err(FsError::BadFileTable);
            }
            let (addr, end, cmp_addr, cmp_end) = (be32(rom, at), be32(rom, at + 4), be32(rom, at + 8), be32(rom, at + 12));
            at += 16;
            if addr == end {
                break;
            }
            if files.len() >= MAX_VANILLA_FILES {
                continue;
            }
            let mut f = RomFile {
                addr,
                end,
                cmp_addr: (cmp_addr != u32::MAX).then_some(cmp_addr),
                cmp_end,
                compressed: cmp_end != 0,
                ..Default::default()
            };
            if let Some(start) = f.cmp_addr {
                let range = if f.compressed { start as usize..cmp_end as usize } else { start as usize..(start + (end - addr)) as usize };
                f.data = rom.get(range).ok_or(FsError::BadFileTable)?.to_vec();
            }
            files.push(f);
        }
        let mut rom = Rom { files };
        // Record each actor overlay's RAM range. Like the old code, this opens
        // (decompresses) every overlay file.
        for i in 0..OVERLAY_COUNT {
            let entry = OVERLAY_TABLE + i * 0x20;
            let vrom = rom.read_u32(entry)?;
            if vrom == 0 {
                continue;
            }
            let (vram_start, vram_end) = (rom.read_u32(entry + 8)?, rom.read_u32(entry + 12)?);
            let index = rom.index_for_writing(vrom)?;
            rom.files[index].vram_start = vram_start;
            rom.files[index].vram_end = vram_end;
        }
        Ok(rom)
    }

    /// Index of the file containing a virtual ROM address.
    pub fn index_of(&self, vrom: u32) -> Option<usize> {
        self.files.iter().position(|f| f.contains(vrom))
    }

    /// Index of the file loaded at a RAM address (code overlays).
    pub fn index_of_vram(&self, vram: u32) -> Option<usize> {
        self.files.iter().position(|f| vram >= f.vram_start && vram < f.vram_end)
    }

    /// Decompress a file so it can be edited (no-op if it already is).
    pub fn open(&mut self, index: usize) -> Result<(), FsError> {
        let f = &mut self.files[index];
        if f.compressed && !f.decompressed {
            f.data = yaz::decode(&f.data).map_err(|e| FsError::Decompress(index, e))?;
            f.decompressed = true;
        }
        Ok(())
    }

    /// A file's contents, decompressed.
    pub fn file(&mut self, index: usize) -> Result<&mut Vec<u8>, FsError> {
        self.open(index)?;
        Ok(&mut self.files[index].data)
    }

    fn index_for_writing(&mut self, vrom: u32) -> Result<usize, FsError> {
        let i = self.index_of(vrom).ok_or(FsError::NoFile(vrom))?;
        self.open(i)?;
        Ok(i)
    }

    /// The bytes at a virtual ROM address (or RAM address, for addresses at
    /// or above 0x80000000), within one file.
    pub fn bytes_mut(&mut self, addr: u32, len: usize) -> Result<&mut [u8], FsError> {
        let (i, offset) = if addr >= 0x8000_0000 {
            let i = self.index_of_vram(addr).ok_or(FsError::NoFile(addr))?;
            self.open(i)?;
            (i, addr - self.files[i].vram_start)
        } else {
            let i = self.index_for_writing(addr)?;
            (i, addr - self.files[i].addr)
        };
        let offset = offset as usize;
        self.files[i].data.get_mut(offset..offset + len).ok_or(FsError::NoFile(addr))
    }

    pub fn read_u8(&mut self, addr: u32) -> Result<u8, FsError> {
        Ok(self.bytes_mut(addr, 1)?[0])
    }
    pub fn read_u16(&mut self, addr: u32) -> Result<u16, FsError> {
        Ok(u16::from_be_bytes(self.bytes_mut(addr, 2)?.try_into().unwrap()))
    }
    pub fn read_u32(&mut self, addr: u32) -> Result<u32, FsError> {
        Ok(u32::from_be_bytes(self.bytes_mut(addr, 4)?.try_into().unwrap()))
    }
    pub fn write(&mut self, addr: u32, bytes: &[u8]) -> Result<(), FsError> {
        self.bytes_mut(addr, bytes.len())?.copy_from_slice(bytes);
        Ok(())
    }
    pub fn write_u8(&mut self, addr: u32, v: u8) -> Result<(), FsError> {
        self.write(addr, &[v])
    }
    pub fn write_u16(&mut self, addr: u32, v: u16) -> Result<(), FsError> {
        self.write(addr, &v.to_be_bytes())
    }
    pub fn write_u32(&mut self, addr: u32, v: u32) -> Result<(), FsError> {
        self.write(addr, &v.to_be_bytes())
    }

    /// Index of the last file without a fixed address.
    fn tail_index(&self) -> usize {
        self.files.iter().rposition(|f| !f.is_static).expect("the ROM has files")
    }

    /// Add a new file after the last non-static file; returns its index.
    pub fn append_file(&mut self, data: Vec<u8>, compressed: bool) -> usize {
        let addr = self.files[self.tail_index()].end;
        self.insert_file(RomFile {
            addr,
            end: addr + data.len() as u32,
            // Like the old code: a compressed file passed in here is already compressed.
            compressed,
            data,
            cmp_addr: Some(0),
            ..Default::default()
        })
    }

    /// Add a file (static files go at the end, others before them).
    pub fn insert_file(&mut self, file: RomFile) -> usize {
        if file.is_static {
            self.files.push(file);
            self.files.len() - 1
        } else {
            let i = self.tail_index() + 1;
            self.files.insert(i, file);
            i
        }
    }

    /// Compress edited files and lay out the final ROM. `signature` is
    /// written where the randomizer signs its ROMs (at most 0x30 bytes).
    pub fn build(&mut self, signature: &[u8]) -> Result<Vec<u8>, FsError> {
        self.build_with(signature, |_, _| {})
    }

    /// [`Rom::build`], with a last patch once every file has its place (and
    /// before the file table, signature and checksum are written).
    pub fn build_with(&mut self, signature: &[u8], patch: impl FnOnce(&mut [u8], &[RomFile])) -> Result<Vec<u8>, FsError> {
        self.compress_edited();
        let mut out = vec![0u8; ROM_SIZE];
        let mut at = 0usize;
        for f in &mut self.files {
            if f.cmp_addr.is_none() {
                continue;
            }
            f.cmp_addr = Some(at as u32);
            let len = f.data.len();
            if f.compressed {
                f.cmp_end = (at + len) as u32;
            }
            if at + len > out.len() {
                let grow = ((at + len - out.len()) / EXPANSION_STEP + 1) * EXPANSION_STEP;
                out.resize(out.len() + grow, 0);
            }
            out[at..at + len].copy_from_slice(&f.data);
            at += len;
            if at & 0xF != 0 {
                at = (at | 0xF) + 1;
            }
        }
        patch(&mut out, &self.files);
        for (i, f) in self.files.iter().enumerate() {
            let o = FILE_TABLE + i * 16;
            out[o..o + 4].copy_from_slice(&f.addr.to_be_bytes());
            out[o + 4..o + 8].copy_from_slice(&f.end.to_be_bytes());
            out[o + 8..o + 12].copy_from_slice(&f.cmp_addr.unwrap_or(u32::MAX).to_be_bytes());
            out[o + 12..o + 16].copy_from_slice(&f.cmp_end.to_be_bytes());
        }
        let n = signature.len().min(0x30);
        out[SIGNATURE_ADDRESS..SIGNATURE_ADDRESS + n].copy_from_slice(&signature[..n]);
        crate::crc::fix(&mut out);
        Ok(out)
    }

    /// Re-compress every edited compressed file.
    fn compress_edited(&mut self) {
        let edited: Vec<&mut RomFile> = self.files.iter_mut().filter(|f| f.compressed && f.decompressed).collect();
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            edited.into_par_iter().for_each(|f| {
                f.data = yaz::encode(&f.data);
                f.decompressed = false;
            });
        }
        #[cfg(not(feature = "parallel"))]
        for f in edited {
            f.data = yaz::encode(&f.data);
            f.decompressed = false;
        }
    }
}
