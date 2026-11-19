// SPDX-License-Identifier: MIT OR Apache-2.0
//! How quantities are written: unit notation and dimensionless results (B2, C).

use crate::common::*;

#[test]
/// B2 (fixed): a unit to the power 0 is the plain number 1, not `m^0`.
fn b2_power_zero_is_dimensionless() {
    let reg = registry().unwrap();
    let one = Calc::new(&reg).pow(&q(&reg, "5 m"), 0).unwrap();
    assert!(one.unit().is_unity(), "{one}");
    assert_eq!(one.to_string(), "1 1");
}

/// A function that does not exist is a `refused` error that lists the ones that do; there is no
/// function `to` (a conversion is `x.to(UNIT)`).
#[test]
fn an_unknown_function_lists_the_known_ones() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg).var("x", "0.5").unwrap();
    for call in ["to(x, K)", "to(20 degC, K)", "frobnicate(1)", "to("] {
        let err = f.eval(call).unwrap_err();
        assert_eq!(err.code(), "refused", "{call}");
        let msg = err.to_string();
        assert!(msg.contains("unknown function"), "{call}: {msg}");
        assert!(msg.contains("sqrt"), "{call}: {msg}");
    }
}

/// `2^3 m` is `(2^3) m`: a unit cannot be part of an exponent, so it multiplies the power.
#[test]
fn a_unit_after_an_exponent_multiplies_the_power() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg).var("x", "3").unwrap();
    for (text, unit, want) in [
        ("2^3 m", "m", 8.0),
        ("2^3 m^2", "m2", 8.0),
        ("2^3^2 m", "m", 512.0),
        ("2^-2 m", "m", 0.25),
        ("x^2 m", "m", 9.0),
    ] {
        assert_eq!(
            f.f64_in(&f.eval(text).unwrap(), unit).unwrap(),
            want,
            "{text}"
        );
    }
}

/// `pow`, `floor`, `ceil` and `round`: `pow` is `^`, the others keep the unit.
#[test]
fn pow_floor_ceil_and_round() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg);
    for (text, want) in [
        ("pow(2, 10)", "1024 1"),
        ("pow(9 m2, 0.5)", "3 m"),
        ("pow(2 m, 3)", "8 m^3"),
        ("pow(8, 1/3)", "2 1"),
        ("floor(7.8 m)", "7 m"),
        ("floor(-7.2 m)", "-8 m"),
        ("ceil(7.2 m)", "8 m"),
        ("ceil(-7.2 m)", "-7 m"),
        ("round(2.5 m)", "3 m"),
        ("round(-2.5 m)", "-3 m"),
        ("round(2.4 m)", "2 m"),
        ("round(20.7 degC)", "21 degC"),
        ("floor(2 pi)", "6 1"),
        ("round(pi * 1 rad)", "3 rad"),
        ("floor(1e-400)", "0 1"),
        ("ceil(1e-400)", "1 1"),
        ("floor(-1e-400)", "-1 1"),
    ] {
        assert_eq!(f.eval(text).unwrap().to_string(), want, "{text}");
    }
    // an approximate value stays approximate, and `pow` is as strict as `^`
    assert!(!f.eval("floor(sqrt(2) * 10 m)").unwrap().is_exact());
    for text in [
        "pow(2, sqrt(2))",
        "pow(2 m, 1 s)",
        "pow(2)",
        "floor(1, 2)",
        "round()",
    ] {
        assert!(f.eval(text).is_err(), "{text}");
    }
    // a value that has pi and is too close to a whole number to tell is refused
    assert_eq!(f.eval("floor(0 pi + 1)").unwrap().to_string(), "1 1");
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
fn the_sign_and_absolute_value_of_a_temperature_point_keep_its_scale() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("Tc", "-5 degC")
        .unwrap()
        .var("T1", "20 degC")
        .unwrap()
        .var("T2", "80 degC")
        .unwrap()
        .var("d", "-5 delta_degC")
        .unwrap();
    assert_eq!(f.eval_exact("abs(Tc)").unwrap().to_string(), "5 degC");
    assert_eq!(f.eval_exact("-Tc").unwrap().to_string(), "5 degC");
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
        ("2 N·m", "2 N*m"),
        ("3 m × 4 m", "12 m*m"),
        ("6 m ÷ 2", "3 m"),
        ("2 m\t*\n3 m", "6 m*m"),
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
    let pi = std::f64::consts::PI;
    assert_eq!(value(&reg, &[("r", "3 m")], "2 pi r", "m"), 6.0 * pi);
    assert_eq!(value(&reg, &[("r", "3 m")], "pi r^2", "m2"), 9.0 * pi);
    assert_eq!(value(&reg, &[], "2 pi", "1"), 2.0 * pi);
    // a number before a variable that is not a unit multiplies
    assert_eq!(value(&reg, &[("r", "3 m")], "2 r", "m"), 6.0);
    // before a name that is also a unit it is refused
    let g = Formula::new(&reg).var("h", "3 m").unwrap();
    assert_eq!(g.eval("2 h").unwrap_err().code(), "invalid_definition");
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
