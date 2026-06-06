# MCP Server

The `mcp` command runs fastC as an MCP (Model Context Protocol) stdio
server. It exposes fastC's tooling surface — explain, check, compile,
context, diff, capability summary — to Claude Code, Cursor, and any
other MCP-speaking client over stdin/stdout JSON-RPC.

The server is fully self-contained: no third-party MCP SDK, no extra
runtime dependency, just `fastc mcp` reading line-delimited JSON-RPC 2.0
requests from stdin and writing responses to stdout.

## Usage

```bash
fastc mcp
```

No flags. The server runs until stdin closes.

## Protocol

- **Transport**: stdin / stdout.
- **Framing**: line-delimited JSON. One JSON-RPC 2.0 message per line.
  No `Content-Length` headers — modern MCP clients (Claude Code, Cursor)
  accept both line-delimited and LSP-style framings; line-delimited is
  simpler and keeps the fastC binary small.
- **Encoding**: UTF-8.
- **Concurrency**: one request at a time, in arrival order.

## Implemented Methods

| Method        | Description |
|---------------|-------------|
| `initialize`  | Return server capabilities and `serverInfo` |
| `tools/list`  | Enumerate the exposed tools and their input schemas |
| `tools/call`  | Dispatch a call to a named tool |

Unknown methods return JSON-RPC error `-32601` (method not found).

### `initialize` response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-03-26",
    "capabilities": { "tools": {} },
    "serverInfo": { "name": "fastc-mcp", "version": "<package version>" }
  }
}
```

## Exposed Tools

All tools take a `path` argument (or two for `diff`) and return their
payload inside an MCP `content` array as a `text` blob.

| Tool           | Arguments          | Description |
|----------------|--------------------|-------------|
| `explain`      | `path`             | JSON of every fn surface — same shape as `fastc explain` |
| `check`        | `path`             | `{ok: bool, diagnostics: [...]}` — type-check result |
| `compile`      | `path`             | Generated C output, or an error envelope |
| `caps_summary` | `path`             | Capability graph — which fns accept which `Cap*` tokens |
| `context`      | `path`, `module?`  | Markdown surface dump — signatures only, optimized for AI context windows |
| `diff`         | `old`, `new`       | Semantic diff — `{added, removed, changed}` |

The schemas served by `tools/list` describe each tool's `inputSchema`
inline; clients should consume `tools/list` at startup and cache the
result.

## Sample JSON-RPC Exchange

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "explain",
    "arguments": { "path": "src/main.fc" }
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"functions\": [...],\n  \"modules\": [...]\n}"
      }
    ]
  }
}
```

The `text` payload is the full JSON `fastc explain` would print to
stdout. Clients parse it as a second-stage JSON to get at the function
surface.

## Editor Integration

### Claude Code

Add an MCP server entry to `~/.claude/mcp.json` (or the project-local
`.mcp.json`):

```json
{
  "mcpServers": {
    "fastc": {
      "command": "fastc",
      "args": ["mcp"]
    }
  }
}
```

Once registered, Claude Code's `/mcp` will list the fastC tools and
expose them as tool calls during a session.

### Cursor

Cursor reads MCP servers from `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "fastc": {
      "command": "fastc",
      "args": ["mcp"]
    }
  }
}
```

Restart Cursor after editing the config. The fastC tools appear in the
agent's tool list automatically.

### Other clients

Any MCP client that supports stdio transport will work. Point it at the
`fastc` binary on `PATH` with the `mcp` argument. No extra flags, no
configuration files.

## Worked Example

Listing the tool surface from the shell:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | fastc mcp
```

Output (formatted for readability):

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "tools": [
      { "name": "explain",      "description": "Return a JSON summary of every fn in a fastC source file...", "inputSchema": { ... } },
      { "name": "check",        "description": "Type-check a fastC source file and return any diagnostics...", "inputSchema": { ... } },
      { "name": "context",      "description": "Return a markdown surface of every pub item in the project...", "inputSchema": { ... } },
      { "name": "diff",         "description": "Compute a semantic AST-level diff between two fastC sources...", "inputSchema": { ... } },
      { "name": "caps_summary", "description": "Return the capability graph (caps.json) for a fastC source...", "inputSchema": { ... } }
    ]
  }
}
```

Calling `explain` on a file:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"explain","arguments":{"path":"examples/cli_demo.fc"}}}' \
  | fastc mcp
```

returns the same JSON `fastc explain examples/cli_demo.fc` would print,
wrapped in the MCP `content` envelope shown above.

## Error Envelope

Per JSON-RPC 2.0, errors come back under the `error` key:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": { "code": -32602, "message": "Missing required 'path' argument" }
}
```

| Code     | Meaning |
|----------|---------|
| `-32700` | Parse error — body was not valid JSON |
| `-32601` | Method not found |
| `-32602` | Invalid params — required argument missing |
| `-32000` | Server error — file IO failed or parser rejected the source |

## Stability

The wire protocol matches MCP version `2025-03-26`. The tool list is
**additive** — new tools may appear in future releases, existing tools
keep their argument shape. Agent integrations should treat unknown tools
as ignorable and use `tools/list` as the source of truth.

## When to Use It

- **Claude Code / Cursor sessions**: lets the agent invoke fastC tooling
  without shelling out to `bash` and re-parsing stdout.
- **Custom agent stacks**: wire your own LLM loop into a fastC project
  via the JSON-RPC server with no extra glue.
- **CI assistants**: a long-running `fastc mcp` process can answer
  successive `check` / `diff` / `explain` calls without paying parse
  startup per request.

## See Also

- [Explain](explain.md) — backs the `explain` tool
- [Context](context.md) — backs the `context` tool
- [Diff](diff.md) — backs the `diff` tool
- [fastc-core](../language/fastc-core.md) — the language surface the MCP tools describe
