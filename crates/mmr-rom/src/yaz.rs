//! Yaz0 compression, as used by Majora's Mask for most ROM files.
//!
//! The encoder is a literal port of the old randomizer's (`MMR.Yaz`), including
//! its exact match-finding order, so it produces byte-identical output. That
//! lets ROMs be compared with ones built by older versions.

/// Size of a Yaz0 header.
pub const HEADER_SIZE: usize = 0x10;
/// "Yaz0".
pub const MAGIC: u32 = 0x5961_7A30;
/// Longest run one copy command can encode.
const MAX_COMPARE: usize = 0xFF + 0x12;
/// How far back a copy command can reach.
const WINDOW: usize = 0x1000;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum YazError {
    #[error("not Yaz0 data")]
    BadMagic,
    #[error("Yaz0 data ends early")]
    Truncated,
}

/// Decompressed size stored in a Yaz0 header.
pub fn decoded_size(src: &[u8]) -> Result<usize, YazError> {
    if src.len() < HEADER_SIZE {
        return Err(YazError::Truncated);
    }
    if u32::from_be_bytes(src[0..4].try_into().unwrap()) != MAGIC {
        return Err(YazError::BadMagic);
    }
    Ok(u32::from_be_bytes(src[4..8].try_into().unwrap()) as usize)
}

/// Decompress Yaz0 data (with header).
pub fn decode(src: &[u8]) -> Result<Vec<u8>, YazError> {
    let size = decoded_size(src)?;
    let src = &src[HEADER_SIZE..];
    let mut dst = vec![0u8; size];
    let (mut d, mut s) = (0usize, 0usize);
    let mut bits = 0u32;
    let mut code = 0u8;
    let byte = |s: usize| src.get(s).copied().ok_or(YazError::Truncated);
    while d < size {
        if bits == 0 {
            code = byte(s)?;
            s += 1;
            if code == 0xFF && d + 8 <= size {
                dst[d..d + 8].copy_from_slice(src.get(s..s + 8).ok_or(YazError::Truncated)?);
                d += 8;
                s += 8;
                continue;
            }
            bits = 8;
        }
        if code & 0x80 != 0 {
            dst[d] = byte(s)?;
            d += 1;
            s += 1;
        } else {
            let (b1, b2) = (byte(s)? as usize, byte(s + 1)? as usize);
            s += 2;
            let dist = ((b1 & 0xF) << 8) | b2;
            let mut from = d.checked_sub(dist + 1).ok_or(YazError::Truncated)?;
            let n = match b1 >> 4 {
                0 => {
                    s += 1;
                    byte(s - 1)? as usize + 0x12
                }
                n => n + 2,
            };
            for _ in 0..n {
                if d >= size {
                    break;
                }
                dst[d] = dst[from];
                d += 1;
                from += 1;
            }
        }
        code <<= 1;
        bits -= 1;
    }
    Ok(dst)
}

/// Compress to Yaz0 (with header), padded to a multiple of 16 bytes like the
/// game's files.
pub fn encode(src: &[u8]) -> Vec<u8> {
    let max = (src.len() / 8 + usize::from(src.len() % 8 != 0)) * 9;
    let mut dst = vec![0u8; HEADER_SIZE + max];
    dst[0..4].copy_from_slice(&MAGIC.to_be_bytes());
    dst[4..8].copy_from_slice(&(src.len() as u32).to_be_bytes());
    let written = HEADER_SIZE + encode_raw(src, &mut dst[HEADER_SIZE..]);
    dst.truncate((written + 0xF) & !0xF);
    dst.resize((written + 0xF) & !0xF, 0);
    dst
}

/// The old encoder's "deltas" ring buffer, kept exactly (including `pop`
/// only shrinking the count, which matters once the buffer is full).
struct Window {
    buf: Vec<i32>,
    amount: usize,
    origin: usize,
}

impl Window {
    fn get(&self, index: usize) -> i32 {
        let p = self.origin + index;
        self.buf[if p < WINDOW { p } else { p - WINDOW }]
    }
    fn put(&mut self, index: usize, v: i32) {
        let p = self.origin + index;
        let i = if p < WINDOW { p } else { p - WINDOW };
        self.buf[i] = v;
    }
    fn push(&mut self, v: i32) {
        if self.amount < WINDOW {
            self.put(self.amount, v);
            self.amount += 1;
        } else {
            self.put(0, v);
            self.origin = if self.origin + 1 < WINDOW { self.origin + 1 } else { 0 };
        }
    }
    fn pop(&mut self) {
        if self.amount != 0 {
            self.amount -= 1;
        }
    }
}

#[derive(Clone, Copy)]
struct Found {
    offset: i32,
    length: usize,
    skip_byte: bool,
}

const NONE: Found = Found { offset: -1, length: 0, skip_byte: false };

struct Encoder<'a> {
    src: &'a [u8],
    /// Last position (+1) of each byte value; 0 = not seen.
    recents: [i32; 256],
    window: Window,
}

impl Encoder<'_> {
    fn remember(&mut self, pos: usize) {
        let v = self.src[pos] as usize;
        let recent = self.recents[v] - 1;
        self.recents[v] = pos as i32 + 1;
        self.window.push(recent);
    }

    fn lookup(&self, pos: usize) -> Found {
        let src = self.src;
        let recent = self.recents[src[pos] as usize] - 1;
        if recent < 0 || WINDOW < pos - recent as usize {
            return NONE;
        }
        let mut search = recent as usize;
        let current = &src[pos..pos + MAX_COMPARE.min(src.len() - pos)];
        let window_start = pos.saturating_sub(WINDOW);
        let mut result = NONE;
        while window_start <= search {
            let candidate = &src[search..];
            let potential = current.len().min(candidate.len());
            let matches = (0..potential).find(|&i| current[i] != candidate[i]).unwrap_or(potential);
            if matches >= 3 && matches >= result.length {
                result = Found { offset: search as i32, length: matches, skip_byte: false };
                if matches == MAX_COMPARE {
                    break;
                }
            }
            let next = self.window.get(search - window_start);
            if next < 0 {
                break;
            }
            search = next as usize;
        }
        result
    }

    fn lookup_ahead(&mut self, pos: usize) -> Found {
        let first = self.lookup(pos);
        if first.offset >= 0 && pos + 1 < self.src.len() {
            let v = self.src[pos] as usize;
            let saved = self.recents[v];
            self.remember(pos);
            let second = self.lookup(pos + 1);
            self.recents[v] = saved;
            self.window.pop();
            if second.length >= first.length + 2 {
                return Found { skip_byte: true, ..second };
            }
        }
        first
    }
}

fn encode_raw(src: &[u8], dst: &mut [u8]) -> usize {
    let mut e = Encoder { src, recents: [0; 256], window: Window { buf: vec![0; WINDOW], amount: 0, origin: 0 } };
    let (mut code_at, mut bit, mut d, mut s) = (0usize, 0u32, 0usize, 0usize);
    let mut result = NONE;
    while s < src.len() {
        if bit == 0 {
            code_at = d;
            d += 1;
            dst[code_at] = 0;
        }
        if result.offset < 0 {
            result = e.lookup_ahead(s);
        }
        e.remember(s);
        if !result.skip_byte && result.offset >= 0 {
            let distance = s - result.offset as usize - 1;
            if result.length < 0x12 {
                dst[d] = (((result.length - 2) << 4) | (distance >> 8)) as u8;
                dst[d + 1] = distance as u8;
                d += 2;
            } else {
                dst[d] = (distance >> 8) as u8;
                dst[d + 1] = distance as u8;
                dst[d + 2] = (result.length - 0x12) as u8;
                d += 3;
            }
            for j in 1..result.length {
                e.remember(s + j);
            }
            s += result.length;
        } else {
            dst[code_at] |= 1 << (7 - bit);
            dst[d] = src[s];
            d += 1;
            s += 1;
        }
        result = if result.skip_byte { Found { skip_byte: false, ..result } } else { NONE };
        bit = (bit + 1) % 8;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let mut data = Vec::new();
        for i in 0..20000u32 {
            data.push((i * 7 % 13) as u8);
            if i % 50 < 20 {
                data.push(0);
            }
            data.extend_from_slice(&(i / 100).to_be_bytes());
        }
        for len in [0, 1, 2, 3, 7, 8, 9, 100, 4097, data.len()] {
            let enc = encode(&data[..len]);
            assert_eq!(enc.len() % 16, 0);
            assert_eq!(decode(&enc).unwrap(), &data[..len], "length {len}");
        }
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(decode(b"nope nope nope nope"), Err(YazError::BadMagic));
        assert_eq!(decode(b"Yaz0"), Err(YazError::Truncated));
    }
}
