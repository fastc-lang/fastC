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
    PrimitiveType, Stmt, TypeExpr, TypeParam,
};
use crate::diag::CompileError;
use crate::resolve::{SymbolKind, SymbolTable};
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

    // Pass 1: collect every generic call reachable from non-generic code.
    for item in &file.items {
        if let Item::Fn(f) = item {
            if f.generics.is_empty() {
                let mut env: HashMap<String, TypeExpr> = HashMap::new();
                for p in &f.params {
                    env.insert(p.name.clone(), p.ty.clone());
                }
                ctx.collect_in_block(&f.body, &HashMap::new(), &mut env);
            }
        }
    }

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
    // instantiations are appended in deterministic order.
    let mut new_items: Vec<Item> = Vec::with_capacity(file.items.len());
    for item in &file.items {
        match item {
            Item::Fn(f) if !f.generics.is_empty() => {
                // Drop the generic declaration; instantiations follow.
            }
            Item::Fn(f) => {
                new_items.push(Item::Fn(rewrite_fn(f, &HashMap::new(), &ctx)));
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
        symbols,
        source: ctx.source,
        trait_impls: ctx.trait_impls.clone(),
        instantiations: ctx.instantiations.clone(),
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
    Ok(File { items: new_items })
}

/// Walking context for monomorphization.
struct MonoCtx<'a> {
    /// All generic-fn declarations in the program, keyed by name.
    generic_fns: HashMap<String, FnDecl>,
    /// Symbol table used to identify generic call sites by name lookup.
    symbols: &'a SymbolTable,
    /// Source text — passed to error constructors so spans render properly.
    source: &'a str,
    /// `(type_name → set of traits implemented)`. Built by walking
    /// `Item::Impl` entries that name a trait. Used to verify bound
    /// satisfaction when specializing generic instantiations.
    trait_impls: HashMap<String, HashSet<String>>,
    /// Known instantiations keyed by mangled name. The value records the
    /// original (fn_name, type_args) tuple so pass 2 can specialize the body.
    /// Mangled-name keying side-steps `Vec<TypeExpr>: !Hash`.
    instantiations: HashMap<String, (String, Vec<TypeExpr>)>,
    /// Worklist for transitive instantiation discovery.
    worklist: Vec<(String, Vec<TypeExpr>)>,
    /// Bound-satisfaction errors accumulated during the collect pass.
    errors: Vec<CompileError>,
}

impl<'a> MonoCtx<'a> {
    fn new(file: &'a File, symbols: &'a SymbolTable, source: &'a str) -> Self {
        let mut generic_fns = HashMap::new();
        let mut trait_impls: HashMap<String, HashSet<String>> = HashMap::new();
        for item in &file.items {
            match item {
                Item::Fn(f) if !f.generics.is_empty() => {
                    generic_fns.insert(f.name.clone(), f.clone());
                }
                Item::Impl(block) => {
                    if let Some(trait_name) = &block.trait_name {
                        trait_impls
                            .entry(block.target.clone())
                            .or_default()
                            .insert(trait_name.clone());
                    }
                }
                _ => {}
            }
        }
        Self {
            generic_fns,
            symbols,
            source,
            trait_impls,
            instantiations: HashMap::new(),
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
                    if let Some(SymbolKind::Function { generic_params, .. }) =
                        self.symbols.lookup(name).map(|s| s.kind.clone())
                    {
                        if !generic_params.is_empty() {
                            if let Some(generic_fn) = self.generic_fns.get(name).cloned() {
                                let type_args =
                                    infer_type_args(&generic_fn, args, &generic_params, subst, env);
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
            Expr::Addr { operand, .. } | Expr::Deref { operand, .. } => {
                self.collect_in_expr(operand, subst, env);
            }
            Expr::At { base, index, .. } => {
                self.collect_in_expr(base, subst, env);
                self.collect_in_expr(index, subst, env);
            }
            Expr::Cast { expr: e, .. } => self.collect_in_expr(e, subst, env),
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
) -> Vec<TypeExpr> {
    let mut inferred: HashMap<String, TypeExpr> = HashMap::new();
    for (arg_expr, param) in args.iter().zip(generic_fn.params.iter()) {
        let arg_ty = approx_expr_type(arg_expr, subst, env);
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
) -> TypeExpr {
    match expr {
        Expr::IntLit { .. } => TypeExpr::Primitive(PrimitiveType::I32),
        Expr::FloatLit { .. } => TypeExpr::Primitive(PrimitiveType::F64),
        Expr::BoolLit { .. } => TypeExpr::Primitive(PrimitiveType::Bool),
        Expr::Cast { ty, .. } => substitute(ty, subst),
        Expr::Paren { inner, .. } => approx_expr_type(inner, subst, env),
        Expr::Ident { name, .. } => {
            // Substitution first — handles `T` inside a specialized body.
            let from_subst = substitute(&TypeExpr::Named(name.clone()), subst);
            if !matches!(&from_subst, TypeExpr::Named(n) if n == name) {
                return from_subst;
            }
            // Then the local env — variable `b` declared `: bool`.
            env.get(name)
                .cloned()
                .unwrap_or_else(|| TypeExpr::Named(name.clone()))
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
    FnDecl {
        is_unsafe: generic_fn.is_unsafe,
        name: mangled.to_string(),
        generics: Vec::new(),
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
        body: rewrite_block(&generic_fn.body, subst, ctx, &mut env),
        span: generic_fn.span.clone(),
    }
}

/// Pass-through for a non-generic FnDecl with rewritten call sites.
fn rewrite_fn(f: &FnDecl, subst: &HashMap<String, TypeExpr>, ctx: &MonoCtx) -> FnDecl {
    let mut env: HashMap<String, TypeExpr> = HashMap::new();
    for p in &f.params {
        env.insert(p.name.clone(), substitute(&p.ty, subst));
    }
    FnDecl {
        is_unsafe: f.is_unsafe,
        name: f.name.clone(),
        generics: f.generics.clone(),
        params: f.params.clone(),
        return_type: f.return_type.clone(),
        body: rewrite_block(&f.body, subst, ctx, &mut env),
        span: f.span.clone(),
    }
}

fn rewrite_block(
    block: &Block,
    subst: &HashMap<String, TypeExpr>,
    ctx: &MonoCtx,
    env: &mut HashMap<String, TypeExpr>,
) -> Block {
    let saved = env.clone();
    let block = Block {
        stmts: block
            .stmts
            .iter()
            .map(|s| rewrite_stmt(s, subst, ctx, env))
            .collect(),
        span: block.span.clone(),
    };
    *env = saved;
    block
}

fn rewrite_stmt(
    stmt: &Stmt,
    subst: &HashMap<String, TypeExpr>,
    ctx: &MonoCtx,
    env: &mut HashMap<String, TypeExpr>,
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
            then_block: rewrite_block(then_block, subst, ctx, env),
            else_block: else_block
                .as_ref()
                .map(|e| rewrite_else(e, subst, ctx, env)),
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
            then_block: rewrite_block(then_block, subst, ctx, env),
            else_block: else_block
                .as_ref()
                .map(|b| rewrite_block(b, subst, ctx, env)),
            span: span.clone(),
        },
        Stmt::While { cond, body, span } => Stmt::While {
            cond: rewrite_expr(cond, subst, ctx, env),
            body: rewrite_block(body, subst, ctx, env),
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
            body: rewrite_block(body, subst, ctx, env),
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
                        .map(|s| rewrite_stmt(s, subst, ctx, env))
                        .collect(),
                    span: c.span.clone(),
                })
                .collect(),
            default: default.as_ref().map(|stmts| {
                stmts
                    .iter()
                    .map(|s| rewrite_stmt(s, subst, ctx, env))
                    .collect()
            }),
            span: span.clone(),
        },
        Stmt::Return { value, span } => Stmt::Return {
            value: value.as_ref().map(|e| rewrite_expr(e, subst, ctx, env)),
            span: span.clone(),
        },
        Stmt::Defer { body, span } => Stmt::Defer {
            body: rewrite_block(body, subst, ctx, env),
            span: span.clone(),
        },
        Stmt::Unsafe { body, span } => Stmt::Unsafe {
            body: rewrite_block(body, subst, ctx, env),
            span: span.clone(),
        },
        Stmt::Block(b) => Stmt::Block(rewrite_block(b, subst, ctx, env)),
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
) -> ElseBranch {
    match br {
        ElseBranch::ElseIf(s) => ElseBranch::ElseIf(Box::new(rewrite_stmt(s, subst, ctx, env))),
        ElseBranch::Else(b) => ElseBranch::Else(rewrite_block(b, subst, ctx, env)),
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

                let recv_ty = approx_expr_type(base, subst, env);
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
                if let Some(SymbolKind::Function { generic_params, .. }) =
                    ctx.symbols.lookup(name).map(|s| s.kind.clone())
                {
                    if !generic_params.is_empty() {
                        if let Some(generic_fn) = ctx.generic_fns.get(name) {
                            let type_args =
                                infer_type_args(generic_fn, args, &generic_params, subst, env);
                            Box::new(Expr::Ident {
                                name: mangled_name(name, &type_args),
                                span: id_span.clone(),
                            })
                        } else {
                            callee.clone()
                        }
                    } else {
                        callee.clone()
                    }
                } else {
                    callee.clone()
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
