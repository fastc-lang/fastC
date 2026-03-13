//! Module resolution for FastC
//!
//! Handles resolving `mod name;` declarations to their source files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Resolved module information
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    /// The module name
    pub name: String,
    /// Path to the source file
    pub path: PathBuf,
    /// Module content (loaded on demand)
    pub content: Option<String>,
}

/// Module resolver for finding source files
pub struct ModuleResolver {
    /// Root directory of the project (where fastc.toml is)
    root: PathBuf,
    /// Source directory (typically "src")
    src_dir: PathBuf,
    /// Cache of resolved modules
    cache: HashMap<String, ResolvedModule>,
}

impl ModuleResolver {
    /// Create a new module resolver
    pub fn new(root: PathBuf) -> Self {
        let src_dir = root.join("src");
        Self {
            root,
            src_dir,
            cache: HashMap::new(),
        }
    }

    /// Create a resolver from a source file path
    pub fn from_source_file(source_path: &Path) -> Option<Self> {
        // Try to find the project root by looking for fastc.toml
        let mut current = source_path.parent()?;
        loop {
            if current.join("fastc.toml").exists() {
                return Some(Self::new(current.to_path_buf()));
            }
            // Also check if we're in a src directory
            if current.file_name().map(|n| n == "src").unwrap_or(false) {
                if let Some(parent) = current.parent() {
                    if parent.join("fastc.toml").exists() {
                        return Some(Self::new(parent.to_path_buf()));
                    }
                }
            }
            current = current.parent()?;
        }
    }

    /// Resolve a module name to its source file
    ///
    /// Given `mod utils;`, this will look for:
    /// 1. src/utils.fc
    /// 2. src/utils/mod.fc (for submodules)
    pub fn resolve(&mut self, module_name: &str) -> Result<&ResolvedModule, ModuleError> {
        // Check cache first
        if self.cache.contains_key(module_name) {
            return Ok(self.cache.get(module_name).unwrap());
        }

        // Try to find the module file
        let module_file = self.find_module_file(module_name)?;

        let resolved = ResolvedModule {
            name: module_name.to_string(),
            path: module_file,
            content: None,
        };

        self.cache.insert(module_name.to_string(), resolved);
        Ok(self.cache.get(module_name).unwrap())
    }

    /// Resolve a module path (e.g., "mylib::utils::Vector")
    pub fn resolve_path(&mut self, path: &[String]) -> Result<&ResolvedModule, ModuleError> {
        if path.is_empty() {
            return Err(ModuleError::EmptyPath);
        }

        // For now, just resolve the first element (the root module)
        // More complex path resolution would be needed for nested modules
        self.resolve(&path[0])
    }

    /// Find the source file for a module
    pub fn find_module_file(&self, module_name: &str) -> Result<PathBuf, ModuleError> {
        // Try src/module_name.fc first
        let direct_path = self.src_dir.join(format!("{}.fc", module_name));
        if direct_path.exists() {
            return Ok(direct_path);
        }

        // Try src/module_name/mod.fc for directory modules
        let dir_path = self.src_dir.join(module_name).join("mod.fc");
        if dir_path.exists() {
            return Ok(dir_path);
        }

        Err(ModuleError::NotFound {
            name: module_name.to_string(),
            searched: vec![direct_path, dir_path],
        })
    }

    /// Load the content of a resolved module
    pub fn load_content(&mut self, module_name: &str) -> Result<String, ModuleError> {
        // Ensure module is resolved
        if !self.cache.contains_key(module_name) {
            self.resolve(module_name)?;
        }

        let module = self.cache.get_mut(module_name).unwrap();

        // Load content if not already loaded
        if module.content.is_none() {
            let content = std::fs::read_to_string(&module.path).map_err(|e| ModuleError::Io {
                path: module.path.clone(),
                error: e.to_string(),
            })?;
            module.content = Some(content);
        }

        Ok(module.content.clone().unwrap())
    }

    /// Get the project root directory
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the source directory
    pub fn src_dir(&self) -> &Path {
        &self.src_dir
    }
}

/// Errors that can occur during module resolution
#[derive(Debug)]
pub enum ModuleError {
    /// Module not found
    NotFound {
        name: String,
        searched: Vec<PathBuf>,
    },
    /// Empty module path
    EmptyPath,
    /// IO error reading module file
    Io { path: PathBuf, error: String },
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleError::NotFound { name, searched } => {
                write!(f, "module '{}' not found, searched: ", name)?;
                for (i, path) in searched.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", path.display())?;
                }
                Ok(())
            }
            ModuleError::EmptyPath => write!(f, "empty module path"),
            ModuleError::Io { path, error } => {
                write!(f, "failed to read {}: {}", path.display(), error)
            }
        }
    }
}

impl std::error::Error for ModuleError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_project() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create project structure
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("fastc.toml"),
            r#"[package]
name = "test_project"
"#,
        )
        .unwrap();

        // Create some module files
        fs::write(
            dir.path().join("src/main.fc"),
            "fn main() -> i32 { return 0; }",
        )
        .unwrap();
        fs::write(
            dir.path().join("src/utils.fc"),
            "fn helper() -> i32 { return 42; }",
        )
        .unwrap();

        // Create a directory module
        fs::create_dir_all(dir.path().join("src/math")).unwrap();
        fs::write(
            dir.path().join("src/math/mod.fc"),
            "fn add(a: i32, b: i32) -> i32 { return (a + b); }",
        )
        .unwrap();

        dir
    }

    #[test]
    fn test_resolve_direct_module() {
        let project = setup_test_project();
        let mut resolver = ModuleResolver::new(project.path().to_path_buf());

        let module = resolver.resolve("utils").unwrap();
        assert_eq!(module.name, "utils");
        assert!(module.path.ends_with("utils.fc"));
    }

    #[test]
    fn test_resolve_directory_module() {
        let project = setup_test_project();
        let mut resolver = ModuleResolver::new(project.path().to_path_buf());

        let module = resolver.resolve("math").unwrap();
        assert_eq!(module.name, "math");
        assert!(module.path.ends_with("mod.fc"));
    }

    #[test]
    fn test_resolve_not_found() {
        let project = setup_test_project();
        let mut resolver = ModuleResolver::new(project.path().to_path_buf());

        let result = resolver.resolve("nonexistent");
        assert!(matches!(result, Err(ModuleError::NotFound { .. })));
    }

    #[test]
    fn test_load_content() {
        let project = setup_test_project();
        let mut resolver = ModuleResolver::new(project.path().to_path_buf());

        let content = resolver.load_content("utils").unwrap();
        assert!(content.contains("helper"));
    }
}
