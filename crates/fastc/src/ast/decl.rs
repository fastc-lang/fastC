//! Declaration AST nodes

use super::{Block, ConstExpr, Span, TypeExpr};

/// `@mem(arena = <ident>)` — names the memory region this function
/// allocates from. v1.x is documentation-only; arena-aware allocators
/// land in v2.x. Stored verbatim so `fastc explain` can echo it and
/// downstream tools can consume it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemAnnot {
    pub arena: String,
}

/// `@panics(never | always | on = <expr>)` — declares the function's
/// panic surface. `Never` cross-checks against the call graph to ensure
/// no path reaches `fc_trap`. `Always` is documentation. `On(cond)`
/// declares the panic happens iff `cond` holds (documentation in v1.x;
/// SMT-discharged in a future stage).
#[derive(Debug, Clone)]
pub enum PanicsAnnot {
    Never,
    Always,
    On(super::Expr),
}

/// `@purity(pure | effect | io)` — observable-effect classification.
/// `Pure` ⇒ no allocation, no global mutation, no I/O. `Effect` ⇒ may
/// mutate parameters or globals but no I/O. `Io` ⇒ consumes a cap
/// token; observable outside the process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurityLevel {
    Pure,
    Effect,
    Io,
}

/// `@complexity(O(<expr>))` — informational time-complexity bound.
/// v1.x is documentation-only; the expression is parsed into a small
/// DSL (Const / N / Log / Mul / Add / Pow) so downstream tools can
/// reason about it without re-parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BigO {
    /// `O(1)` — constant time
    Const,
    /// `O(n)` — linear in the size variable
    N,
    /// `O(log n)`
    Log,
    /// `O(n log n)`
    NLogN,
    /// `O(n^k)` for k ≥ 2
    NPow(u32),
    /// `O(2^n)` — exponential
    Exp,
    /// Fallback: opaque big-O string for shapes we don't classify
    Other(String),
}

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
    /// `@requires(expr)` runtime preconditions. Each entry is the
    /// boolean expression the caller must satisfy. Lowered to
    /// `if (!cond) fc_trap();` at the function-body prologue. SMT
    /// discharge replaces the runtime trap in stage 2.1.
    pub requires: Vec<super::Expr>,
    /// `@ensures(expr)` runtime postconditions. Each entry is a
    /// boolean expression that must hold at every function exit.
    /// Inside the expression, the identifier `result` refers to
    /// the value the function is about to return. Lowered by
    /// capturing every `return EXPR;` into a temp `__ensures_result`
    /// and inserting `if (!cond) fc_trap();` immediately before
    /// the return. v2.1 will hand these to the SMT discharge
    /// pipeline alongside `@requires`.
    pub ensures: Vec<super::Expr>,
    /// `@mem(arena = ident)` — optional memory-region declaration.
    /// Documentation-only in v1.x; surfaced through `fastc explain`.
    pub mem: Option<MemAnnot>,
    /// `@panics(never | always | on = expr)` — optional panic-surface
    /// declaration. `Never` is enforced by the annotation-check pass;
    /// the other variants are documentation-only in v1.x.
    pub panics: Option<PanicsAnnot>,
    /// `@purity(pure | effect | io)` — optional effect classification.
    /// `Pure` is enforced (no alloc, no global mutation, no I/O).
    pub purity: Option<PurityLevel>,
    /// `@complexity(O(<expr>))` — informational complexity bound.
    /// Documentation-only in v1.x; surfaced through `fastc explain`.
    pub complexity: Option<BigO>,
    /// `@test` — function is a unit test, only emitted under `--test`.
    /// B3 inline `test { }` blocks set this implicitly on every fn
    /// inside the block; users can also annotate free functions.
    pub is_test: bool,
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
    /// Module header parsed from `//!` lines at the start of the body.
    /// `None` when no `//!` lines were present. `module_graph::validate`
    /// either requires this set (in strict mode) or treats absence as
    /// a legacy module that compiles untouched.
    pub header: Option<ModuleHeader>,
}

/// Module-level header parsed from `//! @key = "value"` lines at the
/// top of an inline `mod foo { ... }` body or a file-as-module file.
///
/// Stage 1.3 mandates `@module` / `@owns` / `@arch` / `@depends` /
/// `@threading` / `@invariants` when any `//!` line is present. The
/// module-graph pass validates uniqueness of `@owns`, exhaustiveness
/// of `@depends` (every `use mod::X` must point at a declared dep),
/// and `@arch` layering (lower layer can't depend on higher layer).
#[derive(Debug, Clone, Default)]
pub struct ModuleHeader {
    /// `@module = "name"` — display name. Often matches the decl name.
    pub module_name: Option<String>,
    /// `@owns = "ns1, ns2, ..."` — namespaces this module is the sole
    /// owner of. Validated globally unique across all modules.
    pub owns: Vec<String>,
    /// `@arch = "layer"` — architectural layer the module belongs to.
    /// Layering is enforced as a DAG by the module-graph pass.
    pub arch: Option<String>,
    /// `@depends = "dep1, dep2, ..."` — modules this one may import
    /// from. Every actual `use mod::X` must point inside this list.
    pub depends: Vec<String>,
    /// `@threading = "single | thread_safe | concurrent"`.
    pub threading: Option<String>,
    /// `@invariants = "..."` — free-text invariants. Multiple
    /// `@invariants` lines accumulate.
    pub invariants: Vec<String>,
    /// The raw `//!` lines as they appeared, preserved for
    /// `fastc explain` and `fastc fmt`.
    pub raw_lines: Vec<String>,
}
