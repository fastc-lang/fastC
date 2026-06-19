# Why fastC?

fastC is **v1.0 feature-complete** as of 2026-Q2. It exists because four problems aren't being solved by the languages we already have:

1. **Supply-chain attacks.** Rust's `build.rs`, Zig's `build.zig`, and npm postinstall all run arbitrary code at install time. fastC refuses to run anything during dependency resolution — manifests are declarative only. Every dep ships with a content-addressed `sha256`, every release ships with a cosign keyless signature, and `dep_content_hash` is part of the build cache key so dep churn invalidates the cache by construction.
2. **Ambient I/O authority.** Every other systems language lets any function call `open()`, `connect()`, or `system()`. fastC makes capabilities (`fs.read`, `net.connect`, …) typed function arguments. A function with no capability arguments structurally cannot do I/O.
3. **Contracts as comments.** `@requires` and `@ensures` in fastC are compile-time obligations. The three-tier discharge pipeline — tier-1 syntactic (always on), tier-2 SMT via Z3 (opt-in via `--prove`), tier-3 runtime trap (safe fallback) — proves what it can and traps on the rest.
4. **Compile times nobody measures.** fastC has a CI-enforced compile-time budget via `fastc bench`. tcc backend for dev builds, gcc/clang for release. Cross-language perf targets are committed to the repo and checked on every PR.

On top of the four wedges, v1.0 ships a curated **11-package `fastc-core` ecosystem** (cli, log, json, toml, http, time, base64, uuid, crypto-primitives, regex, sqlite) under [fastc-lang](https://github.com/fastc-lang), a v1.3 annotation surface (`@purity` / `@panics` / `@complexity` / `@mem` plus module-level mandatory headers), and an agent-tooling layer (`fastc fix`, `fastc context`, `fastc diff`, `fastc explain`, `fastc mcp`) the other four languages don't have.

This section walks through each piece in detail.

## In this section

- [**Rubric**](rubric.md) — side-by-side comparison with C, Rust, Zig, Go.
- [**Benchmarks**](benchmarks.md) — measured compile time, binary size, runtime, token count, first-compile success rate, and the umbrella harness.
- [**Agent-friendly by design**](agent-friendly.md) — what fastC's constraints buy you when an AI agent writes the code, plus the unified diagnostic envelope and MCP server.
- [**C interop**](c-interop.md) — fastC emits C; what does that mean for using existing C libraries.
- [**Safety defaults**](safety-defaults.md) — what's checked by default, what `--safety-level=critical` adds, and how module-level mandatory headers add a structural safety layer above per-function annotations.

## If you're evaluating fastC for a project

Read the [rubric](rubric.md) first — that's where the wedge is most visible. Then look at the [benchmarks](benchmarks.md) page for the measured numbers behind each claim. Then decide whether the trade-offs (longer source than Rust, no recursion in critical mode, no central registry) match your team's constraints.

fastC is small, opinionated, and not for everyone. The honest framing is: if your code lives inside an organization that is already enforcing capability discipline, contract obligations, and supply-chain integrity by policy, then fastC just hardcodes those policies into the type system so your reviewers stop having to enforce them by hand. If you don't need those properties, plain C or Rust is probably the right tool.
