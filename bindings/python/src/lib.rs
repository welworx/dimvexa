// SPDX-License-Identifier: MIT OR Apache-2.0
//! Python binding for Dimvexa. A thin layer: all conversion logic lives in the Rust core.
//!
//! Every core error becomes a [`DimvexaError`] carrying the stable `code`, the `suggestions`
//! and the `candidates`, so Python callers branch on codes, never on message text.

use std::sync::OnceLock;

use ::dimvexa::{Conversion as CoreConversion, Quantity as CoreQuantity, Registry};
use numpy::{PyArrayDyn, PyArrayMethods, PyReadonlyArrayDyn, PyUntypedArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

pyo3::create_exception!(
    _dimvexa,
    DimvexaError,
    PyValueError,
    "A Dimvexa refusal. Attributes: `code`, `suggestions`, `candidates`."
);

fn to_py(err: &::dimvexa::Error) -> PyErr {
    let py_err = DimvexaError::new_err(err.to_string());
    Python::attach(|py| {
        let value = py_err.value(py);
        // Attribute failures on a fresh exception object cannot be reported more usefully
        // than the error itself, so they are ignored.
        let _ = value.setattr("code", err.code());
        let _ = value.setattr("suggestions", err.suggestions().to_vec());
        let _ = value.setattr("candidates", err.candidates().to_vec());
    });
    py_err
}

fn registry() -> PyResult<&'static Registry> {
    static REG: OnceLock<Result<Registry, ::dimvexa::Error>> = OnceLock::new();
    match REG.get_or_init(::dimvexa::registry) {
        Ok(reg) => Ok(reg),
        Err(e) => Err(to_py(e)),
    }
}

/// A pre-composed conversion between two units (parse once, apply many times).
#[pyclass(module = "dimvexa", frozen)]
struct Conversion {
    inner: CoreConversion,
}

#[pymethods]
impl Conversion {
    /// Shape of the conversion: `linear` or `affine`.
    #[getter]
    fn shape(&self) -> String {
        self.inner.shape().to_string()
    }

    /// The exact factor as a fraction string.
    #[getter]
    fn factor(&self) -> String {
        self.inner.factor().to_fraction_string()
    }

    /// The exact offset as a fraction string.
    #[getter]
    fn offset(&self) -> String {
        self.inner.offset().to_fraction_string()
    }

    /// Apply to a float, a sequence of floats or a NumPy `float64` array.
    fn __call__<'py>(&self, py: Python<'py>, x: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
        apply(py, &self.inner, x, false)
    }

    /// Apply to one float through the exact rational path, rounded once.
    fn apply_exact(&self, x: f64) -> PyResult<f64> {
        self.inner.apply_exact_f64(x).map_err(|e| to_py(&e))
    }

    /// The inverse conversion.
    fn inverse(&self) -> PyResult<Self> {
        self.inner
            .inverse()
            .map(|inner| Self { inner })
            .map_err(|e| to_py(&e))
    }

    fn __repr__(&self) -> String {
        format!(
            "Conversion({}, factor={}, offset={})",
            self.inner.shape(),
            self.inner.factor().to_fraction_string(),
            self.inner.offset().to_fraction_string()
        )
    }
}

/// An exact value with a unit.
#[pyclass(module = "dimvexa", frozen)]
struct Quantity {
    inner: CoreQuantity,
}

#[pymethods]
impl Quantity {
    /// The exact value as a fraction string.
    #[getter]
    fn value(&self) -> String {
        self.inner.value().to_fraction_string()
    }

    /// The unit as text.
    #[getter]
    fn unit(&self) -> String {
        self.inner.unit().to_string()
    }

    /// Re-express in another unit, exactly.
    fn to(&self, unit: &str) -> PyResult<Self> {
        let reg = registry()?;
        let target = reg.parse(unit).map_err(|e| to_py(&e))?;
        self.inner
            .to_unit(&target)
            .map(|inner| Self { inner })
            .map_err(|e| to_py(&e))
    }

    /// The value rounded once to a float.
    fn to_float(&self) -> PyResult<f64> {
        self.inner.to_f64().map_err(|e| to_py(&e))
    }

    fn __add__(&self, other: &Self) -> PyResult<Self> {
        self.inner
            .checked_add(&other.inner)
            .map(|inner| Self { inner })
            .map_err(|e| to_py(&e))
    }

    fn __sub__(&self, other: &Self) -> PyResult<Self> {
        self.inner
            .checked_sub(&other.inner)
            .map(|inner| Self { inner })
            .map_err(|e| to_py(&e))
    }

    fn __mul__(&self, other: &Self) -> PyResult<Self> {
        self.inner
            .checked_mul(&other.inner)
            .map(|inner| Self { inner })
            .map_err(|e| to_py(&e))
    }

    fn __truediv__(&self, other: &Self) -> PyResult<Self> {
        self.inner
            .checked_div(&other.inner)
            .map(|inner| Self { inner })
            .map_err(|e| to_py(&e))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Quantity('{}')", self.inner)
    }
}

fn apply(
    py: Python<'_>,
    conv: &CoreConversion,
    x: &Bound<'_, PyAny>,
    exact: bool,
) -> PyResult<Py<PyAny>> {
    if let Ok(v) = x.extract::<f64>() {
        let out = if exact {
            conv.apply_exact_f64(v).map_err(|e| to_py(&e))?
        } else {
            conv.apply(v)
        };
        return Ok(out.into_pyobject(py)?.into_any().unbind());
    }
    if let Ok(arr) = x.extract::<PyReadonlyArrayDyn<'_, f64>>() {
        let out = PyArrayDyn::<f64>::zeros(py, arr.shape(), false);
        {
            let mut dst = out.readwrite();
            let src = arr.as_array();
            for (o, i) in dst.as_array_mut().iter_mut().zip(src.iter()) {
                *o = if exact {
                    conv.apply_exact_f64(*i).map_err(|e| to_py(&e))?
                } else {
                    conv.apply(*i)
                };
            }
        }
        return Ok(out.into_any().unbind());
    }
    let seq: Vec<f64> = x.extract().map_err(|_| {
        PyValueError::new_err("expected a float, a sequence of floats or a float64 NumPy array")
    })?;
    let out: Vec<f64> = if exact {
        seq.iter()
            .map(|v| conv.apply_exact_f64(*v).map_err(|e| to_py(&e)))
            .collect::<PyResult<_>>()?
    } else {
        conv.apply_vec(&seq)
    };
    Ok(out.into_pyobject(py)?.into_any().unbind())
}

/// Convert a float, a sequence or a NumPy array from one unit to another.
#[pyfunction]
#[pyo3(signature = (value, from_unit, to_unit, *, exact = false))]
fn convert(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
    from_unit: &str,
    to_unit: &str,
    exact: bool,
) -> PyResult<Py<PyAny>> {
    let conv = registry()?
        .conversion(from_unit, to_unit)
        .map_err(|e| to_py(&e))?;
    apply(py, &conv, value, exact)
}

/// Compose a conversion once for repeated use.
#[pyfunction]
fn conversion(from_unit: &str, to_unit: &str) -> PyResult<Conversion> {
    registry()?
        .conversion(from_unit, to_unit)
        .map(|inner| Conversion { inner })
        .map_err(|e| to_py(&e))
}

/// Parse a quantity such as `"20 degC"`.
#[pyfunction]
fn quantity(text: &str) -> PyResult<Quantity> {
    registry()?
        .quantity(text)
        .map(|inner| Quantity { inner })
        .map_err(|e| to_py(&e))
}

/// All unit symbols in the table.
#[pyfunction]
fn units() -> PyResult<Vec<String>> {
    Ok(registry()?
        .units()
        .iter()
        .map(|u| u.symbol().to_owned())
        .collect())
}

/// Details of one unit, or `None` if the symbol is not a table unit.
#[pyfunction]
fn unit_info<'py>(py: Python<'py>, symbol: &str) -> PyResult<Option<Bound<'py, PyDict>>> {
    let Some(u) = registry()?.get(symbol) else {
        return Ok(None);
    };
    let d = PyDict::new(py);
    d.set_item("symbol", u.symbol())?;
    d.set_item("aliases", u.aliases().to_vec())?;
    d.set_item("dimension", u.dim().to_string())?;
    d.set_item("kind", u.kind().to_string())?;
    d.set_item("shape", u.shape().to_string())?;
    d.set_item("factor", u.factor().to_fraction_string())?;
    d.set_item("offset", u.offset().to_fraction_string())?;
    d.set_item("exactness", u.exactness().to_string())?;
    d.set_item("namespace", u.namespace())?;
    d.set_item("source", u.source())?;
    d.set_item("note", u.note())?;
    Ok(Some(d))
}

/// The unit table identifier, `v<version>+<hash>`.
#[pyfunction]
fn data_version() -> String {
    ::dimvexa::data_version()
}

#[pymodule]
fn _dimvexa(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("DimvexaError", m.py().get_type::<DimvexaError>())?;
    m.add_class::<Conversion>()?;
    m.add_class::<Quantity>()?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_function(wrap_pyfunction!(conversion, m)?)?;
    m.add_function(wrap_pyfunction!(quantity, m)?)?;
    m.add_function(wrap_pyfunction!(units, m)?)?;
    m.add_function(wrap_pyfunction!(unit_info, m)?)?;
    m.add_function(wrap_pyfunction!(data_version, m)?)?;
    Ok(())
}
