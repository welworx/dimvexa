// SPDX-License-Identifier: MIT OR Apache-2.0
//! Distance between floating-point numbers in units in the last place.

/// Maps an `f64` to an integer that increases with the value (`-0.0` and `+0.0` coincide).
#[must_use]
const fn ordered(x: f64) -> i64 {
    let bits = i64::from_ne_bytes(x.to_bits().to_ne_bytes());
    if bits < 0 {
        i64::MIN.wrapping_sub(bits)
    } else {
        bits
    }
}

/// Number of representable `f64` values between `a` and `b` (0 when equal).
///
/// Returns `None` if either value is NaN. Infinities are treated as the values next to
/// `f64::MAX`.
#[must_use]
pub const fn ulp_distance(a: f64, b: f64) -> Option<u64> {
    if a.is_nan() || b.is_nan() {
        return None;
    }
    Some(ordered(a).abs_diff(ordered(b)))
}

/// The spacing of `f64` values at the magnitude of `x` (the ulp of `x`).
#[must_use]
pub fn ulp_of(x: f64) -> f64 {
    let a = x.abs();
    if !a.is_finite() {
        return f64::INFINITY;
    }
    let next = f64::from_bits(a.to_bits().saturating_add(1));
    next - a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances() {
        assert_eq!(ulp_distance(1.0, 1.0), Some(0));
        assert_eq!(ulp_distance(0.0, -0.0), Some(0));
        assert_eq!(
            ulp_distance(1.0, f64::from_bits(1.0_f64.to_bits() + 1)),
            Some(1)
        );
        assert_eq!(
            ulp_distance(-1.0, f64::from_bits((-1.0_f64).to_bits() + 1)),
            Some(1)
        );
        assert_eq!(ulp_distance(f64::NAN, 1.0), None);
        assert_eq!(ulp_distance(f64::MAX, f64::INFINITY), Some(1));
        assert!(ulp_distance(-f64::MIN_POSITIVE * 0.5, f64::MIN_POSITIVE * 0.5).is_some());
        assert!((ulp_of(1.0) - f64::EPSILON).abs() < f64::MIN_POSITIVE);
    }
}
