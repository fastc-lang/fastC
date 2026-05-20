# Contracts

This document specifies fastC's contract system: `@requires` and `@ensures` annotations on function signatures that become compile-time obligations the compiler discharges or enforces at runtime. The signature is the operating manual; the compiler is the verifier.

Contracts ship in two stages:

- **Stage 1.5 — Runtime tier.** `@requires` and `@ensures` parse, type-check, and lower to runtime `assert()`s in debug builds and `__builtin_assume` hints in release. No SMT. Ships the surface syntax and the diagnostic story.
- **Stage 2.1 — SMT discharge.** Three-tier pipeline adds Z3-backed proof for obligations the compiler can express in SMT-LIB. Runtime fallback remains for everything else. `--no-prove` flag lets the agent inner loop skip SMT.

The v1 → v2 path is non-breaking: every contract written against 1.5 will be proof-discharged automatically in 2.1 with no source change.

## Why runtime-first, SMT-later

We considered shipping three-tier discharge in stage 1.5 (the original plan from the strategy discussion). We rejected that for three reasons:

1. **SMT performance is the make-or-break factor.** Z3 timeouts and incompleteness messages must be readable by agents, and getting that UX right is a UX problem nobody has solved well at industrial scale. SPARK Ada is the closest existence proof, and SPARK had decades to tune it. We do not want to gamble the launch on a UX research problem.
2. **The surface syntax can ship without proofs.** A contract that lowers to `assert()` is already useful: it catches violations in tests and at runtime, it documents the API for agents, it gives `cert-report` something to count. The proof is a quality upgrade, not a load-bearing feature for the initial pitch.
3. **Stage 1.5 contracts make stage 2.1 cheap.** When SMT discharge lands in 2.1, every contract written against 1.5 source becomes proof-eligible automatically — the source does not change, only the build pipeline. We get to test the SMT pipeline against a real corpus of contracts written by humans and agents in the wild.

## Surface syntax

`@requires` and `@ensures` are part of the annotation set defined in [docs/annotations.md](annotations.md). They appear on function signatures:

```fastc
@requires(s.len > 0)
@requires(needle.len > 0)
@ensures(result is some => at(s, result.val) == at(needle, 0))
pub fn find_byte(s: slice(u8), needle: slice(u8)) -> opt(usize) {
    for (let i: usize = 0; i + needle.len <= s.len; i = i + 1) {
        if memcmp_slice(slice_from(s, i, needle.len), needle) == 0 {
            return some(i);
        }
    }
    return none(usize);
}
```

Three things to note in the example:

1. Multiple `@requires` are conjoined — all must hold on entry.
2. `result` is bound to the function's return value inside `@ensures`.
3. `at(s, i)` is the bounds-checked slice index — the same expression used in safe code. Contract predicates use the same expression sublanguage as runtime code, restricted to pure operations.

## Predicate sublanguage

Contract predicates are a restricted pure-expression sublanguage. Allowed in v1 (stage 1.5):

- **Arithmetic:** `+`, `-`, `*`, `/`, `%` on integer and float types. Bounds and overflow follow the same rules as safe-mode runtime code.
- **Comparison:** `==`, `!=`, `<`, `<=`, `>`, `>=`.
- **Boolean:** `&&`, `||`, `!`, short-circuit semantics.
- **Implication:** `p => q` is sugar for `!p || q`.
- **Field access:** `s.field` on struct types.
- **Slice index:** `at(s, i)` — the compiler emits a bounds-check obligation alongside the contract obligation.
- **Length:** `s.len` on slices and arrays.
- **Pattern checks:** `result is ok`, `result is err`, `o is some`, `o is none` for `res` and `opt` types.
- **Pre-state reference:** `old(<expr>)` inside `@ensures` — the value of `<expr>` evaluated in the pre-state. Used for "the new value is the old value plus one" style invariants.

Disallowed in v1:

- **Function calls.** Easy to make undecidable; we ban them outright. Stage 2.1 may allow calls to `@pure` functions if the SMT integration can handle them cleanly.
- **Quantifiers** (`forall`, `exists`). Not in v1; stage 2.1 may add bounded forms (`forall i in 0..s.len, P(i)`).
- **Pointer arithmetic.** Pointers are checked at the type level, not the contract level.

The sublanguage is small enough that the v1 lowering (to runtime asserts) is mechanical, and small enough that the v2 SMT encoding is tractable.

## Stage 1.5 — Runtime tier

The v1 implementation lowers every contract obligation to a runtime check:

```c
/* Generated C for find_byte */
opt_usize find_byte(slice_u8 s, slice_u8 needle) {
    /* @requires(s.len > 0) */
    fc_contract_assert(s.len > 0, "find_byte:pre:s.len > 0");
    /* @requires(needle.len > 0) */
    fc_contract_assert(needle.len > 0, "find_byte:pre:needle.len > 0");

    opt_usize __result;
    /* ... body ... */

    /* @ensures(result is some => at(s, result.val) == at(needle, 0)) */
    fc_contract_assert(
        !(__result.tag == OPT_SOME) ||
        (at_u8(s, __result.val) == at_u8(needle, 0)),
        "find_byte:post:..."
    );
    return __result;
}
```

`fc_contract_assert` is in the runtime header. In debug mode it calls `fc_trap` with a structured violation message; in release mode (`fastc build --release`) it lowers to `__builtin_assume`, which is a compiler hint that the predicate is true (and an empty no-op at runtime). The release-mode `__builtin_assume` lets the C compiler optimize using the contract as a known fact — contracts pay for themselves in release performance.

Opt-out: `fastc build --release --check-contracts` keeps the runtime asserts in release builds, for users who want the safety in production. Trade-off: a few percent runtime cost.

### Per-build discharge.json (v1 form)

Every build emits `discharge.json` recording the status of every contract obligation:

```json
{
  "version": "1.0",
  "summary": {
    "total_obligations": 412,
    "proven_syntactic": 0,
    "proven_smt": 0,
    "runtime_checked": 412,
    "deferred": 0
  },
  "obligations": [
    {
      "function": "find_byte",
      "kind": "requires",
      "predicate": "s.len > 0",
      "status": "runtime_checked",
      "location": "src/string.fc:14"
    },
    ...
  ]
}
```

In v1, `proven_syntactic` and `proven_smt` are always zero — every obligation is runtime-checked. The structure exists so that v2 only fills in the columns; no consumer of `discharge.json` (cert-report, MCP, agent tooling) has to change shape between v1 and v2.

### Diagnostics in v1

A contract violation at runtime traps with a structured message via `fc_trap`. The trap message includes the function name, the violated predicate as written in source, and the source location. `fastc fmt --debug-traps` (already shipped in 0.4) can pretty-print these.

A contract that fails to parse, or that references undefined names, is a compile-time error using the existing miette diagnostic surface — same quality as type errors.

## Stage 2.1 — SMT discharge

The v2 implementation adds a three-tier discharge pipeline. Every obligation runs through the tiers in order; the first tier that discharges it wins:

### Tier 1 — Syntactic discharge

Pattern-matching against the caller's known facts. Examples:

- The caller has `if x > 0 { f(x) }`, and `f` declares `@requires(x > 0)`. Syntactically discharged at the call site.
- The caller has `let s = vec_new(); s.push(1)`, and `push` declares `@requires(self.len < self.cap)`. The compiler tracks `len = 0, cap = 4` from `vec_new` and discharges.

Tier 1 is cheap: linear-time pattern matching on the AST. It discharges 50–70% of obligations in typical fastC code, based on prior-art numbers from SPARK Ada.

### Tier 2 — SMT discharge (Z3)

For obligations that survive tier 1, encode the predicate into SMT-LIB and submit to Z3 with a budget (default 500ms per obligation). Three outcomes:

- **Proven.** SMT returns unsat for the negation. Marked `proven_smt` in `discharge.json`.
- **Counter-example.** SMT returns sat for the negation, with a model. Compiler emits a structured diagnostic: "contract may be violated when `x = 0, y = -1`. Strengthen `@requires`?" with the counter-example as a fix-it hint.
- **Unknown / timeout.** SMT returns unknown or hits the budget. The obligation drops to tier 3.

Z3 results are cached in `.fastc/cache/contract_discharge/` keyed by the SHA-256 of the SMT-LIB formula. Re-running the build does not re-prove anything: cache hits are instant.

### Tier 3 — Runtime fallback

Anything that survives tiers 1 and 2 lowers to a runtime assert, exactly as in v1.

### Flags

- `--no-prove` — skip tier 2 entirely, run tier 1 + tier 3. Default in `fastc check` for fast inner-loop development. Agent workflows use this.
- `--prove-budget=<ms>` — override the per-obligation Z3 budget. Default 500ms.
- `--prove-cache=<dir>` — override the cache directory.

CI runs full discharge (no `--no-prove`). Developer inner loops use `--no-prove`. SMT discharge runs as a separate CI job in parallel with tests.

### Per-build discharge.json (v2 form)

```json
{
  "version": "1.0",
  "summary": {
    "total_obligations": 412,
    "proven_syntactic": 287,
    "proven_smt": 89,
    "runtime_checked": 32,
    "deferred": 4
  },
  "obligations": [
    {
      "function": "find_byte",
      "kind": "requires",
      "predicate": "s.len > 0",
      "status": "proven_syntactic",
      "discharged_at": "src/string.fc:14",
      "location": "src/string.fc:14"
    },
    {
      "function": "binary_search",
      "kind": "ensures",
      "predicate": "result is some => at(s, result.val) == target",
      "status": "proven_smt",
      "z3_time_ms": 127,
      "location": "src/algorithms.fc:42"
    },
    {
      "function": "complex_pred",
      "kind": "requires",
      "predicate": "...",
      "status": "deferred",
      "reason": "z3_timeout",
      "location": "src/foo.fc:88"
    }
  ]
}
```

The `summary` is the headline number. A real fastC project at maturity should have >80% of obligations proven (tiers 1+2), with the rest either runtime-checked or deferred (with a documented reason).

## What contracts cover, what they do not

Contracts are about *what the caller must guarantee and what the function will then guarantee in return*. They do not cover:

- **What the function can do.** That is `@caps`. A function with `@requires(x > 0)` is still permitted to read the filesystem if it holds `cap.fs.read`.
- **How much memory the function allocates.** That is `@mem`. A contract on `result is ok` does not constrain allocation.
- **Termination.** A contract is checked at the start (`@requires`) or end (`@ensures`) of execution. A function that does not terminate never reaches `@ensures` — termination is `@panics(never)` + `@complexity` bounds, separately.
- **Concurrency.** Contracts assume single-threaded execution within the function. A `thread-safe` module's contracts hold per call, not across concurrent calls; the module is responsible for serializing where contracts depend on shared state.

## Integration with the rest of fastC

- **`cert-report`** (already shipped, 0.5) consumes `discharge.json` and produces certification evidence. In v1 form, evidence is "this function has 4 contract obligations, all runtime-checked." In v2 form, evidence is "this function has 4 contract obligations, 3 proven by Z3 in <500ms, 1 runtime-checked." DO-178C / IEC 62304 auditors prefer the v2 form.
- **`fastc-mcp`** (stage 1.6) serves `discharge.json` as an MCP resource. An agent can query "what obligations did the build prove for `find_byte`?" and get a typed response.
- **`fastc explain`** (stage 1.6) includes contract status in its per-function output.
- **`fastc check --no-prove`** (stage 1.6) runs the type checker + capability checker + contract syntactic check, but skips SMT. The standard agent inner loop.

## Open questions

These are flagged for the 1.5 and 2.1 implementation:

- **Quantifiers in v1.** Currently disallowed. The minimum viable form is `forall i in 0..s.len, P(i)`. Decision: revisit after 1.5 ships and we see how often users want it. If it lands, syntax should not allow unbounded quantification.
- **Predicate functions.** A `@pure` function with a `bool` return type could be called inside a contract predicate. Decision: not in v1, evaluate for v2 once SMT integration is real (calls complicate SMT encoding significantly).
- **`old()` semantics.** In v1, `old(e)` is captured at function entry. In v2 with SMT, `old(e)` becomes an explicit existential in the encoding. Verify the v1 → v2 path leaves user code unchanged.
- **Contract inheritance through trait methods.** When traits land (stage 1.0), should a `trait Foo { fn bar(...); }` impl carry the contract from the trait declaration? Decision: yes; trait declarations can include contracts, implementations must satisfy them. Implementation detail for the 1.0 → 1.5 integration pass.
- **Counter-example quality.** When Z3 returns a counter-example, the variable values may not map cleanly back to source-visible names. UX work needed in stage 2.1.
