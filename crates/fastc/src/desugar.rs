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
    for item in &file.items {
        match item {
            Item::Impl(block) => {
                items.push(item.clone());
                for method in &block.methods {
                    items.push(Item::Fn(lift_method(block, method)));
                }
            }
            _ => items.push(item.clone()),
        }
    }
    File { items }
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

/// Substitute every occurrence of the type `Self` with `Named(target)`.
fn subst_self(ty: &TypeExpr, target: &str) -> TypeExpr {
    match ty {
        TypeExpr::Named(n) if n == "Self" => TypeExpr::Named(target.to_string()),
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
