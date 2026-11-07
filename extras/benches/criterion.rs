//! Criterion version of the M0 benchmarks (unverified: criterion could not be installed).
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dimvexa_data::registry;

fn bench(c: &mut Criterion) {
    let reg = registry().unwrap();
    let conv = reg.conversion("m/s", "km/h").unwrap();
    c.bench_function("scalar apply", |b| b.iter(|| conv.apply(black_box(10.0))));
    let input = vec![1.0_f64; 1_000_000];
    let mut out = vec![0.0_f64; 1_000_000];
    c.bench_function("slice 1e6", |b| {
        b.iter(|| conv.apply_slice(black_box(&input), &mut out).unwrap())
    });
    c.bench_function("parse psi", |b| b.iter(|| reg.parse(black_box("psi")).unwrap()));
}

criterion_group!(benches, bench);
criterion_main!(benches);
