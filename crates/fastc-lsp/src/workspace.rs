//! Workspace support for cross-file LSP features
//!
//! This module handles:
//! - Indexing all .fc files in the workspace
//! - Cross-file go-to-definition
//! - Global symbol lookup

use dashmap::DashMap;
use std::path::Path;
use tower_lsp::lsp_types::{Location, Position, Range, Url};

use crate::diagnostics::byte_to_position;

/// Symbol information for workspace indexing
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// Name of the symbol
    pub name: String,
    /// Kind of symbol
    pub kind: SymbolKind,
    /// File containing the symbol
    pub uri: Url,
    /// Position range in the file
    pub range: Range,
}

/// Kind of symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Const,
    Opaque,
    Module,
}

/// Workspace index for cross-file features
pub struct Workspace {
    /// Global symbol index: name → list of symbols with that name
    symbols: DashMap<String, Vec<SymbolInfo>>,
    /// Indexed files: URI → file content hash (for change detection)
    indexed_files: DashMap<Url, u64>,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            symbols: DashMap::new(),
            indexed_files: DashMap::new(),
        }
    }

    /// Index all .fc files in the given workspace root
    pub fn index_workspace(&self, root: &Path) {
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().is_some_and(|ext| ext == "fc")
                    && e.file_type().is_file()
            })
        {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if let Ok(uri) = Url::from_file_path(entry.path()) {
                    self.index_file(&uri, &content);
                }
            }
        }
    }

    /// Index a single file
    pub fn index_file(&self, uri: &Url, content: &str) {
        // Remove old symbols from this file
        self.remove_file_symbols(uri);

        // Calculate content hash for change detection
        let hash = hash_content(content);
        self.indexed_files.insert(uri.clone(), hash);

        // Parse and extract symbols
        let filename = uri.path().to_string();
        let lexer = fastc::lexer::Lexer::new(content);
        let tokens: Vec<_> = lexer.collect();
        let tokens = fastc::lexer::strip_comments(tokens);
        let mut parser = fastc::parser::Parser::new(&tokens, content, &filename);

        let Ok(ast) = parser.parse_file() else {
            return; // Skip files with parse errors
        };

        // Extract symbols from AST
        for item in &ast.items {
            match item {
                fastc::ast::Item::Fn(decl) => {
                    self.add_symbol(SymbolInfo {
                        name: decl.name.clone(),
                        kind: SymbolKind::Function,
                        uri: uri.clone(),
                        range: Range::new(
                            byte_to_position(content, decl.span.start),
                            byte_to_position(content, decl.span.end),
                        ),
                    });
                }
                fastc::ast::Item::Struct(decl) => {
                    self.add_symbol(SymbolInfo {
                        name: decl.name.clone(),
                        kind: SymbolKind::Struct,
                        uri: uri.clone(),
                        range: Range::new(
                            byte_to_position(content, decl.span.start),
                            byte_to_position(content, decl.span.end),
                        ),
                    });
                }
                fastc::ast::Item::Enum(decl) => {
                    self.add_symbol(SymbolInfo {
                        name: decl.name.clone(),
                        kind: SymbolKind::Enum,
                        uri: uri.clone(),
                        range: Range::new(
                            byte_to_position(content, decl.span.start),
                            byte_to_position(content, decl.span.end),
                        ),
                    });
                }
                fastc::ast::Item::Const(decl) => {
                    self.add_symbol(SymbolInfo {
                        name: decl.name.clone(),
                        kind: SymbolKind::Const,
                        uri: uri.clone(),
                        range: Range::new(
                            byte_to_position(content, decl.span.start),
                            byte_to_position(content, decl.span.end),
                        ),
                    });
                }
                fastc::ast::Item::Opaque(decl) => {
                    self.add_symbol(SymbolInfo {
                        name: decl.name.clone(),
                        kind: SymbolKind::Opaque,
                        uri: uri.clone(),
                        range: Range::new(
                            byte_to_position(content, decl.span.start),
                            byte_to_position(content, decl.span.end),
                        ),
                    });
                }
                fastc::ast::Item::Extern(block) => {
                    for extern_item in &block.items {
                        match extern_item {
                            fastc::ast::ExternItem::Fn(proto) => {
                                self.add_symbol(SymbolInfo {
                                    name: proto.name.clone(),
                                    kind: SymbolKind::Function,
                                    uri: uri.clone(),
                                    range: Range::new(
                                        byte_to_position(content, proto.span.start),
                                        byte_to_position(content, proto.span.end),
                                    ),
                                });
                            }
                            fastc::ast::ExternItem::Struct(decl) => {
                                self.add_symbol(SymbolInfo {
                                    name: decl.name.clone(),
                                    kind: SymbolKind::Struct,
                                    uri: uri.clone(),
                                    range: Range::new(
                                        byte_to_position(content, decl.span.start),
                                        byte_to_position(content, decl.span.end),
                                    ),
                                });
                            }
                            fastc::ast::ExternItem::Enum(decl) => {
                                self.add_symbol(SymbolInfo {
                                    name: decl.name.clone(),
                                    kind: SymbolKind::Enum,
                                    uri: uri.clone(),
                                    range: Range::new(
                                        byte_to_position(content, decl.span.start),
                                        byte_to_position(content, decl.span.end),
                                    ),
                                });
                            }
                            fastc::ast::ExternItem::Opaque(decl) => {
                                self.add_symbol(SymbolInfo {
                                    name: decl.name.clone(),
                                    kind: SymbolKind::Opaque,
                                    uri: uri.clone(),
                                    range: Range::new(
                                        byte_to_position(content, decl.span.start),
                                        byte_to_position(content, decl.span.end),
                                    ),
                                });
                            }
                        }
                    }
                }
                fastc::ast::Item::Use(_) => {
                    // Use declarations don't create symbols in the workspace index
                }
                fastc::ast::Item::Mod(decl) => {
                    self.add_symbol(SymbolInfo {
                        name: decl.name.clone(),
                        kind: SymbolKind::Module,
                        uri: uri.clone(),
                        range: Range::new(
                            byte_to_position(content, decl.span.start),
                            byte_to_position(content, decl.span.end),
                        ),
                    });
                }
            }
        }
    }

    /// Add a symbol to the index
    fn add_symbol(&self, symbol: SymbolInfo) {
        self.symbols
            .entry(symbol.name.clone())
            .or_default()
            .push(symbol);
    }

    /// Remove all symbols from a file
    pub fn remove_file_symbols(&self, uri: &Url) {
        // Remove from indexed files
        self.indexed_files.remove(uri);

        // Remove symbols from this file
        for mut entry in self.symbols.iter_mut() {
            entry.value_mut().retain(|s| &s.uri != uri);
        }

        // Clean up empty entries
        self.symbols.retain(|_, v| !v.is_empty());
    }

    /// Find definition for a symbol name
    pub fn find_definition(&self, name: &str) -> Option<Location> {
        self.symbols.get(name).and_then(|symbols| {
            symbols.first().map(|s| Location {
                uri: s.uri.clone(),
                range: s.range,
            })
        })
    }

    /// Find all definitions for a symbol name (for peek definition)
    pub fn find_all_definitions(&self, name: &str) -> Vec<Location> {
        self.symbols
            .get(name)
            .map(|symbols| {
                symbols
                    .iter()
                    .map(|s| Location {
                        uri: s.uri.clone(),
                        range: s.range,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all symbol names in the workspace
    pub fn all_symbol_names(&self) -> Vec<String> {
        self.symbols.iter().map(|e| e.key().clone()).collect()
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple hash function for content change detection
fn hash_content(content: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}
