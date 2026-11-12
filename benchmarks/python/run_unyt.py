#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Runner for unyt. Run in the first comparison (docs/benchmarks/comparison/history/comparison-run-1.md).

Modes: "plain" is `unyt_quantity(x, from).to(to).value` (array: `unyt_array`); "to_value" uses
`.to_value(unit)`; "factor" uses `Unit.get_conversion_factor` (linear conversions only: unyt
returns an offset for temperature and that mode then reports unsupported).
"""
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402
from common import Unsupported  # noqa: E402


class Unyt(common.Adapter):
    name = "unyt"
    import_name = "unyt"
    modes = {
        "plain": {"idiomatic": "plain", "description": "unyt_quantity(x, from).to(to).value; unyt_array for batches"},
        "to_value": {"idiomatic": "fast", "description": "unyt_quantity(x, from).to_value(to)"},
        "factor": {"idiomatic": "fast", "description": "Unit.get_conversion_factor once, then x*factor (linear only)"},
    }

    def __init__(self):
        import unyt
        self.unyt = unyt
        self.cfg = common.load_translation("unyt")

    def version(self):
        return common.dist_version("unyt")

    def package_bytes(self):
        return common.dist_size("unyt")

    def translated(self, expr):
        return common.translate_expr(expr, self.cfg)

    def prepare(self, mode, conv, state=None):
        u = self.unyt
        fu, tu = u.Unit(self.translated(conv["from"])), u.Unit(self.translated(conv["to"]))
        if mode in ("plain", "to_value"):
            return (fu, tu)
        if mode == "factor":
            f, off = fu.get_conversion_factor(tu)
            if off is not None and off != 0:
                raise Unsupported("unyt reports an offset (temperature); factor mode is linear only")
            return float(f)
        raise Unsupported(mode)

    def scalar_fn(self, mode, h, x):
        q = self.unyt.unyt_quantity
        if mode == "plain":
            fu, tu = h
            return lambda: q(x, fu).to(tu).value
        if mode == "to_value":
            fu, tu = h
            return lambda: q(x, fu).to_value(tu)
        return lambda: x * h

    def make_batch_input(self, mode, values):
        import numpy as np
        return np.array(values, dtype=np.float64)

    def batch_fn(self, mode, h, data):
        arr = self.unyt.unyt_array
        if mode == "plain":
            fu, tu = h
            return lambda: arr(data, fu).to(tu).value
        if mode == "to_value":
            fu, tu = h
            return lambda: arr(data, fu).to_value(tu)
        return lambda: data * h

    def oneshot_fn(self, mode, conv):
        v, u = common.split_quantity(conv["oneshot"])
        tu = self.translated(u)
        # unyt_quantity.from_string rejects parentheses ('1 kJ/(kg*K)' is an invalid expression,
        # checked on unyt 3.2.0) but accepts the flattened form '1 kJ/kg/K'; use that for a/(b*c).
        m = re.fullmatch(r"(.+?)/\((.+)\)", tu)
        if m:
            tu = m.group(1) + "/" + "/".join(m.group(2).split("*"))
        text = v + " " + tu
        to = self.translated(conv["to"])
        q = self.unyt.unyt_quantity
        if not hasattr(q, "from_string"):
            raise Unsupported("unyt_quantity.from_string not available in this version")
        return lambda: q.from_string(text).to_value(to)

    def convert_case(self, mode, value, src, dst):
        q = self.unyt.unyt_quantity(float(value), self.translated(src))
        return q.to(self.translated(dst)).value

    def add_case(self, mode, a, b):
        def q(t):
            v, u = common.split_quantity(t)
            return self.unyt.unyt_quantity(float(v), self.translated(u))
        return (q(a) + q(b)).value

    def knows_unit(self, expr):
        self.unyt.Unit(self.translated(expr))
        return True


if __name__ == "__main__":
    sys.exit(common.main(Unyt()))
