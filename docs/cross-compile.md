# Cross-compilation

fastC compiles to portable C11. That means **every C cross-compiler in the
world is a fastC cross-compiler**. We don't maintain our own
cross-compilation infrastructure — we wrap the best one (`zig cc`) by
default and let teams plug in their own toolchain when they need to.

## TL;DR

```sh
fastc target list                          # see what's supported
fastc build --target=aarch64-linux-musl    # build for ARM Linux (static)
fastc build --target=wasm32-wasi           # build for WASM (sandboxed)
fastc target check x86_64-linux-gnu        # verify the backend works
```

That's it.

## Why this is structurally simpler than what compilers usually do

A "real" cross-compiler maintains:

- A code generator for each target architecture (instruction selection,
  register allocation, scheduling).
- A bundled libc for each target/ABI combination (musl, glibc, wasi-libc,
  Apple's Libsystem, Windows UCRT).
- A sysroot manager for headers, link scripts, and platform stubs.

We do **none of that**. fastC emits C; we hand it to a C compiler that's
already paid the cost. `zig cc` is the easiest one to ship — it bundles
clang plus libcs for 50+ targets and works without any sysroot setup.

The strategic claim is one sentence: *fastC cross-compiles to any target
its underlying C toolchain reaches.* Default to zig and you cover the
practical surface area. Use `--cc-override` and you cover everything else.

## Installation

Cross-compilation needs `zig` on PATH. One-line install:

```sh
brew install zig            # macOS
apt install zig             # Ubuntu 24.04+
pacman -S zig               # Arch
```

Or download a release tarball from [ziglang.org/download](https://ziglang.org/download/).

`fastc target check <triple>` is the diagnostic — it exits 0 when the
backend can produce a binary for that triple, and 1 otherwise. Used in
CI matrices to skip targets the runner doesn't support.

## Shipped targets (v1.9)

| Triple | Use case |
|---|---|
| `aarch64-linux-musl` | ARM cloud, edge, fully static deploy |
| `x86_64-linux-musl` | x86 cloud, fully static deploy |
| `aarch64-linux-gnu` | ARM with glibc compatibility |
| `x86_64-linux-gnu` | Standard Linux x86 |
| `aarch64-macos` | Apple Silicon dev / native deploy |
| `x86_64-macos` | Intel Mac compatibility |
| `wasm32-wasi` | Sandboxed WASM (agent workloads, plug-in execution) |
| `riscv64-linux-musl` | RISC-V — shipping in volume by late 2026 |

`fastc target list` prints this table from the running binary, so it
stays in sync as the matrix grows.

The eight targets cover where fastC plausibly competes today: cloud
deploy (musl/gnu × aarch64/x86), Apple Silicon dev loop, sandboxed
agent runtimes (WASM), and the RISC-V ecosystem now hitting production
volume.

## CLI

### `fastc build --target=<triple>`

Compiles the project for `<triple>`, producing a binary under
`build/<package_name>` (or `build/<package_name>.wasm` for the WASI
target). The fastC compilation step is target-independent — only the
`cc` invocation changes.

```sh
fastc build --target=aarch64-linux-musl
# → build/hello, statically linked, runs on any ARM Linux box
```

### `fastc build --cc-override=<path>`

Bypasses zig entirely. Useful when you have a vendor cross-toolchain
(crosstool-ng, Yocto SDK, proprietary embedded compiler) that already
knows its target.

```sh
fastc build --cc-override=/opt/freertos-gcc/bin/arm-eabi-gcc --cflags="-mcpu=cortex-m4"
```

`--cc-override` wins over `--target`. If you pass both, fastC trusts
that your custom compiler targets the right triple — we don't second-guess
your toolchain.

### `fastc target list`

Prints the shipped target table.

### `fastc target check <triple>`

Verifies the backend can produce a binary for `<triple>`. Returns 0 on
success, non-zero on failure. The output names which command would be
invoked:

```sh
$ fastc target check aarch64-linux-musl
OK: target `aarch64-linux-musl` available via `zig cc --target=aarch64-linux-musl`.
```

CI matrices use this to skip targets the runner doesn't support without
producing red builds.

### `fastc run` and `--target`

`fastc run --target=<triple>` is explicitly **rejected**. Running a
cross-compiled binary needs an emulator (qemu-user, wasmtime,
docker --platform) that fastC doesn't manage. Use `fastc build
--target=<triple>` to produce the binary, then run it yourself.

## Custom toolchains via `--cc-override`

The escape hatch covers three real scenarios:

**Proprietary embedded toolchains.** ARM Compiler 6, IAR EWARM, vendor
gcc-cross with a sysroot baked in. Pass the absolute path; fastC treats
it as a plain `cc` and lets `--cflags` handle the rest.

```sh
fastc build --cc-override=/opt/iar/bin/iccarm --cflags="--cpu=Cortex-M7 -e"
```

**Distro cross-gcc.** Debian's `gcc-aarch64-linux-gnu` package ships
`aarch64-linux-gnu-gcc` ready to use:

```sh
fastc build --cc-override=$(which aarch64-linux-gnu-gcc)
```

**Custom musl-cross.** If you maintain your own musl cross-toolchain
(crosstool-ng, musl.cc downloads):

```sh
fastc build --cc-override=/opt/musl-cross/bin/aarch64-linux-musl-gcc
```

We don't ship sysroot tooling. That's the user's responsibility when
overriding the default. The fact that zig bundles libcs is one of the
reasons we default to it.

## Runtime portability

The fastC runtime (`runtime/fastc_runtime.h`) is C11 plus a small POSIX
slice: `unistd.h` (`access`), `sys/stat.h` (`stat`), `time.h` (`time`),
and `stdlib.h` (`getenv`). Every v1.9 target's libc provides that
surface — including wasi-libc, which has been POSIX-compatible for the
file/time APIs since 2024.

**Not in the v1.9 matrix:** Windows-msvc. Different ABI (no POSIX
`access`, `_stat` instead of `stat`) and the userbase asking for it
isn't large enough yet to justify the `#ifdef _WIN32` wrappers. If you
need it, open an issue with the use case.

## Strategy

fastC's cross-compile story compounds with two of its other promises:

- **Portable C output is auditable.** The same `.c` file goes to every
  target. Diffing the produced C across targets shows the only difference
  is the toolchain's libc resolutions — the fastC-emitted code is
  bit-identical.
- **Cap-typed I/O is platform-neutral.** A `ref(CapFsRead)` is a `ref(CapFsRead)`
  on WASI as much as on Linux. The capability story doesn't fork per
  target because the language doesn't fork per target.

The competition:

- **Rust:** ~200 targets via rustup, but each target needs a separate
  `rustup target add` and a sysroot dance. fastC's eight targets work
  out of the box once zig is installed.
- **Go:** 50+ GOOS/GOARCH combinations, no sysroot needed — but Go's
  static binaries are 2–3 MB, fastC's are 50–60 KB. We're not trying
  to match Go's count; we're trying to match its ergonomics with
  C-class binaries.
- **Zig:** 50+ targets, same backend we're using. fastC inherits Zig's
  cross-compile excellence without inheriting its language.
- **C / C++:** depends entirely on the toolchain. fastC + zig cc is
  effectively "C with the best cross-compiler pre-wired."

## Verified targets

All eight targets are verified by `crates/fastc/tests/cross_compile.rs`,
which compiles `examples/hello.fc` for each triple and checks the
produced file's magic bytes. CI runs this on every PR when zig is
available; the test is skipped gracefully otherwise.

## Open questions / future work

- **iOS / Android.** Both have stable ABIs zig can target, but the
  ecosystems require NDK/SDK paths that aren't a one-line install.
  Deferred until a real user asks.
- **Windows-msvc.** See "Runtime portability" above. The runtime
  wrapping is straightforward; the question is who's going to maintain
  the test coverage.
- **WASM beyond WASI.** Browser WASM (component model, bindgen) is a
  separate effort tracked under stage 2.3+ in the roadmap.
