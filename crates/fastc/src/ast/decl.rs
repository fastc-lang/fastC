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
    /// `impl Type { ... }` (inherent) or `impl Trait for Type { ... }`
    /// (trait impl). Stage 1.0 slice 1 and slice 2 respectively. Methods
    /// are desugared to free functions named `Type_method` between parse
    /// and resolve so the rest of the pipeline sees only ordinary call
    /// sites.
    Impl(ImplBlock),
    /// `trait Foo { fn method(self: ref(Self), ...) -> T; ... }`. Stage
    /// 1.0 slice 2.
    Trait(TraitDecl),
}

/// An inherent or trait impl block.
///
/// * Inherent: `impl Type { ... }` — `trait_name` is `None`.
/// * Trait:    `impl Trait for Type { ... }` — `trait_name` is `Some("Trait")`.
#[derive(Debug, Clone)]
pub struct ImplBlock {
    /// Name of the type these methods are attached to.
    pub target: String,
    /// Name of the trait being implemented (`None` for an inherent impl).
    pub trait_name: Option<String>,
    /// Methods declared inside the block. Each method's body may reference
    /// `Self` (the type) and `self` (the receiver parameter).
    pub methods: Vec<FnDecl>,
    pub span: Span,
    pub doc_comments: Vec<String>,
}

/// `trait Foo { fn method(self: ref(Self), ...) -> T; ... }`.
///
/// A trait declares a set of method prototypes (no bodies). Types
/// implement the trait via `impl Foo for Bar { ... }`, which is parsed as
/// an `ImplBlock` with `trait_name: Some("Foo")`.
#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub name: String,
    /// Method prototypes — same shape as `FnProto` but stored inline so the
    /// signature includes `self` and may use `Self`.
    pub methods: Vec<FnProto>,
    pub span: Span,
    pub doc_comments: Vec<String>,
}

/// Function declaration
#[derive(Debug, Clone)]
pub struct FnDecl {
    pub is_unsafe: bool,
    pub name: String,
    /// Type parameters declared with `fn name[T, U](...)`. Empty for
    /// non-generic functions.
    pub generics: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub body: Block,
    pub span: Span,
    /// Doc comment lines (`///`) preceding this declaration, stripped of
    /// the `///` prefix and a single optional leading space.
    pub doc_comments: Vec<String>,
    /// `@noalloc`, `@nodiverg`, `@pure` and future attribute names
    /// attached to this function. Stored as plain strings so adding
    /// a new attribute keyword in the lexer doesn't require touching
    /// every AST walker.
    pub annotations: Vec<String>,
}

/// A declared type parameter, e.g. the `T` in `fn id[T](x: T) -> T`.
///
/// Constraints take the form `[T: Bound1 + Bound2]`. Each bound is the name
/// of a trait the concrete type-argument must implement.
#[derive(Debug, Clone)]
pub struct TypeParam {
    pub name: String,
    /// Trait names this type parameter is bound by. Empty when unbounded.
    pub bounds: Vec<String>,
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
    /// Type parameters declared with `struct Name[A, B] { ... }`. Empty
    /// for non-generic structs; populated when the struct is generic.
    /// Specialization happens during mono — see `mono::monomorphize`.
    pub generics: Vec<TypeParam>,
    pub fields: Vec<Field>,
    pub span: Span,
    pub doc_comments: Vec<String>,
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
    pub doc_comments: Vec<String>,
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
    pub doc_comments: Vec<String>,
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
    /// Type parameters; same shape as `FnDecl::generics`. Generic externs are
    /// rejected by the typechecker — they have no body to monomorphize.
    pub generics: Vec<TypeParam>,
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
