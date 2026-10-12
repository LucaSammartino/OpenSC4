// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The game's two random generators, bit-exact (`docs/specs/foundations.md`
//! §6.1). Clock-derived seeds are injected (`SeedSource`) so runs are
//! reproducible in tests (§6.3).

/// Seed value that means "derive from the clock" (§6.1).
pub const CLOCK_SEED: u32 = 0xFFFF_FFFF;

/// Supplies the clock values the original reads for clock-based seeding.
pub trait SeedSource {
    /// Wall-clock derived seed (used for `Seed(0xFFFFFFFF)`).
    fn wall_clock_seed(&self) -> u32;
    /// The process clock (used when the double path's state is 0).
    fn process_clock(&self) -> u32;
}

/// A fixed seed source, for tests and replays.
#[derive(Clone, Copy, Debug)]
pub struct FixedSeeds {
    pub wall_clock: u32,
    pub process_clock: u32,
}

impl SeedSource for FixedSeeds {
    fn wall_clock_seed(&self) -> u32 {
        self.wall_clock
    }
    fn process_clock(&self) -> u32 {
        self.process_clock
    }
}

const TWO_POW_32: f64 = 4_294_967_296.0;

/// `cRZRandom`: an LCG for the integer methods (`lcg`) and a multiplicative
/// generator for the double methods (`mul`).
#[derive(Clone, Debug)]
pub struct RzRandom {
    lcg: u32,
    mul: u32,
    cached_gaussian: Option<f64>,
    process_clock: u32,
}

impl RzRandom {
    /// `Seed(s)`; `s == 0xFFFFFFFF` takes the wall-clock seed from `seeds`.
    pub fn new(seed: u32, seeds: &dyn SeedSource) -> Self {
        let mut r = RzRandom {
            lcg: 0,
            mul: 0,
            cached_gaussian: None,
            process_clock: seeds.process_clock(),
        };
        r.seed(seed, seeds);
        r
    }

    pub fn seed(&mut self, seed: u32, seeds: &dyn SeedSource) {
        let s = if seed == CLOCK_SEED {
            seeds.wall_clock_seed()
        } else {
            seed
        };
        self.lcg = s;
        self.mul = s;
        self.cached_gaussian = None;
        self.process_clock = seeds.process_clock();
    }

    pub fn lcg_state(&self) -> u32 {
        self.lcg
    }

    pub fn mul_state(&self) -> u32 {
        self.mul
    }

    /// The Gaussian value cached by the last `gaussian()` call, if unused.
    pub fn cached_gaussian(&self) -> Option<f64> {
        self.cached_gaussian
    }

    /// Restores a saved generator (both paths and the Gaussian cache), as
    /// subsystems that persist their generator do (weather spec §10).
    pub fn set_state(&mut self, lcg: u32, mul: u32, cached_gaussian: Option<f64>) {
        self.lcg = lcg;
        self.mul = mul;
        self.cached_gaussian = cached_gaussian;
    }

    /// `RandomUint32Uniform()`.
    pub fn next_raw(&mut self) -> u32 {
        let t = u64::from(self.lcg) * 0x41C6_4E6D + 0x3039;
        self.lcg = t as u32;
        (t >> 16) as u32
    }

    /// `RandomUint32Uniform(n)`: 0 for n == 0 without advancing.
    pub fn uniform_u32(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        ((u64::from(self.next_raw()) * u64::from(n)) >> 32) as u32
    }

    /// `RandomSint32RangeUniform(lo, hi)`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        lo.wrapping_add(self.uniform_u32(hi.wrapping_sub(lo) as u32) as i32)
    }

    /// `RandomDoubleUniform()`: [0, 1).
    pub fn uniform_f64(&mut self) -> f64 {
        if self.mul == 0 {
            self.mul = self.process_clock;
        }
        self.mul = self.mul.wrapping_mul(0x278D_DE6D);
        f64::from(self.mul as i32) / TWO_POW_32 + 0.5
    }

    /// `RandomDoubleRangeUniform(a, b)`.
    pub fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        a + self.uniform_f64() * (b - a)
    }

    /// `RandomDoubleGaussian()`: Marsaglia polar method; one value is
    /// returned, the other cached for the next call. [choice] The first
    /// coordinate is returned and the second cached.
    pub fn gaussian(&mut self) -> f64 {
        if let Some(g) = self.cached_gaussian.take() {
            return g;
        }
        loop {
            let u = 2.0 * self.uniform_f64() - 1.0;
            let v = 2.0 * self.uniform_f64() - 1.0;
            let s = u * u + v * v;
            if s > 0.0 && s < 1.0 {
                let m = (-2.0 * s.ln() / s).sqrt();
                self.cached_gaussian = Some(v * m);
                return u * m;
            }
        }
    }

    /// `RandomDoubleRangeGaussian(a, b)`.
    pub fn range_gaussian(&mut self, a: f64, b: f64) -> f64 {
        let mut g = self.gaussian() + 3.0;
        if g < 0.0 {
            g = 0.0;
        } else if g >= 6.0 {
            g = 6.0 * 0.999_983_3;
        }
        a + (b - a) * g / 6.0
    }

    /// `RandomDoubleRangeGaussianFast(a, b)`.
    pub fn range_gaussian_fast(&mut self, a: f64, b: f64) -> f64 {
        let s = self.uniform_f64() + self.uniform_f64() + self.uniform_f64();
        a + s * (b - a) / 3.0
    }

    /// `RandomSint32RangeGaussianFast(lo, hi)` (foundations §6.1): always
    /// four raw draws, `a = (r1 + r2 + r3 + r4 + 2) >> 2` in 64 bits, then
    /// `lo + ((a × (u32)(hi − lo)) >> 32)`.
    pub fn range_i32_gaussian_fast(&mut self, lo: i32, hi: i32) -> i32 {
        let sum: u64 = (0..4).map(|_| u64::from(self.next_raw())).sum();
        let avg = ((sum + 2) / 4) as u32;
        let n = hi.wrapping_sub(lo) as u32;
        lo.wrapping_add(((u64::from(avg) * u64::from(n)) >> 32) as i32)
    }
}

const MT_N: usize = 624;
const MT_M: usize = 397;

/// `cRZRandomSuper`: MT19937 with the game's seeding (§6.1).
#[derive(Clone)]
pub struct RzRandomSuper {
    state: [u32; MT_N],
    index: usize,
}

impl std::fmt::Debug for RzRandomSuper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RzRandomSuper {{ index: {} }}", self.index)
    }
}

impl RzRandomSuper {
    pub fn new(seed: u32, seeds: &dyn SeedSource) -> Self {
        let mut r = RzRandomSuper {
            state: [0; MT_N],
            index: MT_N,
        };
        r.seed(seed, seeds);
        r
    }

    pub fn seed(&mut self, seed: u32, seeds: &dyn SeedSource) {
        let s = if seed == CLOCK_SEED {
            seeds.wall_clock_seed()
        } else {
            seed
        };
        let mut x = s | 1;
        for i in 0..MT_N {
            self.state[i] = (x & 0xFFFF_0000) | (x.wrapping_mul(69069) >> 16);
            x = x.wrapping_mul(0x1C58_7629).wrapping_add(0x10DCE);
        }
        self.twist();
        self.index = 0;
    }

    /// The raw (untempered) state word `i` (tests).
    pub fn state_word(&self, i: usize) -> u32 {
        self.state[i]
    }

    fn twist(&mut self) {
        for i in 0..MT_N {
            let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % MT_N] & 0x7FFF_FFFF);
            let mut v = self.state[(i + MT_M) % MT_N] ^ (y >> 1);
            if y & 1 != 0 {
                v ^= 0x9908_B0DF;
            }
            self.state[i] = v;
        }
    }

    /// `RandomUint32Uniform()`: the next tempered word.
    pub fn next_u32(&mut self) -> u32 {
        if self.index >= MT_N {
            self.twist();
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9D2C_5680;
        y ^= (y << 15) & 0xEFC6_0000;
        y ^= y >> 18;
        y
    }

    /// `RandomUint32Uniform(n)`.
    pub fn uniform_u32(&mut self, n: u32) -> u32 {
        ((u64::from(self.next_u32()) * u64::from(n)) >> 32) as u32
    }

    /// `RandomSint32RangeUniform(lo, hi)`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        lo.wrapping_add(self.uniform_u32(hi.wrapping_sub(lo) as u32) as i32)
    }

    /// `RandomDoubleUniform()`.
    pub fn uniform_f64(&mut self) -> f64 {
        f64::from(self.next_u32() as i32) / TWO_POW_32 + 0.5
    }

    /// `RandomDoubleRangeUniform(a, b)`.
    pub fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        a + self.uniform_f64() * (b - a)
    }

    /// `RandomUint32WeightedChoice(n, w)`.
    pub fn weighted_choice(&mut self, weights: &[f64]) -> u32 {
        let n = weights.len();
        if n < 2 {
            return 0;
        }
        let total: f64 = weights.iter().filter(|&&w| w > 0.0).sum();
        if total == 0.0 {
            return self.uniform_u32(n as u32);
        }
        let mut r = self.uniform_f64() * total;
        for (i, &w) in weights.iter().enumerate() {
            if w > 0.0 {
                if r < w {
                    return i as u32;
                }
                r -= w;
            }
        }
        (n - 1) as u32
    }
}

/// The C runtime `rand()` (foundations §12.1). Each replaced subsystem owns
/// one, seeded with 1 by default (§12.6 [choice]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrtRand {
    state: u32,
}

impl Default for CrtRand {
    fn default() -> Self {
        CrtRand { state: 1 }
    }
}

impl CrtRand {
    pub fn new(seed: u32) -> CrtRand {
        CrtRand { state: seed }
    }

    /// `srand(x)`.
    pub fn srand(&mut self, seed: u32) {
        self.state = seed;
    }

    pub fn state(&self) -> u32 {
        self.state
    }

    /// `rand()`: 0 … 32767.
    pub fn rand(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(214_013).wrapping_add(2_531_011);
        (self.state >> 16) & 0x7FFF
    }

    /// `std::random_shuffle(first, last)` (§12.3): exactly `n − 1` calls.
    pub fn random_shuffle<T>(&mut self, a: &mut [T]) {
        for i in 1..a.len() {
            let j = (self.rand() as usize) % (i + 1);
            a.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gaussian_fast_int() {
        // foundations §6.1: seed 1, (0, 25) gives 10, then 7; always 4 draws.
        let seeds = FixedSeeds {
            wall_clock: 5,
            process_clock: 7,
        };
        let mut r = RzRandom::new(1, &seeds);
        assert_eq!(
            [r.range_i32_gaussian_fast(0, 25), r.range_i32_gaussian_fast(0, 25)],
            [10, 7]
        );
        let before = r.lcg_state();
        r.range_i32_gaussian_fast(3, 3);
        let mut probe = RzRandom::new(before, &seeds);
        for _ in 0..4 {
            probe.next_raw();
        }
        assert_eq!(r.lcg_state(), probe.lcg_state());
    }

    #[test]
    fn crt_rand_sequences() {
        let mut r = CrtRand::default();
        let got: Vec<u32> = (0..10).map(|_| r.rand()).collect();
        assert_eq!(got, [41, 18467, 6334, 26500, 19169, 15724, 11478, 29358, 26962, 24464]);
        r.srand(12345);
        assert_eq!([r.rand(), r.rand(), r.rand()], [7584, 19164, 25795]);
        let mut r = CrtRand::default();
        let mut a: Vec<u32> = (0..10).collect();
        r.random_shuffle(&mut a);
        assert_eq!(a, [8, 1, 9, 2, 0, 5, 7, 3, 4, 6]);
    }

    const SEEDS: FixedSeeds = FixedSeeds {
        wall_clock: 12345,
        process_clock: 777,
    };

    #[test]
    fn rz_random_integer_vectors() {
        let mut r = RzRandom::new(1, &SEEDS);
        assert_eq!(r.next_raw(), 0x0000_41C6);
        assert_eq!(r.lcg_state(), 0x41C6_7EA6);
        assert_eq!(r.next_raw(), 0x59D4_967E);
        assert_eq!(r.lcg_state(), 0x967E_B0E7);
        assert_eq!(r.next_raw(), 0xBF02_2781);
        assert_eq!(r.lcg_state(), 0x2781_E494);
        let before = r.lcg_state();
        assert_eq!(r.uniform_u32(0), 0);
        assert_eq!(r.lcg_state(), before);
    }

    #[test]
    fn rz_random_double_vectors() {
        let mut r = RzRandom::new(1, &SEEDS);
        let expect = [
            (0.654_508_496_867_492_8, 0x278D_DE6D),
            (0.481_738_711_008_802_06, 0xFB53_3A69),
            (0.851_256_174_268_201, 0x59EB_ECB5),
        ];
        for (v, state) in expect {
            assert_eq!(r.uniform_f64(), v);
            assert_eq!(r.mul_state(), state);
        }
    }

    #[test]
    fn rz_random_clock_seeding() {
        let r = RzRandom::new(CLOCK_SEED, &SEEDS);
        assert_eq!((r.lcg_state(), r.mul_state()), (12345, 12345));
        let mut z = RzRandom::new(0, &SEEDS);
        let _ = z.uniform_f64(); // mul 0 falls back to the process clock
        assert_eq!(z.mul_state(), 777u32.wrapping_mul(0x278D_DE6D));
    }

    #[test]
    fn rz_random_super_vectors() {
        let mut r = RzRandomSuper::new(1, &SEEDS);
        let mut table = RzRandomSuper {
            state: [0; MT_N],
            index: MT_N,
        };
        // Initial table before the twist.
        let mut x = 1u32;
        for i in 0..2 {
            table.state[i] = (x & 0xFFFF_0000) | (x.wrapping_mul(69069) >> 16);
            x = x.wrapping_mul(0x1C58_7629).wrapping_add(0x10DCE);
        }
        assert_eq!(table.state[0], 0x0000_0001);
        assert_eq!(table.state[1], 0x1C59_C359);
        assert_eq!(r.next_u32(), 0xD3F6_B9E5);
        assert_eq!(r.next_u32(), 0xEF28_4157);
        assert_eq!(r.next_u32(), 0x0D36_39C7);
        let mut r = RzRandomSuper::new(4357, &SEEDS);
        assert_eq!(r.next_u32(), 0xAAE6_4AC3);
        assert_eq!(r.next_u32(), 0x5E7C_47DA);
        assert_eq!(r.next_u32(), 0xB98E_8121);
    }

    #[test]
    fn weighted_choice() {
        let mut r = RzRandomSuper::new(99, &SEEDS);
        for _ in 0..100 {
            assert_eq!(r.weighted_choice(&[0.0, 1.0, 0.0]), 1);
            assert!(r.weighted_choice(&[0.0, -1.0, 0.0]) < 3);
        }
        assert_eq!(r.weighted_choice(&[5.0]), 0);
    }
}
