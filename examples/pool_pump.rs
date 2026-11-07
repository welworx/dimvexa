// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cookbook example: pool pump electricity.
//!
//! A pool pump is rated 1.1 kW and runs 8 hours a day for 30 days. How much energy is that
//! in kWh, MJ and Btu, and what does it cost at 0.30 EUR/kWh?
//!
//! Run with `cargo run --example pool_pump -p dimvexa`. The numbers are asserted by
//! `crates/dimvexa/tests/pool_pump.rs`, which runs in CI.
//!
//! The library converts units; it does not compute pump efficiency (electrical input power
//! and hydraulic output power are different quantities) and it has no currency, so the cost
//! step is plain exact arithmetic on the kWh value.
#![allow(clippy::print_stdout, dead_code)]

use dimvexa::{Error, Ratio, registry};

/// Every number the example reports, exact where the library can be exact.
#[derive(Debug)]
pub struct Report {
    /// Energy per running day in kWh (exact).
    pub daily_kwh: Ratio,
    /// Energy over the period in kWh (exact).
    pub total_kwh: Ratio,
    /// Energy over the period in MJ (exact).
    pub total_mj: Ratio,
    /// Energy over the period in Btu (International Table), correctly rounded `f64`.
    pub total_btu_it: f64,
    /// Btu (IT) per kWh, `f64`.
    pub btu_per_kwh: f64,
    /// Cost in EUR (exact).
    pub cost_eur: Ratio,
    /// Pump flow of 12 m3/h in US gallons per minute, `f64`.
    pub flow_gpm: f64,
}

/// Computes the report with the real API.
///
/// # Errors
///
/// Any library error (none occurs for these inputs).
pub fn compute() -> Result<Report, Error> {
    let reg = registry()?;

    let power = reg.quantity("1.1 kW")?;
    // "8 h/d": run time per day, a quantity with the compound unit h/d.
    let duty = reg.quantity("8 h")?.checked_div(&reg.quantity("1 d")?)?;
    let period = reg.quantity("30 d")?;

    let per_day = power.checked_mul(&duty)?; // 8.8 kW.h/d
    let energy = per_day.checked_mul(&period)?; // 264 kW.(h/d).d

    let daily_kwh = per_day.to_unit(&reg.parse("kWh/d")?)?.value();
    let total = energy.to_unit(&reg.parse("kWh")?)?;
    let total_mj = energy.to_unit(&reg.parse("MJ")?)?.value();
    let total_btu_it = energy.to_unit(&reg.parse("Btu_IT")?)?.to_f64()?;

    // Prepared conversion, applied as often as needed: Btu per kWh.
    let btu_per_kwh = reg.conversion("kWh", "Btu_IT")?.apply(1.0);

    // 79.20 EUR at 0.30 EUR/kWh: exact rational arithmetic, no float.
    let tariff: Ratio = "0.30".parse()?;
    let cost_eur = total.value().checked_mul(tariff)?;

    // Pump flow: 12 m3/h in US gallons per minute (explicit variant, never a bare "gal").
    let flow_gpm = reg.conversion("m3/h", "gal_us/min")?.apply(12.0);

    Ok(Report {
        daily_kwh,
        total_kwh: total.value(),
        total_mj,
        total_btu_it,
        btu_per_kwh,
        cost_eur,
        flow_gpm,
    })
}

/// Formats the report exactly as the example prints it.
///
/// # Errors
///
/// [`Error::Overflow`] if a decimal expansion overflows (cannot happen here).
pub fn render(r: &Report) -> Result<String, Error> {
    let dec = |x: Ratio| x.to_decimal(6).map(|(s, _)| s);
    Ok(format!(
        "1.1 kW for 8 h/day for 30 days\n\
         energy per day : {} kWh\n\
         energy total   : {} kWh\n\
         energy total   : {} MJ\n\
         energy total   : {:.0} Btu(IT)  ({:.2} Btu(IT) per kWh)\n\
         cost at 0.30 EUR/kWh : {:.2} EUR\n\
         12 m3/h = {:.1} US gal/min\n",
        dec(r.daily_kwh)?,
        dec(r.total_kwh)?,
        dec(r.total_mj)?,
        r.total_btu_it,
        r.btu_per_kwh,
        r.cost_eur.to_f64()?,
        r.flow_gpm,
    ))
}

fn main() -> Result<(), Error> {
    let report = compute()?;
    print!("{}", render(&report)?);
    Ok(())
}
