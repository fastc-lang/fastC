//! Type AST nodes

use super::Span;

/// A type expression
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// Primitive types: i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool, usize, isize
    Primitive(PrimitiveType),
    /// Named type (struct, enum, or alias)
    Named(String),
    /// ref(T) - non-null immutable reference
    Ref(Box<TypeExpr>),
    /// mref(T) - non-null mutable reference
    Mref(Box<TypeExpr>),
    /// raw(T) - nullable raw pointer (immutable)
    Raw(Box<TypeExpr>),
    /// rawm(T) - nullable raw pointer (mutable)
    Rawm(Box<TypeExpr>),
    /// own(T) - owning pointer
    Own(Box<TypeExpr>),
    /// slice(T) - view over contiguous elements
    Slice(Box<TypeExpr>),
    /// arr(T, N) - fixed-size array
    Arr(Box<TypeExpr>, Box<super::ConstExpr>),
    /// opt(T) - optional value
    Opt(Box<TypeExpr>),
    /// res(T, E) - result type
    Res(Box<TypeExpr>, Box<TypeExpr>),
    /// fn(...) -> T or unsafe fn(...) -> T
    Fn {
        is_unsafe: bool,
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },
    /// void (return type only)
    Void,
}

/// Primitive types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Usize,
    Isize,
}

/// A type expression with span information
#[derive(Debug, Clone)]
pub struct SpannedType {
    pub ty: TypeExpr,
    pub span: Span,
}
