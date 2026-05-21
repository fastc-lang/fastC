//! Monomorphization pass.
//!
//! Runs between typecheck and lower. Walks the AST collecting every call to
//! a generic function, infers concrete type arguments from the actual call
//! arguments, deduplicates by (fn_name, [type_args]) tuple, and emits a
//! specialized non-generic copy per instantiation. Generic-fn declarations
//! are removed from the output; call sites are rewritten to invoke the
//! mangled specialized name.
//!
//! Stage 0.9 v1 scope:
//!
//! - Generic *functions* only. Generic structs/enums are reserved for the
//!   next slice.
//! - No constraints (`T: Ord` etc) — those are stage 1.0 (traits).
//! - Type-argument inference only at call sites; no explicit `id[i32](x)`
//!   call-site syntax in v1.

use std::collections::{HashMap, HashSet};

use crate::ast::{
    Block, Case, ElseBranch, Expr, FieldInit, File, FnDecl, ForInit, ForStep, Item, Param,
    PrimitiveType, Stmt, StructDecl, TypeExpr, TypeParam,
};
use crate::diag::CompileError;
use crate::resolve::SymbolTable;
use crate::typecheck::{substitute, unify_generic};

/// Run the monomorphization pass. Returns a new `File` with generic fns
/// replaced by their concrete instantiations and all generic call sites
/// rewritten to the mangled names.
///
/// Errors when a generic instantiation does not satisfy its declared trait
/// bounds — e.g. `fn max[T: Ord](...)` is called with `T = i32` but no
/// `impl Ord for i32` exists.
pub fn monomorphize(
    file: &File,
    symbols: &SymbolTable,
    source: &str,
) -> Result<File, CompileError> {
    let mut ctx = MonoCtx::new(file, symbols, source);

    // Pass 1: collect every generic call reachable from non-generic
    // code, including non-generic functions nested inside `mod` bodies
    // (the stdlib `mod str` is the canonical case — its concrete
    // wrapper functions call into generic `vec::*` and need to drive
    // T-inference even though the str fns themselves aren't generic).
    fn collect_recursive(ctx: &mut MonoCtx<'_>, items: &[Item]) {
        for item in items {
            match item {
                Item::Fn(f) if f.generics.is_empty() => {
                    let mut env: HashMap<String, TypeExpr> = HashMap::new();
                    for p in &f.params {
                        env.insert(p.name.clone(), p.ty.clone());
                    }
                    ctx.collect_in_block(&f.body, &HashMap::new(), &mut env);
                }
                Item::Mod(m) => {
                    if let Some(body) = &m.body {
                        collect_recursive(ctx, body);
                    }
                }
                _ => {}
            }
        }
    }
    collect_recursive(&mut ctx, &file.items);

    // Pass 1b: transitive closure — a generic body may call further generics.
    while let Some((fn_name, type_args)) = ctx.worklist.pop() {
        if let Some(generic_fn) = ctx.generic_fns.get(&fn_name).cloned() {
            let subst = build_subst(&generic_fn.generics, &type_args);
            let mut env: HashMap<String, TypeExpr> = HashMap::new();
            for p in &generic_fn.params {
                env.insert(p.name.clone(), substitute(&p.ty, &subst));
            }
            ctx.collect_in_block(&generic_fn.body, &subst, &mut env);
        }
    }

    // Pass 2: emit a new File. Non-generic items pass through with call
    // sites rewritten. Generic declarations are dropped; their
    // instantiations are appended in deterministic order. Mod bodies are
    // filtered the same way so generic fns inside the prelude's `mod math`
    // don't leak through to lower as literal-`T` declarations.
    let mut new_items: Vec<Item> = Vec::with_capacity(file.items.len());
    for item in &file.items {
        match item {
            Item::Fn(f) if !f.generics.is_empty() => {
                // Drop the generic declaration; instantiations follow.
            }
            Item::Fn(f) => {
                new_items.push(Item::Fn(rewrite_fn(f, &HashMap::new(), &ctx)));
            }
            Item::Mod(m) => {
                new_items.push(Item::Mod(strip_generic_fns_from_mod(m, &ctx)));
            }
            _ => new_items.push(item.clone()),
        }
    }

    // Deterministic emission order: by mangled name.
    let mut entries: Vec<(String, (String, Vec<TypeExpr>))> = ctx
        .instantiations
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let pseudo_ctx = MonoCtx {
        generic_fns: ctx.generic_fns.clone(),
        generic_structs: ctx.generic_structs.clone(),
        all_structs: ctx.all_structs.clone(),
        all_fns: ctx.all_fns.clone(),
        symbols,
        source: ctx.source,
        trait_impls: ctx.trait_impls.clone(),
        instantiations: ctx.instantiations.clone(),
        struct_instantiations: ctx.struct_instantiations.clone(),
        worklist: Vec::new(),
        errors: Vec::new(),
    };
    for (mangled, (fn_name, type_args)) in entries {
        if let Some(generic_fn) = pseudo_ctx.generic_fns.get(&fn_name).cloned() {
            let subst = build_subst(&generic_fn.generics, &type_args);
            new_items.push(Item::Fn(specialize_fn(
                &generic_fn,
                &mangled,
                &subst,
                &pseudo_ctx,
            )));
        }
    }

    if !ctx.errors.is_empty() {
        return Err(CompileError::multiple(ctx.errors));
    }

    // Final pass: specialize generic structs. Walk `new_items`, mangle
    // every `NamedGeneric(...)` type reference to `Named(<mangled>)`,
    // rewrite `Pair { ... }` struct-literal names when the typecheck
    // inferred the struct as generic, drop the original generic-struct
    // declarations, and emit one specialized struct decl per instantiation.
    let post = run_struct_mono(File { items: new_items }, &ctx.generic_structs);
    Ok(post)
}

/// Post-fn-mono pass that specializes generic structs. Walks the file,
/// collecting `NamedGeneric` instantiations into `struct_insts`, rewriting
/// every type and struct-literal name to the mangled concrete form,
/// dropping the original generic struct declarations, and appending one
/// `Item::Struct` per unique instantiation.
fn run_struct_mono(file: File, generic_structs: &HashMap<String, StructDecl>) -> File {
    use crate::ast::Field;

    if generic_structs.is_empty() {
        return file;
    }

    let mut struct_insts: HashMap<String, (String, Vec<TypeExpr>)> = HashMap::new();

    // Pass A: collect instantiations and rewrite every type-bearing site.
    let rewritten: Vec<Item> = file
        .items
        .into_iter()
        .filter_map(|item| match item {
            Item::Struct(s) if !s.generics.is_empty() => None,
            Item::Fn(f) => Some(Item::Fn(rewrite_fn_struct_refs(
                f,
                generic_structs,
                &mut struct_insts,
            ))),
            Item::Struct(s) => Some(Item::Struct(StructDecl {
                repr: s.repr,
                name: s.name,
                generics: s.generics,
                fields: s
                    .fields
                    .into_iter()
                    .map(|f| Field {
                        name: f.name,
                        ty: mangle_type_refs(&f.ty, generic_structs, &mut struct_insts),
                        span: f.span,
                    })
                    .collect(),
                span: s.span,
                doc_comments: s.doc_comments,
            })),
            Item::Mod(m) => Some(Item::Mod(rewrite_mod_struct_refs(
                m,
                generic_structs,
                &mut struct_insts,
            ))),
            other => Some(other),
        })
        .collect();

    // Pass B: emit one specialized struct per unique instantiation, in
    // deterministic mangled-name order. Run mangle_type_refs again on
    // each specialized field type so nested generics (Pair[Vec[i32], _])
    // resolve all the way down.
    let mut emit_order: Vec<(String, (String, Vec<TypeExpr>))> = struct_insts
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    emit_order.sort_by(|a, b| a.0.cmp(&b.0));

    let mut nested_insts: HashMap<String, (String, Vec<TypeExpr>)> = struct_insts.clone();
    let mut specialized: Vec<Item> = Vec::with_capacity(emit_order.len());
    for (mangled, (orig_name, type_args)) in emit_order {
        if let Some(decl) = generic_structs.get(&orig_name) {
            let subst: HashMap<String, TypeExpr> = decl
                .generics
                .iter()
                .zip(type_args.iter())
                .map(|(p, a)| (p.name.clone(), a.clone()))
                .collect();
            let new_fields: Vec<Field> = decl
                .fields
                .iter()
                .map(|f| {
                    let after_subst = substitute(&f.ty, &subst);
                    Field {
                        name: f.name.clone(),
                        ty: mangle_type_refs(&after_subst, generic_structs, &mut nested_insts),
                        span: f.span.clone(),
                    }
                })
                .collect();
            specialized.push(Item::Struct(StructDecl {
                repr: decl.repr.clone(),
                name: mangled,
                generics: Vec::new(),
                fields: new_fields,
                span: decl.span.clone(),
                doc_comments: decl.doc_comments.clone(),
            }));
        }
    }

    // If specializing introduced *new* nested instantiations, do a single
    // additional round. For the v1 stdlib (Pair, vec-style) one extra
    // round suffices; deeper nesting would need a fixpoint loop. Punt for
    // now and emit any newly-discovered instantiations as well.
    for (mangled, (orig_name, type_args)) in &nested_insts {
        if struct_insts.contains_key(mangled) {
            continue;
        }
        if let Some(decl) = generic_structs.get(orig_name) {
            let subst: HashMap<String, TypeExpr> = decl
                .generics
                .iter()
                .zip(type_args.iter())
                .map(|(p, a)| (p.name.clone(), a.clone()))
                .collect();
            let new_fields: Vec<Field> = decl
                .fields
                .iter()
                .map(|f| {
                    let after_subst = substitute(&f.ty, &subst);
                    let mut sink: HashMap<String, (String, Vec<TypeExpr>)> = HashMap::new();
                    Field {
                        name: f.name.clone(),
                        ty: mangle_type_refs(&after_subst, generic_structs, &mut sink),
                        span: f.span.clone(),
                    }
                })
                .collect();
            specialized.push(Item::Struct(StructDecl {
                repr: decl.repr.clone(),
                name: mangled.clone(),
                generics: Vec::new(),
                fields: new_fields,
                span: decl.span.clone(),
                doc_comments: decl.doc_comments.clone(),
            }));
        }
    }

    // Emit specialized struct typedefs *before* the user's other items
    // so any non-generic struct that holds a specialized one as a field
    // (e.g. `struct Str { data: Vec[u8] }` -> `Str { Vec_u8 data; }`)
    // sees its dependency declared first. Within the specialized group
    // we keep mangled-name order, which is the longest-existing
    // deterministic key.
    let mut out_items: Vec<Item> = Vec::with_capacity(rewritten.len() + specialized.len());
    out_items.extend(specialized);
    out_items.extend(rewritten);
    File { items: out_items }
}

/// Walk an FnDecl's type-bearing positions (params, return type, body type
/// annotations, and cast/struct-lit expressions) and mangle every generic
/// struct reference.
fn rewrite_fn_struct_refs(
    f: FnDecl,
    generic_structs: &HashMap<String, StructDecl>,
    struct_insts: &mut HashMap<String, (String, Vec<TypeExpr>)>,
) -> FnDecl {
    FnDecl {
        is_unsafe: f.is_unsafe,
        name: f.name,
        generics: f.generics,
        doc_comments: f.doc_comments,
        params: f
            .params
            .into_iter()
            .map(|p| Param {
                name: p.name,
                ty: mangle_type_refs(&p.ty, generic_structs, struct_insts),
                span: p.span,
            })
            .collect(),
        return_type: mangle_type_refs(&f.return_type, generic_structs, struct_insts),
        body: mangle_block_struct_refs(f.body, generic_structs, struct_insts),
        span: f.span,
    }
}

fn rewrite_mod_struct_refs(
    m: crate::ast::ModDecl,
    generic_structs: &HashMap<String, StructDecl>,
    struct_insts: &mut HashMap<String, (String, Vec<TypeExpr>)>,
) -> crate::ast::ModDecl {
    crate::ast::ModDecl {
        is_pub: m.is_pub,
        name: m.name,
        body: m.body.map(|items| {
            items
                .into_iter()
                .filter_map(|item| match item {
                    Item::Struct(s) if !s.generics.is_empty() => None,
                    Item::Fn(f) => Some(Item::Fn(rewrite_fn_struct_refs(
                        f,
                        generic_structs,
                        struct_insts,
                    ))),
                    Item::Mod(inner) => Some(Item::Mod(rewrite_mod_struct_refs(
                        inner,
                        generic_structs,
                        struct_insts,
                    ))),
                    other => Some(other),
                })
                .collect()
        }),
        span: m.span,
    }
}

fn mangle_block_struct_refs(
    block: Block,
    generic_structs: &HashMap<String, StructDecl>,
    struct_insts: &mut HashMap<String, (String, Vec<TypeExpr>)>,
) -> Block {
    Block {
        stmts: block
            .stmts
            .into_iter()
            .map(|s| mangle_stmt_struct_refs(s, generic_structs, struct_insts))
            .collect(),
        span: block.span,
    }
}

fn mangle_stmt_struct_refs(
    stmt: Stmt,
    generic_structs: &HashMap<String, StructDecl>,
    struct_insts: &mut HashMap<String, (String, Vec<TypeExpr>)>,
) -> Stmt {
    match stmt {
        Stmt::Let {
            name,
            ty,
            init,
            span,
        } => Stmt::Let {
            name,
            ty: mangle_type_refs(&ty, generic_structs, struct_insts),
            init: mangle_expr_struct_refs(init, generic_structs, struct_insts),
            span,
        },
        Stmt::Assign { lhs, rhs, span } => Stmt::Assign {
            lhs: mangle_expr_struct_refs(lhs, generic_structs, struct_insts),
            rhs: mangle_expr_struct_refs(rhs, generic_structs, struct_insts),
            span,
        },
        Stmt::If {
            cond,
            then_block,
            else_block,
            span,
        } => Stmt::If {
            cond: mangle_expr_struct_refs(cond, generic_structs, struct_insts),
            then_block: mangle_block_struct_refs(then_block, generic_structs, struct_insts),
            else_block: else_block.map(|e| match e {
                ElseBranch::ElseIf(s) => ElseBranch::ElseIf(Box::new(mangle_stmt_struct_refs(
                    *s,
                    generic_structs,
                    struct_insts,
                ))),
                ElseBranch::Else(b) => {
                    ElseBranch::Else(mangle_block_struct_refs(b, generic_structs, struct_insts))
                }
            }),
            span,
        },
        Stmt::IfLet {
            name,
            expr,
            then_block,
            else_block,
            span,
        } => Stmt::IfLet {
            name,
            expr: mangle_expr_struct_refs(expr, generic_structs, struct_insts),
            then_block: mangle_block_struct_refs(then_block, generic_structs, struct_insts),
            else_block: else_block
                .map(|b| mangle_block_struct_refs(b, generic_structs, struct_insts)),
            span,
        },
        Stmt::While { cond, body, span } => Stmt::While {
            cond: mangle_expr_struct_refs(cond, generic_structs, struct_insts),
            body: mangle_block_struct_refs(body, generic_structs, struct_insts),
            span,
        },
        Stmt::For {
            init,
            cond,
            step,
            body,
            span,
        } => Stmt::For {
            init: init.map(|i| match i {
                ForInit::Let { name, ty, init } => ForInit::Let {
                    name,
                    ty: mangle_type_refs(&ty, generic_structs, struct_insts),
                    init: mangle_expr_struct_refs(init, generic_structs, struct_insts),
                },
                ForInit::Assign { lhs, rhs } => ForInit::Assign {
                    lhs: mangle_expr_struct_refs(lhs, generic_structs, struct_insts),
                    rhs: mangle_expr_struct_refs(rhs, generic_structs, struct_insts),
                },
                ForInit::Call(e) => {
                    ForInit::Call(mangle_expr_struct_refs(e, generic_structs, struct_insts))
                }
            }),
            cond: cond.map(|c| mangle_expr_struct_refs(c, generic_structs, struct_insts)),
            step: step.map(|s| match s {
                ForStep::Assign { lhs, rhs } => ForStep::Assign {
                    lhs: mangle_expr_struct_refs(lhs, generic_structs, struct_insts),
                    rhs: mangle_expr_struct_refs(rhs, generic_structs, struct_insts),
                },
                ForStep::Call(e) => {
                    ForStep::Call(mangle_expr_struct_refs(e, generic_structs, struct_insts))
                }
            }),
            body: mangle_block_struct_refs(body, generic_structs, struct_insts),
            span,
        },
        Stmt::Switch {
            expr,
            cases,
            default,
            span,
        } => Stmt::Switch {
            expr: mangle_expr_struct_refs(expr, generic_structs, struct_insts),
            cases: cases
                .into_iter()
                .map(|c| Case {
                    value: c.value,
                    stmts: c
                        .stmts
                        .into_iter()
                        .map(|s| mangle_stmt_struct_refs(s, generic_structs, struct_insts))
                        .collect(),
                    span: c.span,
                })
                .collect(),
            default: default.map(|stmts| {
                stmts
                    .into_iter()
                    .map(|s| mangle_stmt_struct_refs(s, generic_structs, struct_insts))
                    .collect()
            }),
            span,
        },
        Stmt::Return { value, span } => Stmt::Return {
            value: value.map(|e| mangle_expr_struct_refs(e, generic_structs, struct_insts)),
            span,
        },
        Stmt::Defer { body, span } => Stmt::Defer {
            body: mangle_block_struct_refs(body, generic_structs, struct_insts),
            span,
        },
        Stmt::Unsafe { body, span } => Stmt::Unsafe {
            body: mangle_block_struct_refs(body, generic_structs, struct_insts),
            span,
        },
        Stmt::Block(b) => Stmt::Block(mangle_block_struct_refs(b, generic_structs, struct_insts)),
        Stmt::Expr { expr, span } => Stmt::Expr {
            expr: mangle_expr_struct_refs(expr, generic_structs, struct_insts),
            span,
        },
        Stmt::Discard { expr, span } => Stmt::Discard {
            expr: mangle_expr_struct_refs(expr, generic_structs, struct_insts),
            span,
        },
        Stmt::Break { .. } | Stmt::Continue { .. } => stmt,
    }
}

fn mangle_expr_struct_refs(
    expr: Expr,
    generic_structs: &HashMap<String, StructDecl>,
    struct_insts: &mut HashMap<String, (String, Vec<TypeExpr>)>,
) -> Expr {
    match expr {
        Expr::StructLit { name, fields, span } => {
            // If the struct is generic, infer type args from the field
            // values and rewrite the literal's name to the mangled form.
            let rewritten_fields: Vec<FieldInit> = fields
                .into_iter()
                .map(|f| FieldInit {
                    name: f.name,
                    value: mangle_expr_struct_refs(f.value, generic_structs, struct_insts),
                    span: f.span,
                })
                .collect();
            if let Some(decl) = generic_structs.get(&name) {
                let type_params: Vec<String> =
                    decl.generics.iter().map(|p| p.name.clone()).collect();
                let mut subst: HashMap<String, TypeExpr> = HashMap::new();
                for fld in &rewritten_fields {
                    if let Some(decl_field) = decl.fields.iter().find(|d| d.name == fld.name) {
                        let arg_ty = approx_field_type(&fld.value);
                        unify_generic(&decl_field.ty, &arg_ty, &type_params, &mut subst);
                    }
                }
                let type_args: Vec<TypeExpr> = type_params
                    .iter()
                    .map(|p| {
                        subst
                            .get(p)
                            .cloned()
                            .unwrap_or_else(|| TypeExpr::Named(p.clone()))
                    })
                    .collect();
                let mangled = mangled_name(&name, &type_args);
                struct_insts
                    .entry(mangled.clone())
                    .or_insert((name.clone(), type_args));
                Expr::StructLit {
                    name: mangled,
                    fields: rewritten_fields,
                    span,
                }
            } else {
                Expr::StructLit {
                    name,
                    fields: rewritten_fields,
                    span,
                }
            }
        }
        Expr::Cast { ty, expr: e, span } => Expr::Cast {
            ty: mangle_type_refs(&ty, generic_structs, struct_insts),
            expr: Box::new(mangle_expr_struct_refs(*e, generic_structs, struct_insts)),
            span,
        },
        Expr::SizeOf { ty, span } => Expr::SizeOf {
            ty: mangle_type_refs(&ty, generic_structs, struct_insts),
            span,
        },
        Expr::None { ty, span } => Expr::None {
            ty: mangle_type_refs(&ty, generic_structs, struct_insts),
            span,
        },
        Expr::Call { callee, args, span } => Expr::Call {
            callee: Box::new(mangle_expr_struct_refs(
                *callee,
                generic_structs,
                struct_insts,
            )),
            args: args
                .into_iter()
                .map(|a| mangle_expr_struct_refs(a, generic_structs, struct_insts))
                .collect(),
            span,
        },
        Expr::Binary { op, lhs, rhs, span } => Expr::Binary {
            op,
            lhs: Box::new(mangle_expr_struct_refs(*lhs, generic_structs, struct_insts)),
            rhs: Box::new(mangle_expr_struct_refs(*rhs, generic_structs, struct_insts)),
            span,
        },
        Expr::Unary { op, operand, span } => Expr::Unary {
            op,
            operand: Box::new(mangle_expr_struct_refs(
                *operand,
                generic_structs,
                struct_insts,
            )),
            span,
        },
        Expr::Paren { inner, span } => Expr::Paren {
            inner: Box::new(mangle_expr_struct_refs(
                *inner,
                generic_structs,
                struct_insts,
            )),
            span,
        },
        Expr::Field { base, field, span } => Expr::Field {
            base: Box::new(mangle_expr_struct_refs(
                *base,
                generic_structs,
                struct_insts,
            )),
            field,
            span,
        },
        Expr::Addr { operand, span } => Expr::Addr {
            operand: Box::new(mangle_expr_struct_refs(
                *operand,
                generic_structs,
                struct_insts,
            )),
            span,
        },
        Expr::AddrM { operand, span } => Expr::AddrM {
            operand: Box::new(mangle_expr_struct_refs(
                *operand,
                generic_structs,
                struct_insts,
            )),
            span,
        },
        Expr::Deref { operand, span } => Expr::Deref {
            operand: Box::new(mangle_expr_struct_refs(
                *operand,
                generic_structs,
                struct_insts,
            )),
            span,
        },
        Expr::At { base, index, span } => Expr::At {
            base: Box::new(mangle_expr_struct_refs(
                *base,
                generic_structs,
                struct_insts,
            )),
            index: Box::new(mangle_expr_struct_refs(
                *index,
                generic_structs,
                struct_insts,
            )),
            span,
        },
        Expr::Some { value, span } => Expr::Some {
            value: Box::new(mangle_expr_struct_refs(
                *value,
                generic_structs,
                struct_insts,
            )),
            span,
        },
        Expr::Ok { value, span } => Expr::Ok {
            value: Box::new(mangle_expr_struct_refs(
                *value,
                generic_structs,
                struct_insts,
            )),
            span,
        },
        Expr::Err { value, span } => Expr::Err {
            value: Box::new(mangle_expr_struct_refs(
                *value,
                generic_structs,
                struct_insts,
            )),
            span,
        },
        other => other,
    }
}

/// Best-effort type-of for a struct-literal field value, used to infer
/// type-args. Only handles the leaf cases that show up in v1 stdlib
/// initializers; deeper expressions are typed at typecheck time and
/// don't need to be re-inferred here.
fn approx_field_type(expr: &Expr) -> TypeExpr {
    match expr {
        Expr::IntLit { .. } => TypeExpr::Primitive(PrimitiveType::I32),
        Expr::FloatLit { .. } => TypeExpr::Primitive(PrimitiveType::F64),
        Expr::BoolLit { .. } => TypeExpr::Primitive(PrimitiveType::Bool),
        Expr::Cast { ty, .. } => ty.clone(),
        Expr::Paren { inner, .. } => approx_field_type(inner),
        _ => TypeExpr::Void,
    }
}

/// Walking context for monomorphization.
struct MonoCtx<'a> {
    /// All generic-fn declarations in the program, keyed by name.
    generic_fns: HashMap<String, FnDecl>,
    /// All generic struct declarations, keyed by name.
    generic_structs: HashMap<String, StructDecl>,
    /// Every struct declaration (generic + non-generic), keyed by name.
    /// Used by `approx_expr_type`'s Field handler to look up the type of
    /// a struct field — required when a generic call site's argument is
    /// `addr(struct.field)` and we need to recover the field type to
    /// drive type-argument inference.
    all_structs: HashMap<String, StructDecl>,
    /// Every fn declaration (including non-generic), keyed by bare name.
    /// Used by `approx_expr_type`'s Ident handler to recover the
    /// `fn(P) -> R` type when an identifier passed to a higher-order
    /// generic call is a free function (e.g. `vec::map(addr(v), double)`).
    all_fns: HashMap<String, FnDecl>,
    /// Symbol table used to identify generic call sites by name lookup.
    /// Currently unused — callee resolution looks up `generic_fns`
    /// directly so mod-internal calls work, but kept for future passes
    /// that need scope-aware resolution (e.g. capability checks).
    #[allow(dead_code)]
    symbols: &'a SymbolTable,
    /// Source text — passed to error constructors so spans render properly.
    source: &'a str,
    /// `(type_name → set of traits implemented)`. Built by walking
    /// `Item::Impl` entries that name a trait. Used to verify bound
    /// satisfaction when specializing generic instantiations.
    trait_impls: HashMap<String, HashSet<String>>,
    /// Known fn instantiations keyed by mangled name. The value records the
    /// original (fn_name, type_args) tuple so pass 2 can specialize the body.
    /// Mangled-name keying side-steps `Vec<TypeExpr>: !Hash`.
    instantiations: HashMap<String, (String, Vec<TypeExpr>)>,
    /// Known struct instantiations keyed by mangled name (`Pair_i32_bool`).
    /// Values map back to (struct_name, type_args).
    struct_instantiations: HashMap<String, (String, Vec<TypeExpr>)>,
    /// Worklist for transitive fn instantiation discovery.
    worklist: Vec<(String, Vec<TypeExpr>)>,
    /// Bound-satisfaction errors accumulated during the collect pass.
    errors: Vec<CompileError>,
}

/// Rewrite a `TypeExpr` so every `NamedGeneric(name, args)` referring to a
/// generic struct in `generic_structs` becomes `Named(mangled)`. Also
/// registers each instantiation in `struct_insts` so pass 2 can emit a
/// specialized struct definition.
fn mangle_type_refs(
    ty: &TypeExpr,
    generic_structs: &HashMap<String, StructDecl>,
    struct_insts: &mut HashMap<String, (String, Vec<TypeExpr>)>,
) -> TypeExpr {
    match ty {
        TypeExpr::NamedGeneric(name, args) => {
            // First, recurse into args so nested generics are mangled too.
            let new_args: Vec<TypeExpr> = args
                .iter()
                .map(|a| mangle_type_refs(a, generic_structs, struct_insts))
                .collect();
            if generic_structs.contains_key(name) {
                let mangled = mangled_name(name, &new_args);
                struct_insts
                    .entry(mangled.clone())
                    .or_insert((name.clone(), new_args));
                TypeExpr::Named(mangled)
            } else {
                TypeExpr::NamedGeneric(name.clone(), new_args)
            }
        }
        TypeExpr::Ref(inner) => TypeExpr::Ref(Box::new(mangle_type_refs(
            inner,
            generic_structs,
            struct_insts,
        ))),
        TypeExpr::Mref(inner) => TypeExpr::Mref(Box::new(mangle_type_refs(
            inner,
            generic_structs,
            struct_insts,
        ))),
        TypeExpr::Raw(inner) => TypeExpr::Raw(Box::new(mangle_type_refs(
            inner,
            generic_structs,
            struct_insts,
        ))),
        TypeExpr::Rawm(inner) => TypeExpr::Rawm(Box::new(mangle_type_refs(
            inner,
            generic_structs,
            struct_insts,
        ))),
        TypeExpr::Own(inner) => TypeExpr::Own(Box::new(mangle_type_refs(
            inner,
            generic_structs,
            struct_insts,
        ))),
        TypeExpr::Slice(inner) => TypeExpr::Slice(Box::new(mangle_type_refs(
            inner,
            generic_structs,
            struct_insts,
        ))),
        TypeExpr::Arr(inner, n) => TypeExpr::Arr(
            Box::new(mangle_type_refs(inner, generic_structs, struct_insts)),
            n.clone(),
        ),
        TypeExpr::Opt(inner) => TypeExpr::Opt(Box::new(mangle_type_refs(
            inner,
            generic_structs,
            struct_insts,
        ))),
        TypeExpr::Res(a, b) => TypeExpr::Res(
            Box::new(mangle_type_refs(a, generic_structs, struct_insts)),
            Box::new(mangle_type_refs(b, generic_structs, struct_insts)),
        ),
        TypeExpr::Fn {
            is_unsafe,
            params,
            ret,
        } => TypeExpr::Fn {
            is_unsafe: *is_unsafe,
            params: params
                .iter()
                .map(|p| mangle_type_refs(p, generic_structs, struct_insts))
                .collect(),
            ret: Box::new(mangle_type_refs(ret, generic_structs, struct_insts)),
        },
        TypeExpr::Named(_) | TypeExpr::Primitive(_) | TypeExpr::Void => ty.clone(),
    }
}

/// Filter generic functions out of a module's body. Specialized
/// instantiations are appended at the top level by mono's pass 2;
/// leaving the generic declarations inside the module would cause lower
/// to emit them with literal `T` types, breaking the C output.
///
/// Non-generic items inside the mod are preserved, but their bodies are
/// rewritten so any calls to generic helpers point at the specialized
/// mangled name (e.g. `vec::push` -> `push_u8`). Nested mods recurse.
fn strip_generic_fns_from_mod(m: &crate::ast::ModDecl, ctx: &MonoCtx) -> crate::ast::ModDecl {
    let new_body = m.body.as_ref().map(|items| {
        items
            .iter()
            .filter_map(|item| match item {
                Item::Fn(f) if !f.generics.is_empty() => None,
                Item::Fn(f) => Some(Item::Fn(rewrite_fn(f, &HashMap::new(), ctx))),
                Item::Mod(inner) => Some(Item::Mod(strip_generic_fns_from_mod(inner, ctx))),
                _ => Some(item.clone()),
            })
            .collect()
    });
    crate::ast::ModDecl {
        is_pub: m.is_pub,
        name: m.name.clone(),
        body: new_body,
        span: m.span.clone(),
    }
}

/// Walk `items`, plus the bodies of any inline `Item::Mod`, gathering
/// generic function declarations, generic struct declarations, and
/// trait-impl mappings into the caller's hashmaps. Recursive so generics
/// declared inside a stdlib module (the prelude's `mod math` is the
/// canonical case) are visible to mono.
fn collect_items_recursive(
    items: &[Item],
    generic_fns: &mut HashMap<String, FnDecl>,
    generic_structs: &mut HashMap<String, StructDecl>,
    all_structs: &mut HashMap<String, StructDecl>,
    all_fns: &mut HashMap<String, FnDecl>,
    trait_impls: &mut HashMap<String, HashSet<String>>,
) {
    for item in items {
        match item {
            Item::Fn(f) => {
                if !f.generics.is_empty() {
                    generic_fns.insert(f.name.clone(), f.clone());
                }
                all_fns.insert(f.name.clone(), f.clone());
            }
            Item::Struct(s) => {
                if !s.generics.is_empty() {
                    generic_structs.insert(s.name.clone(), s.clone());
                }
                all_structs.insert(s.name.clone(), s.clone());
            }
            Item::Impl(block) => {
                if let Some(trait_name) = &block.trait_name {
                    trait_impls
                        .entry(block.target.clone())
                        .or_default()
                        .insert(trait_name.clone());
                }
            }
            Item::Mod(m) => {
                if let Some(body) = &m.body {
                    collect_items_recursive(
                        body,
                        generic_fns,
                        generic_structs,
                        all_structs,
                        all_fns,
                        trait_impls,
                    );
                }
            }
            _ => {}
        }
    }
}

impl<'a> MonoCtx<'a> {
    fn new(file: &'a File, symbols: &'a SymbolTable, source: &'a str) -> Self {
        let mut generic_fns = HashMap::new();
        let mut generic_structs = HashMap::new();
        let mut all_structs = HashMap::new();
        let mut all_fns = HashMap::new();
        let mut trait_impls: HashMap<String, HashSet<String>> = HashMap::new();
        collect_items_recursive(
            &file.items,
            &mut generic_fns,
            &mut generic_structs,
            &mut all_structs,
            &mut all_fns,
            &mut trait_impls,
        );
        Self {
            generic_fns,
            generic_structs,
            all_structs,
            all_fns,
            symbols,
            source,
            trait_impls,
            instantiations: HashMap::new(),
            struct_instantiations: HashMap::new(),
            worklist: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// For each declared type parameter, verify that the corresponding
    /// concrete type argument satisfies every bound. Pushes a structured
    /// `CompileError` per unsatisfied bound; does not short-circuit.
    fn check_bounds(&mut self, generic_fn: &FnDecl, type_args: &[TypeExpr]) {
        for (tp, arg) in generic_fn.generics.iter().zip(type_args.iter()) {
            if tp.bounds.is_empty() {
                continue;
            }
            let arg_name = match arg {
                TypeExpr::Named(n) => n.clone(),
                TypeExpr::Primitive(p) => format!("{:?}", p).to_lowercase(),
                _ => format!("{:?}", arg),
            };
            let impls_for_arg = self.trait_impls.get(&arg_name).cloned().unwrap_or_default();
            for bound in &tp.bounds {
                if !impls_for_arg.contains(bound) {
                    self.errors.push(CompileError::resolve(
                        format!(
                            "type '{}' does not implement trait '{}' (required by type parameter '{}' on '{}')",
                            arg_name, bound, tp.name, generic_fn.name
                        ),
                        generic_fn.span.clone(),
                        self.source,
                    ));
                }
            }
        }
    }

    /// Walk a block, recording every reachable generic call site.
    fn collect_in_block(
        &mut self,
        block: &Block,
        subst: &HashMap<String, TypeExpr>,
        env: &mut HashMap<String, TypeExpr>,
    ) {
        let saved = env.clone();
        for s in &block.stmts {
            self.collect_in_stmt(s, subst, env);
        }
        // Restore lexical scope — any `let`s declared inside the block leave
        // when the block ends. Cheap snapshot/restore beats threading a
        // separate scope stack.
        *env = saved;
    }

    fn collect_in_stmt(
        &mut self,
        stmt: &Stmt,
        subst: &HashMap<String, TypeExpr>,
        env: &mut HashMap<String, TypeExpr>,
    ) {
        match stmt {
            Stmt::Let { name, ty, init, .. } => {
                self.collect_in_expr(init, subst, env);
                env.insert(name.clone(), substitute(ty, subst));
            }
            Stmt::Assign { lhs, rhs, .. } => {
                self.collect_in_expr(lhs, subst, env);
                self.collect_in_expr(rhs, subst, env);
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                self.collect_in_expr(cond, subst, env);
                self.collect_in_block(then_block, subst, env);
                if let Some(e) = else_block {
                    self.collect_in_else(e, subst, env);
                }
            }
            Stmt::IfLet {
                expr,
                then_block,
                else_block,
                ..
            } => {
                self.collect_in_expr(expr, subst, env);
                self.collect_in_block(then_block, subst, env);
                if let Some(b) = else_block {
                    self.collect_in_block(b, subst, env);
                }
            }
            Stmt::While { cond, body, .. } => {
                self.collect_in_expr(cond, subst, env);
                self.collect_in_block(body, subst, env);
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                if let Some(i) = init {
                    self.collect_in_for_init(i, subst, env);
                }
                if let Some(c) = cond {
                    self.collect_in_expr(c, subst, env);
                }
                if let Some(s) = step {
                    self.collect_in_for_step(s, subst, env);
                }
                self.collect_in_block(body, subst, env);
            }
            Stmt::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                self.collect_in_expr(expr, subst, env);
                for c in cases {
                    for s in &c.stmts {
                        self.collect_in_stmt(s, subst, env);
                    }
                }
                if let Some(stmts) = default {
                    for s in stmts {
                        self.collect_in_stmt(s, subst, env);
                    }
                }
            }
            Stmt::Return { value, .. } => {
                if let Some(e) = value {
                    self.collect_in_expr(e, subst, env);
                }
            }
            Stmt::Defer { body, .. } | Stmt::Unsafe { body, .. } => {
                self.collect_in_block(body, subst, env);
            }
            Stmt::Block(b) => self.collect_in_block(b, subst, env),
            Stmt::Expr { expr, .. } | Stmt::Discard { expr, .. } => {
                self.collect_in_expr(expr, subst, env);
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }

    fn collect_in_else(
        &mut self,
        br: &ElseBranch,
        subst: &HashMap<String, TypeExpr>,
        env: &mut HashMap<String, TypeExpr>,
    ) {
        match br {
            ElseBranch::ElseIf(b) => self.collect_in_stmt(b, subst, env),
            ElseBranch::Else(b) => self.collect_in_block(b, subst, env),
        }
    }

    fn collect_in_for_init(
        &mut self,
        fi: &ForInit,
        subst: &HashMap<String, TypeExpr>,
        env: &mut HashMap<String, TypeExpr>,
    ) {
        match fi {
            ForInit::Let { name, ty, init } => {
                self.collect_in_expr(init, subst, env);
                env.insert(name.clone(), substitute(ty, subst));
            }
            ForInit::Assign { lhs, rhs } => {
                self.collect_in_expr(lhs, subst, env);
                self.collect_in_expr(rhs, subst, env);
            }
            ForInit::Call(e) => self.collect_in_expr(e, subst, env),
        }
    }

    fn collect_in_for_step(
        &mut self,
        fs: &ForStep,
        subst: &HashMap<String, TypeExpr>,
        env: &mut HashMap<String, TypeExpr>,
    ) {
        match fs {
            ForStep::Assign { lhs, rhs } => {
                self.collect_in_expr(lhs, subst, env);
                self.collect_in_expr(rhs, subst, env);
            }
            ForStep::Call(e) => self.collect_in_expr(e, subst, env),
        }
    }

    fn collect_in_expr(
        &mut self,
        expr: &Expr,
        subst: &HashMap<String, TypeExpr>,
        env: &mut HashMap<String, TypeExpr>,
    ) {
        match expr {
            Expr::Call { callee, args, .. } => {
                for a in args {
                    self.collect_in_expr(a, subst, env);
                }
                if let Expr::Ident { name, .. } = callee.as_ref() {
                    // Look up the callee directly in `generic_fns`, which
                    // was populated by walking every Item — including those
                    // inside `mod` bodies. The symbol-table-based check
                    // would miss calls *between* mod-internal generic fns
                    // (e.g. `vec::new` -> `vec::with_capacity`) because
                    // those symbols don't live in the root scope unless
                    // explicitly re-imported.
                    if let Some(generic_fn) = self.generic_fns.get(name).cloned() {
                        let generic_params: Vec<String> =
                            generic_fn.generics.iter().map(|p| p.name.clone()).collect();
                        if !generic_params.is_empty() {
                            {
                                let type_args = infer_type_args(
                                    &generic_fn,
                                    args,
                                    &generic_params,
                                    subst,
                                    env,
                                    &self.all_structs,
                                    &self.all_fns,
                                );
                                // Bound check: verify each (type_param, type_arg)
                                // pair satisfies the declared trait bounds.
                                self.check_bounds(&generic_fn, &type_args);
                                let mangled = mangled_name(name, &type_args);
                                if !self.instantiations.contains_key(&mangled) {
                                    self.instantiations
                                        .insert(mangled, (name.clone(), type_args.clone()));
                                    self.worklist.push((name.clone(), type_args));
                                }
                            }
                        }
                    }
                }
                self.collect_in_expr(callee, subst, env);
            }
            Expr::Binary { lhs, rhs, .. } => {
                self.collect_in_expr(lhs, subst, env);
                self.collect_in_expr(rhs, subst, env);
            }
            Expr::Unary { operand, .. } => self.collect_in_expr(operand, subst, env),
            Expr::Paren { inner, .. } => self.collect_in_expr(inner, subst, env),
            Expr::Field { base, .. } => self.collect_in_expr(base, subst, env),
            Expr::Addr { operand, .. }
            | Expr::AddrM { operand, .. }
            | Expr::Deref { operand, .. } => {
                self.collect_in_expr(operand, subst, env);
            }
            Expr::At { base, index, .. } => {
                self.collect_in_expr(base, subst, env);
                self.collect_in_expr(index, subst, env);
            }
            Expr::Cast { expr: e, .. } => self.collect_in_expr(e, subst, env),
            Expr::SizeOf { .. } => {}
            Expr::Some { value, .. } | Expr::Ok { value, .. } | Expr::Err { value, .. } => {
                self.collect_in_expr(value, subst, env);
            }
            Expr::StructLit { fields, .. } => {
                for f in fields {
                    self.collect_in_expr(&f.value, subst, env);
                }
            }
            // Pure leaves.
            Expr::IntLit { .. }
            | Expr::FloatLit { .. }
            | Expr::BoolLit { .. }
            | Expr::Ident { .. }
            | Expr::CStr { .. }
            | Expr::Bytes { .. }
            | Expr::None { .. } => {}
            // Lifted away by desugar before mono runs.
            Expr::Closure { .. } => {
                unreachable!("Closure should have been lifted by desugar")
            }
        }
    }
}

/// Infer the concrete type arguments for a generic call.
fn infer_type_args(
    generic_fn: &FnDecl,
    args: &[Expr],
    type_params: &[String],
    subst: &HashMap<String, TypeExpr>,
    env: &HashMap<String, TypeExpr>,
    structs: &HashMap<String, StructDecl>,
    fns: &HashMap<String, FnDecl>,
) -> Vec<TypeExpr> {
    let mut inferred: HashMap<String, TypeExpr> = HashMap::new();
    for (arg_expr, param) in args.iter().zip(generic_fn.params.iter()) {
        let arg_ty = approx_expr_type(arg_expr, subst, env, structs, fns);
        unify_generic(&param.ty, &arg_ty, type_params, &mut inferred);
    }
    type_params
        .iter()
        .map(|p| {
            inferred
                .get(p)
                .cloned()
                .unwrap_or_else(|| TypeExpr::Named(p.clone()))
        })
        .map(|ty| substitute(&ty, subst))
        .collect()
}

/// Approximate the type of an expression. Looks up local variables in the
/// per-function `env` populated by walking `let` statements; everything
/// else is determined structurally. Enough for v1 inference, which only
/// needs literal and variable types at call sites.
fn approx_expr_type(
    expr: &Expr,
    subst: &HashMap<String, TypeExpr>,
    env: &HashMap<String, TypeExpr>,
    structs: &HashMap<String, StructDecl>,
    fns: &HashMap<String, FnDecl>,
) -> TypeExpr {
    match expr {
        Expr::IntLit { .. } => TypeExpr::Primitive(PrimitiveType::I32),
        Expr::FloatLit { .. } => TypeExpr::Primitive(PrimitiveType::F64),
        Expr::BoolLit { .. } => TypeExpr::Primitive(PrimitiveType::Bool),
        Expr::Cast { ty, .. } => substitute(ty, subst),
        Expr::SizeOf { .. } => TypeExpr::Primitive(PrimitiveType::Usize),
        Expr::Paren { inner, .. } => approx_expr_type(inner, subst, env, structs, fns),
        Expr::Addr { operand, .. } => {
            // Mirror typecheck: addr(x) has type ref(typeof x). Without
            // this, `push(addr(v), x)` cannot bind T from the receiver and
            // mono leaves the call generic.
            TypeExpr::Ref(Box::new(approx_expr_type(
                operand, subst, env, structs, fns,
            )))
        }
        Expr::AddrM { operand, .. } => TypeExpr::Mref(Box::new(approx_expr_type(
            operand, subst, env, structs, fns,
        ))),
        Expr::Deref { operand, .. } => match approx_expr_type(operand, subst, env, structs, fns) {
            TypeExpr::Ref(inner) | TypeExpr::Mref(inner) => *inner,
            TypeExpr::Raw(inner) | TypeExpr::Rawm(inner) => *inner,
            other => other,
        },
        Expr::Field { base, field, .. } => {
            // Struct field projection. Look up the field's declared type
            // in `structs` so calls like `push(addrm(s.data), x)` can
            // drive T-inference when `s.data` is a generic struct field.
            let base_ty = approx_expr_type(base, subst, env, structs, fns);
            let core = match &base_ty {
                TypeExpr::Ref(t) | TypeExpr::Mref(t) => (**t).clone(),
                other => other.clone(),
            };
            match core {
                TypeExpr::Named(struct_name) => structs
                    .get(&struct_name)
                    .and_then(|d| d.fields.iter().find(|f| f.name == *field))
                    .map(|f| substitute(&f.ty, subst))
                    .unwrap_or(TypeExpr::Void),
                TypeExpr::NamedGeneric(struct_name, type_args) => {
                    if let Some(decl) = structs.get(&struct_name) {
                        if decl.generics.len() == type_args.len() {
                            let local_subst: HashMap<String, TypeExpr> = decl
                                .generics
                                .iter()
                                .zip(type_args.iter())
                                .map(|(p, a)| (p.name.clone(), a.clone()))
                                .collect();
                            if let Some(f) = decl.fields.iter().find(|f| f.name == *field) {
                                let after_inner = substitute(&f.ty, &local_subst);
                                return substitute(&after_inner, subst);
                            }
                        }
                    }
                    TypeExpr::Void
                }
                _ => TypeExpr::Void,
            }
        }
        Expr::Ident { name, .. } => {
            // Substitution first — handles `T` inside a specialized body.
            let from_subst = substitute(&TypeExpr::Named(name.clone()), subst);
            if !matches!(&from_subst, TypeExpr::Named(n) if n == name) {
                return from_subst;
            }
            // Then the local env — variable `b` declared `: bool`.
            if let Some(local_ty) = env.get(name) {
                return local_ty.clone();
            }
            // Finally fall back to a top-level fn: a bare identifier that
            // names a function has type `fn(P...) -> R`. Needed for
            // higher-order generic calls like `vec::map(addr(v), dbl)`
            // where `dbl` is a free function and Fn-shape unification
            // recovers the result-type parameter from `dbl`'s signature.
            if let Some(decl) = fns.get(name) {
                return TypeExpr::Fn {
                    is_unsafe: decl.is_unsafe,
                    params: decl.params.iter().map(|p| p.ty.clone()).collect(),
                    ret: Box::new(decl.return_type.clone()),
                };
            }
            TypeExpr::Named(name.clone())
        }
        // Look up a non-generic call's return type so `vec::new(make())`
        // can drive T-inference from the inner result. Generic callees
        // would need recursive inference here — skipped for v1 because
        // a generic call's `T` is usually fixed by other arguments
        // anyway, and full inference would risk infinite recursion.
        Expr::Call { callee, .. } => {
            if let Expr::Ident { name, .. } = callee.as_ref() {
                if let Some(decl) = fns.get(name) {
                    if decl.generics.is_empty() {
                        return substitute(&decl.return_type, subst);
                    }
                }
            }
            TypeExpr::Void
        }
        _ => TypeExpr::Void,
    }
}

/// Extract the underlying type-name string from a receiver type — handles
/// named types and built-in primitives, strips one level of `ref`/`mref`.
/// Used to mangle method calls into `Type_method` invocations.
fn struct_name_of(ty: &TypeExpr) -> Option<String> {
    match ty {
        TypeExpr::Named(n) => Some(n.clone()),
        TypeExpr::Primitive(p) => Some(format!("{:?}", p).to_lowercase()),
        TypeExpr::Ref(inner) | TypeExpr::Mref(inner) => struct_name_of(inner),
        _ => None,
    }
}

/// Build a substitution from type-param-name to concrete TypeExpr.
fn build_subst(type_params: &[TypeParam], type_args: &[TypeExpr]) -> HashMap<String, TypeExpr> {
    type_params
        .iter()
        .zip(type_args.iter())
        .map(|(p, a)| (p.name.clone(), a.clone()))
        .collect()
}

/// Render a type expression as a short, deterministic C-identifier suffix.
fn mangle_type(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Primitive(p) => format!("{:?}", p).to_lowercase(),
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::NamedGeneric(n, args) => {
            let inner: Vec<String> = args.iter().map(mangle_type).collect();
            format!("{}_{}", n, inner.join("_"))
        }
        TypeExpr::Ref(inner) => format!("ref_{}", mangle_type(inner)),
        TypeExpr::Mref(inner) => format!("mref_{}", mangle_type(inner)),
        TypeExpr::Raw(inner) => format!("raw_{}", mangle_type(inner)),
        TypeExpr::Rawm(inner) => format!("rawm_{}", mangle_type(inner)),
        TypeExpr::Own(inner) => format!("own_{}", mangle_type(inner)),
        TypeExpr::Slice(inner) => format!("slice_{}", mangle_type(inner)),
        TypeExpr::Arr(inner, _) => format!("arr_{}", mangle_type(inner)),
        TypeExpr::Opt(inner) => format!("opt_{}", mangle_type(inner)),
        TypeExpr::Res(t, e) => format!("res_{}_{}", mangle_type(t), mangle_type(e)),
        TypeExpr::Fn { .. } => "fn".to_string(),
        TypeExpr::Void => "void".to_string(),
    }
}

fn mangled_name(fn_name: &str, type_args: &[TypeExpr]) -> String {
    let suffix: Vec<String> = type_args.iter().map(mangle_type).collect();
    format!("{}_{}", fn_name, suffix.join("_"))
}

/// Produce the specialized FnDecl for an instantiation.
fn specialize_fn(
    generic_fn: &FnDecl,
    mangled: &str,
    subst: &HashMap<String, TypeExpr>,
    ctx: &MonoCtx,
) -> FnDecl {
    let mut env: HashMap<String, TypeExpr> = HashMap::new();
    for p in &generic_fn.params {
        env.insert(p.name.clone(), substitute(&p.ty, subst));
    }
    let mut drop_stack: Vec<Vec<DropEntry>> = Vec::new();
    FnDecl {
        is_unsafe: generic_fn.is_unsafe,
        name: mangled.to_string(),
        generics: Vec::new(),
        doc_comments: generic_fn.doc_comments.clone(),
        params: generic_fn
            .params
            .iter()
            .map(|p| Param {
                name: p.name.clone(),
                ty: substitute(&p.ty, subst),
                span: p.span.clone(),
            })
            .collect(),
        return_type: substitute(&generic_fn.return_type, subst),
        body: rewrite_block(&generic_fn.body, subst, ctx, &mut env, &mut drop_stack),
        span: generic_fn.span.clone(),
    }
}

/// Pass-through for a non-generic FnDecl with rewritten call sites.
fn rewrite_fn(f: &FnDecl, subst: &HashMap<String, TypeExpr>, ctx: &MonoCtx) -> FnDecl {
    let mut env: HashMap<String, TypeExpr> = HashMap::new();
    for p in &f.params {
        env.insert(p.name.clone(), substitute(&p.ty, subst));
    }
    let mut drop_stack: Vec<Vec<DropEntry>> = Vec::new();
    FnDecl {
        is_unsafe: f.is_unsafe,
        name: f.name.clone(),
        generics: f.generics.clone(),
        doc_comments: f.doc_comments.clone(),
        params: f.params.clone(),
        return_type: f.return_type.clone(),
        body: rewrite_block(&f.body, subst, ctx, &mut env, &mut drop_stack),
        span: f.span.clone(),
    }
}

/// One variable in a scope that may need a Drop call on exit.
#[derive(Debug, Clone)]
struct DropEntry {
    name: String,
    ty: TypeExpr,
}

/// Construct a `Stmt::Expr` invoking `T_drop(addr(name))` for one variable.
fn make_drop_call(entry: &DropEntry, span: &crate::ast::Span) -> Stmt {
    let type_name = struct_name_of(&entry.ty).expect("DropEntry must carry a nameable type");
    let drop_fn = format!("{}_drop", type_name);
    let receiver = Expr::Addr {
        operand: Box::new(Expr::Ident {
            name: entry.name.clone(),
            span: span.clone(),
        }),
        span: span.clone(),
    };
    Stmt::Expr {
        expr: Expr::Call {
            callee: Box::new(Expr::Ident {
                name: drop_fn,
                span: span.clone(),
            }),
            args: vec![receiver],
            span: span.clone(),
        },
        span: span.clone(),
    }
}

/// Filter a scope to only the entries whose types implement `Drop`, then
/// emit calls in reverse declaration order (last-declared dropped first).
fn drops_for_scope(scope: &[DropEntry], ctx: &MonoCtx, span: &crate::ast::Span) -> Vec<Stmt> {
    scope
        .iter()
        .rev()
        .filter(|e| {
            struct_name_of(&e.ty)
                .map(|n| {
                    ctx.trait_impls
                        .get(&n)
                        .map(|s| s.contains("Drop"))
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        })
        .map(|e| make_drop_call(e, span))
        .collect()
}

/// Drops for every scope currently on the stack, innermost first. Used at
/// `return` statements to clean up the entire active stack before exit.
fn drops_for_all_scopes(
    stack: &[Vec<DropEntry>],
    ctx: &MonoCtx,
    span: &crate::ast::Span,
) -> Vec<Stmt> {
    let mut out = Vec::new();
    for scope in stack.iter().rev() {
        out.extend(drops_for_scope(scope, ctx, span));
    }
    out
}

/// Does this statement end the current block — meaning the end-of-block
/// drop pass should be skipped because control flow has transferred out?
fn is_terminator(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => true,
        // A Block whose last statement is itself a terminator (because we
        // wrapped `[drops..., return]` into one) also terminates.
        Stmt::Block(b) => b.stmts.last().is_some_and(is_terminator),
        _ => false,
    }
}

fn rewrite_block(
    block: &Block,
    subst: &HashMap<String, TypeExpr>,
    ctx: &MonoCtx,
    env: &mut HashMap<String, TypeExpr>,
    drop_stack: &mut Vec<Vec<DropEntry>>,
) -> Block {
    let saved = env.clone();
    drop_stack.push(Vec::new());

    let mut stmts: Vec<Stmt> = block
        .stmts
        .iter()
        .map(|s| rewrite_stmt(s, subst, ctx, env, drop_stack))
        .collect();

    // Append end-of-block drops only if control flow falls through.
    if !stmts.last().is_some_and(is_terminator) {
        let scope = drop_stack.last().expect("scope pushed");
        let drops = drops_for_scope(scope, ctx, &block.span);
        stmts.extend(drops);
    }

    drop_stack.pop();
    *env = saved;
    Block {
        stmts,
        span: block.span.clone(),
    }
}

fn rewrite_stmt(
    stmt: &Stmt,
    subst: &HashMap<String, TypeExpr>,
    ctx: &MonoCtx,
    env: &mut HashMap<String, TypeExpr>,
    drop_stack: &mut Vec<Vec<DropEntry>>,
) -> Stmt {
    match stmt {
        Stmt::Let {
            name,
            ty,
            init,
            span,
        } => {
            let new_init = rewrite_expr(init, subst, ctx, env);
            let new_ty = substitute(ty, subst);
            env.insert(name.clone(), new_ty.clone());
            // Track this binding for end-of-scope and return-path drops.
            if let Some(scope) = drop_stack.last_mut() {
                scope.push(DropEntry {
                    name: name.clone(),
                    ty: new_ty.clone(),
                });
            }
            Stmt::Let {
                name: name.clone(),
                ty: new_ty,
                init: new_init,
                span: span.clone(),
            }
        }
        Stmt::Assign { lhs, rhs, span } => Stmt::Assign {
            lhs: rewrite_expr(lhs, subst, ctx, env),
            rhs: rewrite_expr(rhs, subst, ctx, env),
            span: span.clone(),
        },
        Stmt::If {
            cond,
            then_block,
            else_block,
            span,
        } => Stmt::If {
            cond: rewrite_expr(cond, subst, ctx, env),
            then_block: rewrite_block(then_block, subst, ctx, env, drop_stack),
            else_block: else_block
                .as_ref()
                .map(|e| rewrite_else(e, subst, ctx, env, drop_stack)),
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
            expr: rewrite_expr(expr, subst, ctx, env),
            then_block: rewrite_block(then_block, subst, ctx, env, drop_stack),
            else_block: else_block
                .as_ref()
                .map(|b| rewrite_block(b, subst, ctx, env, drop_stack)),
            span: span.clone(),
        },
        Stmt::While { cond, body, span } => Stmt::While {
            cond: rewrite_expr(cond, subst, ctx, env),
            body: rewrite_block(body, subst, ctx, env, drop_stack),
            span: span.clone(),
        },
        Stmt::For {
            init,
            cond,
            step,
            body,
            span,
        } => Stmt::For {
            init: init.as_ref().map(|i| rewrite_for_init(i, subst, ctx, env)),
            cond: cond.as_ref().map(|c| rewrite_expr(c, subst, ctx, env)),
            step: step.as_ref().map(|s| rewrite_for_step(s, subst, ctx, env)),
            body: rewrite_block(body, subst, ctx, env, drop_stack),
            span: span.clone(),
        },
        Stmt::Switch {
            expr,
            cases,
            default,
            span,
        } => Stmt::Switch {
            expr: rewrite_expr(expr, subst, ctx, env),
            cases: cases
                .iter()
                .map(|c| Case {
                    value: c.value.clone(),
                    stmts: c
                        .stmts
                        .iter()
                        .map(|s| rewrite_stmt(s, subst, ctx, env, drop_stack))
                        .collect(),
                    span: c.span.clone(),
                })
                .collect(),
            default: default.as_ref().map(|stmts| {
                stmts
                    .iter()
                    .map(|s| rewrite_stmt(s, subst, ctx, env, drop_stack))
                    .collect()
            }),
            span: span.clone(),
        },
        Stmt::Return { value, span } => {
            // Drop every variable in every currently-active scope before
            // returning. Wrap the result in `Stmt::Block` only when at
            // least one drop is actually emitted so we don't introduce a
            // gratuitous C-level scope for the no-drops case.
            let new_return = Stmt::Return {
                value: value.as_ref().map(|e| rewrite_expr(e, subst, ctx, env)),
                span: span.clone(),
            };
            let drops = drops_for_all_scopes(drop_stack, ctx, span);
            if drops.is_empty() {
                new_return
            } else {
                let mut stmts = drops;
                stmts.push(new_return);
                Stmt::Block(Block {
                    stmts,
                    span: span.clone(),
                })
            }
        }
        Stmt::Defer { body, span } => Stmt::Defer {
            body: rewrite_block(body, subst, ctx, env, drop_stack),
            span: span.clone(),
        },
        Stmt::Unsafe { body, span } => Stmt::Unsafe {
            body: rewrite_block(body, subst, ctx, env, drop_stack),
            span: span.clone(),
        },
        Stmt::Block(b) => Stmt::Block(rewrite_block(b, subst, ctx, env, drop_stack)),
        Stmt::Expr { expr, span } => Stmt::Expr {
            expr: rewrite_expr(expr, subst, ctx, env),
            span: span.clone(),
        },
        Stmt::Discard { expr, span } => Stmt::Discard {
            expr: rewrite_expr(expr, subst, ctx, env),
            span: span.clone(),
        },
        Stmt::Break { .. } | Stmt::Continue { .. } => stmt.clone(),
    }
}

fn rewrite_else(
    br: &ElseBranch,
    subst: &HashMap<String, TypeExpr>,
    ctx: &MonoCtx,
    env: &mut HashMap<String, TypeExpr>,
    drop_stack: &mut Vec<Vec<DropEntry>>,
) -> ElseBranch {
    match br {
        ElseBranch::ElseIf(s) => {
            ElseBranch::ElseIf(Box::new(rewrite_stmt(s, subst, ctx, env, drop_stack)))
        }
        ElseBranch::Else(b) => ElseBranch::Else(rewrite_block(b, subst, ctx, env, drop_stack)),
    }
}

fn rewrite_for_init(
    fi: &ForInit,
    subst: &HashMap<String, TypeExpr>,
    ctx: &MonoCtx,
    env: &mut HashMap<String, TypeExpr>,
) -> ForInit {
    match fi {
        ForInit::Let { name, ty, init } => {
            let new_init = rewrite_expr(init, subst, ctx, env);
            let new_ty = substitute(ty, subst);
            env.insert(name.clone(), new_ty.clone());
            ForInit::Let {
                name: name.clone(),
                ty: new_ty,
                init: new_init,
            }
        }
        ForInit::Assign { lhs, rhs } => ForInit::Assign {
            lhs: rewrite_expr(lhs, subst, ctx, env),
            rhs: rewrite_expr(rhs, subst, ctx, env),
        },
        ForInit::Call(e) => ForInit::Call(rewrite_expr(e, subst, ctx, env)),
    }
}

fn rewrite_for_step(
    fs: &ForStep,
    subst: &HashMap<String, TypeExpr>,
    ctx: &MonoCtx,
    env: &mut HashMap<String, TypeExpr>,
) -> ForStep {
    match fs {
        ForStep::Assign { lhs, rhs } => ForStep::Assign {
            lhs: rewrite_expr(lhs, subst, ctx, env),
            rhs: rewrite_expr(rhs, subst, ctx, env),
        },
        ForStep::Call(e) => ForStep::Call(rewrite_expr(e, subst, ctx, env)),
    }
}

fn rewrite_expr(
    expr: &Expr,
    subst: &HashMap<String, TypeExpr>,
    ctx: &MonoCtx,
    env: &mut HashMap<String, TypeExpr>,
) -> Expr {
    match expr {
        // Method-call rewrite: `x.method(args)` (stage 1.0). The receiver's
        // type, looked up in `env`, names the struct the method belongs to.
        // We emit `Type_method(&x, args)` (auto-addressing when the receiver
        // is a value type, passing through when it is already a reference).
        Expr::Call { callee, args, span } if matches!(callee.as_ref(), Expr::Field { .. }) => {
            if let Expr::Field {
                base,
                field,
                span: field_span,
            } = callee.as_ref()
            {
                let rewritten_base = rewrite_expr(base, subst, ctx, env);
                let rewritten_args: Vec<Expr> = args
                    .iter()
                    .map(|a| rewrite_expr(a, subst, ctx, env))
                    .collect();

                let recv_ty = approx_expr_type(base, subst, env, &ctx.all_structs, &ctx.all_fns);
                if let Some(type_name) = struct_name_of(&recv_ty) {
                    let method_fn = format!("{}_{}", type_name, field);
                    // Auto-address value-typed receivers so the call
                    // matches `fn method(self: ref(Self), …)`.
                    let receiver = match &recv_ty {
                        TypeExpr::Ref(_) | TypeExpr::Mref(_) => rewritten_base,
                        _ => Expr::Addr {
                            operand: Box::new(rewritten_base),
                            span: field_span.clone(),
                        },
                    };
                    let mut new_args = Vec::with_capacity(rewritten_args.len() + 1);
                    new_args.push(receiver);
                    new_args.extend(rewritten_args);
                    return Expr::Call {
                        callee: Box::new(Expr::Ident {
                            name: method_fn,
                            span: field_span.clone(),
                        }),
                        args: new_args,
                        span: span.clone(),
                    };
                }
                // Receiver type unknown — fall through to a regular call
                // (will fail typecheck with a useful diagnostic).
                return Expr::Call {
                    callee: Box::new(Expr::Field {
                        base: Box::new(rewritten_base),
                        field: field.clone(),
                        span: field_span.clone(),
                    }),
                    args: rewritten_args,
                    span: span.clone(),
                };
            }
            unreachable!("matched but not Field")
        }
        Expr::Call { callee, args, span } => {
            let rewritten_args: Vec<Expr> = args
                .iter()
                .map(|a| rewrite_expr(a, subst, ctx, env))
                .collect();
            let new_callee = if let Expr::Ident {
                name,
                span: id_span,
            } = callee.as_ref()
            {
                // Mirror collect_in_expr: look up the callee in `generic_fns`
                // directly so mod-internal generic calls (e.g.
                // `vec::new` -> `vec::with_capacity`) rewrite correctly.
                match ctx.generic_fns.get(name) {
                    Some(generic_fn) if !generic_fn.generics.is_empty() => {
                        let generic_params: Vec<String> =
                            generic_fn.generics.iter().map(|p| p.name.clone()).collect();
                        let type_args = infer_type_args(
                            generic_fn,
                            args,
                            &generic_params,
                            subst,
                            env,
                            &ctx.all_structs,
                            &ctx.all_fns,
                        );
                        Box::new(Expr::Ident {
                            name: mangled_name(name, &type_args),
                            span: id_span.clone(),
                        })
                    }
                    _ => callee.clone(),
                }
            } else {
                Box::new(rewrite_expr(callee, subst, ctx, env))
            };
            Expr::Call {
                callee: new_callee,
                args: rewritten_args,
                span: span.clone(),
            }
        }
        Expr::Binary { op, lhs, rhs, span } => Expr::Binary {
            op: *op,
            lhs: Box::new(rewrite_expr(lhs, subst, ctx, env)),
            rhs: Box::new(rewrite_expr(rhs, subst, ctx, env)),
            span: span.clone(),
        },
        Expr::Unary { op, operand, span } => Expr::Unary {
            op: *op,
            operand: Box::new(rewrite_expr(operand, subst, ctx, env)),
            span: span.clone(),
        },
        Expr::Paren { inner, span } => Expr::Paren {
            inner: Box::new(rewrite_expr(inner, subst, ctx, env)),
            span: span.clone(),
        },
        Expr::Field { base, field, span } => Expr::Field {
            base: Box::new(rewrite_expr(base, subst, ctx, env)),
            field: field.clone(),
            span: span.clone(),
        },
        Expr::Addr { operand, span } => Expr::Addr {
            operand: Box::new(rewrite_expr(operand, subst, ctx, env)),
            span: span.clone(),
        },
        Expr::AddrM { operand, span } => Expr::AddrM {
            operand: Box::new(rewrite_expr(operand, subst, ctx, env)),
            span: span.clone(),
        },
        Expr::Deref { operand, span } => Expr::Deref {
            operand: Box::new(rewrite_expr(operand, subst, ctx, env)),
            span: span.clone(),
        },
        Expr::At { base, index, span } => Expr::At {
            base: Box::new(rewrite_expr(base, subst, ctx, env)),
            index: Box::new(rewrite_expr(index, subst, ctx, env)),
            span: span.clone(),
        },
        Expr::Cast { ty, expr: e, span } => Expr::Cast {
            ty: substitute(ty, subst),
            expr: Box::new(rewrite_expr(e, subst, ctx, env)),
            span: span.clone(),
        },
        Expr::SizeOf { ty, span } => Expr::SizeOf {
            ty: substitute(ty, subst),
            span: span.clone(),
        },
        Expr::None { ty, span } => Expr::None {
            ty: substitute(ty, subst),
            span: span.clone(),
        },
        Expr::Some { value, span } => Expr::Some {
            value: Box::new(rewrite_expr(value, subst, ctx, env)),
            span: span.clone(),
        },
        Expr::Ok { value, span } => Expr::Ok {
            value: Box::new(rewrite_expr(value, subst, ctx, env)),
            span: span.clone(),
        },
        Expr::Err { value, span } => Expr::Err {
            value: Box::new(rewrite_expr(value, subst, ctx, env)),
            span: span.clone(),
        },
        Expr::StructLit { name, fields, span } => Expr::StructLit {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|f| FieldInit {
                    name: f.name.clone(),
                    value: rewrite_expr(&f.value, subst, ctx, env),
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
        | Expr::Bytes { .. } => expr.clone(),
        Expr::Closure { .. } => unreachable!("Closure should have been lifted by desugar"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mangled_name_basic() {
        let n = mangled_name("id", &[TypeExpr::Primitive(PrimitiveType::I32)]);
        assert_eq!(n, "id_i32");
    }

    #[test]
    fn mangled_name_multi() {
        let n = mangled_name(
            "pair",
            &[
                TypeExpr::Primitive(PrimitiveType::I32),
                TypeExpr::Primitive(PrimitiveType::F64),
            ],
        );
        assert_eq!(n, "pair_i32_f64");
    }
}
