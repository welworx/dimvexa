#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Runner for Dimvexa's own Python binding (bindings/python, ADR-012), run like the competitor
libraries so the same corpus, tolerance and timers apply. Build and install it first
(`maturin develop --release` in bindings/python, inside the benchmark venv).

Modes: "plain" is `dimvexa.convert(x, from, to)` (parses both unit strings on every call; a NumPy
array for batches); "conv" is one `dimvexa.conversion(from, to)` applied to a float or to a
float64 NumPy array (one call per array). Accuracy and safety use "plain".
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402


class DimvexaPy(common.Adapter):
    name = "dimvexa-py"
    import_name = "dimvexa"
    modes = {
        "plain": {"idiomatic": "plain", "description": "convert(x, from, to) per call; one NumPy array for batches"},
        "conv": {"idiomatic": "fast", "description": "conversion(from, to) once, then conv(x) / conv(float64 array)"},
    }

    def __init__(self):
        import dimvexa
        self.d = dimvexa

    def version(self):
        return f"{self.d.__version__}+{self.d.data_version()}"

    def package_bytes(self):
        return common.dist_size("dimvexa")

    def prepare(self, mode, conv, state=None):
        if mode == "plain":
            self.d.convert(1.0, conv["from"], conv["to"])  # fail early on a bad spelling
            return (conv["from"], conv["to"])
        return self.d.conversion(conv["from"], conv["to"])

    def scalar_fn(self, mode, h, x):
        if mode == "plain":
            f, t = h
            convert = self.d.convert
            return lambda: convert(x, f, t)
        return lambda: h(x)

    def make_batch_input(self, mode, values):
        import numpy as np
        return np.array(values, dtype=np.float64)

    def batch_fn(self, mode, h, data):
        if mode == "plain":
            f, t = h
            convert = self.d.convert
            return lambda: convert(data, f, t)
        return lambda: h(data)

    def oneshot_fn(self, mode, conv):
        text, to, quantity = conv["oneshot"], conv["to"], self.d.quantity
        return lambda: quantity(text).to(to).to_float()

    def convert_case(self, mode, value, src, dst):
        return self.d.convert(float(value), src, dst)

    def knows_unit(self, expr):
        self.d.convert(1.0, expr, expr)
        return True


if __name__ == "__main__":
    sys.exit(common.main(DimvexaPy()))
