//! Scope and symbol table management

use indexmap::IndexMap;

use crate::ast::{PrimitiveType, TypeExpr};
use crate::lexer::Span;

/// A symbol in the symbol table
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: TypeExpr,
    pub span: Span,
}

/// Kind of symbol
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    /// Local or parameter variable
    Variable,
    /// Function
    Function { is_unsafe: bool },
    /// Struct type
    Struct,
    /// Enum type
    Enum,
    /// Constant
    Constant,
    /// Opaque type
    Opaque,
}

/// A scope containing symbols
#[derive(Debug)]
pub struct Scope {
    symbols: IndexMap<String, Symbol>,
    parent: Option<usize>,
}

impl Scope {
    pub fn new(parent: Option<usize>) -> Self {
        Self {
            symbols: IndexMap::new(),
            parent,
        }
    }

    pub fn define(&mut self, symbol: Symbol) -> Result<(), Symbol> {
        if self.symbols.contains_key(&symbol.name) {
            Err(symbol)
        } else {
            self.symbols.insert(symbol.name.clone(), symbol);
            Ok(())
        }
    }

    pub fn lookup_local(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    pub fn parent(&self) -> Option<usize> {
        self.parent
    }
}

/// Symbol table with nested scopes
#[derive(Debug)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
    current: usize,
}

impl SymbolTable {
    pub fn new() -> Self {
        // Start with global scope
        let mut table = Self {
            scopes: vec![Scope::new(None)],
            current: 0,
        };

        // Pre-define builtin types
        table.define_builtins();
        table
    }

    fn define_builtins(&mut self) {
        // Primitive types are handled specially, but we could add builtin functions here
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        let new_scope = Scope::new(Some(self.current));
        self.scopes.push(new_scope);
        self.current = self.scopes.len() - 1;
    }

    /// Exit the current scope
    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current].parent() {
            self.current = parent;
        }
    }

    /// Define a symbol in the current scope
    pub fn define(&mut self, symbol: Symbol) -> Result<(), Symbol> {
        self.scopes[self.current].define(symbol)
    }

    /// Lookup a symbol, searching through parent scopes
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let mut scope_idx = Some(self.current);

        while let Some(idx) = scope_idx {
            if let Some(sym) = self.scopes[idx].lookup_local(name) {
                return Some(sym);
            }
            scope_idx = self.scopes[idx].parent();
        }

        None
    }

    /// Lookup only in the current scope (for redefinition checks)
    pub fn lookup_current(&self, name: &str) -> Option<&Symbol> {
        self.scopes[self.current].lookup_local(name)
    }

    /// Get the current scope depth (for debugging)
    pub fn depth(&self) -> usize {
        let mut depth = 0;
        let mut scope_idx = Some(self.current);
        while let Some(idx) = scope_idx {
            depth += 1;
            scope_idx = self.scopes[idx].parent();
        }
        depth
    }

    /// Get all symbol names visible from the current scope (for suggestions)
    pub fn all_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        let mut scope_idx = Some(self.current);

        while let Some(idx) = scope_idx {
            for name in self.scopes[idx].symbols.keys() {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
            scope_idx = self.scopes[idx].parent();
        }

        names
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a void type for functions that don't return a value
pub fn void_type() -> TypeExpr {
    TypeExpr::Void
}

/// Create a function type from parameters and return type
pub fn fn_type(params: Vec<TypeExpr>, ret: TypeExpr, is_unsafe: bool) -> TypeExpr {
    TypeExpr::Fn {
        is_unsafe,
        params,
        ret: Box::new(ret),
    }
}

/// Get the type for a primitive
pub fn primitive_type(p: PrimitiveType) -> TypeExpr {
    TypeExpr::Primitive(p)
}
