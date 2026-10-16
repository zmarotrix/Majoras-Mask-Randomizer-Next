//! MMR Next patch files (`.mmrn`): a randomized ROM as a difference from the
//! clean ROM, so seeds can be shared without sharing a ROM.
//!
//! The patch is a list of commands that rebuild the output exactly: copy
//! bytes from the clean ROM, insert new bytes, or (for an edited compressed
//! file) rebuild the file's contents from the clean one and compress them
//! again. Files that only moved cost a few bytes. The commands are deflated;
//! SHA-256 checks make sure the right ROM goes in and the right ROM comes
//! out. A small JSON block says what the seed is.
//!
//! Layout: `"MMRNPTCH"`, format version (u32), clean ROM SHA-256 (32 bytes),
//! output SHA-256 (32), output length (u32), info length (u32), info JSON,
//! then the deflated commands. All numbers big-endian.

use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};

const MAGIC: &[u8; 8] = b"MMRNPTCH";
const VERSION: u32 = 1;
/// Matches are searched in blocks of this many bytes, at multiples of
/// [`STEP`] in the output (N64 files sit at 16-byte boundaries).
const BLOCK: usize = 64;
const STEP: usize = 16;
const COPY: u8 = 0;
const INSERT: u8 = 1;
const YAZ: u8 = 2;

#[derive(Debug, thiserror::Error)]
pub enum PatchError {
    #[error("this isn't an MMR Next patch file")]
    NotAPatch,
    #[error("this patch was made by a newer version of the randomizer")]
    NewerVersion,
    #[error("this patch is for a different ROM (it needs a clean Majora's Mask (USA) ROM)")]
    WrongRom,
    #[error("the patch file is damaged")]
    Damaged,
}

fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

fn hash_block(b: &[u8]) -> u64 {
    // FNV-1a: fast, and only used to find candidates (matches are compared).
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn get_varint(data: &[u8], at: &mut usize) -> Result<u64, PatchError> {
    let mut v = 0u64;
    for shift in (0..64).step_by(7) {
        let b = *data.get(*at).ok_or(PatchError::Damaged)?;
        *at += 1;
        v |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 == 0 {
            return Ok(v);
        }
    }
    Err(PatchError::Damaged)
}

/// A region of the target written by a prepared command.
struct Special {
    start: usize,
    end: usize,
    command: Vec<u8>,
}

/// Commands that turn `base` into `target`, with `special` regions (sorted)
/// written by their own commands.
fn diff(base: &[u8], target: &[u8], special: &[Special]) -> Vec<u8> {
    // Index the base's blocks at block-aligned offsets.
    let mut index: HashMap<u64, Vec<u32>> = HashMap::new();
    let mut at = 0;
    while at + BLOCK <= base.len() {
        let block = &base[at..at + BLOCK];
        // Runs of one byte (padding) match everywhere; leave them to inserts,
        // which deflate to almost nothing.
        if block.iter().any(|&b| b != block[0]) {
            index.entry(hash_block(block)).or_default().push(at as u32);
        }
        at += BLOCK;
    }

    let mut commands = Vec::new();
    let mut literal_start = 0;
    let mut pos = 0;
    let mut next = 0;
    let flush = |commands: &mut Vec<u8>, from: usize, to: usize| {
        if to > from {
            commands.push(INSERT);
            put_varint(commands, (to - from) as u64);
            commands.extend_from_slice(&target[from..to]);
        }
    };
    loop {
        if let Some(s) = special.get(next) {
            if pos + BLOCK > s.start {
                flush(&mut commands, literal_start, s.start);
                commands.extend_from_slice(&s.command);
                literal_start = s.end;
                pos = s.end.div_ceil(STEP) * STEP;
                next += 1;
                continue;
            }
        }
        if pos + BLOCK > target.len() {
            break;
        }
        // Matches stop at the next special region.
        let limit = special.get(next).map_or(target.len(), |s| s.start);
        let best = index.get(&hash_block(&target[pos..pos + BLOCK])).and_then(|cands| {
            cands
                .iter()
                .filter(|&&c| base[c as usize..c as usize + BLOCK] == target[pos..pos + BLOCK])
                .map(|&c| {
                    let c = c as usize;
                    // Extend backwards (over pending literal bytes) and forwards.
                    let mut back = 0;
                    while back < pos - literal_start && back < c && base[c - back - 1] == target[pos - back - 1] {
                        back += 1;
                    }
                    let mut len = BLOCK;
                    while pos + len < limit && c + len < base.len() && base[c + len] == target[pos + len] {
                        len += 1;
                    }
                    (c - back, pos - back, len + back)
                })
                .max_by_key(|&(_, _, len)| len)
        });
        match best {
            Some((src, dst, len)) => {
                flush(&mut commands, literal_start, dst);
                commands.push(COPY);
                put_varint(&mut commands, src as u64);
                put_varint(&mut commands, len as u64);
                literal_start = dst + len;
                // Continue at the next step boundary (bytes before it become
                // literals unless the next match reaches back over them).
                pos = literal_start.div_ceil(STEP) * STEP;
            }
            None => pos += STEP,
        }
    }
    flush(&mut commands, literal_start, target.len());
    commands
}

/// A ROM's DMA table: (virtual start, physical start, physical end; 0 for
/// files that aren't compressed).
fn dma_table(rom: &[u8]) -> Vec<(u32, u32, u32)> {
    let be32 = |o: usize| rom.get(o..o + 4).map_or(0, |b| u32::from_be_bytes(b.try_into().unwrap()));
    let mut out = Vec::new();
    let mut at = crate::fs::FILE_TABLE;
    while be32(at + 4) != 0 {
        out.push((be32(at), be32(at + 8), be32(at + 12)));
        at += 16;
    }
    out
}

fn is_compressed(file: &(u32, u32, u32)) -> bool {
    file.2 != 0 && file.1 != u32::MAX && file.2 != u32::MAX && file.2 > file.1
}

/// Make a patch that turns `clean` into `output`. `info` is stored as is
/// (JSON about the seed: settings string, hash, version...).
pub fn create(clean: &[u8], output: &[u8], info: &[u8]) -> Vec<u8> {
    // An edited compressed file would be all new bytes; instead send what
    // changed in its contents, to be compressed again when applying.
    let clean_files = dma_table(clean);
    let files: Vec<(u32, u32, u32)> = dma_table(output).into_iter().filter(is_compressed).collect();
    let make = |file: &(u32, u32, u32)| -> Option<Special> {
        let (start, end) = (file.1 as usize, file.2 as usize);
        let region = output.get(start..end)?;
        let base = clean_files.iter().find(|f| f.0 == file.0 && is_compressed(f))?;
        let base_bytes = clean.get(base.1 as usize..base.2 as usize)?;
        if base_bytes == region {
            return None;
        }
        let (new, old) = (crate::yaz::decode(region).ok()?, crate::yaz::decode(base_bytes).ok()?);
        // Only if compressing again gives exactly these bytes.
        let encoded = crate::yaz::encode(&new);
        if encoded.len() > region.len() || encoded[..] != region[..encoded.len()] || region[encoded.len()..].iter().any(|&b| b != 0) {
            return None;
        }
        let sub = diff(&old, &new, &[]);
        let mut command = vec![YAZ];
        put_varint(&mut command, base.1 as u64);
        put_varint(&mut command, (base.2 - base.1) as u64);
        put_varint(&mut command, region.len() as u64);
        put_varint(&mut command, sub.len() as u64);
        command.extend(sub);
        Some(Special { start, end, command })
    };
    #[cfg(feature = "parallel")]
    let mut special: Vec<Special> = {
        use rayon::prelude::*;
        files.par_iter().filter_map(make).collect()
    };
    #[cfg(not(feature = "parallel"))]
    let mut special: Vec<Special> = files.iter().filter_map(make).collect();
    special.sort_by_key(|s| s.start);
    let commands = diff(clean, output, &special);

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&commands).expect("writing to memory");
    let deflated = encoder.finish().expect("writing to memory");

    let mut out = Vec::with_capacity(deflated.len() + info.len() + 96);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_be_bytes());
    out.extend_from_slice(&sha256(clean));
    out.extend_from_slice(&sha256(output));
    out.extend_from_slice(&(output.len() as u32).to_be_bytes());
    out.extend_from_slice(&(info.len() as u32).to_be_bytes());
    out.extend_from_slice(info);
    out.extend_from_slice(&deflated);
    out
}

struct Header<'a> {
    clean: [u8; 32],
    output: [u8; 32],
    length: usize,
    info: &'a [u8],
}

fn parse(patch: &[u8]) -> Result<(Header<'_>, &[u8]), PatchError> {
    if patch.len() < 84 || &patch[..8] != MAGIC {
        return Err(PatchError::NotAPatch);
    }
    let be32 = |o: usize| u32::from_be_bytes(patch[o..o + 4].try_into().unwrap()) as usize;
    if be32(8) as u32 > VERSION {
        return Err(PatchError::NewerVersion);
    }
    let info_len = be32(80);
    let info = patch.get(84..84 + info_len).ok_or(PatchError::Damaged)?;
    let header = Header { clean: patch[12..44].try_into().unwrap(), output: patch[44..76].try_into().unwrap(), length: be32(76), info };
    Ok((header, &patch[84 + info_len..]))
}

/// What a patch says about its seed, without applying it.
pub fn info(patch: &[u8]) -> Result<&[u8], PatchError> {
    Ok(parse(patch)?.0.info)
}

/// Rebuild the randomized ROM from the clean ROM and a patch. Returns the
/// ROM and the patch's info block.
pub fn apply<'p>(clean: &[u8], patch: &'p [u8]) -> Result<(Vec<u8>, &'p [u8]), PatchError> {
    let (header, deflated) = parse(patch)?;
    if sha256(clean) != header.clean {
        return Err(PatchError::WrongRom);
    }
    let mut commands = Vec::new();
    DeflateDecoder::new(deflated).read_to_end(&mut commands).map_err(|_| PatchError::Damaged)?;
    let mut out = Vec::with_capacity(header.length);
    run(clean, &commands, &mut out)?;
    if out.len() != header.length || sha256(&out) != header.output {
        return Err(PatchError::Damaged);
    }
    Ok((out, header.info))
}

/// One parsed command.
enum Op<'c> {
    Copy(usize, usize),
    Insert(&'c [u8]),
    Yaz { src: usize, len: usize, size: usize, sub: &'c [u8] },
}

fn parse_ops(commands: &[u8]) -> Result<Vec<Op<'_>>, PatchError> {
    let mut ops = Vec::new();
    let mut at = 0;
    while at < commands.len() {
        let kind = commands[at];
        at += 1;
        ops.push(match kind {
            COPY => Op::Copy(get_varint(commands, &mut at)? as usize, get_varint(commands, &mut at)? as usize),
            INSERT => {
                let len = get_varint(commands, &mut at)? as usize;
                let bytes = commands.get(at..at + len).ok_or(PatchError::Damaged)?;
                at += len;
                Op::Insert(bytes)
            }
            YAZ => {
                let src = get_varint(commands, &mut at)? as usize;
                let len = get_varint(commands, &mut at)? as usize;
                let size = get_varint(commands, &mut at)? as usize;
                let sub_len = get_varint(commands, &mut at)? as usize;
                let sub = commands.get(at..at + sub_len).ok_or(PatchError::Damaged)?;
                at += sub_len;
                Op::Yaz { src, len, size, sub }
            }
            _ => return Err(PatchError::Damaged),
        });
    }
    Ok(ops)
}

/// An edited compressed file: rebuild its contents, compress again.
fn rebuild(base: &[u8], src: usize, len: usize, size: usize, sub: &[u8]) -> Result<Vec<u8>, PatchError> {
    let old = crate::yaz::decode(base.get(src..src + len).ok_or(PatchError::Damaged)?).map_err(|_| PatchError::Damaged)?;
    let mut new = Vec::new();
    run(&old, sub, &mut new)?;
    let mut encoded = crate::yaz::encode(&new);
    if encoded.len() > size {
        return Err(PatchError::Damaged);
    }
    encoded.resize(size, 0);
    Ok(encoded)
}

/// Run patch commands against `base`, appending to `out`. Files to
/// compress again are done first, in parallel.
fn run(base: &[u8], commands: &[u8], out: &mut Vec<u8>) -> Result<(), PatchError> {
    let ops = parse_ops(commands)?;
    let jobs: Vec<&Op> = ops.iter().filter(|o| matches!(o, Op::Yaz { .. })).collect();
    let job = |o: &&Op| match **o {
        Op::Yaz { src, len, size, sub } => rebuild(base, src, len, size, sub),
        _ => unreachable!(),
    };
    #[cfg(feature = "parallel")]
    let rebuilt: Vec<Result<Vec<u8>, PatchError>> = {
        use rayon::prelude::*;
        jobs.par_iter().map(job).collect()
    };
    #[cfg(not(feature = "parallel"))]
    let rebuilt: Vec<Result<Vec<u8>, PatchError>> = jobs.iter().map(job).collect();
    let mut rebuilt = rebuilt.into_iter();
    for op in &ops {
        match *op {
            Op::Copy(src, len) => out.extend_from_slice(base.get(src..src + len).ok_or(PatchError::Damaged)?),
            Op::Insert(bytes) => out.extend_from_slice(bytes),
            Op::Yaz { .. } => out.extend(rebuilt.next().ok_or(PatchError::Damaged)??),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_moves_and_edits() {
        // A "ROM" of distinct blocks; the output moves some, edits others.
        let clean: Vec<u8> = (0..0x8000u32).flat_map(|i| (i.wrapping_mul(2654435761)).to_be_bytes()).collect();
        let mut output = clean[0x1000..0x9000].to_vec();
        output.extend_from_slice(&[7u8; 304]);
        output.extend_from_slice(&clean[0x10..0x1000]);
        output[0x2345] ^= 0xFF;
        output.extend_from_slice(b"new file");
        let patch = create(&clean, &output, b"{\"seed\":1}");
        assert!(patch.len() < 1000, "patch is {} bytes", patch.len());
        let (rebuilt, info) = apply(&clean, &patch).unwrap();
        assert_eq!(rebuilt, output);
        assert_eq!(info, b"{\"seed\":1}");
        let mut other = clean.clone();
        other[0] ^= 1;
        assert!(matches!(apply(&other, &patch), Err(PatchError::WrongRom)));
    }
}
