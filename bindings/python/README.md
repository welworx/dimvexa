<p align="center"><img src="https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-256.png" alt="Dimvexa" width="96" height="96"></p>

# dimvexa (Python)

Python binding for [Dimvexa](https://github.com/welworx/dimvexa): *Exact factors. Loud failures.
Zero guessing.* A thin PyO3 layer; all conversion logic is in the Rust core.

```python
import dimvexa

dimvexa.convert(10.0, "m/s", "km/h")  # 36.0
c = dimvexa.conversion("m", "km")  # parse once ...
c(numpy_array)  # ... apply to float64 arrays, lists or floats

try:
    dimvexa.convert(1.0, "gal", "L")
except dimvexa.DimvexaError as e:
    e.code, e.candidates  # 'ambiguous_symbol', ['gal_us', 'gal_imp']
```

Configs, densities and constants work as in the CLI (`dimvexa-config` does the loading):

```python
dimvexa.convert("4pi", "rad", "deg")  # 720.0: a string is an exact value (pi, * / ^, parentheses)
dimvexa.convert("pi/2", "rad", "deg"), dimvexa.quantity("2*pi rad").to("deg")  # 90.0, 360 deg
dimvexa.convert(1.0, "ton", "kg", configs=["us"])  # 907.18474; a name or a file path
dimvexa.convert(1.0, "L", "kg", density="998.2067kg/m3")  # never assumed, only when asked
dimvexa.convert(1.0, "L", "kg", configs=["water"], density="water_25C")
dimvexa.constants(config_texts=[toml_text])  # pi, h_P, e, k_B, N_A, then your own
dimvexa.quantity("1 L", density="998.2067kg/m3").to("kg")  # exact; quantity() takes the same options
dimvexa.units(configs=["us"]), dimvexa.unit_info("bbl_oil", config_texts=[toml_text])  # config units too
```

Every refusal is a `DimvexaError` (a `ValueError`) with a stable `code`, `suggestions` and
`candidates`. `exact=True` routes through the exact rational path, rounded once. A string value is always exact; `1/2pi` is refused as ambiguous (`ambiguous_expression`), write `pi/2` or `1/(2*pi)`.

## Build and test

This crate is its own workspace (it needs PyO3 and NumPy from the registry; the main workspace is
std-only and offline). It is not part of `cargo xtask ci`.

```console
cd bindings/python
python -m venv .venv && . .venv/bin/activate
pip install maturin pytest numpy
maturin develop
pytest
```

Wheels are `abi3` (Python 3.9+). No performance claim is made until a dated benchmark run exists
(`docs/benchmarking.md`).
