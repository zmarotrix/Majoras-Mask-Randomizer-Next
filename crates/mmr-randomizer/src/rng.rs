//! Seeds and random numbers.
//!
//! The generator is ChaCha8, implemented here so that what a seed produces
//! depends only on this file: no library update can silently change seeds.
//! Each phase of randomization (items, entrances, prices, ...) draws from its
//! own stream, so changing settings for one phase never reshuffles another.
//!
//! Same seed, same result on every platform: only whole-number arithmetic is
//! used (no floating point), and callers must never let hash-map iteration
//! order influence a random choice (the randomizer uses ordered collections).

use serde::{Deserialize, Serialize};
use std::fmt;

/// A seed. Displayed as a number; any text can be used to make one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Seed(pub u64);

impl Seed {
    /// A number is used as-is; any other text is hashed, so "racecat" is a
    /// valid, reproducible seed.
    pub fn parse(text: &str) -> Seed {
        let t = text.trim();
        match t.parse::<u64>() {
            Ok(n) => Seed(n),
            Err(_) => Seed(fnv1a(t.as_bytes())),
        }
    }

    /// A fresh seed from the operating system's random source.
    pub fn random() -> Seed {
        use std::hash::{BuildHasher, Hasher};
        // RandomState keys come from the OS; hashing a counter through two
        // independent instances gives 64 unpredictable bits without extra deps.
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(std::collections::hash_map::RandomState::new().hash_one(0u64));
        Seed(h.finish())
    }
}

impl fmt::Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Independent random streams, one per phase. Never renumber these: doing
/// so changes every seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum Stream {
    Items = 1,
    Entrances = 2,
    Prices = 3,
    Hints = 4,
    Traps = 5,
    Cosmetics = 6,
    Gibdos = 7,
    StartingItems = 8,
}

/// ChaCha with 8 rounds.
pub struct Rng {
    state: [u32; 16],
    block: [u32; 16],
    index: usize,
}

const ROUNDS: usize = 8;

impl Rng {
    /// The generator for one phase of one attempt of one seed.
    pub fn new(seed: Seed, stream: Stream, attempt: u32) -> Rng {
        let s = seed.0;
        // Key: seed (2 words), attempt, then a fixed salt (never change it:
        // every seed would change).
        let key = [s as u32, (s >> 32) as u32, attempt, 0, 0, 0, 0, u32::from_le_bytes(*b"MMR2")];
        Rng::from_key(key, stream as u64)
    }

    fn from_key(key: [u32; 8], nonce: u64) -> Rng {
        let mut state = [0u32; 16];
        state[..4].copy_from_slice(&[0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574]); // "expand 32-byte k"
        state[4..12].copy_from_slice(&key);
        // Words 12-13: 64-bit block counter. Words 14-15: 64-bit stream id.
        state[14] = nonce as u32;
        state[15] = (nonce >> 32) as u32;
        Rng { state, block: [0; 16], index: 16 }
    }

    fn refill(&mut self) {
        self.block = chacha_block(&self.state, ROUNDS);
        let counter = (self.state[12] as u64 | (self.state[13] as u64) << 32).wrapping_add(1);
        self.state[12] = counter as u32;
        self.state[13] = (counter >> 32) as u32;
        self.index = 0;
    }

    pub fn next_u32(&mut self) -> u32 {
        if self.index == 16 {
            self.refill();
        }
        let v = self.block[self.index];
        self.index += 1;
        v
    }

    pub fn next_u64(&mut self) -> u64 {
        self.next_u32() as u64 | (self.next_u32() as u64) << 32
    }

    /// Uniform integer in `0..n` (Lemire's method, unbiased). `n` must be > 0.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0, "below(0)");
        let n = n as u64;
        loop {
            let x = self.next_u64();
            let m = (x as u128) * (n as u128);
            let low = m as u64;
            if low >= n || low >= n.wrapping_neg() % n {
                return (m >> 64) as usize;
            }
        }
    }

    /// An index picked with probability proportional to its weight. The
    /// weights must not all be 0.
    pub fn weighted(&mut self, weights: &[u32]) -> usize {
        let total: u64 = weights.iter().map(|&w| w as u64).sum();
        let mut r = self.below(total as usize) as u64;
        for (i, &w) in weights.iter().enumerate() {
            if r < w as u64 {
                return i;
            }
            r -= w as u64;
        }
        weights.len() - 1
    }

    /// Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }

    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.below(items.len())])
        }
    }

    /// `n` distinct elements in random order (all of them if fewer).
    pub fn sample<T: Clone>(&mut self, items: &[T], n: usize) -> Vec<T> {
        let mut v = items.to_vec();
        self.shuffle(&mut v);
        v.truncate(n);
        v
    }
}

fn quarter(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = (s[d] ^ s[a]).rotate_left(16);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_left(12);
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = (s[d] ^ s[a]).rotate_left(8);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_left(7);
}

fn chacha_block(input: &[u32; 16], rounds: usize) -> [u32; 16] {
    let mut s = *input;
    for _ in 0..rounds / 2 {
        quarter(&mut s, 0, 4, 8, 12);
        quarter(&mut s, 1, 5, 9, 13);
        quarter(&mut s, 2, 6, 10, 14);
        quarter(&mut s, 3, 7, 11, 15);
        quarter(&mut s, 0, 5, 10, 15);
        quarter(&mut s, 1, 6, 11, 12);
        quarter(&mut s, 2, 7, 8, 13);
        quarter(&mut s, 3, 4, 9, 14);
    }
    for (o, i) in s.iter_mut().zip(input) {
        *o = o.wrapping_add(*i);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 7539 section 2.3.2 block test vector (ChaCha20).
    #[test]
    fn chacha20_matches_rfc7539() {
        let key: Vec<u32> = (0u8..32).collect::<Vec<_>>().chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
        let mut state = [0u32; 16];
        state[..4].copy_from_slice(&[0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574]);
        state[4..12].copy_from_slice(&key);
        state[12] = 1;
        state[13] = 0x0900_0000;
        state[14] = 0x4a00_0000;
        state[15] = 0;
        let out = chacha_block(&state, 20);
        let expected = [
            0xe4e7_f110, 0x1559_3bd1, 0x1fdd_0f50, 0xc471_20a3, 0xc7f4_d1c7, 0x0368_c033, 0x9aaa_2204, 0x4e6c_d4c3, 0x4664_82d2, 0x09aa_9f07,
            0x05d7_c214, 0xa202_8bd9, 0xd19c_12b5, 0xb94e_16de, 0xe883_d0cb, 0x4e3c_50a2,
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn deterministic_and_independent_streams() {
        let a: Vec<u32> = { let mut r = Rng::new(Seed(42), Stream::Items, 0); (0..8).map(|_| r.next_u32()).collect() };
        let b: Vec<u32> = { let mut r = Rng::new(Seed(42), Stream::Items, 0); (0..8).map(|_| r.next_u32()).collect() };
        let c: Vec<u32> = { let mut r = Rng::new(Seed(42), Stream::Cosmetics, 0); (0..8).map(|_| r.next_u32()).collect() };
        let d: Vec<u32> = { let mut r = Rng::new(Seed(42), Stream::Items, 1); (0..8).map(|_| r.next_u32()).collect() };
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn below_is_roughly_uniform() {
        let mut r = Rng::new(Seed(7), Stream::Items, 0);
        let mut counts = [0usize; 6];
        for _ in 0..60_000 {
            counts[r.below(6)] += 1;
        }
        for c in counts {
            assert!((9_000..11_000).contains(&c), "{counts:?}");
        }
    }

    #[test]
    fn text_seeds_are_stable() {
        assert_eq!(Seed::parse("12345"), Seed(12345));
        assert_eq!(Seed::parse(" racecat "), Seed::parse("racecat"));
        assert_ne!(Seed::parse("racecat"), Seed::parse("racedog"));
    }
}
