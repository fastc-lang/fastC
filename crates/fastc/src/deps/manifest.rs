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
    /// Git dependency with optional version specifier
    Git {
        git: String,
        #[serde(flatten)]
        version: GitVersion,
    },
    /// Local path dependency
    Path { path: String },
}

/// Git version specifier
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitVersion {
    pub tag: Option<String>,
    pub branch: Option<String>,
    pub rev: Option<String>,
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
}
