// SPDX-License-Identifier: MIT OR Apache-2.0
//! Repository automation: `cargo xtask validate-data` and `cargo xtask update-data-hash`.
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

mod chains;
mod validate;

use std::path::PathBuf;
use std::process::ExitCode;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn main() -> ExitCode {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    let root = repo_root();
    match cmd.as_str() {
        "validate-data" => match validate::run(&root) {
            Ok(report) => {
                println!("{}", report.summary());
                if report.errors.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    for e in &report.errors {
                        eprintln!("error: {e}");
                    }
                    eprintln!("validate-data: {} error(s)", report.errors.len());
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("validate-data: cannot run: {e}");
                ExitCode::FAILURE
            }
        },
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
            eprintln!("usage: cargo xtask <validate-data | update-data-hash>");
            ExitCode::from(2)
        }
    }
}
