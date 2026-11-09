// SPDX-License-Identifier: MIT OR Apache-2.0
//! The `dimvexa` binary: a thin shell around [`dimvexa_cli::run`].

use std::io::{IsTerminal as _, Write as _};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let mut out = stdout.lock();
    let mut err = stderr.lock();
    // Colour only for a terminal, never with NO_COLOR set or TERM=dumb.
    let allowed = std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
        && std::env::var("TERM").map_or(true, |t| t != "dumb");
    let color = dimvexa_cli::Color {
        out: allowed && stdout.is_terminal(),
        err: allowed && stderr.is_terminal(),
    };
    let code = dimvexa_cli::run_with(args, &mut out, &mut err, color);
    let _ = out.flush();
    ExitCode::from(code)
}
