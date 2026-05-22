//! FastC manifest (fastc.toml) parsing

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// A FastC project manifest (fastc.toml)
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub dependencies: HashMap<String, Dependency>,
}

/// Package metadata
#[derive(Debug, Clone, Deserialize)]
pub struct Package {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(rename = "type", default)]
    pub project_type: ProjectType,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

/// Project type
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectType {
    #[default]
    Binary,
    Library,
    FfiWrapper,
}

/// Build configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct BuildConfig {
    #[serde(default)]
    pub include_dirs: Vec<String>,
    #[serde(default)]
    pub link_libs: Vec<String>,
}

/// A dependency specification
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    /// Git dependency with optional version specifier and integrity fields.
    Git {
        git: String,
        #[serde(flatten)]
        version: GitVersion,
        /// SHA-256 of the dependency's resolved git tree (`git
        /// archive` of the locked rev). Optional in the manifest;
        /// the lockfile (`fastc.lock`) records it once resolved.
        /// `fastc build --vendor-strict` refuses to fetch anything
        /// without a recorded sha256.
        #[serde(default)]
        sha256: Option<String>,
        /// Sigstore bundle (`.sigstore.json`) attesting the rev.
        /// Required for every `fastc-core` dependency. Optional
        /// for third-party deps until a stage-2.x sub-slice tightens
        /// the policy. Bundle verification uses the public Sigstore
        /// transparency log via `cosign verify-bundle`.
        #[serde(default)]
        sigstore: Option<String>,
    },
    /// Local path dependency. Skipped by the integrity checker
    /// because there's no upstream to attest.
    Path { path: String },
}

/// Git version specifier
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitVersion {
    pub tag: Option<String>,
    pub branch: Option<String>,
    pub rev: Option<String>,
}

impl Dependency {
    /// Apply the vendor-first integrity policy to this dependency.
    /// Returns a list of human-readable warnings. The strict mode
    /// (`fastc build --vendor-strict`) converts these to hard
    /// errors; the default `fastc build` reports them but proceeds.
    pub fn integrity_warnings(&self, name: &str) -> Vec<String> {
        let mut out = Vec::new();
        if let Dependency::Git {
            version,
            sha256,
            sigstore,
            ..
        } = self
        {
            // Must specify a rev — tags and branches can move and
            // are unsafe for supply-chain integrity.
            if version.rev.is_none() {
                out.push(format!(
                    "dependency '{}': missing commit `rev` — tags and branches can move; pin to a 40-char commit hash",
                    name
                ));
            }
            if sha256.is_none() {
                out.push(format!(
                    "dependency '{}': missing `sha256` content hash — record one with `fastc lock` after a successful fetch",
                    name
                ));
            }
            // Sigstore is required for fastc-core deps. We can't tell
            // by URL alone whether a dep is fastc-core, so just warn
            // when missing — strict mode tightens to "required for
            // anything under github.com/fastc-core/".
            if sigstore.is_none() {
                out.push(format!(
                    "dependency '{}': missing `sigstore` bundle — every `fastc-core` package will be required to ship one once stage-1.8 lands",
                    name
                ));
            }
        }
        out
    }
}

impl Manifest {
    /// Load a manifest from a file
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path).map_err(|e| ManifestError::Io {
            path: path.to_path_buf(),
            error: e.to_string(),
        })?;

        toml::from_str(&content).map_err(|e| ManifestError::Parse {
            path: path.to_path_buf(),
            error: e.to_string(),
        })
    }

    /// Find the manifest file in the current directory or ancestors
    pub fn find(start: &Path) -> Option<std::path::PathBuf> {
        let mut current = start;
        loop {
            let manifest_path = current.join("fastc.toml");
            if manifest_path.exists() {
                return Some(manifest_path);
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => return None,
            }
        }
    }
}

/// Errors that can occur when loading a manifest
#[derive(Debug)]
pub enum ManifestError {
    Io {
        path: std::path::PathBuf,
        error: String,
    },
    Parse {
        path: std::path::PathBuf,
        error: String,
    },
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::Io { path, error } => {
                write!(f, "failed to read {}: {}", path.display(), error)
            }
            ManifestError::Parse { path, error } => {
                write!(f, "failed to parse {}: {}", path.display(), error)
            }
        }
    }
}

impl std::error::Error for ManifestError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_manifest() {
        let toml = r#"
[package]
name = "test_project"
"#;
        let manifest: Manifest = toml::from_str(toml).unwrap();
        assert_eq!(manifest.package.name, "test_project");
        assert_eq!(manifest.package.version, "0.1.0");
        assert_eq!(manifest.package.project_type, ProjectType::Binary);
    }

    #[test]
    fn test_parse_full_manifest() {
        let toml = r#"
[package]
name = "my_lib"
version = "1.2.3"
type = "library"

[build]
include_dirs = ["include", "vendor"]
link_libs = ["nng", "pthread"]

[dependencies]
mylib = { git = "https://github.com/user/mylib", tag = "v1.0.0" }
utils = { git = "https://github.com/user/utils", branch = "main" }
local = { path = "../local_lib" }
"#;
        let manifest: Manifest = toml::from_str(toml).unwrap();
        assert_eq!(manifest.package.name, "my_lib");
        assert_eq!(manifest.package.version, "1.2.3");
        assert_eq!(manifest.package.project_type, ProjectType::Library);
        assert_eq!(manifest.build.include_dirs, vec!["include", "vendor"]);
        assert_eq!(manifest.build.link_libs, vec!["nng", "pthread"]);
        assert_eq!(manifest.dependencies.len(), 3);
    }

    #[test]
    fn test_integrity_warnings_flag_missing_pin() {
        // A dep using a moving tag, no sha256, no sigstore — the
        // worst-case supply-chain shape. Should produce three
        // warnings.
        let toml = r#"
[package]
name = "p"

[dependencies]
risky = { git = "https://github.com/x/y", tag = "v1.0.0" }
"#;
        let manifest: Manifest = toml::from_str(toml).unwrap();
        let warns = manifest.dependencies["risky"].integrity_warnings("risky");
        assert_eq!(warns.len(), 3);
        assert!(warns.iter().any(|w| w.contains("rev")));
        assert!(warns.iter().any(|w| w.contains("sha256")));
        assert!(warns.iter().any(|w| w.contains("sigstore")));
    }

    #[test]
    fn test_integrity_warnings_clean_when_fully_pinned() {
        let toml = r#"
[package]
name = "p"

[dependencies]
safe = { git = "https://github.com/fastc-core/json", rev = "abc1234567890abc1234567890abc1234567890a", sha256 = "0000000000000000000000000000000000000000000000000000000000000000", sigstore = "vendor/json.sigstore.json" }
"#;
        let manifest: Manifest = toml::from_str(toml).unwrap();
        let warns = manifest.dependencies["safe"].integrity_warnings("safe");
        assert!(warns.is_empty(), "expected no warnings, got: {:?}", warns);
    }

    #[test]
    fn test_path_dep_has_no_integrity_warnings() {
        // Local path deps are exempt — there's no upstream to
        // attest to.
        let toml = r#"
[package]
name = "p"

[dependencies]
local = { path = "../utils" }
"#;
        let manifest: Manifest = toml::from_str(toml).unwrap();
        let warns = manifest.dependencies["local"].integrity_warnings("local");
        assert!(warns.is_empty());
    }
}
