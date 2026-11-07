#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Ceiling runners: raw float multiply in pure Python, and NumPy multiply. Speed only; they know
no units, they receive the factor from the corpus. They show what the interpreter itself costs."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402


class PurePython(common.Adapter):
    name = "python-float-ceiling"
    kind = "ceiling"
    modes = {"loop": {"idiomatic": "fast", "description": "x * a + b in a Python list comprehension (pure Python floats)"}}

    def version(self):
        return common.platform.python_version()

    def prepare(self, mode, conv, state=None):
        return (conv["factor"], conv["offset"])

    def scalar_fn(self, mode, h, x):
        a, b = h
        return lambda: x * a + b

    def batch_fn(self, mode, h, data):
        a, b = h
        return lambda: [v * a + b for v in data]


class NumpyCeiling(common.Adapter):
    name = "numpy-ceiling"
    kind = "ceiling"
    import_name = "numpy"
    modes = {"multiply": {"idiomatic": "fast", "description": "arr * a + b with NumPy float64 arrays (two passes)"}}

    def version(self):
        import numpy
        return numpy.__version__

    def prepare(self, mode, conv, state=None):
        return (conv["factor"], conv["offset"])

    def scalar_fn(self, mode, h, x):
        import numpy as np
        a, b = h
        xs = np.float64(x)
        return lambda: xs * a + b

    def make_batch_input(self, mode, values):
        import numpy as np
        return np.array(values, dtype=np.float64)

    def batch_fn(self, mode, h, data):
        a, b = h
        return lambda: data * a + b


if __name__ == "__main__":
    which = sys.argv.pop(1) if len(sys.argv) > 1 and sys.argv[1] in ("python", "numpy") else "numpy"
    sys.exit(common.main(PurePython() if which == "python" else NumpyCeiling()))
