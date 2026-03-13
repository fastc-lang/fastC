//! Expression AST nodes

use super::{Span, TypeExpr};

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Logical
    And,
    Or,
    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

/// An expression
#[derive(Debug, Clone)]
pub enum Expr {
    /// Integer literal
    IntLit {
        value: i128,
        span: Span,
    },
    /// Float literal
    FloatLit {
        value: f64,
        raw: String,
        span: Span,
    },
    /// Boolean literal
    BoolLit {
        value: bool,
        span: Span,
    },
    /// Identifier
    Ident {
        name: String,
        span: Span,
    },
    /// Binary operation (exactly one operator per level)
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// Unary operation
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    /// Parenthesized expression
    Paren {
        inner: Box<Expr>,
        span: Span,
    },
    /// Function call
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Field access: expr.field
    Field {
        base: Box<Expr>,
        field: String,
        span: Span,
    },
    /// addr(x) - take address
    Addr {
        operand: Box<Expr>,
        span: Span,
    },
    /// deref(p) - dereference pointer
    Deref {
        operand: Box<Expr>,
        span: Span,
    },
    /// at(arr, i) - array/slice indexing
    At {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// cast(T, expr) - explicit type cast
    Cast {
        ty: TypeExpr,
        expr: Box<Expr>,
        span: Span,
    },
    /// cstr("...") - C string literal
    CStr {
        value: String,
        span: Span,
    },
    /// bytes("...") - byte slice literal
    Bytes {
        value: String,
        span: Span,
    },
    /// none(T) - empty optional
    None {
        ty: TypeExpr,
        span: Span,
    },
    /// some(v) - wrap value in optional
    Some {
        value: Box<Expr>,
        span: Span,
    },
    /// ok(v) - success result
    Ok {
        value: Box<Expr>,
        span: Span,
    },
    /// err(e) - error result
    Err {
        value: Box<Expr>,
        span: Span,
    },
    /// Struct literal: Name { field: value, ... }
    StructLit {
        name: String,
        fields: Vec<FieldInit>,
        span: Span,
    },
}

/// A field initializer in a struct literal
#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

/// A constant expression (subset of Expr)
#[derive(Debug, Clone, PartialEq)]
pub enum ConstExpr {
    IntLit(i128),
    FloatLit(f64),
    BoolLit(bool),
    Ident(String),
    Binary {
        op: BinOp,
        lhs: Box<ConstExpr>,
        rhs: Box<ConstExpr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<ConstExpr>,
    },
    Paren(Box<ConstExpr>),
    Cast {
        ty: TypeExpr,
        expr: Box<ConstExpr>,
    },
    CStr(String),
    Bytes(String),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit { span, .. }
            | Expr::FloatLit { span, .. }
            | Expr::BoolLit { span, .. }
            | Expr::Ident { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Paren { span, .. }
            | Expr::Call { span, .. }
            | Expr::Field { span, .. }
            | Expr::Addr { span, .. }
            | Expr::Deref { span, .. }
            | Expr::At { span, .. }
            | Expr::Cast { span, .. }
            | Expr::CStr { span, .. }
            | Expr::Bytes { span, .. }
            | Expr::None { span, .. }
            | Expr::Some { span, .. }
            | Expr::Ok { span, .. }
            | Expr::Err { span, .. }
            | Expr::StructLit { span, .. } => span.clone(),
        }
    }
}
