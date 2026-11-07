#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Runner for astropy.units. NOT yet run: astropy could not be installed where this was written.

Modes: "plain" is the Quantity API with `u.temperature()` equivalencies supplied (that is what
astropy requires for degC/degF); "unit_to" is `Unit.to(other, value)`, which skips Quantity
creation. The imperial units are enabled once with `u.add_enabled_units(u.imperial)`.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402
from common import Unsupported  # noqa: E402


class Astropy(common.Adapter):
    name = "astropy"
    import_name = "astropy.units"
    modes = {
        "plain": {"idiomatic": "plain", "description": "(x * Unit).to(Unit, equivalencies=temperature()).value"},
        "unit_to": {"idiomatic": "fast", "description": "from_unit.to(to_unit, value, equivalencies=temperature())"},
    }

    def __init__(self):
        import astropy.units as u
        self.u = u
        u.add_enabled_units(u.imperial)
        self.eq = u.temperature()
        self.cfg = common.load_translation("astropy")

    def version(self):
        return common.dist_version("astropy")

    def package_bytes(self):
        return common.dist_size("astropy")

    def translated(self, expr):
        return common.translate_expr(expr, self.cfg)

    def prepare(self, mode, conv, state=None):
        u = self.u
        return (u.Unit(self.translated(conv["from"])), u.Unit(self.translated(conv["to"])))

    def scalar_fn(self, mode, h, x):
        fu, tu = h
        eq = self.eq
        if mode == "plain":
            Q = self.u.Quantity
            return lambda: Q(x, fu).to(tu, equivalencies=eq).value
        return lambda: fu.to(tu, x, equivalencies=eq)

    def make_batch_input(self, mode, values):
        import numpy as np
        return np.array(values, dtype=np.float64)

    def batch_fn(self, mode, h, data):
        fu, tu = h
        eq = self.eq
        if mode == "plain":
            Q = self.u.Quantity
            return lambda: Q(data, fu).to(tu, equivalencies=eq).value
        return lambda: fu.to(tu, data, equivalencies=eq)

    def oneshot_fn(self, mode, conv):
        v, un = common.split_quantity(conv["oneshot"])
        text = v + " " + self.translated(un)
        to = self.translated(conv["to"])
        Q = self.u.Quantity
        eq = self.eq
        return lambda: Q(text).to(to, equivalencies=eq).value

    def convert_case(self, mode, value, src, dst):
        u = self.u
        return u.Quantity(float(value), u.Unit(self.translated(src))).to(
            u.Unit(self.translated(dst)), equivalencies=self.eq).value

    def add_case(self, mode, a, b):
        def q(t):
            v, un = common.split_quantity(t)
            return self.u.Quantity(float(v), self.u.Unit(self.translated(un)))
        return (q(a) + q(b)).value

    def knows_unit(self, expr):
        self.u.Unit(self.translated(expr), parse_strict="raise")
        return True


if __name__ == "__main__":
    sys.exit(common.main(Astropy()))
