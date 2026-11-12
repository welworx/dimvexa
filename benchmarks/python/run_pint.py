#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Runner for pint. Run in the first comparison (docs/benchmarks/comparison/history/comparison-run-1.md).

Modes: "plain" is the Quantity API (`Quantity(x, from).to(to).magnitude`, one Quantity holding a
NumPy array for batches); "factor" extracts the factor once and multiplies (pint's documented
advice for tight loops); "wraps" uses `UnitRegistry.wraps`. Accuracy and safety use "plain".
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402
from common import Unsupported  # noqa: E402


class Pint(common.Adapter):
    name = "pint"
    import_name = "pint"
    modes = {
        "plain": {"idiomatic": "plain", "description": "Quantity(x, from).to(to).magnitude; one array Quantity for batches"},
        "factor": {"idiomatic": "fast", "description": "convert 0 and 1 once with pint, then x*a+b in Python/NumPy"},
        "wraps": {"idiomatic": "fast", "description": "UnitRegistry.wraps on an identity function; Quantity in, magnitude out"},
    }

    def __init__(self):
        import pint
        self.pint = pint
        self.ureg = pint.UnitRegistry()
        self.cfg = common.load_translation("pint")

    def version(self):
        return common.dist_version("pint")

    def package_bytes(self):
        return common.dist_size("pint")

    def translated(self, expr):
        return common.translate_expr(expr, self.cfg)

    def new_state(self):
        return self.pint.UnitRegistry()

    def prepare(self, mode, conv, state=None):
        ureg = state or self.ureg
        fu, tu = self.translated(conv["from"]), self.translated(conv["to"])
        Q = ureg.Quantity
        if mode == "plain":
            return (ureg, ureg.Unit(fu), ureg.Unit(tu))
        if mode == "factor":
            y0 = Q(0.0, fu).to(tu).magnitude
            y1 = Q(1.0, fu).to(tu).magnitude
            return (y1 - y0, y0)
        if mode == "wraps":
            f = ureg.wraps(tu, (tu,))(lambda v: v)
            return (ureg, ureg.Unit(fu), f)
        raise Unsupported(mode)

    def scalar_fn(self, mode, h, x):
        if mode == "plain":
            ureg, fu, tu = h
            Q = ureg.Quantity
            return lambda: Q(x, fu).to(tu).magnitude
        if mode == "factor":
            a, b = h
            return lambda: x * a + b
        ureg, fu, f = h
        Q = ureg.Quantity
        return lambda: f(Q(x, fu)).magnitude

    def make_batch_input(self, mode, values):
        import numpy as np
        return np.array(values, dtype=np.float64)

    def batch_fn(self, mode, h, data):
        if mode == "plain":
            ureg, fu, tu = h
            Q = ureg.Quantity
            return lambda: Q(data, fu).to(tu).magnitude
        if mode == "factor":
            a, b = h
            return lambda: data * a + b
        ureg, fu, f = h
        Q = ureg.Quantity
        return lambda: f(Q(data, fu)).magnitude

    def oneshot_fn(self, mode, conv):
        v, u = common.split_quantity(conv["oneshot"])
        text = v + " " + self.translated(u)
        to = self.translated(conv["to"])
        Q = self.ureg.Quantity
        return lambda: Q(text).to(to).magnitude

    def convert_case(self, mode, value, src, dst):
        return self.ureg.Quantity(float(value), self.translated(src)).to(self.translated(dst)).magnitude

    def add_case(self, mode, a, b):
        def q(t):
            v, u = common.split_quantity(t)
            return self.ureg.Quantity(float(v), self.translated(u))
        return (q(a) + q(b)).magnitude

    def knows_unit(self, expr):
        self.ureg.Unit(self.translated(expr))
        return True


if __name__ == "__main__":
    sys.exit(common.main(Pint()))
