# Capabilities

This document specifies fastC's capability system: typed I/O permissions passed as function arguments, checked at compile time, erased at runtime. Capabilities are the wedge feature for AI-generated code — the property that lets a compiler structurally reject a function that should not be able to read the filesystem, before any sandbox sees the binary.

Capabilities land in **stage 1.4** of the [roadmap](roadmap.md). They depend on stage 1.3 (annotation grammar) and complete the I/O-surface story stage 1.1 (stdlib) started.

## The design in one paragraph

Capabilities are first-class typed values: `cap.fs.read`, `cap.net.connect(host)`, `cap.proc.spawn`, etc. The capability lattice is finite, built-in, and named — users do not define new capabilities in v1. Tokens of each capability type are minted exclusively in `main()`, by calling into the runtime's `fc_cap_root` interface. From `main`, tokens are passed downward through call sites. A function that needs `fs.read` must accept an `fs.read` token argument. A function with no capability arguments cannot perform I/O — the compiler rejects any call to a capability-requiring callee. Capabilities erase to zero bytes at runtime in release mode: they are types, not values, post-codegen.

## Why capabilities, not effects

The discussion that led here considered three designs and rejected two:

1. **Algebraic effects + handlers** (Koka, Effekt). Powerful, but introduces hidden control flow via effect handlers. An effect handler can suspend a computation and resume it elsewhere, which violates fastC's "no hidden control flow" principle. Rejected.
2. **Effect annotations** (Java's `throws`, Rust-RFC-1404 style). Annotation-only — declares what a function does without making the privilege itself a value. Cannot express path-scoped restrictions (`fs.read("/etc/")` vs. `fs.read("/tmp/")`). Cannot express delegation (passing a narrowed token to a callee). Rejected for v1; could be revisited as v2 sugar.
3. **Capabilities as typed arguments** (Austral, occam-π, E). The capability is a value with a type. Holding the value is the permission. This composes well, allows path-scoping and delegation, and erases cleanly. **Chosen.**

The closest existing language is **Austral** (Borretti, 2022). Read its design notes before writing the implementation. The fastC delta against Austral is mainly surface syntax (we use `@caps(fs.read)` on the function declaration plus an implicit token argument, Austral makes the token argument explicit and named) and the integration with the rest of the annotation set (`@requires`, `@ensures`, `@mem`).

## The built-in capability lattice

The lattice is finite, named, and frozen in v1. Adding a capability requires a language version bump.

| Capability | Description | Path-scoped? | Notes |
|------------|-------------|:------------:|-------|
| `fs.read(<path>)` | Read access to a filesystem subtree | yes | Path is glob-matched at mint time |
| `fs.write(<path>)` | Write/create access to a filesystem subtree | yes | |
| `fs.list(<path>)` | Directory listing | yes | |
| `net.connect(<host>)` | Outbound TCP/UDP to a host pattern | yes | Host can be `*` for any |
| `net.listen(<port>)` | Bind a TCP/UDP port | yes | Port range supported |
| `net.dns` | DNS resolution | no | Separated from `connect` because DNS is a distinct exfil channel |
| `proc.spawn` | Fork/exec a process | no | The most dangerous capability; cannot be path-scoped |
| `proc.signal` | Send a signal to another process | no | |
| `time.read` | Read wall-clock or monotonic time | no | Side-channel concern in some threat models |
| `time.sleep` | Block on a timer | no | |
| `rand` | Read entropy from the OS RNG | no | |
| `env.read(<var>)` | Read environment variables | yes | Var name matched at mint |
| `env.write(<var>)` | Write environment variables | yes | |

The `@caps()` empty set denotes a pure function. The `@caps(*)` form is not allowed — there is no "all capabilities" wildcard, by design.

## Syntax

Capability sets appear in `@caps(...)` annotations on function declarations (see [docs/annotations.md](annotations.md)):

```fastc
@caps(fs.read("config/"))
pub fn load_config(cap: cap.fs.read, path: slice(u8)) -> res(Config, Error) {
    let bytes = fs::read_file(cap, path)?;
    return parse(bytes);
}
```

Two things happen here:

1. The `@caps(...)` annotation declares the capability set the function requires.
2. The first parameter is a capability token — a value of type `cap.fs.read`. The compiler checks that every call to `fs::read_file` (which itself declares `@caps(fs.read)`) passes a `cap.fs.read` token.

In practice, **the token argument is implicit when the `@caps` declaration is present**: the compiler synthesizes a hidden parameter and threads it through call sites. The developer (or agent) writes the annotation, not the parameter. This keeps the syntax compact while preserving the type-level guarantee.

For `pub` functions on a module boundary, the compiler may require explicit token parameters to make the API surface clear in `fastc context` output. This is a style call, not a correctness call.

## Capability minting in main

The root capability is obtained from the runtime:

```fastc
pub fn main() -> i32 {
    let root = fc_cap_root();          // returns a cap.root token
    let fs_caps = cap_fs(root);        // mints a cap.fs token (any path)
    let net_caps = cap_net(root);      // mints a cap.net token

    let cfg_cap = cap_fs_read(fs_caps, "config/");   // narrow to a subtree
    let api_cap = cap_net_connect(net_caps, "api.example.com");

    let cfg = load_config(cfg_cap, "config/app.toml")?;
    serve(api_cap, cfg);
    return 0;
}
```

`fc_cap_root` is the only function in the runtime that returns a fresh `cap.root` token, and the compiler refuses to call it from anything other than `main`. From the root, narrower tokens are minted by the runtime's `cap_*` constructors, each of which takes a wider token and a path/host/port restriction and returns a narrower token.

This is the explicit-flow design from Austral. The agent reading `main` sees the entire I/O surface of the program at a glance.

## Checking rules

The capability check is a flow analysis on the typed AST, run after type-checking and before lowering. The rules are simple:

1. **Token possession.** A function that calls a capability-requiring callee must hold a token of that capability (passed as an argument or freshly minted). Holding a token is the only way to satisfy the call.
2. **Subsumption.** A function declaring `@caps(C1, C2)` may call a callee declaring `@caps(C2)` — the callee's set must be a subset of the caller's. Equivalently, the caller must hold tokens for at least the callee's capabilities.
3. **Path scoping.** `cap.fs.read("config/")` is a *subtype* of `cap.fs.read("/")`. A function requiring read access to anywhere will accept a token narrowed to a subtree. Going the other direction is a type error.
4. **No ambient access.** There is no global `fs::read()` that does not take a capability. The stdlib (stage 1.1) is born with capability-typed signatures so this rule has no exception in user code.
5. **`unsafe` opens a hole.** Code in an `unsafe` block can bypass capability checking by directly calling C functions via FFI. The hole is explicit; auditing capability-aware code reduces to auditing `unsafe` blocks (which fastC already requires for raw pointer access).

## Runtime cost

Zero in release mode. Capability tokens are zero-sized types (ZSTs) by default, monomorphized away at codegen. The capability check happens at compile time and erases.

A scoped/narrowed token (like `cap.fs.read("config/")`) is *also* zero-sized — the path restriction is encoded in the type, not the value. This is what makes the system free at runtime: there is no string to compare, because the compiler already verified the call.

In debug mode (`fastc build --dev`), the runtime additionally checks that path/host/port restrictions are honoured by the actual syscall — a backstop against `unsafe`-block escapes. This adds one comparison per I/O syscall and is opt-out via `--no-cap-runtime-check`.

## Build artifact: caps.json

Every fastC build emits `caps.json` — the full capability graph of the program. Format:

```json
{
  "version": "1.0",
  "root_caps": ["fs", "net", "proc", "time", "rand", "env"],
  "main_mints": [
    {"cap": "fs.read", "scope": "config/", "minted_at": "src/main.fc:5"},
    {"cap": "net.connect", "scope": "api.example.com", "minted_at": "src/main.fc:6"}
  ],
  "function_caps": [
    {
      "function": "config::loader::load_config",
      "requires": ["fs.read(config/)"],
      "calls_requiring": ["fs::read_file"],
      "location": "src/config/loader.fc:14"
    }
  ],
  "call_graph_summary": {
    "total_functions": 412,
    "pure_functions": 287,
    "io_functions": 125
  }
}
```

`caps.json` is consumed by:

- `fastc cert-report` — the I/O isolation evidence for safety-critical certification (stage 2.2).
- `fastc-mcp` (stage 1.6) — served to coding agents as an MCP resource.
- `fastc add` (stage 1.7) — when adding a dependency, the user is shown the dep's `caps.json` "requires" set before installation.

## Implementation sketch

The compiler pass that lands in stage 1.4:

1. **Parse capability annotations.** Already handled in stage 1.3 (annotation grammar).
2. **Build the capability requirement table.** One entry per function: the union of `@caps` declarations plus any capabilities transitively required by callees.
3. **Flow-analyze token arguments.** For each function body, walk the AST and assign each call site a token-set requirement. Match against the caller's token bag (parameters + locally minted tokens via `cap_*` constructors).
4. **Subsumption check.** At each call boundary, verify the callee's capability set is a subset of the caller's. Path-scoped capabilities use subtype checking on the path component.
5. **Emit `caps.json`.** Walk the resolved AST and emit the structured artifact.

The pass is linear in the size of the call graph. Caching at the function level (per the 0.8 Salsa skeleton) means re-checking is incremental.

## What capabilities do not cover

Capabilities are about *what a function can ask the operating system to do*. They do not cover:

- **Memory safety.** That is the type system's job (`ref`, `mref`, `own`, `slice`, `arr`, `opt`, `res`) plus the runtime checks (bounds, null, overflow).
- **Termination.** That is `@panics` and `@complexity`. A capability-pure function can still loop forever.
- **Correctness.** That is `@requires` / `@ensures`.
- **Concurrency safety.** That is `@threading` on the module header. Capabilities are orthogonal — a `thread-safe` module still respects capability boundaries.
- **Side-channel leakage.** A function that holds only `time.read` and writes timing information into shared memory is technically only using its declared capabilities. Side-channel hardening is a separate concern; capabilities make it *easier* to audit, not automatic.

## Comparison to Austral

Austral is the closest existing system. The deltas:

| Property | Austral | fastC |
|----------|---------|-------|
| Token argument | Explicit, named in function signature | Implicit, declared via `@caps`, synthesized as hidden parameter |
| Capability lattice | User-extensible via linear types | Built-in, finite, named (v1) |
| Path scoping | Not a built-in concept | Built-in: `fs.read("config/")` |
| Runtime cost | Zero (linear types erase) | Zero (capability tokens are ZSTs) |
| Stdlib integration | Stdlib reorganized around capabilities | Stdlib born capability-aware in 1.1, checking enforced in 1.4 |
| Agent surface | Standard Austral docs | `caps.json` + `fastc-mcp` + `fastc explain` |

Austral is the proof of concept that this design works. fastC is the engineering of it for the agent era.

## Open questions for the 1.4 design pass

- **Capability narrowing UX.** The runtime `cap_fs_read(parent, "config/")` constructor pattern is clean but verbose. Should fastC ship a sugar form (`cap_narrow!(fs.read, "config/")`)? Decision: revisit after 1.4 prototype, based on real usage in stdlib code.
- **Cross-package capabilities.** A dependency declares `@caps(net.connect)`. When `fastc add` displays this to the user, should the host pattern be visible? Decision: yes, always — this is the killer feature for capability-aware dep review (stage 1.7).
- **Capability inference for private functions.** Private functions infer their `@caps` from their body. Stage 1.3 says yes. Edge case: a private function called from a `pub` function inherits the public function's stricter declaration as a constraint. Confirmed correct; document the propagation rule in the 1.4 implementation.
- **Threading + capabilities.** A `thread-safe` module's functions need to be callable from any thread, which means they cannot capture a token in module-level state. Decision: tokens are always passed by argument, never held in module state. Module headers cannot declare `@owns: cap.fs.read` — capabilities are not resources in the `@owns` sense.
