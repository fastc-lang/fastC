//! Dependency and module resolution for FastC
//!
//! This module handles:
//! - Parsing fastc.toml manifest files
//! - Resolving module paths (mod declarations)
//! - Loading and expanding module files
//! - Fetching Git-based dependencies
//! - Managing the dependency cache
//! - Lock file management for reproducible builds

mod cache;
mod fetcher;
mod loader;
mod lockfile;
mod manifest;
mod resolver;

pub use cache::Cache;
pub use fetcher::{FetchError, Fetcher};
pub use loader::{LoaderError, ModuleLoader};
pub use lockfile::{LockedPackage, Lockfile, LockfileError};
pub use manifest::{Dependency, GitVersion, Manifest, ManifestError, Package};
pub use resolver::{ModuleError, ModuleResolver, ResolvedModule};
