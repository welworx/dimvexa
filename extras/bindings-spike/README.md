# bindings-spike (experimental, ADR-010)

Throwaway PyO3 prototype for `docs/adr/010-bindings.md`. Own workspace, not built by
`cargo xtask ci`, no core crate depends on it. Needs network (crates.io, PyPI).

```console
python3 -m venv .venv && . .venv/bin/activate
pip install maturin numpy        # numpy is optional; the smoke test skips its part without it
maturin develop --release
python tests/smoke.py            # bit-identical to `cargo run --example reference`
```

Not for release: no stubs, no wheels CI, `apply_in_place` copies (see the doc comment).
