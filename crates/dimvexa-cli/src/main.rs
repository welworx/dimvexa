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
    // Colour: never with NO_COLOR or TERM=dumb; otherwise on a terminal, or everywhere with
    // CLICOLOR_FORCE set to something other than 0. `--color` overrides all of this.
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty());
    let allowed = var("NO_COLOR").is_none() && std::env::var("TERM").map_or(true, |t| t != "dumb");
    let force = var("CLICOLOR_FORCE").is_some_and(|v| v != "0");
    let color = dimvexa_cli::Color {
        out: allowed && (force || stdout.is_terminal()),
        err: allowed && (force || stderr.is_terminal()),
    };
    let code = dimvexa_cli::run_with(args, &mut out, &mut err, color);
    let _ = out.flush();
    ExitCode::from(code)
}
