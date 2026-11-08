"""Smoke test for the ADR-010 spike: scalar and batch results must be bit-identical to the Rust
result. The Rust reference is produced by the `reference` example (same Conversion, same inputs).
Usage: python tests/smoke.py   (after `maturin develop --release`)."""
import array
import struct
import subprocess
from pathlib import Path

import dimvexa_spike as dv

ROOT = Path(__file__).resolve().parent.parent
CASES = [("m/s", "km/h"), ("psi", "kPa"), ("degC", "degF")]


def lcg(n):
    x, out = 12345, []
    for _ in range(n):
        x = (x * 1664525 + 1013904223) & 0xFFFFFFFF
        out.append(x / 2**32 * 2000.0 - 500.0)
    return out


def bits(v):
    return struct.unpack("<Q", struct.pack("<d", v))[0]


def rust_reference(n):
    out = subprocess.run(
        ["cargo", "run", "-q", "--release", "--example", "reference", "--", str(n)],
        cwd=ROOT, check=True, capture_output=True, text=True).stdout
    return {tuple(l.split()[:2]): [int(h, 16) for h in l.split()[2:]] for l in out.splitlines()}


def raises(exc, fn):
    try:
        fn()
    except exc:
        return
    raise AssertionError("no error")


N = 1000
ref = rust_reference(N)
xs = lcg(N)
for f, t in CASES:
    c = dv.Converter(f, t)
    want = ref[(f, t)]
    assert [bits(c.apply(x)) for x in xs] == want, f"scalar {f}->{t}"
    assert [bits(v) for v in c.apply_slice(array.array("d", xs))] == want, f"slice {f}->{t}"
    buf = array.array("d", xs)
    c.apply_in_place(buf)
    assert [bits(v) for v in buf] == want, f"in_place {f}->{t}"
    assert bits(dv.convert(xs[0], f, t)) == want[0]

# error mapping: stable code and suggestions survive
try:
    dv.Converter("m", "kg")
except dv.DimvexaError as e:
    assert e.code == "dimension_mismatch", e.code
else:
    raise AssertionError("no error")
try:
    dv.Converter("ton", "kg")
except dv.DimvexaError as e:
    assert e.code == "ambiguous_symbol" and e.suggestions, (e.code, e.suggestions)
else:
    raise AssertionError("no error")

# buffer checks: wrong dtype, read-only, non-contiguous
c = dv.Converter("m/s", "km/h")
raises((TypeError, ValueError, BufferError), lambda: c.apply_slice(array.array("f", [1.0])))
raises((TypeError, ValueError, BufferError), lambda: c.apply_slice(array.array("i", [1])))
raises((TypeError, ValueError, BufferError),
       lambda: c.apply_in_place(memoryview(array.array("d", [1.0, 2.0])).toreadonly()))
raises((TypeError, ValueError, BufferError),
       lambda: c.apply_slice(memoryview(array.array("d", [1.0, 2.0, 3.0]))[::2]))

try:
    import numpy as np
except ImportError:
    print("numpy not installed: numpy checks skipped")
else:
    a = np.array(xs)
    c = dv.Converter("psi", "kPa")
    assert [bits(v) for v in c.apply_slice(a)] == ref[("psi", "kPa")]
    c.apply_in_place(a)
    assert [bits(v) for v in a] == ref[("psi", "kPa")]
    raises((TypeError, ValueError, BufferError), lambda: c.apply_slice(np.array(xs, dtype=np.float32)))
    raises((TypeError, ValueError, BufferError), lambda: c.apply_slice(np.array(xs)[::2]))
    print("numpy float64 ok")
print("smoke ok: scalar, apply_slice, apply_in_place bit-identical to Rust for", len(CASES), "conversions")
