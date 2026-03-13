//! Declaration AST nodes

use super::{Block, ConstExpr, Span, TypeExpr};

/// A top-level item
#[derive(Debug, Clone)]
pub enum Item {
    Fn(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Const(ConstDecl),
    Opaque(OpaqueDecl),
    Extern(ExternBlock),
    Use(UseDecl),
    Mod(ModDecl),
}

/// Function declaration
#[derive(Debug, Clone)]
pub struct FnDecl {
    pub is_unsafe: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub body: Block,
    pub span: Span,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

/// Struct declaration
#[derive(Debug, Clone)]
pub struct StructDecl {
    pub repr: Option<Repr>,
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

/// Struct field
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

/// Enum declaration
#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub repr: Option<Repr>,
    pub name: String,
    pub variants: Vec<Variant>,
    pub span: Span,
}

/// Enum variant
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Option<Vec<TypeExpr>>,
    pub span: Span,
}

/// Constant declaration
#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: String,
    pub ty: TypeExpr,
    pub value: ConstExpr,
    pub span: Span,
}

/// Opaque type declaration
#[derive(Debug, Clone)]
pub struct OpaqueDecl {
    pub name: String,
    pub span: Span,
}

/// Extern block
#[derive(Debug, Clone)]
pub struct ExternBlock {
    pub abi: String,
    pub items: Vec<ExternItem>,
    pub span: Span,
}

/// Items inside an extern block
#[derive(Debug, Clone)]
pub enum ExternItem {
    Fn(FnProto),
    Struct(StructDecl),
    Enum(EnumDecl),
    Opaque(OpaqueDecl),
}

/// Function prototype (no body)
#[derive(Debug, Clone)]
pub struct FnProto {
    pub is_unsafe: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub span: Span,
}

/// Representation attribute
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repr {
    C,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
}

/// Use declaration for importing items
#[derive(Debug, Clone)]
pub struct UseDecl {
    /// Module path (e.g., ["mylib", "utils"])
    pub path: Vec<String>,
    /// Items to import
    pub items: UseItems,
    /// Source span
    pub span: Span,
}

/// What items are being imported
#[derive(Debug, Clone)]
pub enum UseItems {
    /// Import a single item: `use mylib::Vector;`
    Single(String),
    /// Import multiple items: `use mylib::{Vector, Point};`
    Multiple(Vec<String>),
    /// Import all items: `use mylib::*;`
    Glob,
    /// Import the module itself: `use mylib;`
    Module,
}

/// Module declaration
#[derive(Debug, Clone)]
pub struct ModDecl {
    /// Visibility (true if public)
    pub is_pub: bool,
    /// Module name
    pub name: String,
    /// Inline module body, or None to load from file
    pub body: Option<Vec<Item>>,
    /// Source span
    pub span: Span,
}
