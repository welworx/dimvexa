// SPDX-License-Identifier: MIT OR Apache-2.0
//! Equations as text (ADR-030): quantities written in the equation, named values, exponents,
//! and hostile input.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use dimvexa::registry;
use dimvexa_calc::{Formula, MAX_EQUATION_CHARS};

fn close(got: f64, want: f64) {
    assert!(
        ((got - want) / want).abs() < 1e-14,
        "got {got:e}, want {want:e}"
    );
}

#[test]
fn quantities_can_be_written_in_the_equation() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg);
    // an orifice flow, no names: Cd A sqrt(2 dp / rho)
    let q = f
        .eval("0.61 * 10 cm2 * sqrt(2 * 50 kPa / 998 kg/m3)")
        .unwrap();
    assert!(!q.is_exact());
    close(
        f.f64_in(&q, "L/s").unwrap(),
        0.61 * 1e-3 * (2.0 * 50e3 / 998.0_f64).sqrt() * 1000.0,
    );
    // units with a power, a product and a quotient in them
    assert_eq!(
        f.f64_in(&f.eval("8.314 J/(mol*K)").unwrap(), "J/(mol*K)")
            .unwrap(),
        8.314
    );
    assert_eq!(f.f64_in(&f.eval("3 kg.m/s2").unwrap(), "N").unwrap(), 3.0);
    assert_eq!(
        f.f64_in(&f.eval("2 m^3 / 4 m").unwrap(), "m^2").unwrap(),
        0.5
    );
    // a temperature point, a difference, a unit that is written right after the number
    assert_eq!(
        f.eval_exact("20 degC - 5 degC").unwrap().to_string(),
        "15 delta_degC"
    );
    assert_eq!(f.f64_in(&f.eval("10cm2").unwrap(), "m2").unwrap(), 0.001);
    assert_eq!(f.f64_in(&f.eval("1e3 m").unwrap(), "km").unwrap(), 1.0);
    assert_eq!(f.f64_in(&f.eval("5eV").unwrap(), "eV").unwrap(), 5.0);
}

#[test]
fn a_unit_is_the_text_after_the_number_up_to_a_space() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg);
    let m3 = |text: &str| f.f64_in(&f.eval(text).unwrap(), "kg/m3").unwrap();
    // spaces around an operator separate two quantities...
    assert_eq!(
        f.f64_in(&f.eval("50 kPa / 998 kg/m3").unwrap(), "kPa*m3/kg")
            .unwrap(),
        50.0 / 998.0
    );
    // ...and a `/` followed by a digit is an operator even without spaces
    assert_eq!(
        f.f64_in(&f.eval("50 kPa/998 kg/m3").unwrap(), "kPa*m3/kg")
            .unwrap(),
        50.0 / 998.0
    );
    assert_eq!(m3("998 kg/m3"), 998.0);
    assert_eq!(m3("998 (kg/m3)"), 998.0);
    // a `*` followed by a digit is a product
    assert_eq!(f.f64_in(&f.eval("10 cm2*3").unwrap(), "cm2").unwrap(), 30.0);
    // closing parentheses that belong to the equation are not part of the unit
    assert_eq!(
        f.f64_in(&f.eval("(20 ft)^2 / 2").unwrap(), "ft2").unwrap(),
        200.0
    );
    assert_eq!(
        f.f64_in(&f.eval("max(1 km, 900 m)").unwrap(), "m").unwrap(),
        1000.0
    );
    // a unit that does not exist is a loud error with suggestions
    let err = f.eval("5 foo").unwrap_err();
    assert_eq!(err.code(), "unknown_unit");
    // a number that is a bare number stays dimensionless
    assert_eq!(f.f64_in(&f.eval("2 * 3").unwrap(), "1").unwrap(), 6.0);
}

#[test]
fn names_and_quantities_mix() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("Cd", "0.61")
        .unwrap()
        .var("dp", "50 kPa")
        .unwrap();
    let q = f.eval("Cd * 10 cm2 * sqrt(2 * dp / 998 kg/m3)").unwrap();
    close(
        f.f64_in(&q, "L/s").unwrap(),
        0.61 * 1e-3 * (2.0 * 50e3 / 998.0_f64).sqrt() * 1000.0,
    );
    // a later binding replaces an earlier one
    let f = f.var("dp", "100 kPa").unwrap();
    close(
        f.f64_in(&f.eval("sqrt(dp / 1 kPa)").unwrap(), "1").unwrap(),
        10.0,
    );
    assert_eq!(f.eval("nothing * 2").unwrap_err().code(), "parse_error");
}

#[test]
fn pi_is_a_constant_and_stays_exact() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg);
    let c = f.eval("2 * pi * 3 m").unwrap();
    assert!(c.is_exact());
    assert_eq!(
        f.f64_in(&c, "m").unwrap(),
        2.0 * core::f64::consts::PI * 3.0
    );
    // a full turn is exact too
    assert_eq!(f.eval("sin(2 * pi * 1 rad)").unwrap().to_string(), "0 1");
    assert_eq!(f.eval("cos(pi * 1 rad)").unwrap().to_string(), "-1 1");
    // 1 + pi has no exact form: loud
    assert_eq!(f.eval("1 + pi").unwrap_err().code(), "unrepresentable");
}

/// `x^0.5`, `x^(1/2)` and `sqrt(x)` are the same number; so are the forms of the other powers.
#[test]
fn the_forms_of_a_fractional_power_agree() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg)
        .var("x", "2")
        .unwrap()
        .var("y", "8")
        .unwrap();
    let value = |text: &str| f.f64_in(&f.eval(text).unwrap(), "1").unwrap();
    assert_eq!(value("x^0.5"), 2.0_f64.sqrt());
    assert_eq!(value("x^(1/2)"), 2.0_f64.sqrt());
    assert_eq!(value("sqrt(x)"), 2.0_f64.sqrt());
    assert_eq!(f.eval_exact("y^(1/3)").unwrap().to_string(), "2 1");
    assert_eq!(value("y^(2/3)"), 4.0);
    assert_eq!(value("y^(-1/3)"), 0.5);
    assert_eq!(value("y^-1"), 0.125);
    assert_eq!(value("y^0.5"), 8.0_f64.sqrt());
    // a unit-carrying value needs integer exponents of the unit
    let g = Formula::new(&reg);
    assert_eq!(g.eval("(9 m^2)^0.5").unwrap().to_string(), "3 m");
    assert_eq!(g.eval("(9 m^2)^(1/2)").unwrap().to_string(), "3 m");
    assert_eq!(g.eval("(4 m)^0.5").unwrap_err().code(), "refused");
}

#[test]
fn hostile_equations_are_errors_and_never_panic() {
    let reg = registry().unwrap();
    let f = Formula::new(&reg).var("x", "1").unwrap();
    let deep = format!("{}1{}", "(".repeat(10_000), ")".repeat(10_000));
    assert_eq!(f.eval(&deep).unwrap_err().code(), "parse_error");
    let nested_calls = format!("{}1{}", "sqrt(".repeat(1_000), ")".repeat(1_000));
    assert_eq!(f.eval(&nested_calls).unwrap_err().code(), "parse_error");
    let long = "1 + ".repeat(MAX_EQUATION_CHARS);
    assert_eq!(f.eval(&long).unwrap_err().code(), "parse_error");
    // many unary minus signs are fine and cheap
    assert_eq!(
        f.f64_in(&f.eval(&format!("{}1", "-".repeat(2_000))).unwrap(), "1")
            .unwrap(),
        1.0
    );
    for text in [
        "",
        " ",
        "+",
        "*",
        "1 +",
        "(",
        ")",
        "()",
        "1 2",
        "1 m m",
        "sqrt",
        "sqrt(",
        "sqrt()",
        "sqrt(1,)",
        "min()",
        "x^",
        "x^(",
        "x^(1/",
        "x^()",
        "1e",
        "1e+",
        ".",
        "..",
        "1..2",
        "2 ^ 3 ^ 4",
        "x^1e400",
        "3 m^",
        "3 m/",
        "3 m*",
        "3 (m",
        "5 \u{0}",
        "5 \u{202e}m",
        "\u{1F600}",
        "1 @ 2",
        "a.b",
        "x.y",
        "1e999999999 m",
        "99999999999999999999999999999999999999999 m",
        "1 / 0",
        "0 ^ -1",
        "min(1 m, 1 s)",
        "hypot(1 m)",
        "log(1)",
        "tan(1)",
        "atan2(1, 2)",
        "asin(1 m)",
        "atan2(1 m)",
        "frobnicate(1)",
    ] {
        let _ = f.eval(text);
    }
}

#[test]
fn names_are_identifiers_pi_stays_and_the_count_is_bounded() {
    let reg = dimvexa::registry().unwrap();
    for bad in ["", " x", "d p", "1x", "a-b", "pi"] {
        match Formula::new(&reg).var(bad, "1 m") {
            Ok(_) => panic!("{bad:?} was accepted"),
            Err(e) => assert_eq!(e.code(), "invalid_definition", "{bad:?}"),
        }
    }
    // a name that is also a unit symbol is allowed (variables are looked up first)
    let f = Formula::new(&reg).var("A", "10 cm2").unwrap();
    assert_eq!(f.eval("A * 2").unwrap().to_string(), "20 cm^2");
    let mut f = Formula::new(&reg);
    for i in 0..dimvexa_calc::MAX_VARS {
        f = f.var(&format!("v{i}"), "1 m").unwrap();
    }
    assert_eq!(
        f.var("one_too_many", "1 m").err().unwrap().code(),
        "invalid_definition"
    );
}

#[test]
fn a_name_that_is_also_a_unit_in_the_equation_is_refused() {
    let reg = registry().unwrap();
    let h = Formula::new(&reg).var("h", "3 m").unwrap();
    for expr in ["0.5 h", "2 h * 1 m", "1 m/h"] {
        let e = h.eval(expr).err().map(|e| e.code());
        assert_eq!(e, Some("invalid_definition"), "{expr}");
    }
    // alone, the name is its value; a longer name is another name
    assert_eq!(h.eval("h * 2").unwrap().to_string(), "6 m");
    let m = Formula::new(&reg).var("m", "3 kg").unwrap();
    assert_eq!(
        m.eval("2 m3").err().map(|e| e.code()),
        Some("invalid_definition")
    );
    assert_eq!(m.eval("m * 2").unwrap().to_string(), "6 kg");
}
