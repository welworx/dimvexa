// SPDX-License-Identifier: MIT OR Apache-2.0
//! 256-bit unsigned helpers for exact comparison without intermediate overflow.
//!
//! A value is a pair `(high, low)` of `u128`, so tuple ordering is numeric ordering.
// Private module: `pub` here is crate-visible, and the two visibility lints disagree about it.
#![allow(unreachable_pub, clippy::redundant_pub_crate)]

/// An unsigned 256-bit number as `(high, low)`.
pub(crate) type Wide = (u128, u128);

/// Full 256-bit product of two `u128`, as `(high, low)`.
pub(crate) fn mul_wide(a: u128, b: u128) -> Option<Wide> {
    let mask = u128::from(u64::MAX);
    let (a1, a0) = (a >> 64, a & mask);
    let (b1, b0) = (b >> 64, b & mask);
    let p00 = a0.checked_mul(b0)?;
    let p01 = a0.checked_mul(b1)?;
    let p10 = a1.checked_mul(b0)?;
    let p11 = a1.checked_mul(b1)?;
    let mid = (p00 >> 64)
        .checked_add(p01 & mask)?
        .checked_add(p10 & mask)?;
    let lo = ((mid & mask) << 64) | (p00 & mask);
    let hi = p11
        .checked_add(p01 >> 64)?
        .checked_add(p10 >> 64)?
        .checked_add(mid >> 64)?;
    Some((hi, lo))
}

/// `w * 10`, or `None` if the result needs more than 256 bits.
pub(crate) fn mul10(w: Wide) -> Option<Wide> {
    let (carry, lo) = mul_wide(w.1, 10)?;
    let (over, hi) = mul_wide(w.0, 10)?;
    if over != 0 {
        return None;
    }
    Some((hi.checked_add(carry)?, lo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul10_carries_between_the_halves() {
        assert_eq!(mul10((0, 7)), Some((0, 70)));
        assert_eq!(mul10((0, u128::MAX)), Some((9, u128::MAX - 9)));
        assert_eq!(mul10((1, 0)), Some((10, 0)));
        assert_eq!(mul10((u128::MAX, 0)), None);
        assert_eq!(mul10((u128::MAX / 10 + 1, 0)), None);
        assert_eq!(mul10((u128::MAX / 10, 0)), Some((u128::MAX / 10 * 10, 0)));
    }

    #[test]
    fn mul_wide_matches_u128_when_small() {
        assert_eq!(mul_wide(1 << 100, 1 << 20), Some((0, 1 << 120)));
        assert_eq!(mul_wide(1 << 100, 1 << 40), Some((1 << 12, 0)));
        assert_eq!(mul_wide(u128::MAX, u128::MAX), Some((u128::MAX - 1, 1)));
    }
}
