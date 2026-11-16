# Dimvexa for AI assistants (MCP)

`dimvexa mcp` turns the `dimvexa` command into a [Model Context Protocol](https://modelcontextprotocol.io)
(MCP) server. An assistant such as Claude, Cursor or Codex then converts units through dimvexa
instead of from memory: exact factors with their sources, and a clear error, never a guess, when
a request is ambiguous (`ton`), mixes quantities, or needs a density you have not given.

It runs on your machine, reads from stdin and writes to stdout, and needs no network and no API
key. The decisions behind it are in [ADR-027](adr/027-mcp-server.md).

## 1. Install dimvexa

Any of these puts a `dimvexa` command on your `PATH`:

```console
$ cargo install dimvexa-cli          # builds from crates.io
$ brew install welworx/tap/dimvexa   # prebuilt (Homebrew)
```

or download the binary for your system from the
[releases page](https://github.com/welworx/dimvexa/releases). Check it:

```console
$ dimvexa --version
$ dimvexa mcp --help
```

`dimvexa mcp` is new: it is in the *Unreleased* section of the [changelog](../CHANGELOG.md), so
it is in the first release after 0.10.0. Until then build it from a clone of the repository with
`cargo install --path crates/dimvexa-cli`.

You never type into `dimvexa mcp` yourself. The assistant starts it when it needs it and stops it
when it is done, so there is nothing to keep running.

## 2. Add it to your assistant

Every client needs the same two facts: the **command** is `dimvexa` and the **argument** is
`mcp`. Use the full path of the binary (`which dimvexa`, on Windows `where dimvexa`) if the
client is a desktop app that does not see your shell's `PATH`.

### Claude Code

```console
$ claude mcp add dimvexa -- dimvexa mcp
```

That adds it for you in the current project. For every project add `--scope user`; to share it
with your team through the repository add `--scope project`, which writes `.mcp.json`:

```json
{
  "mcpServers": {
    "dimvexa": { "command": "dimvexa", "args": ["mcp"] }
  }
}
```

Check it with `claude mcp list`, or `/mcp` inside Claude Code.

### Claude Desktop

Open *Settings*, *Developer*, *Edit Config* (the file is `claude_desktop_config.json`: on macOS
`~/Library/Application Support/Claude/`, on Windows `%APPDATA%\Claude\`) and add:

```json
{
  "mcpServers": {
    "dimvexa": { "command": "/usr/local/bin/dimvexa", "args": ["mcp"] }
  }
}
```

Use the path `which dimvexa` prints (on Windows, for example
`C:\\Users\\you\\.cargo\\bin\\dimvexa.exe`, with doubled backslashes). Quit and start Claude
Desktop again; the tools appear under the tools (hammer) menu of a new chat.

### Cursor

Add the same block to `~/.cursor/mcp.json` (all projects) or `.cursor/mcp.json` (one project):

```json
{
  "mcpServers": {
    "dimvexa": { "command": "dimvexa", "args": ["mcp"] }
  }
}
```

### VS Code (GitHub Copilot)

In `.vscode/mcp.json`, or *MCP: Add Server* from the command palette (choose a command, stdio):

```json
{
  "servers": {
    "dimvexa": { "type": "stdio", "command": "dimvexa", "args": ["mcp"] }
  }
}
```

### Codex CLI

```console
$ codex mcp add dimvexa -- dimvexa mcp
```

or in `~/.codex/config.toml`:

```toml
[mcp_servers.dimvexa]
command = "dimvexa"
args = ["mcp"]
```

### Gemini CLI

In `~/.gemini/settings.json` (or `.gemini/settings.json` in a project):

```json
{
  "mcpServers": {
    "dimvexa": { "command": "dimvexa", "args": ["mcp"] }
  }
}
```

### Windsurf, Cline, Zed and others

Windsurf (`~/.codeium/windsurf/mcp_config.json`) and Cline use the `mcpServers` block above.
Any other client that can start a local ("stdio") MCP server takes the same command and
arguments; the settings file differs, the two facts do not.

### What does not work yet

A client that can only connect to a **remote** server by URL (the chat websites, the mobile
apps) cannot use it: `dimvexa mcp` is a local, stdio server. A remote transport is an open item
in [ADR-027](adr/027-mcp-server.md).

## 3. Use it

Ask in plain words; the assistant picks the tool:

* "How many km/h is 10 m/s?"
* "Convert 4 pi radians to degrees." (pi stays exact)
* "What is a ton in kilograms?" The answer is a refusal that lists `ton_short`, `ton_long` and
  `tonne`; the assistant asks which one you mean.
* "How much does 1.5 litres of water at 25 degrees C weigh?" Mass and volume need a density; the
  assistant uses the `water` config's named, sourced value and says so, or asks for yours.
* "Which unit is `psi`, and where does its factor come from?"

The tools, all read-only:

| Tool | Arguments | What it returns (the same JSON as the command with `--json`) |
|---|---|---|
| `convert` | `value`, `from`, `to`; `exact`, `configs`, `density` | the result, the exact fraction, the factor and its exactness, the sources (`dimvexa.convert/3`); or an error with a stable `code` and `candidates` |
| `units` | `query` (a group such as `volume` or a unit such as `psi`) or `groups` | the groups, a group's units, or one unit with factor, exactness, status and sources (`dimvexa.units/5`) |
| `constants` | `name`; `configs` | the named constants with exact value, unit, conditions and source (`dimvexa.constants/1`) |
| `configs` | `name` | the shipped configs, or what one contains (`dimvexa.configs/3`) |

`configs` lists names of **shipped** configs (`us-customary`, `imperial`, `water`, ...), which
resolve names such as `ton` or supply a density. `density` is for mass to volume: your own
number (`998.2067kg/m3`) or the name of a constant from a config. See the
[user guide](guide.md) for what each means; the JSON is documented in [`schemas/`](schemas).

A refusal is an *error result*, not a broken server: the assistant is told to relay it and not to
work around it. The refusals are the point.

## Your own units and densities

A config file of your own (see the [user guide](guide.md#names-and-densities---config)) can be
applied to every call. Name it when you start the server; the assistant cannot, so it cannot
make the server read other files:

```console
$ claude mcp add dimvexa -- dimvexa mcp --config /home/you/units/oil.toml --config water
```

```json
{ "command": "dimvexa", "args": ["mcp", "--config", "/home/you/units/oil.toml"] }
```

A config that cannot be loaded stops the server at the start with the reason on stderr, so a typo
shows up when the client starts it, not as a wrong answer later. Use absolute paths: the client
decides the working directory.

## Check it without an assistant

The server is a program that reads one JSON message per line. This sends the handshake and one
conversion and prints the replies:

```console
$ printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"convert","arguments":{"value":"10","from":"m/s","to":"km/h"}}}' \
  | dimvexa mcp
```

The second reply has `"isError":false` and, in its `text`, the JSON that
`dimvexa convert 10 m/s km/h --json` prints. The official
[MCP Inspector](https://github.com/modelcontextprotocol/inspector) (`npx @modelcontextprotocol/inspector dimvexa mcp`)
shows the tools and lets you call them.

## Safety

* The server only reads. No tool changes a file, runs a program or opens a network connection.
* The assistant's arguments are checked before any command runs: a word that looks like an
  option, an unknown argument, or a config that is not a shipped name is refused.
* Messages are limited to 1 MiB and the JSON reader limits nesting; a malformed message gets an
  error reply and the server keeps going.
* Nothing is sent anywhere by dimvexa. What the assistant does with a result is up to the
  assistant and your client's permissions.
* The results come from the same tables as the command line, with the same caveats: units and
  sources can contain errors, so verify what matters against the source
  ([licensing](licensing.md)), and do not rely on an assistant's summary alone in safety-critical,
  medical, legal, financial or custody-transfer settings.

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| The client says the server failed to start, or "command not found" | The client cannot see `dimvexa` on its `PATH` (common for desktop apps). Use the full path from `which dimvexa`. |
| It starts, then stops at once | A `--config` that cannot be loaded. Run the same command in a terminal: `dimvexa mcp --config PATH < /dev/null` prints the reason on stderr. |
| The tools do not appear | Restart the client after editing its config file; in Claude Code run `claude mcp list`. |
| `unrecognized subcommand`, or a conversion instead of a server | The installed `dimvexa` is older than the `mcp` command. `dimvexa mcp --help` must show the page. |
| The assistant answers from memory instead of using the tool | Ask for it ("use the dimvexa tool"), or add a line to your assistant's instructions: "convert units with the dimvexa tools". |
| The reply is an error such as `ambiguous_symbol` | Working as designed: pick one of the `candidates`, or apply a config that decides it. |
