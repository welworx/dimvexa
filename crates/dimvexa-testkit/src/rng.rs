// SPDX-License-Identifier: MIT OR Apache-2.0
//! Deterministic pseudo-random numbers (`SplitMix64`).

/// `SplitMix64`: tiny, fast, statistically decent, fully reproducible from a seed.
///
/// Not for cryptography. Tests print their seed so a failure can be replayed.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A generator with the given seed.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Next 64 random bits.
    pub const fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A value in `0..n` (`n` must be positive; returns 0 for `n == 0`).
    pub const fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }

    /// A signed value in `lo..=hi`.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        let span = hi.saturating_sub(lo).unsigned_abs().saturating_add(1);
        lo.saturating_add(i64::try_from(self.below(span)).unwrap_or(0))
    }

    /// A random boolean.
    pub const fn flip(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Picks an element of a non-empty slice.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        let n = u64::try_from(items.len()).unwrap_or(0);
        items.get(usize::try_from(self.below(n)).unwrap_or(0))
    }
}
