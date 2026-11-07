//! iai-callgrind instruction-count benchmarks (unverified; needs valgrind and the runner).
use dimvexa_data::registry;
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

#[library_benchmark]
fn scalar_apply() -> f64 {
    let conv = registry().unwrap().conversion("m/s", "km/h").unwrap();
    black_box(conv.apply(black_box(10.0)))
}

library_benchmark_group!(name = g; benchmarks = scalar_apply);
main!(library_benchmark_groups = g);
