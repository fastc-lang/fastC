# Modules

fastC has a small but strict module system. `mod foo;` declares a
module loaded from `src/foo.fc`; `mod foo { ... }` is an inline
module body; `use mod::item;` imports a name from another module.

Stage 1.3 added optional but structurally-checked module headers:
`//! @module / @owns / @arch / @depends / @threading / @invariants`
lines at the top of a `mod` body. Lenient mode (the v1.0 default)
accepts header-less modules; a module with even one `//!` line must
declare every required key.

## Basic module shapes

An inline module body lives in the same file:

```c
mod legacy {
    pub fn helper(x: i32) -> i32 {
        return (x + 1);
    }
}

fn main() -> i32 {
    return legacy::helper(0);
}
```

An external module declaration loads its body from `src/<name>.fc`:

```c
// In src/main.fc — loads body from src/helper.fc
mod helper;

fn main() -> i32 {
    return helper::run();
}
```

`pub` marks an item as visible to other modules. Items without `pub`
are module-private and resolve only inside the declaring scope.

## Module headers

The full shape of a header-bearing module:

```c
mod tested {
    //! @module = "tested"
    //! @owns = "tested"
    //! @arch = "core"
    //! @depends = ""
    //! @threading = "single"
    //! @invariants = "non-negative inputs"

    pub fn double(x: i32) -> i32 {
        return (x + x);
    }
}
```

Six required keys. Every `//!` line sits inside the `{ ... }` block
at the top of the body. The parser stops absorbing header lines at
the first non-`//!` token.

### `@module`

Display name for the module. Conventionally matches the `mod` decl
name, though the compiler doesn't require it. Used by `fastc explain`
and the diagnostic surface when it has to identify the module by
name.

### `@owns`

Comma-separated list of namespaces this module is the sole owner of.
A single module typically owns one namespace that matches its own
name; library modules sometimes own several adjacent ones.

```c
//! @owns = "logging, log_format, log_sink"
```

Globally unique across the whole compilation unit — two modules
claiming the same `@owns` value is a compile error (see
[Cross-module checks](#cross-module-checks)).

### `@arch`

Architectural layer the module belongs to. Layers form a DAG; a
lower layer cannot depend on a higher one. Ordering is implicit —
the first module to declare a given `@arch` value pins it to the
"lowest" rank, subsequent distinct values rank higher in declaration
order.

Common values in practice: `"core"`, `"runtime"`, `"adapters"`,
`"app"`.

### `@depends`

Comma-separated list of modules this one may import from. An empty
string is valid for leaf modules with no cross-module imports:

```c
//! @depends = ""
```

Every `use mod::X` in the body must point at a module listed in
`@depends` (or at `std::` / `core::`, which are exempt). The check
fires only on modules that themselves declare a header — legacy
header-less modules are not retro-audited.

### `@threading`

One of `"single"`, `"thread_safe"`, or `"concurrent"`. Documentation
in v1.x — a hook for tooling and reviewers, not yet enforced by the
compiler. The key is mandatory in any complete header so the
intent is recorded explicitly rather than assumed.

### `@invariants`

Free-text invariants the module promises to uphold. Multiple
`@invariants` lines accumulate:

```c
//! @invariants = "buffer length fits in i32"
//! @invariants = "no item is dropped twice"
```

Lifted into the `fastc explain` JSON output and surfaced by the
review tooling. Unlike `@requires` / `@ensures` on functions
(see [Contracts](contracts.md)), `@invariants` is informational —
it is not discharged by the SMT solver.

## Cross-module checks

When at least one module in the file declares a header, three
structural checks fire across every header-bearing module. Modules
without a header are skipped entirely in lenient mode.

### `@owns` uniqueness

Two modules claiming `@owns = "shared"` is a compile error:

```c
mod alpha {
    //! @module = "alpha"
    //! @owns = "shared"
    //! @arch = "core"
    //! @depends = ""
    //! @threading = "single"
    //! @invariants = "ok"
    pub fn one() -> i32 { return 1; }
}

mod beta {
    //! @module = "beta"
    //! @owns = "shared"           // duplicate — rejected
    //! @arch = "core"
    //! @depends = ""
    //! @threading = "single"
    //! @invariants = "ok"
    pub fn two() -> i32 { return 2; }
}
```

Diagnostic:

```
@owns namespace 'shared' is claimed by both 'alpha' and 'beta'.
Each namespace must have exactly one owner module.
```

### `@depends` exhaustiveness

Every `use mod::X` from a header-bearing module must name a
module in the declared `@depends` list:

```c
mod app {
    //! @module = "app"
    //! @owns = "app"
    //! @arch = "app"
    //! @depends = ""              // empty, but app uses log
    //! @threading = "single"
    //! @invariants = "ok"

    use log::info;                  // rejected — not in @depends
    pub fn run() { info(cstr("hi")); }
}
```

Diagnostic:

```
module 'app' uses 'log' but does not declare it in @depends.
Add 'log' to the @depends list at the top of the module.
```

`std::` and `core::` imports are exempt — the prelude is always
available without listing it.

### `@arch` DAG layering

A module whose `@arch` ranks lower than another cannot depend on
the higher-ranked one. The first distinct `@arch` value encountered
ranks lowest:

```c
mod bottom {
    //! @module = "bottom"
    //! @owns = "bottom"
    //! @arch = "lower"             // rank 0 (declared first)
    //! @depends = "top"
    //! @threading = "single"
    //! @invariants = "ok"

    use top::ping;
    pub fn call_top() -> i32 { return ping(); }
}

mod top {
    //! @module = "top"
    //! @owns = "top"
    //! @arch = "upper"             // rank 1
    //! @depends = ""
    //! @threading = "single"
    //! @invariants = "ok"

    pub fn ping() -> i32 { return 1; }
}
```

Diagnostic:

```
module 'bottom' (arch='lower') depends on 'top' (arch='upper').
Architecture layering is a DAG — lower layers cannot depend on
higher layers.
```

To fix: flip the declaration order so `top` ranks lowest and
`bottom` depends downward, or merge the modules if the layering
was incidental.

## Strict mode

Opt-in via `fastc.toml`:

```toml
[package]
name = "myapp"
strict_modules = true
```

In strict mode, every inline `mod` block in the project must have a
complete header. A header-less module trips the same diagnostic as
a partial header:

```
module 'foo' is missing the mandatory `//!` header. Add
`//! @module / @owns / @arch / @depends / @threading / @invariants`
at the top of the body. Disable with `strict_modules = false` in
`fastc.toml`.
```

Lenient mode (the v1.0 default) lets header-less modules through
untouched. `fastc new` scaffolds projects with `strict_modules = true`
so new code starts honest; established codebases adopt it gradually.

## Partial headers are rejected in either mode

A module with even one `//!` line must declare every required key.
A partial header is always a compile error — lenient mode is not a
license to be sloppy, it's a license to opt out of the header system
entirely:

```c
mod partial {
    //! @module = "partial"
    //! @owns = "partial"
    // missing @arch, @depends, @threading, @invariants
    pub fn id(x: i32) -> i32 { return x; }
}
```

Diagnostic:

```
module 'partial' has a partial header (missing @arch). A `//!`
block must declare every required key.
```

One diagnostic per missing key, so the fix is a single edit pass.

## The modules array in `fastc explain` JSON

`fastc explain <file>` JSON output now carries a top-level `modules`
array with each header-bearing module's parsed fields:

```json
{
  "file": "src/main.fc",
  "modules": [
    {
      "name": "tested",
      "module": "tested",
      "owns": ["tested"],
      "arch": "core",
      "depends": [],
      "threading": "single",
      "invariants": ["non-negative inputs"]
    }
  ],
  "functions": [...]
}
```

Header-less modules are omitted from the array — the field is only
present for modules that have opted in. Useful for review tooling
that wants to render an architecture map without re-parsing source.

## Cross-links

- CLI: [`fastc explain`](../cli/explain.md) — emits the modules array
- [fastc-core](fastc-core.md) — the curated module set bundled with
  the v1.0 prelude
- [Capabilities](capabilities.md) — `Cap*` tokens are typically
  threaded across module boundaries
