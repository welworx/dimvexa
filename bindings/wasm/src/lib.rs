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

use std::sync::OnceLock;

use dimvexa::{Conversion, Error, Registry};
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

/// Adds two quantity texts exactly (`"20 degC"` + `"5 delta_K"`) and returns the result as text.
/// A temperature point plus a temperature point is refused with code `kind_mismatch`.
///
/// # Errors
///
/// Throws a `DimvexaError` with a stable `code`.
#[wasm_bindgen(js_name = addQuantities)]
pub fn add_quantities(a: &str, b: &str) -> Result<String, JsValue> {
    let reg = registry()?;
    let qa = reg.quantity(a).map_err(|e| fail(&e))?;
    let qb = reg.quantity(b).map_err(|e| fail(&e))?;
    Ok(qa.checked_add(&qb).map_err(|e| fail(&e))?.to_string())
}
