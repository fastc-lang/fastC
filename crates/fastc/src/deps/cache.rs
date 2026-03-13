//! Dependency cache management
//!
//! Manages the local cache of downloaded dependencies at ~/.fastc/cache

use std::path::{Path, PathBuf};

/// Dependency cache manager
pub struct Cache {
    /// Cache directory (typically ~/.fastc/cache)
    cache_dir: PathBuf,
}

impl Cache {
    /// Create a new cache manager using the default cache directory
    pub fn new() -> Option<Self> {
        let cache_dir = dirs::cache_dir()?.join("fastc").join("deps");
        Some(Self { cache_dir })
    }

    /// Create a cache manager with a custom cache directory
    pub fn with_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Get the cache directory
    pub fn dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Ensure the cache directory exists
    pub fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.cache_dir)
    }

    /// Get the path where a dependency should be cached
    ///
    /// Dependencies are stored at: cache_dir/<name>/<hash>/
    /// where hash is derived from the Git URL and version specifier
    pub fn dep_path(&self, name: &str, url: &str, version: &str) -> PathBuf {
        let hash = Self::hash_dep(url, version);
        self.cache_dir.join(name).join(hash)
    }

    /// Check if a dependency is already cached
    pub fn is_cached(&self, name: &str, url: &str, version: &str) -> bool {
        let path = self.dep_path(name, url, version);
        path.exists() && path.join("fastc.toml").exists()
    }

    /// Generate a hash for a dependency based on URL and version
    fn hash_dep(url: &str, version: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        url.hash(&mut hasher);
        version.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// List all cached dependencies
    pub fn list_cached(&self) -> std::io::Result<Vec<CachedDep>> {
        let mut deps = Vec::new();

        if !self.cache_dir.exists() {
            return Ok(deps);
        }

        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                for version_entry in std::fs::read_dir(entry.path())? {
                    let version_entry = version_entry?;
                    if version_entry.file_type()?.is_dir() {
                        deps.push(CachedDep {
                            name: name.clone(),
                            path: version_entry.path(),
                        });
                    }
                }
            }
        }

        Ok(deps)
    }

    /// Clean the entire cache
    pub fn clean(&self) -> std::io::Result<()> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new().expect("could not determine cache directory")
    }
}

/// Information about a cached dependency
#[derive(Debug, Clone)]
pub struct CachedDep {
    /// Dependency name
    pub name: String,
    /// Path to the cached dependency
    pub path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_dep_path() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_dir(temp.path().to_path_buf());

        let path = cache.dep_path("mylib", "https://github.com/user/mylib", "v1.0.0");
        assert!(path.starts_with(temp.path()));
        assert!(path.to_string_lossy().contains("mylib"));
    }

    #[test]
    fn test_hash_consistency() {
        // Same inputs should produce same hash
        let hash1 = Cache::hash_dep("https://example.com/repo", "v1.0");
        let hash2 = Cache::hash_dep("https://example.com/repo", "v1.0");
        assert_eq!(hash1, hash2);

        // Different inputs should produce different hashes
        let hash3 = Cache::hash_dep("https://example.com/repo", "v2.0");
        assert_ne!(hash1, hash3);
    }
}
