// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask bench-check`: the performance regression guard for the hot-path rows of the M0
//! benchmark (`crates/dimvexa/benches/m0.rs`), as the `benchmarks` skill asks.
//!
//! ```text
//! cargo xtask bench-check                  run the bench, compare with the baseline
//! cargo xtask bench-check --update         re-record the baseline (3 runs, median per row)
//! cargo xtask bench-check --input FILE     compare a saved bench output, no run, no rerun
//! cargo xtask bench-check --threshold 15   allowed slowdown in percent (default 10)
//! cargo xtask bench-check --runs 5         at most this many runs when a row looks slow
//! cargo xtask bench-check --against REV    A/B: REV against the working tree, same machine
//! cargo xtask bench-check --against REV --layouts 1   no layout escalation (see below)
//! ```
//!
//! **A/B mode (`--against REV`, the one to use on a shared cloud VM).** The stored baseline below
//! is only valid on the machine and compiler that recorded it, and a cloud VM is a different
//! machine each time. `--against` needs no baseline: it checks `REV` out into a temporary git
//! worktree, builds its `m0` bench there, and runs the two builds one after the other, `REV` then
//! the working tree, in rounds, so slow drift of the machine hits both alike. A row is judged by
//! its best median per build over all rounds, with the same threshold and floor as below. Round 1
//! always runs; further rounds (up to `--runs`, default 3) only run while a guarded row looks
//! slow.
//!
//! **Layout escalation.** On tiny rows (one call, a 1000-element slice, 0.04 ns per element) the
//! alignment of a loop decides the result, and a rebuild that changes no code can move a row by
//! 20 percent: on 2026-11-10 a version bump alone did (`docs/benchmarks/M0.md`). A real slowdown
//! shifts every layout, an alignment effect does not. So if a guarded row is still slow after the
//! rounds, both sides are rebuilt under another crate-metadata salt (`RUSTFLAGS="-C
//! metadata=bench-layout-K"`, which changes symbol hashes and with them the link order) and run
//! once more, up to `--layouts` layouts in total (default 5), and every row is judged by its
//! best median per side over all layouts. The same layouts are tried on both sides, so the
//! comparison stays fair; a row that is slow in every layout is a regression. The working tree may be dirty: what it contains is what is measured. The worktree is
//! removed afterwards, also on failure; its build output lives in `target/bench-ab-base` so a
//! second run is incremental.
//!
//! Baseline: `docs/benchmarks/baseline-m0.tsv` (`name<TAB>median ns/op`, one row per bench row,
//! with the machine and compiler in `#` comment lines). It is only comparable on the machine
//! that recorded it: record a new one with `--update` when the machine or toolchain changes, and
//! say so in the commit message.
//!
//! Guarded rows: scalar apply (throughput, latency chain, single call), `apply_slice`,
//! `apply_in_place` and the exact `f64` path (`apply_exact_f64`). Other rows are recorded in the
//! baseline but never fail the check.
//!
//! Noise handling (shared cloud machines are noisy, noise only ever adds time):
//!
//! 1. A row is *slow* when its time exceeds `baseline * (1 + threshold)` **and** exceeds the
//!    baseline by more than [`ABS_FLOOR_NS`] (sub-nanosecond rows jitter by that much).
//! 2. If any guarded row is slow, the whole bench is rerun, up to `--runs` runs in total
//!    (default 3). Each row is judged by its **best** (smallest) median over all runs so far.
//! 3. The check fails only if a row is still slow after the last run. A slowdown that survives
//!    three independent runs is real; a noisy neighbour does not.
//! 4. `--update` records the median of three runs per row, so the baseline is typical, not lucky.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::indexing_slicing,
    clippy::float_arithmetic
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::process::{Command, ExitCode};

/// Default allowed slowdown, in percent.
const DEFAULT_THRESHOLD_PCT: f64 = 10.0;
/// Differences below this many ns are never a regression (timer and scheduler jitter).
const ABS_FLOOR_NS: f64 = 0.03;
/// Default maximum number of runs per check.
const DEFAULT_RUNS: usize = 3;
/// Default number of code layouts an A/B check may try before it calls a row slow.
const DEFAULT_LAYOUTS: usize = 5;
/// Number of runs a baseline update takes the median of.
const UPDATE_RUNS: usize = 3;
/// Baseline path relative to the repository root.
const BASELINE: &str = "docs/benchmarks/baseline-m0.tsv";

/// Row-name prefixes that are guarded.
const GUARDED: &[&str] = &[
    "scalar linear apply",
    "scalar affine apply",
    "single call via",
    "slice linear apply_slice",
    "slice affine apply_slice",
    "slice linear apply_in_place",
    "apply_exact_f64",
];

type Rows = BTreeMap<String, f64>;

fn guarded(name: &str) -> bool {
    GUARDED.iter().any(|p| name.starts_with(p))
}

/// Parses the Markdown table printed by the m0 bench into `name -> median ns/op`.
fn parse_table(text: &str) -> Rows {
    let mut rows = Rows::new();
    for line in text.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // "| name | median | min | max | note |" splits into 7 cells with empty ends.
        if cells.len() >= 4 && line.starts_with('|') {
            if let Ok(v) = cells[2].parse::<f64>() {
                rows.insert(cells[1].to_string(), v);
            }
        }
    }
    rows
}

fn parse_baseline(text: &str) -> Rows {
    let mut rows = Rows::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        if let Some((name, v)) = line.split_once('\t') {
            if let Ok(v) = v.trim().parse::<f64>() {
                rows.insert(name.to_string(), v);
            }
        }
    }
    rows
}

/// Is `now` a slowdown against `base` beyond the threshold and the absolute floor?
fn slow(base: f64, now: f64, threshold_pct: f64) -> bool {
    now > base * (1.0 + threshold_pct / 100.0) && now - base > ABS_FLOOR_NS
}

/// Per-row minimum over several runs.
fn best_of(runs: &[Rows]) -> Rows {
    let mut best = Rows::new();
    for run in runs {
        for (k, v) in run {
            best.entry(k.clone())
                .and_modify(|b| *b = b.min(*v))
                .or_insert(*v);
        }
    }
    best
}

/// Per-row median over several runs.
fn median_of(runs: &[Rows]) -> Rows {
    let mut all: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for run in runs {
        for (k, v) in run {
            all.entry(k.clone()).or_default().push(*v);
        }
    }
    all.into_iter()
        .map(|(k, mut v)| {
            v.sort_by(f64::total_cmp);
            (k, v[v.len() / 2])
        })
        .collect()
}

/// Guarded rows that are slow against the baseline, as `(name, base, now)`.
fn slow_rows(base: &Rows, now: &Rows, threshold_pct: f64) -> Vec<(String, f64, f64)> {
    base.iter()
        .filter(|(k, _)| guarded(k))
        .filter_map(|(k, b)| now.get(k).map(|n| (k.clone(), *b, *n)))
        .filter(|(_, b, n)| slow(*b, *n, threshold_pct))
        .collect()
}

fn run_bench(root: &Path) -> Result<Rows, String> {
    println!("bench-check: running cargo bench -p dimvexa --bench m0 (about a minute)");
    run_bench_in(root, None)
}

/// Runs the m0 bench in `dir`, optionally with its own cargo target directory.
fn run_bench_in(dir: &Path, target_dir: Option<&Path>) -> Result<Rows, String> {
    run_bench_layout(dir, target_dir, 0)
}

/// Like [`run_bench_in`], in layout `layout` (0 is the normal build; another number rebuilds with
/// a different crate-metadata salt, so symbol hashes and link order differ; code does not).
fn run_bench_layout(dir: &Path, target_dir: Option<&Path>, layout: usize) -> Result<Rows, String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["bench", "-p", "dimvexa", "--bench", "m0", "--locked"])
        .current_dir(dir);
    if let Some(t) = target_dir {
        cmd.env("CARGO_TARGET_DIR", t);
    }
    if layout > 0 {
        cmd.env("RUSTFLAGS", format!("-C metadata=bench-layout-{layout}"));
    }
    let out = cmd.output().map_err(|e| format!("cannot run cargo: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "bench failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let rows = parse_table(&String::from_utf8_lossy(&out.stdout));
    if rows.is_empty() {
        return Err("no benchmark table in the bench output".to_string());
    }
    Ok(rows)
}

fn rustc_version() -> String {
    Command::new("rustc").arg("--version").output().map_or_else(
        |_| "rustc unknown".to_string(),
        |o| String::from_utf8_lossy(&o.stdout).trim().to_string(),
    )
}

fn write_baseline(root: &Path, rows: &Rows) -> Result<(), String> {
    let mut s = String::new();
    s.push_str(
        "# Baseline for `cargo xtask bench-check` (median ns/op of 3 runs of the m0 bench).\n",
    );
    s.push_str(
        "# Only comparable on the machine that recorded it; see xtask/src/bench_check.rs.\n",
    );
    let _ = writeln!(s, "# compiler: {}", rustc_version());
    let _ = writeln!(
        s,
        "# machine: {} logical CPUs, shared VM; profile: bench (release, thin LTO)\n",
        std::thread::available_parallelism().map_or(0, std::num::NonZero::get)
    );
    for (k, v) in rows {
        let _ = writeln!(s, "{k}\t{v:.3}");
    }
    std::fs::write(root.join(BASELINE), s).map_err(|e| format!("cannot write baseline: {e}"))
}

struct Opts {
    update: bool,
    against: Option<String>,
    layouts: usize,
    input: Option<String>,
    threshold: f64,
    runs: usize,
}

fn parse_opts(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts {
        update: false,
        against: None,
        layouts: DEFAULT_LAYOUTS,
        input: None,
        threshold: DEFAULT_THRESHOLD_PCT,
        runs: DEFAULT_RUNS,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--update" => o.update = true,
            "--against" => {
                o.against = Some(it.next().ok_or("--against needs a git revision")?.clone());
            }
            "--layouts" => {
                o.layouts = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|n| *n >= 1)
                    .ok_or("--layouts needs a positive integer")?;
            }
            "--input" => o.input = Some(it.next().ok_or("--input needs a file")?.clone()),
            "--threshold" => {
                o.threshold = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("--threshold needs a number (percent)")?;
            }
            "--runs" => {
                o.runs = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|n| *n >= 1)
                    .ok_or("--runs needs a positive integer")?;
            }
            other => return Err(format!("unknown option {other}")),
        }
    }
    if o.against.is_some() && (o.update || o.input.is_some()) {
        return Err("--against cannot be combined with --update or --input".to_string());
    }
    Ok(o)
}

fn report(base: &Rows, now: &Rows, threshold: f64, base_label: &str) {
    println!(
        "{:<48} {:>10} {:>10} {:>8}",
        "guarded row", base_label, "now", "change"
    );
    for (k, b) in base.iter().filter(|(k, _)| guarded(k)) {
        if let Some(n) = now.get(k) {
            let flag = if slow(*b, *n, threshold) {
                "  SLOW"
            } else {
                ""
            };
            println!(
                "{:<48} {:>10.3} {:>10.3} {:>+7.1}%{flag}",
                k,
                b,
                n,
                (n / b - 1.0) * 100.0
            );
        }
    }
}

/// Output of a git command in `root`, or the reason it failed.
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// A temporary detached worktree of a revision, removed on drop (also after an error).
struct Worktree {
    root: std::path::PathBuf,
    dir: std::path::PathBuf,
}

impl Worktree {
    fn create(root: &Path, rev: &str) -> Result<Self, String> {
        let commit = git(
            root,
            &["rev-parse", "--verify", &format!("{rev}^{{commit}}")],
        )?;
        let dir = root.join("target").join("bench-ab-worktree");
        // A leftover from an interrupted run.
        let _ = git(
            root,
            &["worktree", "remove", "--force", &dir.to_string_lossy()],
        );
        let _ = std::fs::remove_dir_all(&dir);
        git(
            root,
            &[
                "worktree",
                "add",
                "--detach",
                &dir.to_string_lossy(),
                &commit,
            ],
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            dir,
        })
    }
}

impl Drop for Worktree {
    fn drop(&mut self) {
        let _ = git(
            &self.root,
            &["worktree", "remove", "--force", &self.dir.to_string_lossy()],
        );
        let _ = git(&self.root, &["worktree", "prune"]);
    }
}

/// A/B check: `rev` against the working tree on this machine, in alternating rounds.
fn execute_against(rev: &str, o: &Opts, root: &Path) -> Result<bool, String> {
    let short = git(
        root,
        &["rev-parse", "--short", &format!("{rev}^{{commit}}")],
    )?;
    let wt = Worktree::create(root, rev)?;
    let target = root.join("target").join("bench-ab-base");
    println!(
        "bench-check: A/B on this machine: {rev} ({short}) against the working tree, alternating rounds"
    );
    let (mut a_runs, mut b_runs): (Vec<Rows>, Vec<Rows>) = (Vec::new(), Vec::new());
    loop {
        println!(
            "bench-check: round {} of at most {}: {short}, then the working tree",
            a_runs.len() + 1,
            o.runs
        );
        a_runs.push(run_bench_in(&wt.dir, Some(&target))?);
        b_runs.push(run_bench(root)?);
        let (a, b) = (best_of(&a_runs), best_of(&b_runs));
        if a_runs.len() >= o.runs || slow_rows(&a, &b, o.threshold).is_empty() {
            break;
        }
    }
    // Layout escalation: a row still slow may be alignment, not code. Rebuild both sides under
    // another metadata salt and keep the best median per side over every layout tried.
    let mut layouts_used = 1;
    // Best median per side of each layout tried; layout 0 is the rounds above.
    let mut per_layout: Vec<(Rows, Rows)> = vec![(best_of(&a_runs), best_of(&b_runs))];
    for layout in 1..o.layouts {
        let (a, b) = (best_of(&a_runs), best_of(&b_runs));
        if slow_rows(&a, &b, o.threshold).is_empty() {
            break;
        }
        println!(
            "bench-check: a guarded row is still slow; trying code layout {} of {} (same code, other link order)",
            layout + 1,
            o.layouts
        );
        let ta = root
            .join("target")
            .join(format!("bench-ab-base-layout{layout}"));
        let tb = root
            .join("target")
            .join(format!("bench-ab-tree-layout{layout}"));
        let ra = run_bench_layout(&wt.dir, Some(&ta), layout)?;
        let rb = run_bench_layout(root, Some(&tb), layout)?;
        per_layout.push((ra.clone(), rb.clone()));
        a_runs.push(ra);
        b_runs.push(rb);
        layouts_used += 1;
    }
    let (a, b) = (best_of(&a_runs), best_of(&b_runs));
    let missing: Vec<&String> = a
        .keys()
        .filter(|k| guarded(k) && !b.contains_key(*k))
        .collect();
    report(&a, &b, o.threshold, &short);
    for m in &missing {
        eprintln!(
            "error: guarded row `{m}` exists in {rev} but not in the working tree (renamed?)"
        );
    }
    let slow = slow_rows(&a, &b, o.threshold);
    if layouts_used > 1 {
        for (k, _, _) in &slow {
            let cells: Vec<String> = per_layout
                .iter()
                .enumerate()
                .filter_map(|(i, (la, lb))| {
                    Some(format!("L{i} {:.3}|{:.3}", la.get(k)?, lb.get(k)?))
                })
                .collect();
            println!(
                "bench-check: `{k}` per layout ({short}|now): {}",
                cells.join("  ")
            );
        }
    }
    println!(
        "bench-check: {} run(s) per side in {layouts_used} layout(s), threshold {}%, floor {ABS_FLOOR_NS} ns, best median per row and side",
        a_runs.len(),
        o.threshold
    );
    for (k, x, y) in &slow {
        eprintln!(
            "error: `{k}` is {:.1}% slower than {rev} in {layouts_used} layout(s) ({x:.3} -> {y:.3} ns/op); explain it or fix it",
            (y / x - 1.0) * 100.0
        );
    }
    Ok(slow.is_empty() && missing.is_empty())
}

fn execute(args: &[String], root: &Path) -> Result<bool, String> {
    let o = parse_opts(args)?;
    if let Some(rev) = &o.against {
        return execute_against(rev, &o, root);
    }
    if o.update {
        let runs: Result<Vec<Rows>, String> = (0..UPDATE_RUNS).map(|_| run_bench(root)).collect();
        let rows = median_of(&runs?);
        write_baseline(root, &rows)?;
        println!("bench-check: wrote {BASELINE} ({} rows)", rows.len());
        return Ok(true);
    }
    let base_text = std::fs::read_to_string(root.join(BASELINE))
        .map_err(|e| format!("cannot read {BASELINE} (record it with --update): {e}"))?;
    let base = parse_baseline(&base_text);
    let mut runs: Vec<Rows> = Vec::new();
    if let Some(path) = &o.input {
        let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        runs.push(parse_table(&text));
    } else {
        runs.push(run_bench(root)?);
        while runs.len() < o.runs && !slow_rows(&base, &best_of(&runs), o.threshold).is_empty() {
            println!(
                "bench-check: a guarded row looks slow, rerunning ({} of {})",
                runs.len() + 1,
                o.runs
            );
            runs.push(run_bench(root)?);
        }
    }
    let best = best_of(&runs);
    let missing: Vec<&String> = base
        .keys()
        .filter(|k| guarded(k) && !best.contains_key(*k))
        .collect();
    report(&base, &best, o.threshold, "baseline");
    for m in &missing {
        eprintln!(
            "error: guarded row `{m}` is in the baseline but not in the bench output (renamed? re-record with --update)"
        );
    }
    let slow = slow_rows(&base, &best, o.threshold);
    println!(
        "bench-check: {} run(s), threshold {}%, floor {ABS_FLOOR_NS} ns, best median per row",
        runs.len(),
        o.threshold
    );
    for (k, b, n) in &slow {
        eprintln!(
            "error: `{k}` regressed {:.1}% ({b:.3} -> {n:.3} ns/op); explain it or fix it",
            (n / b - 1.0) * 100.0
        );
    }
    Ok(slow.is_empty() && missing.is_empty())
}

/// Entry point for `cargo xtask bench-check`.
pub fn run(args: &[String], root: &Path) -> ExitCode {
    match execute(args, root) {
        Ok(true) => {
            println!("bench-check: ok");
            ExitCode::SUCCESS
        }
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("bench-check: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_bench_table() {
        let t = "| benchmark | median ns/op | min | max | note |\n|---|---:|---:|---:|---|\n| scalar linear apply (throughput) | 0.625 | 0.594 | 0.730 | x |\n";
        let rows = parse_table(t);
        assert_eq!(rows.len(), 1);
        assert!((rows["scalar linear apply (throughput)"] - 0.625).abs() < 1e-12);
    }

    #[test]
    fn threshold_and_floor() {
        assert!(!slow(100.0, 109.0, 10.0));
        assert!(slow(100.0, 111.0, 10.0));
        // 50 percent but only 0.01 ns: jitter, not a regression
        assert!(!slow(0.02, 0.03, 10.0));
    }

    #[test]
    fn best_and_median() {
        let mk = |v: f64| Rows::from([("a".to_string(), v)]);
        let runs = [mk(3.0), mk(1.0), mk(2.0)];
        assert!((best_of(&runs)["a"] - 1.0).abs() < 1e-12);
        assert!((median_of(&runs)["a"] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn against_parses_and_excludes_baseline_modes() {
        let a = |v: &[&str]| parse_opts(&v.iter().map(|s| (*s).to_string()).collect::<Vec<_>>());
        assert_eq!(
            a(&["--against", "main"]).map(|o| o.against),
            Ok(Some("main".to_string()))
        );
        assert!(a(&["--against"]).is_err());
        assert!(a(&["--against", "main", "--update"]).is_err());
        assert_eq!(a(&["--layouts", "1"]).map(|o| o.layouts), Ok(1));
        assert!(a(&["--layouts", "0"]).is_err());
        assert!(a(&["--against", "main", "--input", "f"]).is_err());
    }

    #[test]
    fn only_guarded_rows_are_flagged() {
        let base = Rows::from([
            ("slice linear apply_slice 10^6".to_string(), 1.0),
            ("Dim::checked_add".to_string(), 1.0),
        ]);
        let now = Rows::from([
            ("slice linear apply_slice 10^6".to_string(), 2.0),
            ("Dim::checked_add".to_string(), 9.0),
        ]);
        let s = slow_rows(&base, &now, 10.0);
        assert_eq!(s.len(), 1);
        assert!(s[0].0.starts_with("slice linear"));
    }
}
