// SPDX-License-Identifier: MIT OR Apache-2.0
//! The prepared-conversion path must not allocate (plan section 5.3).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::missing_const_for_fn,
    clippy::if_not_else,
    clippy::similar_names,
    clippy::doc_markdown,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::needless_pass_by_value,
    clippy::items_after_statements,
    clippy::unreadable_literal,
    clippy::redundant_closure_for_method_calls
)]

use std::hint::black_box;

use dimvexa::{Conversion, Ratio, registry};
use dimvexa_testkit::alloc_counter::{CountingAlloc, measure};

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

#[test]
fn prepared_conversion_paths_do_not_allocate() {
    // Setup allocates freely: parsing, composing, buffers.
    let reg = registry().unwrap();
    let linear = reg.conversion("m/s", "km/h").unwrap();
    let affine = reg.conversion("degF", "degC").unwrap();
    let input: Vec<f64> = (0..10_000).map(f64::from).collect();
    let mut output = vec![0.0_f64; input.len()];
    let mut data = input.clone();

    // Control: the counter really sees allocations.
    let (_, control) = measure(|| black_box(vec![0_u8; 64]));
    assert_eq!(control.allocations, 1, "counting allocator is not counting");
    let (_, v) = measure(|| linear.apply_vec(black_box(&input)));
    assert_eq!(
        v.allocations, 1,
        "apply_vec allocates exactly once (documented)"
    );

    let (sum, usage) = measure(|| {
        let mut acc = 0.0;
        for i in 0..1_000_000_u32 {
            acc += linear.apply(black_box(f64::from(i)));
            acc += affine.apply(black_box(f64::from(i)));
        }
        acc
    });
    assert!(sum.is_finite());
    assert_eq!(
        usage.allocations, 0,
        "scalar apply allocated {} bytes",
        usage.bytes
    );

    let (res, usage) = measure(|| {
        let a = linear.apply_slice(black_box(&input), black_box(&mut output));
        let b = affine.apply_slice(black_box(&input), black_box(&mut output));
        linear.apply_in_place(black_box(&mut data));
        affine.apply_in_place(black_box(&mut data));
        // The error path (length mismatch) must not allocate either.
        let c = linear.apply_slice(black_box(&input), black_box(&mut output[..3]));
        (a, b, c)
    });
    assert!(res.0.is_ok() && res.1.is_ok() && res.2.is_err());
    assert_eq!(
        usage.allocations, 0,
        "slice paths allocated {} bytes",
        usage.bytes
    );

    let exact = Conversion::linear(Ratio::new(18, 5).unwrap()).unwrap();
    let (r, usage) = measure(|| exact.apply_exact(black_box(Ratio::from_int(10))));
    assert_eq!(r.unwrap(), Ratio::from_int(36));
    assert_eq!(usage.allocations, 0, "exact path allocated");
}
