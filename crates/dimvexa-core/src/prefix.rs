// SPDX-License-Identifier: MIT OR Apache-2.0
//! SI and binary prefixes.

use crate::error::Result;
use crate::ratio::Ratio;

/// Which prefixes a unit accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum PrefixMode {
    /// No prefixes (for example `min`, `psi`, `degC`).
    #[default]
    None,
    /// The 24 SI prefixes from quecto to quetta.
    Si,
    /// SI and binary prefixes (`Ki`, `Mi`, ...), for information units.
    SiAndBinary,
}

impl PrefixMode {
    /// The stable lower-case name used in data files.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Si => "si",
            Self::SiAndBinary => "si_binary",
        }
    }

    /// Reads a name produced by [`PrefixMode::name`].
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "none" => Some(Self::None),
            "si" => Some(Self::Si),
            "si_binary" => Some(Self::SiAndBinary),
            _ => None,
        }
    }

    /// True if a prefix of the given family is accepted.
    #[must_use]
    pub const fn accepts(self, family: PrefixFamily) -> bool {
        matches!(
            (self, family),
            (Self::Si | Self::SiAndBinary, PrefixFamily::Si)
                | (Self::SiAndBinary, PrefixFamily::Binary)
        )
    }
}

/// The family a prefix belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PrefixFamily {
    /// Decimal SI prefixes, factor `10^exp`.
    Si,
    /// Binary prefixes, factor `2^exp`.
    Binary,
}

/// A prefix such as `k` (kilo) or `Mi` (mebi).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prefix {
    /// The symbol as written in unit expressions.
    pub symbol: &'static str,
    /// The prefix name.
    pub name: &'static str,
    /// Decimal or binary family.
    pub family: PrefixFamily,
    /// Exponent of 10 (SI) or of 2 (binary).
    pub exp: i32,
}

impl Prefix {
    /// The exact multiplication factor.
    ///
    /// # Errors
    ///
    /// [`crate::Error::Overflow`] cannot occur for the built-in tables (10^30 and 2^80 fit
    /// in `i128`) but is reported rather than assumed.
    pub fn factor(self) -> Result<Ratio> {
        let base = match self.family {
            PrefixFamily::Si => 10,
            PrefixFamily::Binary => 2,
        };
        Ratio::from_int(base).checked_pow(self.exp)
    }
}

const fn si(symbol: &'static str, name: &'static str, exp: i32) -> Prefix {
    Prefix {
        symbol,
        name,
        family: PrefixFamily::Si,
        exp,
    }
}

const fn bin(symbol: &'static str, name: &'static str, exp: i32) -> Prefix {
    Prefix {
        symbol,
        name,
        family: PrefixFamily::Binary,
        exp,
    }
}

/// All 24 SI prefixes (ronna and quetta, ronto and quecto included, as adopted in 2022),
/// plus the ASCII `u` and the Greek-mu spellings `\u{b5}` and `\u{3bc}` of micro.
pub const SI_PREFIXES: [Prefix; 26] = [
    si("Q", "quetta", 30),
    si("R", "ronna", 27),
    si("Y", "yotta", 24),
    si("Z", "zetta", 21),
    si("E", "exa", 18),
    si("P", "peta", 15),
    si("T", "tera", 12),
    si("G", "giga", 9),
    si("M", "mega", 6),
    si("k", "kilo", 3),
    si("h", "hecto", 2),
    si("da", "deca", 1),
    si("d", "deci", -1),
    si("c", "centi", -2),
    si("m", "milli", -3),
    si("\u{b5}", "micro", -6),
    si("\u{3bc}", "micro", -6),
    si("u", "micro", -6),
    si("n", "nano", -9),
    si("p", "pico", -12),
    si("f", "femto", -15),
    si("a", "atto", -18),
    si("z", "zepto", -21),
    si("y", "yocto", -24),
    si("r", "ronto", -27),
    si("q", "quecto", -30),
];

/// The eight binary prefixes kibi to yobi.
pub const BINARY_PREFIXES: [Prefix; 8] = [
    bin("Ki", "kibi", 10),
    bin("Mi", "mebi", 20),
    bin("Gi", "gibi", 30),
    bin("Ti", "tebi", 40),
    bin("Pi", "pebi", 50),
    bin("Ei", "exbi", 60),
    bin("Zi", "zebi", 70),
    bin("Yi", "yobi", 80),
];

/// Iterates over every known prefix (SI first, then binary).
pub fn all_prefixes() -> impl Iterator<Item = Prefix> {
    SI_PREFIXES.iter().chain(BINARY_PREFIXES.iter()).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn there_are_24_distinct_si_prefixes() {
        let exps: BTreeSet<i32> = SI_PREFIXES.iter().map(|p| p.exp).collect();
        // 24 prefixes: 10^-30 .. 10^30 (the 24 named steps, no 10^0).
        assert_eq!(exps.len(), 24);
        let names: BTreeSet<&str> = SI_PREFIXES.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), 24);
        assert!(names.contains("ronna") && names.contains("quetta"));
        assert!(names.contains("ronto") && names.contains("quecto"));
    }

    #[test]
    fn factors_are_exact() {
        let q = SI_PREFIXES[0];
        assert_eq!(q.factor().unwrap(), Ratio::from_int(10_i128.pow(30)));
        let quecto = SI_PREFIXES[25];
        assert_eq!(
            quecto.factor().unwrap(),
            Ratio::new(1, 10_i128.pow(30)).unwrap()
        );
        let yobi = BINARY_PREFIXES[7];
        assert_eq!(yobi.factor().unwrap(), Ratio::from_int(1_i128 << 80));
    }

    #[test]
    fn mode_names() {
        for m in [PrefixMode::None, PrefixMode::Si, PrefixMode::SiAndBinary] {
            assert_eq!(PrefixMode::from_name(m.name()), Some(m));
        }
        assert!(PrefixMode::Si.accepts(PrefixFamily::Si));
        assert!(!PrefixMode::Si.accepts(PrefixFamily::Binary));
        assert!(!PrefixMode::None.accepts(PrefixFamily::Si));
        assert!(PrefixMode::SiAndBinary.accepts(PrefixFamily::Binary));
    }
}
