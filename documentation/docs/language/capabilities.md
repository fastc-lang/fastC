# Capabilities

A function that takes no capability arguments structurally cannot
perform I/O. Not because a sandbox blocked it — because the compiler
rejected the call.

This page explains how fastC's capability-typed I/O works, the eight
built-in capabilities, how they're minted, and what the
fabrication-check pass prevents.

## Why ambient I/O is structurally wrong

In C, Rust, Zig, and Go any function can reach for `fs::read`,
`net::connect`, or `time::now` without the signature giving it away.
A logging helper three layers deep can open a socket and you have no
way to tell from its prototype. The "what can this code do" question
isn't answerable from the type — you have to read every line of every
transitive dependency.

fastC takes the opposite stance. Each I/O surface is gated by a typed
capability value that the function must accept as an argument. The
master bundle is minted exactly once, inside `main`, and threaded
downward through the call graph. A function that never receives, say,
a `ref(CapNetConnect)` cannot reach the network — not by convention,
not by sandbox, but because the call to `net::connect` wouldn't
type-check.

An audit by disassembly can then answer "does this function touch
the filesystem?" by reading the signature. No body, no inlined
helper, no transitive call can sneak in I/O the signature didn't
declare.

## The eight built-in capabilities

The prelude declares eight sealed capability structs. Each one gates
exactly one I/O surface area.

| Cap type | Purpose | Lifetime |
|---|---|---|
| `CapFsRead` | read files (stat, open, read) | held by `main` until drop |
| `CapFsWrite` | write or create files | same |
| `CapNetConnect` | outbound network connections | same |
| `CapNetListen` | bind ports, accept connections | same |
| `CapProcSpawn` | spawn child processes | same |
| `CapTimeRead` | wall-clock and monotonic time | same |
| `CapRand` | secure random number generation | same |
| `CapEnvRead` | read environment variables | same |

Each is a zero-field struct living inside `mod caps` in the prelude.
Each is *sealed* — the compiler rejects any struct literal
`CapFsRead {}` written outside `mod caps`. That's the fabrication
check, and it's why a library author can't manufacture authority out
of thin air.

## caps::init

`caps::init() -> Caps` is the one and only constructor for the
master bundle, and it can only be called from the top-level `main`
function (or from inside `mod caps` itself, where the function is
defined).

```c
use caps::init;

fn main() -> i32 {
    let caps: Caps = init();
    return 0;
}
```

The returned `Caps` struct holds one field per capability:

```c
struct Caps {
    fs_read: CapFsRead,
    fs_write: CapFsWrite,
    net_connect: CapNetConnect,
    net_listen: CapNetListen,
    proc_spawn: CapProcSpawn,
    time_read: CapTimeRead,
    rand: CapRand,
    env_read: CapEnvRead,
}
```

`main` is the universe's single source of authority. Anything it
doesn't hand out, no other function can obtain.

## Threading a capability through

A function that wants to read a file's size needs to accept a
`ref(CapFsRead)`. It can then pass that reference to any other
function that requires the same capability — including the cap-typed
stdlib entry points.

```c
use fs::size_bytes;
use caps::init;

fn config_size(fs_read: ref(CapFsRead), path: raw(u8)) -> i64 {
    return size_bytes(fs_read, path);
}

fn main() -> i32 {
    let caps: Caps = init();
    let n: i64 = config_size(addr(caps.fs_read), cstr("/etc/hosts"));
    return cast(i32, n);
}
```

The capability is borrowed, not consumed, so one cap value can be
shared across an unbounded number of calls. Caps are passed by
reference (`ref` for read-only, `mref` for mutable) so the calling
convention is a single pointer per cap.

Drop the cap parameter and the program stops compiling:

```c
fn config_size(path: raw(u8)) -> i64 {
    return size_bytes(path);  // ERROR
}
```

You'll get a type error at the `size_bytes` call site:

```
error: expected 2 arguments, found 1
  --> config.fc:2:12
   |
 2 |     return size_bytes(path);
   |            ^^^^^^^^^^^^^^^^ missing argument of type `ref(CapFsRead)`
```

The fix isn't to disable a lint or to silence a warning — it's to
add the capability parameter to the signature, which makes the I/O
authority visible at every call site upstream.

## Fabrication is rejected

A motivated library author might try to forge a capability instead
of asking for it:

```c
fn evil() -> CapFsRead {
    return CapFsRead {};
}
```

The `cap_check` pass rejects this with a structured diagnostic:

```
error: capability fabrication: 'CapFsRead' can only be constructed
       inside `mod caps`. Receive it as a function argument instead,
       or call `caps::init()` from `main`.
  --> evil.fc:2:12
   |
 2 |     return CapFsRead {};
   |            ^^^^^^^^^^^^
```

The same check fires for `Caps`, `CapFsWrite`, `CapNetConnect`,
`CapNetListen`, `CapProcSpawn`, `CapTimeRead`, `CapRand`, and
`CapEnvRead`. The only place those struct literals are legal is
inside `mod caps`, which lives in the compiler's prelude and is not
user-extensible.

The same pass also rejects `caps::init()` calls outside `main`,
including the aliased form:

```c
use caps::init;

fn sneaky() -> Caps {
    return init();  // ERROR: `caps::init()` is `main`-only.
}
```

Without that second check, a library could mint the whole bundle on
your behalf and you'd be back to ambient authority. The lint scans
`use` items so the bare-name spelling fails the same way as the
qualified one.

## Capabilities erase to zero at runtime

Capability tokens carry no runtime payload. They compile down to
empty C structs (`struct { }`), the C compiler inlines and discards
them, and the resulting binary is byte-for-byte identical to the
ambient-I/O equivalent. The fastC hello-world binary is 53 KB on
darwin-arm64 with the full capability machinery active — the same
size it would be without it. A function that takes seven cap
parameters compiles to a function that takes zero parameters at the
ABI level.

The cost is entirely paid at compile time, in the form of
type-checker work. The runtime cost is zero bytes and zero cycles.

## The caps.json artifact

Every compile can emit a per-build summary of which functions take
which capability arguments. Pass `--caps-output=<path>` to `fastc
compile` and the compiler writes a JSON document describing the
program's full capability surface.

```bash
fastc compile main.fc --emit=c --caps-output=caps.json
```

The artifact looks like:

```json
{
  "schema": "fastc.caps.v1",
  "functions": [
    { "name": "config_size", "caps": ["CapFsRead"] },
    { "name": "main", "caps": [] }
  ],
  "summary": {
    "total_functions": 12,
    "capability_using": 4,
    "capabilities_seen": ["CapFsRead", "CapNetConnect"]
  }
}
```

The headline number for an auditor is `capabilities_seen`: the
aggregate set of capabilities any function in the program declares.
"This program reaches the network and the read-side of the
filesystem; it does not write files, spawn processes, listen on
ports, read the clock, generate random numbers, or read environment
variables" — directly from the artifact, no source needed.

Per-function entries let downstream tooling (agents, MCP servers,
static-analysis dashboards) answer the same question for any
specific symbol.

See the [`fastc compile` reference](../cli/compile.md) for the full
flag list.

## Auditing without source

The two structural properties together give the audit-by-disassembly
guarantee:

1. **Fabrication forbidden.** Every `Cap*` value in the program
   traces back to a single `caps::init()` call in `main`. There is
   no other constructor.
2. **`caps::init` is `main`-only.** The bundle mint point is itself
   not reachable from library code. The compiler enforces this in
   the same pass that enforces fabrication.

Combine those with the per-build `caps.json` and an auditor with
only the compiled binary can reason about the program's authority:
read `caps.json` to enumerate what the program declares, then
disassemble each cap-using function and confirm its body only calls
other functions that are declared in the same artifact. A function
whose entry in `caps.json` lists `[]` is structurally pure with
respect to I/O — it might still loop forever or trap on overflow,
but it provably cannot open a socket, touch the filesystem, or read
the clock.

This is the property fastC exists to provide. Ambient-authority
languages cannot offer it at all.

## Cross-links

- [Annotations](annotations.md) — `@pure`, `@nolibc`, and other
  function-level audit markers
- [`fastc compile --caps-output`](../cli/compile.md) — CLI flag
  reference for the caps.json artifact
- [Safety reference](../reference/safety.md) — full list of the
  safety guarantees fastC enforces
- [fastc-core packages](fastc-core.md) — `mod fs`, `mod net`,
  `mod time`, `mod env` and the rest of the cap-gated stdlib surface
