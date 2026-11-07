// SPDX-License-Identifier: MIT OR Apache-2.0
//! The `dimvexa` binary: a thin shell around [`dimvexa_cli::run`].

use std::io::Write as _;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let mut out = stdout.lock();
    let mut err = stderr.lock();
    let code = dimvexa_cli::run(args, &mut out, &mut err);
    let _ = out.flush();
    ExitCode::from(code)
}
