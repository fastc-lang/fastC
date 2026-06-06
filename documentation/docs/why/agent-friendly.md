# Agent-friendly by design

The argument that fastC is agent-friendly rests on what the compiler can *prove* about agent-written code before it runs. The longer source (see [benchmarks](benchmarks.md)) pays for itself if it materially improves what a reviewing human (or another agent) sees.

## What an agent needs from a language

| Need | Why | fastC's answer |
|---|---|---|
| Unambiguous grammar | An agent must not guess what `int *x[10]` parses as | Explicit `let name: type` everywhere; no declarator ambiguity |
| Explicit conversions | Implicit truncation is the #1 source of "compiles but wrong" | All conversions require `cast(T, x)`; no implicit promotion |
| No undefined behavior in safe code | The agent can't reason about a behavior the spec doesn't define | UB only inside `unsafe` blocks, statically scoped |
| Mechanically-readable artifacts | The agent shouldn't parse `cargo check` output as text | `fastc explain` and `fastc-mcp` serve AST + types + caps as JSON / MCP |
| Side-effect visibility | The agent must know what a function can reach | Capability-typed I/O: `fn read_file(c: ref(CapFsRead), ...)` |
| Cheap iteration | The agent compiles 10× per task; each cycle must be fast | Compile-time budget enforced in CI; tcc dev backend; Salsa incremental |
| Provenance for dependencies | The agent might suggest pulling a typosquat | Vendor-first manifest with sha256 + Sigstore |

## What each constraint pays off, concretely

### Unambiguous grammar pays off at parse time

C's "most vexing parse" doesn't exist in fastC. Every declaration starts with `let` or `fn` or `struct` or `enum`. An agent writing fastC can't accidentally write something that parses as a function pointer when it meant a function call.

### Explicit conversions pay off at type-check time

`cast(i32, large_i64)` is many tokens but the compiler can flag the truncation site. In C an integer can silently narrow at any assignment, function call, or comparison; in fastC the truncation is named at every site. When an agent makes a mistake here, the diagnostic points at the exact `cast(...)` and says what type it expected.

### Capability-typed I/O pays off at review time

A reviewing human (or agent) reading a fastC `pub fn` immediately knows what I/O the function can touch from the signature alone. No archaeology, no global grep for `system(` / `open(` / `socket(`. If the signature is `fn process(s: ref(Str)) -> i32`, the function structurally cannot reach the filesystem, the network, the clock, or the env. Compare to Rust: any `pub fn` can call `std::fs::read` or `std::process::Command::new(...).output()` and the type system has no idea.

This is the wedge for code where the writer is an LLM and the reviewer's bandwidth is limited.

### Mechanically-readable artifacts pay off at agent-loop time

`fastc explain prog.fc` emits one JSON document covering every function (name, params, return type, annotations, requires/ensures clauses, `purity`, `panics`, `complexity`, `caps`, `is_test`) plus a top-level `modules` array carrying any `//! @module / @owns / @arch / @depends` headers. See [CLI: explain](../cli/explain.md).

The same surface is exposed via [`fastc mcp`](../cli/mcp.md) — a stdio JSON-RPC 2.0 server that any MCP-speaking client (Claude Code, Cursor, Codex) can consume. The tools it advertises:

- `explain` — same JSON as the CLI subcommand
- `check` — type-check + structured diagnostics
- `context` — the project's pub type surface (see [`fastc context`](../cli/context.md))
- `diff` — semantic AST-level diff between two snapshots (see [`fastc diff`](../cli/diff.md))
- `caps_summary` — the per-build capability graph

The alternative — text-scraping `cargo check` output — is what every Rust agent tool does today, and it breaks every time the diagnostic format changes. fastC is the protocol's first-class consumer; the compiler ships an MCP server alongside the binary.

### The unified diagnostic envelope

Every fastC diagnostic — compile errors, P10 violations, capability violations, contract violations, discharge failures — funnels through one JSON shape:

```json
{
  "kind": "compile_error",
  "rule_id": "E_PARSE",
  "severity": "error",
  "span": { "file": "foo.fc", "start": 10, "end": 12 },
  "message": "expected ;",
  "hint": "add a semicolon"
}
```

Available via `fastc compile --output-format=json` and `fastc check --output-format=json`. Editors, CI gates, and agent loops parse one envelope instead of five.

### LSP capabilities

`fastc-lsp` advertises `code_action_provider` (quick-fixes from the `Fixit` infrastructure), `semantic_tokens_provider` (fastC-specific highlighting that knows about `@purity` / `@panics` / cap params), and `rename_provider` (workspace rename through the resolver's symbol table). Wire-up details in [Editor setup](../getting-started/editor-setup.md).

### Agent-applicable fix-its

`fastc fix <file> [--dry-run]` walks structured `Fixit` records carried by diagnostics and applies the mechanical ones (wrap-in-unsafe, missing semicolon, missing `use`, chained-binop parens). See [`fastc fix`](../cli/fix.md). The same Fixit set surfaces via LSP code actions in the editor.

### Provenance for dependencies pays off at install time

`fastc add` shows the requested capabilities before fetching and records the sha256 in `fastc.lock`. Every subsequent build verifies the cached vendor copy against the recorded sha. The build cache key (the `dep_content_hash` field added in v1.0.x) is derived from concatenating every dep's sha — a dep change always invalidates the cache. See [`fastc add`](../cli/add.md) and [`fastc lock`](../cli/lock.md). An LLM that suggests `use json_safe` (typosquat of `json`) doesn't get to silently install — the human sees the diff.

## What this claims, with measurement

- **First-compile success.** Initially measured at 0/9 on T1 sum_array against four Ollama Cloud open-weight models, with an inaccurate cheatsheet shipped to the LLM. After rewriting the cheatsheet around a verified worked example and a "common mistakes" inverse guide — same task, same prompts, same N=3 trials — the result moved to **12/12**. fastC is competitive with C / Rust / Zig / Go on first-compile rate **conditional on faithful prompting documentation**. The lesson: every strict-syntax language pays for its strictness at LLM-write-time and recovers the cost via documentation. fastC's documentation now ships at the cheatsheet level, and `fastc fix` has shipped as of v1.0.x — the parser-integrated mechanical-fix loop is real.
- **Safety wedge against silently-wrong runtime behavior.** On T5 (sum 1..100000 with no overflow warning), GLM produced silently-wrapped output 3/3 trials in Go and 2/3 in Rust. fastC and Zig refused to compile or computed correctly — neither shipped a silently-wrong binary. See [benchmarks](benchmarks.md#safety-wedge-compile-vs-correct-gap).

## What this doesn't claim

- **Token count.** fastC is the most verbose of the five languages we benchmarked (see [benchmarks](benchmarks.md)). The trade is more typing to get the type system to enforce more invariants; the first-compile data above says the extra typing isn't free at LLM-write-time, and the safety-wedge data says it pays off at runtime.
- **Smarter agents.** fastC doesn't make agents smarter. It makes the language more legible to agents that already exist, and forgiving of the specific failure modes agents make most often.
- **Frontier-model performance.** All benchmarks above are open-weight Ollama Cloud models. Claude, GPT-4o, and Gemini 2.5 Pro likely do better on first-compile against capable cheatsheets and worse on the safety wedge (more sophisticated default-correct behavior). Running the harness with all three sets of keys would close this remaining gap.
