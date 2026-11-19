// SPDX-License-Identifier: MIT OR Apache-2.0
//! Helpers and independent SI constants shared by the engineering tests.

use dimvexa_config::{Config, apply};

// Re-exported so each field's file starts with `use crate::common::*;` and nothing else.
pub use dimvexa::{Quantity, Ratio, Registry, registry};
pub use dimvexa_calc::Calc;
pub use dimvexa_testkit::formula::Formula;

// Independent SI constants for the expected values (exact by definition).
pub const FT: f64 = 0.3048;
pub const IN: f64 = 0.0254;
pub const LB: f64 = 0.453_592_37;
pub const LBF: f64 = 4.448_221_615_260_5;
pub const G_N: f64 = 9.806_65;
pub const GAL_US: f64 = 3.785_411_784e-3;
pub const PSI: f64 = LBF / (IN * IN);
pub const HP_MECH: f64 = 550.0 * FT * LBF;
pub const BTU_IT: f64 = 1_055.055_852_62;
pub const PI: f64 = std::f64::consts::PI;

/// Evaluates `formula` over `vars`, reads the result in `unit` and compares it with `want`.
pub fn check(vars: &[(&str, &str)], formula: &str, unit: &str, want: f64) {
    check_in(&registry().unwrap(), vars, formula, unit, want);
}

/// The same with a registry that carries a user config.
pub fn check_in(reg: &Registry, vars: &[(&str, &str)], formula: &str, unit: &str, want: f64) {
    let got = value(reg, vars, formula, unit);
    let rel = if want == 0.0 {
        got.abs()
    } else {
        ((got - want) / want).abs()
    };
    assert!(
        rel < 1e-12,
        "{formula} [{unit}]: got {got:e}, want {want:e} (relative error {rel:e})"
    );
}

/// The shipped table plus `examples/configs/engineering.toml`: the names a design office uses
/// (`gpm`, `mph`, `rpm`, `ksi`, `BTU`, ...) that dimvexa leaves to the user.
pub fn with_engineering_config() -> Registry {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/configs/engineering.toml"
    );
    let text = std::fs::read_to_string(path).unwrap();
    let mut reg = registry().unwrap();
    apply(
        &mut reg,
        &[Config::parse(&text, "engineering.toml").unwrap()],
    )
    .unwrap();
    reg
}

pub fn value(reg: &Registry, vars: &[(&str, &str)], formula: &str, unit: &str) -> f64 {
    let mut f = Formula::new(reg);
    for (name, q) in vars {
        f = f.var(name, q).unwrap();
    }
    let q = f.eval(formula).unwrap_or_else(|e| panic!("{formula}: {e}"));
    f.f64_in(&q, unit)
        .unwrap_or_else(|e| panic!("{formula} -> {unit}: {e} (result is {q})"))
}

/// The error code of evaluating `formula` and reading it in `unit`.
pub fn code(vars: &[(&str, &str)], formula: &str, unit: &str) -> &'static str {
    let reg = registry().unwrap();
    let mut f = Formula::new(&reg);
    for (name, q) in vars {
        f = f.var(name, q).unwrap();
    }
    let r = f.eval(formula).and_then(|q| f.f64_in(&q, unit));
    r.expect_err(formula).code()
}

pub fn q(reg: &Registry, s: &str) -> Quantity {
    reg.quantity(s).unwrap()
}
