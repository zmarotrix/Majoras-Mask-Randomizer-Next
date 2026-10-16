//! Golden seeds: the same seed must give exactly the same result on every
//! platform (Windows, macOS, Linux, Android, wasm) and in debug and release
//! builds. CI runs this on each platform.
//!
//! When placement rules change on purpose, these values change too: update
//! them in the same commit and say so in the commit message. They must never
//! differ between platforms.

use mmr_randomizer::rng::{Rng, Seed, Stream};
use mmr_randomizer::{fingerprint, generate, Style};

#[test]
fn rng_stream_is_fixed() {
    let mut r = Rng::new(Seed(1), Stream::Items, 0);
    let first: Vec<u32> = (0..4).map(|_| r.next_u32()).collect();
    assert_eq!(first, [860321602, 885986139, 3836248502, 4153363010]);
}

#[test]
fn golden_seeds() {
    let logic = mmr_logic::bundled("casual").unwrap();
    for (preset, seed, style, expected) in [
        ("season-2-standard", "racecat", Style::Random, 0x8cff_2b8b_ea83_decfu64),
        ("modern", "12345", Style::Bespoke, 0x5754_c809_1bf5_8e25),
        ("season-1-standard", "9", Style::Random, 0xa5f2_497b_12f6_97a9),
    ] {
        let p = mmr_settings::presets().into_iter().find(|p| p.id == preset).unwrap();
        let values = mmr_settings::apply_preset(&p, None);
        let out = generate(&logic, &values, Seed::parse(seed), style).unwrap();
        assert_eq!(fingerprint(&out), expected, "{preset} / seed {seed} / {style:?} changed");
    }
}

/// Prints current fingerprints, for updating the values above after a
/// deliberate rule change: `cargo test --test golden -- --ignored --nocapture`.
#[test]
#[ignore]
fn print_golden_values() {
    let logic = mmr_logic::bundled("casual").unwrap();
    for (preset, seed, style) in [("season-2-standard", "racecat", Style::Random), ("modern", "12345", Style::Bespoke), ("season-1-standard", "9", Style::Random)] {
        let p = mmr_settings::presets().into_iter().find(|p| p.id == preset).unwrap();
        let out = generate(&logic, &mmr_settings::apply_preset(&p, None), Seed::parse(seed), style).unwrap();
        println!("{preset} {seed} {style:?}: {:#018x}", fingerprint(&out));
    }
}
