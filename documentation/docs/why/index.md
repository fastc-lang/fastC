# Why fastC?

fastC exists because four problems aren't being solved by the languages we already have:

1. **Supply-chain attacks.** Rust's `build.rs`, Zig's `build.zig`, and npm postinstall all run arbitrary code at install time. fastC refuses to run anything during dependency resolution — manifests are declarative only.
2. **Ambient I/O authority.** Every other systems language lets any function call `open()`, `connect()`, or `system()`. fastC makes capabilities (`fs.read`, `net.connect`, …) typed function arguments. A function with no capability arguments structurally cannot do I/O.
3. **Contracts as comments.** `@requires` and `@ensures` in fastC are compile-time obligations checked by the compiler — runtime-asserted in v1, SMT-discharged in v2.
4. **Compile times nobody measures.** fastC has a CI-enforced compile-time budget. tcc backend for dev builds, gcc/clang for release. Targets are committed to the repo and checked on every PR.

This section walks through each piece in detail.

## In this section

- [**Rubric**](rubric.md) — side-by-side comparison with C, Rust, Zig, Go.
- [**Benchmarks**](benchmarks.md) — measured compile time, binary size, runtime, token count, and first-compile success rate.
- [**Agent-friendly by design**](agent-friendly.md) — what fastC's constraints buy you when an AI agent writes the code.
- [**C interop**](c-interop.md) — fastC emits C; what does that mean for using existing C libraries.
- [**Safety defaults**](safety-defaults.md) — what's checked by default and what `--safety-level=critical` adds.

## If you're evaluating fastC for a project

Read the [rubric](rubric.md) first — that's where the wedge is most visible. Then look at the [benchmarks](benchmarks.md) page for the measured numbers behind each claim. Then decide whether the trade-offs (longer source than Rust, no recursion in critical mode, no central registry) match your team's constraints.

fastC is small, opinionated, and not for everyone. The honest framing is: if your code lives inside an organization that is already enforcing capability discipline, contract obligations, and supply-chain integrity by policy, then fastC just hardcodes those policies into the type system so your reviewers stop having to enforce them by hand. If you don't need those properties, plain C or Rust is probably the right tool.
