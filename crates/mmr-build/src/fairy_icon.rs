//! The Clock Town stray fairy icon: the Snowhead fairy icon with its hue
//! rotated by -80 degrees, as the old randomizer made it with ImageSharp's
//! hue filter.
//!
//! The filter's cosine and sine are fixed bit patterns (taken from .NET),
//! because platform maths libraries can differ in the last bit, and the
//! output must be identical on every OS.

/// cos(280 degrees) and sin(280 degrees) as .NET's `MathF` computes them.
const COS: f32 = f32::from_bits(0x3E31_D0D2);
const SIN: f32 = f32::from_bits(0xBF7C_1C5C);

/// The hue rotation matrix (`KnownFilterMatrices.CreateHueFilter`), as
/// (M11, M12, M13, M21, M22, M23, M31, M32, M33).
fn matrix() -> [[f32; 3]; 3] {
    let (c, s) = (COS, SIN);
    [
        [0.213 + (c * 0.787) - (s * 0.213), 0.213 - (c * 0.213) + (s * 0.143), 0.213 - (c * 0.213) - (s * 0.787)],
        [0.715 - (c * 0.715) - (s * 0.715), 0.715 + (c * 0.285) + (s * 0.140), 0.715 - (c * 0.715) + (s * 0.715)],
        [0.072 - (c * 0.072) + (s * 0.928), 0.072 - (c * 0.072) - (s * 0.283), 0.072 + (c * 0.928) + (s * 0.072)],
    ]
}

/// Recolour a 32x24 RGBA icon.
pub fn clock_town_fairy_icon(snowhead_icon: &[u8]) -> Vec<u8> {
    let m = matrix();
    let mut out = snowhead_icon.to_vec();
    for px in out.chunks_exact_mut(4) {
        let v = [px[0] as f32 / 255.0, px[1] as f32 / 255.0, px[2] as f32 / 255.0];
        for (i, value) in px.iter_mut().take(3).enumerate() {
            let t = v[0] * m[0][i] + v[1] * m[1][i] + v[2] * m[2][i];
            let scaled = (t * 255.0 + 0.5).clamp(0.0, 255.0);
            *value = scaled as u8;
        }
    }
    out
}
