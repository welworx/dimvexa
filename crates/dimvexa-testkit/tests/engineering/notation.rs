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
    for name in ["floor", "round"] {
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
    for call in ["pow(x, y)", "floor("] {
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
    assert!(hint("pow(2, 3)").contains("write x^p"));
    // no way to write it from the implemented ones: no hint
    assert!(!hint("floor(1.5)").contains("write"));
}

#[test]
fn the_way_to_write_tan_gives_the_right_number() {
    check(&[("a", "45 deg")], "sin(a) / cos(a)", "1", 1.0);
    check(
        &[("a", "30 deg")],
        "sin(a) / cos(a)",
        "1",
        30.0_f64.to_radians().tan(),
    );
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

/// `log(x)` alone could be natural or base 10: not guessed (ADR-006); the base is a second
/// argument.
#[test]
fn log_of_one_argument_is_ambiguous_and_refused() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg).var("x", "100").unwrap();
    let err = f.eval("log(x)").unwrap_err();
    assert_eq!(err.code(), "refused");
    let msg = err.to_string();
    for name in [
        "ln(x)",
        "lg(x)",
        "log10(x)",
        "lb(x)",
        "log2(x)",
        "log(x, base)",
    ] {
        assert!(msg.contains(name), "{name} missing in {msg}");
    }
    assert!(msg.contains("ambiguous"), "{msg}");
    for name in ["ln", "lg", "log10", "lb", "log2"] {
        assert!(err.suggestions().contains(&name.to_owned()), "{name}");
    }
    assert_eq!(f.eval_exact("log(x, 10)").unwrap().to_string(), "2 1");
}

#[test]
fn a_function_with_the_wrong_number_of_arguments_is_a_syntax_error() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg).var("x", "4").unwrap();
    for call in [
        "sqrt(x, x)",
        "hypot(x)",
        "min(x)",
        "hypot(x, x, x)",
        "log(x, x, x)",
        "ln()",
    ] {
        let err = f.eval(call).unwrap_err();
        assert_eq!(err.code(), "parse_error", "{call}: {err}");
    }
}

/// `min` and `max` take two or more arguments.
#[test]
fn min_and_max_take_any_number_of_arguments() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("a", "1 km")
        .unwrap()
        .var("b", "900 m")
        .unwrap()
        .var("c", "0.5 mi")
        .unwrap();
    assert_eq!(f.eval_exact("min(a, b, c)").unwrap().to_string(), "0.5 mi");
    assert_eq!(f.eval_exact("max(a, b, c)").unwrap().to_string(), "1 km");
    assert_eq!(f.eval_exact("max(b, a)").unwrap().to_string(), "1 km");
}

/// ISO 80000-2 names: `lg` is the base 10 logarithm and `lb` the base 2 one.
#[test]
fn lg_and_lb_are_log10_and_log2() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("a", "1000")
        .unwrap()
        .var("b", "8")
        .unwrap()
        .var("c", "3")
        .unwrap();
    assert_eq!(f.eval_exact("lg(a)").unwrap().to_string(), "3 1");
    assert_eq!(f.eval_exact("lb(b)").unwrap().to_string(), "3 1");
    assert_eq!(
        f.eval("lg(c)").unwrap().to_string(),
        f.eval("log10(c)").unwrap().to_string()
    );
    assert_eq!(
        f.eval("lb(c)").unwrap().to_string(),
        f.eval("log2(c)").unwrap().to_string()
    );
}

/// The absolute value and the negation of a temperature point mean nothing: `-5 degC` and
/// `5 degC` are different states. A temperature difference is fine, and so is `abs(T1 - T2)`.
#[test]
fn the_absolute_value_of_a_temperature_point_is_refused_and_names_the_delta_unit() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("T", "-5 degC")
        .unwrap()
        .var("T1", "20 degC")
        .unwrap()
        .var("T2", "80 degC")
        .unwrap()
        .var("d", "-5 delta_degC")
        .unwrap();
    for call in ["abs(T)", "-T"] {
        let err = f.eval(call).unwrap_err();
        assert_eq!(err.code(), "kind_mismatch", "{call}");
        assert_eq!(err.suggestions(), ["delta_degC".to_owned()], "{call}");
        assert!(
            err.to_string().contains("for a difference use delta_degC"),
            "{err}"
        );
    }
    assert_eq!(f.eval_exact("abs(d)").unwrap().to_string(), "5 delta_degC");
    assert_eq!(
        f.eval_exact("abs(T1 - T2)").unwrap().to_string(),
        "60 delta_degC"
    );
}

/// Review F2: an exponent with a far power of ten used to be read from the digits alone, so
/// `x^1e-50` was `x^1` and `x^2e40` was `x^2`.
#[test]
fn an_exponent_that_does_not_fit_is_refused_not_truncated() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg).var("x", "2").unwrap();
    for call in ["x^1e-50", "x^1e39", "x^2e40", "x^1e100"] {
        let err = f.eval(call).unwrap_err();
        assert_eq!(err.code(), "parse_error", "{call}: {err}");
        assert!(
            err.to_string().contains("too large or too precise"),
            "{call}: {err}"
        );
    }
    // exponents that fit: a decimal is a fraction (sqrt(2) is approximate), a power of ten
    // that folds is a whole number
    let half = f.eval("x^0.5").unwrap();
    assert!(!half.is_exact());
    assert_eq!(f.f64_in(&half, "1").unwrap(), 2.0_f64.sqrt());
    assert!(f.eval_exact("x^0.5").is_err());
    assert_eq!(f.f64_in(&f.eval("x^2").unwrap(), "1").unwrap(), 4.0);
    assert_eq!(f.f64_in(&f.eval("x^2e0").unwrap(), "1").unwrap(), 4.0);
    assert_eq!(f.f64_in(&f.eval("x^1e1").unwrap(), "1").unwrap(), 1024.0);
}

/// Superscripts, dots and crosses, and any whitespace are read in an equation.
#[test]
fn superscripts_dots_and_whitespace_are_read_in_equations() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg);
    for (text, want) in [
        ("1 m³", "1 m^3"),
        ("9.81 m/s²", "9.81 m/s^2"),
        ("2 N·m", "2 N.m"),
        ("3 m × 4 m", "12 m.m"),
        ("6 m ÷ 2", "3 m"),
        ("2 m\t*\n3 m", "6 m.m"),
        ("−2 m", "-2 m"),
        ("1 s⁻¹", "1 s^-1"),
    ] {
        assert_eq!(f.eval(text).unwrap().to_string(), want, "{text}");
    }
}

/// `2 pi r` and `pi r^2` multiply; a name after anything else is not guessed.
#[test]
fn pi_after_a_number_and_before_a_name_multiplies() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg).var("r", "3 m").unwrap();
    let pi = std::f64::consts::PI;
    assert_eq!(value(&reg, &[("r", "3 m")], "2 pi r", "m"), 6.0 * pi);
    assert_eq!(value(&reg, &[("r", "3 m")], "pi r^2", "m2"), 9.0 * pi);
    assert_eq!(value(&reg, &[], "2 pi", "1"), 2.0 * pi);
    // `2 r` stays refused: the reader sees a number and a unit, the evaluator a name
    assert_eq!(f.eval("2 r").unwrap_err().code(), "invalid_definition");
}

/// An exponent may be a name or an expression, if it is an exact plain number.
#[test]
fn exponents_may_be_names_and_expressions() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("x", "9 m2")
        .unwrap()
        .var("n", "0.5")
        .unwrap()
        .var("k", "2")
        .unwrap();
    for (text, unit, want) in [
        ("2^3^2", "1", 512.0),
        ("2^(1+1)", "1", 4.0),
        ("x^n", "m", 3.0),
        ("x^(1/k)", "m", 3.0),
        ("2^-k", "1", 0.25),
        ("2^(sqrt(4))", "1", 4.0),
    ] {
        assert_eq!(
            f.f64_in(&f.eval(text).unwrap(), unit).unwrap(),
            want,
            "{text}"
        );
    }
    for text in ["2^sqrt(2)", "x^(1 m)", "2^(pi)", "2^99999999999"] {
        assert!(f.eval(text).is_err(), "{text}");
    }
}
