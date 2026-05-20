# Annotations

This document specifies the fastC annotation grammar — the typed metadata that turns every public function signature into a complete operating manual the compiler enforces. Annotations are not comments. They are first-class syntactic elements parsed as part of function and module declarations, and they generate compile-time obligations.

Annotations land in three roadmap stages:

- **Stage 1.3** — the lint-checked subset: `@mem`, `@panics`, `@purity`, `@complexity`, and the mandatory module-header annotations (`@module`, `@owns`, `@arch`, `@depends`, `@threading`, `@invariants`). Inferred for private functions; mandatory and checked on `pub`.
- **Stage 1.4** — `@caps` becomes a flow-checked typed argument set (see [docs/capabilities.md](capabilities.md)).
- **Stage 1.5** — `@requires` / `@ensures` parse and lower to runtime asserts. Stage 2.1 adds SMT discharge.

## Design philosophy

Three properties any annotation system has to get right or it dies:

1. **Annotations must be cheap on the human path.** Inferred by default for private functions. `fastc fmt --annotate` writes inferred annotations back into source so the next iteration sees them. Java-verbosity is the failure mode; Rust's lifetime elision is the right ancestor.
2. **Annotations must be mandatory on the public path.** The moment a function is `pub`, the compiler requires the full set. Module headers are mandatory unconditionally. This is what gives an agent the structural guarantee that *reading the signature is sufficient*.
3. **Annotations must be machine-readable.** Every annotation that appears in source emits a corresponding entry in `manifest.json` (the per-build artifact described in [docs/mcp.md](mcp.md)). `fastc explain` and `fastc-mcp` serve them directly.

Precedent acknowledged: Cyclone (regions, early 2000s), Austral (linear caps as arguments, 2022), Koka and Effekt (effect inference), SPARK Ada and F\* (SMT-discharged contracts at industrial scale). The novel contribution of fastC is fusion plus surface syntax for LLM tokenizers and one toolchain that emits machine-readable artifacts.

## Function-level annotation grammar

The complete annotation set for a public function:

```fastc
@mem(arena=req_arena, alloc=O(n))
@caps(fs.read("config/"), net.none)
@requires(input.len > 0 && input.len < 4096)
@ensures(result is ok => result.val.version >= 1)
@panics(never)
@purity(effect)
@complexity(O(n))
pub fn parse_config(input: slice(u8), into: mref(arena)) -> res(Config, Error) { ... }
```

Each annotation is one syntactic form. The parser handles them as a leading sequence on a function or method declaration. Order is not significant; duplicates are an error.

### `@mem(arena=<name>, alloc=<bound>)`

Declares the memory region(s) a function allocates into, and an upper bound on allocation count. Built-in arena names: `req_arena` (request-scoped), `frame_arena` (stack-frame-scoped), `global` (program-lifetime), `stack` (truly on the stack), `static` (read-only data). User-defined arenas are declared at the module level (see module headers).

The compiler tracks region provenance through the AST in a Cyclone-style constraint pass. A function declaring `@mem(arena=req_arena)` may not allocate into `global`; the type system rejects it. Regions nest, and a child region's references cannot escape to a parent.

The `alloc=` clause is a complexity hint, not a hard cap. It is lint-checked: if the body contains an unbounded loop with allocation inside, the declared `O(1)` is rejected.

### `@caps(<capability-set>)`

Declares the capability set the function requires from its caller. Capabilities are typed (see [docs/capabilities.md](capabilities.md)). The empty set — `@caps()` — denotes a pure function with no I/O access. The compiler propagates capability requirements up the call graph; calling a function that requires `fs.read` from a `@caps()` context is a compile error.

The annotation parses as a comma-separated list of capability expressions:

- Atom: `fs.read`, `net.connect`, `proc.spawn`.
- Path-scoped atom: `fs.read("config/")`, `net.connect("api.example.com")`.
- Negation: `net.none` means "explicitly no network capabilities" (useful for clarity, equivalent to omitting `net.*` from the set).
- Union: capabilities are combined by listing them comma-separated.

### `@requires(<predicate>)` and `@ensures(<predicate>)`

Boolean expressions evaluated against the function's argument scope (for `@requires`) and the function's return scope (for `@ensures`, with `result` bound to the return value). See [docs/contracts.md](contracts.md) for the discharge pipeline.

Special forms in `@ensures`:

- `result` — the function's return value.
- `old(<expr>)` — the value of `<expr>` evaluated in the pre-state. Used to express "the new value is the old value plus 1" style invariants.

Predicates are restricted to a pure expression sublanguage: arithmetic, boolean operators, comparison, `result is ok` / `result is err` for `res(T, E)` types, field access on structs, `at(slice, i)` for slice indexing (compiler ensures bounds). No function calls in predicates in v1 (stage 1.5) — too easy to make undecidable. v2 (stage 2.1) may allow calls to `@pure` functions.

### `@panics(<set>)`

Declares which panic kinds a function may raise. The set vocabulary:

- `@panics(never)` — function provably does not panic. Compiler verifies: every `at(s, i)` has an `@requires` covering the bounds; every checked-arithmetic operation has an `@requires` covering overflow; no explicit `panic()` calls reachable.
- `@panics(on=alloc_failure)` — function may panic on allocation failure (via `fc_alloc` returning null in safety mode). Allowed in non-critical safety levels.
- `@panics(on=alloc_failure, on=oob_unchecked)` — explicit enumeration.
- `@panics(always)` — function unconditionally panics (used for `unreachable()`, `todo()`, etc.).

The compiler does a dataflow analysis: collect all panic sites in the body, intersect with the declared set, error on uncovered sites.

### `@purity(<level>)`

Three-level purity lattice:

- `@purity(pure)` — no capabilities, no mutation of arguments, no `loop`/`while` without a termination measure, no recursion. Composes: a pure function can only call other pure functions. Equivalent to `@caps() + @mem(arena=stack) + @panics(never) + bounded termination`.
- `@purity(effect)` — capabilities allowed (per `@caps`), no mutation of arguments other than declared `mref` parameters, no global state mutation. Default for most functions.
- `@purity(io)` — anything goes within declared capabilities. Required for functions that mutate global state via raw pointers (in `unsafe` blocks).

### `@complexity(O(<bound>))`

Big-O time complexity bound. Lint-checked, not proven. Examples: `O(1)`, `O(n)`, `O(n^2)`, `O(n log n)`, `O(log n)`. The lint flags obvious lies (declaring `O(1)` while looping over a slice parameter) but does not attempt to prove tight bounds in the general case. Polynomial bounds only — exponential bounds are not expressible in v1, which is deliberate (an agent looking at an `O(2^n)` function should redesign, not annotate).

## Module-level annotation grammar

Every module begins with a header block of module-level annotations. The header is mandatory unconditionally — even a leaf utility module must have one. The compiler refuses to compile a module that lacks any of the required headers.

```fastc
//! @module: config::loader
//! @owns: file("~/.app/config.toml")
//! @arch: io-boundary
//! @depends: config::parser (parse_toml), config::schema (validate)
//! @threading: single-threaded
//! @invariants:
//!   I1: cached_config is None until load() has returned ok
//!   I2: cached_config never mutates after first set
```

The `//!` prefix matches Rust's inner-doc-comment convention but the contents are first-class annotations, parsed by the fastC parser, not opaque text.

### `@module: <fully-qualified-path>`

The canonical module name. Must match the module's location in the source tree. Mismatch is an error.

### `@owns: <resource-list>`

Declares the resources this module is responsible for. Examples: `file("~/.app/config.toml")`, `port(8080)`, `env_var("APP_TOKEN")`, `arena("req_arena")`. The compiler enforces *global uniqueness*: only one module in the program can claim a given resource path. Two modules claiming `file("/etc/passwd")` is a build error.

This is the property that lets an agent look at a module header and know "this is the module that owns the config file" without reading any code in the rest of the project.

### `@arch: <role>`

Architectural role within the project's layering DAG. Built-in roles:

- `pure` — no I/O, no state, no capabilities. Leaf math/data-transformation modules.
- `leaf` — terminal module, called by others but does not call up.
- `io-boundary` — talks to the outside world; reads files, writes network.
- `gateway` — translates between system layers (e.g., wire protocol → domain model).
- `orchestrator` — top-level coordination; calls into io-boundaries and pure modules.

The compiler enforces the layering DAG: `leaf` cannot depend on `gateway`; `pure` cannot depend on `io-boundary`. Users can declare custom roles in a `fastc.toml` `[architecture]` section.

### `@depends: <module-list>`

Exhaustive list of modules this module depends on, with parenthesized lists of the specific items imported. This is *more than* `use` statements: it is the semantic dependency graph, checked against actual call sites. A module that imports `foo` but never calls anything from it must remove the dependency.

Calling a function from a module not listed in `@depends` is a build error. This is import-graph enforcement at the semantic level, not just the file level.

### `@threading: <model>`

Threading guarantees the module promises to uphold. Values:

- `single-threaded` — all functions assume single-threaded execution. The compiler refuses to compile code that calls into this module from multiple threads (checked via capability annotations on the calling context).
- `thread-safe` — all public functions can be called from any thread concurrently. Internal synchronization is the module's responsibility.
- `send-only` — instances of types from this module can move between threads, but cannot be shared concurrently.

### `@invariants: <named-predicate-list>`

Named module-level invariants. Each invariant has an identifier (`I1`, `I2`, …) and a natural-language predicate. The compiler does not parse the predicates — but every named invariant must be referenced by at least one `@ensures` on a public function of the module, or the build emits a warning. This forces invariants to be tied to the API surface that establishes them.

## Inference and the `--annotate` workflow

Annotations are mandatory on `pub` functions and module headers. They are *optional* on private functions, where the compiler infers them from the body.

When a private function is annotated by inference and the developer wants to see the inferred values, they run:

```
fastc fmt --annotate
```

This rewrites the source file in place, inserting the inferred annotations as if the developer had typed them. The output is idempotent: running `--annotate` twice produces the same file.

The agent workflow is:

1. Agent writes a function. Annotations may be present (if the agent prompt asked for them) or absent.
2. `fastc check` runs. Annotations are inferred for private functions; mandatory checks run for `pub` functions and module headers. If the function is `pub` and missing annotations, the compiler emits a structured diagnostic with a fix-it hint containing the inferred set.
3. Agent runs `fastc fmt --annotate`. The inferred annotations now appear in source. Next turn, the agent sees the annotations and can reason from them.

This is the same loop as Rust's lifetime elision, generalized to the full annotation set.

## Compile-time cost

Annotations are designed to be cheap to check. The expected pass ordering and cost (see [docs/compile-time-budget.md](compile-time-budget.md)):

1. Lex + parse, annotations as first-class grammar. O(source size).
2. Module-graph build — resolve `@owns`, `@depends`, `@arch`. O(modules). Sub-second on any reasonable project; fails fast before body parsing if a module declares a conflicting `@owns`.
3. Type-check with capability types and region variables threaded through. O(program size).
4. Region inference — Cyclone-style constraint solving, one pass per function. O(function size).
5. Capability inference + flow analysis. Linear-time.
6. Purity check — dataflow over the call graph. O(call-graph size).
7. Contract discharge — syntactic pattern matching first (linear), then SMT (stage 2.1, budgeted), then runtime fallback.
8. Panic analysis — collect panic-sites, intersect with `@panics`. Linear-time.

The Salsa-style query system (stage 0.8) caches each of these per function and per module. An incremental edit that touches one function re-runs only that function's passes.

## Build artifacts

Every fastC build emits three machine-readable artifacts alongside the C output:

- `manifest.json` — every function's complete annotation set.
- `caps.json` — the program's full capability graph.
- `discharge.json` — contract discharge results (`proven N`, `runtime-checked M`, `deferred K`).

These are the resources `fastc-mcp` serves to coding agents. They are also what `fastc cert-report` consumes to produce certification evidence.

The JSON shape aligns with the existing `ComplianceReport` / `ViolationDetail` / `SourceLocation` structs in the cert-report machinery, so the MCP server can serve them with minimal new code.

## What is not in the annotation system

Deliberate exclusions, with rationale:

- **No higher-kinded effects or effect polymorphism.** Capabilities are a finite, named lattice. Users do not define new capabilities in v1. This is a deliberate restriction; if a user needs application-level "permissions," they should be a value-level domain model, not a capability extension.
- **No proof terms.** fastC contracts are checked, not proved by user-provided proofs. F\* and Lean provide that path; it does not fit the agent-iteration loop. We rely on Z3 (stage 2.1) for automated discharge.
- **No effect handlers.** Algebraic effects with handlers introduce hidden control flow. fastC capabilities are checked annotations, not a computation model.
- **No annotation polymorphism over types in v1.** A generic function (stage 0.9) is monomorphized per instantiation; each instantiation gets its own annotations. This is the simpler design and matches monomorphization's structure.

## Open questions

These are flagged for the design pass before stage 1.3 begins:

- Should `@invariants` predicates be machine-checked, not just natural-language? Trade-off: easier to write naturally vs. provable mechanically. Current choice is natural-language with `@ensures` reference required; revisit after 1.5 ships.
- Should `@complexity` accept amortized bounds (`O(1) amortized`)? Current answer: no in v1, revisit if real stdlib code needs it.
- Should capabilities support runtime restriction (a `fs.read(*)` token narrowed to `fs.read("./scratch/")` at call time)? Current answer: only at mint time in v1, narrowing is a v2 question.
- Module-header `@threading: single-threaded` — should this be a capability instead of a module-level property? Argument for: composability with the rest of the cap system. Argument against: threading is a *runtime* property, capabilities are a *static* property. Current choice: module-level for v1.
