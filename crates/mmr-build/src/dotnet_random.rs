//! .NET's seeded `System.Random` (the Knuth subtractive generator that
//! `new Random(seed)` uses), for the parts of the build the old randomizer
//! made with it. Integer maths plus IEEE double multiplies only, so it gives
//! the same numbers on every OS.

pub struct DotNetRandom {
    seed_array: [i32; 56],
    inext: usize,
    inextp: usize,
}

const MBIG: i32 = i32::MAX;
const MSEED: i32 = 161_803_398;

impl DotNetRandom {
    pub fn new(seed: i32) -> Self {
        let mut a = [0i32; 56];
        let subtraction = if seed == i32::MIN { i32::MAX } else { seed.abs() };
        let mut mj = MSEED - subtraction;
        a[55] = mj;
        let mut mk = 1i32;
        let mut ii = 0usize;
        for _ in 1..55 {
            ii += 21;
            if ii >= 55 {
                ii -= 55;
            }
            a[ii] = mk;
            mk = mj.wrapping_sub(mk);
            if mk < 0 {
                mk = mk.wrapping_add(MBIG);
            }
            mj = a[ii];
        }
        for _ in 1..5 {
            for i in 1..56 {
                let mut n = i + 30;
                if n >= 55 {
                    n -= 55;
                }
                a[i] = a[i].wrapping_sub(a[1 + n]);
                if a[i] < 0 {
                    a[i] = a[i].wrapping_add(MBIG);
                }
            }
        }
        DotNetRandom { seed_array: a, inext: 0, inextp: 21 }
    }

    fn internal_sample(&mut self) -> i32 {
        let mut inext = self.inext + 1;
        if inext >= 56 {
            inext = 1;
        }
        let mut inextp = self.inextp + 1;
        if inextp >= 56 {
            inextp = 1;
        }
        let mut r = self.seed_array[inext].wrapping_sub(self.seed_array[inextp]);
        if r == MBIG {
            r -= 1;
        }
        if r < 0 {
            r = r.wrapping_add(MBIG);
        }
        self.seed_array[inext] = r;
        self.inext = inext;
        self.inextp = inextp;
        r
    }

    fn sample(&mut self) -> f64 {
        self.internal_sample() as f64 * (1.0 / MBIG as f64)
    }

    /// `Next()`.
    pub fn next(&mut self) -> i32 {
        self.internal_sample()
    }

    /// `Next(max)`: 0 ≤ n < max.
    pub fn next_below(&mut self, max: i32) -> i32 {
        (self.sample() * max as f64) as i32
    }

    /// `Next(min, max)`: min ≤ n < max.
    pub fn next_range(&mut self, min: i32, max: i32) -> i32 {
        let range = max as i64 - min as i64;
        if range <= i32::MAX as i64 {
            return (self.sample() * range as f64) as i32 + min;
        }
        (self.large_range_sample() * range as f64) as i64 as i32 + min
    }

    fn large_range_sample(&mut self) -> f64 {
        let mut result = self.internal_sample();
        if self.internal_sample() % 2 == 0 {
            result = -result;
        }
        let mut d = result as f64;
        d += (i32::MAX - 1) as f64;
        d /= 2.0 * (u32::MAX >> 1) as f64 - 1.0;
        d
    }

    /// `NextDouble()`.
    pub fn next_double(&mut self) -> f64 {
        self.sample()
    }

    /// `NextBytes`.
    pub fn next_bytes(&mut self, buf: &mut [u8]) {
        for b in buf {
            *b = self.internal_sample() as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_dotnet() {
        // new Random(42).Next() x3, and new Random(0).Next(100) x3, in .NET.
        let mut r = DotNetRandom::new(42);
        assert_eq!([r.next(), r.next(), r.next()], [1434747710, 302596119, 269548474]);
        for (seed, next, below, range, double) in [
            (0, [1559595546, 1755192844, 1649316166], [72, 81, 76], 555, 0.2060331540210327),
            (-7, [822959691, 1871007331, 1419354884], [38, 87, 66], 47, 0.36643333237917786),
            (123456789, [1091672793, 381850644, 1335622286], [50, 17, 62], 400, 0.9167651384681301),
        ] {
            let mut r = DotNetRandom::new(seed);
            assert_eq!([r.next(), r.next(), r.next()], next);
            let mut r = DotNetRandom::new(seed);
            assert_eq!([r.next_below(100), r.next_below(100), r.next_below(100)], below);
            assert_eq!(r.next_range(-5, 1000), range);
            assert_eq!(r.next_double(), double);
        }
    }
}
