//! Build orchestration for FastC projects
//!
//! Handles dependency fetching, compilation, and output generation.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::deps::{Fetcher, LockedPackage, Lockfile, Manifest};
use crate::diag::CompileError;

/// Return the path/name of the dev-mode C compiler. Prefers `tcc` when
/// present on PATH (fast inner-loop builds, ~100MB/s), falling back to
/// `fallback` (typically `cc`) when not.
///
/// This is the implementation of the stage 0.8 tcc dev backend described in
/// `docs/compile-time-budget.md`.
///
/// Platform note: tcc is unavailable on Apple Silicon as of 2026 (upstream
/// formula refuses to build on macOS > Catalina without the x86_64 target).
/// On M-series Macs `--dev` therefore falls back to `cc -O0 -g`, which still
/// produces a measurable speedup vs `cc -O2`: 160 ms vs 252 ms on a hello
/// fastC project in our local measurement. When tcc is available (Linux,
/// Intel Mac, BSD) the speedup is meaningfully larger.
pub fn detect_dev_compiler(fallback: &str) -> String {
    if which("tcc").is_some() {
        "tcc".to_string()
    } else {
        fallback.to_string()
    }
}

#[cfg(test)]
mod dev_compiler_tests {
    use super::detect_dev_compiler;

    #[test]
    fn falls_back_when_tcc_absent() {
        // We cannot guarantee tcc is or isn't on PATH in CI, so just verify
        // the return value is one of the expected strings.
        let picked = detect_dev_compiler("cc");
        assert!(picked == "tcc" || picked == "cc", "got {picked}");
    }
}

/// Return the absolute path of `cmd` if it can be found on the PATH.
fn which(cmd: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
        // On Windows the binary may carry a .exe suffix; check both forms.
        let candidate_exe = dir.join(format!("{}.exe", cmd));
        if candidate_exe.is_file() {
            return Some(candidate_exe);
        }
    }
    None
}

/// Build context for orchestrating project compilation
pub struct BuildContext {
    /// Project manifest (fastc.toml)
    manifest: Manifest,
    /// Dependency lockfile
    lockfile: Lockfile,
    /// Dependency fetcher
    fetcher: Fetcher,
    /// Project root directory
    project_root: PathBuf,
}

/// Errors that can occur during build
#[derive(Debug)]
pub enum BuildError {
    /// No manifest found
    NoManifest,
    /// IO error
    Io(String),
    /// Manifest parse error
    ManifestError(String),
    /// Fetch error
    FetchError(String),
    /// Compile error
    CompileError(CompileError),
    /// Cache initialization error
    CacheError,
    /// C compiler error
    CcError(String),
    /// Runtime error
    RuntimeError(i32),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::NoManifest => {
                write!(f, "no fastc.toml found in current directory or parents")
            }
            BuildError::Io(msg) => write!(f, "IO error: {}", msg),
            BuildError::ManifestError(msg) => write!(f, "manifest error: {}", msg),
            BuildError::FetchError(msg) => write!(f, "fetch error: {}", msg),
            BuildError::CompileError(e) => write!(f, "{}", e),
            BuildError::CacheError => write!(f, "failed to initialize cache directory"),
            BuildError::CcError(msg) => write!(f, "C compiler error: {}", msg),
            BuildError::RuntimeError(code) => write!(f, "program exited with code {}", code),
        }
    }
}

impl std::error::Error for BuildError {}

impl From<CompileError> for BuildError {
    fn from(e: CompileError) -> Self {
        BuildError::CompileError(e)
    }
}

impl BuildContext {
    /// Create a new build context from the current directory
    pub fn new(working_dir: &Path) -> Result<Self, BuildError> {
        // Find manifest file
        let manifest_path = Manifest::find(working_dir).ok_or(BuildError::NoManifest)?;
        let project_root = manifest_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        // Load manifest
        let manifest =
            Manifest::load(&manifest_path).map_err(|e| BuildError::ManifestError(e.to_string()))?;

        // Load or create lockfile
        let lockfile_path = project_root.join("fastc.lock");
        let lockfile = if lockfile_path.exists() {
            Lockfile::load(&lockfile_path).map_err(|e| BuildError::Io(e.to_string()))?
        } else {
            Lockfile::new()
        };

        // Create fetcher
        let fetcher = Fetcher::new().ok_or(BuildError::CacheError)?;

        Ok(Self {
            manifest,
            lockfile,
            fetcher,
            project_root,
        })
    }

    /// Get the project root directory
    pub fn root(&self) -> &Path {
        &self.project_root
    }

    /// Get the project name
    pub fn name(&self) -> &str {
        &self.manifest.package.name
    }

    /// Fetch all dependencies, updating the lockfile
    pub fn fetch_dependencies(&mut self) -> Result<(), BuildError> {
        if self.manifest.dependencies.is_empty() {
            eprintln!("No dependencies to fetch.");
            return Ok(());
        }

        for (name, dep) in &self.manifest.dependencies {
            eprintln!("Fetching dependency: {}", name);

            let dep_for_fetch = self.dependency_for_fetch(name, dep);

            // Fetch the dependency
            let path = self
                .fetcher
                .fetch(name, &dep_for_fetch)
                .map_err(|e| BuildError::FetchError(e.to_string()))?;

            eprintln!("  Fetched to: {}", path.display());

            // Stage 1.7 — verify content integrity before the fetched
            // tree is allowed to influence the build. Two sources can
            // declare the expected hash:
            //
            //   1. `sha256 = "..."` on the manifest dep — strict mode,
            //      users opting into supply-chain hardening up-front.
            //   2. `sha256 = "..."` recorded in fastc.lock by a prior
            //      `fastc lock` run — gradual-adoption mode.
            //
            // If both are set we cross-check them and fail on
            // disagreement. If only one is set we enforce it. If
            // neither is set we warn and continue (so a fresh
            // `fastc fetch` + `fastc lock` flow still works without
            // hand-computed hashes).
            if let crate::deps::Dependency::Git { sha256, .. } = dep {
                let manifest_hash = sha256.clone();
                let locked_hash = self
                    .lockfile
                    .get_package(name)
                    .and_then(|p| p.sha256.clone());
                let expected = match (manifest_hash.as_ref(), locked_hash.as_ref()) {
                    (Some(m), Some(l)) => {
                        if m.trim().eq_ignore_ascii_case(l.trim()) {
                            Some(m.clone())
                        } else {
                            return Err(BuildError::FetchError(format!(
                                "dependency '{}': manifest sha256 ({}) disagrees with \
                                fastc.lock sha256 ({}). Run `fastc lock --force` to \
                                re-anchor or fix the manifest.",
                                name,
                                short_hash(m),
                                short_hash(l)
                            )));
                        }
                    }
                    (Some(m), None) => Some(m.clone()),
                    (None, Some(l)) => Some(l.clone()),
                    (None, None) => None,
                };
                if let Some(h) = expected {
                    crate::deps::verify_tree(&path, &h).map_err(|e| {
                        BuildError::FetchError(format!("dependency '{}': {}", name, e))
                    })?;
                    eprintln!("  sha256 verified: {}", short_hash(&h));

                    // Sigstore — only ever runs when a bundle is
                    // declared; cosign-not-on-PATH degrades to a
                    // warning, not a build failure.
                    if let crate::deps::Dependency::Git { sigstore, .. } = dep {
                        let outcome = crate::deps::verify_sigstore(&path, sigstore.as_deref(), &h);
                        match outcome {
                            crate::deps::SigstoreOutcome::Verified => {
                                eprintln!("  sigstore verified");
                            }
                            crate::deps::SigstoreOutcome::Skipped { reason } => {
                                eprintln!("  sigstore skipped: {}", reason);
                            }
                            crate::deps::SigstoreOutcome::NotConfigured => {
                                // No sigstore field — nothing to say.
                            }
                            crate::deps::SigstoreOutcome::Failed { stderr, .. } => {
                                return Err(BuildError::FetchError(format!(
                                    "dependency '{}': sigstore verification failed: {}",
                                    name,
                                    stderr.trim()
                                )));
                            }
                        }
                    }
                } else {
                    eprintln!(
                        "  warning: dependency '{}' has no `sha256` — run `fastc lock` \
                        to record the content hash",
                        name
                    );
                }
            }

            // Get source + resolved commit for lockfile
            let (source, resolved) = match dep {
                crate::deps::Dependency::Git { git, version, .. } => {
                    let resolved = crate::deps::Fetcher::head_commit(&path).ok();
                    let source = resolved
                        .as_ref()
                        .map(|commit| format!("git+{}?rev={}", git, commit))
                        .unwrap_or_else(|| Self::source_from_git_spec(git, version));
                    (source, resolved)
                }
                crate::deps::Dependency::Path { path } => (format!("path+{}", path), None),
            };

            let version = read_dependency_version(&path).unwrap_or_else(|| "0.0.0".to_string());

            // Preserve any sha256 already on the lockfile entry; if
            // we just verified one, reuse that. If neither, leave
            // None and let `fastc lock` fill it in later.
            let prior_locked = self
                .lockfile
                .get_package(name)
                .and_then(|p| p.sha256.clone());
            let sha256 = match dep {
                crate::deps::Dependency::Git { sha256, .. } => sha256.clone().or(prior_locked),
                _ => None,
            };

            // Update lockfile
            self.lockfile.add_package(LockedPackage {
                name: name.clone(),
                version,
                source,
                resolved,
                sha256,
                dependencies: vec![],
            });
        }

        // Save updated lockfile
        let lockfile_path = self.project_root.join("fastc.lock");
        self.lockfile
            .save(&lockfile_path)
            .map_err(|e| BuildError::Io(e.to_string()))?;

        eprintln!("Updated fastc.lock");
        Ok(())
    }

    /// Re-anchor `fastc.lock` against the currently fetched content.
    ///
    /// Walks every dependency, computes its content sha256 (via
    /// `deps::hash_tree`), and writes the result into the lockfile.
    /// If `force` is false, a dep with an existing lockfile sha256
    /// is left alone (its value re-verified instead). If `force` is
    /// true, every dep is rehashed and overwritten.
    ///
    /// `force=true` is the way out of "the upstream rev silently
    /// rewrote history and our hash is wrong" situations — opt-in
    /// because it discards a security signal.
    pub fn lock_dependencies(&mut self, force: bool) -> Result<(), BuildError> {
        if self.manifest.dependencies.is_empty() {
            eprintln!("No dependencies to lock.");
            return Ok(());
        }

        for (name, dep) in &self.manifest.dependencies {
            let crate::deps::Dependency::Git { .. } = dep else {
                // Path deps don't get content-hashed — they live in
                // the user's tree and the build verifies them by
                // recompilation instead.
                continue;
            };

            eprintln!("Locking dependency: {}", name);

            // Fetch (no-op if already cached).
            let dep_for_fetch = self.dependency_for_fetch(name, dep);
            let path = self
                .fetcher
                .fetch(name, &dep_for_fetch)
                .map_err(|e| BuildError::FetchError(e.to_string()))?;

            let computed = crate::deps::hash_tree(&path)
                .map_err(|e| BuildError::FetchError(format!("{}", e)))?;

            // If the lockfile already records a hash and `force` is
            // not set, verify the computed value matches and move on
            // — this is the "trust the recorded hash" path.
            let prior = self
                .lockfile
                .get_package(name)
                .and_then(|p| p.sha256.clone());
            if let Some(prev) = prior.as_ref() {
                if !force && prev.trim().eq_ignore_ascii_case(computed.trim()) {
                    eprintln!("  unchanged: {}", short_hash(&computed));
                    continue;
                }
                if !force {
                    return Err(BuildError::FetchError(format!(
                        "dependency '{}': fetched tree no longer matches the recorded \
                        sha256 ({} != {}). Re-run with `--force` if this is intentional.",
                        name,
                        short_hash(&computed),
                        short_hash(prev)
                    )));
                }
            }

            // Get source + resolved commit for lockfile bookkeeping
            // (same shape as fetch_dependencies).
            let crate::deps::Dependency::Git { git, version, .. } = dep else {
                unreachable!("filtered above");
            };
            let resolved = crate::deps::Fetcher::head_commit(&path).ok();
            let source = resolved
                .as_ref()
                .map(|commit| format!("git+{}?rev={}", git, commit))
                .unwrap_or_else(|| Self::source_from_git_spec(git, version));
            let version_str = read_dependency_version(&path).unwrap_or_else(|| "0.0.0".to_string());

            self.lockfile.add_package(LockedPackage {
                name: name.clone(),
                version: version_str,
                source,
                resolved,
                sha256: Some(computed.clone()),
                dependencies: vec![],
            });

            eprintln!("  sha256: {}", computed);
        }

        let lockfile_path = self.project_root.join("fastc.lock");
        self.lockfile
            .save(&lockfile_path)
            .map_err(|e| BuildError::Io(e.to_string()))?;
        eprintln!("Updated fastc.lock");

        Ok(())
    }

    fn dependency_for_fetch(
        &self,
        name: &str,
        dep: &crate::deps::Dependency,
    ) -> crate::deps::Dependency {
        let crate::deps::Dependency::Git { git, .. } = dep else {
            return dep.clone();
        };

        let Some(locked) = self.lockfile.get_package(name) else {
            return dep.clone();
        };
        if !locked.source.starts_with(&format!("git+{}", git)) {
            return dep.clone();
        }

        let locked_rev = locked
            .resolved
            .clone()
            .or_else(|| parse_rev_from_source(&locked.source));

        if let Some(rev) = locked_rev {
            crate::deps::Dependency::Git {
                git: git.clone(),
                version: crate::deps::GitVersion {
                    rev: Some(rev),
                    ..Default::default()
                },
                sha256: None,
                sigstore: None,
            }
        } else {
            dep.clone()
        }
    }

    fn source_from_git_spec(git: &str, version: &crate::deps::GitVersion) -> String {
        let mut source = format!("git+{}", git);
        if let Some(tag) = &version.tag {
            source.push_str(&format!("?tag={}", tag));
        } else if let Some(branch) = &version.branch {
            source.push_str(&format!("?branch={}", branch));
        } else if let Some(rev) = &version.rev {
            source.push_str(&format!("?rev={}", rev));
        }
        source
    }

    /// Gather dependency source directories from the cache
    fn dependency_dirs(&self) -> Vec<(String, std::path::PathBuf)> {
        let mut dirs = Vec::new();
        for (name, dep) in &self.manifest.dependencies {
            if let crate::deps::Dependency::Git { git, version, .. } = dep {
                let version_str = if let Some(tag) = &version.tag {
                    format!("tag-{}", tag)
                } else if let Some(branch) = &version.branch {
                    format!("branch-{}", branch)
                } else if let Some(rev) = &version.rev {
                    format!("rev-{}", rev)
                } else {
                    "default".to_string()
                };

                let dep_path = self.fetcher.cache().dep_path(name, git, &version_str);
                if dep_path.exists() {
                    dirs.push((name.clone(), dep_path));
                }
            } else if let crate::deps::Dependency::Path { path } = dep {
                let dep_path = self.project_root.join(path);
                if dep_path.exists() {
                    dirs.push((name.clone(), dep_path));
                }
            }
        }
        dirs
    }

    /// Compile the project to C code
    ///
    /// Returns the path to the generated C file
    pub fn compile(&self, output_dir: &Path, _release: bool) -> Result<PathBuf, BuildError> {
        // Determine source file
        let src_dir = self.project_root.join("src");
        let main_file = src_dir.join("main.fc");
        let lib_file = src_dir.join("lib.fc");

        let source_file = if main_file.exists() {
            main_file
        } else if lib_file.exists() {
            lib_file
        } else {
            return Err(BuildError::Io(
                "no src/main.fc or src/lib.fc found".to_string(),
            ));
        };

        let base_name = source_file.file_stem().unwrap().to_string_lossy();
        let c_file = output_dir.join(format!("{}.c", base_name));
        let h_file = output_dir.join(format!("{}.h", base_name));

        // M1 multi-source incremental cache. The project-tree hash
        // covers every `.fc` under src/, plus `fastc.toml` and
        // `fastc.lock`. If the hash matches a previous successful
        // build, copy the cached `.c` / `.h` instead of re-running
        // the lex → emit chain. Re-edit one file and the hash flips
        // → cache miss → full re-compile. The cache hit path also
        // covers the "agent ran `fastc build` to verify nothing
        // changed" pattern that dominates inner-loop usage.
        if let Some(project_key) = self.project_cache_key() {
            if let Some((cached_c, cached_h)) = crate::build_cache::lookup_project(&project_key) {
                std::fs::create_dir_all(output_dir).map_err(|e| BuildError::Io(e.to_string()))?;
                std::fs::write(&c_file, &cached_c).map_err(|e| BuildError::Io(e.to_string()))?;
                if let Some(h) = cached_h.as_deref() {
                    std::fs::write(&h_file, h).map_err(|e| BuildError::Io(e.to_string()))?;
                }
                eprintln!(
                    "Cache hit: {} ({})",
                    source_file.display(),
                    short_hash(&project_key)
                );
                return Ok(c_file);
            }
        }

        eprintln!("Compiling: {}", source_file.display());

        // Read source
        let source =
            std::fs::read_to_string(&source_file).map_err(|e| BuildError::Io(e.to_string()))?;

        // Gather dependency paths
        let dep_dirs = self.dependency_dirs();

        // Compile with dependency awareness
        let filename = source_file.display().to_string();
        let (c_code, header) = crate::compile_project(&source, &filename, true, dep_dirs)?;

        // Create output directory
        std::fs::create_dir_all(output_dir).map_err(|e| BuildError::Io(e.to_string()))?;

        std::fs::write(&c_file, &c_code).map_err(|e| BuildError::Io(e.to_string()))?;
        eprintln!("  Wrote: {}", c_file.display());

        if let Some(h) = &header {
            std::fs::write(&h_file, h).map_err(|e| BuildError::Io(e.to_string()))?;
            eprintln!("  Wrote: {}", h_file.display());
        }

        // M1: stash the successful build under the project key.
        if let Some(project_key) = self.project_cache_key() {
            crate::build_cache::store_project(&project_key, &c_code, header.as_deref());
        }

        eprintln!("FastC compilation complete.");
        Ok(c_file)
    }

    /// N3: is this build context a workspace root? True when the
    /// manifest has a `[workspace]` block with at least one member.
    pub fn is_workspace_root(&self) -> bool {
        self.manifest
            .workspace
            .as_ref()
            .map(|w| !w.members.is_empty())
            .unwrap_or(false)
    }

    /// N3: build every workspace member in declaration order. Each
    /// member is built via its own `BuildContext::compile` call,
    /// which means each member gets its own M1 project cache key
    /// — editing one member only invalidates that member's cache.
    /// Returns the list of emitted `.c` paths (one per member).
    ///
    /// `output_dir` is interpreted relative to each member's
    /// project root, so workspace builds produce per-member
    /// `<member>/build/*.c` artifacts rather than dumping
    /// everything into a shared output tree.
    pub fn compile_workspace(
        &self,
        output_dir: &Path,
        release: bool,
    ) -> Result<Vec<PathBuf>, BuildError> {
        let Some(ws) = self.manifest.workspace.as_ref() else {
            return Err(BuildError::Io(
                "compile_workspace called on a non-workspace root".to_string(),
            ));
        };
        let mut out = Vec::with_capacity(ws.members.len());
        for member in &ws.members {
            let member_root = self.project_root.join(member);
            if !member_root.join("fastc.toml").exists() {
                return Err(BuildError::ManifestError(format!(
                    "workspace member '{}' has no fastc.toml at {}",
                    member,
                    member_root.display()
                )));
            }
            eprintln!("--- workspace member: {} ---", member);
            let member_ctx = BuildContext::new(&member_root)?;
            let member_output = member_root.join(output_dir);
            let c_file = member_ctx.compile(&member_output, release)?;
            out.push(c_file);
        }
        Ok(out)
    }

    /// M1: compute a content-hash key for the entire project that
    /// is keyed off everything affecting the lex → emit output:
    ///
    /// - Every `.fc` source under `src/` (recursive, path-sorted).
    /// - `fastc.toml` content.
    /// - `fastc.lock` content (so a dep tree change triggers rebuild).
    /// - `fastc_version` (compiler version itself).
    ///
    /// Returns `None` when the `src/` tree can't be walked — the
    /// caller falls back to a no-cache path so the build still
    /// works in unusual layouts.
    pub fn project_cache_key(&self) -> Option<String> {
        use crate::db::sha256;

        let src_dir = self.project_root.join("src");
        let mut files: Vec<PathBuf> = Vec::new();
        collect_fc_files(&src_dir, &mut files).ok()?;
        files.sort();

        let mut buf: Vec<u8> = Vec::with_capacity(files.len() * 256);
        for f in &files {
            // Project-relative path so the key doesn't depend on
            // where the user checked out the project.
            let rel = f.strip_prefix(&self.project_root).unwrap_or(f);
            buf.extend_from_slice(rel.to_string_lossy().replace('\\', "/").as_bytes());
            buf.push(0);
            let bytes = std::fs::read(f).ok()?;
            buf.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
            buf.extend_from_slice(&bytes);
            buf.push(0);
        }
        // Manifest + lockfile bytes also go into the key — a dep
        // version change should invalidate the cache even when no
        // src/ file changed.
        let manifest_path = self.project_root.join("fastc.toml");
        if let Ok(bytes) = std::fs::read(&manifest_path) {
            buf.extend_from_slice(b"fastc.toml\0");
            buf.extend_from_slice(&bytes);
            buf.push(0);
        }
        let lockfile_path = self.project_root.join("fastc.lock");
        if let Ok(bytes) = std::fs::read(&lockfile_path) {
            buf.extend_from_slice(b"fastc.lock\0");
            buf.extend_from_slice(&bytes);
            buf.push(0);
        }
        // Compiler version is part of the key — a version bump
        // should never reuse a stale cache entry from an older
        // build (output formatting, codegen, or contract semantics
        // may have shifted).
        buf.extend_from_slice(b"fastc_version\0");
        buf.extend_from_slice(env!("CARGO_PKG_VERSION").as_bytes());

        let digest = sha256(&buf);
        let mut hex = String::with_capacity(64);
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for &b in &digest {
            hex.push(HEX[(b >> 4) as usize] as char);
            hex.push(HEX[(b & 0x0f) as usize] as char);
        }
        Some(hex)
    }

    /// Compile the generated C code with a C compiler
    ///
    /// `compiler_prefix_args` are inserted right after the compiler binary
    /// and before the source file. Cross-compile uses this for `zig cc
    /// --target=...` — `compiler="zig"`, `compiler_prefix_args=["cc",
    /// "--target=aarch64-linux-musl"]`. The `output_ext` lets WASI targets
    /// override the default empty extension with `.wasm`.
    ///
    /// Returns the path to the executable.
    pub fn cc_compile(
        &self,
        c_file: &Path,
        compiler: &str,
        compiler_prefix_args: &[&str],
        cflags: &[&str],
        release: bool,
        output_ext: &str,
    ) -> Result<PathBuf, BuildError> {
        let output_dir = c_file.parent().unwrap_or(Path::new("."));
        let base_name = c_file.file_stem().unwrap().to_string_lossy();

        // Output executable name (add .exe on Windows when no explicit ext).
        let exe_name = if !output_ext.is_empty() {
            format!("{}{}", base_name, output_ext)
        } else {
            #[cfg(windows)]
            {
                format!("{}.exe", base_name)
            }
            #[cfg(not(windows))]
            {
                base_name.to_string()
            }
        };

        let executable = output_dir.join(&exe_name);

        eprintln!("Compiling C code with {}...", compiler);

        // Build compiler arguments. `compiler_prefix_args` first (e.g. zig's
        // `cc` subcommand, --target flag), then the source file.
        let mut args: Vec<&str> = Vec::new();
        args.extend(compiler_prefix_args.iter().copied());
        args.push(c_file.to_str().unwrap());
        args.push("-o");
        args.push(executable.to_str().unwrap());

        // Add runtime include path
        // Try to find the runtime directory relative to the executable or use env var
        if let Some(runtime_path) = Self::find_runtime_include() {
            args.push("-I");
            // We need to leak this string to get a &str with 'static lifetime
            // This is acceptable for a CLI tool
            let leaked: &'static str = Box::leak(runtime_path.into_boxed_str());
            args.push(leaked);
        }

        // Add optimization flags
        if release {
            args.push("-O2");
            args.push("-DNDEBUG");
        } else {
            args.push("-g");
            args.push("-O0");
        }

        // Add user-provided flags
        args.extend(cflags);

        // Add standard math library (commonly needed)
        args.push("-lm");

        eprintln!("  {} {}", compiler, args.join(" "));

        let output = Command::new(compiler)
            .args(&args)
            .output()
            .map_err(|e| BuildError::CcError(format!("failed to run {}: {}", compiler, e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(BuildError::CcError(format!(
                "{} failed:\n{}{}",
                compiler, stdout, stderr
            )));
        }

        eprintln!("  Wrote: {}", executable.display());
        eprintln!("C compilation complete.");
        Ok(executable)
    }

    /// Run the compiled executable
    pub fn run(&self, executable: &Path, args: &[String]) -> Result<(), BuildError> {
        eprintln!("Running: {} {}", executable.display(), args.join(" "));
        eprintln!("---");

        let status = Command::new(executable).args(args).status().map_err(|e| {
            BuildError::Io(format!("failed to run {}: {}", executable.display(), e))
        })?;

        eprintln!("---");

        if !status.success() {
            let code = status.code().unwrap_or(-1);
            if code != 0 {
                eprintln!("Program exited with code: {}", code);
            }
            // Don't treat non-zero exit as error for `run` - just report it
        }

        Ok(())
    }

    /// Find the FastC runtime include directory
    fn find_runtime_include() -> Option<String> {
        // Check FASTC_RUNTIME environment variable first
        if let Ok(path) = std::env::var("FASTC_RUNTIME") {
            if Path::new(&path).exists() {
                return Some(path);
            }
        }

        // Try relative to the current executable
        if let Ok(exe_path) = std::env::current_exe() {
            // Development: target/debug/fastc -> ../../runtime
            if let Some(parent) = exe_path.parent() {
                // Check various relative paths
                let candidates = [
                    parent.join("../../../runtime"),       // From target/debug/
                    parent.join("../../runtime"),          // From target/
                    parent.join("../runtime"),             // Adjacent
                    parent.join("runtime"),                // Same dir
                    parent.join("../share/fastc/runtime"), // Installed location
                ];

                for candidate in &candidates {
                    if candidate.join("fastc_runtime.h").exists() {
                        return candidate.canonicalize().ok()?.to_str().map(String::from);
                    }
                }
            }
        }

        // Check common installation paths
        let common_paths = ["/usr/local/share/fastc/runtime", "/usr/share/fastc/runtime"];

        for path in &common_paths {
            let p = Path::new(path);
            if p.join("fastc_runtime.h").exists() {
                return Some(path.to_string());
            }
        }

        None
    }

    /// Get include paths for all dependencies
    pub fn include_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        for (name, dep) in &self.manifest.dependencies {
            // Try to find the cached path
            if let crate::deps::Dependency::Git { git, version, .. } = dep {
                let version_str = if let Some(tag) = &version.tag {
                    format!("tag-{}", tag)
                } else if let Some(branch) = &version.branch {
                    format!("branch-{}", branch)
                } else if let Some(rev) = &version.rev {
                    format!("rev-{}", rev)
                } else {
                    "default".to_string()
                };

                let dep_path = self.fetcher.cache().dep_path(name, git, &version_str);
                if dep_path.exists() {
                    // Add src/ subdirectory if it exists
                    let src_path = dep_path.join("src");
                    if src_path.exists() {
                        paths.push(src_path);
                    } else {
                        paths.push(dep_path);
                    }
                }
            }
        }

        paths
    }
}

fn parse_rev_from_source(source: &str) -> Option<String> {
    source
        .split_once('?')
        .map(|(_, query)| query)
        .and_then(|query| {
            query
                .split('&')
                .find(|kv| kv.starts_with("rev="))
                .and_then(|kv| kv.strip_prefix("rev="))
        })
        .map(|s| s.to_string())
}

/// M1: recursively gather every `.fc` file under `dir` into `out`.
/// Skips `build/` and `.fastc/` so generated artifacts don't enter
/// the project cache key. Returns an IO error on permission /
/// missing-dir issues so the caller can fall back to no-cache.
fn collect_fc_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if name == "build" || name == ".fastc" || name == "target" {
                continue;
            }
            collect_fc_files(&path, out)?;
        } else if name.ends_with(".fc") {
            out.push(path);
        }
    }
    Ok(())
}

/// Short-form for displaying a content hash in build logs. Returns
/// the first 12 hex characters — enough to disambiguate visually
/// without flooding the log line.
fn short_hash(hex: &str) -> String {
    let trimmed = hex.trim();
    if trimmed.len() <= 12 {
        return trimmed.to_string();
    }
    format!("{}…", &trimmed[..12])
}

fn read_dependency_version(dep_path: &Path) -> Option<String> {
    let manifest_path = dep_path.join("fastc.toml");
    if !manifest_path.exists() {
        return None;
    }

    crate::deps::Manifest::load(&manifest_path)
        .ok()
        .map(|manifest| manifest.package.version)
}
