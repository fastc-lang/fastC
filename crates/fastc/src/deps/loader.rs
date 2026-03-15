//! Module loader for expanding `mod` declarations
//!
//! Recursively loads module files and merges them into a single AST.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::ast::{File, Item, ModDecl};
use crate::diag::CompileError;
use crate::lexer::{Lexer, strip_comments};
use crate::parser::Parser;

use super::resolver::{ModuleError, ModuleResolver};

/// Module loader that expands `mod name;` declarations
pub struct ModuleLoader {
    /// Module resolver for finding files
    resolver: ModuleResolver,
    /// Tracks loaded files to prevent circular imports
    loaded: HashSet<PathBuf>,
}

impl ModuleLoader {
    /// Create a new module loader for the given project root
    pub fn new(project_root: &Path) -> Self {
        Self {
            resolver: ModuleResolver::new(project_root.to_path_buf()),
            loaded: HashSet::new(),
        }
    }

    /// Create a module loader from a source file path
    ///
    /// Walks up the directory tree to find the project root (fastc.toml)
    pub fn from_source_file(source_path: &Path) -> Option<Self> {
        ModuleResolver::from_source_file(source_path).map(|resolver| Self {
            resolver,
            loaded: HashSet::new(),
        })
    }

    /// Expand all external `mod name;` declarations in an AST
    ///
    /// This modifies the AST in place, replacing external module declarations
    /// with inline modules containing the loaded content.
    pub fn expand_modules(&mut self, ast: &mut File, source_dir: &Path) -> Result<(), LoaderError> {
        let mut new_items = Vec::new();

        for item in std::mem::take(&mut ast.items) {
            match item {
                Item::Mod(mod_decl) if mod_decl.body.is_none() => {
                    // External module - load and expand
                    let expanded = self.load_and_expand_module(&mod_decl, source_dir)?;
                    new_items.push(Item::Mod(expanded));
                }
                Item::Mod(mut mod_decl) if mod_decl.body.is_some() => {
                    // Inline module - recursively expand nested modules
                    if let Some(ref mut body) = mod_decl.body {
                        let mut inner_ast = File {
                            items: std::mem::take(body),
                        };
                        self.expand_modules(&mut inner_ast, source_dir)?;
                        *body = inner_ast.items;
                    }
                    new_items.push(Item::Mod(mod_decl));
                }
                other => new_items.push(other),
            }
        }

        ast.items = new_items;
        Ok(())
    }

    /// Load a module file and expand any nested modules
    fn load_and_expand_module(
        &mut self,
        mod_decl: &ModDecl,
        source_dir: &Path,
    ) -> Result<ModDecl, LoaderError> {
        // Resolve module to file path
        let module_path = self.resolve_module_path(&mod_decl.name, source_dir)?;

        // Check for circular imports
        let canonical = module_path
            .canonicalize()
            .unwrap_or_else(|_| module_path.clone());
        if self.loaded.contains(&canonical) {
            return Err(LoaderError::CircularImport {
                module: mod_decl.name.clone(),
                path: module_path,
            });
        }
        self.loaded.insert(canonical.clone());

        // Load and parse the module file
        let source = std::fs::read_to_string(&module_path).map_err(|e| LoaderError::Io {
            path: module_path.clone(),
            error: e.to_string(),
        })?;

        let mut ast = self.parse_module(&source, &module_path)?;

        // Get the directory of the module for resolving nested modules
        let module_dir = module_path.parent().unwrap_or(source_dir);

        // Recursively expand any nested modules
        self.expand_modules(&mut ast, module_dir)?;

        // Create an inline module with the loaded content
        Ok(ModDecl {
            is_pub: mod_decl.is_pub,
            name: mod_decl.name.clone(),
            body: Some(ast.items),
            span: mod_decl.span.clone(),
        })
    }

    /// Resolve a module name to its file path
    fn resolve_module_path(&self, name: &str, source_dir: &Path) -> Result<PathBuf, LoaderError> {
        // First try relative to source directory
        let direct_path = source_dir.join(format!("{}.fc", name));
        if direct_path.exists() {
            return Ok(direct_path);
        }

        // Try directory module
        let dir_path = source_dir.join(name).join("mod.fc");
        if dir_path.exists() {
            return Ok(dir_path);
        }

        // Fall back to resolver (uses src/ directory)
        match self.resolver.find_module_file(name) {
            Ok(path) => Ok(path),
            Err(_) => Err(LoaderError::ModuleNotFound {
                name: name.to_string(),
                searched: vec![direct_path, dir_path],
            }),
        }
    }

    /// Parse a module file into an AST
    fn parse_module(&self, source: &str, path: &Path) -> Result<File, LoaderError> {
        let lexer = Lexer::new(source);
        let tokens = strip_comments(lexer.collect());

        let filename = path.to_string_lossy().to_string();
        let mut parser = Parser::new(&tokens, source, &filename);
        parser.parse_file().map_err(|e| LoaderError::Parse {
            path: path.to_path_buf(),
            error: e.to_string(),
        })
    }

    /// Get the project root directory
    pub fn root(&self) -> &Path {
        self.resolver.root()
    }
}

/// Errors that can occur during module loading
#[derive(Debug)]
pub enum LoaderError {
    /// Module file not found
    ModuleNotFound {
        name: String,
        searched: Vec<PathBuf>,
    },
    /// Circular import detected
    CircularImport { module: String, path: PathBuf },
    /// IO error reading module file
    Io { path: PathBuf, error: String },
    /// Parse error in module file
    Parse { path: PathBuf, error: String },
    /// Module resolution error
    Resolution(ModuleError),
}

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoaderError::ModuleNotFound { name, searched } => {
                write!(f, "module '{}' not found, searched: ", name)?;
                for (i, path) in searched.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", path.display())?;
                }
                Ok(())
            }
            LoaderError::CircularImport { module, path } => {
                write!(
                    f,
                    "circular import detected: module '{}' at {}",
                    module,
                    path.display()
                )
            }
            LoaderError::Io { path, error } => {
                write!(f, "failed to read {}: {}", path.display(), error)
            }
            LoaderError::Parse { path, error } => {
                write!(f, "failed to parse {}: {}", path.display(), error)
            }
            LoaderError::Resolution(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for LoaderError {}

impl From<ModuleError> for LoaderError {
    fn from(e: ModuleError) -> Self {
        LoaderError::Resolution(e)
    }
}

impl From<LoaderError> for CompileError {
    fn from(e: LoaderError) -> Self {
        CompileError::parse(e.to_string(), 0..0, "")
    }
}

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
            "[package]\nname = \"test\"\n",
        )
        .unwrap();

        dir
    }

    #[test]
    fn test_expand_simple_module() {
        let project = setup_test_project();

        // Create main file with mod declaration
        fs::write(
            project.path().join("src/main.fc"),
            "mod utils;\nfn main() -> i32 { return 0; }",
        )
        .unwrap();

        // Create utils module
        fs::write(
            project.path().join("src/utils.fc"),
            "fn helper() -> i32 { return 42; }",
        )
        .unwrap();

        // Parse main file
        let source = fs::read_to_string(project.path().join("src/main.fc")).unwrap();
        let lexer = Lexer::new(&source);
        let tokens = strip_comments(lexer.collect());
        let mut parser = Parser::new(&tokens, &source, "src/main.fc");
        let mut ast = parser.parse_file().unwrap();

        // Expand modules
        let mut loader = ModuleLoader::new(project.path());
        loader
            .expand_modules(&mut ast, &project.path().join("src"))
            .unwrap();

        // Check that module was expanded
        assert_eq!(ast.items.len(), 2); // mod utils (now inline) + fn main
        if let Item::Mod(mod_decl) = &ast.items[0] {
            assert_eq!(mod_decl.name, "utils");
            assert!(mod_decl.body.is_some());
            assert_eq!(mod_decl.body.as_ref().unwrap().len(), 1); // fn helper
        } else {
            panic!("expected mod item");
        }
    }

    #[test]
    fn test_circular_import_detection() {
        let project = setup_test_project();

        // Create circular imports: a -> b -> a
        fs::write(project.path().join("src/a.fc"), "mod b;").unwrap();
        fs::write(project.path().join("src/b.fc"), "mod a;").unwrap();

        let source = "mod a;";
        let lexer = Lexer::new(source);
        let tokens = strip_comments(lexer.collect());
        let mut parser = Parser::new(&tokens, source, "main.fc");
        let mut ast = parser.parse_file().unwrap();

        let mut loader = ModuleLoader::new(project.path());
        let result = loader.expand_modules(&mut ast, &project.path().join("src"));

        assert!(matches!(result, Err(LoaderError::CircularImport { .. })));
    }
}
