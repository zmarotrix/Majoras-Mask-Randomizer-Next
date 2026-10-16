//! The N64 boot checksum (CIC 6105, which Majora's Mask uses).
//! Reference: http://n64dev.org/n64crc.html

/// Recompute the two header checksums in place.
pub fn fix(rom: &mut [u8]) {
    let (a, b) = compute(rom);
    rom[0x10..0x14].copy_from_slice(&a.to_be_bytes());
    rom[0x14..0x18].copy_from_slice(&b.to_be_bytes());
}

/// The checksums a ROM should have.
pub fn compute(rom: &[u8]) -> (u32, u32) {
    let word = |o: usize| u32::from_be_bytes(rom[o..o + 4].try_into().unwrap());
    let seed = 0xDF26_F436u32;
    let (mut t1, mut t2, mut t3, mut t4, mut t5, mut t6) = (seed, seed, seed, seed, seed, seed);
    for i in (0x1000..0x10_1000).step_by(4) {
        let d = word(i);
        if t6.wrapping_add(d) < t6 {
            t4 = t4.wrapping_add(1);
        }
        t6 = t6.wrapping_add(d);
        t3 ^= d;
        let r = d.rotate_left(d & 0x1F);
        t5 = t5.wrapping_add(r);
        t2 ^= if t2 < d { t6 ^ d } else { r };
        t1 = t1.wrapping_add(word(0x750 + (i & 0xFF)) ^ d);
    }
    (t6 ^ t4 ^ t3, t5 ^ t2 ^ t1)
}
