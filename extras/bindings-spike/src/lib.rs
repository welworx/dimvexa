//! Experimental PyO3 spike for ADR-010. Not part of the workspace. Surface: `convert`,
//! `Converter.apply`, `Converter.apply_slice`, `Converter.apply_in_place`, `DimvexaError`.
//!
//! No numpy crate: arrays come in through the Python buffer protocol (`PyBuffer<f64>`), so
//! `array.array('d')`, `numpy.ndarray` (float64), `memoryview` and `mmap` all work.
//! This file contains no hand-written `unsafe`.

use dimvexa::{Conversion, Error, Registry, registry};
use pyo3::buffer::PyBuffer;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::sync::OnceLock;

pyo3::create_exception!(_dimvexa_spike, DimvexaError, PyValueError);

/// Process-wide registry, built once (the table is compiled in).
fn reg() -> PyResult<&'static Registry> {
    static REG: OnceLock<Result<Registry, Error>> = OnceLock::new();
    match REG.get_or_init(registry) {
        Ok(r) => Ok(r),
        Err(e) => Err(map_err(e)),
    }
}

/// Maps a core error to `DimvexaError(message)` with `.code`, `.suggestions` attributes.
fn map_err(e: &Error) -> PyErr {
    let err = DimvexaError::new_err(e.to_string());
    Python::attach(|py| {
        let v = err.value(py);
        let _ = v.setattr("code", e.code());
        let _ = v.setattr("suggestions", e.suggestions().to_vec());
    });
    err
}

fn f64_buffer(obj: &Bound<'_, PyAny>) -> PyResult<PyBuffer<f64>> {
    // `PyBuffer::<f64>::get` checks format "d" and itemsize 8, so float32/int arrays are
    // rejected with a TypeError instead of being reinterpreted.
    let buf = PyBuffer::<f64>::get(obj)?;
    if buf.dimensions() != 1 {
        return Err(PyValueError::new_err(
            "expected a one-dimensional float64 buffer",
        ));
    }
    if !buf.is_c_contiguous() {
        return Err(PyValueError::new_err("buffer must be C-contiguous"));
    }
    Ok(buf)
}

/// A prepared conversion between two unit expressions.
#[pyclass(frozen, module = "dimvexa_spike")]
struct Converter {
    inner: Conversion,
}

#[pymethods]
impl Converter {
    #[new]
    fn new(from_unit: &str, to_unit: &str) -> PyResult<Self> {
        let inner = reg()?
            .conversion(from_unit, to_unit)
            .map_err(|e| map_err(&e))?;
        Ok(Self { inner })
    }

    /// Scalar fast path (`Conversion::apply`).
    fn apply(&self, x: f64) -> f64 {
        self.inner.apply(x)
    }

    /// Converts a float64 buffer into a new `array.array('d')`.
    fn apply_slice<'py>(
        &self,
        py: Python<'py>,
        data: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let buf = f64_buffer(data)?;
        let mut out = vec![0.0_f64; buf.item_count()];
        buf.copy_to_slice(py, &mut out)?;
        self.inner.apply_in_place(&mut out);
        let mut bytes = Vec::with_capacity(out.len().saturating_mul(8));
        for v in &out {
            bytes.extend_from_slice(&v.to_ne_bytes());
        }
        py.import("array")?
            .call_method1("array", ("d", PyBytes::new(py, &bytes)))
    }

    /// Converts a writable float64 buffer in place.
    ///
    /// NOT zero-copy: `PyBuffer::as_mut_slice` is deprecated as unsound in PyO3 0.30 and the
    /// safe API only offers `copy_to_slice` / `copy_from_slice`, so this copies out, converts
    /// with the core's `apply_in_place` and copies back (one temporary `Vec`, two memcpys).
    /// A zero-copy version needs `unsafe` (`as_slice_ptr`) or the `numpy` crate (ADR-010).
    fn apply_in_place(&self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<()> {
        let buf = f64_buffer(data)?;
        if buf.readonly() {
            return Err(PyValueError::new_err("buffer is read-only"));
        }
        let mut tmp = vec![0.0_f64; buf.item_count()];
        buf.copy_to_slice(py, &mut tmp)?;
        self.inner.apply_in_place(&mut tmp);
        buf.copy_from_slice(py, &tmp)?;
        Ok(())
    }
}

/// One-shot: `convert(value, "m/s", "km/h")`.
#[pyfunction]
fn convert(value: f64, from_unit: &str, to_unit: &str) -> PyResult<f64> {
    Ok(reg()?
        .conversion(from_unit, to_unit)
        .map_err(|e| map_err(&e))?
        .apply(value))
}

#[pymodule]
fn _dimvexa_spike(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Converter>()?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add("DimvexaError", m.py().get_type::<DimvexaError>())?;
    Ok(())
}
