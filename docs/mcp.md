# fastc-mcp: Native MCP Server for Coding Agents

This document specifies `fastc-mcp` — the Model Context Protocol server that exposes the fastC compiler's machine-readable artifacts to coding agents (Claude Code, Cursor, Codex, and anything else MCP-speaking) as native protocol resources rather than text-parsed compiler output.

`fastc-mcp` lands in **stage 1.6** of the [roadmap](roadmap.md). It depends on stage 1.3 (annotation grammar), 1.4 (capabilities), and 1.5 (contracts) for the artifacts it serves. The 1.6 work is mostly wiring: the artifacts already exist as compiler outputs; MCP exposes them.

## The argument

Today, an agent working in Rust runs `cargo check`, captures the stderr text, parses it for line/column/error-code triples, and constructs a fix. The text-parsing layer is a recurring source of agent failures — the message format changes between rustc versions, error codes are not always present, span information is hard to recover precisely.

The right fix is not a better text format. The right fix is to serve the compiler's structured information over a typed protocol. MCP is that protocol, MCP is widely adopted (Claude Code, Cursor, ChatGPT desktop, Codex, plus open-source clients), and fastC's compiler already produces the artifacts MCP would serve. `fastc-mcp` is the thin server that exposes them.

The agent's experience becomes:

1. Connect to `fastc-mcp` over MCP.
2. Query `manifest.json` for the full annotation surface of any function: name, parameters, capabilities, contracts, complexity. Reason from the signature without reading the body.
3. Query `caps.json` for the program's capability graph: which functions need network, which need fs.read, what `main` mints.
4. Query `discharge.json` for which contracts are proven, which are runtime-checked, which were deferred.
5. Run `fastc check` over MCP — get structured diagnostic responses, not text.
6. Apply `fastc fix` over MCP — receive deterministic fix-it patches.

This is qualitatively different from "the agent runs `fastc check` in a subprocess and parses the output."

## The protocol surface

`fastc-mcp` implements the standard MCP server interface (https://modelcontextprotocol.io). Resources and tools exposed:

### Resources

Resources are queryable, cacheable, identified by URI. Each fastC project served by `fastc-mcp` exposes:

| URI | Contents | Backed by |
|-----|----------|-----------|
| `fastc://project/manifest` | Every function's complete annotation surface | `manifest.json` build artifact |
| `fastc://project/caps` | The program's capability graph | `caps.json` build artifact |
| `fastc://project/discharge` | Contract discharge results | `discharge.json` build artifact |
| `fastc://project/diagnostics` | Current compile errors and warnings | `fastc check --output-format=json` |
| `fastc://project/symbols` | Project-wide symbol index | `fastc context --output-format=json` |
| `fastc://function/<path>` | Single function: annotations + AST + callers | derived from manifest |
| `fastc://module/<path>` | Single module: header annotations + symbols | derived from manifest |

All resources are JSON. Schemas are published at `fastc://schema/<resource>` and align with the existing `ComplianceReport` / `ViolationDetail` / `SourceLocation` structures so cert-report and MCP share types.

### Tools

Tools are callable, side-effecting. The minimum set for the agent inner loop:

| Tool | Purpose | Arguments |
|------|---------|-----------|
| `fastc.check` | Run `fastc check` over the current state of source | `{"target": "src/main.fc"}` |
| `fastc.fix` | Apply deterministic fix-it hints to a diagnostic | `{"diagnostic_id": "..."}` |
| `fastc.explain` | Return the full annotation surface of a symbol | `{"symbol": "config::loader::load"}` |
| `fastc.format` | Run `fastc fmt` over a file | `{"file": "src/foo.fc"}` |
| `fastc.annotate` | Run `fastc fmt --annotate` to fill inferred annotations | `{"file": "src/foo.fc"}` |
| `fastc.diff` | Run `fastc diff` (semantic AST diff) | `{"from": "...", "to": "..."}` |
| `fastc.search` | Search symbols by name across the project | `{"query": "load_config"}` |
| `fastc.caps_required_for` | Show capability set required to call a function | `{"symbol": "..."}` |

Tools return JSON. Errors include a structured error code, message, and (where applicable) location.

### Subscriptions

`fastc-mcp` supports MCP subscriptions for resources that change:

- `fastc://project/diagnostics` — notifies on file change events.
- `fastc://project/manifest` — notifies when annotation surface changes.

Agents can subscribe to diagnostics and get pushed updates instead of polling.

## How the artifacts are produced

Three artifacts feed the MCP server. They are emitted by `fastc build` (or `fastc check`) into the build cache:

- **`manifest.json`** — emitted by the annotation pass (stage 1.3). Every function's annotations + signature + location. About 1KB per function in practice.
- **`caps.json`** — emitted by the capability pass (stage 1.4). The capability graph + main's mint points. Tens of KB for a typical project.
- **`discharge.json`** — emitted by the contract pass (stage 1.5, expanded in 2.1). One entry per contract obligation. Hundreds of KB for a large project.

The artifacts are versioned (`"version": "1.0"`). `fastc-mcp` reads them from `.fastc/cache/artifacts/` and serves them. When the build runs (triggered by the agent's `fastc.check` call, or by file watching), the artifacts update, and subscribers are notified.

Implementation note: the artifacts already align with the JSON shape used by `cert-report` (the `ComplianceReport`, `ViolationDetail`, `SourceLocation` types). The MCP server is mostly a transport layer; no new data model.

## Server lifecycle

`fastc-mcp` runs as a standalone process per project. The agent (Claude Code, etc.) launches it on demand:

```
# Launched by the MCP client
$ fastc-mcp --project /path/to/project

[fastc-mcp] Listening on stdio
[fastc-mcp] Project: /path/to/project
[fastc-mcp] Resources: 7
[fastc-mcp] Tools: 8
```

The server speaks MCP over stdio (the standard transport for local servers). It watches the project's source files; on change, it triggers an incremental `fastc check` (using the Salsa cache from stage 0.8) and updates the artifact cache. Subscribers are notified.

Memory footprint: dominated by the Salsa cache, ~10MB per 10k lines of fastC.

CPU footprint: idle most of the time. On a save event, runs incremental check (< 200ms per stage 0.8 budget), then idles.

## The agent UX

What this looks like from Claude Code or Cursor's perspective:

```
[User asks Claude Code: "implement a config loader that reads from ~/.app/config.toml"]

Claude Code:
  1. Calls fastc-mcp.search("config") — returns existing config-related symbols.
  2. Calls fastc-mcp.explain("config::schema::Config") — gets the type signature without reading file.
  3. Calls fastc-mcp.caps_required_for("toml::parse") — returns "()" (pure parser).
  4. Generates a draft function: pub fn load_config(cap: cap.fs.read, path: slice(u8)) -> res(Config, Error) {...}.
  5. Writes the file.
  6. Calls fastc-mcp.check() — receives a structured diagnostic: "config::loader::load: missing @caps annotation on pub function. Inferred: fs.read".
  7. Calls fastc-mcp.annotate(file) — annotations are written back.
  8. Calls fastc-mcp.check() — clean.
  9. Returns to the user.
```

No text parsing. No subprocess output management. No "what does this error message mean?" Every step is a typed MCP call.

The token efficiency win is real and measurable: agents in this loop produce shorter prompts (they query specific functions, not file dumps) and shorter outputs (they generate code that compiles first try because they know what the compiler expects). Stage 1.2's token-efficiency benchmark will measure exactly this.

## Comparison to alternatives

| Approach | Pros | Cons |
|----------|------|------|
| Text-parse `cargo check` / `fastc check` | Works today, no infrastructure | Format drift, lossy spans, hard to recover error code |
| `rust-analyzer` over LSP | Rich, IDE-integrated | LSP is designed for human IDEs, not agent workflows; resources are coarse-grained |
| Custom HTTP API per language | Maximum flexibility | Every language reinvents the protocol |
| **MCP server** | Typed, standard, designed for agents, growing client support | New protocol; needs server implementation per language |

MCP is the right bet because (a) the agent ecosystem has converged on it in 2025–2026, (b) it is designed for exactly this use case (typed resources + tools served to an LLM-driven client), and (c) the fastC artifacts already fit cleanly.

## The implementation

`crates/fastc-mcp/` — a new crate in the workspace.

Dependencies:

- The existing `fastc` library crate (for the compiler API and artifact emission).
- An MCP server library (we will use the official Rust SDK if available at the time of implementation; otherwise an in-house implementation following the published protocol spec).
- Tokio for async I/O on stdio.

Code structure (rough estimate):

- `crates/fastc-mcp/src/main.rs` — entry point.
- `crates/fastc-mcp/src/server.rs` — MCP server loop, request routing.
- `crates/fastc-mcp/src/resources.rs` — resource handlers (manifest, caps, discharge, etc.).
- `crates/fastc-mcp/src/tools.rs` — tool handlers (check, fix, explain, etc.).
- `crates/fastc-mcp/src/watcher.rs` — filesystem watcher, incremental check triggering.
- `crates/fastc-mcp/src/subscriptions.rs` — change notification routing.

About 1500–2500 lines of Rust. The implementation is small because the compiler does the heavy lifting; the server is glue.

## Integration with `fastc-core` packages

Every `fastc-core` package (stage 1.8) ships its `manifest.json` and `caps.json` alongside the source. When `fastc-mcp` serves a project that depends on `fastc-http`, the MCP resources include the dep's manifest and caps — so an agent can query "what does `fastc_http::Server::new` require?" and get a typed answer without reading the dep's source.

This is the multiplier effect: every package in the ecosystem extends the agent's knowable surface area through MCP, with no per-package integration work.

## Auth and security

`fastc-mcp` runs locally as a per-project process; there is no remote attack surface in v1. Specifically:

- No network listener. stdio only.
- No write access outside the project directory.
- No execution of arbitrary code on behalf of the MCP client. Tools like `fastc.fix` apply deterministic fix-it hints; they never run user-supplied code.

A future v2 could expose `fastc-mcp` over HTTPS for remote agent workflows (e.g., a cloud IDE), but v1 is local-only.

## Open questions

- **Streaming for large resources.** `discharge.json` for a large project can be hundreds of KB. MCP supports streaming responses; we should use it for the larger resources. Specify the streaming chunking in the implementation.
- **Versioning the MCP surface.** As fastC evolves, the resource shapes will too. Version every resource URI (`fastc://project/manifest/v1`) so older agents do not break. Lockstep with the artifact JSON schema versioning.
- **Multi-project workspaces.** Some projects are monorepos with multiple fastC projects. Should one `fastc-mcp` instance serve all of them, or one per project? Current lean: one per project (simpler isolation), with a workspace-level coordinator if needed later.
- **Discovery.** How does an MCP client know which fastC binary to use for `fastc-mcp`? Convention: `fastc-mcp` is on PATH alongside `fastc`. Optionally, `.claude/mcp-config.json` (or similar per client) declares the server. Document in `AGENTS.md` scaffolded by `fastc new`.
- **Sandboxing.** Should `fastc-mcp` enforce that the agent client's tool calls cannot escape the project root? Current answer: yes by default. The compiler itself does not write outside the project; the MCP layer rejects any tool call that would.
