# Supply-chain side-by-side: Cargo build.rs vs fastc.toml

This directory contains two minimal projects designed to illustrate the dominant supply-chain attack vector of the 2025–2026 Rust / npm / Zig wave, and fastC's structural defense against it.

## The two projects

`cargo_attack/` — a normal Cargo project with a `build.rs` build script. The script prints what a real malicious dependency would have done (read env vars / hostname, open an outbound connection, drop a binary). The script does NOT actually exfiltrate — but it prints a banner to prove it ran.

`fastc_safe/` — the equivalent fastC project. The manifest (`fastc.toml`) has no key for a build script, because the schema doesn't define one. A malicious dependency has nowhere to put a script.

## Run the demo

```bash
# Show that cargo silently executes build.rs as part of `cargo build`:
cd cargo_attack
cargo build
# You'll see lines like:
#   warning: INSTALLED MALWARE (DEMO — see build.rs)
#   warning: A real attacker here could read every
#   warning: file your user account can touch...
# These came from build.rs, which ran with full user privileges
# BEFORE main.rs was compiled. No prompt, no confirmation.
```

```bash
# Show that fastC's manifest has no place to put a build script:
cd ../fastc_safe
# Edit fastc.toml and uncomment the `build = "build.fc"` line.
fastc build
# Build fails with:
#   error: unknown field `build` for type `Manifest`
# The schema literally has no key to attach executable code to.
```

## What real attacks looked like

- **faster_log (April 2025).** Typosquat of `fast_log`. The `build.rs` read `~/.aws/credentials` and `~/.ssh/id_rsa`, base64-encoded them, posted to a Telegram bot. Affected 4 weeks before takedown.
- **async_println (June 2025).** Plausible-looking async logging crate. The build script ran during `cargo install`, even though the user only intended to run a cli tool. Dropped a persistent backdoor in `~/.cargo/bin`.
- **evm-units (November 2025).** Targeted Ethereum tooling developers. `build.rs` scanned for `~/.ethereum`, `~/.config/Solidity`, and posted entire keystore directories. Estimated USD-equivalent loss in published incident reports: $1.2M.
- **CVE-2026-28353** (early 2026). Affected the `time` crate's build.rs through a transitive dependency on a freshly-published utility crate. The attack was supply-chain-deep: most affected users had no direct dependency on the malicious crate.

Each of these attacks had **the same shape**: a `build.rs` file in a published crate, executed by Cargo without confirmation when any downstream user ran `cargo build`. The Rust language and Cargo's design both permit this. The user receives no notification, no review prompt, no opt-out.

## What fastC does instead

fastC's manifest format is **closed-schema TOML**. The `Manifest` struct in `crates/fastc/src/deps/manifest.rs` lists every legal field:

```rust
pub struct Manifest {
    pub package: Package,
    pub build: BuildConfig,                 // include_dirs + link_libs; no script field
    pub dependencies: HashMap<String, Dependency>,
}
```

There is no `script` field, no `postinstall` hook, no `hook`-anywhere. A malicious dependency has no syntactic place to attach executable code. The compiler reads the manifest as pure data, looks up the dependencies (each pinned by commit + sha256 + Sigstore bundle), and compiles them. At no point does any user code from a dependency run before the program does.

If a dependency wants to run code, it has to ship that code as a regular fastC function the user explicitly calls. The act of `fastc build` does not invoke that code.

## What this trade-off costs

fastC can't ship build-time codegen the way Cargo's `proc-macro` crates or `build.rs` scripts do. A dependency that needs to (for example) parse a `.proto` file at build time has to either:

1. Generate the `.fc` file at the dependency-author's machine and check it in, OR
2. Provide a runtime parser the user invokes from their own code.

Some real build-time codegen workflows lose ergonomics under this constraint. We think that's an acceptable cost for closing the dominant supply-chain attack vector of the era. Languages that disagree should keep `build.rs`; fastC will not.

## Re-running this demo

The `cargo_attack` project requires Cargo / a Rust toolchain. The `fastc_safe` project requires the fastC binary at `target/release/fastc`. Both projects are deliberately minimal so re-running is fast: `cargo build` and `fastc build` complete in seconds. The banner in cargo's output is the entire demonstration.
