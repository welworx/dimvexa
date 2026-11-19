// SPDX-License-Identifier: MIT OR Apache-2.0
//! `dimvexa convert`: the conversion and how its result is printed (text and JSON).
// Private module: `pub` here is crate-visible, and the two visibility lints disagree about it.
#![allow(unreachable_pub)]

use crate::args::Args;
use crate::color::{BOLD_GREEN, CYAN, RESET};
use crate::json::{configs_json, f64_text, json_error, json_string, schema_id, warnings_json};
use crate::load::Loaded;
use dimvexa::{Conversion, Dim, Error, Ratio, UnitExpr};
use dimvexa_config::Density;
use std::io::Write;

/// Everything computed for a successful conversion, before formatting.
pub struct Outcome<'a> {
    pub args: &'a Args,
    pub configs: &'a [(String, String)],
    /// What a later config replaced from an earlier one.
    pub warnings: &'a [String],
    /// The density the conversion went through, if it needed one.
    pub density: Option<&'a Density>,
    pub from: UnitExpr,
    pub to: UnitExpr,
    pub conv: Conversion,
    pub exact: Ratio,
    /// The exact result rounded once to the nearest `f64` (the CLI always uses the exact path).
    pub approx: f64,
}

pub fn compute<'a>(
    args: &'a Args,
    loaded: &'a Loaded,
    density: Option<&'a Density>,
) -> Result<Outcome<'a>, Error> {
    let reg = &loaded.lib.reg;
    let value = Ratio::parse_expression(&args.value, dimvexa::CONSTANTS)?;
    let from = reg.parse(&args.from)?;
    let to = reg.parse(&args.to)?;
    // A density is used only when asked for and only when the plain conversion is refused
    // for lack of a density; otherwise the ordinary exact conversion stands.
    let (conv, used) = match (Conversion::between(&from, &to), density) {
        (Ok(conv), _) => (conv, None),
        (Err(Error::DimensionMismatch { .. }), Some(d)) => (d.conversion(&from, &to)?, Some(d)),
        (Err(e), _) => return Err(e),
    };
    let exact = conv.apply_exact(value)?;
    let approx = exact.to_f64()?;
    Ok(Outcome {
        args,
        configs: &loaded.configs,
        warnings: &loaded.lib.warnings,
        density: used,
        from,
        to,
        conv,
        exact,
        approx,
    })
}

pub fn render_text(o: &Outcome<'_>, color: bool) -> String {
    let value = if o.args.exact {
        // The symbolic exact value and the result rounded once to the nearest f64 (ADR-018).
        // A whole number is its own decimal, so it is printed once, without "/1".
        match o.exact.folded() {
            Ok(w) if w.is_integer() && !w.has_exponents() => w.num().to_string(),
            _ => format!("{} = {}", o.exact.to_fraction_string(), f64_text(o.approx)),
        }
    } else {
        f64_text(o.approx)
    };
    if color {
        format!("{BOLD_GREEN}{value}{RESET} {CYAN}{}{RESET}", o.to)
    } else {
        format!("{value} {}", o.to)
    }
}

/// The JSON object describing a density: everything a reader needs to judge the assumption.
pub fn density_json(b: &Density) -> String {
    format!(
        "{{\"name\":{},\"kind\":{},\"value\":{},\"unit\":{},\"conditions\":{},\"source\":{},\"file\":{},\"line\":{}}}",
        json_string(&b.name),
        json_string("density"),
        json_string(&b.value.to_fraction_string()),
        json_string("kg/m3"),
        json_string(&b.conditions),
        json_string(&b.source),
        json_string(&b.origin),
        b.line
    )
}

/// One line for people: the assumption behind a result that used a density. Text from the config has its
/// control characters replaced, and a density that is not a short decimal is shown as the
/// exact fraction, so the note never reads as a different number.
pub fn density_note(b: &Density) -> String {
    let value = match b.value.to_decimal(15) {
        Ok((decimal, true)) => decimal,
        Ok((decimal, false)) => format!("{} (about {decimal})", b.value.to_fraction_string()),
        Err(_) => b.value.to_fraction_string(),
    };
    let clean = |s: &str| -> String {
        s.chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect()
    };
    let at = if b.line == 0 {
        clean(&b.origin)
    } else {
        format!("{}:{}", clean(&b.origin), b.line)
    };
    format!(
        "note: used density {} ({} {}; {}; source: {}; {})",
        clean(&b.name),
        value,
        "kg/m3",
        clean(&b.conditions),
        clean(&b.source),
        at
    )
}

pub fn render_json(o: &Outcome<'_>) -> Result<String, Error> {
    // A value with pi has no finite expansion: the decimal is then the result rounded once to
    // the nearest f64, and `truncated` says it is not the whole value.
    // The same fallback applies to a power of ten that does not fit the fraction.
    let (decimal, complete) = match o.exact.to_decimal(15) {
        Ok(d) => d,
        Err(_) if o.exact.pi_exp() != 0 || o.exact.ten_exp() != 0 => (f64_text(o.approx), false),
        Err(e) => return Err(e),
    };
    let dims: Vec<String> =
        o.to.dim()
            .exponents()
            .iter()
            .map(ToString::to_string)
            .collect();
    let mut sources: Vec<String> = o.from.sources().to_vec();
    for s in o.to.sources() {
        if !sources.contains(s) {
            sources.push(s.clone());
        }
    }
    if let Some(b) = o.density {
        sources.push(format!("{} (density {})", b.source, b.name));
    }
    let sources: Vec<String> = sources.iter().map(|s| json_string(s)).collect();
    let exactness = o.density.map_or_else(
        || o.from.exactness().combine(o.to.exactness()),
        |b| {
            o.from
                .exactness()
                .combine(o.to.exactness())
                .combine(b.exactness())
        },
    );
    let packs: Vec<String> = dimvexa::PACKS.iter().map(|p| json_string(p)).collect();
    Ok(format!(
        "{{\"schema\":{},\"ok\":true,\
\"input\":{{\"value\":{},\"unit\":{}}},\
\"output\":{{\"value\":{},\"unit\":{},\"f64\":{},\"truncated\":{}}},\
\"exact\":{{\"value\":{},\"decimal\":{},\"factor\":{},\"offset\":{}}},\
\"exactness\":{},\"dimension\":[{}],\"kind\":{},\"quantity_kind\":{},\
\"packs\":[{}],\"data_version\":{},\"sources\":[{}],\"config\":{},\"density\":{},\"warnings\":{}}}",
        json_string(&schema_id("convert")),
        json_string(&o.args.value),
        json_string(&o.from.to_string()),
        json_string(&decimal),
        json_string(&o.to.to_string()),
        f64_text(o.approx),
        !complete,
        json_string(&o.exact.to_fraction_string()),
        json_string(&f64_text(o.approx)),
        json_string(&o.conv.factor().to_fraction_string()),
        json_string(&o.conv.offset().to_fraction_string()),
        json_string(exactness.name()),
        dims.join(","),
        json_string(o.conv.shape().name()),
        json_string(o.to.kind().name()),
        packs.join(","),
        json_string(&dimvexa::data_version()),
        sources.join(","),
        configs_json(o.configs),
        o.density.map_or_else(|| "null".to_string(), density_json),
        warnings_json(o.warnings),
    ))
}

pub fn fail(args: &Args, e: &Error, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    if args.json {
        let _ = writeln!(
            out,
            "{}",
            json_error(
                "convert",
                e.code(),
                &e.to_string(),
                e.candidates(),
                e.suggestions()
            )
        );
    } else {
        let _ = writeln!(err, "error[{}]: {e}", e.code());
        if let Error::DimensionMismatch { from, to } = e {
            let bare = |w: &str| matches!(w, "F" | "C");
            for (word, other, other_word) in
                [(&args.to, *from, &args.from), (&args.from, *to, &args.to)]
            {
                // A bare F or C beside a temperature, or beside another bare F or C
                // (`0 C F`), is almost always a temperature scale.
                if bare(word) && (other == Dim::K || bare(other_word)) {
                    let (what, fix) = if word == "F" {
                        ("farad", "degF or °F")
                    } else {
                        ("coulomb", "degC or °C")
                    };
                    let _ = writeln!(
                        err,
                        "hint: '{word}' alone is the {what}, not a temperature scale; for the scale write {fix}"
                    );
                }
            }
            if Density::density_needed(*from, *to) && args.density.is_none() {
                let _ = writeln!(
                    err,
                    "hint: mass and volume need a density. Pass your own number, for example \
--const 998.2067kg/m3, or a named one: --config water --const water_25C \
(docs/adr/014-one-config-mechanism.md)"
                );
            }
        }
    }
    1
}
