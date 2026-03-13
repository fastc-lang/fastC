//! Statement AST nodes

use super::{Expr, Span, TypeExpr};

/// A statement
#[derive(Debug, Clone)]
pub enum Stmt {
    /// let name: Type = expr;
    Let {
        name: String,
        ty: TypeExpr,
        init: Expr,
        span: Span,
    },
    /// lhs = expr;
    Assign {
        lhs: Expr,
        rhs: Expr,
        span: Span,
    },
    /// if (cond) { ... } else { ... }
    If {
        cond: Expr,
        then_block: Block,
        else_block: Option<ElseBranch>,
        span: Span,
    },
    /// if let name = unwrap_checked(expr) { ... } else { ... }
    IfLet {
        name: String,
        expr: Expr,
        then_block: Block,
        else_block: Option<Block>,
        span: Span,
    },
    /// while (cond) { ... }
    While {
        cond: Expr,
        body: Block,
        span: Span,
    },
    /// for (init; cond; step) { ... }
    For {
        init: Option<ForInit>,
        cond: Option<Expr>,
        step: Option<ForStep>,
        body: Block,
        span: Span,
    },
    /// switch (expr) { case ...: ... }
    Switch {
        expr: Expr,
        cases: Vec<Case>,
        default: Option<Vec<Stmt>>,
        span: Span,
    },
    /// return expr;
    Return {
        value: Option<Expr>,
        span: Span,
    },
    /// break;
    Break { span: Span },
    /// continue;
    Continue { span: Span },
    /// defer { ... }
    Defer {
        body: Block,
        span: Span,
    },
    /// Expression statement (call only, or discard)
    Expr {
        expr: Expr,
        span: Span,
    },
    /// discard(expr);
    Discard {
        expr: Expr,
        span: Span,
    },
    /// unsafe { ... }
    Unsafe {
        body: Block,
        span: Span,
    },
    /// A block as a statement
    Block(Block),
}

/// A block of statements
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// Else branch: either another if or a block
#[derive(Debug, Clone)]
pub enum ElseBranch {
    ElseIf(Box<Stmt>),
    Else(Block),
}

/// For loop initializer
#[derive(Debug, Clone)]
pub enum ForInit {
    Let {
        name: String,
        ty: TypeExpr,
        init: Expr,
    },
    Assign {
        lhs: Expr,
        rhs: Expr,
    },
    Call(Expr),
}

/// For loop step
#[derive(Debug, Clone)]
pub enum ForStep {
    Assign { lhs: Expr, rhs: Expr },
    Call(Expr),
}

/// A switch case
#[derive(Debug, Clone)]
pub struct Case {
    pub value: super::ConstExpr,
    pub stmts: Vec<Stmt>,
    pub span: Span,
}
