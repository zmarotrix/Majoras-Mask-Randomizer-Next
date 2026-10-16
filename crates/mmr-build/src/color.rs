//! Colour maths, exactly as .NET does it (`System.Drawing.Color` hue,
//! saturation and brightness, and the old `ColorUtils.FromAHSB`), so hue
//! shifts give the same bytes as the old randomizer.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    fn min_max(self) -> (i32, i32) {
        let (r, g, b) = (self.r as i32, self.g as i32, self.b as i32);
        (r.min(g).min(b), r.max(g).max(b))
    }

    /// `Color.GetHue`, in degrees.
    pub fn hue(self) -> f32 {
        let (r, g, b) = (self.r as i32, self.g as i32, self.b as i32);
        if r == g && g == b {
            return 0.0;
        }
        let (min, max) = self.min_max();
        let delta = (max - min) as f32;
        let mut hue = if r == max {
            (g - b) as f32 / delta
        } else if g == max {
            (b - r) as f32 / delta + 2.0
        } else {
            (r - g) as f32 / delta + 4.0
        };
        hue *= 60.0;
        if hue < 0.0 {
            hue += 360.0;
        }
        hue
    }

    /// `Color.GetSaturation`.
    pub fn saturation(self) -> f32 {
        let (r, g, b) = (self.r as i32, self.g as i32, self.b as i32);
        if r == g && g == b {
            return 0.0;
        }
        let (min, max) = self.min_max();
        let mut div = max + min;
        if div > 255 {
            div = 255 * 2 - max - min;
        }
        (max - min) as f32 / div as f32
    }

    /// `Color.GetBrightness`.
    pub fn brightness(self) -> f32 {
        let (min, max) = self.min_max();
        (max + min) as f32 / (255.0f32 * 2.0)
    }

    /// The same colour with its hue rotated by `degrees` (the old title
    /// screen and file select recolouring).
    pub fn rotate_hue(self, degrees: f32) -> Rgb {
        let h = (self.hue() + degrees) % 360.0;
        from_hsb(h, self.saturation(), self.brightness())
    }
}

/// `Convert.ToInt32(float)`: round half to even.
fn to_int(v: f32) -> i32 {
    (v as f64).round_ties_even() as i32
}

/// `ColorUtils.FromAHSB` (alpha is passed through unchanged by callers).
pub fn from_hsb(hue: f32, saturation: f32, brightness: f32) -> Rgb {
    let rgb = |r: i32, g: i32, b: i32| Rgb::new(r as u8, g as u8, b as u8);
    if saturation == 0.0 {
        let v = to_int(brightness * 255.0);
        return rgb(v, v, v);
    }
    let (f_max, f_min) = if 0.5 < brightness as f64 {
        (brightness - (brightness * saturation) + saturation, brightness + (brightness * saturation) - saturation)
    } else {
        (brightness + (brightness * saturation), brightness - (brightness * saturation))
    };
    let sextant = ((hue / 60.0) as f64).floor() as i32;
    let mut hue = hue;
    if 300.0 <= hue {
        hue -= 360.0;
    }
    hue /= 60.0;
    hue -= 2.0 * ((((sextant as f32 + 1.0) % 6.0) / 2.0) as f64).floor() as f32;
    let f_mid = if sextant % 2 == 0 { hue * (f_max - f_min) + f_min } else { f_min - hue * (f_max - f_min) };
    let (max, mid, min) = (to_int(f_max * 255.0), to_int(f_mid * 255.0), to_int(f_min * 255.0));
    match sextant {
        1 => rgb(mid, max, min),
        2 => rgb(min, max, mid),
        3 => rgb(min, mid, max),
        4 => rgb(mid, min, max),
        5 => rgb(max, min, mid),
        _ => rgb(max, mid, min),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_primary_colours() {
        for c in [Rgb::new(255, 0, 0), Rgb::new(0, 255, 0), Rgb::new(0, 0, 255), Rgb::new(100, 150, 255), Rgb::new(10, 10, 10)] {
            assert_eq!(c.rotate_hue(0.0), c);
            assert_eq!(c.rotate_hue(360.0), c);
        }
        assert_eq!(Rgb::new(255, 0, 0).rotate_hue(120.0), Rgb::new(0, 255, 0));
    }
}

impl Rgb {
    /// `ColorExtensions.ChangeBrightness`: towards white (positive) or
    /// black (negative), truncating.
    pub fn change_brightness(self, factor: f32) -> Rgb {
        let (mut r, mut g, mut b) = (self.r as f32, self.g as f32, self.b as f32);
        if factor < 0.0 {
            let f = 1.0 + factor;
            r *= f;
            g *= f;
            b *= f;
        } else {
            r = (255.0 - r) * factor + r;
            g = (255.0 - g) * factor + g;
            b = (255.0 - b) * factor + b;
        }
        Rgb::new(r as i32 as u8, g as i32 as u8, b as i32 as u8)
    }
    pub fn brighten(self, amount: f32) -> Rgb {
        self.change_brightness(amount)
    }
    pub fn darken(self, amount: f32) -> Rgb {
        self.change_brightness(amount * -1.0)
    }
}

/// A colour in ImageSharp's HSV space (H in degrees, S and V 0-1).
#[derive(Debug, Clone, Copy)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

const EPSILON: f32 = 0.001;

impl Hsv {
    /// Clamped like ImageSharp's `Hsv` constructor.
    pub fn new(h: f32, s: f32, v: f32) -> Hsv {
        Hsv { h: h.clamp(0.0, 360.0), s: s.clamp(0.0, 1.0), v: v.clamp(0.0, 1.0) }
    }

    /// ImageSharp `HsvAndRgbConverter` (RGB to HSV).
    pub fn from_rgb(c: Rgb) -> Hsv {
        let (r, g, b) = (c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0);
        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let chroma = max - min;
        let v = max;
        if chroma.abs() < EPSILON {
            return Hsv::new(0.0, 0.0, v);
        }
        let mut h = 0.0;
        if (r - max).abs() < EPSILON {
            h = (g - b) / chroma;
        } else if (g - max).abs() < EPSILON {
            h = 2.0 + ((b - r) / chroma);
        } else if (b - max).abs() < EPSILON {
            h = 4.0 + ((r - g) / chroma);
        }
        h *= 60.0;
        if (h as f64) < 0.0 {
            h += 360.0;
        }
        Hsv::new(h, chroma / v, v)
    }

    /// HSV to RGB, then to bytes by truncation (`ColorSpaceConverterExtensions.ToColor`).
    pub fn to_rgb(self) -> Rgb {
        let (s, v) = (self.s, self.v);
        let (r, g, b) = if s.abs() < EPSILON {
            (v, v, v)
        } else {
            let h = if (self.h - 360.0).abs() < EPSILON { 0.0 } else { self.h / 60.0 };
            let i = (h as f64).trunc() as i32;
            let f = h - i as f32;
            let p = v * (1.0 - s);
            let q = v * (1.0 - (s * f));
            let t = v * (1.0 - (s * (1.0 - f)));
            match i {
                0 => (v, t, p),
                1 => (q, v, p),
                2 => (p, v, t),
                3 => (p, q, v),
                4 => (t, p, v),
                _ => (v, p, q),
            }
        };
        let (r, g, b) = (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
        Rgb::new((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }
}

/// `TranslateHsv`: adjust a colour in HSV space.
pub fn translate_hsv(c: Rgb, f: impl Fn(Hsv) -> Hsv) -> Rgb {
    f(Hsv::from_rgb(c)).to_rgb()
}
