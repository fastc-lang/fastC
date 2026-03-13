//! Git-based dependency fetching
//!
//! Fetches dependencies from Git repositories

use git2::{FetchOptions, RemoteCallbacks, Repository};
use std::path::Path;

use super::cache::Cache;
use super::manifest::{Dependency, GitVersion};

/// Fetches dependencies from Git repositories
pub struct Fetcher {
    cache: Cache,
}

impl Fetcher {
    /// Create a new fetcher with the default cache
    pub fn new() -> Option<Self> {
        Some(Self {
            cache: Cache::new()?,
        })
    }

    /// Create a fetcher with a custom cache
    pub fn with_cache(cache: Cache) -> Self {
        Self { cache }
    }

    /// Fetch a dependency if not already cached
    ///
    /// Returns the path to the fetched dependency
    pub fn fetch(
        &self,
        name: &str,
        dep: &Dependency,
    ) -> Result<std::path::PathBuf, FetchError> {
        match dep {
            Dependency::Git { git, version } => self.fetch_git(name, git, version),
            Dependency::Path { path } => {
                // Local paths don't need fetching
                let path = std::path::PathBuf::from(path);
                if !path.exists() {
                    return Err(FetchError::PathNotFound(path));
                }
                Ok(path)
            }
        }
    }

    /// Fetch a Git dependency
    fn fetch_git(
        &self,
        name: &str,
        url: &str,
        version: &GitVersion,
    ) -> Result<std::path::PathBuf, FetchError> {
        let version_str = self.version_string(version);
        let dest = self.cache.dep_path(name, url, &version_str);

        // Check if already cached
        if self.cache.is_cached(name, url, &version_str) {
            return Ok(dest);
        }

        // Ensure parent directory exists
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| FetchError::Io(e.to_string()))?;
        }

        // Clone the repository
        self.clone_repo(url, &dest, version)?;

        Ok(dest)
    }

    /// Clone a Git repository
    fn clone_repo(
        &self,
        url: &str,
        dest: &Path,
        version: &GitVersion,
    ) -> Result<(), FetchError> {
        // Set up callbacks for progress (could be extended for authentication)
        let mut callbacks = RemoteCallbacks::new();
        callbacks.transfer_progress(|_stats| {
            // Could report progress here
            true
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        // Clone the repository
        let repo = Repository::clone(url, dest).map_err(|e| FetchError::Git(e.to_string()))?;

        // Checkout the specific version
        self.checkout_version(&repo, version)?;

        Ok(())
    }

    /// Checkout a specific version (tag, branch, or rev)
    fn checkout_version(&self, repo: &Repository, version: &GitVersion) -> Result<(), FetchError> {
        let refspec = if let Some(tag) = &version.tag {
            // For tags, try refs/tags/TAG first
            format!("refs/tags/{}", tag)
        } else if let Some(branch) = &version.branch {
            format!("refs/remotes/origin/{}", branch)
        } else if let Some(rev) = &version.rev {
            rev.clone()
        } else {
            // Default to HEAD (already at default branch after clone)
            return Ok(());
        };

        // Find the reference
        let reference = if version.rev.is_some() {
            // For revisions, look up the commit directly
            let oid = git2::Oid::from_str(&refspec)
                .map_err(|e| FetchError::Git(format!("invalid revision: {}", e)))?;
            repo.find_commit(oid)
                .map_err(|e| FetchError::Git(format!("commit not found: {}", e)))?;
            repo.set_head_detached(oid)
                .map_err(|e| FetchError::Git(format!("failed to checkout: {}", e)))?;
            return Ok(());
        } else {
            repo.find_reference(&refspec)
                .map_err(|e| FetchError::Git(format!("reference '{}' not found: {}", refspec, e)))?
        };

        // Checkout the reference
        let commit = reference
            .peel_to_commit()
            .map_err(|e| FetchError::Git(format!("failed to resolve commit: {}", e)))?;

        repo.checkout_tree(commit.as_object(), None)
            .map_err(|e| FetchError::Git(format!("failed to checkout: {}", e)))?;

        repo.set_head(reference.name().unwrap_or(&refspec))
            .map_err(|e| FetchError::Git(format!("failed to set HEAD: {}", e)))?;

        Ok(())
    }

    /// Convert version specifier to a string for caching
    fn version_string(&self, version: &GitVersion) -> String {
        if let Some(tag) = &version.tag {
            format!("tag-{}", tag)
        } else if let Some(branch) = &version.branch {
            format!("branch-{}", branch)
        } else if let Some(rev) = &version.rev {
            format!("rev-{}", rev)
        } else {
            "default".to_string()
        }
    }

    /// Get the cache being used
    pub fn cache(&self) -> &Cache {
        &self.cache
    }
}

/// Errors that can occur during fetching
#[derive(Debug)]
pub enum FetchError {
    /// Git operation failed
    Git(String),
    /// IO error
    Io(String),
    /// Path dependency not found
    PathNotFound(std::path::PathBuf),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Git(msg) => write!(f, "git error: {}", msg),
            FetchError::Io(msg) => write!(f, "io error: {}", msg),
            FetchError::PathNotFound(path) => {
                write!(f, "path dependency not found: {}", path.display())
            }
        }
    }
}

impl std::error::Error for FetchError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_string() {
        let fetcher = Fetcher::with_cache(Cache::with_dir(std::path::PathBuf::from("/tmp")));

        assert_eq!(
            fetcher.version_string(&GitVersion {
                tag: Some("v1.0.0".to_string()),
                ..Default::default()
            }),
            "tag-v1.0.0"
        );

        assert_eq!(
            fetcher.version_string(&GitVersion {
                branch: Some("main".to_string()),
                ..Default::default()
            }),
            "branch-main"
        );

        assert_eq!(
            fetcher.version_string(&GitVersion::default()),
            "default"
        );
    }
}
