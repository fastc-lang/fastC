//! Cross-compilation target registry.
//!
//! fastC compiles to portable C11, so any C cross-compiler reaches any target
//! that compiler supports. We ship presets for `zig cc` (clang + bundled libcs,
//! 50+ targets out of the box, no sysroot setup) and let teams swap in their
//! own toolchain via `--cc-override`.
//!
//! Adding a target is: add a `Target` enum variant, add a case to every match
//! arm. The CLI exposes the variants through `fastc target list` and
//! `fastc target check <triple>`.

use std::fmt;

/// One of the cross-compilation targets fastC ships presets for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// aarch64-linux-musl — ARM cloud, edge, static deploy.
    Aarch64LinuxMusl,
    /// x86_64-linux-musl — x86 cloud, static deploy.
    X86_64LinuxMusl,
    /// aarch64-linux-gnu — ARM with glibc compatibility.
    Aarch64LinuxGnu,
    /// x86_64-linux-gnu — Standard Linux x86.
    X86_64LinuxGnu,
    /// aarch64-macos — Apple Silicon.
    Aarch64Macos,
    /// x86_64-macos — Intel Mac.
    X86_64Macos,
    /// wasm32-wasi — Sandboxed WASM runtimes (agent workloads, plugins).
    Wasm32Wasi,
    /// riscv64-linux-musl — RISC-V, forward-looking.
    Riscv64LinuxMusl,
}

impl Target {
    /// All known targets, in the canonical order shown by `fastc target list`.
    pub fn all() -> &'static [Target] {
        &[
            Target::Aarch64LinuxMusl,
            Target::X86_64LinuxMusl,
            Target::Aarch64LinuxGnu,
            Target::X86_64LinuxGnu,
            Target::Aarch64Macos,
            Target::X86_64Macos,
            Target::Wasm32Wasi,
            Target::Riscv64LinuxMusl,
        ]
    }

    /// Look up a target by its triple string. Case-sensitive.
    pub fn from_triple(s: &str) -> Option<Target> {
        match s {
            "aarch64-linux-musl" => Some(Target::Aarch64LinuxMusl),
            "x86_64-linux-musl" => Some(Target::X86_64LinuxMusl),
            "aarch64-linux-gnu" => Some(Target::Aarch64LinuxGnu),
            "x86_64-linux-gnu" => Some(Target::X86_64LinuxGnu),
            "aarch64-macos" => Some(Target::Aarch64Macos),
            "x86_64-macos" => Some(Target::X86_64Macos),
            "wasm32-wasi" => Some(Target::Wasm32Wasi),
            "riscv64-linux-musl" => Some(Target::Riscv64LinuxMusl),
            _ => None,
        }
    }

    /// Canonical triple string (e.g. "aarch64-linux-musl").
    pub fn triple(&self) -> &'static str {
        match self {
            Target::Aarch64LinuxMusl => "aarch64-linux-musl",
            Target::X86_64LinuxMusl => "x86_64-linux-musl",
            Target::Aarch64LinuxGnu => "aarch64-linux-gnu",
            Target::X86_64LinuxGnu => "x86_64-linux-gnu",
            Target::Aarch64Macos => "aarch64-macos",
            Target::X86_64Macos => "x86_64-macos",
            Target::Wasm32Wasi => "wasm32-wasi",
            Target::Riscv64LinuxMusl => "riscv64-linux-musl",
        }
    }

    /// Short use-case label shown by `fastc target list`.
    pub fn description(&self) -> &'static str {
        match self {
            Target::Aarch64LinuxMusl => "ARM cloud, edge, static deploy",
            Target::X86_64LinuxMusl => "x86 cloud, static deploy",
            Target::Aarch64LinuxGnu => "ARM with glibc compatibility",
            Target::X86_64LinuxGnu => "Standard Linux x86",
            Target::Aarch64Macos => "Apple Silicon native",
            Target::X86_64Macos => "Intel Mac compatibility",
            Target::Wasm32Wasi => "Sandboxed WASM (agent workloads, plug-ins)",
            Target::Riscv64LinuxMusl => "RISC-V, forward-looking",
        }
    }

    /// File extension to append to the output binary (e.g. ".wasm" for wasi).
    pub fn output_extension(&self) -> &'static str {
        match self {
            Target::Wasm32Wasi => ".wasm",
            _ => "",
        }
    }

    /// Default `zig cc` flags for this target. The first element is always
    /// `--target=<triple-zig-style>`; subsequent flags handle ABI / libc /
    /// sysroot quirks that zig doesn't pick up automatically.
    ///
    /// The "zig-style" triple differs slightly from fastC's canonical form:
    /// macOS uses `aarch64-macos` in fastC, `aarch64-macos-none` in zig. We
    /// translate here so users never see the difference.
    pub fn zig_cc_flags(&self) -> Vec<&'static str> {
        match self {
            Target::Aarch64LinuxMusl => vec!["--target=aarch64-linux-musl"],
            Target::X86_64LinuxMusl => vec!["--target=x86_64-linux-musl"],
            Target::Aarch64LinuxGnu => vec!["--target=aarch64-linux-gnu"],
            Target::X86_64LinuxGnu => vec!["--target=x86_64-linux-gnu"],
            Target::Aarch64Macos => vec!["--target=aarch64-macos-none"],
            Target::X86_64Macos => vec!["--target=x86_64-macos-none"],
            Target::Wasm32Wasi => vec!["--target=wasm32-wasi"],
            Target::Riscv64LinuxMusl => vec!["--target=riscv64-linux-musl"],
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.triple())
    }
}

/// Resolve which C compiler binary to invoke for a given target/override
/// combination. Logic in priority order:
///
/// 1. `--cc-override=<path>` (`cc_override` arg) wins unconditionally.
///    Used when a team has a proprietary toolchain (e.g. crosstool-ng,
///    gcc-cross with a vendor sysroot).
/// 2. If `target` is set and `zig` is on PATH, return `("zig", &["cc", ...zig_cc_flags()])`.
/// 3. If `target` is set and zig is NOT on PATH, return an error indicating
///    the user needs to either install zig or pass --cc-override.
/// 4. If `target` is unset and `--dev` is set, fall back to the existing
///    `detect_dev_compiler("cc")` behavior.
/// 5. Otherwise return the literal "cc" string.
pub fn resolve_target_compiler(
    target: Option<Target>,
    cc_override: Option<&str>,
) -> Result<TargetCompiler, String> {
    if let Some(path) = cc_override {
        return Ok(TargetCompiler {
            command: path.to_string(),
            extra_args: vec![],
        });
    }
    if let Some(t) = target {
        if which_zig().is_some() {
            let mut extras = vec!["cc".to_string()];
            for flag in t.zig_cc_flags() {
                extras.push(flag.to_string());
            }
            return Ok(TargetCompiler {
                command: "zig".to_string(),
                extra_args: extras,
            });
        }
        return Err(format!(
            "target `{}` requested but `zig` is not on PATH. Install zig (e.g. `brew install zig`) \
            or pass `--cc-override=<path-to-cross-compiler>` to use a different toolchain.",
            t.triple()
        ));
    }
    Ok(TargetCompiler {
        command: "cc".to_string(),
        extra_args: vec![],
    })
}

/// Result of resolving the right C compiler for a target/override
/// combination. `command` is what we hand to `Command::new`; `extra_args`
/// are flags prepended to the user-supplied arg list before any cflags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetCompiler {
    pub command: String,
    pub extra_args: Vec<String>,
}

fn which_zig() -> Option<std::path::PathBuf> {
    let path_env = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_env) {
        let cand = dir.join("zig");
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_round_trips_through_triple() {
        for t in Target::all() {
            let triple = t.triple();
            assert_eq!(Target::from_triple(triple), Some(*t));
        }
    }

    #[test]
    fn unknown_triple_returns_none() {
        assert_eq!(Target::from_triple("riscv32-freestanding"), None);
        assert_eq!(Target::from_triple("x86_64-windows-msvc"), None);
        assert_eq!(Target::from_triple(""), None);
    }

    #[test]
    fn wasm_target_has_wasm_extension() {
        assert_eq!(Target::Wasm32Wasi.output_extension(), ".wasm");
        assert_eq!(Target::Aarch64LinuxMusl.output_extension(), "");
    }

    #[test]
    fn override_wins_over_target() {
        let r = resolve_target_compiler(
            Some(Target::Aarch64LinuxMusl),
            Some("/opt/custom/cross-gcc"),
        )
        .unwrap();
        assert_eq!(r.command, "/opt/custom/cross-gcc");
        assert!(r.extra_args.is_empty());
    }

    #[test]
    fn no_target_returns_plain_cc() {
        let r = resolve_target_compiler(None, None).unwrap();
        assert_eq!(r.command, "cc");
        assert!(r.extra_args.is_empty());
    }

    #[test]
    fn target_with_zig_returns_zig_cc() {
        // This test depends on zig being on PATH; skip when absent so CI
        // runners without zig still pass.
        if which_zig().is_none() {
            return;
        }
        let r = resolve_target_compiler(Some(Target::Aarch64LinuxMusl), None).unwrap();
        assert_eq!(r.command, "zig");
        assert_eq!(r.extra_args[0], "cc");
        assert!(
            r.extra_args
                .iter()
                .any(|a| a == "--target=aarch64-linux-musl")
        );
    }
}
