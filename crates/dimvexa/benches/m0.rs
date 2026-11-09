// SPDX-License-Identifier: MIT OR Apache-2.0
//! M0 benchmark suite (std-only harness; the criterion version lives in `extras/`).
//!
//! `cargo bench -p dimvexa --bench m0` prints a Markdown table: median, minimum and
//! maximum nanoseconds per operation over repeated samples. Nothing here is cached by the
//! library (plan section 6.2); "prepared" means the caller keeps the `Conversion` handle.
#![allow(
    clippy::suboptimal_flops,
    clippy::items_after_statements,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::missing_const_for_fn,
    clippy::doc_markdown,
    clippy::unreadable_literal,
    clippy::many_single_char_names,
    clippy::semicolon_if_nothing_returned,
    clippy::needless_range_loop,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::print_stdout
)]

use std::hint::black_box;
use std::io::Write as _;
use std::time::{Duration, Instant};

use dimvexa::{Conversion, Dim, Quantity, Ratio, Registry, Unit, registry};

struct Row {
    name: String,
    median: f64,
    min: f64,
    max: f64,
    note: String,
}

struct Bench {
    rows: Vec<Row>,
}

impl Bench {
    /// Times `f`, which performs `ops` operations per call.
    fn run<R>(&mut self, name: &str, ops: u64, note: &str, mut f: impl FnMut() -> R) {
        // Warm up and calibrate the number of calls per sample to about 40 ms.
        let mut calls = 1_u64;
        loop {
            let t = Instant::now();
            for _ in 0..calls {
                black_box(f());
            }
            if t.elapsed() >= Duration::from_millis(40) || calls >= 1 << 40 {
                break;
            }
            calls *= 2;
        }
        let mut samples: Vec<f64> = (0..15)
            .map(|_| {
                let t = Instant::now();
                for _ in 0..calls {
                    black_box(f());
                }
                t.elapsed().as_secs_f64() * 1e9 / (calls * ops) as f64
            })
            .collect();
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        self.rows.push(Row {
            name: name.to_string(),
            median: samples[samples.len() / 2],
            min: samples[0],
            max: samples[samples.len() - 1],
            note: note.to_string(),
        });
    }

    fn print(&self) {
        let mut out = std::io::stdout().lock();
        writeln!(out, "| benchmark | median ns/op | min | max | note |").unwrap();
        writeln!(out, "|---|---:|---:|---:|---|").unwrap();
        for r in &self.rows {
            writeln!(
                out,
                "| {} | {:.3} | {:.3} | {:.3} | {} |",
                r.name, r.median, r.min, r.max, r.note
            )
            .unwrap();
        }
    }

    fn median(&self, name: &str) -> f64 {
        self.rows
            .iter()
            .find(|r| r.name == name)
            .map_or(f64::NAN, |r| r.median)
    }
}

fn letters(mut n: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, char::from(b'a' + u8::try_from(n % 26).unwrap()));
        n /= 26;
        if n == 0 {
            break;
        }
        n -= 1;
    }
    format!("u{s}")
}

fn synthetic(n: usize) -> Registry {
    let mut reg = Registry::new();
    for i in 0..n {
        let f = Ratio::new(i128::try_from(i).unwrap() + 1, 7).unwrap();
        reg.register(Unit::linear(&letters(i), Dim::M, f).unwrap())
            .unwrap();
    }
    reg
}

fn main() {
    let mut b = Bench { rows: Vec::new() };
    let reg = registry().unwrap();

    // ---- 1. pre-parsed scalar conversion
    let lin = reg.conversion("m/s", "km/h").unwrap();
    let aff = reg.conversion("degF", "degC").unwrap();
    let aff_k = reg.conversion("K", "degC").unwrap();
    const N: u64 = 1000;
    let xs: Vec<f64> = (0..N).map(|i| (i as f64) * 0.37 + 1.0).collect();
    b.run(
        "scalar linear apply (throughput)",
        N,
        "independent inputs, black_box on input and sum",
        || {
            let mut acc = 0.0;
            for x in &xs {
                acc += lin.apply(black_box(*x));
            }
            acc
        },
    );
    b.run(
        "scalar affine apply (throughput)",
        N,
        "degF -> degC",
        || {
            let mut acc = 0.0;
            for x in &xs {
                acc += aff.apply(black_box(*x));
            }
            acc
        },
    );
    b.run(
        "scalar linear apply (latency chain)",
        N,
        "each result feeds the next call",
        || {
            let mut x = 1.0_f64;
            for _ in 0..N {
                x = lin.apply(black_box(x)) * 0.2777777777777778;
            }
            x
        },
    );
    b.run(
        "scalar affine apply (latency chain)",
        N,
        "each result feeds the next call",
        || {
            let mut x = 1.0_f64;
            for _ in 0..N {
                x = aff_k.apply(black_box(x)) + 273.15;
            }
            x
        },
    );
    b.run(
        "single call via black_box(&conv)",
        1,
        "one call, handle and input opaque to the optimiser",
        || black_box(&lin).apply(black_box(10.0)),
    );
    b.run(
        "plain multiply `x * 3.6` (reference)",
        N,
        "hand-written, same loop shape as scalar linear",
        || {
            let mut acc = 0.0;
            for x in &xs {
                acc += black_box(*x) * 3.6;
            }
            acc
        },
    );
    let exact = Ratio::from_int(10);
    b.run(
        "apply_exact (Ratio, linear)",
        1,
        "checked i128 rational multiply+add",
        || lin.apply_exact(black_box(exact)).unwrap(),
    );

    b.run(
        "apply_exact_f64 (linear, correctly rounded)",
        1,
        "f64 in, f64 out, one rounding",
        || lin.apply_exact_f64(black_box(10.5)).unwrap(),
    );
    b.run(
        "apply_exact_f64 (affine, correctly rounded)",
        1,
        "degF -> degC",
        || aff.apply_exact_f64(black_box(98.6)).unwrap(),
    );

    // ---- 2. parse cost (the library has no cache by design)
    b.run(
        "parse unit `psi` (cold, every call)",
        1,
        "resolve one symbol",
        || reg.parse(black_box("psi")).unwrap(),
    );
    b.run(
        "parse unit `kg.m2/s2` (cold)",
        1,
        "compound, three symbols",
        || reg.parse(black_box("kg.m2/s2")).unwrap(),
    );
    b.run(
        "parse `Btu_IT/(h.ft2.delta_degF)` (cold)",
        1,
        "long compound",
        || reg.parse(black_box("Btu_IT/(h.ft2.delta_degF)")).unwrap(),
    );
    b.run(
        "conversion(\"m/s\",\"km/h\") (parse+compose both)",
        1,
        "everything done once per prepared handle",
        || reg.conversion(black_box("m/s"), black_box("km/h")).unwrap(),
    );
    b.run("conversion(\"degF\",\"degC\")", 1, "affine", || {
        reg.conversion(black_box("degF"), black_box("degC"))
            .unwrap()
    });
    b.run(
        "reuse prepared handle (`cached` by the caller)",
        1,
        "copy of a Conversion + one apply",
        || {
            let c = *black_box(&lin);
            c.apply(black_box(10.0))
        },
    );
    b.run(
        "registry construction, 63 units (cold start)",
        1,
        "Registry::from_defs on the compiled table",
        || registry().unwrap(),
    );

    // ---- 3. slices
    for &(label, n) in &[
        ("10^3", 1_000_usize),
        ("10^5", 100_000),
        ("10^6", 1_000_000),
    ] {
        let input: Vec<f64> = (0..n).map(|i| (i as f64) * 0.001 + 0.5).collect();
        let mut out = vec![0.0_f64; n];
        let a = lin.factor_f64();
        let nn = n as u64;
        b.run(
            &format!("slice linear apply_slice {label}"),
            nn,
            "ns per element",
            || {
                lin.apply_slice(black_box(&input), black_box(&mut out))
                    .unwrap();
            },
        );
        b.run(
            &format!("slice plain multiply loop {label}"),
            nn,
            "for (y,x) in zip { *y = *x * a }",
            || {
                let a = black_box(a);
                for (y, x) in black_box(&mut out).iter_mut().zip(black_box(&input)) {
                    *y = *x * a;
                }
            },
        );
        b.run(
            &format!("slice memcpy ceiling {label}"),
            nn,
            "copy_from_slice",
            || {
                black_box(&mut out).copy_from_slice(black_box(&input));
            },
        );
        b.run(
            &format!("slice linear apply_in_place {label}"),
            nn,
            "in place (values drift, finite)",
            || {
                let mut d = input.clone();
                lin.apply_in_place(black_box(&mut d));
                d
            },
        );
        b.run(
            &format!("slice clone only {label}"),
            nn,
            "cost of the clone inside the in-place row",
            || input.clone(),
        );
        b.run(
            &format!("slice affine apply_slice {label}"),
            nn,
            "degF -> degC, split shift",
            || {
                aff.apply_slice(black_box(&input), black_box(&mut out))
                    .unwrap();
            },
        );
        let (s, af) = (32.0_f64, 5.0 / 9.0);
        b.run(
            &format!("slice plain (x - 32) * 5/9 loop {label}"),
            nn,
            "hand-written reference",
            || {
                let (s, af) = (black_box(s), black_box(af));
                for (y, x) in black_box(&mut out).iter_mut().zip(black_box(&input)) {
                    *y = (*x - s) * af;
                }
            },
        );
    }

    // ---- 4. dimension operations
    let d1 = Dim::new([1, 2, -3, 0, 0, 0, 0]);
    let d2 = Dim::new([0, -1, 2, 1, 0, 0, 0]);
    b.run("Dim::checked_add", 1, "7 exponents", || {
        black_box(d1).checked_add(black_box(d2)).unwrap()
    });
    b.run("Dim::checked_sub", 1, "", || {
        black_box(d1).checked_sub(black_box(d2)).unwrap()
    });
    b.run("Dim::checked_scale(3)", 1, "", || {
        black_box(d1).checked_scale(black_box(3)).unwrap()
    });
    let (r1, r2) = (Ratio::new(381, 1250).unwrap(), Ratio::new(5, 9).unwrap());
    b.run("Ratio::checked_mul", 1, "reduced fractions", || {
        black_box(r1).checked_mul(black_box(r2)).unwrap()
    });
    b.run("Ratio::checked_add", 1, "", || {
        black_box(r1).checked_add(black_box(r2)).unwrap()
    });
    b.run("Ratio::to_f64", 1, "correctly rounded", || {
        black_box(r1).to_f64().unwrap()
    });

    // ---- 5. quantity composition
    let q_pow = reg.quantity("1.1 kW").unwrap();
    let q_t = reg.quantity("8 h").unwrap();
    b.run(
        "Quantity mul (kW * h)",
        1,
        "allocates the product unit",
        || black_box(&q_pow).checked_mul(black_box(&q_t)).unwrap(),
    );
    let kwh = reg.parse("kWh").unwrap();
    let prod = q_pow.checked_mul(&q_t).unwrap();
    b.run(
        "Quantity::to_unit (kW.h -> kWh)",
        1,
        "prepare + exact apply",
        || black_box(&prod).to_unit(black_box(&kwh)).unwrap(),
    );

    // ---- 6. registry lookup with 10 and 10,000 units
    for &n in &[10_usize, 10_000] {
        let r = synthetic(n);
        let names: Vec<String> = (0..n).step_by((n / 10).max(1)).map(letters).collect();
        let k = names.len() as u64;
        b.run(
            &format!("registry get() with {n} units"),
            k,
            "exact symbol, HashMap",
            || {
                let mut acc = 0_usize;
                for s in &names {
                    acc += usize::from(r.get(black_box(s)).is_some());
                }
                acc
            },
        );
        b.run(
            &format!("registry parse() with {n} units"),
            k,
            "lookup + resolve",
            || {
                for s in &names {
                    black_box(r.parse(black_box(s)).unwrap());
                }
            },
        );
        b.run(
            &format!("registry miss (unknown symbol) with {n} units"),
            1,
            "prefix split + indexed suggestion lookup",
            || r.parse(black_box("zzzz9")).is_err(),
        );
        b.run(
            &format!("registry miss, valid-looking, {n} units"),
            1,
            "unknown symbol `Zork`",
            || r.parse(black_box("Zork")).is_err(),
        );
    }

    b.print();
    let mut out = std::io::stdout().lock();
    let ceil = b.median("slice memcpy ceiling 10^6");
    let ratio = |a: &str, c: &str| b.median(a) / b.median(c);
    writeln!(out, "\nsummary:").unwrap();
    writeln!(
        out,
        "- scalar linear apply, throughput: {:.3} ns",
        b.median("scalar linear apply (throughput)")
    )
    .unwrap();
    writeln!(
        out,
        "- slice 10^6 apply_slice / plain loop: {:.3}",
        ratio(
            "slice linear apply_slice 10^6",
            "slice plain multiply loop 10^6"
        )
    )
    .unwrap();
    writeln!(
        out,
        "- slice 10^6 apply_slice / memcpy: {:.3} (memcpy {ceil:.3} ns/elem)",
        ratio("slice linear apply_slice 10^6", "slice memcpy ceiling 10^6")
    )
    .unwrap();
    let _ = (
        Conversion::linear(Ratio::ONE),
        Quantity::new(Ratio::ONE, reg.parse("m").unwrap()),
    );
}
