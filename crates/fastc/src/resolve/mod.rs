//! Name resolution pass
//!
//! This pass walks the AST and:
//! 1. Builds a symbol table with all definitions
//! 2. Resolves all name references to their declarations
//! 3. Reports undefined name errors

mod scope;

pub use scope::*;

use crate::ast::{
    Block, ConstExpr, Expr, ExternBlock, ExternItem, File, FnDecl, Item, Stmt, StructDecl,
    TypeExpr,
};
use crate::diag::CompileError;
use crate::lexer::Span;

/// Name resolver
pub struct Resolver<'a> {
    symbols: SymbolTable,
    source: &'a str,
    errors: Vec<CompileError>,
}

impl<'a> Resolver<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            symbols: SymbolTable::new(),
            source,
            errors: Vec::new(),
        }
    }

    /// Resolve names in a file
    pub fn resolve(&mut self, file: &File) -> Result<(), CompileError> {
        // First pass: collect all top-level declarations
        for item in &file.items {
            self.declare_item(item);
        }

        // Second pass: resolve all references
        for item in &file.items {
            self.resolve_item(item);
        }

        // Return all errors collected during resolution
        if !self.errors.is_empty() {
            Err(CompileError::multiple(std::mem::take(&mut self.errors)))
        } else {
            Ok(())
        }
    }

    /// Get the symbol table (for type checking)
    pub fn into_symbols(self) -> SymbolTable {
        self.symbols
    }

    // === First pass: Declare all items ===

    fn declare_item(&mut self, item: &Item) {
        match item {
            Item::Fn(fn_decl) => self.declare_fn(fn_decl),
            Item::Struct(struct_decl) => self.declare_struct(struct_decl),
            Item::Enum(enum_decl) => self.declare_enum(enum_decl),
            Item::Const(const_decl) => self.declare_const(const_decl),
            Item::Opaque(opaque_decl) => self.declare_opaque(opaque_decl),
            Item::Extern(extern_block) => self.declare_extern(extern_block),
            Item::Use(_) => {} // Use declarations handled later
            Item::Mod(mod_decl) => self.declare_mod(mod_decl),
        }
    }

    fn declare_mod(&mut self, mod_decl: &crate::ast::ModDecl) {
        // For inline modules (body is Some), declare all items from the module
        // This makes module items available in the current scope
        if let Some(body) = &mod_decl.body {
            for item in body {
                self.declare_item(item);
            }
        }
        // External modules (body is None) should have been expanded by ModuleLoader
    }

    fn declare_fn(&mut self, fn_decl: &FnDecl) {
        let param_types: Vec<TypeExpr> = fn_decl.params.iter().map(|p| p.ty.clone()).collect();
        let fn_type = fn_type(param_types, fn_decl.return_type.clone(), fn_decl.is_unsafe);

        let symbol = Symbol {
            name: fn_decl.name.clone(),
            kind: SymbolKind::Function {
                is_unsafe: fn_decl.is_unsafe,
            },
            ty: fn_type,
            span: fn_decl.span.clone(),
        };

        if let Err(sym) = self.symbols.define(symbol) {
            self.error_redefinition(&sym.name, &sym.span);
        }
    }

    fn declare_struct(&mut self, struct_decl: &StructDecl) {
        let symbol = Symbol {
            name: struct_decl.name.clone(),
            kind: SymbolKind::Struct,
            ty: TypeExpr::Named(struct_decl.name.clone()),
            span: struct_decl.span.clone(),
        };

        if let Err(sym) = self.symbols.define(symbol) {
            self.error_redefinition(&sym.name, &sym.span);
        }
    }

    fn declare_enum(&mut self, enum_decl: &crate::ast::EnumDecl) {
        let symbol = Symbol {
            name: enum_decl.name.clone(),
            kind: SymbolKind::Enum,
            ty: TypeExpr::Named(enum_decl.name.clone()),
            span: enum_decl.span.clone(),
        };

        if let Err(sym) = self.symbols.define(symbol) {
            self.error_redefinition(&sym.name, &sym.span);
        }

        // Declare each variant as a constant (e.g., Color_Red, Color_Green)
        for variant in &enum_decl.variants {
            let variant_name = format!("{}_{}", enum_decl.name, variant.name);
            let variant_symbol = Symbol {
                name: variant_name,
                kind: SymbolKind::Constant,
                ty: TypeExpr::Named(enum_decl.name.clone()),
                span: variant.span.clone(),
            };

            if let Err(sym) = self.symbols.define(variant_symbol) {
                self.error_redefinition(&sym.name, &sym.span);
            }
        }
    }

    fn declare_const(&mut self, const_decl: &crate::ast::ConstDecl) {
        let symbol = Symbol {
            name: const_decl.name.clone(),
            kind: SymbolKind::Constant,
            ty: const_decl.ty.clone(),
            span: const_decl.span.clone(),
        };

        if let Err(sym) = self.symbols.define(symbol) {
            self.error_redefinition(&sym.name, &sym.span);
        }
    }

    fn declare_opaque(&mut self, opaque_decl: &crate::ast::OpaqueDecl) {
        let symbol = Symbol {
            name: opaque_decl.name.clone(),
            kind: SymbolKind::Opaque,
            ty: TypeExpr::Named(opaque_decl.name.clone()),
            span: opaque_decl.span.clone(),
        };

        if let Err(sym) = self.symbols.define(symbol) {
            self.error_redefinition(&sym.name, &sym.span);
        }
    }

    fn declare_extern(&mut self, extern_block: &ExternBlock) {
        for item in &extern_block.items {
            match item {
                ExternItem::Fn(fn_proto) => {
                    let param_types: Vec<TypeExpr> =
                        fn_proto.params.iter().map(|p| p.ty.clone()).collect();
                    let fn_ty =
                        fn_type(param_types, fn_proto.return_type.clone(), fn_proto.is_unsafe);

                    let symbol = Symbol {
                        name: fn_proto.name.clone(),
                        kind: SymbolKind::Function {
                            is_unsafe: true, // All extern functions are unsafe to call
                        },
                        ty: fn_ty,
                        span: fn_proto.span.clone(),
                    };

                    if let Err(sym) = self.symbols.define(symbol) {
                        self.error_redefinition(&sym.name, &sym.span);
                    }
                }
                ExternItem::Struct(struct_decl) => self.declare_struct(struct_decl),
                ExternItem::Enum(enum_decl) => self.declare_enum(enum_decl),
                ExternItem::Opaque(opaque_decl) => self.declare_opaque(opaque_decl),
            }
        }
    }

    // === Second pass: Resolve references ===

    fn resolve_item(&mut self, item: &Item) {
        match item {
            Item::Fn(fn_decl) => self.resolve_fn(fn_decl),
            Item::Struct(struct_decl) => self.resolve_struct(struct_decl),
            Item::Enum(_) => {} // Enum variants don't reference other names
            Item::Const(const_decl) => self.resolve_const(const_decl),
            Item::Opaque(_) => {} // Opaque types don't reference other names
            Item::Extern(extern_block) => self.resolve_extern(extern_block),
            Item::Use(_) => {} // Use declarations are informational
            Item::Mod(mod_decl) => self.resolve_mod(mod_decl),
        }
    }

    fn resolve_mod(&mut self, mod_decl: &crate::ast::ModDecl) {
        // For inline modules, resolve all items in the module body
        if let Some(body) = &mod_decl.body {
            for item in body {
                self.resolve_item(item);
            }
        }
    }

    fn resolve_fn(&mut self, fn_decl: &FnDecl) {
        // Enter function scope
        self.symbols.enter_scope();

        // Declare parameters
        for param in &fn_decl.params {
            let symbol = Symbol {
                name: param.name.clone(),
                kind: SymbolKind::Variable,
                ty: param.ty.clone(),
                span: param.span.clone(),
            };

            if let Err(sym) = self.symbols.define(symbol) {
                self.error_redefinition(&sym.name, &sym.span);
            }

            // Resolve type references in parameter type
            self.resolve_type(&param.ty);
        }

        // Resolve return type
        self.resolve_type(&fn_decl.return_type);

        // Resolve function body
        self.resolve_block(&fn_decl.body);

        // Exit function scope
        self.symbols.exit_scope();
    }

    fn resolve_struct(&mut self, struct_decl: &StructDecl) {
        for field in &struct_decl.fields {
            self.resolve_type(&field.ty);
        }
    }

    fn resolve_const(&mut self, const_decl: &crate::ast::ConstDecl) {
        self.resolve_type(&const_decl.ty);
        self.resolve_const_expr(&const_decl.value);
    }

    fn resolve_extern(&mut self, extern_block: &ExternBlock) {
        for item in &extern_block.items {
            match item {
                ExternItem::Fn(fn_proto) => {
                    for param in &fn_proto.params {
                        self.resolve_type(&param.ty);
                    }
                    self.resolve_type(&fn_proto.return_type);
                }
                ExternItem::Struct(struct_decl) => self.resolve_struct(struct_decl),
                ExternItem::Enum(_) => {}
                ExternItem::Opaque(_) => {}
            }
        }
    }

    fn resolve_block(&mut self, block: &Block) {
        self.symbols.enter_scope();

        for stmt in &block.stmts {
            self.resolve_stmt(stmt);
        }

        self.symbols.exit_scope();
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                name,
                ty,
                init,
                span,
            } => {
                // Resolve the initializer first (can't reference the variable being declared)
                self.resolve_expr(init);
                self.resolve_type(ty);

                // Then declare the variable
                let symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable,
                    ty: ty.clone(),
                    span: span.clone(),
                };

                if let Err(sym) = self.symbols.define(symbol) {
                    self.error_redefinition(&sym.name, &sym.span);
                }
            }
            Stmt::Assign { lhs, rhs, .. } => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                self.resolve_expr(cond);
                self.resolve_block(then_block);
                if let Some(else_branch) = else_block {
                    match else_branch {
                        crate::ast::ElseBranch::ElseIf(if_stmt) => self.resolve_stmt(if_stmt),
                        crate::ast::ElseBranch::Else(block) => self.resolve_block(block),
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
                self.resolve_expr(expr);

                // The bound variable is only in scope in the then block
                self.symbols.enter_scope();
                // We don't know the exact type yet - type checker will figure it out
                let symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable,
                    ty: TypeExpr::Void, // Placeholder, will be refined by type checker
                    span: span.clone(),
                };
                let _ = self.symbols.define(symbol);

                for stmt in &then_block.stmts {
                    self.resolve_stmt(stmt);
                }
                self.symbols.exit_scope();

                if let Some(else_blk) = else_block {
                    self.resolve_block(else_blk);
                }
            }
            Stmt::While { cond, body, .. } => {
                self.resolve_expr(cond);
                self.resolve_block(body);
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                // For loop has its own scope for the init variable
                self.symbols.enter_scope();

                if let Some(init) = init {
                    match init {
                        crate::ast::ForInit::Let { name, ty, init } => {
                            self.resolve_expr(init);
                            self.resolve_type(ty);
                            let symbol = Symbol {
                                name: name.clone(),
                                kind: SymbolKind::Variable,
                                ty: ty.clone(),
                                span: 0..0, // No specific span for for-init
                            };
                            let _ = self.symbols.define(symbol);
                        }
                        crate::ast::ForInit::Assign { lhs, rhs } => {
                            self.resolve_expr(lhs);
                            self.resolve_expr(rhs);
                        }
                        crate::ast::ForInit::Call(expr) => {
                            self.resolve_expr(expr);
                        }
                    }
                }

                if let Some(cond) = cond {
                    self.resolve_expr(cond);
                }

                if let Some(step) = step {
                    match step {
                        crate::ast::ForStep::Assign { lhs, rhs } => {
                            self.resolve_expr(lhs);
                            self.resolve_expr(rhs);
                        }
                        crate::ast::ForStep::Call(expr) => {
                            self.resolve_expr(expr);
                        }
                    }
                }

                // Body is nested inside the for scope
                for stmt in &body.stmts {
                    self.resolve_stmt(stmt);
                }

                self.symbols.exit_scope();
            }
            Stmt::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                self.resolve_expr(expr);
                for case in cases {
                    self.resolve_const_expr(&case.value);
                    for stmt in &case.stmts {
                        self.resolve_stmt(stmt);
                    }
                }
                if let Some(default_stmts) = default {
                    for stmt in default_stmts {
                        self.resolve_stmt(stmt);
                    }
                }
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.resolve_expr(value);
                }
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::Defer { body, .. } => {
                self.resolve_block(body);
            }
            Stmt::Expr { expr, .. } => {
                self.resolve_expr(expr);
            }
            Stmt::Discard { expr, .. } => {
                self.resolve_expr(expr);
            }
            Stmt::Unsafe { body, .. } => {
                self.resolve_block(body);
            }
            Stmt::Block(block) => {
                self.resolve_block(block);
            }
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::IntLit { .. }
            | Expr::FloatLit { .. }
            | Expr::BoolLit { .. }
            | Expr::CStr { .. }
            | Expr::Bytes { .. } => {}

            Expr::Ident { name, span } => {
                if self.symbols.lookup(name).is_none() {
                    self.error_undefined(name, span);
                }
            }

            Expr::Binary { lhs, rhs, .. } => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
            Expr::Unary { operand, .. } => {
                self.resolve_expr(operand);
            }
            Expr::Paren { inner, .. } => {
                self.resolve_expr(inner);
            }
            Expr::Call { callee, args, .. } => {
                self.resolve_expr(callee);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            Expr::Field { base, .. } => {
                self.resolve_expr(base);
                // Field name resolution is done during type checking
            }
            Expr::Addr { operand, .. } => {
                self.resolve_expr(operand);
            }
            Expr::Deref { operand, .. } => {
                self.resolve_expr(operand);
            }
            Expr::At { base, index, .. } => {
                self.resolve_expr(base);
                self.resolve_expr(index);
            }
            Expr::Cast { ty, expr, .. } => {
                self.resolve_type(ty);
                self.resolve_expr(expr);
            }
            Expr::None { ty, .. } => {
                self.resolve_type(ty);
            }
            Expr::Some { value, .. } => {
                self.resolve_expr(value);
            }
            Expr::Ok { value, .. } => {
                self.resolve_expr(value);
            }
            Expr::Err { value, .. } => {
                self.resolve_expr(value);
            }
            Expr::StructLit { name, fields, span } => {
                // Check that the struct type exists
                if self.symbols.lookup(name).is_none() {
                    self.error_undefined(name, span);
                }
                for field in fields {
                    self.resolve_expr(&field.value);
                }
            }
        }
    }

    fn resolve_const_expr(&mut self, expr: &ConstExpr) {
        match expr {
            ConstExpr::IntLit(_)
            | ConstExpr::FloatLit(_)
            | ConstExpr::BoolLit(_)
            | ConstExpr::CStr(_)
            | ConstExpr::Bytes(_) => {}

            ConstExpr::Ident(name) => {
                // Const expressions can reference other constants
                if let Some(sym) = self.symbols.lookup(name) {
                    if sym.kind != SymbolKind::Constant {
                        self.errors.push(CompileError::resolve(
                            format!("'{}' is not a constant", name),
                            0..0,
                            self.source,
                        ));
                    }
                } else {
                    self.errors.push(CompileError::resolve(
                        format!("undefined constant '{}'", name),
                        0..0,
                        self.source,
                    ));
                }
            }

            ConstExpr::Binary { lhs, rhs, .. } => {
                self.resolve_const_expr(lhs);
                self.resolve_const_expr(rhs);
            }
            ConstExpr::Unary { operand, .. } => {
                self.resolve_const_expr(operand);
            }
            ConstExpr::Paren(inner) => {
                self.resolve_const_expr(inner);
            }
            ConstExpr::Cast { ty, expr } => {
                self.resolve_type(ty);
                self.resolve_const_expr(expr);
            }
        }
    }

    fn resolve_type(&mut self, ty: &TypeExpr) {
        match ty {
            TypeExpr::Primitive(_) | TypeExpr::Void => {}

            TypeExpr::Named(name) => {
                if let Some(sym) = self.symbols.lookup(name) {
                    match sym.kind {
                        SymbolKind::Struct | SymbolKind::Enum | SymbolKind::Opaque => {}
                        _ => {
                            self.errors.push(CompileError::resolve(
                                format!("'{}' is not a type", name),
                                0..0,
                                self.source,
                            ));
                        }
                    }
                } else {
                    self.errors.push(CompileError::resolve(
                        format!("undefined type '{}'", name),
                        0..0,
                        self.source,
                    ));
                }
            }

            TypeExpr::Ref(inner)
            | TypeExpr::Mref(inner)
            | TypeExpr::Raw(inner)
            | TypeExpr::Rawm(inner)
            | TypeExpr::Own(inner)
            | TypeExpr::Slice(inner)
            | TypeExpr::Opt(inner) => {
                self.resolve_type(inner);
            }

            TypeExpr::Arr(elem, _size) => {
                self.resolve_type(elem);
                // Size is a const expr, but typically just a literal
            }

            TypeExpr::Res(ok, err) => {
                self.resolve_type(ok);
                self.resolve_type(err);
            }

            TypeExpr::Fn { params, ret, .. } => {
                for param in params {
                    self.resolve_type(param);
                }
                self.resolve_type(ret);
            }
        }
    }

    // === Error helpers ===

    fn error_undefined(&mut self, name: &str, span: &Span) {
        // Try to find a similar name to suggest
        let hint = self.find_similar_name(name).map(|similar| {
            format!("did you mean '{}'?", similar)
        });

        if let Some(hint) = hint {
            self.errors.push(CompileError::resolve_with_hint(
                format!("undefined name '{}'", name),
                span.clone(),
                self.source,
                hint,
            ));
        } else {
            self.errors.push(CompileError::resolve(
                format!("undefined name '{}'", name),
                span.clone(),
                self.source,
            ));
        }
    }

    fn error_redefinition(&mut self, name: &str, span: &Span) {
        self.errors.push(CompileError::resolve(
            format!("redefinition of '{}'", name),
            span.clone(),
            self.source,
        ));
    }

    /// Find a similar name in the symbol table (for "did you mean" hints)
    fn find_similar_name(&self, target: &str) -> Option<String> {
        let all_names = self.symbols.all_names();
        let mut best_match: Option<(String, usize)> = None;

        for name in all_names {
            let dist = Self::edit_distance(target, &name);
            // Only suggest if the edit distance is reasonable (e.g., <= 3)
            // and the name is somewhat similar in length
            if dist <= 3 && dist < target.len() {
                match &best_match {
                    None => best_match = Some((name, dist)),
                    Some((_, best_dist)) if dist < *best_dist => {
                        best_match = Some((name, dist))
                    }
                    _ => {}
                }
            }
        }

        best_match.map(|(name, _)| name)
    }

    /// Calculate Levenshtein edit distance between two strings
    fn edit_distance(a: &str, b: &str) -> usize {
        let a_len = a.len();
        let b_len = b.len();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        let mut prev_row: Vec<usize> = (0..=b_len).collect();
        let mut curr_row: Vec<usize> = vec![0; b_len + 1];

        for i in 0..a_len {
            curr_row[0] = i + 1;

            for j in 0..b_len {
                let cost = if a_chars[i] == b_chars[j] { 0 } else { 1 };
                curr_row[j + 1] = (prev_row[j + 1] + 1)
                    .min(curr_row[j] + 1)
                    .min(prev_row[j] + cost);
            }

            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        prev_row[b_len]
    }
}

impl Default for Resolver<'_> {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn test_undefined_variable() {
        check_error(
            "fn foo() -> i32 { return x; }",
            "undefined name 'x'",
        );
    }

    #[test]
    fn test_undefined_function() {
        check_error(
            "fn foo() -> i32 { return bar(); }",
            "undefined name 'bar'",
        );
    }

    #[test]
    fn test_redefinition_variable() {
        check_error(
            "fn foo() -> i32 { let x: i32 = 1; let x: i32 = 2; return x; }",
            "redefinition of 'x'",
        );
    }

    #[test]
    fn test_redefinition_function() {
        check_error(
            "fn foo() -> void {} fn foo() -> void {}",
            "redefinition of 'foo'",
        );
    }

    #[test]
    fn test_undefined_type() {
        check_error(
            "fn foo(x: Foo) -> void {}",
            "undefined type 'Foo'",
        );
    }

    #[test]
    fn test_variable_used_before_declaration() {
        check_error(
            "fn foo() -> i32 { let y: i32 = x; let x: i32 = 1; return y; }",
            "undefined name 'x'",
        );
    }

    #[test]
    fn test_parameter_in_scope() {
        check_ok("fn foo(x: i32) -> i32 { return x; }");
    }

    #[test]
    fn test_nested_scope() {
        check_ok(
            "fn foo() -> i32 { let x: i32 = 1; { let y: i32 = x; return y; } }",
        );
    }

    #[test]
    fn test_inner_scope_variable_not_visible() {
        check_error(
            "fn foo() -> i32 { { let x: i32 = 1; } return x; }",
            "undefined name 'x'",
        );
    }

    #[test]
    fn test_function_forward_reference() {
        // Functions should be able to call other functions declared later
        check_ok("fn foo() -> i32 { return bar(); } fn bar() -> i32 { return 1; }");
    }
}
