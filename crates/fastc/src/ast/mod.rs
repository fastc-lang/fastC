//! AST node definitions for FastC

mod decl;
mod expr;
mod span;
mod stmt;
mod types;

pub use decl::*;
pub use expr::*;
pub use span::*;
pub use stmt::*;
pub use types::*;

/// A complete FastC source file
#[derive(Debug, Clone)]
pub struct File {
    pub items: Vec<Item>,
}
