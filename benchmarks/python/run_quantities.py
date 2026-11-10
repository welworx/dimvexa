#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Runner for python-quantities (PyPI `quantities`, imported as `quantities`). Run in the fourth comparison
(docs/benchmarks/comparison-run-4.md).

Modes: "plain" is `Quantity(x, "from").rescale("to").magnitude` with unit strings (the README form; one array
Quantity for batches); "units" builds the unit objects once (`Quantity(1, "from").units`) and passes them to
`Quantity(x, unit)` / `rescale(unit)`. The library has no prepared converter and no string quantity parser
("10 m/s"), so one-shot is reported as unsupported. It has no affine (offset) temperature units: degC and
degF are scale-only units there, and the runner does not add an offset.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402
from common import Unsupported  # noqa: E402


class Quantities(common.Adapter):
    name = "quantities"
    import_name = "quantities"
    modes = {
        "plain": {"idiomatic": "plain", "description": "Quantity(x, from).rescale(to).magnitude with unit strings; one array Quantity for batches"},
        "units": {"idiomatic": "fast", "description": "unit objects built once (Quantity(1, from).units), then Quantity(x, unit).rescale(unit)"},
    }

    def __init__(self):
        import quantities as pq
        self.pq = pq
        self.cfg = common.load_translation("quantities")

    def version(self):
        return common.dist_version("quantities")

    def package_bytes(self):
        return common.dist_size("quantities")

    def translated(self, expr):
        return common.translate_expr(expr, self.cfg)

    def prepare(self, mode, conv, state=None):
        pq = self.pq
        fu, tu = self.translated(conv["from"]), self.translated(conv["to"])
        if mode == "plain":
            pq.Quantity(1.0, fu).rescale(tu)
            return (fu, tu)
        return (pq.Quantity(1.0, fu).units, pq.Quantity(1.0, tu).units)

    def scalar_fn(self, mode, h, x):
        fu, tu = h
        Q = self.pq.Quantity
        return lambda: Q(x, fu).rescale(tu).magnitude

    def make_batch_input(self, mode, values):
        import numpy as np
        return np.array(values, dtype=np.float64)

    def batch_fn(self, mode, h, data):
        fu, tu = h
        Q = self.pq.Quantity
        return lambda: Q(data, fu).rescale(tu).magnitude

    def oneshot_fn(self, mode, conv):
        raise Unsupported("quantities has no string quantity parser")

    def convert_case(self, mode, value, src, dst):
        return self.pq.Quantity(float(value), self.translated(src)).rescale(self.translated(dst)).magnitude

    def add_case(self, mode, a, b):
        def q(t):
            v, u = common.split_quantity(t)
            return self.pq.Quantity(float(v), self.translated(u))
        return (q(a) + q(b)).magnitude

    def knows_unit(self, expr):
        self.pq.Quantity(1.0, self.translated(expr))
        return True


if __name__ == "__main__":
    sys.exit(common.main(Quantities()))
