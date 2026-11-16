<p align="center"><img src="https://raw.githubusercontent.com/welworx/dimvexa/main/docs/brand/icon-256.png" alt="Dimvexa" width="96" height="96"></p>

# dimvexa-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) server for [dimvexa](../../README.md):
an AI assistant converts units with exact factors and cited sources, and gets a loud, coded error
instead of a guess when the request is ambiguous.

You do not use this crate directly. Install the `dimvexa` command and run `dimvexa mcp`; the
setup for Claude Code, Claude Desktop, Cursor, VS Code, Codex and others is in
[docs/mcp.md](../../docs/mcp.md).

The crate speaks JSON-RPC over the MCP stdio transport and has four read-only tools (`convert`,
`units`, `constants`, `configs`). It has no dependencies and no conversion logic: each tool is one
`dimvexa ... --json` command line, run by the function the caller passes to `serve`, so an
assistant gets exactly what the command line prints. See
[ADR-027](../../docs/adr/027-mcp-server.md).

Licensed under MIT OR Apache-2.0.
