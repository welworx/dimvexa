// SPDX-License-Identifier: MIT OR Apache-2.0
//! A [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server for dimvexa, so an
//! AI assistant can convert units with exact factors instead of guessing them.
//!
//! This crate has **no dependencies and no conversion logic**. It speaks JSON-RPC over a
//! pair of streams (the MCP `stdio` transport: one JSON message per line) and offers four
//! read-only tools: `convert`, `units`, `constants` and `configs`. Each tool is one `dimvexa`
//! command line with `--json`; the caller of [`serve`] supplies the function that runs it
//! (the `dimvexa` binary passes its own command line runner, see `dimvexa mcp`), so the answer
//! an assistant gets is exactly the one `dimvexa` prints, with the same stable error codes.
//! The reasons are in `docs/adr/027-mcp-server.md`; how to connect it to Claude and other
//! clients is in `docs/mcp.md`.
//!
//! A refusal is a result, not a failure of the server: `isError` is set and the text is the
//! JSON error (`ok: false`, a stable `code`, `candidates`). The assistant is told to report it
//! and not to guess a number.
//!
//! ```
//! use dimvexa_mcp::{Options, Output, serve};
//!
//! let input = br#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"convert","arguments":{"value":"1","from":"km","to":"m"}}}
//! "#;
//! let mut output = Vec::new();
//! // A stand-in for the real runner: it receives the `dimvexa` arguments of the tool call.
//! let mut run = |args: &[String]| Output {
//!     code: 0,
//!     stdout: format!("{{\"args\":{}}}\n", args.len()),
//!     stderr: String::new(),
//! };
//! serve(&mut &input[..], &mut output, &Options::default(), &mut run).unwrap();
//! let reply = String::from_utf8(output).unwrap();
//! assert!(reply.contains(r#""isError":false"#), "{reply}");
//! ```
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-256.png",
    html_favicon_url = "https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-32.png"
)]

mod json;

use std::fmt::Write as _;
use std::io::{self, BufRead, Read, Write};

use json::{Json, quote};

/// The protocol versions this server can speak, newest first. The tools use only features that
/// are the same in all of them.
const PROTOCOL_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// The longest message accepted, in bytes. A longer line is refused and skipped.
const MAX_MESSAGE_BYTES: u64 = 1 << 20;

/// The longest text accepted for one tool argument, in characters.
const MAX_ARGUMENT_CHARS: usize = 512;

/// What the assistant is told when it connects.
const INSTRUCTIONS: &str = "\
Unit conversion with exact factors from cited sources. Use these tools instead of doing unit \
arithmetic yourself. A refusal (an error result with a code such as ambiguous_symbol, \
dimension_mismatch or unknown_unit) is the answer, not a fault to work around: tell the user \
what it says and offer its candidates, and never guess a number. Mass <-> volume needs a density \
from the user (the density argument) or a named one from a config. Start with the units tool \
(groups) to find a unit's name.";

/// The tool list, as the `tools/list` result. A test reads it back as JSON.
const TOOLS: &str = r#"{"tools":[
{"name":"convert",
"title":"Convert a value between units",
"description":"Convert a value from one unit to another using exact rational factors, not floating-point approximations. Returns JSON (schema dimvexa.convert/3): the result (output.value, output.f64), the exact fraction (exact.value) and factor, whether the factor is defined or measured (exactness), and the sources. Ambiguous symbols (for example 'ton'), units of different quantities and unknown units are errors with a stable code and candidates: report the error, do not guess. Temperatures are written degC, degF, K. Units may be products and quotients (m/s, kW.h). Mass <-> volume is refused unless you give a density.",
"inputSchema":{"type":"object","properties":{
"value":{"type":"string","description":"The number to convert, for example '10', '-40', '1.5e3'. May use pi exactly: '4pi', 'pi/2'."},
"from":{"type":"string","description":"The unit of the value, for example 'm/s', 'psi', 'degF'. Use the units tool to look a unit up."},
"to":{"type":"string","description":"The unit to convert to."},
"exact":{"type":"boolean","description":"Also return the exact result as a fraction with its decimal (the default result already carries exact.value)."},
"configs":{"type":"array","items":{"type":"string"},"description":"Names of shipped configs to apply, such as one that resolves 'ton' or supplies the density of water. List them with the configs tool."},
"density":{"type":"string","description":"For mass <-> volume only: your own density with its unit, '998.2067kg/m3', or the name of a constant from a config, 'water_25C' (with configs ['water']). The assumption is reported."}},
"required":["value","from","to"],"additionalProperties":false},
"annotations":{"title":"Convert a value between units","readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}},
{"name":"units",
"title":"List and look up units",
"description":"Find the name of a unit or show one. With groups=true: the quantities (length, mass, pressure, ...) with the number of units in each, which is small. With query set to a group name ('volume'): the units of that group. With query set to a unit ('psi', 'km'): its id, symbol, name, exact factor, exactness, status (current or legacy) and sources. With neither: every unit (large, about 60 KB; prefer groups first). Returns JSON (schema dimvexa.units/5). An unknown name is an error with suggestions.",
"inputSchema":{"type":"object","properties":{
"query":{"type":"string","description":"A group such as 'volume', or a unit such as 'psi' or 'km'."},
"groups":{"type":"boolean","description":"Only list the groups and their sizes. Cannot be combined with query."},
"configs":{"type":"array","items":{"type":"string"},"description":"Names of shipped configs whose units and names are included."}},
"additionalProperties":false},
"annotations":{"title":"List and look up units","readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}},
{"name":"constants",
"title":"List and look up constants",
"description":"List the named constants (pi, h_P, e, k_B, N_A, and the densities of a config) with exact value, unit, conditions and source, or show one by name. Returns JSON (schema dimvexa.constants/1).",
"inputSchema":{"type":"object","properties":{
"name":{"type":"string","description":"A constant name such as 'k_B'. Omit to list all."},
"configs":{"type":"array","items":{"type":"string"},"description":"Names of shipped configs whose constants are included, for example ['water']."}},
"additionalProperties":false},
"annotations":{"title":"List and look up constants","readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}},
{"name":"configs",
"title":"List and show configs",
"description":"A config adds names, units and constants to the strict defaults, for example it decides what 'ton' means or supplies the density of water. Without name: the configs that ship with dimvexa. With name: what one contains, with conditions and sources. Returns JSON (schema dimvexa.configs/3). Apply a config by listing its name in the configs argument of the other tools.",
"inputSchema":{"type":"object","properties":{
"name":{"type":"string","description":"A shipped config name. Omit to list them."}},
"additionalProperties":false},
"annotations":{"title":"List and show configs","readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}}
]}"#;

/// What running one `dimvexa` command line produced.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    /// The exit code: `0` is success, anything else is a refusal or an error.
    pub code: u8,
    /// What the command wrote to stdout. With `--json` this is the JSON result or error.
    pub stdout: String,
    /// What the command wrote to stderr.
    pub stderr: String,
}

/// Settings of the server, chosen by whoever starts it (not by the assistant).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Configs applied to every call: a shipped name or a path to your own file, as for
    /// `dimvexa --config`. Only the person who starts the server can name a file; the
    /// assistant can only name shipped configs.
    pub configs: Vec<String>,
}

/// Serves MCP on `input` and `output` until `input` ends, one JSON message per line, running
/// each tool call through `run` (the arguments of a `dimvexa` command line, without the
/// program name).
///
/// # Errors
///
/// Only a failure to read or write the streams. A malformed message gets a JSON-RPC error
/// reply and the server goes on.
pub fn serve(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    options: &Options,
    run: &mut dyn FnMut(&[String]) -> Output,
) -> io::Result<()> {
    loop {
        let mut line = Vec::new();
        let read = Read::take(&mut *input, MAX_MESSAGE_BYTES.saturating_add(1))
            .read_until(b'\n', &mut line)?;
        if read == 0 {
            return Ok(());
        }
        let too_long = u64::try_from(line.len()).map_or(true, |n| n > MAX_MESSAGE_BYTES);
        let reply = if too_long {
            skip_rest_of_line(input, &line)?;
            Some(error_reply(
                &Json::Null,
                -32600,
                "message too large (limit 1 MiB)",
            ))
        } else {
            match String::from_utf8(line) {
                Ok(text) if text.trim().is_empty() => None,
                Ok(text) => handle(&text, options, run),
                Err(_) => Some(error_reply(&Json::Null, -32700, "message is not UTF-8")),
            }
        };
        if let Some(reply) = reply {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
}

/// After an over-long line was cut at the limit, reads on to its end.
fn skip_rest_of_line(input: &mut dyn BufRead, cut: &[u8]) -> io::Result<()> {
    let mut ended = cut.last() == Some(&b'\n');
    while !ended {
        let mut chunk = Vec::new();
        let read = Read::take(&mut *input, MAX_MESSAGE_BYTES).read_until(b'\n', &mut chunk)?;
        ended = read == 0 || chunk.last() == Some(&b'\n');
    }
    Ok(())
}

/// A JSON-RPC error reply.
fn error_reply(id: &Json, code: i32, message: &str) -> String {
    let mut out = String::from(r#"{"jsonrpc":"2.0","id":"#);
    id.write(&mut out);
    let _ = write!(out, r#","error":{{"code":{code},"message":"#);
    quote(message, &mut out);
    out.push_str("}}");
    out
}

/// A JSON-RPC success reply; `result` is JSON text.
fn result_reply(id: &Json, result: &str) -> String {
    let mut out = String::from(r#"{"jsonrpc":"2.0","id":"#);
    id.write(&mut out);
    out.push_str(r#","result":"#);
    out.push_str(result);
    out.push('}');
    out
}

/// Answers one message: `None` for a notification or a response (nothing to say).
fn handle(
    text: &str,
    options: &Options,
    run: &mut dyn FnMut(&[String]) -> Output,
) -> Option<String> {
    let message = match Json::parse(text) {
        Ok(m) => m,
        Err(e) => {
            return Some(error_reply(
                &Json::Null,
                -32700,
                &format!("Parse error: {e}"),
            ));
        }
    };
    if !matches!(message, Json::Object(_)) {
        return Some(error_reply(
            &Json::Null,
            -32600,
            "Invalid Request: expected one JSON object per line (batches are not supported)",
        ));
    }
    let Some(method) = message.get("method").and_then(Json::as_str) else {
        // A response to a request of ours (this server sends none) is ignored.
        let is_response = message.get("result").is_some() || message.get("error").is_some();
        return (!is_response)
            .then(|| error_reply(&Json::Null, -32600, "Invalid Request: no method"));
    };
    let id = match message.get("id") {
        None => return None, // a notification never gets a reply
        Some(id @ (Json::String(_) | Json::Number(_))) => id,
        Some(_) => {
            return Some(error_reply(
                &Json::Null,
                -32600,
                "Invalid Request: id must be a string or a number",
            ));
        }
    };
    let empty = Json::Object(Vec::new());
    let params = message.get("params").unwrap_or(&empty);
    Some(match method {
        "initialize" => result_reply(id, &initialize_result(params)),
        "ping" => result_reply(id, "{}"),
        // The list is written over several lines to be readable here; a message is one line.
        "tools/list" => result_reply(id, &TOOLS.replace('\n', "")),
        "tools/call" => call_tool(id, params, options, run),
        other => error_reply(id, -32601, &format!("Method not found: {other}")),
    })
}

/// The `initialize` result: the protocol version both sides speak, and what this server offers.
fn initialize_result(params: &Json) -> String {
    let wanted = params.get("protocolVersion").and_then(Json::as_str);
    let version = PROTOCOL_VERSIONS
        .iter()
        .copied()
        .find(|v| Some(*v) == wanted)
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    let mut out = String::from(r#"{"protocolVersion":"#);
    quote(version, &mut out);
    out.push_str(r#","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"dimvexa","title":"Dimvexa","version":"#);
    quote(env!("CARGO_PKG_VERSION"), &mut out);
    out.push_str(r#"},"instructions":"#);
    quote(INSTRUCTIONS, &mut out);
    out.push('}');
    out
}

/// The result of a tool call: its text, and whether it is an error the assistant should report.
fn tool_result(text: &str, is_error: bool) -> String {
    let mut out = String::from(r#"{"content":[{"type":"text","text":"#);
    quote(text, &mut out);
    out.push_str(r#"}],"isError":"#);
    out.push_str(if is_error { "true" } else { "false" });
    out.push('}');
    out
}

fn call_tool(
    id: &Json,
    params: &Json,
    options: &Options,
    run: &mut dyn FnMut(&[String]) -> Output,
) -> String {
    let Some(name) = params.get("name").and_then(Json::as_str) else {
        return error_reply(id, -32602, "Invalid params: 'name' (the tool) is required");
    };
    let empty = Json::Object(Vec::new());
    let arguments = match params.get("arguments") {
        None | Some(Json::Null) => &empty,
        Some(a @ Json::Object(_)) => a,
        Some(_) => {
            return error_reply(id, -32602, "Invalid params: 'arguments' must be an object");
        }
    };
    if !matches!(name, "convert" | "units" | "constants" | "configs") {
        return error_reply(id, -32602, &format!("Unknown tool: {name}"));
    }
    // A bad argument is a tool error the assistant can read and correct, not a protocol error.
    let command = match command_line(name, arguments, options) {
        Ok(c) => c,
        Err(message) => {
            return result_reply(
                id,
                &tool_result(&format!("invalid arguments: {message}"), true),
            );
        }
    };
    let out = run(&command);
    let stdout = out.stdout.trim();
    let text = if stdout.is_empty() {
        out.stderr.trim()
    } else {
        stdout
    };
    let text = if text.is_empty() {
        "the command printed nothing"
    } else {
        text
    };
    result_reply(id, &tool_result(text, out.code != 0))
}

/// The `dimvexa` command line for a tool call, or what is wrong with the arguments.
///
/// Every word that reaches the command line is checked first: none may look like an option
/// (`--config /etc/...`), so the assistant cannot change what the command does, only what it
/// is asked about. Configs it names must be shipped names, never files.
fn command_line(tool: &str, args: &Json, options: &Options) -> Result<Vec<String>, String> {
    let allowed: &[&str] = match tool {
        "convert" => &["value", "from", "to", "exact", "configs", "density"],
        "units" => &["query", "groups", "configs"],
        "constants" => &["name", "configs"],
        _ => &["name"],
    };
    if let Json::Object(members) = args {
        if let Some((key, _)) = members.iter().find(|(k, _)| !allowed.contains(&k.as_str())) {
            return Err(format!(
                "unknown argument '{key}'; this tool takes {}",
                allowed.join(", ")
            ));
        }
    }
    let mut line = vec![tool.to_string()];
    match tool {
        "convert" => {
            line.push(text_arg(args, "value", true, true)?.unwrap_or_default());
            line.push(text_arg(args, "from", true, false)?.unwrap_or_default());
            line.push(text_arg(args, "to", true, false)?.unwrap_or_default());
            if flag_arg(args, "exact")? {
                line.push("--exact".to_string());
            }
        }
        "units" => {
            if flag_arg(args, "groups")? {
                line.push("--groups".to_string());
            }
            line.extend(text_arg(args, "query", false, false)?);
        }
        _ => line.extend(text_arg(args, "name", false, false)?),
    }
    line.push("--json".to_string());
    if tool != "configs" {
        for config in &options.configs {
            line.push("--config".to_string());
            line.push(config.clone());
        }
        for config in shipped_names(args)? {
            line.push("--config".to_string());
            line.push(config);
        }
    }
    if tool == "convert" {
        if let Some(density) = text_arg(args, "density", false, false)? {
            line.push("--const".to_string());
            line.push(density);
        }
    }
    Ok(line)
}

/// A text argument; a JSON number is accepted as its text. `negative` allows a leading `-`
/// (a negative value), never `--`.
fn text_arg(
    args: &Json,
    key: &str,
    required: bool,
    negative: bool,
) -> Result<Option<String>, String> {
    let text = match args.get(key) {
        None | Some(Json::Null) if required => return Err(format!("'{key}' is required")),
        None | Some(Json::Null) => return Ok(None),
        Some(Json::String(s)) => s.clone(),
        Some(Json::Number(n)) => n.clone(),
        Some(_) => return Err(format!("'{key}' must be a string")),
    };
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(format!("'{key}' must not be empty"));
    }
    if text.chars().count() > MAX_ARGUMENT_CHARS {
        return Err(format!(
            "'{key}' is longer than {MAX_ARGUMENT_CHARS} characters"
        ));
    }
    let option_like = if negative {
        text.starts_with("--")
    } else {
        text.starts_with('-')
    };
    if option_like {
        return Err(format!("'{key}' must not start with '-'"));
    }
    Ok(Some(text))
}

fn flag_arg(args: &Json, key: &str) -> Result<bool, String> {
    match args.get(key) {
        None | Some(Json::Null) => Ok(false),
        Some(Json::Bool(b)) => Ok(*b),
        Some(_) => Err(format!("'{key}' must be true or false")),
    }
}

/// The `configs` argument: shipped config names only (letters, digits, `-` and `_`), so the
/// assistant can never make the server read a file.
fn shipped_names(args: &Json) -> Result<Vec<String>, String> {
    let items = match args.get("configs") {
        None | Some(Json::Null) => return Ok(Vec::new()),
        Some(Json::Array(items)) => items,
        Some(_) => return Err("'configs' must be a list of config names".to_string()),
    };
    let mut names = Vec::new();
    for item in items {
        let name = item
            .as_str()
            .ok_or_else(|| "'configs' must be a list of config names".to_string())?;
        let plain = !name.is_empty()
            && name.chars().count() <= 64
            && !name.starts_with('-')
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
        if !plain {
            return Err(format!(
                "'{name}' is not a shipped config name; list them with the configs tool"
            ));
        }
        names.push(name.to_string());
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `messages` through the server; the runner records the command lines.
    fn session(messages: &[&str], options: &Options) -> (Vec<Json>, Vec<Vec<String>>) {
        let input = messages.join("\n");
        let mut output = Vec::new();
        let mut calls = Vec::new();
        let mut run = |args: &[String]| {
            calls.push(args.to_vec());
            if args.iter().any(|a| a == "bad") {
                Output {
                    code: 1,
                    stdout: "{\"ok\":false}\n".to_string(),
                    stderr: String::new(),
                }
            } else {
                Output {
                    code: 0,
                    stdout: "{\"ok\":true}\n".to_string(),
                    stderr: "warning on stderr\n".to_string(),
                }
            }
        };
        serve(&mut input.as_bytes(), &mut output, options, &mut run).unwrap();
        let replies = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|l| Json::parse(l).unwrap())
            .collect();
        (replies, calls)
    }

    fn call(arguments: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"convert","arguments":{arguments}}}}}"#
        )
    }

    fn text_of(reply: &Json) -> String {
        reply
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| match c {
                Json::Array(a) => a.first(),
                _ => None,
            })
            .and_then(|c| c.get("text"))
            .and_then(Json::as_str)
            .unwrap()
            .to_string()
    }

    fn is_error(reply: &Json) -> bool {
        reply.get("result").and_then(|r| r.get("isError")) == Some(&Json::Bool(true))
    }

    fn code_of(reply: &Json) -> String {
        match reply.get("error").and_then(|e| e.get("code")) {
            Some(Json::Number(n)) => n.clone(),
            other => panic!("no error code: {other:?}"),
        }
    }

    #[test]
    fn the_tool_list_is_valid_json_with_four_read_only_tools() {
        // No string in the list contains a raw newline, so removing them cannot change it.
        assert!(!TOOLS.contains("\\n"));
        let list = Json::parse(TOOLS).unwrap();
        let Some(Json::Array(tools)) = list.get("tools") else {
            panic!("no tools");
        };
        let names: Vec<&str> = tools
            .iter()
            .map(|t| t.get("name").and_then(Json::as_str).unwrap())
            .collect();
        assert_eq!(names, ["convert", "units", "constants", "configs"]);
        for t in tools {
            let hints = t.get("annotations").unwrap();
            assert_eq!(hints.get("readOnlyHint"), Some(&Json::Bool(true)));
            assert_eq!(hints.get("openWorldHint"), Some(&Json::Bool(false)));
            let schema = t.get("inputSchema").unwrap();
            assert_eq!(schema.get("type").and_then(Json::as_str), Some("object"));
            assert!(t.get("description").and_then(Json::as_str).is_some());
        }
    }

    #[test]
    fn handshake_lists_tools_and_ignores_notifications() {
        let (replies, calls) = session(
            &[
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                r#"{"jsonrpc":"2.0","id":"two","method":"ping"}"#,
                r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#,
            ],
            &Options::default(),
        );
        assert_eq!(replies.len(), 3, "a notification gets no reply");
        let init = replies[0].get("result").unwrap();
        assert_eq!(
            init.get("protocolVersion").and_then(Json::as_str),
            Some("2025-03-26")
        );
        assert_eq!(
            init.get("serverInfo")
                .and_then(|s| s.get("name"))
                .and_then(Json::as_str),
            Some("dimvexa")
        );
        assert!(
            init.get("capabilities")
                .and_then(|c| c.get("tools"))
                .is_some()
        );
        assert_eq!(replies[1].get("id"), Some(&Json::String("two".into())));
        assert!(
            replies[2]
                .get("result")
                .and_then(|r| r.get("tools"))
                .is_some()
        );
        assert_eq!(calls, Vec::<Vec<String>>::new());
    }

    #[test]
    fn an_unknown_protocol_version_gets_the_newest_one() {
        let (replies, _) = session(
            &[
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1999-01-01"}}"#,
            ],
            &Options::default(),
        );
        let version = replies[0]
            .get("result")
            .and_then(|r| r.get("protocolVersion"))
            .and_then(Json::as_str);
        assert_eq!(version, Some(PROTOCOL_VERSIONS[0]));
    }

    #[test]
    fn convert_becomes_one_command_line() {
        let (replies, calls) = session(
            &[&call(
                r#"{"value":"-40","from":"degC","to":"degF","exact":true,"configs":["water"],"density":"998kg/m3"}"#,
            )],
            &Options {
                configs: vec!["./mine.toml".to_string()],
            },
        );
        assert_eq!(
            calls,
            [[
                "convert",
                "-40",
                "degC",
                "degF",
                "--exact",
                "--json",
                "--config",
                "./mine.toml",
                "--config",
                "water",
                "--const",
                "998kg/m3"
            ]]
        );
        assert!(!is_error(&replies[0]));
        assert_eq!(text_of(&replies[0]), "{\"ok\":true}");
        assert_eq!(replies[0].get("id"), Some(&Json::Number("7".into())));
    }

    #[test]
    fn a_number_value_is_accepted_and_a_refusal_is_an_error_result() {
        let (replies, calls) = session(
            &[&call(r#"{"value":1.5,"from":"bad","to":"m"}"#)],
            &Options::default(),
        );
        assert_eq!(calls[0][1], "1.5");
        assert!(is_error(&replies[0]));
        assert_eq!(text_of(&replies[0]), "{\"ok\":false}");
    }

    #[test]
    fn other_tools_map_to_their_commands() {
        let m = |name: &str, args: &str| {
            format!(
                r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{name}","arguments":{args}}}}}"#
            )
        };
        let (_, calls) = session(
            &[
                &m("units", r#"{"groups":true}"#),
                &m("units", r#"{"query":"psi","configs":["us-customary"]}"#),
                &m("units", "{}"),
                &m("constants", r#"{"name":"k_B"}"#),
                &m("constants", r#"{"configs":["water"]}"#),
                &m("configs", r#"{"name":"water"}"#),
                &m("configs", "{}"),
            ],
            &Options::default(),
        );
        let expected: [&[&str]; 7] = [
            &["units", "--groups", "--json"],
            &["units", "psi", "--json", "--config", "us-customary"],
            &["units", "--json"],
            &["constants", "k_B", "--json"],
            &["constants", "--json", "--config", "water"],
            &["configs", "water", "--json"],
            &["configs", "--json"],
        ];
        assert_eq!(calls, expected);
    }

    #[test]
    fn startup_configs_do_not_reach_the_configs_command() {
        let options = Options {
            configs: vec!["water".to_string()],
        };
        let (_, calls) = session(
            &[r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"configs"}}"#],
            &options,
        );
        assert_eq!(calls, [["configs", "--json"]]);
    }

    #[test]
    fn the_assistant_cannot_add_options_or_read_files() {
        let hostile = [
            r#"{"value":"1","from":"--config","to":"m"}"#,
            r#"{"value":"--json","from":"m","to":"km"}"#,
            r#"{"value":"1","from":"-m","to":"km"}"#,
            r#"{"value":"1","from":"m","to":"km","density":"--exact"}"#,
            r#"{"value":"1","from":"m","to":"km","configs":["/etc/passwd"]}"#,
            r#"{"value":"1","from":"m","to":"km","configs":["./x.toml"]}"#,
            r#"{"value":"1","from":"m","to":"km","configs":["a.toml"]}"#,
            r#"{"value":"1","from":"m","to":"km","configs":["--json"]}"#,
            r#"{"value":"1","from":"m","to":"km","configs":[""]}"#,
            r#"{"value":"1","from":"m","to":"km","configs":"water"}"#,
            r#"{"value":"1","from":"m","to":"km","configs":[1]}"#,
            r#"{"value":"1","from":"m","to":"km","--config":"x"}"#,
            r#"{"value":"1","from":"m","to":"km","extra":1}"#,
            r#"{"value":"1","from":"m","to":"km","exact":"yes"}"#,
            r#"{"value":"1","from":["m"],"to":"km"}"#,
            r#"{"value":"","from":"m","to":"km"}"#,
            r#"{"from":"m","to":"km"}"#,
        ];
        let messages: Vec<String> = hostile.iter().map(|a| call(a)).collect();
        let refs: Vec<&str> = messages.iter().map(String::as_str).collect();
        let (replies, calls) = session(&refs, &Options::default());
        assert!(calls.is_empty(), "ran {calls:?}");
        for (reply, args) in replies.iter().zip(hostile) {
            assert!(is_error(reply), "{args}");
            assert!(text_of(reply).starts_with("invalid arguments: "), "{args}");
        }
        let long = call(&format!(
            r#"{{"value":"1","from":"{}","to":"m"}}"#,
            "m".repeat(600)
        ));
        let (replies, calls) = session(&[&long], &Options::default());
        assert!(calls.is_empty() && is_error(&replies[0]));
    }

    #[test]
    fn protocol_errors() {
        let (replies, calls) = session(
            &[
                "{not json",
                "[1,2]",
                r#"{"jsonrpc":"2.0","id":1}"#,
                r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#,
                r#"{"jsonrpc":"2.0","id":2,"method":"resources/list"}"#,
                r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{}}"#,
                r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"rm"}}"#,
                r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"units","arguments":[]}}"#,
                r#"{"jsonrpc":"2.0","id":6,"result":{}}"#,
                "",
                "   ",
            ],
            &Options::default(),
        );
        let codes: Vec<String> = replies.iter().map(code_of).collect();
        assert_eq!(
            codes,
            [
                "-32700", "-32600", "-32600", "-32600", "-32601", "-32602", "-32602", "-32602"
            ]
        );
        assert_eq!(calls, Vec::<Vec<String>>::new());
    }

    #[test]
    fn a_long_line_is_refused_and_the_server_goes_on() {
        let big = "x".repeat(usize::try_from(MAX_MESSAGE_BYTES).unwrap() + 10);
        let input = format!("{big}\n{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}}\n");
        let mut output = Vec::new();
        let mut run = |_: &[String]| Output::default();
        serve(
            &mut input.as_bytes(),
            &mut output,
            &Options::default(),
            &mut run,
        )
        .unwrap();
        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "{text}");
        assert!(lines[0].contains("-32600"));
        assert!(lines[1].contains(r#""id":1"#) && lines[1].contains("result"));
    }

    #[test]
    fn invalid_utf8_is_a_parse_error() {
        let input: &[u8] = b"\xff\xfe\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n";
        let mut output = Vec::new();
        let mut run = |_: &[String]| Output::default();
        serve(&mut &input[..], &mut output, &Options::default(), &mut run).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(
            text.starts_with(r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700"#),
            "{text}"
        );
    }

    #[test]
    fn a_command_that_printed_only_to_stderr_is_reported() {
        let mut run = |_: &[String]| Output {
            code: 2,
            stdout: String::new(),
            stderr: "error[usage]: no\n".to_string(),
        };
        let input = call(r#"{"value":"1","from":"m","to":"km"}"#);
        let mut output = Vec::new();
        serve(
            &mut input.as_bytes(),
            &mut output,
            &Options::default(),
            &mut run,
        )
        .unwrap();
        let reply = Json::parse(String::from_utf8(output).unwrap().trim()).unwrap();
        assert!(is_error(&reply));
        assert_eq!(text_of(&reply), "error[usage]: no");
    }
}
