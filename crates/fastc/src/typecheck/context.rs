//! Type context for type checking

use crate::ast::TypeExpr;

/// Type context for tracking types during type checking
#[derive(Debug, Default)]
pub struct TypeContext {
    // TODO: Add type tracking
}

impl TypeContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if two types are equal
    pub fn types_equal(&self, _a: &TypeExpr, _b: &TypeExpr) -> bool {
        // TODO: Implement type equality
        true
    }
}
