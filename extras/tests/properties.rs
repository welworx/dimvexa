//! Proptest properties (unverified in M0; mirrors the std-only suite in dimvexa-testkit).
use dimvexa_data::registry;
use proptest::prelude::*;

proptest! {
    #[test]
    fn print_parse_roundtrip(a in 0usize..40, b in 0usize..40) {
        let reg = registry().unwrap();
        let syms = ["m", "s", "kg", "N", "Pa", "J", "W", "L", "bar", "psi"];
        let text = format!("{}.{}/{}", syms[a % 10], syms[b % 10], syms[(a + b) % 10]);
        if let Ok(e) = reg.parse(&text) {
            let printed = e.to_string();
            prop_assert_eq!(reg.parse(&printed).unwrap().to_string(), printed);
        }
    }

    #[test]
    fn linear_roundtrip_within_tolerance(x in -1e9f64..1e9) {
        let reg = registry().unwrap();
        let there = reg.conversion("bar", "psi").unwrap();
        let back = reg.conversion("psi", "bar").unwrap();
        let y = back.apply(there.apply(x));
        prop_assert!((y - x).abs() <= x.abs() * 4.0 * f64::EPSILON);
    }

    #[test]
    fn parser_never_panics(s in ".{0,64}") {
        let reg = registry().unwrap();
        let _ = reg.parse(&s);
    }
}
