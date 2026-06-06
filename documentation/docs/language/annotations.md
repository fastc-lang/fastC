# Annotations

fastC has a small, fixed set of structured `@`-prefix annotations that
attach to functions. Some are enforced by the compiler — violating them
is a hard compile error. The rest are documentation-only: they parse,
they round-trip through the AST, and they surface in `fastc explain`
JSON for agent tooling, but they do not gate compilation.

The whole catalog is finite and frozen for the v1.x line. New
annotations require a minor version bump and a stability review.

## Annotation catalog

| Annotation                       | Grammar                                | Enforced? | Notes                                          |
|----------------------------------|----------------------------------------|-----------|------------------------------------------------|
| `@noalloc`                       | `@noalloc`                             | Yes       | Transitive BFS over the allocator banned-list. |
| `@purity(pure)`                  | `@purity(pure)`                        | Yes       | No alloc, no I/O, no logging in the call set.  |
| `@purity(effect)`                | `@purity(effect)`                      | No        | Documentation-only.                            |
| `@purity(io)`                    | `@purity(io)`                          | No        | Documentation-only.                            |
| `@panics(never)`                 | `@panics(never)`                       | Yes       | No `fc_trap` / `panic` / `abort` / `exit`.     |
| `@panics(always)`                | `@panics(always)`                      | No        | Documentation-only.                            |
| `@panics(on = <expr>)`           | `@panics(on = <expr>)`                 | No        | Documentation-only.                            |
| `@requires(<expr>)`              | `@requires(<expr>)`                    | Runtime   | Lowers to `if (!cond) fc_trap()` at entry.     |
| `@ensures(<expr>)`               | `@ensures(<expr>)`                     | Runtime   | Same, at return; `result` is bound.            |
| `@mem(arena = <ident>)`          | `@mem(arena = <ident>)`                | No        | Documentation-only in v1.x.                    |
| `@complexity(O(<shape>))`        | see [shapes](#complexityoshape)        | No        | Documentation-only.                            |
| `@pure`                          | `@pure`                                | No        | Pre-v1.3 boolean flag; documentation.          |
| `@nodiverg`                      | `@nodiverg`                            | No        | Pre-v1.3 boolean flag; documentation.          |
| `@test`                          | `@test`                                | Yes       | Stripped in default builds.                    |
| `@repr(C)`                       | `@repr(C)`                             | Yes       | Struct attribute; layout = C.                  |

Duplicate annotations on the same function are a compile error
(`duplicate @purity`, etc.), which keeps the canonical AST one-to-one
with the source.

## `@noalloc`

### Grammar

```c
@noalloc
fn name(...) -> T { ... }
```

No arguments. Bare flag.

### Semantics

The annotated function — and every function reachable from it via the
transitive call graph — must not call into the allocator. The banned
set is fixed:

- `alloc`, `resize`, `free_bytes`
- `mem::alloc`, `mem::resize`, `mem::free_bytes`
- `malloc`, `realloc`, `free` (the libc externs)
- `mem::malloc`, `mem::realloc`, `mem::free`

### Enforcement

`crates/fastc/src/noalloc_check.rs` runs after resolve. It builds an
outgoing-calls map for every function in the file, then for each
`@noalloc` entry point it BFS-walks the call set. Any banned-list hit
emits one diagnostic per reached callee.

Indirect calls via function pointers are not resolved in v1 — the
analysis treats fn-pointer arguments as opaque. That is a documented
gap; a future points-to analysis will close it.

### Example

```c
@noalloc
fn pure_math(x: i32, y: i32) -> i32 {
    return (x * y) + x;
}
```

Compiles fine. Now violate it:

```c
use mem::alloc;

@noalloc
fn evil() -> rawm(u8) {
    return alloc(cast(usize, 16));
}
```

Diagnostic:

```
error: @noalloc function 'evil' reaches 'alloc' (transitive call).
       Either drop the @noalloc annotation or refactor to avoid the
       heap allocator.
```

The diagnostic points at the function declaration (not the inner
call), because the annotation contract is what's broken.

## `@requires` and `@ensures`

### Grammar

```c
@requires(<expr>)
@ensures(<expr>)
fn name(...) -> T { ... }
```

The expression is a normal boolean fastC expression evaluated in the
function's parameter scope. `@ensures` additionally binds `result` to
the about-to-return value.

### Semantics

These are runtime preconditions and postconditions. Each lowers to:

```c
if (!cond) fc_trap();
```

at the appropriate edge — `@requires` at function entry,
`@ensures` at every `return` site, with `result` bound to the
returned expression.

In v2.1 a tier-2 SMT discharger can prove obligations statically and
elide the runtime check. Until then they always fire at runtime.

See [Contracts](contracts.md) for the full design, including the tier
boundary between syntactic and SMT discharge.

### Example

```c
@requires(n > 0)
@ensures(result >= 0)
fn isqrt(n: i32) -> i32 {
    let r: i32 = 0;
    while (r + 1) * (r + 1) <= n {
        r = r + 1;
    }
    return r;
}
```

Calling `isqrt(-1)` traps at entry. Returning a negative value would
trap at the `return` (it cannot, given the loop, but the check is
emitted regardless until the discharger proves it).

## `@purity(pure | effect | io)`

### Grammar

```c
@purity(pure)
@purity(effect)
@purity(io)
```

Exactly one argument. The three levels form a totally-ordered scale.

### Semantics

- `pure` — the function must be free of allocation, observable I/O,
  and structured logging via the transitive call graph.
- `effect` — the function may mutate state but does not perform I/O.
  Documentation only.
- `io` — the function performs observable I/O. Documentation only.

### Enforcement

`crates/fastc/src/annotation_check.rs` runs the same BFS walker as
`@noalloc`, with an expanded banned-list that includes the
allocator surface plus `io::*`, `log::*`, `http::*`, `fs::*`,
`net::*`, `env::*`, `time::*`, `rand::*` — every cap-using prelude
function.

Only `pure` is checked. `effect` and `io` flow through the AST and
surface in `fastc explain` JSON but never reject compilation.

### Example

```c
@purity(pure)
fn square(x: i32) -> i32 {
    return (x * x);
}
```

Compiles. Now make it impure:

```c
use io::println;

@purity(pure)
fn impure() -> i32 {
    println(cstr("hi"));
    return 0;
}
```

Diagnostic:

```
error: @purity(pure) function 'impure' reaches 'println'
       (transitive call). Pure functions cannot allocate, log, or
       perform I/O. Drop the annotation or refactor to remove the
       side effect.
```

## `@panics(never | always | on = <expr>)`

### Grammar

```c
@panics(never)
@panics(always)
@panics(on = <expr>)
```

### Semantics

- `never` — the function must not reach `fc_trap`, `panic`, `abort`,
  or `exit` via the transitive call graph.
- `always` — the function always traps. Documentation only.
- `on = <expr>` — the function traps when `<expr>` is true.
  Documentation only.

The banned-set for `never` is: `fc_trap`, `panic`, `abort`,
`exit`, `fc_panic`, `core::panic`, `core::abort`.

### Enforcement

`@panics(never)` uses the same transitive BFS as `@noalloc`. There
is one important caveat: v1.x cannot statically track *implicit*
traps from integer overflow or bounds-check failures inside the
function body — only explicit calls. The annotation means "this
function does not *intentionally* trap", not "no path can fault".

The discharger work in v2.1 will tighten that bound.

### Example

```c
@panics(never)
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

Compiles. Mark a trap-reaching function `never`:

```c
@panics(never)
fn boom() -> i32 {
    abort();
}
```

Diagnostic:

```
error: @panics(never) function 'boom' reaches 'abort' (transitive
       call). Either drop the annotation or refactor to avoid the
       trap path.
```

## `@mem(arena = <ident>)`

### Grammar

```c
@mem(arena = scratch)
@mem(arena = request)
```

The argument is an identifier naming an arena.

### Semantics

Documentation-only in v1.x. The annotation declares which arena the
function's allocations come from. Arena-aware allocators land in
v2.x; when they do, this annotation will become an enforced
allocator-selection contract.

For now, it parses and surfaces in `fastc explain` JSON.

### Example

```c
@mem(arena = scratch)
fn from_scratch(x: i32) -> i32 {
    return (x * 2);
}
```

## `@complexity(O(<shape>))`

### Grammar

```c
@complexity(O(1))
@complexity(O(n))
@complexity(O(log n))
@complexity(O(n log n))
@complexity(O(n^2))
@complexity(O(2^n))
```

Recognized shapes:

- `O(1)` — constant
- `O(n)` — linear
- `O(log n)` — logarithmic
- `O(n log n)` — linearithmic
- `O(n^k)` for k ≥ 2 — polynomial
- `O(2^n)` — exponential

Anything the parser does not recognize as one of the above falls
into the opaque `Other(s)` slot, carrying the raw string through to
agent tooling.

### Semantics

Documentation-only. Surfaces in `fastc explain` JSON so that agents
and reviewers can sort/filter functions by claimed cost class. The
compiler does not verify the claim.

### Example

```c
@complexity(O(n))
fn sum(xs: slice(i32)) -> i32 {
    let total: i32 = 0;
    for x in xs {
        total = total + x;
    }
    return total;
}
```

## `@pure` and `@nodiverg`

These are the pre-v1.3 boolean-flag annotations. They predate the
structured `@purity` / `@panics` forms.

- `@pure` — declares the function is side-effect-free.
- `@nodiverg` — declares the function does not diverge (no infinite
  loops, no unconditional traps).

Both are documentation-only and remain accepted for source
compatibility. New code should prefer `@purity(pure)` and
`@panics(never)`, which the compiler enforces.

## `@test`

### Grammar

```c
@test
fn name() { ... }
```

No arguments.

### Semantics

The function is stripped from the AST in default builds — it never
reaches the lowerer, never enters the C output, and never contributes
to binary size. Under `fastc compile --test`, `@test` functions are
preserved.

A v1.x follow-up will add the runner main that discovers and invokes
every `@test` in the project; the strip / preserve plumbing is
already in place.

### Example

```c
@test
fn isqrt_zero_is_zero() {
    assert(isqrt(0) == 0);
}
```

## `@repr(C)`

A struct attribute (not a function annotation) used at the FFI
boundary. It forces the C-ABI layout — same field order, same
padding, no compiler-internal reordering — so the struct can be
shared with C code.

```c
@repr(C)
struct Header {
    magic: u32,
    version: u16,
    flags: u16,
}
```

See [C Interop](../c-interop/index.md) for the full FFI surface.

## How annotations flow through agent tooling

Every function's annotations are recorded on the AST and surface in
`fastc explain <file>` JSON, one entry per function. The keys are:

- `purity` — `"pure"` | `"effect"` | `"io"` | `null`
- `panics` — `"never"` | `"always"` | `{ "on": "<expr>" }` | `null`
- `complexity` — `"O(1)"` | `"O(n)"` | ... | `{ "other": "<raw>" }` | `null`
- `mem` — `{ "arena": "<ident>" }` | `null`
- `is_test` — `true` | `false`
- `noalloc` — `true` | `false`

A sample fragment:

```json
{
  "functions": [
    {
      "name": "isqrt",
      "purity": "pure",
      "panics": "never",
      "complexity": "O(log n)",
      "mem": null,
      "is_test": false,
      "noalloc": true,
      "requires": ["n > 0"],
      "ensures": ["result >= 0"]
    }
  ]
}
```

This is the agent-facing surface. Tools like reviewers, capability
auditors, and `cert-report` consume it directly — they never re-parse
the source.

## Multiple annotations on one function

Annotations stack. The canonical ordering, from outermost (capability
class) to innermost (proof obligation), is:

```c
@purity(pure)
@complexity(O(n))
@noalloc
@requires(n >= 0)
@ensures(result >= 0)
fn sum_first_n(n: i32) -> i32 {
    let total: i32 = 0;
    let i: i32 = 0;
    while i < n {
        total = total + i;
        i = i + 1;
    }
    return total;
}
```

What each one buys:

- `@purity(pure)` — the compiler proves no I/O, no logging, no
  allocator reach.
- `@complexity(O(n))` — documents the cost class for agent tooling.
- `@noalloc` — the compiler proves no allocator reach.
- `@requires(n >= 0)` — runtime check on entry.
- `@ensures(result >= 0)` — runtime check on return.

`@purity(pure)` already implies `@noalloc`'s banned-list at the
allocator end, so stating both is belt-and-braces; the enforcer
handles it cleanly because each annotation runs its own BFS.

Order in the source file is free — the compiler reads them as a set,
not a sequence. The order above is the convention because it reads
top-down from "what kind of function is this" to "what must hold when
you call it".

## Cross-links

- [Contracts deep-dive](contracts.md) — full semantics of
  `@requires` / `@ensures`, the tier-1/tier-2 discharge split, and
  the runtime trap protocol.
- [CLI: `fastc explain`](../cli/explain.md) — the JSON surface that
  agent tooling consumes.
- [CLI: `fastc compile --output-format=json`](../cli/compile.md) —
  machine-readable compilation output.
- [Power-of-10 reference](../reference/power-of-10.md) — the rule set
  that motivates `@noalloc`, `@panics(never)`, and the bounded-loop
  posture.
