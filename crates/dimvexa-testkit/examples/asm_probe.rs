//! Non-inlined wrappers around the batch conversion loops, so the generated assembly can be
//! inspected. See `docs/benchmarks/M0.md` for how it is used.
#![allow(clippy::missing_docs_in_private_items, missing_docs)]

use dimvexa_core::{Conversion, Ratio};

#[inline(never)]
pub fn probe_apply_slice(c: &Conversion, input: &[f64], output: &mut [f64]) -> bool {
    c.apply_slice(input, output).is_ok()
}

#[inline(never)]
pub fn probe_apply_in_place(c: &Conversion, data: &mut [f64]) {
    c.apply_in_place(data);
}

fn main() {
    // Calls the probes once with empty slices so they are neither inlined nor removed.
    let Ok(factor) = Ratio::new(18, 5) else {
        return;
    };
    let Ok(c) = Conversion::linear(factor) else {
        return;
    };
    let mut out = [0.0_f64; 0];
    let ok = std::hint::black_box(probe_apply_slice(
        std::hint::black_box(&c),
        std::hint::black_box(&[]),
        std::hint::black_box(&mut out),
    ));
    probe_apply_in_place(&c, std::hint::black_box(&mut out));
    std::hint::black_box(ok);
}
