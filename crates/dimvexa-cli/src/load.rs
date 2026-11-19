// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading config files and building the registry that applies them.
// Private module: `pub` here is crate-visible, and the two visibility lints disagree about it.
#![allow(unreachable_pub)]

use crate::json::json_error;
use dimvexa::{Error, registry};
use dimvexa_config::Config;
use std::io::Write;

/// A registry with the user's configs applied, and which configs those were.
pub struct Loaded {
    /// The registry, the names, units and constants in effect (shared with the bindings).
    pub lib: dimvexa_config::Loaded,
    /// `(origin, content hash)` of each config, in the order given. The origin is the file
    /// path, or `bundled:NAME` for a config that ships with dimvexa.
    pub configs: Vec<(String, String)>,
}

/// Why the registry could not be built.
pub enum LoadError {
    /// A config file could not be read (a usage problem).
    Io(String),
    /// The table or a config was refused by the library.
    Lib(Error),
}

/// The largest config file we read: a config is a handful of lines, so anything bigger is
/// a mistake (or a device such as `/dev/zero`), not a config.
pub const MAX_CONFIG_BYTES: u64 = 1 << 20;

/// Reads a config file: a regular file of at most [`MAX_CONFIG_BYTES`], valid UTF-8.
pub fn read_config(file: &str) -> Result<String, LoadError> {
    use std::io::Read as _;
    let fail = |why: String| LoadError::Io(format!("cannot read config '{file}': {why}"));
    let meta = std::fs::metadata(file).map_err(|e| fail(e.to_string()))?;
    if !meta.is_file() {
        return Err(fail("not a regular file".to_string()));
    }
    let mut text = String::new();
    std::fs::File::open(file)
        .map_err(|e| fail(e.to_string()))?
        .take(MAX_CONFIG_BYTES.saturating_add(1))
        .read_to_string(&mut text)
        .map_err(|e| fail(e.to_string()))?;
    if u64::try_from(text.len()).map_or(true, |n| n > MAX_CONFIG_BYTES) {
        return Err(fail(format!("larger than {MAX_CONFIG_BYTES} bytes")));
    }
    Ok(text)
}

/// Reads one `--config` value: a shipped config by name, or a file by path (ADR-014). A bare
/// word is always a shipped name; a file called `us` is written `./us`.
pub fn read_spec(spec: &str) -> Result<Config, LoadError> {
    let config = if dimvexa_config::is_path(spec) {
        let text = read_config(spec)?;
        Config::parse(&text, spec).map_err(LoadError::Lib)?
    } else {
        dimvexa_config::bundled(spec).map_or_else(
            || {
                Err(LoadError::Io(format!(
                    "no config named '{spec}'; shipped configs: {} (a file needs a path, for example ./{spec} or {spec}.toml)",
                    bundled_names()
                )))
            },
            |b| b.config().map_err(LoadError::Lib),
        )?
    };
    // A built-in constant cannot be redefined: a user-defined pi would change every angle.
    let builtin: Vec<&str> = dimvexa::CONSTANTS.iter().map(|c| c.id).collect();
    dimvexa_config::reject_builtin_constants(&config, &builtin).map_err(LoadError::Lib)?;
    Ok(config)
}

pub fn bundled_names() -> String {
    dimvexa_config::BUNDLED
        .iter()
        .map(|b| b.name)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn load(config_specs: &[String]) -> Result<Loaded, LoadError> {
    let reg = registry().map_err(LoadError::Lib)?;
    let (mut parsed, mut configs) = (Vec::new(), Vec::new());
    for spec in config_specs {
        let cfg = read_spec(spec)?;
        configs.push((cfg.origin().to_string(), cfg.hash().to_string()));
        parsed.push(cfg);
    }
    let l = dimvexa_config::load(reg, &parsed, dimvexa::CONSTANTS).map_err(LoadError::Lib)?;
    Ok(Loaded { lib: l, configs })
}

/// Prints one `warning:` line per entry a later config replaced from an earlier one, on stderr
/// (replacing is allowed, never silent).
pub fn print_warnings(loaded: &Loaded, err: &mut dyn Write) {
    for w in &loaded.lib.warnings {
        let _ = writeln!(err, "warning: {w}");
    }
}

/// Prints a load failure for `schema` and returns the exit code.
pub fn report_load_error(
    e: &LoadError,
    json: bool,
    schema: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let (code, message, status) = match e {
        LoadError::Io(m) => ("usage", m.clone(), 2),
        LoadError::Lib(e) => (e.code(), e.to_string(), 1),
    };
    if json {
        let _ = writeln!(out, "{}", json_error(schema, code, &message, &[], &[]));
    } else {
        let _ = writeln!(err, "error[{code}]: {message}");
    }
    status
}
