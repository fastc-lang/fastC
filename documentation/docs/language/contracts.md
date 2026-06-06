# Contracts

`@requires(<expr>)` and `@ensures(<expr>)` are first-class function
annotations. Every clause is checked. Some are proven at compile time
via SMT and cost zero at runtime. The rest fall back to a runtime trap.

The model is simple: the signature is the operating manual, and the
compiler is the verifier. A `@requires` clause is a promise the caller
makes to the function. An `@ensures` clause is a promise the function
makes back to the caller. fastC enforces both, statically where it can,
dynamically where it must.

## The basic shape

```c
@requires(divisor != 0)
fn safe_div(value: i32, divisor: i32) -> i32 {
    return (value / divisor);
}

@ensures(result >= 0)
fn abs(x: i32) -> i32 {
    if (x < 0) { return (0 - x); }
    return x;
}
```

A few rules to keep in mind:

- Multiple `@requires` clauses on the same function are an implicit
  `AND`. All of them must hold on entry.
- `@ensures` exposes a special `result` identifier bound to the
  function's return value.
- `@requires` is checked at function entry. `@ensures` is checked at
  every `return` site in the body, including early returns.
- Clauses are pure boolean expressions over the function's parameters
  (plus `result` for `@ensures`). No side effects, no calls to user
  functions, no allocation.

The same annotation works on void functions, where `@ensures` can
constrain global state observable through reference parameters. The
`result` identifier is unavailable in that case.

## The three-tier discharge pipeline

Every obligation runs through three tiers in order. The first tier
that discharges it wins; everything else falls through to the next.

### Tier 1: syntactic

Always on. Walks the AST and tries to fold the clause to literal
`true` using cheap structural pattern matches:

- Constant folding (`@requires(1 + 1 == 2)`).
- Tautology detection (`@requires((x > 0) || (x == 0) || (x < 0))`
  collapses to `true`).
- Identity arithmetic (`@ensures(x + 0 == x)`).
- Unsigned-parameter shortcuts (`@requires(n >= 0)` for a parameter
  declared `n: u64` is true by construction).

If the clause folds to `true`, the runtime check is elided entirely.
Zero cost. Tier 1 runs even with `--no-prove` because there is no
failure mode to surface and no external dependency.

### Tier 2: SMT

Opt-in via `--prove`. Shells out to `z3 -smt2 -in` with a
per-obligation budget enforced by both `(set-option :timeout <ms>)`
and a process-level kill at twice the budget.

The encoding is body-aware:

- Every parameter becomes a Z3 constant of the matching sort
  (`Int` for integer types, `Bool` for `bool`).
- For `@ensures`, an additional `result` constant of the return
  type is declared.
- The function's `@requires` clauses are fed in as assumptions so
  the postcondition can lean on the precondition.
- For straight-line bodies (a single `return EXPR;` over the
  supported expression subset), the body's return expression is
  asserted as the model for `result` via
  `(assert (= result <body_expr>))`. The obligation can then
  reference body-computed values.

The pipeline `assert (not <clause>)` and checks for `unsat`. `unsat`
means no counterexample exists; the clause is proven and the
runtime trap is elided. `sat` means Z3 found a counterexample and
the runtime trap stays in. `unknown` or timeout drops to tier 3.

### Tier 3: runtime trap

The fallback. Anything tier 1 and tier 2 couldn't prove lowers to
`if (!cond) fc_trap()` in the generated C. This is the v1.5
baseline; later tiers are pure optimization. The proof gap stays
observable in the binary and in `discharge.json`, so a failed
proof never silently weakens the program.

## --prove / --no-prove / --prove-budget

The discharge pass is configured at the CLI:

- `--prove` enables tier 2. Z3 must be on `PATH`; if it isn't,
  fastC degrades to a warning plus tier-3 fallback rather than
  failing the build.
- `--no-prove` (the default for `fastc check`) skips tier 2
  entirely. Tier 1 still runs because it's free.
- `--prove-budget=<ms>` overrides the per-obligation SMT budget.
  Default is 500 ms. Raise it for clauses that need more solver
  time; lower it for tighter inner loops.
- `--discharge-output=<path>` writes the report JSON to that path.
  Passing `-` writes the report to stderr instead, which is
  useful for piping into other tooling.

The intended split is `fastc check --no-prove` for the inner
loop and `fastc compile --prove` (or a dedicated CI job) for the
full discharge run. The same source compiles under both modes;
only the report and the elision pattern differ.

## The discharge.json artifact

Every build emits a structured report. The shape:

```c
{
  "proven": 7,
  "runtime": 3,
  "unknown": 1,
  "obligations": [
    {
      "function": "safe_div",
      "clause": "requires",
      "index": 0,
      "status": "proven",
      "tier": 1
    },
    {
      "function": "abs",
      "clause": "ensures",
      "index": 0,
      "status": "proven",
      "tier": 2
    },
    {
      "function": "complex_pred",
      "clause": "requires",
      "index": 0,
      "status": "unknown",
      "reason": "SMT timed out after 500 ms ..."
    }
  ]
}
```

The aggregate counts at the top are the headline number. Each
entry in `obligations` records:

- `function` — the mangled, module-qualified function name.
- `clause` — one of `requires`, `ensures`, or `call_site`. The
  last is the N1 call-site discharge (covered below) for direct
  calls, method dispatch, and fn-pointer bindings.
- `index` — zero-based position of the clause inside the
  function's clause list. `@requires[0]`, `@requires[1]`, etc.
- `status` — `proven`, `runtime`, or `unknown`.
- `tier` — `syntactic` or `smt`, present only for `proven`.
- `reason` — a kind-aware diagnostic hint, present for `runtime`
  and `unknown`.

The schema is stable across the v1 to v2 path: consumers of
`discharge.json` (the `cert-report` tool, `fastc-mcp`, agent
tooling) never have to handle a shape change.

## The on-disk cache

SMT discharge is deterministic on the SMT-LIB text fastC hands the
solver. fastC exploits that by caching results keyed on the SHA-256
of the encoded query:

```c
.fastc/cache/discharge/<sha256-of-smt-text>.bin
```

Each `.bin` file is a single byte:

- `P` (0x50) — proven.
- `F` (0x46) — Z3 returned a counterexample. Falls to runtime.
- `T` (0x54) — timeout or `unknown`.
- `U` (0x55) — encoder skipped (unsupported clause shape).

No version header, no JSON, no length prefix. A cache miss is cheap
(fastC re-runs Z3 anyway), and the hit path benefits from minimal
I/O. The cache survives across builds and produces roughly an 18x
speedup on SMT-heavy reruns.

The cache key is the full SMT-LIB text. Any change to the
obligation expression, the assumptions, the body model, or the
budget produces a new hash and a new entry. Old entries pile up;
`rm -rf .fastc/cache/discharge/` reclaims the space whenever you
want.

## When the encoder times out

If Z3 exceeds the budget, the obligation gets `status: "unknown"`
and the runtime trap stays in. The report's `reason` field looks
like:

```c
SMT timed out after 500 ms on @requires[0] in safe_div. Try
splitting the clause into smaller conjuncts (each is discharged
independently), or raise --prove-budget=<ms>. The runtime check
is still emitted.
```

Two ways to react:

1. **Split the clause.** Multiple `@requires` are an implicit
   `AND`, but each clause is discharged independently. A single
   complex predicate `@requires(p1 && p2 && p3)` can become
   three smaller clauses that the tier-1 syntactic pass can
   often catch piecemeal.
2. **Raise the budget.** `--prove-budget=2000` gives the solver
   2 seconds per obligation. Useful for clauses that genuinely
   need more time; less useful as a blanket fix.

A failed proof is never silently fatal. The runtime check is
emitted regardless of whether tier 2 ran, returned `sat`, or
timed out.

## Call-site discharge

Tier 2 proves clauses that are *universally true* over their
parameters. A precondition like `@requires(x > 0)` isn't
universally true — it depends on what the caller passed — so by
itself it would always fall to runtime.

The N1 call-site pass closes that gap. For every call site
`caller(args)` to a function `f` with `@requires`, fastC
substitutes the call's arguments for `f`'s parameters in each
`@requires` clause and runs the resulting expression through the
same three-tier pipeline. Direct calls, method dispatch, and
bound fn-pointers (`let g = f; apply(g, x)`) all discharge.

A small example:

```c
@requires(divisor != 0)
fn safe_div(value: i32, divisor: i32) -> i32 {
    return (value / divisor);
}

fn use_it() -> i32 {
    return safe_div(10, 5);
}
```

At the call site `safe_div(10, 5)`, `divisor` substitutes to `5`,
the clause becomes `5 != 0`, and tier 1 constant-folds it to
`true`. The call-site obligation is reported with
`"clause": "call_site"` and `"status": "proven"`. The callee's
own `@requires` runtime trap stays in for defense in depth, but
the report shows that this specific call site is statically safe.

Caller `@requires` clauses are also passed in as assumptions, so
a call like `safe_div(x, 2)` inside a function declared
`@requires(x > 0)` discharges both the substituted clause and any
postcondition the caller may want to prove over the call's
result.

Opaque fn-pointer parameters — `fn apply(f: fn(i32) -> i32, x: i32)`
where the callee isn't statically known — fall through to runtime.
Whole-program callee inference is a v2.x follow-up; for now the
call-site report flags those as `runtime` with a hint pointing at
the parameter.

## Cross-links

- [Annotations](annotations.md) covers the full annotation set
  (`@caps`, `@mem`, `@panics`, `@complexity`) that `@requires`
  and `@ensures` are part of.
- [CLI: fastc compile](../cli/compile.md) documents the
  `--prove`, `--no-prove`, `--prove-budget`, and
  `--discharge-output` flags in context.
- [Certification reference](../reference/certification.md)
  explains how `discharge.json` plugs into DO-178C and IEC 62304
  evidence packs.
- [Power-of-10 reference](../reference/power-of-10.md) covers the
  related static-analysis rules that contracts complement.
