//! Type checking pass
//!
//! This pass:
//! 1. Infers types for all expressions
//! 2. Checks that operators are applied to correct types
//! 3. Checks that function calls have correct arguments
//! 4. Tracks unsafe context and enforces safety rules

mod context;
mod safety;

pub use context::*;
pub use safety::*;

use crate::ast::{
    BinOp, Block, ConstExpr, EnumDecl, Expr, ExternItem, File, FnDecl, ImplBlock, Item,
    PrimitiveType, Repr, Stmt, StructDecl, TraitDecl, TypeExpr, UnaryOp,
};
use crate::diag::CompileError;
use crate::lexer::Span;
use crate::resolve::{Symbol, SymbolKind, SymbolTable};
use std::collections::{HashMap, HashSet};

/// Type checker
pub struct TypeChecker<'a> {
    source: &'a str,
    symbols: SymbolTable,
    safety: SafetyContext,
    current_fn_return_type: Option<TypeExpr>,
    errors: Vec<CompileError>,
    enum_decls: HashMap<String, EnumDecl>,
    struct_decls: HashMap<String, StructDecl>,
    /// Trait declarations indexed by name. Populated from `Item::Trait`
    /// entries during the first pass; consulted by the method-call
    /// dispatch path when the receiver's type is a bounded type parameter.
    traits: HashMap<String, TraitDecl>,
    /// `(type_name → set of trait names it implements)`. Populated from
    /// `Item::Impl` entries with a `trait_name`. Mono uses this to verify
    /// bound satisfaction; typecheck uses it to confirm calls on
    /// concrete-typed receivers resolve to a real impl.
    trait_impls: HashMap<String, std::collections::HashSet<String>>,
}

impl<'a> TypeChecker<'a> {
    pub fn new(source: &'a str, symbols: SymbolTable) -> Self {
        Self {
            source,
            symbols,
            safety: SafetyContext::new(),
            current_fn_return_type: None,
            errors: Vec::new(),
            enum_decls: HashMap::new(),
            struct_decls: HashMap::new(),
            traits: HashMap::new(),
            trait_impls: HashMap::new(),
        }
    }

    pub fn check(&mut self, file: &File) -> Result<(), CompileError> {
        // First pass: collect type declarations for validation (including from modules)
        self.collect_type_decls(&file.items);

        // Second pass: type check items
        for item in &file.items {
            self.check_item(item);
        }

        // Return all errors collected during type checking
        if !self.errors.is_empty() {
            Err(CompileError::multiple(std::mem::take(&mut self.errors)))
        } else {
            Ok(())
        }
    }

    /// Recursively collect enum/struct/trait declarations and trait-impl
    /// table entries from items (including modules).
    fn collect_type_decls(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::Enum(enum_decl) => {
                    self.enum_decls
                        .insert(enum_decl.name.clone(), enum_decl.clone());
                }
                Item::Struct(struct_decl) => {
                    self.struct_decls
                        .insert(struct_decl.name.clone(), struct_decl.clone());
                }
                Item::Trait(trait_decl) => {
                    self.traits
                        .insert(trait_decl.name.clone(), trait_decl.clone());
                }
                Item::Impl(impl_block) => {
                    if let Some(trait_name) = &impl_block.trait_name {
                        self.trait_impls
                            .entry(impl_block.target.clone())
                            .or_default()
                            .insert(trait_name.clone());
                    }
                }
                Item::Mod(mod_decl) => {
                    if let Some(body) = &mod_decl.body {
                        self.collect_type_decls(body);
                    }
                }
                _ => {}
            }
        }
    }

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Fn(fn_decl) => self.check_fn(fn_decl),
            Item::Struct(_) => {} // Struct fields were checked during resolution
            Item::Enum(_) => {}
            Item::Const(_) => {} // Const type was declared
            Item::Opaque(_) => {}
            Item::Extern(extern_block) => {
                // Validate FFI types in extern signatures
                for extern_item in &extern_block.items {
                    if let ExternItem::Fn(proto) = extern_item {
                        // Check return type
                        self.validate_ffi_type(&proto.return_type, &proto.span);
                        // Check parameters
                        for param in &proto.params {
                            self.validate_ffi_type(&param.ty, &proto.span);
                        }
                    }
                }
            }
            Item::Use(_) => {} // Module imports resolved during name resolution
            Item::Mod(mod_decl) => self.check_mod(mod_decl),
            Item::Impl(_) | Item::Trait(_) => {} // Desugared away before typecheck.
        }
    }

    /// Verify a method-call invocation against the method's function type.
    /// Returns the method's return type. Reports diagnostics for arity,
    /// receiver-compatibility, and argument-type mismatches.
    fn check_method_call(
        &mut self,
        field: &str,
        method_ty: &TypeExpr,
        recv_ty: &TypeExpr,
        base: &Expr,
        args: &[Expr],
        span: &Span,
    ) -> TypeExpr {
        if let TypeExpr::Fn {
            is_unsafe,
            params,
            ret,
        } = method_ty
        {
            if *is_unsafe && !self.safety.is_unsafe() {
                self.error_with_hint(
                    "call to unsafe method requires unsafe block".to_string(),
                    span.clone(),
                    "wrap the call in an unsafe block: unsafe { ... }",
                );
            }
            if args.len() + 1 != params.len() {
                self.error(
                    format!(
                        "method '{}' expected {} argument(s) (after receiver), got {}",
                        field,
                        params.len().saturating_sub(1),
                        args.len()
                    ),
                    span.clone(),
                );
                return (**ret).clone();
            }
            let self_formal = &params[0];
            // Auto-address value receivers; pass references through.
            let recv_compat = self
                .types_compatible(self_formal, &TypeExpr::Ref(Box::new(recv_ty.clone())))
                || self.types_compatible(self_formal, recv_ty);
            if !recv_compat {
                self.error_type_mismatch(self_formal, recv_ty, &base.span());
            }
            for (arg, param_ty) in args.iter().zip(params.iter().skip(1)) {
                let arg_ty = self.infer_expr(arg);
                if !self.types_compatible(param_ty, &arg_ty) {
                    self.error_type_mismatch(param_ty, &arg_ty, &arg.span());
                }
            }
            return (**ret).clone();
        }
        // Method symbol exists but its type isn't a function — programmer
        // error somewhere upstream. Report and continue.
        self.error(
            format!("'{}' is not callable as a method", field),
            span.clone(),
        );
        TypeExpr::Void
    }

    fn check_fn(&mut self, fn_decl: &FnDecl) {
        // Enter function scope
        self.symbols.enter_scope();

        // Track if this is an unsafe function
        if fn_decl.is_unsafe {
            self.safety.enter_unsafe();
        }

        // Set current return type
        self.current_fn_return_type = Some(fn_decl.return_type.clone());

        // Define type parameters in scope so the method-dispatch path can
        // recover their bounds (`T: Trait`) at call sites inside the body.
        for tp in &fn_decl.generics {
            let symbol = Symbol {
                name: tp.name.clone(),
                kind: SymbolKind::TypeParam {
                    bounds: tp.bounds.clone(),
                },
                ty: TypeExpr::Named(tp.name.clone()),
                span: tp.span.clone(),
            };
            let _ = self.symbols.define(symbol);
        }

        // Define parameters in scope
        for param in &fn_decl.params {
            let symbol = Symbol {
                name: param.name.clone(),
                kind: SymbolKind::Variable,
                ty: param.ty.clone(),
                span: param.span.clone(),
            };
            let _ = self.symbols.define(symbol);
        }

        // Check body
        self.check_block(&fn_decl.body);

        // Reset state
        self.current_fn_return_type = None;
        if fn_decl.is_unsafe {
            self.safety.exit_unsafe();
        }
        self.symbols.exit_scope();
    }

    fn check_mod(&mut self, mod_decl: &crate::ast::ModDecl) {
        if let Some(body) = &mod_decl.body {
            // Look up the module's scope_id from the symbol table
            if let Some(sym) = self.symbols.lookup(&mod_decl.name) {
                if let SymbolKind::Module { scope_id } = sym.kind {
                    let old = self.symbols.set_scope(scope_id);
                    for item in body {
                        self.check_item(item);
                    }
                    self.symbols.set_scope(old);
                }
            }
        }
    }

    fn check_block(&mut self, block: &Block) {
        self.symbols.enter_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        self.symbols.exit_scope();
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                name,
                ty,
                init,
                span,
            } => {
                let init_ty = self.infer_expr(init);
                if !self.types_compatible(ty, &init_ty) {
                    self.error_type_mismatch(ty, &init_ty, span);
                }

                // Define variable
                let symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable,
                    ty: ty.clone(),
                    span: span.clone(),
                };
                let _ = self.symbols.define(symbol);
            }

            Stmt::Assign { lhs, rhs, span } => {
                let lhs_ty = self.infer_expr(lhs);
                let rhs_ty = self.infer_expr(rhs);

                if !self.types_compatible(&lhs_ty, &rhs_ty) {
                    self.error_type_mismatch(&lhs_ty, &rhs_ty, span);
                }

                // Check that lhs is assignable
                self.check_assignable(lhs, span);
            }

            Stmt::If {
                cond,
                then_block,
                else_block,
                span,
            } => {
                let cond_ty = self.infer_expr(cond);
                if !self.is_bool(&cond_ty) {
                    self.error(
                        format!("condition must be bool, got {:?}", cond_ty),
                        span.clone(),
                    );
                }

                self.check_block(then_block);

                if let Some(else_branch) = else_block {
                    match else_branch {
                        crate::ast::ElseBranch::ElseIf(if_stmt) => self.check_stmt(if_stmt),
                        crate::ast::ElseBranch::Else(block) => self.check_block(block),
                    }
                }
            }

            Stmt::IfLet {
                name,
                expr,
                then_block,
                else_block,
                span,
            } => {
                let expr_ty = self.infer_expr(expr);

                // The expression should be opt(T) or res(T, E)
                let inner_ty = match &expr_ty {
                    TypeExpr::Opt(inner) => (**inner).clone(),
                    TypeExpr::Res(ok, _) => (**ok).clone(),
                    _ => {
                        self.error(
                            format!("if-let requires opt or res type, got {:?}", expr_ty),
                            span.clone(),
                        );
                        TypeExpr::Void
                    }
                };

                // Check then block with bound variable
                self.symbols.enter_scope();
                let symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable,
                    ty: inner_ty,
                    span: span.clone(),
                };
                let _ = self.symbols.define(symbol);

                for stmt in &then_block.stmts {
                    self.check_stmt(stmt);
                }
                self.symbols.exit_scope();

                if let Some(else_blk) = else_block {
                    self.check_block(else_blk);
                }
            }

            Stmt::While { cond, body, span } => {
                let cond_ty = self.infer_expr(cond);
                if !self.is_bool(&cond_ty) {
                    self.error(
                        format!("condition must be bool, got {:?}", cond_ty),
                        span.clone(),
                    );
                }
                self.check_block(body);
            }

            Stmt::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                self.symbols.enter_scope();

                if let Some(init) = init {
                    match init {
                        crate::ast::ForInit::Let { name, ty, init } => {
                            let init_ty = self.infer_expr(init);
                            if !self.types_compatible(ty, &init_ty) {
                                self.error_type_mismatch(ty, &init_ty, &init.span());
                            }
                            let symbol = Symbol {
                                name: name.clone(),
                                kind: SymbolKind::Variable,
                                ty: ty.clone(),
                                span: 0..0,
                            };
                            let _ = self.symbols.define(symbol);
                        }
                        crate::ast::ForInit::Assign { lhs, rhs } => {
                            let lhs_ty = self.infer_expr(lhs);
                            let rhs_ty = self.infer_expr(rhs);
                            if !self.types_compatible(&lhs_ty, &rhs_ty) {
                                self.error_type_mismatch(&lhs_ty, &rhs_ty, &lhs.span());
                            }
                        }
                        crate::ast::ForInit::Call(expr) => {
                            self.infer_expr(expr);
                        }
                    }
                }

                if let Some(cond) = cond {
                    let cond_ty = self.infer_expr(cond);
                    if !self.is_bool(&cond_ty) {
                        self.error(
                            format!("for condition must be bool, got {:?}", cond_ty),
                            cond.span(),
                        );
                    }
                }

                if let Some(step) = step {
                    match step {
                        crate::ast::ForStep::Assign { lhs, rhs } => {
                            let lhs_ty = self.infer_expr(lhs);
                            let rhs_ty = self.infer_expr(rhs);
                            if !self.types_compatible(&lhs_ty, &rhs_ty) {
                                self.error_type_mismatch(&lhs_ty, &rhs_ty, &lhs.span());
                            }
                        }
                        crate::ast::ForStep::Call(expr) => {
                            self.infer_expr(expr);
                        }
                    }
                }

                for stmt in &body.stmts {
                    self.check_stmt(stmt);
                }

                self.symbols.exit_scope();
            }

            Stmt::Switch {
                expr,
                cases,
                default,
                span,
            } => {
                let expr_ty = self.infer_expr(expr);

                // Switch must be on integer or enum type
                if !self.is_integer(&expr_ty) && !matches!(expr_ty, TypeExpr::Named(_)) {
                    self.error(
                        format!(
                            "switch expression must be integer or enum, got {:?}",
                            expr_ty
                        ),
                        expr.span(),
                    );
                }

                // Exhaustiveness check for enums
                if let TypeExpr::Named(enum_name) = &expr_ty {
                    if let Some(enum_decl) = self.enum_decls.get(enum_name).cloned() {
                        let expected_variants: HashSet<String> = enum_decl
                            .variants
                            .iter()
                            .map(|v| format!("{}_{}", enum_name, v.name))
                            .collect();

                        let mut covered_variants = HashSet::new();

                        for case in cases {
                            // Extract variant name from case value
                            if let ConstExpr::Ident(name) = &case.value {
                                covered_variants.insert(name.clone());
                            }
                        }

                        let missing: Vec<_> = expected_variants
                            .difference(&covered_variants)
                            .cloned()
                            .collect();

                        if !missing.is_empty() && default.is_none() {
                            self.error(
                                format!(
                                    "non-exhaustive switch on enum '{}': missing variants {:?}",
                                    enum_name, missing
                                ),
                                span.clone(),
                            );
                        }
                    }
                }

                for case in cases {
                    for stmt in &case.stmts {
                        self.check_stmt(stmt);
                    }
                }

                // Check default block if present
                if let Some(default_stmts) = default {
                    for stmt in default_stmts {
                        self.check_stmt(stmt);
                    }
                }
            }

            Stmt::Return { value, span } => {
                let expected = self
                    .current_fn_return_type
                    .clone()
                    .unwrap_or(TypeExpr::Void);

                if let Some(value) = value {
                    let actual = self.infer_expr(value);
                    if !self.types_compatible(&expected, &actual) {
                        self.error_type_mismatch(&expected, &actual, span);
                    }
                } else if !matches!(expected, TypeExpr::Void) {
                    self.error(
                        format!("expected return value of type {:?}", expected),
                        span.clone(),
                    );
                }
            }

            Stmt::Break { .. } | Stmt::Continue { .. } => {}

            Stmt::Defer { body, .. } => {
                self.check_block(body);
            }

            Stmt::Expr { expr, .. } => {
                self.infer_expr(expr);
            }

            Stmt::Discard { expr, .. } => {
                self.infer_expr(expr);
            }

            Stmt::Unsafe { body, .. } => {
                self.safety.enter_unsafe();
                self.check_block(body);
                self.safety.exit_unsafe();
            }

            Stmt::Block(block) => {
                self.check_block(block);
            }
        }
    }

    fn infer_expr(&mut self, expr: &Expr) -> TypeExpr {
        match expr {
            Expr::IntLit { .. } => TypeExpr::Primitive(PrimitiveType::I32), // Default to i32
            Expr::FloatLit { .. } => TypeExpr::Primitive(PrimitiveType::F64), // Default to f64
            Expr::BoolLit { .. } => TypeExpr::Primitive(PrimitiveType::Bool),
            Expr::CStr { .. } => TypeExpr::Raw(Box::new(TypeExpr::Primitive(PrimitiveType::U8))),
            Expr::Bytes { .. } => TypeExpr::Slice(Box::new(TypeExpr::Primitive(PrimitiveType::U8))),

            Expr::Ident { name, .. } => {
                if let Some(sym) = self.symbols.lookup(name) {
                    sym.ty.clone()
                } else {
                    TypeExpr::Void // Error already reported during resolution
                }
            }

            Expr::Binary { op, lhs, rhs, span } => {
                let lhs_ty = self.infer_expr(lhs);
                let rhs_ty = self.infer_expr(rhs);

                // Binary ops require same types
                if !self.types_compatible(&lhs_ty, &rhs_ty) {
                    self.error_type_mismatch(&lhs_ty, &rhs_ty, span);
                }

                match op {
                    // Comparison operators return bool
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        TypeExpr::Primitive(PrimitiveType::Bool)
                    }
                    // Logical operators require bool and return bool
                    BinOp::And | BinOp::Or => {
                        if !self.is_bool(&lhs_ty) {
                            self.error(
                                format!("logical operator requires bool, got {:?}", lhs_ty),
                                span.clone(),
                            );
                        }
                        TypeExpr::Primitive(PrimitiveType::Bool)
                    }
                    // Arithmetic and bitwise operators return the same type
                    _ => lhs_ty,
                }
            }

            Expr::Unary { op, operand, span } => {
                let operand_ty = self.infer_expr(operand);

                match op {
                    UnaryOp::Neg => {
                        if !self.is_numeric(&operand_ty) {
                            self.error(
                                format!("negation requires numeric type, got {:?}", operand_ty),
                                span.clone(),
                            );
                        }
                        operand_ty
                    }
                    UnaryOp::Not => {
                        if !self.is_bool(&operand_ty) {
                            self.error(
                                format!("logical not requires bool, got {:?}", operand_ty),
                                span.clone(),
                            );
                        }
                        TypeExpr::Primitive(PrimitiveType::Bool)
                    }
                    UnaryOp::BitNot => {
                        if !self.is_integer(&operand_ty) {
                            self.error(
                                format!("bitwise not requires integer, got {:?}", operand_ty),
                                span.clone(),
                            );
                        }
                        operand_ty
                    }
                }
            }

            Expr::Paren { inner, .. } => self.infer_expr(inner),

            Expr::Call { callee, args, span } => {
                // Stage 1.0: method-call shape `x.method(args)` parses as a
                // Call whose callee is a Field. Look up the method via the
                // lifted free function `Type_method` and typecheck as a
                // regular call with the receiver as the first argument.
                if let Expr::Field { base, field, .. } = callee.as_ref() {
                    let recv_ty = self.infer_expr(base);
                    let type_name = type_name_of(&recv_ty);
                    if let Some(type_name) = type_name {
                        // First: try the concrete-type path — look up the
                        // lifted `Type_method` free function.
                        let method_fn = format!("{}_{}", type_name, field);
                        if let Some(sym) = self.symbols.lookup(&method_fn).cloned() {
                            return self
                                .check_method_call(field, &sym.ty, &recv_ty, base, args, span);
                        }

                        // Second: if the receiver type names a bounded type
                        // parameter, look up the method in each bound trait
                        // and use the trait's prototype signature for
                        // typecheck. Mono later rewrites the call to the
                        // concrete `Type_method` after specialization.
                        if let Some(SymbolKind::TypeParam { bounds }) =
                            self.symbols.lookup(&type_name).map(|s| s.kind.clone())
                        {
                            let mut found: Option<(String, crate::ast::FnProto)> = None;
                            for bound in &bounds {
                                if let Some(tr) = self.traits.get(bound) {
                                    if let Some(proto) =
                                        tr.methods.iter().find(|p| p.name == *field)
                                    {
                                        found = Some((bound.clone(), proto.clone()));
                                        break;
                                    }
                                }
                            }
                            if let Some((_bound_name, proto)) = found {
                                // Build the trait method's effective fn type
                                // with `Self` substituted to the type-param's
                                // own name. We *do not* sub to a concrete
                                // type here — that's mono's job.
                                let mut subst: HashMap<String, TypeExpr> = HashMap::new();
                                subst
                                    .insert("Self".to_string(), TypeExpr::Named(type_name.clone()));
                                let method_ty = TypeExpr::Fn {
                                    is_unsafe: proto.is_unsafe,
                                    params: proto
                                        .params
                                        .iter()
                                        .map(|p| substitute(&p.ty, &subst))
                                        .collect(),
                                    ret: Box::new(substitute(&proto.return_type, &subst)),
                                };
                                return self.check_method_call(
                                    field, &method_ty, &recv_ty, base, args, span,
                                );
                            }
                            // Bounded type param but no matching trait method.
                            self.error(
                                format!(
                                    "no method '{}' on type parameter '{}': bounds = {:?}",
                                    field, type_name, bounds
                                ),
                                span.clone(),
                            );
                            return TypeExpr::Void;
                        }

                        // Concrete type with no matching method.
                        self.error(
                            format!("no method '{}' on type '{}'", field, type_name),
                            span.clone(),
                        );
                        return TypeExpr::Void;
                    }
                    // Fall through to the regular Field error path if the
                    // receiver type is not a struct or bounded type-param.
                }

                // Detect generic callees: look the identifier up directly so
                // we can read `generic_params`. Non-Ident callees (rare —
                // e.g. function pointer values) fall through to the existing
                // path which has no generics.
                let generic_params: Vec<String> = match callee.as_ref() {
                    Expr::Ident { name, .. } => match self.symbols.lookup(name).map(|s| &s.kind) {
                        Some(SymbolKind::Function { generic_params, .. }) => generic_params.clone(),
                        _ => Vec::new(),
                    },
                    _ => Vec::new(),
                };

                let callee_ty = self.infer_expr(callee);

                match callee_ty {
                    TypeExpr::Fn {
                        is_unsafe,
                        params,
                        ret,
                    } => {
                        // Check unsafe
                        if is_unsafe && !self.safety.is_unsafe() {
                            self.error_with_hint(
                                "call to unsafe function requires unsafe block".to_string(),
                                span.clone(),
                                "wrap the call in an unsafe block: unsafe { ... }",
                            );
                        }

                        // Check argument count
                        if args.len() != params.len() {
                            self.error(
                                format!("expected {} arguments, got {}", params.len(), args.len()),
                                span.clone(),
                            );
                        }

                        // Generic call: unify formal parameter types against
                        // actual arg types to build a type-substitution
                        // (`T -> i32`, …), then substitute in the return type.
                        if !generic_params.is_empty() {
                            let mut subst: HashMap<String, TypeExpr> = HashMap::new();
                            for (arg, param_ty) in args.iter().zip(params.iter()) {
                                let arg_ty = self.infer_expr(arg);
                                unify_generic(param_ty, &arg_ty, &generic_params, &mut subst);
                            }

                            // After unification, verify each formal type (post-substitution)
                            // is actually compatible with the actual arg type — catches
                            // cases like passing different types for the same T.
                            for (arg, param_ty) in args.iter().zip(params.iter()) {
                                let arg_ty = self.infer_expr(arg);
                                let expected = substitute(param_ty, &subst);
                                if !self.types_compatible(&expected, &arg_ty) {
                                    self.error_type_mismatch(&expected, &arg_ty, &arg.span());
                                }
                            }

                            return substitute(&ret, &subst);
                        }

                        // Check argument types
                        for (arg, param_ty) in args.iter().zip(params.iter()) {
                            let arg_ty = self.infer_expr(arg);
                            if !self.types_compatible(param_ty, &arg_ty) {
                                self.error_type_mismatch(param_ty, &arg_ty, &arg.span());
                            }
                        }

                        *ret
                    }
                    _ => {
                        self.error(
                            format!("cannot call non-function type {:?}", callee_ty),
                            span.clone(),
                        );
                        TypeExpr::Void
                    }
                }
            }

            Expr::Field { base, field, span } => {
                let base_ty = self.infer_expr(base);

                // Look up field in struct
                if let TypeExpr::Named(struct_name) = &base_ty {
                    // TODO: Look up struct definition and find field type
                    // For now, return void as placeholder
                    let _ = (struct_name, field, span);
                    TypeExpr::Void
                } else {
                    self.error(
                        format!("field access on non-struct type {:?}", base_ty),
                        span.clone(),
                    );
                    TypeExpr::Void
                }
            }

            Expr::Addr { operand, span } => {
                let operand_ty = self.infer_expr(operand);
                self.check_addressable(operand, span);
                TypeExpr::Ref(Box::new(operand_ty))
            }

            Expr::Deref { operand, span } => {
                let operand_ty = self.infer_expr(operand);

                match operand_ty {
                    TypeExpr::Ref(inner) | TypeExpr::Mref(inner) => *inner,
                    TypeExpr::Raw(inner) | TypeExpr::Rawm(inner) => {
                        // Deref of raw pointer requires unsafe
                        if !self.safety.is_unsafe() {
                            self.error(
                                "dereference of raw pointer requires unsafe block".to_string(),
                                span.clone(),
                            );
                        }
                        *inner
                    }
                    _ => {
                        self.error(
                            format!("cannot dereference non-pointer type {:?}", operand_ty),
                            span.clone(),
                        );
                        TypeExpr::Void
                    }
                }
            }

            Expr::At { base, index, span } => {
                let base_ty = self.infer_expr(base);
                let index_ty = self.infer_expr(index);

                // Index must be usize
                if !matches!(index_ty, TypeExpr::Primitive(PrimitiveType::Usize)) {
                    // Allow any integer for now
                    if !self.is_integer(&index_ty) {
                        self.error(
                            format!("index must be integer, got {:?}", index_ty),
                            span.clone(),
                        );
                    }
                }

                match base_ty {
                    TypeExpr::Slice(inner) => *inner,
                    TypeExpr::Arr(inner, _) => *inner,
                    _ => {
                        self.error(
                            format!("cannot index non-array type {:?}", base_ty),
                            span.clone(),
                        );
                        TypeExpr::Void
                    }
                }
            }

            Expr::Cast { ty, expr, span } => {
                let expr_ty = self.infer_expr(expr);

                // Check that cast is valid
                if !self.can_cast(&expr_ty, ty) {
                    self.error(
                        format!("cannot cast {:?} to {:?}", expr_ty, ty),
                        span.clone(),
                    );
                }

                ty.clone()
            }

            Expr::None { ty, .. } => TypeExpr::Opt(Box::new(ty.clone())),

            Expr::Some { value, .. } => {
                let inner_ty = self.infer_expr(value);
                TypeExpr::Opt(Box::new(inner_ty))
            }

            Expr::Ok { value, .. } => {
                let inner_ty = self.infer_expr(value);
                // We don't know the error type, use void as placeholder
                TypeExpr::Res(Box::new(inner_ty), Box::new(TypeExpr::Void))
            }

            Expr::Err { value, .. } => {
                let inner_ty = self.infer_expr(value);
                // We don't know the ok type, use void as placeholder
                TypeExpr::Res(Box::new(TypeExpr::Void), Box::new(inner_ty))
            }

            Expr::StructLit { name, .. } => TypeExpr::Named(name.clone()),
        }
    }

    // === Type compatibility checks ===

    fn types_compatible(&self, expected: &TypeExpr, actual: &TypeExpr) -> bool {
        match (expected, actual) {
            (TypeExpr::Void, TypeExpr::Void) => true,
            (TypeExpr::Primitive(a), TypeExpr::Primitive(b)) => a == b,
            (TypeExpr::Named(a), TypeExpr::Named(b)) => a == b,
            (TypeExpr::Ref(a), TypeExpr::Ref(b)) => self.types_compatible(a, b),
            (TypeExpr::Mref(a), TypeExpr::Mref(b)) => self.types_compatible(a, b),
            (TypeExpr::Raw(a), TypeExpr::Raw(b)) => self.types_compatible(a, b),
            (TypeExpr::Rawm(a), TypeExpr::Rawm(b)) => self.types_compatible(a, b),
            (TypeExpr::Own(a), TypeExpr::Own(b)) => self.types_compatible(a, b),
            (TypeExpr::Slice(a), TypeExpr::Slice(b)) => self.types_compatible(a, b),
            (TypeExpr::Arr(a, _), TypeExpr::Arr(b, _)) => self.types_compatible(a, b),
            (TypeExpr::Opt(a), TypeExpr::Opt(b)) => self.types_compatible(a, b),
            (TypeExpr::Res(a1, a2), TypeExpr::Res(b1, b2)) => {
                self.types_compatible(a1, b1) && self.types_compatible(a2, b2)
            }
            (
                TypeExpr::Fn {
                    is_unsafe: u1,
                    params: p1,
                    ret: r1,
                },
                TypeExpr::Fn {
                    is_unsafe: u2,
                    params: p2,
                    ret: r2,
                },
            ) => {
                u1 == u2
                    && p1.len() == p2.len()
                    && p1
                        .iter()
                        .zip(p2.iter())
                        .all(|(a, b)| self.types_compatible(a, b))
                    && self.types_compatible(r1, r2)
            }
            _ => false,
        }
    }

    fn is_bool(&self, ty: &TypeExpr) -> bool {
        matches!(ty, TypeExpr::Primitive(PrimitiveType::Bool))
    }

    fn is_integer(&self, ty: &TypeExpr) -> bool {
        matches!(
            ty,
            TypeExpr::Primitive(
                PrimitiveType::I8
                    | PrimitiveType::I16
                    | PrimitiveType::I32
                    | PrimitiveType::I64
                    | PrimitiveType::U8
                    | PrimitiveType::U16
                    | PrimitiveType::U32
                    | PrimitiveType::U64
                    | PrimitiveType::Usize
                    | PrimitiveType::Isize
            )
        )
    }

    fn is_numeric(&self, ty: &TypeExpr) -> bool {
        self.is_integer(ty)
            || matches!(
                ty,
                TypeExpr::Primitive(PrimitiveType::F32 | PrimitiveType::F64)
            )
    }

    fn can_cast(&self, from: &TypeExpr, to: &TypeExpr) -> bool {
        // Allow casts between numeric types
        if self.is_numeric(from) && self.is_numeric(to) {
            return true;
        }

        // Allow casts between pointer types
        matches!(
            (from, to),
            (TypeExpr::Ref(_), TypeExpr::Raw(_))
                | (TypeExpr::Mref(_), TypeExpr::Rawm(_))
                | (TypeExpr::Raw(_), TypeExpr::Raw(_))
                | (TypeExpr::Rawm(_), TypeExpr::Rawm(_))
        )
    }

    fn check_assignable(&mut self, expr: &Expr, span: &Span) {
        match expr {
            Expr::Ident { .. } => {}
            Expr::Deref { .. } => {}
            Expr::At { .. } => {}
            Expr::Field { .. } => {}
            _ => {
                self.error("expression is not assignable".to_string(), span.clone());
            }
        }
    }

    fn check_addressable(&mut self, expr: &Expr, span: &Span) {
        match expr {
            Expr::Ident { .. } => {}
            Expr::Deref { .. } => {}
            Expr::At { .. } => {}
            Expr::Field { .. } => {}
            _ => {
                self.error(
                    "cannot take address of expression".to_string(),
                    span.clone(),
                );
            }
        }
    }

    // === FFI validation ===

    /// Validate that a type is allowed in FFI signatures
    fn validate_ffi_type(&mut self, ty: &TypeExpr, span: &Span) {
        match ty {
            TypeExpr::Opt(_) => {
                self.error(
                    "opt(T) is not permitted in extern signatures".to_string(),
                    span.clone(),
                );
            }
            TypeExpr::Res(_, _) => {
                self.error(
                    "res(T, E) is not permitted in extern signatures".to_string(),
                    span.clone(),
                );
            }
            TypeExpr::Named(name) => {
                // Check if it's a struct passed by value without @repr(C)
                if let Some(struct_decl) = self.struct_decls.get(name) {
                    if struct_decl.repr != Some(Repr::C) {
                        self.error(
                            format!(
                                "struct '{}' passed by value in extern must have @repr(C)",
                                name
                            ),
                            span.clone(),
                        );
                    }
                }
                // Note: enums are allowed without @repr as they default to i32
            }
            // Primitives, pointers, slices, void are OK in FFI
            _ => {}
        }
    }

    // === Error helpers ===

    fn error(&mut self, message: String, span: Span) {
        self.errors
            .push(CompileError::type_error(message, span, self.source));
    }

    fn error_with_hint(&mut self, message: String, span: Span, hint: impl Into<String>) {
        self.errors.push(CompileError::type_error_with_hint(
            message,
            span,
            self.source,
            hint,
        ));
    }

    fn error_type_mismatch(&mut self, expected: &TypeExpr, actual: &TypeExpr, span: &Span) {
        self.error(
            format!("type mismatch: expected {:?}, got {:?}", expected, actual),
            span.clone(),
        );
    }
}

impl Default for TypeChecker<'_> {
    fn default() -> Self {
        Self::new("", SymbolTable::new())
    }
}

/// Extract the type-name string used to look up methods. Recognizes named
/// types and built-in primitives; strips one level of ref/mref.
pub(crate) fn type_name_of(ty: &TypeExpr) -> Option<String> {
    match ty {
        TypeExpr::Named(n) => Some(n.clone()),
        TypeExpr::Primitive(p) => Some(format!("{:?}", p).to_lowercase()),
        TypeExpr::Ref(inner) | TypeExpr::Mref(inner) => type_name_of(inner),
        _ => None,
    }
}

/// Walk a (formal, actual) type pair and record any type parameter bindings.
/// Used during generic call typecheck to infer `T = <concrete>` from the
/// shape of the actual argument.
///
/// Recurses into compound types so `slice(T)` against `slice(i32)` binds
/// `T -> i32`. Conflicting bindings silently take the first one; the
/// subsequent `types_compatible` check surfaces the mismatch as a regular
/// type error.
pub(crate) fn unify_generic(
    formal: &TypeExpr,
    actual: &TypeExpr,
    type_params: &[String],
    subst: &mut HashMap<String, TypeExpr>,
) {
    match (formal, actual) {
        // Type parameter at the leaf: record the binding.
        (TypeExpr::Named(name), concrete) if type_params.iter().any(|p| p == name) => {
            subst
                .entry(name.clone())
                .or_insert_with(|| concrete.clone());
        }
        // Recurse into matching compound shapes.
        (TypeExpr::Ref(f), TypeExpr::Ref(a))
        | (TypeExpr::Mref(f), TypeExpr::Mref(a))
        | (TypeExpr::Raw(f), TypeExpr::Raw(a))
        | (TypeExpr::Rawm(f), TypeExpr::Rawm(a))
        | (TypeExpr::Own(f), TypeExpr::Own(a))
        | (TypeExpr::Slice(f), TypeExpr::Slice(a))
        | (TypeExpr::Opt(f), TypeExpr::Opt(a)) => unify_generic(f, a, type_params, subst),
        (TypeExpr::Res(ft, fe), TypeExpr::Res(at, ae)) => {
            unify_generic(ft, at, type_params, subst);
            unify_generic(fe, ae, type_params, subst);
        }
        // No deeper structure to bind through; non-matching shapes will be
        // caught by the substituted compatibility check at the call site.
        _ => {}
    }
}

/// Replace every `Named(T)` whose name is in `subst` with the bound type.
/// Used to produce the concrete return type at a generic call site.
pub(crate) fn substitute(ty: &TypeExpr, subst: &HashMap<String, TypeExpr>) -> TypeExpr {
    match ty {
        TypeExpr::Named(name) => subst
            .get(name)
            .cloned()
            .unwrap_or(TypeExpr::Named(name.clone())),
        TypeExpr::NamedGeneric(name, args) => TypeExpr::NamedGeneric(
            name.clone(),
            args.iter().map(|a| substitute(a, subst)).collect(),
        ),
        TypeExpr::Ref(inner) => TypeExpr::Ref(Box::new(substitute(inner, subst))),
        TypeExpr::Mref(inner) => TypeExpr::Mref(Box::new(substitute(inner, subst))),
        TypeExpr::Raw(inner) => TypeExpr::Raw(Box::new(substitute(inner, subst))),
        TypeExpr::Rawm(inner) => TypeExpr::Rawm(Box::new(substitute(inner, subst))),
        TypeExpr::Own(inner) => TypeExpr::Own(Box::new(substitute(inner, subst))),
        TypeExpr::Slice(inner) => TypeExpr::Slice(Box::new(substitute(inner, subst))),
        TypeExpr::Arr(inner, n) => TypeExpr::Arr(Box::new(substitute(inner, subst)), n.clone()),
        TypeExpr::Opt(inner) => TypeExpr::Opt(Box::new(substitute(inner, subst))),
        TypeExpr::Res(t, e) => TypeExpr::Res(
            Box::new(substitute(t, subst)),
            Box::new(substitute(e, subst)),
        ),
        TypeExpr::Fn {
            is_unsafe,
            params,
            ret,
        } => TypeExpr::Fn {
            is_unsafe: *is_unsafe,
            params: params.iter().map(|p| substitute(p, subst)).collect(),
            ret: Box::new(substitute(ret, subst)),
        },
        TypeExpr::Primitive(_) | TypeExpr::Void => ty.clone(),
    }
}

#[cfg(test)]
mod tests {
    use crate::driver::compile;

    fn check_error(source: &str, expected_substr: &str) {
        let result = compile(source, "test.fc");
        assert!(result.is_err(), "Expected error for: {}", source);
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains(expected_substr),
            "Expected error containing '{}', got: {}",
            expected_substr,
            err_msg
        );
    }

    fn check_ok(source: &str) {
        let result = compile(source, "test.fc");
        assert!(
            result.is_ok(),
            "Expected success for: {}\nGot error: {:?}",
            source,
            result.err()
        );
    }

    // === Type mismatch tests ===

    #[test]
    fn test_type_mismatch_let() {
        check_error("fn foo() -> void { let x: i32 = true; }", "type mismatch");
    }

    #[test]
    fn test_type_mismatch_return() {
        check_error("fn foo() -> i32 { return true; }", "type mismatch");
    }

    #[test]
    fn test_type_mismatch_binary() {
        check_error("fn foo() -> i32 { return (1 + true); }", "type mismatch");
    }

    #[test]
    fn test_type_mismatch_assignment() {
        check_error(
            "fn foo() -> void { let x: i32 = 1; x = true; }",
            "type mismatch",
        );
    }

    // === Operator type tests ===

    #[test]
    fn test_logical_requires_bool() {
        check_error(
            "fn foo() -> bool { return (1 && 2); }",
            "logical operator requires bool",
        );
    }

    #[test]
    fn test_not_requires_bool() {
        check_error(
            "fn foo() -> bool { return !1; }",
            "logical not requires bool",
        );
    }

    #[test]
    fn test_condition_requires_bool() {
        check_error("fn foo() -> void { if (1) { } }", "condition must be bool");
    }

    #[test]
    fn test_while_condition_requires_bool() {
        check_error(
            "fn foo() -> void { while (1) { } }",
            "condition must be bool",
        );
    }

    // === Unsafe context tests ===

    #[test]
    fn test_unsafe_function_call_requires_unsafe() {
        check_error(
            "unsafe fn danger() -> i32 { return 1; } fn foo() -> i32 { return danger(); }",
            "call to unsafe function requires unsafe block",
        );
    }

    #[test]
    fn test_unsafe_function_call_in_unsafe_block() {
        check_ok(
            "unsafe fn danger() -> i32 { return 1; } fn foo() -> i32 { unsafe { return danger(); } }",
        );
    }

    #[test]
    fn test_unsafe_function_can_call_unsafe() {
        check_ok(
            "unsafe fn danger() -> i32 { return 1; } unsafe fn foo() -> i32 { return danger(); }",
        );
    }

    // === Function call tests ===

    #[test]
    fn test_wrong_argument_count() {
        check_error(
            "fn bar(x: i32, y: i32) -> i32 { return (x + y); } fn foo() -> i32 { return bar(1); }",
            "expected 2 arguments, got 1",
        );
    }

    #[test]
    fn test_wrong_argument_type() {
        check_error(
            "fn bar(x: i32) -> i32 { return x; } fn foo() -> i32 { return bar(true); }",
            "type mismatch",
        );
    }

    #[test]
    fn test_call_non_function() {
        // When calling a non-function, the type checker reports "cannot call non-function type"
        // but the return type mismatch may also be reported
        check_error(
            "fn foo() -> i32 { let x: i32 = 1; return x(); }",
            "cannot call non-function",
        );
    }

    // === Return type tests ===

    #[test]
    fn test_missing_return_value() {
        check_error("fn foo() -> i32 { return; }", "expected return value");
    }

    #[test]
    fn test_void_function_ok() {
        check_ok("fn foo() -> void { return; }");
    }

    #[test]
    fn test_void_function_implicit_return() {
        check_ok("fn foo() -> void { let x: i32 = 1; }");
    }

    // === Valid programs ===

    #[test]
    fn test_basic_arithmetic() {
        check_ok("fn foo() -> i32 { return (1 + 2); }");
    }

    #[test]
    fn test_comparison() {
        check_ok("fn foo() -> bool { return (1 < 2); }");
    }

    #[test]
    fn test_logical_ops() {
        check_ok("fn foo() -> bool { return (true && false); }");
    }

    #[test]
    fn test_function_call() {
        check_ok("fn bar(x: i32) -> i32 { return x; } fn foo() -> i32 { return bar(1); }");
    }

    #[test]
    fn test_nested_calls() {
        check_ok(
            "fn a(x: i32) -> i32 { return x; } fn b(x: i32) -> i32 { return a(x); } fn foo() -> i32 { return b(1); }",
        );
    }

    // === Generic fn tests (stage 0.9) ===

    #[test]
    fn test_generic_identity_inferred() {
        check_ok(
            "fn id[T](x: T) -> T { return x; } \
             fn main() -> i32 { return id(7); }",
        );
    }

    #[test]
    fn test_generic_two_instantiations() {
        check_ok(
            "fn id[T](x: T) -> T { return x; } \
             fn main() -> i32 { let a: i32 = id(1); let b: bool = id(true); return a; }",
        );
    }

    #[test]
    fn test_generic_two_params() {
        check_ok(
            "fn pick[A, B](a: A, b: B) -> A { return a; } \
             fn main() -> i32 { return pick(5, true); }",
        );
    }

    #[test]
    fn test_generic_return_type_mismatch_errors() {
        // `id(42)` infers T=i32 but the let binding expects bool.
        check_error(
            "fn id[T](x: T) -> T { return x; } \
             fn main() -> i32 { let b: bool = id(42); return 0; }",
            "type mismatch",
        );
    }

    // === Method-call tests (stage 1.0) ===

    #[test]
    fn test_method_call_ok() {
        check_ok(
            "struct P { x: i32, y: i32 } \
             impl P { fn x_value(self: ref(Self)) -> i32 { return 0; } } \
             fn main() -> i32 { let p: P = P { x: 1, y: 2 }; return p.x_value(); }",
        );
    }

    #[test]
    fn test_method_call_with_extra_args() {
        check_ok(
            "struct P { x: i32, y: i32 } \
             impl P { fn add(self: ref(Self), dx: i32) -> i32 { return dx; } } \
             fn main() -> i32 { let p: P = P { x: 1, y: 2 }; return p.add(7); }",
        );
    }

    #[test]
    fn test_unknown_method_errors() {
        check_error(
            "struct P { x: i32, y: i32 } \
             fn main() -> i32 { let p: P = P { x: 1, y: 2 }; return p.nope(); }",
            "no method 'nope'",
        );
    }

    #[test]
    fn test_method_arg_type_mismatch_errors() {
        check_error(
            "struct P { x: i32, y: i32 } \
             impl P { fn add(self: ref(Self), dx: i32) -> i32 { return dx; } } \
             fn main() -> i32 { let p: P = P { x: 1, y: 2 }; return p.add(true); }",
            "type mismatch",
        );
    }

    // === Trait + bounded generic tests (stage 1.0 slice 2) ===

    #[test]
    fn test_trait_bounded_call_ok() {
        check_ok(
            "trait Greeter { fn greet(self: ref(Self)) -> i32; } \
             struct P { x: i32, y: i32 } \
             impl Greeter for P { fn greet(self: ref(Self)) -> i32 { return 7; } } \
             fn shout[T: Greeter](x: T) -> i32 { return x.greet(); } \
             fn main() -> i32 { let p: P = P { x: 0, y: 0 }; return shout(p); }",
        );
    }

    #[test]
    fn test_trait_unsatisfied_bound_errors() {
        check_error(
            "trait Greeter { fn greet(self: ref(Self)) -> i32; } \
             struct P { x: i32, y: i32 } \
             struct Q { v: i32 } \
             impl Greeter for P { fn greet(self: ref(Self)) -> i32 { return 7; } } \
             fn shout[T: Greeter](x: T) -> i32 { return 0; } \
             fn main() -> i32 { let q: Q = Q { v: 1 }; return shout(q); }",
            "does not implement trait",
        );
    }

    #[test]
    fn test_trait_method_on_bounded_type_param_unknown() {
        // Calling a method that the bounded trait does not declare.
        check_error(
            "trait Greeter { fn greet(self: ref(Self)) -> i32; } \
             struct P { x: i32, y: i32 } \
             impl Greeter for P { fn greet(self: ref(Self)) -> i32 { return 7; } } \
             fn shout[T: Greeter](x: T) -> i32 { return x.nope(); } \
             fn main() -> i32 { let p: P = P { x: 0, y: 0 }; return shout(p); }",
            "no method 'nope'",
        );
    }

    // === Built-in trait tests (stage 1.0 slice 3) ===

    #[test]
    fn test_builtin_ord_on_i32() {
        check_ok(
            "fn max[T: Ord](a: T, b: T) -> T { \
                if (a.less_than(addr(b))) { return b; } \
                return a; \
             } \
             fn main() -> i32 { return max(7, 35); }",
        );
    }

    #[test]
    fn test_builtin_ord_on_f64() {
        check_ok(
            "fn max[T: Ord](a: T, b: T) -> T { \
                if (a.less_than(addr(b))) { return b; } \
                return a; \
             } \
             fn main() -> i32 { let f: f64 = max(1.5, 2.5); return 0; }",
        );
    }

    #[test]
    fn test_builtin_ord_on_bool_errors() {
        // `bool` deliberately does not implement Ord — only Eq + Copy.
        check_error(
            "fn max[T: Ord](a: T, b: T) -> T { return a; } \
             fn main() -> i32 { let x: bool = max(true, false); return 0; }",
            "does not implement trait 'Ord'",
        );
    }

    // === Drop trait tests (stage 1.0 slice 4) ===

    #[test]
    fn test_drop_impl_ok() {
        check_ok(
            "struct R { v: i32 } \
             impl Drop for R { fn drop(self: mref(Self)) -> void {} } \
             fn main() -> i32 { let r: R = R { v: 1 }; return 0; }",
        );
    }

    #[test]
    fn test_drop_without_impl_no_op() {
        // A struct without `impl Drop` simply has no drop call inserted —
        // it still typechecks fine.
        check_ok(
            "struct R { v: i32 } \
             fn main() -> i32 { let r: R = R { v: 1 }; return 0; }",
        );
    }

    // === Stdlib math module tests (stage 1.1 slice 1) ===

    #[test]
    fn test_math_min_max_i32() {
        check_ok(
            "use math::min; use math::max; \
             fn main() -> i32 { return (min(7, 35) + max(7, 35)); }",
        );
    }

    #[test]
    fn test_math_clamp_f64() {
        // Bounded-generic clamp works on any Ord-implementing primitive.
        check_ok(
            "use math::clamp; \
             fn main() -> i32 { let x: f64 = clamp(50.0, 1.0, 35.0); return 0; }",
        );
    }

    #[test]
    fn test_math_abs_i32() {
        check_ok(
            "use math::abs_i32; \
             fn main() -> i32 { return abs_i32(0 - 100); }",
        );
    }

    // === Doc comment tests (stage 1.1 slice 2) ===

    #[test]
    fn test_doc_comments_on_fn() {
        check_ok(
            "/// First line.\n\
             /// Second line.\n\
             fn add(a: i32, b: i32) -> i32 { return (a + b); }\n\
             fn main() -> i32 { return add(1, 2); }",
        );
    }

    #[test]
    fn test_doc_comments_on_struct() {
        check_ok(
            "/// Point doc.\n\
             struct P { x: i32, y: i32 }\n\
             fn main() -> i32 { let p: P = P { x: 1, y: 2 }; return 0; }",
        );
    }

    #[test]
    fn test_regular_comments_still_stripped() {
        // `//` (single-slash) comments must not appear as doc comments — they
        // continue to be stripped by `strip_comments` before parsing.
        check_ok(
            "// regular comment\n\
             fn main() -> i32 { return 0; }",
        );
    }

    // === Function-pointer tests (stage 1.1 slice 5) ===

    #[test]
    fn test_fn_ptr_pass_and_call() {
        check_ok(
            "fn dbl(x: i32) -> i32 { return (x * 2); } \
             fn apply(f: fn(i32) -> i32, x: i32) -> i32 { return f(x); } \
             fn main() -> i32 { return apply(dbl, 5); }",
        );
    }

    #[test]
    fn test_fn_ptr_local_binding() {
        check_ok(
            "fn add_one(x: i32) -> i32 { return (x + 1); } \
             fn main() -> i32 { \
                let f: fn(i32) -> i32 = add_one; \
                return f(41); \
             }",
        );
    }
}
