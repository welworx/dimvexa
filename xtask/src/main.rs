// SPDX-License-Identifier: MIT OR Apache-2.0
//! Repository automation: `cargo xtask ci`, `check-docs`, `bench-check`,
//! `validate-data` and `update-data-hash`.
//!
//! `validate-data` is the gatekeeper of the unit table. It fails on a missing source,
//! exactness, dimension, note or golden vector, on duplicate symbols, ambiguous bare
//! symbols used as units, non-reduced or non-positive rationals, broken derivation chains
//! (psi, atm, Btu and friends) and on a table whose hash no longer matches the recorded one.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::unnecessary_wraps,
    dead_code,
    unreachable_pub
)]

#[path = "../../crates/dimvexa-data/tooling/toml_lite.rs"]
mod toml_lite;

mod bench_check;
mod bench_matrix;
mod chains;
mod ci;
mod docs_check;
mod validate;

use std::path::PathBuf;
use std::process::ExitCode;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Runs the unit table validation and prints the report. Returns true on success.
fn validate_data(root: &std::path::Path) -> bool {
    match validate::run(root) {
        Ok(report) => {
            println!("{}", report.summary());
            if report.errors.is_empty() {
                true
            } else {
                for e in &report.errors {
                    eprintln!("error: {e}");
                }
                eprintln!("validate-data: {} error(s)", report.errors.len());
                false
            }
        }
        Err(e) => {
            eprintln!("validate-data: cannot run: {e}");
            false
        }
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_default();
    let rest: Vec<String> = args.collect();
    let root = repo_root();
    match cmd.as_str() {
        "ci" => ci::run(&rest, &root, &|| validate_data(&root)),
        "validate-data" => {
            if validate_data(&root) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        "bench-check" => bench_check::run(&rest, &root),
        "bench-matrix" => bench_matrix::run(&rest, &root),
        "check-docs" => {
            let errors = docs_check::run(&root);
            for e in &errors {
                eprintln!("error: {e}");
            }
            if errors.is_empty() {
                println!("check-docs: ok");
                ExitCode::SUCCESS
            } else {
                eprintln!("check-docs: {} problem(s)", errors.len());
                ExitCode::FAILURE
            }
        }
        "update-data-hash" => match validate::update_hash(&root) {
            Ok(msg) => {
                println!("{msg}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("update-data-hash: {e}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!(
                "usage: cargo xtask <ci [STAGE...] | check-docs | bench-check | bench-matrix | validate-data | update-data-hash>"
            );
            ExitCode::from(2)
        }
    }
}
