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

`fastc explain prog.fc` emits one JSON document per function: name, params, return type, annotations, requires/ensures clauses. The MCP server (`fastc-mcp`) exposes the same information over Model Context Protocol so Claude Code / Cursor / Codex consume it without parsing text.

The alternative — text-scraping `cargo check` output — is what every Rust agent tool does today, and it breaks every time the diagnostic format changes. fastC is the protocol's first-class consumer; the compiler ships an MCP server alongside the binary.

### Provenance for dependencies pays off at install time

`fastc add` (planned) shows the requested capabilities before fetching. `fastc fetch` (already shipped) refuses to use a dep without a recorded `sha256`. `--vendor-strict` (planned) makes Sigstore mandatory for `fastc-core/*` packages. An LLM that suggests `use json_safe` (typosquat of `json`) doesn't get to silently install — the human sees the diff.

## What this doesn't claim

- **Token count.** fastC is the most verbose of the five languages we benchmarked (see [benchmarks](benchmarks.md)). The wedge is the *quality* of the tokens written, not the count.
- **First-compile success.** Pending benchmark data. If the number doesn't show fastC ahead, the wedge needs rethinking.
- **Smarter agents.** fastC doesn't make agents smarter. It makes the language more legible to agents that already exist, and forgiving of the specific failure modes agents make most often.
