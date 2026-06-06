# Target Command

The `target` command group inspects fastC's cross-compile target registry
and verifies that the backend toolchain can produce a binary for a given
triple.

fastC emits portable C11, so any C cross-compiler reaches any target it
supports. We ship presets for `zig cc` — clang plus bundled libcs, dozens
of targets out of the box, no sysroot setup — and let teams swap in their
own toolchain via `--cc-override`.

## Usage

```bash
fastc target <SUBCOMMAND>
```

## Subcommands

| Subcommand | Description |
|-----------|-------------|
| `list` | Print the table of known cross-compile targets. |
| `check <TRIPLE>` | Verify the toolchain can produce a binary for `<TRIPLE>`. |

---

## `fastc target list`

Print the canonical table of cross-compile targets fastC ships presets for.

```bash
fastc target list
```

Output:

| Triple | Use case |
|---|---|
| `aarch64-linux-musl` | ARM cloud, edge, static deploy |
| `x86_64-linux-musl` | x86 cloud, static deploy |
| `aarch64-linux-gnu` | ARM with glibc compatibility |
| `x86_64-linux-gnu` | Standard Linux x86 |
| `aarch64-macos` | Apple Silicon native |
| `x86_64-macos` | Intel Mac compatibility |
| `wasm32-wasi` | Sandboxed WASM (agent workloads, plug-ins) |
| `riscv64-linux-musl` | RISC-V, forward-looking |

A footer line reminds you the backend is `zig cc` by default and points at
`--cc-override` as the escape hatch for proprietary cross-toolchains.

Adding a target is a six-line patch in `crates/fastc/src/targets.rs` — the
list grows as upstream demand arrives.

---

## `fastc target check <TRIPLE>`

Verify that the configured backend can produce a binary for `<TRIPLE>`.
Exits 0 on success, 1 on failure. The intended use is CI matrices that
want to skip targets the runner can't build.

```bash
fastc target check <TRIPLE> [--cc-override <PATH>]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<TRIPLE>` | A triple from the `target list` table (case-sensitive). |

### Options

| Option | Description |
|--------|-------------|
| `--cc-override <PATH>` | Use the named binary as the C compiler instead of `zig cc`. |

### Examples

A target that's available:

```bash
fastc target check aarch64-linux-musl
```

```
OK: target `aarch64-linux-musl` available via `zig cc --target=aarch64-linux-musl`.
```

Exit code: 0.

A target whose backend is missing:

```bash
fastc target check wasm32-wasi
```

```
ERROR: target `wasm32-wasi` requested but `zig` is not on PATH. Install zig
(e.g. `brew install zig`) or pass `--cc-override=<path-to-cross-compiler>` to
use a different toolchain.
```

Exit code: 1.

An unknown triple:

```bash
fastc target check riscv32-freestanding
```

```
Error: unknown target `riscv32-freestanding`. Run `fastc target list` to see
supported triples.
```

Exit code: non-zero.

---

## Cross-Compiling

The `--target=<TRIPLE>` flag is wired into both `fastc compile` and
`fastc build`:

```bash
fastc compile src/main.fc --target=aarch64-linux-musl -o build/main.c
fastc build --target=aarch64-linux-musl --cc -o build/main
```

The resolver runs in priority order:

1. `--cc-override=<PATH>` wins unconditionally. Use it for proprietary
   toolchains (crosstool-ng, distro `gcc-cross-*`, vendor SDKs).
2. With `--target` and `zig` on `PATH`, the resolver invokes
   `zig cc --target=<triple>`.
3. With `--target` and no `zig`, the build fails with the same message
   you'd see from `fastc target check`.
4. Without `--target`, the default is plain `cc` (or `tcc` under
   `--dev`).

### Forwarding C Flags

Use `--cflags` to forward additional flags to the C backend. The value is
a single string that's passed through to the compiler:

```bash
fastc build --target=x86_64-linux-musl --cc --cflags "-O3 -DPRODUCTION=1" -o build/server
```

For target-specific sysroot or include directories, combine `--cflags`
with `--cc-override`:

```bash
fastc build \
  --target=aarch64-linux-gnu \
  --cc-override=/opt/cross/bin/aarch64-linux-gnu-gcc \
  --cflags "--sysroot=/opt/cross/sysroot -O2" \
  --cc -o build/edge-agent
```

---

## Worked Example: Static ARM Linux Binary

Produce a fully-static ARM Linux binary from a macOS dev box. Requires
`zig` on `PATH`:

```bash
fastc build --target=aarch64-linux-musl --cc -o build/agent
file build/agent
```

```
build/agent: ELF 64-bit LSB executable, ARM aarch64, version 1 (SYSV),
statically linked, with debug_info, not stripped
```

The musl libc is statically linked — drop the binary on any aarch64
Linux box and run it. No shared-library hunt.

### Worked Example: WASI Runtime Plug-in

```bash
fastc build --target=wasm32-wasi --cc -o build/plugin.wasm
file build/plugin.wasm
```

```
build/plugin.wasm: WebAssembly (wasm) binary module version 0x1 (MVP)
```

Run with any WASI runtime:

```bash
wasmtime build/plugin.wasm
```

The `.wasm` extension is appended automatically for `wasm32-wasi`
outputs.

---

## Note on `fastc run --target=...`

`fastc run` accepts `--target=<TRIPLE>` for symmetry with `build`, but
refuses to execute non-native binaries. Use `build` to produce the
cross-compiled artifact and ship it to the target host.

---

## See Also

- [Compile](compile.md) — `--target` flag on the compile step.
- [Build & Run](build-run.md) — the full build pipeline that consumes
  the resolved C backend.
