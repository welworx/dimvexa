# ADR-027: an MCP server in its own crate, `dimvexa mcp`, over the CLI's JSON

Status: Proposed (the owner has asked for MCP support and has not yet confirmed the decisions
below; the OPEN items wait for that)

Keeps ADR-003 (no third-party dependencies, std only) and ADR-001 (no guessing); adds a crate and
a command, so it is recorded here.

## Context

An AI assistant that converts units in its head or in generated code gets the factor wrong or
guesses an ambiguous name (`ton`, `oz`, `gal`) without saying so. That is the failure dimvexa
exists to prevent: exact factors, loud failures, zero guessing. The Model Context Protocol (MCP)
is how assistants (Claude Code, Claude Desktop, Cursor, VS Code, Codex, Gemini CLI and others)
call a local tool. A user should download `dimvexa`, add one line to the assistant, and get exact
conversions, with the same refusals and error codes as the command line.

Constraints from this repository: no third-party dependencies and an offline build (so no JSON or
MCP crate), library code without `unwrap`, indexing or unchecked arithmetic, MSRV 1.85, and the
rule that the CLI has no conversion logic of its own and the JSON it prints is a versioned,
documented schema (`docs/schemas/`).

What the protocol needs is small. The `stdio` transport is one JSON-RPC 2.0 message per line.
A server answers `initialize`, `ping`, `tools/list` and `tools/call`, and ignores notifications.

## Decision

1. **A separate crate, `dimvexa-mcp`.** It holds the JSON reader and writer, the JSON-RPC
   handling, the tool definitions and the argument checks, and nothing about units. It has no
   dependencies, not even on `dimvexa`. The core, the data and the config crates stay free of a
   wire protocol.
2. **The tools are the CLI's commands.** `convert`, `units`, `constants` and `configs` each map
   to one `dimvexa ... --json` command line. The crate takes a function that runs a command line
   and returns its exit code, stdout and stderr; `dimvexa-cli` passes its own `run`. So the result
   an assistant reads is the documented JSON (`dimvexa.convert/3`, `dimvexa.units/5`,
   `dimvexa.constants/1`, `dimvexa.configs/3`), byte for byte what `dimvexa` prints, and a change
   of a schema reaches the assistant in the same commit. A refusal is a tool result with
   `isError: true` and the JSON error with its stable `code` and `candidates`, not a protocol
   error. The server's `instructions` and the tool descriptions tell the assistant to report a
   refusal and not to guess.
3. **`dimvexa mcp [--config NAME_OR_PATH]...` starts it.** The binary a user already installs is
   the server, nothing else to download. The direction of the dependency is `dimvexa-cli` on
   `dimvexa-mcp`, which is why the crate takes the runner as a function instead of depending on
   the CLI. A `--config` of the command applies to every call, and a config that cannot be loaded
   stops the server at the start (exit 1), not at the first call.
4. **Read-only, local, no network.** Every tool is annotated read-only and closed-world. The
   server reads stdin and writes stdout, and loads a file only for a `--config` the person who
   starts it names. The assistant can name configs, but only shipped ones (letters, digits, `-`
   and `_`); a path, an option-like word or an unknown argument is a tool error before anything
   runs, so an assistant cannot make the server read a file or add an option to the command.
5. **Transport and protocol.** `stdio` only. The server negotiates a version from 2025-11-25,
   2025-06-18, 2025-03-26 and 2024-11-05 (the tool features it uses are the same in all four) and
   answers the newest it knows when the client asks for another. Messages are limited to 1 MiB, a
   longer line is refused and skipped, and a JSON-RPC batch is refused (the 2025-06-18 revision
   dropped batches). The JSON reader is strict (RFC 8259) and limits nesting to 64.
6. **Not in this change.** OPEN, recommended default in brackets:
   * OPEN: a remote (streamable HTTP) transport, which web chat clients need [not now: it needs a
     network listener, authentication and a TLS story, and is a different security decision];
   * OPEN: `batch_convert`, `explain_refusal`, `list_packs` and `list_contexts` from the ROADMAP
     list [not now: the refusal already carries its explanation and candidates; add `batch_convert`
     when a client needs it, as another command's JSON];
   * OPEN: MCP resources (the unit tables as documents) and prompts [not now].

## Consequences

* A user installs `dimvexa` once and runs `claude mcp add dimvexa -- dimvexa mcp` (or the
  equivalent in another client); `docs/mcp.md` has each client.
* The tool results cannot disagree with the command line, and every new CLI schema field is
  visible to assistants without a protocol change. A removed or renamed field is the breaking
  change it already is (`api-change` skill).
* Each tool call loads the unit tables again, because it is a command run. A call takes
  milliseconds; if a client ever needs thousands of calls a second, a `batch_convert` tool is the
  answer, not a cache.
* `dimvexa-mcp` is published, before `dimvexa-cli`, with the others (`docs/releasing.md`).
* A new crate to keep to the rules: the workspace lints apply, the JSON reader has hostile-input
  tests, and the hand-written reader is the one piece to review when the protocol changes.
* Not offered: anything that changes state, anything that needs a network. An MCP client that
  can only talk to a remote server cannot use this server yet.
