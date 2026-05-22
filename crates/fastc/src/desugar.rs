//! Pre-resolve desugar pass.
//!
//! Lifts every `Item::Impl` method to a free function named
//! `Target_method`, substituting `Self` for the impl'd target type in
//! signatures and body type annotations. After this pass, every method is
//! reachable as an ordinary `Type_method` function.
//!
//! `Item::Impl` and `Item::Trait` items remain in the AST so subsequent
//! passes can read the trait-impl mapping (which type implements which
//! trait) and trait method signatures. Resolve/typecheck/mono skip the
//! bodies of these items but consult them for trait dispatch.
//!
//! Method-call *expressions* (`x.method(args)`) are not rewritten here —
//! that needs type information and happens later, inside the
//! monomorphization pass (see `mono::rewrite_expr`).

use crate::ast::{
    Block, Expr, FieldInit, File, FnDecl, ForInit, ForStep, ImplBlock, Item, Param, Stmt, TypeExpr,
};

/// Lift every `Item::Impl` method into a free `Item::Fn` named
/// `Target_method`. The original `Item::Impl` and `Item::Trait` items are
/// preserved so later passes can read the trait-impl table; the lifted
/// methods follow them in the items list.
pub fn desugar(file: &File) -> File {
    let mut items: Vec<Item> = Vec::with_capacity(file.items.len() * 2);
    let mut lifter = ClosureLifter::new();
    for item in &file.items {
        match item {
            Item::Impl(block) => {
                let mut new_methods: Vec<FnDecl> = Vec::with_capacity(block.methods.len());
                for method in &block.methods {
                    new_methods.push(lifter.rewrite_fn(method));
                }
                let new_block = ImplBlock {
                    target: block.target.clone(),
                    trait_name: block.trait_name.clone(),
                    methods: new_methods.clone(),
                    span: block.span.clone(),
                    doc_comments: block.doc_comments.clone(),
                };
                items.push(Item::Impl(new_block.clone()));
                for method in &new_methods {
                    items.push(Item::Fn(lift_method(&new_block, method)));
                }
            }
            Item::Fn(f) => {
                items.push(Item::Fn(lifter.rewrite_fn(f)));
            }
            Item::Mod(m) => {
                items.push(Item::Mod(lifter.rewrite_mod(m)));
            }
            _ => items.push(item.clone()),
        }
    }
    // Append every synthesized closure fn at the end so the user's
    // ordering stays stable.
    for f in lifter.into_lifted() {
        items.push(Item::Fn(f));
    }
    File { items }
}

/// Counter-bearing walker that replaces every `Expr::Closure` with an
/// `Expr::Ident(__lambda_N)` and accumulates the corresponding
/// synthesized `FnDecl` on the side. Single shared counter across the
/// whole file so names are unique even when closures live in different
/// modules. v1 closures have no captured environment, so the lifted
/// function is structurally identical to a hand-written fn — no
/// environment struct, no per-call allocation.
struct ClosureLifter {
    counter: usize,
    lifted: Vec<FnDecl>,
}

impl ClosureLifter {
    fn new() -> Self {
        Self {
            counter: 0,
            lifted: Vec::new(),
        }
    }

    fn into_lifted(self) -> Vec<FnDecl> {
        self.lifted
    }

    fn fresh_name(&mut self) -> String {
        let n = self.counter;
        self.counter += 1;
        format!("__lambda_{}", n)
    }

    fn rewrite_fn(&mut self, f: &FnDecl) -> FnDecl {
        FnDecl {
            is_unsafe: f.is_unsafe,
            name: f.name.clone(),
            generics: f.generics.clone(),
            doc_comments: f.doc_comments.clone(),
            annotations: f.annotations.clone(),
            params: f.params.clone(),
            return_type: f.return_type.clone(),
            body: self.rewrite_block(&f.body),
            span: f.span.clone(),
        }
    }

    fn rewrite_mod(&mut self, m: &crate::ast::ModDecl) -> crate::ast::ModDecl {
        crate::ast::ModDecl {
            is_pub: m.is_pub,
            name: m.name.clone(),
            body: m.body.as_ref().map(|items| {
                items
                    .iter()
                    .map(|it| match it {
                        Item::Fn(f) => Item::Fn(self.rewrite_fn(f)),
                        Item::Mod(inner) => Item::Mod(self.rewrite_mod(inner)),
                        other => other.clone(),
                    })
                    .collect()
            }),
            span: m.span.clone(),
        }
    }

    fn rewrite_block(&mut self, b: &Block) -> Block {
        Block {
            stmts: b.stmts.iter().map(|s| self.rewrite_stmt(s)).collect(),
            span: b.span.clone(),
        }
    }

    fn rewrite_stmt(&mut self, s: &Stmt) -> Stmt {
        match s {
            Stmt::Let {
                name,
                ty,
                init,
                span,
            } => Stmt::Let {
                name: name.clone(),
                ty: ty.clone(),
                init: self.rewrite_expr(init),
                span: span.clone(),
            },
            Stmt::Assign { lhs, rhs, span } => Stmt::Assign {
                lhs: self.rewrite_expr(lhs),
                rhs: self.rewrite_expr(rhs),
                span: span.clone(),
            },
            Stmt::If {
                cond,
                then_block,
                else_block,
                span,
            } => Stmt::If {
                cond: self.rewrite_expr(cond),
                then_block: self.rewrite_block(then_block),
                else_block: else_block.as_ref().map(|e| self.rewrite_else(e)),
                span: span.clone(),
            },
            Stmt::IfLet {
                name,
                expr,
                then_block,
                else_block,
                span,
            } => Stmt::IfLet {
                name: name.clone(),
                expr: self.rewrite_expr(expr),
                then_block: self.rewrite_block(then_block),
                else_block: else_block.as_ref().map(|b| self.rewrite_block(b)),
                span: span.clone(),
            },
            Stmt::While { cond, body, span } => Stmt::While {
                cond: self.rewrite_expr(cond),
                body: self.rewrite_block(body),
                span: span.clone(),
            },
            Stmt::For {
                init,
                cond,
                step,
                body,
                span,
            } => Stmt::For {
                init: init.as_ref().map(|i| self.rewrite_for_init(i)),
                cond: cond.as_ref().map(|c| self.rewrite_expr(c)),
                step: step.as_ref().map(|s| self.rewrite_for_step(s)),
                body: self.rewrite_block(body),
                span: span.clone(),
            },
            Stmt::Switch {
                expr,
                cases,
                default,
                span,
            } => Stmt::Switch {
                expr: self.rewrite_expr(expr),
                cases: cases
                    .iter()
                    .map(|c| crate::ast::Case {
                        value: c.value.clone(),
                        stmts: c.stmts.iter().map(|s| self.rewrite_stmt(s)).collect(),
                        span: c.span.clone(),
                    })
                    .collect(),
                default: default
                    .as_ref()
                    .map(|d| d.iter().map(|s| self.rewrite_stmt(s)).collect()),
                span: span.clone(),
            },
            Stmt::Return { value, span } => Stmt::Return {
                value: value.as_ref().map(|e| self.rewrite_expr(e)),
                span: span.clone(),
            },
            Stmt::Defer { body, span } => Stmt::Defer {
                body: self.rewrite_block(body),
                span: span.clone(),
            },
            Stmt::Unsafe { body, span } => Stmt::Unsafe {
                body: self.rewrite_block(body),
                span: span.clone(),
            },
            Stmt::Block(b) => Stmt::Block(self.rewrite_block(b)),
            Stmt::Expr { expr, span } => Stmt::Expr {
                expr: self.rewrite_expr(expr),
                span: span.clone(),
            },
            Stmt::Discard { expr, span } => Stmt::Discard {
                expr: self.rewrite_expr(expr),
                span: span.clone(),
            },
            Stmt::Break { .. } | Stmt::Continue { .. } => s.clone(),
        }
    }

    fn rewrite_else(&mut self, e: &crate::ast::ElseBranch) -> crate::ast::ElseBranch {
        match e {
            crate::ast::ElseBranch::ElseIf(s) => {
                crate::ast::ElseBranch::ElseIf(Box::new(self.rewrite_stmt(s)))
            }
            crate::ast::ElseBranch::Else(b) => crate::ast::ElseBranch::Else(self.rewrite_block(b)),
        }
    }

    fn rewrite_for_init(&mut self, fi: &ForInit) -> ForInit {
        match fi {
            ForInit::Let { name, ty, init } => ForInit::Let {
                name: name.clone(),
                ty: ty.clone(),
                init: self.rewrite_expr(init),
            },
            ForInit::Assign { lhs, rhs } => ForInit::Assign {
                lhs: self.rewrite_expr(lhs),
                rhs: self.rewrite_expr(rhs),
            },
            ForInit::Call(e) => ForInit::Call(self.rewrite_expr(e)),
        }
    }

    fn rewrite_for_step(&mut self, fs: &ForStep) -> ForStep {
        match fs {
            ForStep::Assign { lhs, rhs } => ForStep::Assign {
                lhs: self.rewrite_expr(lhs),
                rhs: self.rewrite_expr(rhs),
            },
            ForStep::Call(e) => ForStep::Call(self.rewrite_expr(e)),
        }
    }

    fn rewrite_expr(&mut self, e: &Expr) -> Expr {
        match e {
            // The interesting case: lift to a synthetic top-level fn
            // and replace this expression with a reference by name.
            Expr::Closure {
                params,
                ret,
                body,
                span,
            } => {
                let name = self.fresh_name();
                let new_body = self.rewrite_block(body);
                self.lifted.push(FnDecl {
                    is_unsafe: false,
                    name: name.clone(),
                    generics: Vec::new(),
                    doc_comments: Vec::new(),
                    annotations: Vec::new(),
                    params: params.clone(),
                    return_type: ret.clone(),
                    body: new_body,
                    span: span.clone(),
                });
                Expr::Ident {
                    name,
                    span: span.clone(),
                }
            }
            Expr::Call { callee, args, span } => Expr::Call {
                callee: Box::new(self.rewrite_expr(callee)),
                args: args.iter().map(|a| self.rewrite_expr(a)).collect(),
                span: span.clone(),
            },
            Expr::Binary { op, lhs, rhs, span } => Expr::Binary {
                op: *op,
                lhs: Box::new(self.rewrite_expr(lhs)),
                rhs: Box::new(self.rewrite_expr(rhs)),
                span: span.clone(),
            },
            Expr::Unary { op, operand, span } => Expr::Unary {
                op: *op,
                operand: Box::new(self.rewrite_expr(operand)),
                span: span.clone(),
            },
            Expr::Paren { inner, span } => Expr::Paren {
                inner: Box::new(self.rewrite_expr(inner)),
                span: span.clone(),
            },
            Expr::Field { base, field, span } => Expr::Field {
                base: Box::new(self.rewrite_expr(base)),
                field: field.clone(),
                span: span.clone(),
            },
            Expr::Addr { operand, span } => Expr::Addr {
                operand: Box::new(self.rewrite_expr(operand)),
                span: span.clone(),
            },
            Expr::AddrM { operand, span } => Expr::AddrM {
                operand: Box::new(self.rewrite_expr(operand)),
                span: span.clone(),
            },
            Expr::Deref { operand, span } => Expr::Deref {
                operand: Box::new(self.rewrite_expr(operand)),
                span: span.clone(),
            },
            Expr::At { base, index, span } => Expr::At {
                base: Box::new(self.rewrite_expr(base)),
                index: Box::new(self.rewrite_expr(index)),
                span: span.clone(),
            },
            Expr::Cast {
                ty,
                expr: inner,
                span,
            } => Expr::Cast {
                ty: ty.clone(),
                expr: Box::new(self.rewrite_expr(inner)),
                span: span.clone(),
            },
            Expr::SizeOf { ty, span } => Expr::SizeOf {
                ty: ty.clone(),
                span: span.clone(),
            },
            Expr::Some { value, span } => Expr::Some {
                value: Box::new(self.rewrite_expr(value)),
                span: span.clone(),
            },
            Expr::Ok { value, span } => Expr::Ok {
                value: Box::new(self.rewrite_expr(value)),
                span: span.clone(),
            },
            Expr::Err { value, span } => Expr::Err {
                value: Box::new(self.rewrite_expr(value)),
                span: span.clone(),
            },
            Expr::StructLit { name, fields, span } => Expr::StructLit {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|f| FieldInit {
                        name: f.name.clone(),
                        value: self.rewrite_expr(&f.value),
                        span: f.span.clone(),
                    })
                    .collect(),
                span: span.clone(),
            },
            Expr::Ident { .. }
            | Expr::IntLit { .. }
            | Expr::FloatLit { .. }
            | Expr::BoolLit { .. }
            | Expr::CStr { .. }
            | Expr::Bytes { .. }
            | Expr::None { .. } => e.clone(),
        }
    }
}

/// Specialize a method definition into a free function:
///
/// * Name becomes `Target_method`.
/// * `Self` in parameter types, return type, and body type annotations
///   becomes `Named(Target)`.
/// * Span is preserved so diagnostics still point at the original source.
fn lift_method(block: &ImplBlock, method: &FnDecl) -> FnDecl {
    FnDecl {
        is_unsafe: method.is_unsafe,
        name: format!("{}_{}", block.target, method.name),
        generics: method.generics.clone(),
        doc_comments: method.doc_comments.clone(),
        annotations: method.annotations.clone(),
        params: method
            .params
            .iter()
            .map(|p| Param {
                name: p.name.clone(),
                ty: subst_self(&p.ty, &block.target),
                span: p.span.clone(),
            })
            .collect(),
        return_type: subst_self(&method.return_type, &block.target),
        body: subst_self_in_block(&method.body, &block.target),
        span: method.span.clone(),
    }
}

/// Resolve an impl target name to a concrete `TypeExpr`. Names matching a
/// built-in primitive ("i32", "f64", "bool", …) become `TypeExpr::Primitive`;
/// everything else stays a `Named(target)`.
fn target_to_type(target: &str) -> TypeExpr {
    use crate::ast::PrimitiveType;
    let prim = match target {
        "i8" => Some(PrimitiveType::I8),
        "i16" => Some(PrimitiveType::I16),
        "i32" => Some(PrimitiveType::I32),
        "i64" => Some(PrimitiveType::I64),
        "u8" => Some(PrimitiveType::U8),
        "u16" => Some(PrimitiveType::U16),
        "u32" => Some(PrimitiveType::U32),
        "u64" => Some(PrimitiveType::U64),
        "f32" => Some(PrimitiveType::F32),
        "f64" => Some(PrimitiveType::F64),
        "bool" => Some(PrimitiveType::Bool),
        "usize" => Some(PrimitiveType::Usize),
        "isize" => Some(PrimitiveType::Isize),
        _ => None,
    };
    match prim {
        Some(p) => TypeExpr::Primitive(p),
        None => TypeExpr::Named(target.to_string()),
    }
}

/// Substitute every occurrence of the type `Self` with `Named(target)`
/// (or `Primitive(...)` when target names a built-in primitive type — see
/// `target_to_type`).
fn subst_self(ty: &TypeExpr, target: &str) -> TypeExpr {
    match ty {
        TypeExpr::Named(n) if n == "Self" => target_to_type(target),
        TypeExpr::NamedGeneric(n, args) => {
            let new_name = if n == "Self" {
                target.to_string()
            } else {
                n.clone()
            };
            TypeExpr::NamedGeneric(
                new_name,
                args.iter().map(|a| subst_self(a, target)).collect(),
            )
        }
        TypeExpr::Ref(t) => TypeExpr::Ref(Box::new(subst_self(t, target))),
        TypeExpr::Mref(t) => TypeExpr::Mref(Box::new(subst_self(t, target))),
        TypeExpr::Raw(t) => TypeExpr::Raw(Box::new(subst_self(t, target))),
        TypeExpr::Rawm(t) => TypeExpr::Rawm(Box::new(subst_self(t, target))),
        TypeExpr::Own(t) => TypeExpr::Own(Box::new(subst_self(t, target))),
        TypeExpr::Slice(t) => TypeExpr::Slice(Box::new(subst_self(t, target))),
        TypeExpr::Arr(t, n) => TypeExpr::Arr(Box::new(subst_self(t, target)), n.clone()),
        TypeExpr::Opt(t) => TypeExpr::Opt(Box::new(subst_self(t, target))),
        TypeExpr::Res(t, e) => TypeExpr::Res(
            Box::new(subst_self(t, target)),
            Box::new(subst_self(e, target)),
        ),
        TypeExpr::Fn {
            is_unsafe,
            params,
            ret,
        } => TypeExpr::Fn {
            is_unsafe: *is_unsafe,
            params: params.iter().map(|p| subst_self(p, target)).collect(),
            ret: Box::new(subst_self(ret, target)),
        },
        TypeExpr::Named(_) | TypeExpr::Primitive(_) | TypeExpr::Void => ty.clone(),
    }
}

// === Body walkers ===
//
// Self appears in body type annotations on `let x: Self = ...`, in casts,
// in `none(Self)` etc. Walk every type annotation and rewrite.

fn subst_self_in_block(block: &Block, target: &str) -> Block {
    Block {
        stmts: block
            .stmts
            .iter()
            .map(|s| subst_self_in_stmt(s, target))
            .collect(),
        span: block.span.clone(),
    }
}

fn subst_self_in_stmt(stmt: &Stmt, target: &str) -> Stmt {
    match stmt {
        Stmt::Let {
            name,
            ty,
            init,
            span,
        } => Stmt::Let {
            name: name.clone(),
            ty: subst_self(ty, target),
            init: subst_self_in_expr(init, target),
            span: span.clone(),
        },
        Stmt::Assign { lhs, rhs, span } => Stmt::Assign {
            lhs: subst_self_in_expr(lhs, target),
            rhs: subst_self_in_expr(rhs, target),
            span: span.clone(),
        },
        Stmt::If {
            cond,
            then_block,
            else_block,
            span,
        } => Stmt::If {
            cond: subst_self_in_expr(cond, target),
            then_block: subst_self_in_block(then_block, target),
            else_block: else_block.as_ref().map(|e| subst_self_in_else(e, target)),
            span: span.clone(),
        },
        Stmt::IfLet {
            name,
            expr,
            then_block,
            else_block,
            span,
        } => Stmt::IfLet {
            name: name.clone(),
            expr: subst_self_in_expr(expr, target),
            then_block: subst_self_in_block(then_block, target),
            else_block: else_block.as_ref().map(|b| subst_self_in_block(b, target)),
            span: span.clone(),
        },
        Stmt::While { cond, body, span } => Stmt::While {
            cond: subst_self_in_expr(cond, target),
            body: subst_self_in_block(body, target),
            span: span.clone(),
        },
        Stmt::For {
            init,
            cond,
            step,
            body,
            span,
        } => Stmt::For {
            init: init.as_ref().map(|i| subst_self_in_for_init(i, target)),
            cond: cond.as_ref().map(|c| subst_self_in_expr(c, target)),
            step: step.as_ref().map(|s| subst_self_in_for_step(s, target)),
            body: subst_self_in_block(body, target),
            span: span.clone(),
        },
        Stmt::Switch {
            expr,
            cases,
            default,
            span,
        } => Stmt::Switch {
            expr: subst_self_in_expr(expr, target),
            cases: cases
                .iter()
                .map(|c| crate::ast::Case {
                    value: c.value.clone(),
                    stmts: c
                        .stmts
                        .iter()
                        .map(|s| subst_self_in_stmt(s, target))
                        .collect(),
                    span: c.span.clone(),
                })
                .collect(),
            default: default.as_ref().map(|stmts| {
                stmts
                    .iter()
                    .map(|s| subst_self_in_stmt(s, target))
                    .collect()
            }),
            span: span.clone(),
        },
        Stmt::Return { value, span } => Stmt::Return {
            value: value.as_ref().map(|e| subst_self_in_expr(e, target)),
            span: span.clone(),
        },
        Stmt::Defer { body, span } => Stmt::Defer {
            body: subst_self_in_block(body, target),
            span: span.clone(),
        },
        Stmt::Unsafe { body, span } => Stmt::Unsafe {
            body: subst_self_in_block(body, target),
            span: span.clone(),
        },
        Stmt::Block(b) => Stmt::Block(subst_self_in_block(b, target)),
        Stmt::Expr { expr, span } => Stmt::Expr {
            expr: subst_self_in_expr(expr, target),
            span: span.clone(),
        },
        Stmt::Discard { expr, span } => Stmt::Discard {
            expr: subst_self_in_expr(expr, target),
            span: span.clone(),
        },
        Stmt::Break { .. } | Stmt::Continue { .. } => stmt.clone(),
    }
}

fn subst_self_in_else(br: &crate::ast::ElseBranch, target: &str) -> crate::ast::ElseBranch {
    match br {
        crate::ast::ElseBranch::ElseIf(s) => {
            crate::ast::ElseBranch::ElseIf(Box::new(subst_self_in_stmt(s, target)))
        }
        crate::ast::ElseBranch::Else(b) => {
            crate::ast::ElseBranch::Else(subst_self_in_block(b, target))
        }
    }
}

fn subst_self_in_for_init(fi: &ForInit, target: &str) -> ForInit {
    match fi {
        ForInit::Let { name, ty, init } => ForInit::Let {
            name: name.clone(),
            ty: subst_self(ty, target),
            init: subst_self_in_expr(init, target),
        },
        ForInit::Assign { lhs, rhs } => ForInit::Assign {
            lhs: subst_self_in_expr(lhs, target),
            rhs: subst_self_in_expr(rhs, target),
        },
        ForInit::Call(e) => ForInit::Call(subst_self_in_expr(e, target)),
    }
}

fn subst_self_in_for_step(fs: &ForStep, target: &str) -> ForStep {
    match fs {
        ForStep::Assign { lhs, rhs } => ForStep::Assign {
            lhs: subst_self_in_expr(lhs, target),
            rhs: subst_self_in_expr(rhs, target),
        },
        ForStep::Call(e) => ForStep::Call(subst_self_in_expr(e, target)),
    }
}

fn subst_self_in_expr(expr: &Expr, target: &str) -> Expr {
    match expr {
        Expr::Cast { ty, expr: e, span } => Expr::Cast {
            ty: subst_self(ty, target),
            expr: Box::new(subst_self_in_expr(e, target)),
            span: span.clone(),
        },
        Expr::SizeOf { ty, span } => Expr::SizeOf {
            ty: subst_self(ty, target),
            span: span.clone(),
        },
        Expr::None { ty, span } => Expr::None {
            ty: subst_self(ty, target),
            span: span.clone(),
        },
        Expr::Call { callee, args, span } => Expr::Call {
            callee: Box::new(subst_self_in_expr(callee, target)),
            args: args.iter().map(|a| subst_self_in_expr(a, target)).collect(),
            span: span.clone(),
        },
        Expr::Binary { op, lhs, rhs, span } => Expr::Binary {
            op: *op,
            lhs: Box::new(subst_self_in_expr(lhs, target)),
            rhs: Box::new(subst_self_in_expr(rhs, target)),
            span: span.clone(),
        },
        Expr::Unary { op, operand, span } => Expr::Unary {
            op: *op,
            operand: Box::new(subst_self_in_expr(operand, target)),
            span: span.clone(),
        },
        Expr::Paren { inner, span } => Expr::Paren {
            inner: Box::new(subst_self_in_expr(inner, target)),
            span: span.clone(),
        },
        Expr::Field { base, field, span } => Expr::Field {
            base: Box::new(subst_self_in_expr(base, target)),
            field: field.clone(),
            span: span.clone(),
        },
        Expr::Addr { operand, span } => Expr::Addr {
            operand: Box::new(subst_self_in_expr(operand, target)),
            span: span.clone(),
        },
        Expr::AddrM { operand, span } => Expr::AddrM {
            operand: Box::new(subst_self_in_expr(operand, target)),
            span: span.clone(),
        },
        Expr::Deref { operand, span } => Expr::Deref {
            operand: Box::new(subst_self_in_expr(operand, target)),
            span: span.clone(),
        },
        Expr::At { base, index, span } => Expr::At {
            base: Box::new(subst_self_in_expr(base, target)),
            index: Box::new(subst_self_in_expr(index, target)),
            span: span.clone(),
        },
        Expr::Some { value, span } => Expr::Some {
            value: Box::new(subst_self_in_expr(value, target)),
            span: span.clone(),
        },
        Expr::Ok { value, span } => Expr::Ok {
            value: Box::new(subst_self_in_expr(value, target)),
            span: span.clone(),
        },
        Expr::Err { value, span } => Expr::Err {
            value: Box::new(subst_self_in_expr(value, target)),
            span: span.clone(),
        },
        Expr::StructLit { name, fields, span } => {
            let new_name = if name == "Self" {
                target.to_string()
            } else {
                name.clone()
            };
            Expr::StructLit {
                name: new_name,
                fields: fields
                    .iter()
                    .map(|f| FieldInit {
                        name: f.name.clone(),
                        value: subst_self_in_expr(&f.value, target),
                        span: f.span.clone(),
                    })
                    .collect(),
                span: span.clone(),
            }
        }
        Expr::Ident { .. }
        | Expr::IntLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::CStr { .. }
        | Expr::Bytes { .. } => expr.clone(),
        // `Self`-substitution only walks impl-method bodies; closures
        // there are already lifted by ClosureLifter before this pass
        // runs over the original Impl items. We never see one.
        Expr::Closure { .. } => unreachable!("Closure should have been lifted by ClosureLifter"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Span;

    fn mk_span() -> Span {
        0..0
    }

    #[test]
    fn lifts_method_to_freestanding_fn() {
        let block = ImplBlock {
            target: "Point".to_string(),
            trait_name: None,
            methods: vec![FnDecl {
                is_unsafe: false,
                name: "x_value".to_string(),
                generics: vec![],
                doc_comments: vec![],
                annotations: vec![],
                params: vec![Param {
                    name: "self".to_string(),
                    ty: TypeExpr::Ref(Box::new(TypeExpr::Named("Self".to_string()))),
                    span: mk_span(),
                }],
                return_type: TypeExpr::Primitive(crate::ast::PrimitiveType::I32),
                body: Block {
                    stmts: vec![],
                    span: mk_span(),
                },
                span: mk_span(),
            }],
            span: mk_span(),
            doc_comments: vec![],
        };
        let file = File {
            items: vec![Item::Impl(block)],
        };
        let out = desugar(&file);
        // Impl block survives + one lifted free fn.
        assert_eq!(out.items.len(), 2);
        // The lifted fn comes after the impl block.
        match &out.items[1] {
            Item::Fn(f) => {
                assert_eq!(f.name, "Point_x_value");
                match &f.params[0].ty {
                    TypeExpr::Ref(inner) => assert_eq!(**inner, TypeExpr::Named("Point".into())),
                    other => panic!("unexpected param type: {:?}", other),
                }
            }
            other => panic!("expected Item::Fn, got {:?}", other),
        }
        // Impl block itself preserved for the trait-impl table.
        assert!(matches!(&out.items[0], Item::Impl(_)));
    }

    #[test]
    fn passes_through_non_impl_items() {
        let file = File {
            items: vec![Item::Fn(FnDecl {
                is_unsafe: false,
                name: "untouched".to_string(),
                generics: vec![],
                doc_comments: vec![],
                annotations: vec![],
                params: vec![],
                return_type: TypeExpr::Void,
                body: Block {
                    stmts: vec![],
                    span: mk_span(),
                },
                span: mk_span(),
            })],
        };
        let out = desugar(&file);
        assert_eq!(out.items.len(), 1);
        if let Item::Fn(f) = &out.items[0] {
            assert_eq!(f.name, "untouched");
        } else {
            panic!("expected Item::Fn");
        }
    }
}
