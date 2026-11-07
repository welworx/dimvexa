// SPDX-License-Identifier: MIT OR Apache-2.0
//! Benchmark runner for Dimvexa (Rust API and CLI) and the Rust ceilings (plain loop, memcpy).
//!
//! Reads `benchmarks/corpus/*.json`, runs the same measurements as the Python and Node runners
//! and writes `results/<lib>-<version>.json` in the schema `dimvexa.bench.result/1`
//! (see docs/benchmarking.md). Standard library only, apart from the local `dimvexa` crate.
//!
//! Usage: dimvexa-bench [--quick] [--ceiling] [--cli PATH] [--out DIR] [--corpus DIR] [--only perf|semantics]

mod harness;
mod json;

use harness::*;

use dimvexa::{Registry, registry};
use json::Json;
use std::collections::BTreeMap;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// "0.0.0+<git short sha>[-dirty]": the workspace version and the exact commit that was measured.
fn dimvexa_version() -> String {
    let root = repo_root();
    let toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    let ver = toml
        .lines()
        .find(|l| l.starts_with("version = "))
        .and_then(|l| l.split('"').nth(1))
        .unwrap_or("unknown")
        .to_string();
    let dir = root.to_string_lossy().to_string();
    let sha = cmd_out("git", &["-C", &dir, "rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "nogit".to_string());
    let dirty = cmd_out("git", &["-C", &dir, "status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    format!("{ver}+{sha}{}", if dirty { "-dirty" } else { "" })
}

// ---------------------------------------------------------------- performance

fn conv_ids(perf: &Json) -> Vec<&Json> {
    perf.get("conversions").arr().iter().collect()
}

fn err_entry(msg: &str) -> Json {
    let mut e = Json::obj();
    e.set("unsupported", Json::Bool(true));
    e.set("error", Json::s(msg));
    e
}

fn measure_dimvexa_perf(reg: &Registry, perf: &Json, p: &Params, cli: Option<&str>) -> Json {
    let inp = perf.get("inputs");
    let max_n = *p.batch_sizes.iter().max().unwrap_or(&1000);
    let values = lcg32(max_n, inp.get("seed").num() as u32, inp.get("lo").num(), inp.get("hi").num());
    let mut modes = Json::obj();
    for mode in ["prepared", "exact"] {
        let mut per_conv = Json::obj();
        for c in conv_ids(perf) {
            let (id, from, to) = (c.get("id").str(), c.get("from").str(), c.get("to").str());
            eprintln!("  [{mode}] {id}");
            let conv = match reg.conversion(from, to) {
                Ok(c) => c,
                Err(e) => {
                    per_conv.set(id, err_entry(&e.to_string()));
                    continue;
                }
            };
            let mut entry = Json::obj();
            let cold: Vec<f64> = (0..p.cold_samples)
                .map(|_| {
                    let t = Instant::now();
                    let r = registry().ok();
                    let c2 = r.as_ref().and_then(|r| r.conversion(from, to).ok());
                    black_box(&c2);
                    t.elapsed().as_nanos() as f64
                })
                .collect();
            entry.set("prepare_cold", stat(cold, "ns"));
            entry.set(
                "prepare_warm",
                time_loop(|| { black_box(reg.conversion(black_box(from), black_box(to)).ok()); }, p.warmup_s, p.samples, p.min_sample_s, 1.0),
            );
            let x = 12.5_f64;
            if mode == "prepared" {
                entry.set("scalar", time_loop(|| { black_box(conv.apply(black_box(x))); }, p.warmup_s, p.samples, p.min_sample_s, 1.0));
            } else {
                entry.set("scalar", time_loop(|| { black_box(conv.apply_exact_f64(black_box(x)).ok()); }, p.warmup_s, p.samples, p.min_sample_s, 1.0));
            }
            entry.set("loop_overhead", time_loop(|| { black_box(0_u8); }, p.warmup_s, p.samples, p.min_sample_s, 1.0));
            let (v, u) = split_quantity(c.get("oneshot").str());
            if mode == "prepared" {
                let val: f64 = v.parse().unwrap_or(f64::NAN);
                entry.set("oneshot", time_loop(
                    || { black_box(reg.conversion(black_box(u), black_box(to)).map(|c| c.apply(black_box(val))).ok()); },
                    p.warmup_s, p.samples, p.min_sample_s, 1.0));
            } else {
                let text = c.get("oneshot").str().to_string();
                entry.set("oneshot", time_loop(
                    || {
                        let r = reg.quantity(black_box(&text)).and_then(|q| q.to_unit(&reg.parse(to)?)).and_then(|q| q.to_f64());
                        black_box(r.ok());
                    },
                    p.warmup_s, p.samples, p.min_sample_s, 1.0));
            }
            let mut batches = Json::obj();
            for &n in &p.batch_sizes {
                let input = &values[..n];
                let mut out = vec![0.0_f64; n];
                let samples = if n < 1_000_000 { p.samples } else { (p.samples / 3).max(5) };
                let r = if mode == "prepared" {
                    time_loop(|| { let _ = conv.apply_slice(black_box(input), &mut out); black_box(&out); }, p.warmup_s.min(0.05), samples, 0.0, n as f64)
                } else {
                    time_loop(|| {
                        for (o, &i) in out.iter_mut().zip(input) {
                            *o = conv.apply_exact_f64(i).unwrap_or(f64::NAN);
                        }
                        black_box(&out);
                    }, p.warmup_s.min(0.05), samples, 0.0, n as f64)
                };
                batches.set(&n.to_string(), r);
            }
            entry.set("batch", batches);
            per_conv.set(id, entry);
        }
        modes.set(mode, per_conv);
    }
    if let Some(cli) = cli {
        let mut per_conv = Json::obj();
        for c in conv_ids(perf) {
            let (v, u) = split_quantity(c.get("oneshot").str());
            let to = c.get("to").str();
            eprintln!("  [cli] {}", c.get("id").str());
            let run = || {
                let o = Command::new(cli).args(["convert", v, u, to]).output();
                black_box(o.ok());
            };
            let mut entry = Json::obj();
            let pp = Params { warmup_s: 0.1_f64.min(p.warmup_s * 5.0), ..Params::new(p.quick) };
            entry.set("oneshot", time_loop(run, pp.warmup_s, pp.samples, p.min_sample_s.max(0.05), 1.0));
            entry.set("batch", Json::obj());
            per_conv.set(c.get("id").str(), entry);
        }
        modes.set("cli", per_conv);
    }
    modes
}

fn measure_ceiling_perf(perf: &Json, p: &Params) -> Json {
    let inp = perf.get("inputs");
    let max_n = *p.batch_sizes.iter().max().unwrap_or(&1000);
    let values = lcg32(max_n, inp.get("seed").num() as u32, inp.get("lo").num(), inp.get("hi").num());
    let mut modes = Json::obj();
    for mode in ["plain_loop", "memcpy"] {
        let mut per_conv = Json::obj();
        for c in conv_ids(perf) {
            let (a, b) = (c.get("factor").num(), c.get("offset").num());
            let mut entry = Json::obj();
            if mode == "plain_loop" {
                let x = 12.5_f64;
                entry.set("scalar", time_loop(|| { black_box(black_box(x) * black_box(a) + black_box(b)); }, p.warmup_s, p.samples, p.min_sample_s, 1.0));
            }
            entry.set("loop_overhead", time_loop(|| { black_box(0_u8); }, p.warmup_s, p.samples, p.min_sample_s, 1.0));
            let mut batches = Json::obj();
            for &n in &p.batch_sizes {
                let input = &values[..n];
                let mut out = vec![0.0_f64; n];
                let samples = if n < 1_000_000 { p.samples } else { (p.samples / 3).max(5) };
                let r = if mode == "plain_loop" {
                    time_loop(|| {
                        for (o, &i) in out.iter_mut().zip(black_box(input)) {
                            *o = i * a + b;
                        }
                        black_box(&out);
                    }, p.warmup_s.min(0.05), samples, 0.0, n as f64)
                } else {
                    time_loop(|| { out.copy_from_slice(black_box(input)); black_box(&out); }, p.warmup_s.min(0.05), samples, 0.0, n as f64)
                };
                batches.set(&n.to_string(), r);
            }
            entry.set("batch", batches);
            per_conv.set(c.get("id").str(), entry);
        }
        modes.set(mode, per_conv);
    }
    modes
}

// ---------------------------------------------------------------- accuracy and safety

fn convert_case(mode: &str, reg: &Registry, value: &str, from: &str, to: &str) -> Result<f64, (String, String)> {
    let e = |e: dimvexa::Error| (e.code().to_string(), e.to_string());
    match mode {
        "fast_f64" => {
            let conv = reg.conversion(from, to).map_err(e)?;
            let x: f64 = value.parse().map_err(|_| ("invalid_number".to_string(), value.to_string()))?;
            Ok(conv.apply(x))
        }
        "exact_f64" => {
            let conv = reg.conversion(from, to).map_err(e)?;
            let x: f64 = value.parse().map_err(|_| ("invalid_number".to_string(), value.to_string()))?;
            conv.apply_exact_f64(x).map_err(e)
        }
        _ => {
            let q = reg.quantity(&format!("{value} {from}")).map_err(e)?;
            let t = reg.parse(to).map_err(e)?;
            q.to_unit(&t).map_err(e)?.to_f64().map_err(e)
        }
    }
}

fn rec(id: &str, cat: &str, outcome: &str) -> Json {
    let mut r = Json::obj();
    r.set("id", Json::s(id));
    r.set("category", Json::s(cat));
    r.set("outcome", Json::s(outcome));
    r
}

fn count(recs: &[Json], k: &str) -> f64 {
    recs.iter().filter(|r| r.get("outcome").str() == k).count() as f64
}

fn by_category(recs: &[Json], outcomes: &[&str]) -> Json {
    let mut cats: BTreeMap<String, Json> = BTreeMap::new();
    for r in recs {
        let c = cats.entry(r.get("category").str().to_string()).or_insert_with(|| {
            let mut o = Json::obj();
            o.set("total", Json::Num(0.0));
            for k in outcomes {
                o.set(k, Json::Num(0.0));
            }
            o
        });
        let bump = |c: &mut Json, k: &str| {
            let n = c.get(k).num();
            c.set(k, Json::Num(if n.is_nan() { 1.0 } else { n + 1.0 }));
        };
        bump(c, "total");
        bump(c, r.get("outcome").str());
    }
    Json::Obj(cats)
}

fn semantics(reg: &Registry, corpus: &BTreeMap<String, Json>) -> Json {
    let mut out = Json::obj();
    for mode in ["fast_f64", "exact_f64", "text"] {
        eprintln!("  semantics [{mode}]");
        let mut mc = Vec::new();
        let mut ulps: Vec<f64> = Vec::new();
        let mut worst: Vec<(f64, Json)> = Vec::new();
        for c in corpus["must_convert"].get("cases").arr() {
            let (id, cat) = (c.get("id").str(), c.get("category").str());
            let want: f64 = c.get("expect_f64").str().parse().unwrap_or(f64::NAN);
            let r = match convert_case(mode, reg, c.get("value").str(), c.get("from").str(), c.get("to").str()) {
                Ok(got) => {
                    let u = ulp_distance(got, want);
                    let mut r = rec(id, cat, if u.is_some_and(|u| u <= TOLERANCE_ULP) { "correct" } else { "wrong_value" });
                    r.set("ulp", u.map_or(Json::Null, |u| Json::Num(u as f64)));
                    r.set("got", Json::Str(format!("{got:?}")));
                    r.set("want", Json::s(c.get("expect_f64").str()));
                    if let Some(u) = u {
                        ulps.push(u as f64);
                        let mut w = Json::obj();
                        w.set("id", Json::s(id));
                        w.set("ulp", Json::Num(u as f64));
                        w.set("got", Json::Str(format!("{got:?}")));
                        w.set("want", Json::s(c.get("expect_f64").str()));
                        worst.push((u as f64, w));
                    }
                    r
                }
                Err((code, msg)) => {
                    let mut r = rec(id, cat, "unsupported");
                    r.set("detail", Json::Str(format!("{code}: {msg}").chars().take(160).collect()));
                    r.set("code", Json::Str(code));
                    r
                }
            };
            mc.push(r);
        }
        let mut mr = Vec::new();
        for c in corpus["must_refuse"].get("cases").arr() {
            let (id, cat) = (c.get("id").str(), c.get("category").str());
            let result: Result<String, (String, String)> = if c.get("op").str() == "convert" {
                convert_case(mode, reg, c.get("value").str(), c.get("from").str(), c.get("to").str()).map(|v| format!("{v:?}"))
            } else {
                let a = reg.quantity(c.get("a").str());
                let b = reg.quantity(c.get("b").str());
                match (a, b) {
                    (Ok(a), Ok(b)) => a.checked_add(&b).map(|q| q.to_string()).map_err(|e| (e.code().to_string(), e.to_string())),
                    (Err(e), _) | (_, Err(e)) => Err((e.code().to_string(), e.to_string())),
                }
            };
            mr.push(match result {
                Ok(v) => {
                    let mut r = rec(id, cat, "silently_accepted");
                    r.set("got", Json::Str(v));
                    r.set("why_it_should_refuse", Json::s(c.get("why").str()));
                    r
                }
                Err((code, msg)) => {
                    let intended = c.get("dimvexa_codes").arr().iter().any(|k| k.str() == code);
                    let mut r = rec(id, cat, "refused");
                    r.set("detail", Json::Str(format!("{code}: {msg}").chars().take(160).collect()));
                    r.set("code", Json::Str(code));
                    r.set("intended_reason", Json::Bool(intended));
                    r
                }
            });
        }
        let mut cov = Json::obj();
        for l in corpus["coverage"].get("lists").arr() {
            let unknown: Vec<Json> = l.get("units").arr().iter().filter(|u| reg.parse(u.str()).is_err()).cloned().collect();
            let total = l.get("units").arr().len();
            let mut o = Json::obj();
            o.set("total", Json::Num(total as f64));
            o.set("known", Json::Num((total - unknown.len()) as f64));
            o.set("unknown", Json::Arr(unknown));
            cov.set(l.get("category").str(), o);
        }
        worst.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut acc = Json::obj();
        acc.set("cases", Json::Num(mc.len() as f64));
        acc.set("answered", Json::Num(ulps.len() as f64));
        acc.set("unsupported", Json::Num(count(&mc, "unsupported")));
        acc.set("tolerance_ulp", Json::Num(TOLERANCE_ULP as f64));
        acc.set("max_ulp", if ulps.is_empty() { Json::Null } else { Json::Num(ulps.iter().copied().fold(0.0, f64::max)) });
        acc.set("median_ulp", if ulps.is_empty() { Json::Null } else { Json::Num(median(&ulps)) });
        acc.set("bit_identical", Json::Num(ulps.iter().filter(|&&u| u == 0.0).count() as f64));
        acc.set("over_1_ulp", Json::Num(ulps.iter().filter(|&&u| u > 1.0).count() as f64));
        acc.set("wrong", Json::Num(count(&mc, "wrong_value")));
        acc.set("worst", Json::Arr(worst.into_iter().take(5).map(|w| w.1).collect()));
        let mut m = Json::obj();
        let mut mcj = Json::obj();
        mcj.set("total", Json::Num(mc.len() as f64));
        mcj.set("correct", Json::Num(count(&mc, "correct")));
        mcj.set("wrong_value", Json::Num(count(&mc, "wrong_value")));
        mcj.set("unsupported", Json::Num(count(&mc, "unsupported")));
        mcj.set("by_category", by_category(&mc, &["correct", "wrong_value", "unsupported"]));
        let mut mrj = Json::obj();
        mrj.set("total", Json::Num(mr.len() as f64));
        mrj.set("refused", Json::Num(count(&mr, "refused")));
        mrj.set("silently_accepted", Json::Num(count(&mr, "silently_accepted")));
        mrj.set("not_applicable", Json::Num(0.0));
        mrj.set("refused_for_intended_reason", Json::Num(mr.iter().filter(|r| matches!(r.get("intended_reason"), Json::Bool(true))).count() as f64));
        mrj.set("by_category", by_category(&mr, &["refused", "silently_accepted", "not_applicable"]));
        let mut cases = Json::obj();
        cases.set("must_convert", Json::Arr(mc));
        cases.set("must_refuse", Json::Arr(mr));
        m.set("must_convert", mcj);
        m.set("must_refuse", mrj);
        m.set("coverage", cov);
        m.set("cases", cases);
        m.set("accuracy", acc);
        out.set(mode, m);
    }
    out
}

// ---------------------------------------------------------------- main

fn load_corpus(dir: &Path) -> Result<BTreeMap<String, Json>, String> {
    let mut m = BTreeMap::new();
    for n in ["must_convert", "must_refuse", "coverage", "perf", "manifest"] {
        let t = std::fs::read_to_string(dir.join(format!("{n}.json"))).map_err(|e| format!("{n}.json: {e}"))?;
        m.insert(n.to_string(), json::parse(&t).map_err(|e| format!("{n}.json: {e}"))?);
    }
    Ok(m)
}

fn safe(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || "._+-".contains(c) { c } else { '_' }).collect()
}

fn mode_info(modes: &[(&str, &str, &str)]) -> Json {
    let mut o = Json::obj();
    for (k, idiom, d) in modes {
        let mut m = Json::obj();
        m.set("idiomatic", Json::s(idiom));
        m.set("description", Json::s(d));
        o.set(k, m);
    }
    o
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let opt = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).cloned();
    let bench = repo_root().join("benchmarks");
    let corpus_dir = opt("--corpus").map_or_else(|| bench.join("corpus"), PathBuf::from);
    let out_dir = opt("--out").map_or_else(|| bench.join("results"), PathBuf::from);
    let corpus = match load_corpus(&corpus_dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cannot read the corpus in {}: {e}\nrun: python3 benchmarks/tools/gen_corpus.py", corpus_dir.display());
            std::process::exit(2);
        }
    };
    let p = Params::new(flag("--quick"));
    let only = opt("--only").unwrap_or_else(|| "all".to_string());
    let ceiling = flag("--ceiling");
    let (name, version) = if ceiling {
        ("rust-ceiling".to_string(), cmd_out("rustc", &["--version"]).and_then(|v| v.split_whitespace().nth(1).map(String::from)).unwrap_or_else(|| "unknown".to_string()))
    } else {
        ("dimvexa".to_string(), dimvexa_version())
    };
    eprintln!("{name} {version}");
    let reg = match registry() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot build the registry: {e}");
            std::process::exit(2);
        }
    };
    let perf = &corpus["perf"];
    let mut res = Json::obj();
    res.set("schema", Json::s("dimvexa.bench.result/1"));
    res.set("fixture", Json::Bool(false));
    res.set("kind", Json::s(if ceiling { "ceiling" } else { "library" }));
    let mut lib = Json::obj();
    lib.set("name", Json::Str(name.clone()));
    lib.set("version", Json::Str(version.clone()));
    if !ceiling {
        lib.set("data_version", Json::Str(dimvexa::data_version()));
    }
    res.set("library", lib);
    let mut rt = Json::obj();
    rt.set("name", Json::s("rustc"));
    rt.set("version", Json::Str(cmd_out("rustc", &["--version"]).unwrap_or_else(|| "unknown".to_string())));
    rt.set("language", Json::s("rust"));
    rt.set("profile", Json::s("release (thin LTO, codegen-units 1, default target CPU)"));
    res.set("runtime", rt);
    res.set("machine", machine_info());
    res.set("date_utc", Json::Str(date_utc()));
    res.set("lockfile", Json::Str(std::env::var("DIMVEXA_BENCH_LOCK").unwrap_or_else(|_| "unknown".to_string())));
    res.set("corpus_id", Json::s(corpus["manifest"].get("corpus_id").str()));
    res.set("parameters", p.json());
    if ceiling {
        res.set("modes", mode_info(&[
            ("plain_loop", "fast", "out[i] = in[i] * a + b in a plain for loop over slices"),
            ("memcpy", "fast", "copy_from_slice: the memory-bandwidth ceiling"),
        ]));
    } else {
        let mut modes = mode_info(&[
            ("prepared", "fast", "Registry::conversion once, then Conversion::apply / apply_slice (f64 fast path, 1 ulp linear, 2 ulp affine); one-shot = conversion(from,to).apply(x) from strings"),
            ("exact", "plain", "Conversion::apply_exact_f64 (correctly rounded); one-shot = Registry::quantity(text).to_unit(..).to_f64() (exact rational arithmetic)"),
        ]);
        if let Some(cli) = opt("--cli") {
            let _ = cli;
            let mut m = Json::obj();
            m.set("idiomatic", Json::s("plain"));
            m.set("description", Json::s("one process per conversion: `dimvexa convert VALUE FROM TO` (spawn + parse + print); one-shot only"));
            modes.set("cli", m);
        }
        res.set("modes", modes);
    }
    if only != "semantics" {
        let cli = opt("--cli");
        let perf_res = if ceiling { measure_ceiling_perf(perf, &p) } else { measure_dimvexa_perf(&reg, perf, &p, cli.as_deref()) };
        res.set("performance", perf_res);
        let mut su = Json::obj();
        // "import" has no meaning for a compiled library; the closest cost is building the registry.
        let reg_ms: Vec<f64> = (0..p.cold_samples.max(5)).map(|_| {
            let t = Instant::now();
            black_box(registry().ok());
            t.elapsed().as_secs_f64() * 1e3
        }).collect();
        su.set("import_ms", if ceiling { Json::Null } else { stat(reg_ms, "ms") });
        su.set("import_ms_meaning", Json::s("time to build the registry (the seed unit table); there is no import step in Rust"));
        su.set("peak_rss_kb", peak_rss_kb());
        let exe_size = std::env::current_exe().ok().and_then(|e| std::fs::metadata(e).ok()).map_or(Json::Null, |m| Json::Num(m.len() as f64));
        su.set("package_bytes", exe_size);
        su.set("package_bytes_meaning", Json::s("size of this runner executable (includes the runner itself)"));
        if let Some(cli) = opt("--cli") {
            su.set("cli_binary_bytes", std::fs::metadata(cli).map_or(Json::Null, |m| Json::Num(m.len() as f64)));
        }
        res.set("startup", su);
    }
    if !ceiling && only != "perf" {
        res.set("semantics", semantics(&reg, &corpus));
    }
    let mut text = String::new();
    res.write(&mut text, 0);
    text.push('\n');
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!("cannot create {}: {e}", out_dir.display());
        std::process::exit(2);
    }
    let file = out_dir.join(format!("{}-{}.json", safe(&name), safe(&version)));
    match std::fs::write(&file, text) {
        Ok(()) => eprintln!("wrote {}", file.display()),
        Err(e) => {
            eprintln!("cannot write {}: {e}", file.display());
            std::process::exit(2);
        }
    }
}
