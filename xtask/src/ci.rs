// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask ci`: the single local entry point that runs what CI runs.
//!
//! The GitHub workflow calls this same command (see `.github/workflows/ci.yml`), so a green
//! `cargo xtask ci` on your machine is the same check as the CI job, with the same flags.
//! Warnings are errors in every stage.
//!
//! ```text
//! cargo xtask ci                  all stages, stop at the first failure
//! cargo xtask ci fmt clippy       only the named stages, in the given order
//! cargo xtask ci --keep-going     run every stage and summarise at the end
//! cargo xtask ci --list           print the stages and their commands
//! ```

use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::Instant;

/// One step of the pipeline.
struct Stage {
    name: &'static str,
    /// What it checks, for `--list`.
    about: &'static str,
    kind: Kind,
}

enum Kind {
    /// An external `cargo` invocation.
    Cargo(&'static [&'static str]),
    /// The in-process unit table validation.
    ValidateData,
    /// `cargo xtask check-docs`, in process.
    CheckDocs,
    /// `cargo check` on the minimum supported Rust version (`rust-version` in `Cargo.toml`), so
    /// that a newer language feature (let chains need 1.88, the MSRV is 1.85) cannot pass here
    /// on the pinned toolchain and fail in CI.
    Msrv,
    /// `cargo package --list` for the data crate must contain the unit tables, so that the
    /// published crate is self-contained.
    PackageList,
}

/// All stages in the order CI runs them. Keep `docs/` and `CONTRIBUTING.md` in step.
const STAGES: &[Stage] = &[
    Stage {
        name: "fmt",
        about: "formatting is canonical",
        kind: Kind::Cargo(&["fmt", "--all", "--check"]),
    },
    Stage {
        name: "clippy",
        about: "lints, warnings are errors",
        kind: Kind::Cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--",
            "-D",
            "warnings",
        ]),
    },
    Stage {
        name: "msrv",
        about: "the workspace compiles on the minimum supported Rust version (cargo check)",
        kind: Kind::Msrv,
    },
    Stage {
        name: "test",
        about: "unit, integration and property tests",
        kind: Kind::Cargo(&[
            "test",
            "--workspace",
            "--all-features",
            "--locked",
            "--profile",
            "ci",
            "--tests",
        ]),
    },
    Stage {
        name: "doctest",
        about: "documentation examples compile and run",
        kind: Kind::Cargo(&[
            "test",
            "--workspace",
            "--all-features",
            "--locked",
            "--profile",
            "ci",
            "--doc",
        ]),
    },
    Stage {
        name: "docs",
        about: "rustdoc builds without warnings",
        kind: Kind::Cargo(&[
            "doc",
            "--workspace",
            "--all-features",
            "--no-deps",
            "--locked",
        ]),
    },
    Stage {
        name: "data",
        about: "unit table: sources, vectors, derivations, hash",
        kind: Kind::ValidateData,
    },
    Stage {
        name: "package",
        about: "the dimvexa-data package contains its unit tables",
        kind: Kind::PackageList,
    },
    Stage {
        name: "check-docs",
        about: "docs, skills and workflows are consistent with the code",
        kind: Kind::CheckDocs,
    },
    Stage {
        name: "example",
        about: "the pool pump cookbook example runs",
        kind: Kind::Cargo(&[
            "run",
            "--locked",
            "--profile",
            "ci",
            "--package",
            "dimvexa",
            "--example",
            "pool_pump",
        ]),
    },
];

/// Names of all stages, in order.
pub fn stage_names() -> Vec<&'static str> {
    STAGES.iter().map(|s| s.name).collect()
}

/// Appends `-D warnings` to an existing flag string without dropping what the user set.
fn with_deny_warnings(existing: Option<String>) -> String {
    match existing {
        Some(s) if s.contains("-D warnings") => s,
        Some(s) if !s.trim().is_empty() => format!("{s} -D warnings"),
        _ => String::from("-D warnings"),
    }
}

fn run_stage(stage: &Stage, root: &Path, validate: &dyn Fn() -> bool) -> bool {
    match &stage.kind {
        Kind::ValidateData => validate(),
        Kind::PackageList => package_list(root),
        Kind::Msrv => msrv_check(root),
        Kind::CheckDocs => {
            let errors = crate::docs_check::run(root);
            for e in &errors {
                eprintln!("    error: {e}");
            }
            if errors.is_empty() {
                println!("    docs, skills and workflows are consistent");
            }
            errors.is_empty()
        }
        Kind::Cargo(args) => {
            println!("    $ cargo {}", args.join(" "));
            let status = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
                .args(*args)
                .current_dir(root)
                .env(
                    "RUSTFLAGS",
                    with_deny_warnings(std::env::var("RUSTFLAGS").ok()),
                )
                .env(
                    "RUSTDOCFLAGS",
                    with_deny_warnings(std::env::var("RUSTDOCFLAGS").ok()),
                )
                .env("CARGO_TERM_COLOR", "always")
                .status();
            match status {
                Ok(s) => s.success(),
                Err(e) => {
                    eprintln!("    cannot start cargo: {e}");
                    false
                }
            }
        }
    }
}

/// The `rust-version` of the workspace, the single source for the MSRV.
fn msrv_version(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    text.lines()
        .find_map(|l| l.trim().strip_prefix("rust-version = "))
        .map(|v| v.trim().trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
}

/// Checks the whole workspace with the MSRV toolchain, installing it with rustup if missing.
fn msrv_check(root: &Path) -> bool {
    let Some(version) = msrv_version(root) else {
        eprintln!("    cannot read rust-version from Cargo.toml");
        return false;
    };
    let has_toolchain = |v: &str| {
        Command::new("rustc")
            .arg(format!("+{v}"))
            .arg("-V")
            .output()
            .is_ok_and(|o| o.status.success())
    };
    if !has_toolchain(&version) {
        println!("    installing the {version} toolchain (rustup toolchain install {version})");
        let installed = Command::new("rustup")
            .args(["toolchain", "install", &version])
            .args(["--profile", "minimal", "--no-self-update"])
            .status()
            .is_ok_and(|s| s.success());
        if !installed {
            eprintln!(
                "    cannot install Rust {version}; install it with: \
                 rustup toolchain install {version} --profile minimal"
            );
            return false;
        }
    }
    let args = [
        "check",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--locked",
    ];
    println!("    $ RUSTUP_TOOLCHAIN={version} cargo {}", args.join(" "));
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(args)
        .current_dir(root)
        .env("RUSTUP_TOOLCHAIN", &version)
        .env("CARGO_TERM_COLOR", "always")
        .status()
        .is_ok_and(|s| s.success())
}

/// Files that must be in the `dimvexa-data` package for it to build on its own.
const PACKAGED_DATA_FILES: &[&str] = &[
    "build.rs",
    "tooling/toml_lite.rs",
    "data/data_version.toml",
    "data/units/base.toml",
    "data/units/common.toml",
    "data/units/ambiguous.toml",
    "data/vectors/base.toml",
    "data/vectors/common.toml",
];

/// Checks the package file list of the data crate.
fn package_list(root: &Path) -> bool {
    let args = [
        "package",
        "--list",
        "--package",
        "dimvexa-data",
        "--locked",
        "--allow-dirty",
    ];
    println!("    $ cargo {}", args.join(" "));
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(args)
        .current_dir(root)
        .output();
    let output = match output {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            eprintln!("{}", String::from_utf8_lossy(&o.stderr));
            return false;
        }
        Err(e) => {
            eprintln!("    cannot start cargo: {e}");
            return false;
        }
    };
    let listing = String::from_utf8_lossy(&output.stdout);
    let files: Vec<&str> = listing
        .lines()
        .map(|l| l.trim().trim_start_matches("./"))
        .collect();
    let missing: Vec<&&str> = PACKAGED_DATA_FILES
        .iter()
        .filter(|f| !files.iter().any(|p| p.replace('\\', "/") == **f))
        .collect();
    if missing.is_empty() {
        println!(
            "    package lists {} files, all required data files present",
            files.len()
        );
        true
    } else {
        eprintln!("    missing from the package: {missing:?}");
        false
    }
}

/// Runs the pipeline. `validate` runs the in-process data validation and reports success.
pub fn run(args: &[String], root: &Path, validate: &dyn Fn() -> bool) -> ExitCode {
    let mut keep_going = false;
    let mut wanted: Vec<&Stage> = Vec::new();
    for a in args {
        match a.as_str() {
            "--keep-going" => keep_going = true,
            "--list" => {
                for s in STAGES {
                    let how = match &s.kind {
                        Kind::Cargo(c) => format!("cargo {}", c.join(" ")),
                        Kind::ValidateData => String::from("cargo xtask validate-data"),
                        Kind::CheckDocs => String::from("cargo xtask check-docs"),
                        Kind::PackageList => String::from(
                            "cargo package --list -p dimvexa-data, then check the file list",
                        ),
                        Kind::Msrv => String::from(
                            "RUSTUP_TOOLCHAIN=<rust-version> cargo check --workspace --all-targets --all-features --locked",
                        ),
                    };
                    println!("{:<8} {} ({how})", s.name, s.about);
                }
                return ExitCode::SUCCESS;
            }
            name => {
                let Some(stage) = STAGES.iter().find(|s| s.name == name) else {
                    let names: Vec<&str> = STAGES.iter().map(|s| s.name).collect();
                    eprintln!(
                        "ci: unknown stage or option '{name}'; stages: {}; options: --keep-going, --list",
                        names.join(", ")
                    );
                    return ExitCode::from(2);
                };
                wanted.push(stage);
            }
        }
    }
    if wanted.is_empty() {
        wanted = STAGES.iter().collect();
    }
    let started = Instant::now();
    let mut failed: Vec<&str> = Vec::new();
    for stage in wanted {
        println!("==> {}: {}", stage.name, stage.about);
        let t = Instant::now();
        let ok = run_stage(stage, root, validate);
        println!(
            "<== {} {} ({:.1}s)",
            stage.name,
            if ok { "ok" } else { "FAILED" },
            t.elapsed().as_secs_f64()
        );
        if !ok {
            failed.push(stage.name);
            if !keep_going {
                break;
            }
        }
    }
    if failed.is_empty() {
        println!(
            "ci: all stages passed ({:.1}s)",
            started.elapsed().as_secs_f64()
        );
        ExitCode::SUCCESS
    } else {
        eprintln!("ci: FAILED: {}", failed.join(", "));
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_warnings_is_appended_once() {
        assert_eq!(with_deny_warnings(None), "-D warnings");
        assert_eq!(with_deny_warnings(Some(String::new())), "-D warnings");
        assert_eq!(
            with_deny_warnings(Some("-C target-cpu=native".into())),
            "-C target-cpu=native -D warnings"
        );
        assert_eq!(
            with_deny_warnings(Some("-D warnings".into())),
            "-D warnings"
        );
    }

    #[test]
    fn stage_names_are_unique_and_include_the_documented_set() {
        let names: Vec<&str> = STAGES.iter().map(|s| s.name).collect();
        for n in [
            "fmt", "clippy", "msrv", "test", "doctest", "docs", "data", "example",
        ] {
            assert!(names.contains(&n), "{n}");
        }
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len());
    }
}
