// SPDX-License-Identifier: MIT OR Apache-2.0
//! WebAssembly binding for Dimvexa (ADR-011).
//!
//! No conversion logic lives here: every function calls the Rust API and maps the result. Errors
//! become JavaScript `Error` objects with the stable `code` plus `suggestions` and `candidates`
//! arrays, so a JS caller sees the same loud failures as a Rust caller.
//!
//! The batch entry points exist because a JS to WASM call costs more than the multiplication it
//! wraps: one call per array amortises that cost (`docs/DESIGN.md`).
#![deny(unsafe_code)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

use dimvexa::{Conversion, Error, Registry, UnitExpr};
use dimvexa_config::{Density, DensityError};
use js_sys::{Array, Float64Array, Reflect};
use wasm_bindgen::prelude::*;

static REGISTRY: OnceLock<Result<Registry, Error>> = OnceLock::new();

fn registry() -> Result<&'static Registry, JsValue> {
    match REGISTRY.get_or_init(dimvexa::registry) {
        Ok(reg) => Ok(reg),
        Err(e) => Err(js_error(e)),
    }
}

fn strings(items: &[String]) -> Array {
    items.iter().map(|s| JsValue::from_str(s)).collect()
}

/// Builds `Error(message)` with `.name = "DimvexaError"`, `.code`, `.suggestions`, `.candidates`.
fn js_error(e: &Error) -> JsValue {
    let err = js_sys::Error::new(&e.to_string());
    err.set_name("DimvexaError");
    let obj: &JsValue = err.as_ref();
    // Setting a property on a fresh Error object cannot fail; ignore the bool result.
    let _ = Reflect::set(obj, &"code".into(), &e.code().into());
    let _ = Reflect::set(obj, &"suggestions".into(), &strings(e.suggestions()));
    let _ = Reflect::set(obj, &"candidates".into(), &strings(e.candidates()));
    err.into()
}

fn fail(e: &Error) -> JsValue {
    js_error(e)
}

thread_local! {
    /// The last few config combinations, so a worksheet that calls the same one thousands of
    /// times builds its registry once. Oldest entry is dropped first.
    static LOADED: RefCell<Vec<(String, Rc<Loaded>)>> = const { RefCell::new(Vec::new()) };
}

const LOADED_CAP: usize = 8;

/// The registry with the configs applied and the constants in effect, built by
/// `dimvexa-config` so the binding cannot differ from the CLI (ADR-018).
type Loaded = dimvexa_config::Loaded;

fn invalid(symbol: &str, message: String) -> JsValue {
    fail(&Error::InvalidDefinition {
        symbol: symbol.to_string(),
        message,
    })
}

/// Applies shipped configs by name (`"us"`, `"us,water"`; commas or spaces) and then your own
/// config files given as TOML text, to a copy of the registry. A file path cannot be read from a
/// worksheet or a browser, so the caller reads the file and passes its text. Later configs win
/// (ADR-009). No config is the plain strict table.
fn load_config(spec: &str, texts: &[String]) -> Result<Rc<Loaded>, JsValue> {
    let mut names: Vec<&str> = Vec::new();
    for name in spec.split([',', ' ', ';']).filter(|n| !n.is_empty()) {
        // Later entries win (ADR-009), so keep the last occurrence of a repeated name.
        names.retain(|n| *n != name);
        names.push(name);
    }
    let key = format!("{}\u{0}{}", names.join(","), texts.join("\u{0}"));
    let cached = LOADED.with(|c| {
        c.borrow()
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| Rc::clone(v))
    });
    if let Some(hit) = cached {
        return Ok(hit);
    }
    let mut configs = Vec::new();
    for name in &names {
        let bundled = dimvexa_config::bundled(name).ok_or_else(|| {
            let known: Vec<&str> = dimvexa_config::BUNDLED.iter().map(|b| b.name).collect();
            invalid(
                "config",
                format!(
                    "no shipped config named '{name}' (a file path cannot be used here: pass your own file as TOML text); shipped configs: {}",
                    known.join(", ")
                ),
            )
        })?;
        configs.push(bundled.config().map_err(|e| fail(&e))?);
    }
    for (i, text) in texts.iter().enumerate() {
        let origin = format!("config-text-{}", i.saturating_add(1));
        configs.push(dimvexa_config::Config::parse(text, &origin).map_err(|e| fail(&e))?);
    }
    let loaded = Rc::new(
        dimvexa_config::load(registry()?.clone(), &configs, dimvexa::CONSTANTS)
            .map_err(|e| fail(&e))?,
    );
    // Replacing an entry of an earlier config is allowed, never silent: `console.warn`.
    for w in &loaded.warnings {
        warn(w);
    }
    LOADED.with(|c| {
        let mut list = c.borrow_mut();
        if list.len() >= LOADED_CAP {
            list.remove(0);
        }
        list.push((key, Rc::clone(&loaded)));
    });
    Ok(loaded)
}

/// `console.warn(message)`, where a console exists.
fn warn(message: &str) {
    let console = Reflect::get(&js_sys::global(), &"console".into()).unwrap_or(JsValue::UNDEFINED);
    if let Ok(f) = Reflect::get(&console, &"warn".into()) {
        if let Some(f) = f.dyn_ref::<js_sys::Function>() {
            let _ = f.call1(&console, &format!("warning: {message}").into());
        }
    }
}

/// Resolves a density argument through `dimvexa-config` (a number with a unit, or the name of a
/// constant of a density unit). Nothing is computed or guessed.
fn resolve_density(loaded: &Loaded, spec: &str) -> Result<Density, JsValue> {
    loaded.resolve_density(spec).map_err(|e| match e {
        DensityError::Lib(e) => fail(&e),
        DensityError::Unknown { message, .. } => invalid("density", message),
        _ => invalid("density", "unusable density".to_string()),
    })
}

/// The ordinary conversion if one exists. A density that was given is always checked; it is used
/// only when the plain conversion is refused for lack of one (mass to volume or back). `config`,
/// `config_texts` and `density` are empty for "none".
fn conversion_with_options(
    from: &str,
    to: &str,
    density: &str,
    config: &str,
    config_texts: &[String],
) -> Result<Conversion, JsValue> {
    let loaded = load_config(config, config_texts)?;
    let reg = &loaded.reg;
    // A density that was given must be a valid one even when this conversion does not need it,
    // as in the CLI: a typo in a density cell must not stay invisible until a row needs it.
    let density = density.trim();
    let resolved = if density.is_empty() {
        None
    } else {
        Some(resolve_density(&loaded, density)?)
    };
    let f: UnitExpr = reg.parse(from).map_err(|e| fail(&e))?;
    let t: UnitExpr = reg.parse(to).map_err(|e| fail(&e))?;
    match Conversion::between(&f, &t) {
        Ok(conv) => Ok(conv),
        Err(e @ Error::DimensionMismatch { .. }) => match resolved {
            Some(d) => d.conversion(&f, &t).map_err(|e| fail(&e)),
            None => Err(fail(&e)),
        },
        Err(e) => Err(fail(&e)),
    }
}

/// A conversion between two unit expressions, composed once and applied many times.
#[wasm_bindgen]
#[derive(Debug)]
pub struct DimvexaConversion {
    inner: Conversion,
}

#[wasm_bindgen]
impl DimvexaConversion {
    /// Parses both expressions and composes the exact factor once.
    ///
    /// # Errors
    ///
    /// Throws a `DimvexaError` (unknown or ambiguous unit, dimension or kind mismatch, ...).
    #[wasm_bindgen(constructor)]
    pub fn new(from: &str, to: &str) -> Result<DimvexaConversion, JsValue> {
        let inner = registry()?.conversion(from, to).map_err(|e| fail(&e))?;
        Ok(Self { inner })
    }

    /// Like the constructor, with the options of a worksheet: `density` is your own number such
    /// as `"998.2067kg/m3"` or the name of a constant of a density unit (empty for none),
    /// `config` is a shipped config name or a list such as `"us"` or `"us,water"` (empty for
    /// none), and `configTexts` are your own config files as TOML text, applied after the
    /// shipped ones. The density is used only when the plain conversion needs one.
    ///
    /// # Errors
    ///
    /// Throws a `DimvexaError` (unknown config or density, a density that is not a density, ...).
    #[wasm_bindgen(js_name = withOptions)]
    pub fn with_options(
        from: &str,
        to: &str,
        density: &str,
        config: &str,
        config_texts: Vec<String>,
    ) -> Result<DimvexaConversion, JsValue> {
        let inner = conversion_with_options(from, to, density, config, &config_texts)?;
        Ok(Self { inner })
    }

    /// Fast path (`f64`): at most 1 ulp (linear) or 2 ulp (affine), `docs/precision.md`.
    #[must_use]
    pub fn apply(&self, x: f64) -> f64 {
        self.inner.apply(x)
    }

    /// Exact path for one value: rational arithmetic, rounded once.
    ///
    /// # Errors
    ///
    /// Throws for NaN, infinity, or exact-arithmetic overflow.
    #[wasm_bindgen(js_name = applyExact)]
    pub fn apply_exact(&self, x: f64) -> Result<f64, JsValue> {
        self.inner.apply_exact_f64(x).map_err(|e| fail(&e))
    }

    /// Fast path over a whole array in one call; returns a new `Float64Array`. The caller's
    /// array is not changed. To avoid allocating a result array on every call, reuse one array
    /// with `applyInPlace`.
    #[wasm_bindgen(js_name = applyBatch)]
    #[must_use]
    pub fn apply_batch(&self, mut input: Vec<f64>) -> Float64Array {
        // `input` is already a copy in WASM memory: convert it there, no second buffer.
        self.inner.apply_in_place(&mut input);
        Float64Array::from(input.as_slice())
    }

    /// Fast path in place: overwrites the array's contents, allocates nothing on the JS side.
    #[wasm_bindgen(js_name = applyInPlace)]
    pub fn apply_in_place(&self, data: &mut [f64]) {
        self.inner.apply_in_place(data);
    }

    /// The inverse conversion.
    ///
    /// # Errors
    ///
    /// Throws on exact-arithmetic overflow.
    pub fn inverse(&self) -> Result<DimvexaConversion, JsValue> {
        let inner = self.inner.inverse().map_err(|e| fail(&e))?;
        Ok(Self { inner })
    }
}

/// One-shot fast-path conversion: `convert(1, "km", "m")`.
///
/// # Errors
///
/// Throws a `DimvexaError` with a stable `code`.
#[wasm_bindgen]
pub fn convert(value: f64, from: &str, to: &str) -> Result<f64, JsValue> {
    Ok(registry()?
        .conversion(from, to)
        .map_err(|e| fail(&e))?
        .apply(value))
}

/// One-shot fast-path conversion with the options of a worksheet: `convertWith(1, "L", "kg",
/// "998.2067kg/m3", "", [])`, `convertWith(1, "ton", "kg", "", "us", [])` or `convertWith(1, "L",
/// "kg", "water_25C", "water", [])`. The last argument is your own config files as TOML text.
/// See `DimvexaConversion.withOptions`.
///
/// # Errors
///
/// Throws a `DimvexaError` with a stable `code`.
#[wasm_bindgen(js_name = convertWith)]
pub fn convert_with(
    value: f64,
    from: &str,
    to: &str,
    density: &str,
    config: &str,
    config_texts: Vec<String>,
) -> Result<f64, JsValue> {
    Ok(conversion_with_options(from, to, density, config, &config_texts)?.apply(value))
}

/// The named constants in effect (ADR-018): the built-in ones, then those of the configs, as an
/// array of objects with `name`, `value` (an exact string such as `"662607015 10^-42"`), `valueText` (the same number for
/// people, `"6.62607015e-34"`), `unit`,
/// `exactness`, `codata`, `conditions`, `source`, `note`, `file` and `line`.
///
/// # Errors
///
/// Throws a `DimvexaError` (unknown config, a config that redefines a built-in constant, an
/// unknown unit, ...).
#[wasm_bindgen]
pub fn constants(config: &str, config_texts: Vec<String>) -> Result<Array, JsValue> {
    let loaded = load_config(config, &config_texts)?;
    let list = Array::new();
    for c in &loaded.constants {
        let obj = js_sys::Object::new();
        let text = |k: &str, v: &str| Reflect::set(&obj, &k.into(), &v.into());
        let _ = text("name", &c.name);
        let _ = text("value", &c.value.to_fraction_string());
        let _ = text("valueText", &c.value_text());
        let _ = text("unit", &c.unit);
        let _ = text("exactness", c.exactness.name());
        let _ = text("codata", &c.codata);
        let _ = text("conditions", &c.conditions);
        let _ = text("source", &c.source);
        let _ = text("note", &c.note);
        let _ = text("file", &c.origin);
        let _ = Reflect::set(
            &obj,
            &"line".into(),
            &JsValue::from_f64(f64::from(u32::try_from(c.line).unwrap_or(u32::MAX))),
        );
        list.push(&obj);
    }
    Ok(list)
}

/// One-shot exact-path conversion of a decimal string, e.g. `convertExact("98.6", "degF", "degC")`.
/// The string is parsed as an exact rational, so no input rounding happens; the result is
/// rounded once to `f64`.
///
/// # Errors
///
/// Throws a `DimvexaError` with a stable `code`.
#[wasm_bindgen(js_name = convertExact)]
pub fn convert_exact(value: &str, from: &str, to: &str) -> Result<f64, JsValue> {
    let reg = registry()?;
    let q = reg
        .quantity(&format!("{} {}", value.trim(), from))
        .map_err(|e| fail(&e))?;
    let target = reg.parse(to).map_err(|e| fail(&e))?;
    q.to_unit(&target)
        .and_then(|r| r.to_f64())
        .map_err(|e| fail(&e))
}

/// Converts a quantity text such as `"5 ft"` to `to`, returning the exact quantity as text.
///
/// # Errors
///
/// Throws a `DimvexaError` with a stable `code`.
#[wasm_bindgen(js_name = convertQuantity)]
pub fn convert_quantity(quantity: &str, to: &str) -> Result<String, JsValue> {
    let reg = registry()?;
    let q = reg.quantity(quantity).map_err(|e| fail(&e))?;
    let target = reg.parse(to).map_err(|e| fail(&e))?;
    Ok(q.to_unit(&target).map_err(|e| fail(&e))?.to_string())
}

/// Number of units in the table.
#[wasm_bindgen(js_name = unitCount)]
pub fn unit_count() -> Result<usize, JsValue> {
    Ok(registry()?.len())
}

/// Identifier of the unit table: `v<version>+<hash>`.
#[wasm_bindgen(js_name = dataVersion)]
#[must_use]
pub fn data_version() -> String {
    dimvexa::data_version()
}

/// Every stable error code (`Error::ALL_CODES`).
#[wasm_bindgen(js_name = errorCodes)]
#[must_use]
pub fn error_codes() -> Array {
    Error::ALL_CODES
        .iter()
        .map(|c| JsValue::from_str(c))
        .collect()
}
