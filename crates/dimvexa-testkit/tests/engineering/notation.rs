// SPDX-License-Identifier: MIT OR Apache-2.0
//! How quantities are written: unit notation and dimensionless results (B2, C).

use crate::common::*;

#[test]
#[ignore = "open B2: pow(5 m, 0) is `1 m^0`, not dimensionless"]
fn open_b2_power_zero_is_dimensionless() {
    let reg = registry().unwrap();
    let one = Calc::new(&reg).pow(&q(&reg, "5 m"), 0).unwrap();
    assert_eq!(one.to_string(), "1");
}

#[test]
#[ignore = "open C: superscripts and a space as multiplication are parse errors"]
fn open_c_common_notation_is_read() {
    let reg = registry().unwrap();
    let bad: Vec<&str> = ["1 m³", "1 ft/s²", "1 N m", "1 m s^-1"]
        .into_iter()
        .filter(|s| reg.quantity(s).is_err())
        .collect();
    assert!(bad.is_empty(), "not read: {bad:?}");
}

/// A well-known function that is not implemented is a `refused` error that says so, with the
/// way to write it where there is one; it is never a wrong number or a syntax error.
#[test]
fn functions_that_are_not_implemented_say_so() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("x", "0.5")
        .unwrap()
        .var("y", "2")
        .unwrap();
    for name in [
        "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "asinh", "acosh", "atanh", "log",
        "log10", "log2", "cbrt", "floor", "round",
    ] {
        let err = f.eval(&format!("{name}(x)")).unwrap_err();
        assert_eq!(err.code(), "refused", "{name}");
        let msg = err.to_string();
        assert!(
            msg.contains(&format!("'{name}' is not implemented")),
            "{msg}"
        );
        assert!(err.suggestions().contains(&"sqrt".to_owned()), "{name}");
    }
    // more than one argument: the name decides, not the comma
    for call in [
        "atan2(x, y)",
        "hypot(x, y)",
        "pow(x, y)",
        "log(x, y)",
        "tan(",
    ] {
        let err = f.eval(call).unwrap_err();
        assert_eq!(err.code(), "refused", "{call}");
        assert!(
            err.to_string().contains("is not implemented"),
            "{call}: {err}"
        );
    }
}

#[test]
fn a_function_that_is_not_implemented_says_how_to_write_it() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg);
    let hint = |call: &str| f.eval(call).unwrap_err().to_string();
    assert!(hint("tan(0.5 rad)").contains("write sin(x) / cos(x)"));
    assert!(hint("log10(100)").contains("write ln(x) / ln(10)"));
    assert!(hint("cbrt(8)").contains("write x^(1/3)"));
    assert!(hint("hypot(3, 4)").contains("write sqrt(x^2 + y^2)"));
    // no way to write it from the implemented ones: no hint
    assert!(!hint("atan(1)").contains("write"));
}

#[test]
fn the_ways_to_write_the_missing_functions_give_the_right_numbers() {
    check(&[("a", "45 deg")], "sin(a) / cos(a)", "1", 1.0);
    check(&[("x", "1000")], "ln(x) / ln(10)", "1", 3.0);
    check(&[("x", "27")], "x^(1/3)", "1", 3.0);
    check(&[("x", "3"), ("y", "4")], "sqrt(x^2 + y^2)", "1", 5.0);
    check(&[("x", "1")], "(exp(x) - exp(-x)) / 2", "1", 1.0_f64.sinh());
}

#[test]
fn an_unknown_function_name_is_refused_as_unknown() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg).var("x", "1").unwrap();
    let err = f.eval("frobnicate(x)").unwrap_err();
    assert_eq!(err.code(), "refused");
    assert!(
        err.to_string().contains("unknown function 'frobnicate'"),
        "{err}"
    );
    assert!(!err.to_string().contains("is not implemented"), "{err}");
}
