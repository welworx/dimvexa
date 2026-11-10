#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Runner for ucumvert (PyPI; UCUM unit strings parsed with lark and turned into pint units). Run in the fourth
comparison (docs/benchmarks/comparison-run-4.md).

Unit names are UCUM codes (case-sensitive), mapped from Dimvexa's names in translations/ucumvert.json.
Modes: "plain" is `ureg.from_ucum(code)` for both units on every call, then pint (`Quantity(x, unit).to(unit)`);
"prepared" parses the UCUM codes once (`ureg.from_ucum(code).units`) and then only runs pint conversions.
ucumvert converts UCUM to pint only; arithmetic and conversion are pint's, so safety behaviour is pint's.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402
from common import Unsupported  # noqa: E402


class Ucumvert(common.Adapter):
    name = "ucumvert"
    import_name = "ucumvert"
    modes = {
        "plain": {"idiomatic": "plain", "description": "ureg.from_ucum(code) for both units per call, then Quantity(x, unit).to(unit)"},
        "prepared": {"idiomatic": "fast", "description": "UCUM codes parsed once to pint units (from_ucum(code).units), then pint Quantity(x, unit).to(unit)"},
    }

    def __init__(self):
        from ucumvert import PintUcumRegistry
        self.cls = PintUcumRegistry
        self.ureg = PintUcumRegistry()
        self.cfg = common.load_translation("ucumvert")

    def version(self):
        return common.dist_version("ucumvert")

    def package_bytes(self):
        return common.dist_size("ucumvert")

    def translated(self, expr):
        return common.translate_expr(expr, self.cfg)

    def new_state(self):
        return self.cls()

    def _unit(self, ureg, code):
        return ureg.from_ucum(code).units

    def prepare(self, mode, conv, state=None):
        ureg = state or self.ureg
        fu, tu = self.translated(conv["from"]), self.translated(conv["to"])
        if mode == "plain":
            ureg.from_ucum(fu); ureg.from_ucum(tu)
            return (ureg, fu, tu)
        return (ureg, self._unit(ureg, fu), self._unit(ureg, tu))

    def scalar_fn(self, mode, h, x):
        ureg, fu, tu = h
        Q = ureg.Quantity
        if mode == "plain":
            return lambda: Q(x, ureg.from_ucum(fu).units).to(ureg.from_ucum(tu).units).magnitude
        return lambda: Q(x, fu).to(tu).magnitude

    def make_batch_input(self, mode, values):
        import numpy as np
        return np.array(values, dtype=np.float64)

    def batch_fn(self, mode, h, data):
        ureg, fu, tu = h
        Q = ureg.Quantity
        if mode == "plain":
            return lambda: Q(data, ureg.from_ucum(fu).units).to(ureg.from_ucum(tu).units).magnitude
        return lambda: Q(data, fu).to(tu).magnitude

    def oneshot_fn(self, mode, conv):
        v, u = common.split_quantity(conv["oneshot"])
        fu = self.translated(u)
        to = self.translated(conv["to"])
        ureg = self.ureg
        x = float(v)
        return lambda: ureg.Quantity(x, ureg.from_ucum(fu).units).to(ureg.from_ucum(to).units).magnitude

    def convert_case(self, mode, value, src, dst):
        ureg = self.ureg
        return ureg.Quantity(float(value), self._unit(ureg, self.translated(src))).to(self._unit(ureg, self.translated(dst))).magnitude

    def add_case(self, mode, a, b):
        def q(t):
            v, u = common.split_quantity(t)
            return self.ureg.Quantity(float(v), self._unit(self.ureg, self.translated(u)))
        return (q(a) + q(b)).magnitude

    def knows_unit(self, expr):
        self._unit(self.ureg, self.translated(expr))
        return True


if __name__ == "__main__":
    sys.exit(common.main(Ucumvert()))
