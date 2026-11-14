// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask bench-impact`: what did my change do to the speed of the library?
//!
//! ```text
//! cargo xtask bench-impact                 decide from the diff, then A/B against the merge base
//! cargo xtask bench-impact --base REV      compare with REV instead of the merge base
//! cargo xtask bench-impact --force         benchmark even if no hot-path file changed
//! cargo xtask bench-impact --dry-run       only print the decision
//! cargo xtask bench-impact --threshold 15 --runs 5 --layouts 5   passed on to bench-check
//! ```
//!
//! One question, answered once per change, so the benchmark is neither run for everything nor
//! forgotten. It lists the files changed since the merge base with `origin/main` (committed,
//! staged, unstaged and new files). If none of them is a hot-path file ([`HOT_PATHS`]) it says
//! so and exits 0: nothing to measure, and the skip is on the record. Otherwise it runs
//! `bench-check --against BASE` (an A/B run of the guarded rows on this machine, no baseline, no
//! old releases) and exits non-zero when a guarded row is more than the threshold slower than the
//! base outside the noise. The `benchmarks` skill says what to do then: explain it, or have a
//! fresh Opus subagent look for a faster implementation. Paste the output into the pull request.
//!
//! It never runs the release matrix or the cross-library comparison; those stay manual
//! (`bench-release` runs once per release, `bench-matrix` and `benchmarks/run_all.sh` are manual).
#![allow(clippy::arithmetic_side_effects)]

use std::path::Path;
use std::process::ExitCode;

use crate::bench_check;

/// Files whose change can move a guarded row of `crates/dimvexa/benches/m0.rs`: the conversion
/// core, the benchmark itself, and what changes how everything is built. A path ending in `/` is
/// a directory prefix, any other entry is one file. Keep this list true: a test checks that every
/// entry exists, so a rename cannot silently turn the guard off.
pub const HOT_PATHS: &[&str] = &[
    "crates/dimvexa-core/src/",
    // The guarded exact rows convert `deg -> rad` and `eV -> J` from the compiled unit table; the
    // stored ratio decides which branch of `apply_exact_f64` runs. A missed regression is silent,
    // a needless run is not, so the table and what compiles it count.
    "crates/dimvexa-data/data/units/",
    "crates/dimvexa-data/data/constants/",
    "crates/dimvexa-data/build.rs",
    "crates/dimvexa-data/src/",
    "crates/dimvexa/src/",
    "crates/dimvexa/benches/",
    ".cargo/",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
];

/// The changed files that are hot-path files.
fn hot_files(changed: &[String]) -> Vec<&str> {
    changed
        .iter()
        .map(String::as_str)
        .filter(|f| {
            HOT_PATHS.iter().any(|h| {
                if h.ends_with('/') {
                    f.starts_with(h)
                } else {
                    f == h
                }
            })
        })
        .collect()
}

#[derive(Debug, PartialEq, Default)]
struct Opts {
    base: Option<String>,
    force: bool,
    dry_run: bool,
    /// Options passed on to `bench-check` unchanged.
    pass: Vec<String>,
}

fn parse_opts(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--base" => o.base = Some(it.next().ok_or("--base needs a git revision")?.clone()),
            "--force" => o.force = true,
            "--dry-run" => o.dry_run = true,
            "--threshold" | "--runs" | "--layouts" => {
                let v = it.next().ok_or_else(|| format!("{a} needs a value"))?;
                o.pass.push(a.clone());
                o.pass.push(v.clone());
            }
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(o)
}

/// The revision to compare with: the merge base of HEAD with `origin/main`, or `main`.
fn merge_base(root: &Path) -> Result<(String, &'static str), String> {
    for branch in ["origin/main", "main"] {
        if let Ok(sha) = bench_check::git(root, &["merge-base", "HEAD", branch]) {
            if !sha.is_empty() {
                return Ok((sha, branch));
            }
        }
    }
    Err(
        "cannot find a merge base with origin/main or main (shallow clone?); pass --base REV"
            .to_string(),
    )
}

/// Files changed since `base`: committed, staged and unstaged changes, and new untracked files.
fn changed_files(root: &Path, base: &str) -> Result<Vec<String>, String> {
    // `-z` and `core.quotePath=false` keep non-ASCII names intact; `--no-renames` lists both the
    // old and the new path, so a file moved out of a hot path still counts.
    let mut files: Vec<String> = bench_check::git(
        root,
        &[
            "-c",
            "core.quotePath=false",
            "diff",
            "--name-only",
            "--no-renames",
            "-z",
            base,
        ],
    )?
    .split('\0')
    .map(str::to_string)
    .collect();
    files.extend(
        bench_check::git(
            root,
            &[
                "-c",
                "core.quotePath=false",
                "ls-files",
                "--others",
                "--exclude-standard",
                "-z",
            ],
        )?
        .split('\0')
        .map(str::to_string),
    );
    files.retain(|f| !f.is_empty());
    files.sort();
    files.dedup();
    Ok(files)
}

fn execute(args: &[String], root: &Path) -> Result<bool, String> {
    let o = parse_opts(args)?;
    let (base, label) = match &o.base {
        Some(rev) => (rev.clone(), "--base"),
        None => merge_base(root)?,
    };
    let short = bench_check::git(
        root,
        &["rev-parse", "--short", &format!("{base}^{{commit}}")],
    )?;
    let head = bench_check::git(root, &["rev-parse", "--short", "HEAD"])?;
    let changed = changed_files(root, &base)?;
    let hot = hot_files(&changed);
    println!(
        "bench-impact: base {short} ({label}), HEAD {head}, {} file(s) changed",
        changed.len()
    );
    if hot.is_empty() && !o.force {
        println!(
            "bench-impact: skipped: no hot-path file changed (hot paths: {}). Nothing to benchmark; \
             say so in the pull request.",
            HOT_PATHS.join(", ")
        );
        return Ok(true);
    }
    if hot.is_empty() {
        println!("bench-impact: --force: no hot-path file changed, benchmarking anyway");
    } else {
        println!("bench-impact: hot-path file(s) changed:");
        for f in &hot {
            println!("  {f}");
        }
    }
    if o.dry_run {
        println!("bench-impact: dry run, not benchmarking");
        return Ok(true);
    }
    let mut check: Vec<String> = vec!["--against".to_string(), base];
    check.extend(o.pass);
    let ok = bench_check::execute(&check, root)?;
    println!(
        "bench-impact: machine {} {}; paste this output into the pull request",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    if !ok {
        println!(
            "bench-impact: a guarded row is slower than the threshold. Explain it in the pull \
             request, or look for a faster implementation (a fresh Opus subagent, see the \
             `benchmarks` skill), then run this again."
        );
    }
    Ok(ok)
}

/// Entry point for `cargo xtask bench-impact`.
pub fn run(args: &[String], root: &Path) -> ExitCode {
    match execute(args, root) {
        Ok(true) => {
            println!("bench-impact: ok");
            ExitCode::SUCCESS
        }
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("bench-impact: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn only_hot_files_count() {
        let changed = v(&[
            "crates/dimvexa-core/src/conversion.rs",
            "crates/dimvexa-core/tests/x.rs",
            "crates/dimvexa-data/data/vectors/common.toml",
            "crates/dimvexa-config/src/lib.rs",
            "crates/dimvexa-cli/src/lib.rs",
            "docs/guide.md",
            "Cargo.toml",
            "crates/dimvexa/Cargo.toml",
            "skills/review/SKILL.md",
        ]);
        assert_eq!(
            hot_files(&changed),
            vec!["crates/dimvexa-core/src/conversion.rs", "Cargo.toml"]
        );
        assert_eq!(
            hot_files(&v(&[
                "crates/dimvexa-data/data/units/common.toml",
                "crates/dimvexa-data/build.rs"
            ])),
            vec![
                "crates/dimvexa-data/data/units/common.toml",
                "crates/dimvexa-data/build.rs"
            ]
        );
        assert_eq!(
            hot_files(&v(&["docs/guide.md", "crates/dimvexa-core/README.md"])),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn the_benchmark_file_is_hot() {
        assert_eq!(
            hot_files(&v(&["crates/dimvexa/benches/m0.rs"])),
            vec!["crates/dimvexa/benches/m0.rs"]
        );
    }

    #[test]
    fn every_hot_path_exists() {
        let root = crate::repo_root();
        for h in HOT_PATHS {
            assert!(root.join(h).exists(), "HOT_PATHS entry {h} does not exist");
        }
    }

    #[test]
    fn parses_options() {
        let a = |x: &[&str]| parse_opts(&v(x));
        assert_eq!(a(&[]), Ok(Opts::default()));
        let o = a(&[
            "--base",
            "main",
            "--force",
            "--runs",
            "5",
            "--threshold",
            "15",
        ])
        .unwrap();
        assert_eq!(o.base.as_deref(), Some("main"));
        assert!(o.force);
        assert_eq!(o.pass, v(&["--runs", "5", "--threshold", "15"]));
        assert!(a(&["--base"]).is_err());
        assert!(a(&["--runs"]).is_err());
        assert!(a(&["--update"]).is_err());
    }
}
