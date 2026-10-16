//! Music packs: `.mmrs` files (zip archives holding a sequence, its
//! categories, and optionally a custom instrument bank and samples) and
//! loose `.zseq` files, read as `SequenceUtils.ReadSequenceInfo` does.
//!
//! The randomizer ships with a set of packs (see `music_files.rs`); players
//! can add their own. Packs are read in name order, so a music folder gives
//! the same songs on every OS.

use std::borrow::Cow;
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

/// A music file: an `.mmrs` pack or a loose `.zseq`.
#[derive(Debug, Clone)]
pub struct MusicFile {
    pub name: String,
    pub data: Cow<'static, [u8]>,
}

/// The packs that come with the randomizer.
pub fn bundled() -> Vec<MusicFile> {
    crate::music_files::BUNDLED.iter().map(|(name, data)| MusicFile { name: name.to_string(), data: Cow::Borrowed(data) }).collect()
}

/// Files in the order the old randomizer's `Directory.GetFiles` gave them
/// on Windows (NTFS order: names compared uppercased).
pub fn sorted(mut files: Vec<MusicFile>) -> Vec<MusicFile> {
    files.sort_by(|a, b| a.name.to_uppercase().cmp(&b.name.to_uppercase()).then_with(|| a.name.cmp(&b.name)));
    files.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    files
}

/// The `.mmrs` and `.zseq` files in a folder and the folders inside it.
pub fn read_folder(dir: &std::path::Path) -> std::io::Result<Vec<MusicFile>> {
    let mut out = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(d) = dirs.pop() {
        for entry in std::fs::read_dir(&d)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".mmrs") || lower.ends_with(".zseq") || lower.ends_with("musicplando.json") {
                out.push(MusicFile { name, data: Cow::Owned(std::fs::read(&path)?) });
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- zip ---

/// One file in a zip archive.
pub(crate) struct ZipEntry {
    pub full_name: String,
    pub data: Vec<u8>,
}

impl ZipEntry {
    /// The name without folders (`ZipArchiveEntry.Name`).
    pub fn name(&self) -> &str {
        self.full_name.rsplit(['/', '\\']).next().unwrap_or("")
    }
}

fn le16(b: &[u8], o: usize) -> Result<usize, String> {
    b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]]) as usize).ok_or_else(|| "truncated zip".to_string())
}

fn le32(b: &[u8], o: usize) -> Result<usize, String> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize).ok_or_else(|| "truncated zip".to_string())
}

/// Read every entry of a zip archive, in central directory order (the
/// order `ZipArchive.Entries` lists them). Stored and deflated entries only.
pub(crate) fn read_zip(b: &[u8]) -> Result<Vec<ZipEntry>, String> {
    let end = (0..=b.len().saturating_sub(22)).rev().find(|&i| le32(b, i) == Ok(0x0605_4B50)).ok_or("not a zip archive")?;
    let count = le16(b, end + 10)?;
    let mut at = le32(b, end + 16)?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        if le32(b, at)? != 0x0201_4B50 {
            return Err("bad zip directory".into());
        }
        let method = le16(b, at + 10)?;
        let compressed = le32(b, at + 20)?;
        let (name_len, extra_len, comment_len) = (le16(b, at + 28)?, le16(b, at + 30)?, le16(b, at + 32)?);
        let local = le32(b, at + 42)?;
        let name = b.get(at + 46..at + 46 + name_len).ok_or("truncated zip")?;
        let full_name = String::from_utf8_lossy(name).into_owned();
        at += 46 + name_len + extra_len + comment_len;

        if le32(b, local)? != 0x0403_4B50 {
            return Err("bad zip entry".into());
        }
        let start = local + 30 + le16(b, local + 26)? + le16(b, local + 28)?;
        let raw = b.get(start..start + compressed).ok_or("truncated zip")?;
        let data = match method {
            0 => raw.to_vec(),
            8 => {
                let mut out = Vec::new();
                flate2::read::DeflateDecoder::new(raw).read_to_end(&mut out).map_err(|e| format!("bad zip data: {e}"))?;
                out
            }
            m => return Err(format!("unsupported zip compression {m}")),
        };
        entries.push(ZipEntry { full_name, data });
    }
    Ok(entries)
}

// -------------------------------------------------------------- songs ---

/// `Convert.ToInt32(s, 16)`: hex, with an optional 0x prefix.
pub(crate) fn hex(s: &str) -> Result<i32, String> {
    let digits = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    if digits.is_empty() || digits.len() > 8 {
        return Err(format!("bad hex number {s:?}"));
    }
    u32::from_str_radix(digits, 16).map(|v| v as i32).map_err(|_| format!("bad hex number {s:?}"))
}

/// The low 8 bytes of an MD5 hash, as the old code compares banks and samples.
fn md5_hash(data: &[u8]) -> i64 {
    use md5::{Digest, Md5};
    let d = Md5::digest(data);
    i64::from_le_bytes(d[0..8].try_into().unwrap())
}

/// A custom instrument sample (`SequenceSoundSampleBinaryData`).
#[derive(Debug, Clone)]
pub(crate) struct Sample {
    pub data: Rc<Vec<u8>>,
    pub addr: u32,
    pub marker: u32,
    pub hash: i64,
}

/// An instrument bank (`InstrumentSetInfo`).
#[derive(Debug, Clone)]
pub(crate) struct Bank {
    pub data: Rc<Vec<u8>>,
    pub slot: i32,
    pub meta: Vec<u8>,
    pub modified: i32,
    pub hash: i64,
    pub samples: Option<Rc<Vec<RefCell<Sample>>>>,
}

pub(crate) type BankRef = Rc<RefCell<Bank>>;

/// One version of a song's sequence (`SequenceBinaryData`).
#[derive(Debug)]
pub(crate) struct SeqBinary {
    pub data: Rc<Vec<u8>>,
    pub bank: Option<BankRef>,
    pub form_mask: Option<Vec<u8>>,
}

/// A song (`SequenceInfo`).
#[derive(Debug, Clone)]
pub(crate) struct Song {
    pub name: String,
    /// The file it came from, if any (the old code re-reads it by name).
    pub file: Option<Cow<'static, [u8]>>,
    pub replaces: i32,
    pub mm_seq: i32,
    pub categories: Vec<i32>,
    pub instrument: i32,
    pub binaries: Option<Vec<Rc<SeqBinary>>>,
    pub samples: Option<Rc<Vec<RefCell<Sample>>>>,
}

impl Song {
    pub fn new(name: &str) -> Song {
        Song { name: name.into(), file: None, replaces: -1, mm_seq: -1, categories: Vec::new(), instrument: 0, binaries: None, samples: None }
    }
}

/// Banks a pack's sequence can't name get this (`MARK_REQUIRES_NEW_BANK`).
pub(crate) const NEW_BANK: i32 = 0x28;

/// `ScanMMRSCategories`.
fn categories(text: &str, song: &str) -> Result<Vec<i32>, String> {
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let delimiter = if text.contains(',') {
        ','
    } else if text.contains('-') {
        '-'
    } else if text.contains('\n') {
        '\n'
    } else {
        ','
    };
    text.split(delimiter).filter(|l| !l.is_empty()).map(|l| hex(l.trim()).map_err(|_| format!("categories can't be read: {song}"))).collect()
}

/// `ScanForMMRSFormMask`: which channels play in which states.
fn form_mask(json: &str) -> Result<Vec<u8>, String> {
    const FIERCE_DEITY: u16 = 1;
    const HUMAN: u16 = 1 << 4;
    const ALL: u16 = 0x1F;
    let values: Vec<serde_json::Value> = serde_json::from_str(json.strip_prefix('\u{FEFF}').unwrap_or(json)).map_err(|e| format!("formmask: {e}"))?;
    let mut states: Vec<u16> = values.iter().map(|v| v.as_u64().filter(|&n| n <= 0xFFFF).map(|n| n as u16).ok_or("formmask: bad value")).collect::<Result<_, _>>()?;
    // Sequences from before Fierce Deity had its own state play as Human.
    if !states.iter().any(|s| s & FIERCE_DEITY != 0 && s & HUMAN == 0) {
        for s in states.iter_mut().filter(|s| **s & HUMAN != 0) {
            *s |= FIERCE_DEITY;
        }
    }
    // States the file doesn't mention shouldn't mute anything.
    for bit in 5..=14 {
        let state = 1u16 << bit;
        debug_assert!(state > ALL);
        if !states.iter().any(|s| s & state != 0) {
            *states.get_mut(0x10).ok_or("formmask: too short")? |= state;
        }
    }
    Ok(states.iter().flat_map(|s| s.to_be_bytes()).collect())
}

/// Read one `.mmrs` pack (`ScanForMMRS`). Errors skip the pack, as the old
/// code did.
pub(crate) fn read_pack(file: &MusicFile) -> Result<Song, String> {
    let zip = read_zip(&file.data)?;
    let entry = |full: &str| zip.iter().find(|e| e.full_name == full);
    let mut song = Song::new(&file.name);
    song.file = Some(file.data.clone());
    let cats = entry("categories.txt").ok_or_else(|| format!("no categories file for {}", file.name))?;
    song.categories = categories(&String::from_utf8_lossy(&cats.data), &file.name)?;

    let mut samples = Vec::new();
    for e in zip.iter().filter(|e| e.name().contains(".zsound")) {
        let parts: Vec<&str> = e.name().split('_').collect();
        let name = if parts.len() > 1 { parts[parts.len() - 1] } else { e.name() };
        let name = name.split('.').next().unwrap_or("");
        let marker = hex(name)? as u32;
        samples.push(RefCell::new(Sample { data: Rc::new(e.data.clone()), addr: marker, marker, hash: md5_hash(&e.data) }));
    }
    song.samples = Some(Rc::new(samples));

    // `ScanMMRSSequences`: a pack can hold several versions of the song.
    let mut binaries: Vec<Rc<SeqBinary>> = Vec::new();
    let mut claimed = 0;
    for kind in [".zseq", ".seq", ".aseq"] {
        for e in zip.iter().filter(|e| e.name().contains(kind)) {
            let name = e.name();
            let base = &name[..name.rfind(kind).unwrap()];
            let parts: Vec<&str> = base.split('_').collect();
            let bank_name = if parts.len() > 1 { parts[parts.len() - 1] } else { base };
            song.instrument = match hex(bank_name) {
                Ok(v) => v,
                Err(_) if bank_name.is_empty() || bank_name.len() > 8 && bank_name.chars().all(|c| c.is_ascii_hexdigit()) => return Err(format!("bad sequence name {base}")),
                Err(_) => NEW_BANK,
            };
            let bank = match entry(&format!("{base}.zbank")) {
                Some(b) => {
                    let meta = entry(&format!("{base}.bankmeta")).ok_or_else(|| format!("{base} has no bankmeta"))?;
                    Some(Rc::new(RefCell::new(Bank { data: Rc::new(b.data.clone()), slot: song.instrument, meta: meta.data.clone(), modified: 1, hash: md5_hash(&b.data), samples: None })))
                }
                None => None,
            };
            let custom = bank.is_some();
            // Custom banks always get a new bank slot now.
            if song.instrument > 0x28 || custom {
                song.instrument = NEW_BANK;
                for b in &binaries {
                    b.bank.as_ref().ok_or("a pack mixes sequences with and without banks")?.borrow_mut().slot = song.instrument;
                }
            }
            if song.instrument == NEW_BANK && !custom {
                continue;
            }
            claimed += custom as usize;
            let mask = entry(&format!("{base}.formmask")).map(|m| form_mask(&String::from_utf8_lossy(&m.data))).transpose()?;
            binaries.push(Rc::new(SeqBinary { data: Rc::new(e.data.clone()), bank, form_mask: mask }));
        }
    }
    if claimed < zip.iter().filter(|e| e.name().contains("zbank")).count() {
        return Err(format!("more banks than sequences in {}", file.name));
    }
    song.binaries = Some(binaries);
    Ok(song)
}

/// A loose `NAME_BANK_CATEGORIES.zseq` file (`ScanZSEQUENCE`). `None` for
/// names that don't follow the pattern.
pub(crate) fn read_zseq(file: &MusicFile) -> Result<Option<Song>, String> {
    let tokens: Vec<&str> = file.name.split('_').collect();
    if tokens.len() != 3 {
        return Ok(None);
    }
    let bad = || format!("music: file name can't be read: {}", file.name);
    let cats = tokens[2].get(..tokens[2].len().saturating_sub(5)).ok_or_else(bad)?;
    let mut song = Song::new(&file.name);
    song.instrument = hex(tokens[1]).map_err(|_| bad())?;
    song.categories = cats.split('-').map(|c| hex(c).map_err(|_| bad())).collect::<Result<_, _>>()?;
    song.file = Some(file.data.clone());
    Ok(Some(song))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_bundled_pack() {
        let packs = bundled();
        assert!(packs.len() > 100);
        for p in &packs {
            let song = read_pack(p).unwrap_or_else(|e| panic!("{}: {e}", p.name));
            assert!(!song.categories.is_empty(), "{}", p.name);
        }
    }
}
