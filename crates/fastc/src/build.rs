//! Build orchestration for FastC projects
//!
//! Handles dependency fetching, compilation, and output generation.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::deps::{Fetcher, LockedPackage, Lockfile, Manifest};
use crate::diag::CompileError;

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

            // Get source + resolved commit for lockfile
            let (source, resolved) = match dep {
                crate::deps::Dependency::Git { git, version } => {
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

            // Update lockfile
            self.lockfile.add_package(LockedPackage {
                name: name.clone(),
                version,
                source,
                resolved,
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

        eprintln!("Compiling: {}", source_file.display());

        // Read source
        let source =
            std::fs::read_to_string(&source_file).map_err(|e| BuildError::Io(e.to_string()))?;

        // Compile
        let filename = source_file.display().to_string();
        let (c_code, header) = crate::compile_with_options(&source, &filename, true)?;

        // Create output directory
        std::fs::create_dir_all(output_dir).map_err(|e| BuildError::Io(e.to_string()))?;

        // Write output files
        let base_name = source_file.file_stem().unwrap().to_string_lossy();
        let c_file = output_dir.join(format!("{}.c", base_name));
        let h_file = output_dir.join(format!("{}.h", base_name));

        std::fs::write(&c_file, &c_code).map_err(|e| BuildError::Io(e.to_string()))?;
        eprintln!("  Wrote: {}", c_file.display());

        if let Some(h) = header {
            std::fs::write(&h_file, &h).map_err(|e| BuildError::Io(e.to_string()))?;
            eprintln!("  Wrote: {}", h_file.display());
        }

        eprintln!("FastC compilation complete.");
        Ok(c_file)
    }

    /// Compile the generated C code with a C compiler
    ///
    /// Returns the path to the executable
    pub fn cc_compile(
        &self,
        c_file: &Path,
        compiler: &str,
        cflags: &[&str],
        release: bool,
    ) -> Result<PathBuf, BuildError> {
        let output_dir = c_file.parent().unwrap_or(Path::new("."));
        let base_name = c_file.file_stem().unwrap().to_string_lossy();

        // Output executable name (add .exe on Windows)
        #[cfg(windows)]
        let exe_name = format!("{}.exe", base_name);
        #[cfg(not(windows))]
        let exe_name = base_name.to_string();

        let executable = output_dir.join(&exe_name);

        eprintln!("Compiling C code with {}...", compiler);

        // Build compiler arguments
        let mut args: Vec<&str> =
            vec![c_file.to_str().unwrap(), "-o", executable.to_str().unwrap()];

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
            if let crate::deps::Dependency::Git { git, version } = dep {
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

fn read_dependency_version(dep_path: &Path) -> Option<String> {
    let manifest_path = dep_path.join("fastc.toml");
    if !manifest_path.exists() {
        return None;
    }

    crate::deps::Manifest::load(&manifest_path)
        .ok()
        .map(|manifest| manifest.package.version)
}
