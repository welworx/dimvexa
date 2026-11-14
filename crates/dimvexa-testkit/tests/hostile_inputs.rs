// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hostile input never panics and never hangs (rule 1 and rule 4 of `AGENTS.md`, `panic-free`
//! skill). Every public entry point that takes text or a float gets hand-written nasty cases and
//! seeded random junk; the result may be an error with a stable code, but never a panic.
//!
//! A panic fixed in the field gets a case here (add it to `NASTY`), plus a test next to the code.
//! Replay a failure with `DIMVEXA_PROP_SEED=<seed>`; the seed is printed on failure.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dimvexa::registry;
use dimvexa_config::{Config, apply};
use dimvexa_core::Ratio;

/// Cases that have broken, or could break, a parser: huge exponents, deep nesting, odd Unicode,
/// control characters, empty and near-empty input, numbers at the edge of `i128`.
const NASTY: &[&str] = &[
    "",
    " ",
    "\0",
    "\u{feff}m",
    "m^",
    "m^^2",
    "^2",
    "/",
    "//",
    "m/",
    "/m",
    ".",
    "..",
    "m..s",
    "m.",
    ".m",
    "(",
    ")",
    "()",
    "((((((((((((((((((((((((((((((((((((((((m))))))))))))))))))))))))))))))))))))))))",
    "m^999999999999999999999999999999999999999",
    "m^-999999999999999999999999999999999999999",
    "m^170141183460469231731687303715884105727",
    "m^-170141183460469231731687303715884105728",
    "km^9999999999",
    "10^999999999999999999",
    "1e99999999999999999999",
    "170141183460469231731687303715884105728",
    "-170141183460469231731687303715884105728/-1",
    "1/0",
    "0/0",
    "-0",
    "NaN",
    "inf",
    "-inf",
    "m s kg A K mol cd rad sr Hz N Pa J W C V F Ohm S Wb T H lm lx Bq Gy Sv kat",
    "\u{202e}m\u{202c}",
    "m\u{0301}",
    "Å Ω µ ° ′ ″ ² ³ ⁻¹",
    "ｍ",
    "m\n\n\ns",
    "m\tm\tm",
    "ton",
    "gal",
    "4pi",
    "pi/2",
    "pi^999999999999",
    "1/pi^999999999999",
    "°C",
    "degC^2",
    "delta_degC",
];

/// A tiny seeded generator (xorshift64*), so a failure replays from its seed.
struct Rng(u64);

impl Rng {
    const fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n.max(1) as u64).unwrap_or(0)
    }
}

fn seed() -> u64 {
    std::env::var("DIMVEXA_PROP_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5eed_1234_abcd_ef01)
}

/// Random junk assembled from tokens a parser reacts to, and from arbitrary characters.
fn junk(rng: &mut Rng) -> String {
    const PIECES: &[&str] = &[
        "m",
        "km",
        "s",
        "kg",
        "degC",
        "degF",
        "K",
        "ft",
        "in",
        "lb",
        "mol",
        "Pa",
        "bar",
        "^",
        "-",
        "+",
        "/",
        ".",
        "*",
        "(",
        ")",
        " ",
        "2",
        "3",
        "0",
        "10",
        "99999999999999999999",
        "e",
        "pi",
        "1/2",
        "\u{00b0}",
        "\u{00b5}",
        "\u{03a9}",
        "\0",
        "\n",
        "=",
        "#",
        "\"",
        "[",
        "]",
    ];
    let mut out = String::new();
    for _ in 0..rng.below(24) {
        if rng.below(8) == 0 {
            let c = char::from_u32(u32::try_from(rng.next() % 0x11_0000).unwrap_or(0));
            out.push(c.unwrap_or('\u{fffd}'));
        } else if let Some(p) = PIECES.get(rng.below(PIECES.len())) {
            out.push_str(p);
        }
    }
    out
}

fn inputs() -> Vec<String> {
    let mut rng = Rng(seed());
    let mut all: Vec<String> = NASTY.iter().map(|s| (*s).to_string()).collect();
    all.extend((0..4000).map(|_| junk(&mut rng)));
    all
}

#[test]
fn parsing_unit_expressions_never_panics() {
    let reg = registry().unwrap();
    for (i, text) in inputs().iter().enumerate() {
        // An error is fine; a panic fails the test, and `i` plus the seed replay it.
        let result = reg.parse(text);
        if let Ok(e) = result {
            // What parses must print and parse again to the same thing.
            let printed = e.to_string();
            let again = reg.parse(&printed).map(|x| x.to_string());
            assert_eq!(
                again.as_deref(),
                Ok(printed.as_str()),
                "case {i} ({text:?}), seed {}",
                seed()
            );
        }
    }
}

#[test]
fn building_conversions_never_panics() {
    let reg = registry().unwrap();
    let inputs = inputs();
    for pair in inputs.windows(2) {
        let (Some(from), Some(to)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        if let Ok(c) = reg.conversion(from, to) {
            // A built conversion must also apply to hostile numbers without panicking.
            for x in [
                0.0,
                -0.0,
                1.0,
                -1.0,
                f64::MAX,
                f64::MIN,
                f64::MIN_POSITIVE,
                5e-324,
            ] {
                let _ = c.apply_exact_f64(x);
            }
            let _ = c.apply_exact(Ratio::new(i128::MAX, 1).unwrap());
            let _ = c.apply_exact(Ratio::new(i128::MIN + 1, i128::MAX).unwrap());
        }
    }
}

#[test]
fn applying_non_finite_floats_never_panics() {
    let reg = registry().unwrap();
    for (from, to) in [("m", "ft"), ("degC", "degF"), ("K", "degC"), ("kg", "lb")] {
        let c = reg.conversion(from, to).unwrap();
        for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let _ = c.apply_exact_f64(x);
        }
    }
}

#[test]
fn parsing_exact_numbers_never_panics() {
    for text in inputs() {
        let _ = Ratio::parse_symbolic(&text);
        let _ = text.parse::<Ratio>();
        let _ = Ratio::parse_prefix(&text);
    }
}

#[test]
fn parsing_and_applying_config_files_never_panics() {
    let reg = registry().unwrap();
    for text in inputs() {
        // The whole text as a file, and as the value of each key a config understands.
        let mut texts = vec![text.clone()];
        for key in [
            "symbol",
            "base",
            "factor",
            "name",
            "source",
            "exactness",
            "prefixes",
        ] {
            texts.push(format!("[[unit]]\n{key} = \"{}\"\n", text.escape_default()));
        }
        texts.push(format!(
            "[[unit]]\nsymbol = \"hx\"\nbase = \"{}\"\nfactor = \"{}\"\nexactness = \"defined\"\nsource = \"s\"\n",
            text.escape_default(),
            text.escape_default()
        ));
        for t in texts {
            if let Ok(cfg) = Config::parse(&t, "hostile.toml") {
                let mut r = reg.clone();
                let _ = apply(&mut r, &[cfg]);
            }
        }
    }
}
