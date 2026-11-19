// SPDX-License-Identifier: MIT OR Apache-2.0
//! Time of `Ratio::to_f64` for the kinds of value in `docs/how-numbers-work.md`, section 7.
//!
//! ```console
//! cargo run --release -p dimvexa-testkit --example ratio_to_f64_bench
//! ```
//!
//! Each row is the best of five runs of 400 000 calls (after a warm-up), in nanoseconds per
//! call. It is a micro benchmark of one function on one machine: read differences of a few
//! percent as noise, and use `cargo xtask bench-impact` for the guarded rows of a change.
#![allow(clippy::print_stdout, clippy::cast_precision_loss)]

use std::hint::black_box;
use std::time::Instant;

use dimvexa_core::Ratio;

fn row(name: &str, value: Ratio) {
    const CALLS: u32 = 400_000;
    for _ in 0..20_000 {
        black_box(black_box(value).to_f64().ok());
    }
    let mut best = f64::MAX;
    for _ in 0..5 {
        let start = Instant::now();
        for _ in 0..CALLS {
            black_box(black_box(value).to_f64().ok());
        }
        best = best.min(start.elapsed().as_nanos() as f64 / f64::from(CALLS));
    }
    println!("{name:58} {best:8.1} ns");
}

fn main() {
    let Ok(plain) = Ratio::new(15, 1000) else {
        return;
    };
    let ten =
        |num: i128, den: i128, exp: i32| Ratio::new(num, den).map(|r| r.with_exponents(exp, 0));
    let (Ok(folds), Ok(bounds), Ok(with_den), Ok(pi)) = (
        ten(1_602_176_634, 1, -28),
        ten(662_607_015, 1, -42),
        ten(6, 7, -50),
        Ratio::new(1, 180).map(|r| r.with_exponents(0, 1)),
    ) else {
        return;
    };
    row("plain fraction 15/1000", plain);
    row("power of ten that folds (1602176634e-28)", folds);
    row("power of ten that needs the bounds (662607015e-42)", bounds);
    row("power of ten with a denominator (6/7 e-50)", with_den);
    row("power of pi (1/180 pi)", pi);
}
