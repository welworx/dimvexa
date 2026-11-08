// SPDX-License-Identifier: MIT OR Apache-2.0
//! Benchmark runner for `uom` (compile-time dimensional analysis). Run in the first comparison (docs/benchmarks/comparison-run-1.md).
//!
//! uom resolves units at compile time and has no string interface, so:
//! * speed rows measure `Quantity::new::<from>(x).get::<to>()`, the idiomatic typed conversion
//!   (the compiler can fold the factor, which is its advantage and is reported as such);
//! * "one-shot parse+convert from strings" does not exist and is reported as unsupported;
//! * accuracy covers only the pairs listed in `convert_pair` (each needs a typed unit chosen at
//!   compile time); everything else is "unsupported";
//! * the must-refuse requests cannot be expressed (no strings): reported as not applicable. That
//!   is a real property of the library (ambiguity is resolved by the programmer choosing a type),
//!   not a safety win or loss we can measure here.
//!
//! Usage: dimvexa-bench-uom [--quick] [--out DIR] [--corpus DIR]

#[path = "../../dimvexa-rs/src/harness.rs"]
mod harness;
#[path = "../../dimvexa-rs/src/json.rs"]
mod json;

use harness::{Params, TOLERANCE_ULP, lcg32, ulp_distance};
use json::Json;
use std::hint::black_box;
use std::path::PathBuf;

use uom::si::f64::{Length, Mass, Pressure, ThermodynamicTemperature, Velocity};
use uom::si::length::{foot, inch, kilometer, meter, mile};
use uom::si::mass::{kilogram, pound};
use uom::si::pressure::{kilopascal, pascal, pound_force_per_square_inch};
use uom::si::thermodynamic_temperature::{degree_celsius, degree_fahrenheit, kelvin};
use uom::si::velocity::{kilometer_per_hour, meter_per_second};

/// Typed conversions available to this runner, written once. `pairs!` expands the list into
/// `convert_pair` (used for accuracy; matches the strings per call) and `dispatch` (used for
/// timing; matches once and hands the typed conversion to a `Job` as a statically dispatched
/// closure, so the compiler sees the same code a user of uom writes).
macro_rules! pairs {
    ($(($from:literal, $to:literal) => |$x:ident| $body:expr),+ $(,)?) => {
        fn convert_pair(from: &str, to: &str, x: f64) -> Option<f64> {
            Some(match (from, to) {
                $(($from, $to) => { let $x = x; $body },)+
                _ => return None,
            })
        }
        fn dispatch(from: &str, to: &str, job: &mut impl Job) -> bool {
            match (from, to) {
                $(($from, $to) => { job.run(|$x: f64| $body); true },)+
                _ => false,
            }
        }
    };
}

/// A timing job that is generic over the typed conversion it measures.
trait Job {
    fn run<F: Fn(f64) -> f64>(&mut self, f: F);
}

pairs! {
    ("m", "ft") => |x| Length::new::<meter>(x).get::<foot>(),
    ("ft", "m") => |x| Length::new::<foot>(x).get::<meter>(),
    ("km", "m") => |x| Length::new::<kilometer>(x).get::<meter>(),
    ("in", "m") => |x| Length::new::<inch>(x).get::<meter>(),
    ("mi", "km") => |x| Length::new::<mile>(x).get::<kilometer>(),
    ("kg", "lb") => |x| Mass::new::<kilogram>(x).get::<pound>(),
    ("lb", "kg") => |x| Mass::new::<pound>(x).get::<kilogram>(),
    ("m/s", "km/h") => |x| Velocity::new::<meter_per_second>(x).get::<kilometer_per_hour>(),
    ("km/h", "m/s") => |x| Velocity::new::<kilometer_per_hour>(x).get::<meter_per_second>(),
    ("psi", "kPa") => |x| Pressure::new::<pound_force_per_square_inch>(x).get::<kilopascal>(),
    ("psi", "Pa") => |x| Pressure::new::<pound_force_per_square_inch>(x).get::<pascal>(),
    ("degC", "degF") => |x| ThermodynamicTemperature::new::<degree_celsius>(x).get::<degree_fahrenheit>(),
    ("degF", "degC") => |x| ThermodynamicTemperature::new::<degree_fahrenheit>(x).get::<degree_celsius>(),
    ("degC", "K") => |x| ThermodynamicTemperature::new::<degree_celsius>(x).get::<kelvin>(),
    ("K", "degC") => |x| ThermodynamicTemperature::new::<kelvin>(x).get::<degree_celsius>(),
}

/// Times the scalar call and the batch loops of one typed conversion.
struct Timing<'a> {
    p: &'a Params,
    values: &'a [f64],
    entry: Json,
}

impl Job for Timing<'_> {
    fn run<F: Fn(f64) -> f64>(&mut self, f: F) {
        let p = self.p;
        let x = 12.5_f64;
        self.entry.set("scalar", harness::time_loop(|| { black_box(f(black_box(x))); }, p.warmup_s, p.samples, p.min_sample_s, 1.0));
        self.entry.set("loop_overhead", harness::time_loop(|| { black_box(0_u8); }, p.warmup_s, p.samples, p.min_sample_s, 1.0));
        self.entry.set("oneshot", Json::Null);
        let mut batches = Json::obj();
        for &n in &p.batch_sizes {
            let input = &self.values[..n];
            let mut out = vec![0.0_f64; n];
            let samples = if n < 1_000_000 { p.samples } else { (p.samples / 3).max(5) };
            let r = harness::time_loop(|| {
                for (o, &i) in out.iter_mut().zip(black_box(input)) {
                    *o = f(i);
                }
                black_box(&out);
            }, p.warmup_s.min(0.05), samples, 0.0, n as f64);
            batches.set(&n.to_string(), r);
        }
        self.entry.set("batch", batches);
    }
}

const KNOWN: &[&str] = &["m", "ft", "km", "in", "mi", "kg", "lb", "m/s", "km/h", "psi", "kPa", "Pa", "degC", "degF", "K"];

fn lock_version(pkg: &str) -> String {
    let lock = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock")).unwrap_or_default();
    let mut lines = lock.lines();
    while let Some(l) = lines.next() {
        if l == format!("name = \"{pkg}\"") {
            if let Some(v) = lines.next().and_then(|v| v.strip_prefix("version = \"")) {
                return v.trim_end_matches('"').to_string();
            }
        }
    }
    "unknown".to_string()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).cloned();
    let p = Params::new(args.iter().any(|a| a == "--quick"));
    let bench = harness::repo_root().join("benchmarks");
    let corpus_dir = opt("--corpus").map_or_else(|| bench.join("corpus"), PathBuf::from);
    let out_dir = opt("--out").map_or_else(|| bench.join("results"), PathBuf::from);
    let rd = |n: &str| json::parse(&std::fs::read_to_string(corpus_dir.join(format!("{n}.json"))).unwrap_or_default()).unwrap_or(Json::Null);
    let (perf, mc, mr, cov, man) = (rd("perf"), rd("must_convert"), rd("must_refuse"), rd("coverage"), rd("manifest"));
    let version = lock_version("uom");
    eprintln!("uom {version}");

    let max_n = *p.batch_sizes.iter().max().unwrap_or(&1000);
    let values = lcg32(max_n, perf.get("inputs").get("seed").num() as u32, perf.get("inputs").get("lo").num(), perf.get("inputs").get("hi").num());
    let mut per_conv = Json::obj();
    for c in perf.get("conversions").arr() {
        let (id, from, to) = (c.get("id").str(), c.get("from").str(), c.get("to").str());
        if convert_pair(from, to, 1.0).is_none() {
            let mut e = Json::obj();
            e.set("unsupported", Json::Bool(true));
            e.set("error", Json::s("no typed unit pair compiled for this conversion in the runner"));
            per_conv.set(id, e);
            continue;
        }
        eprintln!("  [typed] {id}");
        let mut job = Timing { p: &p, values: &values, entry: Json::obj() };
        dispatch(from, to, &mut job);
        per_conv.set(id, job.entry);
    }
    let mut performance = Json::obj();
    performance.set("typed", per_conv);

    // semantics: accuracy over the pairs we can express
    let mut recs = Vec::new();
    let mut ulps = Vec::new();
    for c in mc.get("cases").arr() {
        let want: f64 = c.get("expect_f64").str().parse().unwrap_or(f64::NAN);
        let mut r = Json::obj();
        r.set("id", Json::s(c.get("id").str()));
        r.set("category", Json::s(c.get("category").str()));
        match convert_pair(c.get("from").str(), c.get("to").str(), c.get("value").str().parse().unwrap_or(f64::NAN)) {
            Some(got) => {
                let u = ulp_distance(got, want);
                r.set("outcome", Json::s(if u.is_some_and(|u| u <= TOLERANCE_ULP) { "correct" } else { "wrong_value" }));
                r.set("ulp", u.map_or(Json::Null, |u| Json::Num(u as f64)));
                r.set("got", Json::Str(format!("{got:?}")));
                r.set("want", Json::s(c.get("expect_f64").str()));
                if let Some(u) = u {
                    ulps.push(u as f64);
                }
            }
            None => {
                r.set("outcome", Json::s("unsupported"));
                r.set("detail", Json::s("no typed unit pair compiled for this request"));
            }
        }
        recs.push(r);
    }
    let n = |recs: &[Json], k: &str| recs.iter().filter(|r| r.get("outcome").str() == k).count() as f64;
    let refs: Vec<Json> = mr.get("cases").arr().iter().map(|c| {
        let mut r = Json::obj();
        r.set("id", Json::s(c.get("id").str()));
        r.set("category", Json::s(c.get("category").str()));
        r.set("outcome", Json::s("not_applicable"));
        r.set("detail", Json::s("uom has no string interface: the programmer picks types at compile time"));
        r
    }).collect();
    let mut covj = Json::obj();
    for l in cov.get("lists").arr() {
        let unknown: Vec<Json> = l.get("units").arr().iter().filter(|u| !KNOWN.contains(&u.str())).cloned().collect();
        let mut o = Json::obj();
        o.set("total", Json::Num(l.get("units").arr().len() as f64));
        o.set("known", Json::Num((l.get("units").arr().len() - unknown.len()) as f64));
        o.set("unknown", Json::Arr(unknown));
        covj.set(l.get("category").str(), o);
    }
    let mut acc = Json::obj();
    acc.set("cases", Json::Num(recs.len() as f64));
    acc.set("answered", Json::Num(ulps.len() as f64));
    acc.set("unsupported", Json::Num(n(&recs, "unsupported")));
    acc.set("tolerance_ulp", Json::Num(TOLERANCE_ULP as f64));
    acc.set("max_ulp", if ulps.is_empty() { Json::Null } else { Json::Num(ulps.iter().copied().fold(0.0, f64::max)) });
    acc.set("median_ulp", if ulps.is_empty() { Json::Null } else { Json::Num(harness::median(&ulps)) });
    acc.set("bit_identical", Json::Num(ulps.iter().filter(|&&u| u == 0.0).count() as f64));
    acc.set("over_1_ulp", Json::Num(ulps.iter().filter(|&&u| u > 1.0).count() as f64));
    acc.set("wrong", Json::Num(n(&recs, "wrong_value")));
    acc.set("worst", Json::Arr(Vec::new()));
    let mut mcj = Json::obj();
    mcj.set("total", Json::Num(recs.len() as f64));
    mcj.set("correct", Json::Num(n(&recs, "correct")));
    mcj.set("wrong_value", Json::Num(n(&recs, "wrong_value")));
    mcj.set("unsupported", Json::Num(n(&recs, "unsupported")));
    mcj.set("by_category", Json::obj());
    let mut mrj = Json::obj();
    mrj.set("total", Json::Num(refs.len() as f64));
    mrj.set("refused", Json::Num(0.0));
    mrj.set("silently_accepted", Json::Num(0.0));
    mrj.set("not_applicable", Json::Num(refs.len() as f64));
    mrj.set("by_category", Json::obj());
    let mut cases = Json::obj();
    cases.set("must_convert", Json::Arr(recs));
    cases.set("must_refuse", Json::Arr(refs));
    let mut typed = Json::obj();
    typed.set("must_convert", mcj);
    typed.set("must_refuse", mrj);
    typed.set("coverage", covj);
    typed.set("cases", cases);
    typed.set("accuracy", acc);
    let mut sem = Json::obj();
    sem.set("typed", typed);

    let mut res = Json::obj();
    res.set("schema", Json::s("dimvexa.bench.result/1"));
    res.set("fixture", Json::Bool(false));
    res.set("kind", Json::s("library"));
    let mut lib = Json::obj();
    lib.set("name", Json::s("uom"));
    lib.set("version", Json::Str(version.clone()));
    res.set("library", lib);
    let mut rt = Json::obj();
    rt.set("name", Json::s("rustc"));
    rt.set("version", Json::Str(harness::cmd_out("rustc", &["--version"]).unwrap_or_default()));
    rt.set("language", Json::s("rust"));
    res.set("runtime", rt);
    res.set("machine", harness::machine_info());
    res.set("date_utc", Json::Str(harness::date_utc()));
    res.set("lockfile", Json::Str(std::env::var("DIMVEXA_BENCH_LOCK").unwrap_or_else(|_| "unknown".to_string())));
    res.set("corpus_id", Json::s(man.get("corpus_id").str()));
    res.set("parameters", p.json());
    let mut modes = Json::obj();
    let mut m = Json::obj();
    m.set("idiomatic", Json::s("fast"));
    m.set("description", Json::s("compile-time typed conversion: Quantity::new::<from>(x).get::<to>(); no strings, no runtime unit parsing"));
    modes.set("typed", m);
    res.set("modes", modes);
    res.set("performance", performance);
    let mut su = Json::obj();
    su.set("import_ms", Json::Null);
    su.set("peak_rss_kb", harness::peak_rss_kb());
    su.set("package_bytes", std::env::current_exe().ok().and_then(|e| std::fs::metadata(e).ok()).map_or(Json::Null, |m| Json::Num(m.len() as f64)));
    res.set("startup", su);
    res.set("semantics", sem);
    let mut text = String::new();
    res.write(&mut text, 0);
    text.push('\n');
    let _ = std::fs::create_dir_all(&out_dir);
    let file = out_dir.join(format!("uom-{}.json", safe(&version)));
    match std::fs::write(&file, text) {
        Ok(()) => eprintln!("wrote {}", file.display()),
        Err(e) => {
            eprintln!("cannot write {}: {e}", file.display());
            std::process::exit(2);
        }
    }
}

fn safe(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || "._+-".contains(c) { c } else { '_' }).collect()
}
