// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hostile input never panics and never hangs (rule 1 and rule 4 of `AGENTS.md`, `panic-free`
//! skill). Every public entry point that takes text or a float gets hand-written nasty cases and
//! seeded random junk; the result may be an error, and its code must be a stable one
//! (`Error::ALL_CODES`), but never a panic.
//!
//! A panic fixed in the field gets a case here (add it to `NASTY`), plus a test next to the code.
//! Each case is printed before it runs (the harness shows it when a test fails), with the seed;
//! replay with `DIMVEXA_PROP_SEED=<seed>`. The tests also count what they reached, so a change
//! that makes the generators produce only rejected input fails instead of passing quietly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dimvexa::registry;
use dimvexa_config::{Config, apply};
use dimvexa_core::{Error, Ratio};

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

/// Real units, so that hostile text is paired with something a conversion can be built from.
const VALID: &[&str] = &[
    "m", "ft", "km/h", "m/s", "kg", "lb", "degC", "degF", "K", "Pa", "bar", "mol", "rad", "deg",
];

/// Numbers that stress rounding, overflow and the exact path.
const HOSTILE_FLOATS: &[f64] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    f64::MAX,
    f64::MIN,
    f64::MIN_POSITIVE,
    5e-324,
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
];

fn assert_stable(e: &Error, what: &str) {
    assert!(
        Error::ALL_CODES.contains(&e.code()),
        "{what}: code {:?} is not in Error::ALL_CODES",
        e.code()
    );
}

#[test]
fn parsing_unit_expressions_never_panics() {
    let reg = registry().unwrap();
    let mut parsed = 0_usize;
    eprintln!("seed {}", seed());
    for (i, text) in inputs().iter().enumerate() {
        eprintln!("case {i}: {text:?}");
        match reg.parse(text) {
            Ok(e) => {
                parsed += 1;
                // What parses must print and parse again to the same thing.
                let printed = e.to_string();
                let again = reg.parse(&printed).map(|x| x.to_string());
                assert_eq!(
                    again.as_deref(),
                    Ok(printed.as_str()),
                    "case {i} ({text:?})"
                );
            }
            Err(e) => assert_stable(&e, text),
        }
    }
    assert!(
        parsed >= 20,
        "only {parsed} hostile inputs parsed; the generator went quiet"
    );
}

#[test]
fn building_and_applying_conversions_never_panics() {
    let reg = registry().unwrap();
    let mut built = 0_usize;
    let apply = |c: &dimvexa_core::Conversion| {
        for x in HOSTILE_FLOATS {
            let _ = c.apply_exact_f64(*x);
        }
        let _ = c.apply_exact(Ratio::new(i128::MAX, 1).unwrap());
        let _ = c.apply_exact(Ratio::new(i128::MIN + 1, i128::MAX).unwrap());
    };
    eprintln!("seed {}", seed());
    // Hostile text on one side, a real unit on the other (both orders), and real pairs.
    for text in inputs() {
        eprintln!("case: {text:?}");
        for v in VALID {
            for (from, to) in [(text.as_str(), *v), (*v, text.as_str())] {
                match reg.conversion(from, to) {
                    Ok(c) => {
                        built += 1;
                        apply(&c);
                    }
                    Err(e) => assert_stable(&e, &format!("{from:?} -> {to:?}")),
                }
            }
        }
    }
    for from in VALID {
        for to in VALID {
            if let Ok(c) = reg.conversion(from, to) {
                built += 1;
                apply(&c);
            }
        }
    }
    assert!(
        built >= 100,
        "only {built} conversions were built; the test went quiet"
    );
}

#[test]
fn parsing_exact_numbers_never_panics() {
    eprintln!("seed {}", seed());
    for text in inputs() {
        eprintln!("case: {text:?}");
        let _ = Ratio::parse_symbolic(&text);
        let _ = text.parse::<Ratio>();
        let _ = Ratio::parse_prefix(&text);
    }
}

/// A TOML string value: only `\"` and `\\` are escapes in a config file, and a control character
/// is replaced so the line structure survives and the value reaches the unit validation.
fn quoted(value: &str) -> String {
    let clean: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    format!("\"{}\"", clean.replace('\\', "\\\\").replace('"', "\\\""))
}

#[test]
fn parsing_and_applying_config_files_never_panics() {
    let reg = registry().unwrap();
    let (mut parsed, mut applied) = (0_usize, 0_usize);
    eprintln!("seed {}", seed());
    for text in inputs() {
        eprintln!("case: {text:?}");
        // The raw text as a file, and the text as the value of each key of a complete entry.
        let mut files = vec![text.clone()];
        let fields = [
            "symbol",
            "base",
            "factor",
            "exactness",
            "source",
            "prefixes",
        ];
        for hostile in fields {
            let mut body = String::from("[[unit]]\n");
            for key in fields {
                let value = if key == hostile {
                    quoted(&text)
                } else {
                    match key {
                        "symbol" => "\"hx\"".to_string(),
                        "base" => "\"m\"".to_string(),
                        "factor" => "\"3\"".to_string(),
                        "exactness" => "\"defined\"".to_string(),
                        "source" => "\"s\"".to_string(),
                        _ => "\"si\"".to_string(),
                    }
                };
                body += &[key, " = ", &value, "\n"].concat();
            }
            files.push(body);
        }
        for file in files {
            if let Ok(cfg) = Config::parse(&file, "hostile.toml") {
                parsed += 1;
                let mut r = reg.clone();
                match apply(&mut r, &[cfg]) {
                    Ok(_) => applied += 1,
                    Err(e) => assert_stable(&e, &file),
                }
            }
        }
    }
    assert!(
        parsed >= 1000,
        "only {parsed} hostile configs parsed; the test went quiet"
    );
    assert!(
        applied >= 100,
        "only {applied} hostile configs applied; the test went quiet"
    );
}
