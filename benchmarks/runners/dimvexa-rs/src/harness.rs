// SPDX-License-Identifier: MIT OR Apache-2.0
//! Timing, statistics and environment helpers shared by the Rust runners (also included by the
//! uom runner through `#[path]`). Standard library only.
#![allow(dead_code)]

use crate::json::Json;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

pub const TOLERANCE_ULP: u64 = 4;

pub struct Params {
    pub warmup_s: f64,
    pub samples: usize,
    pub min_sample_s: f64,
    pub cold_samples: usize,
    pub batch_sizes: Vec<usize>,
    pub quick: bool,
}

impl Params {
    pub fn new(quick: bool) -> Self {
        Self {
            warmup_s: if quick { 0.02 } else { 0.2 },
            samples: if quick { 5 } else { 15 },
            min_sample_s: if quick { 0.005 } else { 0.05 },
            cold_samples: if quick { 3 } else { 5 },
            batch_sizes: if quick { vec![1000, 100_000] } else { vec![1000, 100_000, 1_000_000] },
            quick,
        }
    }
    pub fn json(&self) -> Json {
        let mut o = Json::obj();
        o.set("warmup_s", Json::Num(self.warmup_s));
        o.set("samples", Json::Num(self.samples as f64));
        o.set("min_sample_s", Json::Num(self.min_sample_s));
        o.set("cold_samples", Json::Num(self.cold_samples as f64));
        o.set("batch_sizes", Json::Arr(self.batch_sizes.iter().map(|&n| Json::Num(n as f64)).collect()));
        o.set("tolerance_ulp", Json::Num(TOLERANCE_ULP as f64));
        o.set("quick", Json::Bool(self.quick));
        o.set("timer", Json::s("std::time::Instant"));
        o
    }
}

pub fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = s.len();
    if n == 0 {
        f64::NAN
    } else if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

pub fn stat(raw: Vec<f64>, unit: &str) -> Json {
    let m = median(&raw);
    let mad = median(&raw.iter().map(|x| (x - m).abs()).collect::<Vec<_>>());
    let mut o = Json::obj();
    o.set("unit", Json::s(unit));
    o.set("samples", Json::Num(raw.len() as f64));
    o.set("median", Json::Num(m));
    o.set("mad", Json::Num(mad));
    o.set("min", Json::Num(raw.iter().copied().fold(f64::INFINITY, f64::min)));
    o.set("max", Json::Num(raw.iter().copied().fold(f64::NEG_INFINITY, f64::max)));
    o.set("raw", Json::Arr(raw.into_iter().map(Json::Num).collect()));
    o
}

/// ns per item; `f` is called in a loop sized so that one sample lasts at least `min_sample_s`.
pub fn time_loop<F: FnMut()>(mut f: F, warmup_s: f64, samples: usize, min_sample_s: f64, items: f64) -> Json {
    let t = Instant::now();
    while t.elapsed().as_secs_f64() < warmup_s {
        f();
    }
    let mut inner: u64 = 1;
    loop {
        let t = Instant::now();
        for _ in 0..inner {
            f();
        }
        let dt = t.elapsed().as_secs_f64();
        if dt >= min_sample_s || inner >= 1 << 28 {
            break;
        }
        inner *= 2;
    }
    let mut raw = Vec::with_capacity(samples);
    for _ in 0..samples {
        let t = Instant::now();
        for _ in 0..inner {
            f();
        }
        raw.push(t.elapsed().as_nanos() as f64 / inner as f64 / items);
    }
    stat(raw, "ns")
}

pub fn lcg32(n: usize, seed: u32, lo: f64, hi: f64) -> Vec<f64> {
    let mut x = seed;
    (0..n)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            f64::from(x) / 4_294_967_296.0 * (hi - lo) + lo
        })
        .collect()
}

pub fn ordered(x: f64) -> i64 {
    let b = x.to_bits() as i64;
    if b >= 0 { b } else { -(b & 0x7fff_ffff_ffff_ffff) }
}

pub fn ulp_distance(got: f64, want: f64) -> Option<u64> {
    if got.is_finite() && want.is_finite() {
        Some(ordered(got).abs_diff(ordered(want)))
    } else {
        None
    }
}

pub fn split_quantity(t: &str) -> (&str, &str) {
    let t = t.trim();
    t.split_once(' ').map_or((t, ""), |(v, u)| (v, u.trim()))
}

// ---------------------------------------------------------------- environment

pub fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

pub fn cmd_out(prog: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(prog).args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

pub fn machine_info() -> Json {
    let cpu = read("/proc/cpuinfo")
        .lines()
        .find(|l| l.to_lowercase().starts_with("model name"))
        .and_then(|l| l.split_once(':').map(|x| x.1.trim().to_string()))
        .unwrap_or_else(|| "unknown".to_string());
    let gov = read("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor");
    let mut o = Json::obj();
    o.set("os", Json::Str(cmd_out("uname", &["-sr"]).unwrap_or_else(|| std::env::consts::OS.to_string())));
    o.set("cpu_model", Json::Str(cpu));
    o.set("logical_cores", Json::Num(std::thread::available_parallelism().map_or(0, usize::from) as f64));
    o.set("governor", if gov.is_empty() { Json::Null } else { Json::Str(gov.trim().to_string()) });
    o.set("arch", Json::s(std::env::consts::ARCH));
    o
}

pub fn date_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

pub fn peak_rss_kb() -> Json {
    read("/proc/self/status")
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1).and_then(|n| n.parse::<f64>().ok()))
        .map_or(Json::Null, Json::Num)
}

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

