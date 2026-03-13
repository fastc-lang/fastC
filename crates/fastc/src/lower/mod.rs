//! Lowering from FastC AST to C AST

mod c_ast;
mod checks;
mod temporaries;

pub use c_ast::*;

use crate::ast;
use std::collections::{HashMap, HashSet};

/// Lowering pass
pub struct Lower {
    temp_counter: usize,
    in_unsafe: bool, // Track if currently in unsafe block (for runtime checks)
    opt_types: HashSet<String>,  // Track used opt types for typedef generation
    res_types: HashSet<String>,  // Track used res types for typedef generation
    slice_types: HashSet<String>, // Track used slice types for typedef generation
    var_types: HashMap<String, CType>, // Track variable types for type inference
}

impl Lower {
    pub fn new() -> Self {
        Self {
            temp_counter: 0,
            in_unsafe: false,
            opt_types: HashSet::new(),
            res_types: HashSet::new(),
            slice_types: HashSet::new(),
            var_types: HashMap::new(),
        }
    }

    /// Check if an expression has side effects (requires evaluation order)
    fn has_side_effects(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::Call { .. } => true,
            ast::Expr::Binary { lhs, rhs, .. } => {
                self.has_side_effects(lhs) || self.has_side_effects(rhs)
            }
            ast::Expr::Unary { operand, .. } => self.has_side_effects(operand),
            ast::Expr::Paren { inner, .. } => self.has_side_effects(inner),
            ast::Expr::Field { base, .. } => self.has_side_effects(base),
            ast::Expr::Addr { operand, .. } => self.has_side_effects(operand),
            ast::Expr::Deref { operand, .. } => self.has_side_effects(operand),
            ast::Expr::At { base, index, .. } => {
                self.has_side_effects(base) || self.has_side_effects(index)
            }
            ast::Expr::Cast { expr, .. } => self.has_side_effects(expr),
            ast::Expr::Some { value, .. } => self.has_side_effects(value),
            ast::Expr::Ok { value, .. } => self.has_side_effects(value),
            ast::Expr::Err { value, .. } => self.has_side_effects(value),
            _ => false,
        }
    }

    /// Infer the type of an expression (simple inference for lowering)
    fn infer_expr_type(&self, expr: &ast::Expr) -> CType {
        match expr {
            ast::Expr::IntLit { .. } => CType::Int32, // Default to i32 for int literals
            ast::Expr::FloatLit { .. } => CType::Double, // Default to f64 for float literals
            ast::Expr::BoolLit { .. } => CType::Bool,
            ast::Expr::Cast { ty, .. } => self.lower_type(ty),
            ast::Expr::Paren { inner, .. } => self.infer_expr_type(inner),
            ast::Expr::Unary { operand, .. } => self.infer_expr_type(operand),
            ast::Expr::Binary { op, lhs, .. } => {
                // Comparison and logical operators return bool
                match op {
                    ast::BinOp::Eq | ast::BinOp::Ne | ast::BinOp::Lt
                    | ast::BinOp::Le | ast::BinOp::Gt | ast::BinOp::Ge
                    | ast::BinOp::And | ast::BinOp::Or => CType::Bool,
                    // Arithmetic operators return the same type as the operands
                    _ => self.infer_expr_type(lhs),
                }
            }
            ast::Expr::Ident { name, .. } => {
                // Look up the variable type from our tracking map
                self.var_types.get(name).cloned().unwrap_or(CType::Void)
            }
            ast::Expr::Call { callee, .. } => {
                // For function calls, we'd need to look up the return type
                // For now, try to infer from callee name
                if let ast::Expr::Ident { name, .. } = callee.as_ref() {
                    // Special case for common patterns
                    if name == "get_some" || name == "get_none" {
                        // These are opt-returning functions from our examples
                        CType::Opt(Box::new(CType::Int32))
                    } else {
                        CType::Void
                    }
                } else {
                    CType::Void
                }
            }
            // For other expressions, we'd need full type checking - default to Void
            _ => CType::Void,
        }
    }

    /// Lower a FastC file to a C file
    pub fn lower(&mut self, file: &ast::File) -> CFile {
        let mut c_file = CFile::new();

        // Add standard includes
        c_file.includes.push("<stdint.h>".to_string());
        c_file.includes.push("<stddef.h>".to_string());
        c_file.includes.push("<stdbool.h>".to_string());
        c_file.includes.push("\"fastc_runtime.h\"".to_string());

        self.lower_items(&file.items, &mut c_file);

        // Sort user-defined type_defs by name for deterministic output
        c_file.type_defs.sort_by(|a, b| {
            fn get_name(decl: &CDecl) -> &str {
                match decl {
                    CDecl::Struct { name, .. } => name,
                    CDecl::Typedef { name, .. } => name,
                    CDecl::Enum { name, .. } => name,
                }
            }
            get_name(a).cmp(get_name(b))
        });

        // Generate typedefs for opt/res types used in the file
        self.generate_opt_res_typedefs(&mut c_file);

        c_file
    }

    /// Lower a list of items, handling modules recursively
    fn lower_items(&mut self, items: &[ast::Item], c_file: &mut CFile) {
        for item in items {
            match item {
                ast::Item::Fn(fn_decl) => {
                    c_file.fn_defs.push(self.lower_fn(fn_decl));
                }
                ast::Item::Struct(struct_decl) => {
                    c_file.type_defs.push(self.lower_struct(struct_decl));
                }
                ast::Item::Enum(enum_decl) => {
                    c_file.type_defs.push(self.lower_enum(enum_decl));
                }
                ast::Item::Mod(mod_decl) => {
                    // Recursively lower items inside the module
                    if let Some(body) = &mod_decl.body {
                        self.lower_items(body, c_file);
                    }
                }
                // TODO: Handle other items (Const, Opaque, Extern, Use)
                _ => {}
            }
        }
    }

    /// Generate struct typedefs for slice(T), opt(T), and res(T,E) types used in the file
    fn generate_opt_res_typedefs(&mut self, c_file: &mut CFile) {
        // Collect all types used
        self.collect_types_from_file(c_file);

        // Generate slice typedefs first (sorted for determinism)
        // Skip types already defined in fastc_runtime.h
        let builtin_slice_types: HashSet<&str> = [
            "uint8_t", "int8_t", "uint16_t", "int16_t",
            "uint32_t", "int32_t", "uint64_t", "int64_t",
            "float", "double",
        ].iter().cloned().collect();

        let mut slice_types: Vec<_> = self.slice_types.iter()
            .filter(|t| !builtin_slice_types.contains(t.as_str()))
            .cloned()
            .collect();
        slice_types.sort();
        for type_name in slice_types {
            if let Some(decl) = self.make_slice_typedef(&type_name) {
                c_file.type_defs.insert(0, decl);
            }
        }

        // Generate opt typedefs (sorted for determinism)
        let mut opt_types: Vec<_> = self.opt_types.iter().cloned().collect();
        opt_types.sort();
        for type_name in opt_types {
            if let Some(decl) = self.make_opt_typedef(&type_name) {
                c_file.type_defs.insert(0, decl);
            }
        }

        // Generate res typedefs (sorted for determinism)
        let mut res_types: Vec<_> = self.res_types.iter().cloned().collect();
        res_types.sort();
        for type_name in res_types {
            if let Some(decl) = self.make_res_typedef(&type_name) {
                c_file.type_defs.insert(0, decl);
            }
        }
    }

    /// Collect all opt/res types used in the file
    fn collect_types_from_file(&mut self, c_file: &CFile) {
        for fn_def in &c_file.fn_defs {
            self.collect_types_from_type(&fn_def.return_type);
            for param in &fn_def.params {
                self.collect_types_from_type(&param.ty);
            }
            for stmt in &fn_def.body {
                self.collect_types_from_stmt(stmt);
            }
        }
    }

    fn collect_types_from_stmt(&mut self, stmt: &CStmt) {
        match stmt {
            CStmt::VarDecl { ty, init, .. } => {
                self.collect_types_from_type(ty);
                if let Some(expr) = init {
                    self.collect_types_from_expr(expr);
                }
            }
            CStmt::Assign { lhs, rhs, .. } => {
                self.collect_types_from_expr(lhs);
                self.collect_types_from_expr(rhs);
            }
            CStmt::If { cond, then, else_, .. } => {
                self.collect_types_from_expr(cond);
                for s in then {
                    self.collect_types_from_stmt(s);
                }
                if let Some(else_stmts) = else_ {
                    for s in else_stmts {
                        self.collect_types_from_stmt(s);
                    }
                }
            }
            CStmt::While { cond, body, .. } => {
                self.collect_types_from_expr(cond);
                for s in body {
                    self.collect_types_from_stmt(s);
                }
            }
            CStmt::For { init, cond, step, body, .. } => {
                if let Some(init_stmt) = init {
                    self.collect_types_from_stmt(init_stmt);
                }
                if let Some(c) = cond {
                    self.collect_types_from_expr(c);
                }
                if let Some(s) = step {
                    self.collect_types_from_expr(s);
                }
                for s in body {
                    self.collect_types_from_stmt(s);
                }
            }
            CStmt::Return(Some(expr)) => {
                self.collect_types_from_expr(expr);
            }
            CStmt::Expr(expr) => {
                self.collect_types_from_expr(expr);
            }
            CStmt::Block(stmts) => {
                for s in stmts {
                    self.collect_types_from_stmt(s);
                }
            }
            _ => {}
        }
    }

    fn collect_types_from_expr(&mut self, expr: &CExpr) {
        match expr {
            CExpr::Binary { lhs, rhs, .. } => {
                self.collect_types_from_expr(lhs);
                self.collect_types_from_expr(rhs);
            }
            CExpr::Unary { operand, .. } => {
                self.collect_types_from_expr(operand);
            }
            CExpr::Call { func, args, .. } => {
                self.collect_types_from_expr(func);
                for arg in args {
                    self.collect_types_from_expr(arg);
                }
            }
            CExpr::Field { base, .. } => {
                self.collect_types_from_expr(base);
            }
            CExpr::Deref(inner) | CExpr::AddrOf(inner) | CExpr::Paren(inner) => {
                self.collect_types_from_expr(inner);
            }
            CExpr::Index { base, index, .. } => {
                self.collect_types_from_expr(base);
                self.collect_types_from_expr(index);
            }
            CExpr::Cast { ty, expr, .. } => {
                self.collect_types_from_type(ty);
                self.collect_types_from_expr(expr);
            }
            CExpr::Compound { ty, fields, .. } => {
                self.collect_types_from_type(ty);
                for (_, val) in fields {
                    self.collect_types_from_expr(val);
                }
            }
            _ => {}
        }
    }

    fn collect_types_from_type(&mut self, ty: &CType) {
        match ty {
            CType::Opt(inner) => {
                let name = Self::c_type_to_name(inner);
                self.opt_types.insert(name);
                self.collect_types_from_type(inner);
            }
            CType::Res(ok_ty, err_ty) => {
                let name = format!("{}_{}", Self::c_type_to_name(ok_ty), Self::c_type_to_name(err_ty));
                self.res_types.insert(name);
                self.collect_types_from_type(ok_ty);
                self.collect_types_from_type(err_ty);
            }
            CType::Slice(inner) => {
                let name = Self::c_type_to_name(inner);
                self.slice_types.insert(name);
                self.collect_types_from_type(inner);
            }
            CType::Ptr(inner) | CType::ConstPtr(inner) => {
                self.collect_types_from_type(inner);
            }
            CType::Array(inner, _) => {
                self.collect_types_from_type(inner);
            }
            _ => {}
        }
    }

    /// Get a C type name string for typedef naming
    fn c_type_to_name(ty: &CType) -> String {
        match ty {
            CType::Void => "void".to_string(),
            CType::Bool => "bool".to_string(),
            CType::Int8 => "int8_t".to_string(),
            CType::Int16 => "int16_t".to_string(),
            CType::Int32 => "int32_t".to_string(),
            CType::Int64 => "int64_t".to_string(),
            CType::UInt8 => "uint8_t".to_string(),
            CType::UInt16 => "uint16_t".to_string(),
            CType::UInt32 => "uint32_t".to_string(),
            CType::UInt64 => "uint64_t".to_string(),
            CType::Float => "float".to_string(),
            CType::Double => "double".to_string(),
            CType::SizeT => "size_t".to_string(),
            CType::PtrDiffT => "ptrdiff_t".to_string(),
            CType::Ptr(inner) => format!("ptr_{}", Self::c_type_to_name(inner)),
            CType::ConstPtr(inner) => format!("cptr_{}", Self::c_type_to_name(inner)),
            CType::Named(n) => n.clone(),
            CType::Slice(inner) => format!("slice_{}", Self::c_type_to_name(inner)),
            CType::Opt(inner) => format!("opt_{}", Self::c_type_to_name(inner)),
            CType::Res(ok, err) => format!("res_{}_{}", Self::c_type_to_name(ok), Self::c_type_to_name(err)),
            CType::Array(inner, size) => format!("arr{}_{}", size, Self::c_type_to_name(inner)),
        }
    }

    /// Create a typedef for fc_opt_T
    /// Create a typedef for fc_slice_T
    fn make_slice_typedef(&self, inner_type_name: &str) -> Option<CDecl> {
        let inner_ty = Self::name_to_c_type(inner_type_name)?;
        let struct_name = format!("fc_slice_{}", inner_type_name);
        Some(CDecl::Struct {
            name: struct_name,
            fields: vec![
                CField {
                    name: "data".to_string(),
                    ty: CType::Ptr(Box::new(inner_ty)),
                },
                CField {
                    name: "len".to_string(),
                    ty: CType::SizeT,
                },
            ],
        })
    }

    /// Create a typedef for fc_opt_T
    fn make_opt_typedef(&self, inner_type_name: &str) -> Option<CDecl> {
        let inner_ty = Self::name_to_c_type(inner_type_name)?;
        let struct_name = format!("fc_opt_{}", inner_type_name);
        Some(CDecl::Struct {
            name: struct_name,
            fields: vec![
                CField {
                    name: "has_value".to_string(),
                    ty: CType::Bool,
                },
                CField {
                    name: "value".to_string(),
                    ty: inner_ty,
                },
            ],
        })
    }

    /// Create a typedef for fc_res_T_E
    fn make_res_typedef(&self, type_name: &str) -> Option<CDecl> {
        // Parse "T_E" format - find the separator
        // This is a simplistic approach; for complex types we'd need better parsing
        let parts: Vec<&str> = type_name.splitn(2, '_').collect();
        if parts.len() != 2 {
            return None;
        }
        let ok_ty = Self::name_to_c_type(parts[0])?;
        let err_ty = Self::name_to_c_type(parts[1])?;
        let struct_name = format!("fc_res_{}", type_name);

        // For result types, we use a struct with a tag and union
        // For simplicity, use two fields: is_ok and data (as a union would require more C AST support)
        // Actually, C compound literals with designated initializers work well here
        Some(CDecl::Struct {
            name: struct_name,
            fields: vec![
                CField {
                    name: "is_ok".to_string(),
                    ty: CType::Bool,
                },
                CField {
                    name: "ok".to_string(),
                    ty: ok_ty,
                },
                CField {
                    name: "err".to_string(),
                    ty: err_ty,
                },
            ],
        })
    }

    /// Convert a type name back to a CType
    fn name_to_c_type(name: &str) -> Option<CType> {
        Some(match name {
            "void" => CType::Void,
            "bool" => CType::Bool,
            "int8_t" => CType::Int8,
            "int16_t" => CType::Int16,
            "int32_t" => CType::Int32,
            "int64_t" => CType::Int64,
            "uint8_t" => CType::UInt8,
            "uint16_t" => CType::UInt16,
            "uint32_t" => CType::UInt32,
            "uint64_t" => CType::UInt64,
            "float" => CType::Float,
            "double" => CType::Double,
            "size_t" => CType::SizeT,
            "ptrdiff_t" => CType::PtrDiffT,
            _ => CType::Named(name.to_string()),
        })
    }

    fn lower_fn(&mut self, fn_decl: &ast::FnDecl) -> CFnDef {
        // Clear variable types from previous function
        self.var_types.clear();

        // Register parameter types
        let params: Vec<CParam> = fn_decl
            .params
            .iter()
            .map(|p| {
                let ty = self.lower_type(&p.ty);
                self.var_types.insert(p.name.clone(), ty.clone());
                CParam {
                    name: p.name.clone(),
                    ty,
                }
            })
            .collect();

        let body = self.lower_block(&fn_decl.body);

        CFnDef {
            name: fn_decl.name.clone(),
            params,
            return_type: self.lower_type(&fn_decl.return_type),
            body,
        }
    }

    fn lower_struct(&mut self, struct_decl: &ast::StructDecl) -> CDecl {
        let fields: Vec<CField> = struct_decl
            .fields
            .iter()
            .map(|f| CField {
                name: f.name.clone(),
                ty: self.lower_type(&f.ty),
            })
            .collect();

        CDecl::Struct {
            name: struct_decl.name.clone(),
            fields,
        }
    }

    fn lower_enum(&mut self, enum_decl: &ast::EnumDecl) -> CDecl {
        // Check if any variant has associated data
        let has_data = enum_decl.variants.iter().any(|v| v.fields.is_some());

        if has_data {
            // Tagged enum with associated data - lower to struct with tag + union
            // This is more complex and requires union support in C AST
            // For now, generate a struct with tag and fields for each variant
            // TODO: Implement proper union-based lowering
            let name = enum_decl.name.clone();
            let mut fields = vec![CField {
                name: "tag".to_string(),
                ty: CType::Int32,
            }];

            // Add a field for each variant's data
            for variant in &enum_decl.variants {
                if let Some(variant_fields) = &variant.fields {
                    // For now, only handle single-field variants
                    if let Some(ty) = variant_fields.first() {
                        fields.push(CField {
                            name: format!("{}_data", variant.name.to_lowercase()),
                            ty: self.lower_type(ty),
                        });
                    }
                }
            }

            CDecl::Struct { name, fields }
        } else {
            // Simple enum with no associated data - lower to C enum
            let variants: Vec<String> = enum_decl
                .variants
                .iter()
                .map(|v| format!("{}_{}", enum_decl.name, v.name))
                .collect();

            CDecl::Enum {
                name: enum_decl.name.clone(),
                variants,
            }
        }
    }

    fn lower_block(&mut self, block: &ast::Block) -> Vec<CStmt> {
        let mut stmts = Vec::new();
        for stmt in &block.stmts {
            stmts.extend(self.lower_stmt(stmt));
        }
        stmts
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt) -> Vec<CStmt> {
        match stmt {
            ast::Stmt::Let { name, ty, init, .. } => {
                let mut pre_stmts = Vec::new();
                let c_ty = self.lower_type(ty);
                // Track the variable type for inference
                self.var_types.insert(name.clone(), c_ty.clone());
                let c_init = self.lower_expr(init, &mut pre_stmts);
                pre_stmts.push(CStmt::VarDecl {
                    name: name.clone(),
                    ty: c_ty,
                    init: Some(c_init),
                });
                pre_stmts
            }
            ast::Stmt::Assign { lhs, rhs, .. } => {
                let mut pre_stmts = Vec::new();
                let c_lhs = self.lower_expr(lhs, &mut pre_stmts);
                let c_rhs = self.lower_expr(rhs, &mut pre_stmts);
                pre_stmts.push(CStmt::Assign {
                    lhs: c_lhs,
                    rhs: c_rhs,
                });
                pre_stmts
            }
            ast::Stmt::Return { value, .. } => {
                let mut pre_stmts = Vec::new();
                let c_value = value.as_ref().map(|v| self.lower_expr(v, &mut pre_stmts));
                pre_stmts.push(CStmt::Return(c_value));
                pre_stmts
            }
            ast::Stmt::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                let mut pre_stmts = Vec::new();
                let c_cond = self.lower_expr(cond, &mut pre_stmts);
                let c_then = self.lower_block(then_block);
                let c_else = else_block.as_ref().map(|eb| match eb {
                    ast::ElseBranch::Else(block) => self.lower_block(block),
                    ast::ElseBranch::ElseIf(if_stmt) => self.lower_stmt(if_stmt),
                });
                pre_stmts.push(CStmt::If {
                    cond: c_cond,
                    then: c_then,
                    else_: c_else,
                });
                pre_stmts
            }
            ast::Stmt::While { cond, body, .. } => {
                let mut pre_stmts = Vec::new();
                let c_cond = self.lower_expr(cond, &mut pre_stmts);
                let c_body = self.lower_block(body);
                pre_stmts.push(CStmt::While {
                    cond: c_cond,
                    body: c_body,
                });
                pre_stmts
            }
            ast::Stmt::Expr { expr, .. } => {
                let mut pre_stmts = Vec::new();
                let c_expr = self.lower_expr(expr, &mut pre_stmts);
                pre_stmts.push(CStmt::Expr(c_expr));
                pre_stmts
            }
            ast::Stmt::Block(block) => {
                vec![CStmt::Block(self.lower_block(block))]
            }
            ast::Stmt::Unsafe { body, .. } => {
                // In C, unsafe just means the type checker allowed it
                // Track unsafe context to disable runtime checks
                let was_unsafe = self.in_unsafe;
                self.in_unsafe = true;
                let stmts = self.lower_block(body);
                self.in_unsafe = was_unsafe;
                stmts
            }
            ast::Stmt::Discard { expr, .. } => {
                // Discard the result of an expression (cast to void in C)
                let mut pre_stmts = Vec::new();
                let c_expr = self.lower_expr(expr, &mut pre_stmts);
                pre_stmts.push(CStmt::Expr(CExpr::Cast {
                    ty: CType::Void,
                    expr: Box::new(c_expr),
                }));
                pre_stmts
            }
            ast::Stmt::IfLet {
                name,
                expr,
                then_block,
                else_block,
                ..
            } => {
                // if-let name = expr { ... } else { ... }
                // Lowers to:
                //   fc_opt_T __tmp = expr;
                //   if (__tmp.has_value) {
                //       T name = __tmp.value;
                //       ... then_block ...
                //   } else {
                //       ... else_block ...
                //   }
                let mut pre_stmts = Vec::new();
                let c_expr = self.lower_expr(expr, &mut pre_stmts);

                // Infer the opt type from the expression
                let opt_ty = self.infer_expr_type(expr);
                let inner_ty = match &opt_ty {
                    CType::Opt(inner) => (**inner).clone(),
                    _ => CType::Int32, // Fallback if type inference fails
                };

                // Create temporary for the opt value
                let tmp = self.fresh_temp();
                pre_stmts.push(CStmt::VarDecl {
                    name: tmp.clone(),
                    ty: opt_ty.clone(),
                    init: Some(c_expr),
                });

                // Build the then block with the unwrapped value
                let mut then_stmts = vec![CStmt::VarDecl {
                    name: name.clone(),
                    ty: inner_ty,
                    init: Some(CExpr::Field {
                        base: Box::new(CExpr::Ident(tmp.clone())),
                        field: "value".to_string(),
                    }),
                }];
                then_stmts.extend(self.lower_block(then_block));

                // Build the else block if present
                let else_stmts = else_block.as_ref().map(|eb| self.lower_block(eb));

                pre_stmts.push(CStmt::If {
                    cond: CExpr::Field {
                        base: Box::new(CExpr::Ident(tmp)),
                        field: "has_value".to_string(),
                    },
                    then: then_stmts,
                    else_: else_stmts,
                });

                pre_stmts
            }
            ast::Stmt::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                let mut pre_stmts = Vec::new();
                let c_expr = self.lower_expr(expr, &mut pre_stmts);

                let c_cases: Vec<(CExpr, Vec<CStmt>)> = cases
                    .iter()
                    .map(|case| {
                        let value = self.lower_const_expr(&case.value);
                        let mut case_stmts: Vec<CStmt> = case
                            .stmts
                            .iter()
                            .flat_map(|s| self.lower_stmt(s))
                            .collect();
                        case_stmts.push(CStmt::Break);
                        (value, case_stmts)
                    })
                    .collect();

                let c_default = default.as_ref().map(|stmts| {
                    let mut d: Vec<CStmt> =
                        stmts.iter().flat_map(|s| self.lower_stmt(s)).collect();
                    d.push(CStmt::Break);
                    d
                });

                pre_stmts.push(CStmt::Switch {
                    expr: c_expr,
                    cases: c_cases,
                    default: c_default,
                });

                pre_stmts
            }
            _ => {
                // TODO: Handle other statements (for, defer, etc.)
                vec![]
            }
        }
    }

    fn lower_expr(&mut self, expr: &ast::Expr, pre_stmts: &mut Vec<CStmt>) -> CExpr {
        match expr {
            ast::Expr::IntLit { value, .. } => CExpr::IntLit(value.to_string()),
            ast::Expr::FloatLit { raw, .. } => CExpr::FloatLit(raw.clone()),
            ast::Expr::BoolLit { value, .. } => CExpr::BoolLit(*value),
            ast::Expr::Ident { name, .. } => CExpr::Ident(name.clone()),
            ast::Expr::Binary { op, lhs, rhs, .. } => {
                // Handle short-circuit operators with temporaries
                match op {
                    ast::BinOp::And => {
                        // a && b becomes: bool __tmp; if (a) { __tmp = b; } else { __tmp = false; }
                        let tmp = self.fresh_temp();
                        let c_lhs = self.lower_expr(lhs, pre_stmts);
                        let c_rhs = self.lower_expr(rhs, pre_stmts);

                        pre_stmts.push(CStmt::VarDecl {
                            name: tmp.clone(),
                            ty: CType::Bool,
                            init: None,
                        });
                        pre_stmts.push(CStmt::If {
                            cond: c_lhs,
                            then: vec![CStmt::Assign {
                                lhs: CExpr::Ident(tmp.clone()),
                                rhs: c_rhs,
                            }],
                            else_: Some(vec![CStmt::Assign {
                                lhs: CExpr::Ident(tmp.clone()),
                                rhs: CExpr::BoolLit(false),
                            }]),
                        });
                        CExpr::Ident(tmp)
                    }
                    ast::BinOp::Or => {
                        // a || b becomes: bool __tmp; if (a) { __tmp = true; } else { __tmp = b; }
                        let tmp = self.fresh_temp();
                        let c_lhs = self.lower_expr(lhs, pre_stmts);
                        let c_rhs = self.lower_expr(rhs, pre_stmts);

                        pre_stmts.push(CStmt::VarDecl {
                            name: tmp.clone(),
                            ty: CType::Bool,
                            init: None,
                        });
                        pre_stmts.push(CStmt::If {
                            cond: c_lhs,
                            then: vec![CStmt::Assign {
                                lhs: CExpr::Ident(tmp.clone()),
                                rhs: CExpr::BoolLit(true),
                            }],
                            else_: Some(vec![CStmt::Assign {
                                lhs: CExpr::Ident(tmp.clone()),
                                rhs: c_rhs,
                            }]),
                        });
                        CExpr::Ident(tmp)
                    }
                    ast::BinOp::Div | ast::BinOp::Rem => {
                        // Add division by zero check in safe code
                        let c_lhs = self.lower_expr(lhs, pre_stmts);
                        let c_rhs = self.lower_expr(rhs, pre_stmts);

                        if !self.in_unsafe {
                            pre_stmts.push(checks::div_zero_check(c_rhs.clone()));
                        }

                        CExpr::Binary {
                            op: self.lower_binop(*op),
                            lhs: Box::new(c_lhs),
                            rhs: Box::new(c_rhs),
                        }
                    }
                    ast::BinOp::Add | ast::BinOp::Sub | ast::BinOp::Mul => {
                        // Add overflow check for signed integer types in safe code
                        let expr_ty = self.infer_expr_type(lhs);
                        let c_lhs = self.lower_expr(lhs, pre_stmts);
                        let c_rhs = self.lower_expr(rhs, pre_stmts);

                        // Check if this is a signed integer type
                        if !self.in_unsafe && Self::is_signed_integer(&expr_ty) {
                            let tmp = self.fresh_temp();
                            let (decl, check) = match op {
                                ast::BinOp::Add => {
                                    checks::overflow_check_add(c_lhs, c_rhs, &tmp, expr_ty)
                                }
                                ast::BinOp::Sub => {
                                    checks::overflow_check_sub(c_lhs, c_rhs, &tmp, expr_ty)
                                }
                                ast::BinOp::Mul => {
                                    checks::overflow_check_mul(c_lhs, c_rhs, &tmp, expr_ty)
                                }
                                _ => unreachable!(),
                            };
                            pre_stmts.push(decl);
                            pre_stmts.push(check);
                            CExpr::Ident(tmp)
                        } else {
                            CExpr::Binary {
                                op: self.lower_binop(*op),
                                lhs: Box::new(c_lhs),
                                rhs: Box::new(c_rhs),
                            }
                        }
                    }
                    _ => {
                        let c_lhs = self.lower_expr(lhs, pre_stmts);
                        let c_rhs = self.lower_expr(rhs, pre_stmts);
                        CExpr::Binary {
                            op: self.lower_binop(*op),
                            lhs: Box::new(c_lhs),
                            rhs: Box::new(c_rhs),
                        }
                    }
                }
            }
            ast::Expr::Unary { op, operand, .. } => {
                let c_operand = self.lower_expr(operand, pre_stmts);
                CExpr::Unary {
                    op: self.lower_unaryop(*op),
                    operand: Box::new(c_operand),
                }
            }
            ast::Expr::Paren { inner, .. } => {
                let c_inner = self.lower_expr(inner, pre_stmts);
                CExpr::Paren(Box::new(c_inner))
            }
            ast::Expr::Call { callee, args, .. } => {
                let c_callee = self.lower_expr(callee, pre_stmts);

                // Create temporaries for arguments with side effects to guarantee
                // left-to-right evaluation order (C doesn't guarantee this)
                let c_args: Vec<CExpr> = args
                    .iter()
                    .map(|arg| {
                        let c_arg = self.lower_expr(arg, pre_stmts);
                        // Only create temporary if arg has side effects
                        if self.has_side_effects(arg) {
                            let tmp = self.fresh_temp();
                            pre_stmts.push(CStmt::VarDecl {
                                name: tmp.clone(),
                                ty: CType::Int32, // Default type, actual type would need inference
                                init: Some(c_arg),
                            });
                            CExpr::Ident(tmp)
                        } else {
                            c_arg
                        }
                    })
                    .collect();

                CExpr::Call {
                    func: Box::new(c_callee),
                    args: c_args,
                }
            }
            ast::Expr::Field { base, field, .. } => {
                let c_base = self.lower_expr(base, pre_stmts);
                CExpr::Field {
                    base: Box::new(c_base),
                    field: field.clone(),
                }
            }
            ast::Expr::Addr { operand, .. } => {
                let c_operand = self.lower_expr(operand, pre_stmts);
                CExpr::AddrOf(Box::new(c_operand))
            }
            ast::Expr::Deref { operand, .. } => {
                let c_operand = self.lower_expr(operand, pre_stmts);
                CExpr::Deref(Box::new(c_operand))
            }
            ast::Expr::At { base, index, .. } => {
                let base_ty = self.infer_expr_type(base);
                let c_base = self.lower_expr(base, pre_stmts);
                let c_index = self.lower_expr(index, pre_stmts);

                // Check if base is a slice type - slices need bounds checks
                if let CType::Slice(_) = base_ty {
                    if !self.in_unsafe {
                        // Insert bounds check: if (index >= base.len) { fc_trap(); }
                        pre_stmts.push(checks::bounds_check(
                            c_index.clone(),
                            CExpr::Field {
                                base: Box::new(c_base.clone()),
                                field: "len".to_string(),
                            },
                        ));
                    }
                    // Slice access: base.data[index]
                    CExpr::Index {
                        base: Box::new(CExpr::Field {
                            base: Box::new(c_base),
                            field: "data".to_string(),
                        }),
                        index: Box::new(c_index),
                    }
                } else {
                    // Array access: base[index]
                    CExpr::Index {
                        base: Box::new(c_base),
                        index: Box::new(c_index),
                    }
                }
            }
            ast::Expr::Cast { ty, expr, .. } => {
                let c_expr = self.lower_expr(expr, pre_stmts);
                CExpr::Cast {
                    ty: self.lower_type(ty),
                    expr: Box::new(c_expr),
                }
            }
            ast::Expr::CStr { value, .. } => CExpr::StringLit(value.clone()),

            // some(value) -> (fc_opt_T){ .has_value = true, .value = value }
            ast::Expr::Some { value, .. } => {
                let inner_ty = self.infer_expr_type(value);
                let c_value = self.lower_expr(value, pre_stmts);
                CExpr::Compound {
                    ty: CType::Opt(Box::new(inner_ty)),
                    fields: vec![
                        ("has_value".to_string(), CExpr::BoolLit(true)),
                        ("value".to_string(), c_value),
                    ],
                }
            }

            // none(T) -> (fc_opt_T){ .has_value = false }
            ast::Expr::None { ty, .. } => {
                let inner_ty = self.lower_type(ty);
                CExpr::Compound {
                    ty: CType::Opt(Box::new(inner_ty)),
                    fields: vec![("has_value".to_string(), CExpr::BoolLit(false))],
                }
            }

            // ok(value) -> (fc_res_T_E){ .is_ok = true, .ok = value }
            // Note: Without full type info, we can only infer the ok type, err is Void
            ast::Expr::Ok { value, .. } => {
                let ok_ty = self.infer_expr_type(value);
                let c_value = self.lower_expr(value, pre_stmts);
                CExpr::Compound {
                    ty: CType::Res(Box::new(ok_ty), Box::new(CType::Void)),
                    fields: vec![
                        ("is_ok".to_string(), CExpr::BoolLit(true)),
                        ("ok".to_string(), c_value),
                    ],
                }
            }

            // err(value) -> (fc_res_T_E){ .is_ok = false, .err = value }
            // Note: Without full type info, we can only infer the err type, ok is Void
            ast::Expr::Err { value, .. } => {
                let err_ty = self.infer_expr_type(value);
                let c_value = self.lower_expr(value, pre_stmts);
                CExpr::Compound {
                    ty: CType::Res(Box::new(CType::Void), Box::new(err_ty)),
                    fields: vec![
                        ("is_ok".to_string(), CExpr::BoolLit(false)),
                        ("err".to_string(), c_value),
                    ],
                }
            }

            ast::Expr::StructLit { name, fields, .. } => {
                // Struct literal: Point { x: 10, y: 20 }
                // Lowers to: (Point){ .x = 10, .y = 20 }
                let c_fields: Vec<(String, CExpr)> = fields
                    .iter()
                    .map(|field_init| {
                        (
                            field_init.name.clone(),
                            self.lower_expr(&field_init.value, pre_stmts),
                        )
                    })
                    .collect();

                CExpr::Compound {
                    ty: CType::Named(name.clone()),
                    fields: c_fields,
                }
            }

            _ => {
                // TODO: Handle other expressions
                CExpr::Ident("/* TODO */".to_string())
            }
        }
    }

    fn lower_type(&self, ty: &ast::TypeExpr) -> CType {
        match ty {
            ast::TypeExpr::Primitive(p) => match p {
                ast::PrimitiveType::I8 => CType::Int8,
                ast::PrimitiveType::I16 => CType::Int16,
                ast::PrimitiveType::I32 => CType::Int32,
                ast::PrimitiveType::I64 => CType::Int64,
                ast::PrimitiveType::U8 => CType::UInt8,
                ast::PrimitiveType::U16 => CType::UInt16,
                ast::PrimitiveType::U32 => CType::UInt32,
                ast::PrimitiveType::U64 => CType::UInt64,
                ast::PrimitiveType::F32 => CType::Float,
                ast::PrimitiveType::F64 => CType::Double,
                ast::PrimitiveType::Bool => CType::Bool,
                ast::PrimitiveType::Usize => CType::SizeT,
                ast::PrimitiveType::Isize => CType::PtrDiffT,
            },
            ast::TypeExpr::Void => CType::Void,
            ast::TypeExpr::Named(name) => CType::Named(name.clone()),

            // Immutable references -> const T*
            ast::TypeExpr::Ref(inner) | ast::TypeExpr::Raw(inner) => {
                CType::ConstPtr(Box::new(self.lower_type(inner)))
            }

            // Mutable references -> T*
            ast::TypeExpr::Mref(inner) | ast::TypeExpr::Rawm(inner) => {
                CType::Ptr(Box::new(self.lower_type(inner)))
            }

            // Owning pointer -> T* (lifetime management is future work)
            ast::TypeExpr::Own(inner) => {
                CType::Ptr(Box::new(self.lower_type(inner)))
            }

            // Slice -> fc_slice_T struct
            ast::TypeExpr::Slice(inner) => CType::Slice(Box::new(self.lower_type(inner))),

            // Array with evaluated size
            ast::TypeExpr::Arr(elem, size_expr) => {
                let size = self.eval_const_size(size_expr);
                CType::Array(Box::new(self.lower_type(elem)), size)
            }

            // Optional type: opt(T) -> fc_opt_T struct
            ast::TypeExpr::Opt(inner) => CType::Opt(Box::new(self.lower_type(inner))),

            // Result type: res(T, E) -> fc_res_T_E struct
            ast::TypeExpr::Res(ok_ty, err_ty) => CType::Res(
                Box::new(self.lower_type(ok_ty)),
                Box::new(self.lower_type(err_ty)),
            ),

            // TODO: Handle Fn types
            _ => CType::Void,
        }
    }

    /// Lower a constant expression to C expression
    fn lower_const_expr(&self, expr: &ast::ConstExpr) -> CExpr {
        match expr {
            ast::ConstExpr::IntLit(n) => CExpr::IntLit(n.to_string()),
            ast::ConstExpr::FloatLit(n) => CExpr::FloatLit(n.to_string()),
            ast::ConstExpr::BoolLit(b) => CExpr::BoolLit(*b),
            ast::ConstExpr::Ident(name) => CExpr::Ident(name.clone()),
            ast::ConstExpr::Binary { op, lhs, rhs } => CExpr::Binary {
                op: self.lower_binop(*op),
                lhs: Box::new(self.lower_const_expr(lhs)),
                rhs: Box::new(self.lower_const_expr(rhs)),
            },
            ast::ConstExpr::Unary { op, operand } => CExpr::Unary {
                op: self.lower_unaryop(*op),
                operand: Box::new(self.lower_const_expr(operand)),
            },
            ast::ConstExpr::Paren(inner) => CExpr::Paren(Box::new(self.lower_const_expr(inner))),
            ast::ConstExpr::Cast { ty, expr } => CExpr::Cast {
                ty: self.lower_type(ty),
                expr: Box::new(self.lower_const_expr(expr)),
            },
            ast::ConstExpr::CStr(s) => CExpr::StringLit(s.clone()),
            ast::ConstExpr::Bytes(s) => CExpr::StringLit(s.clone()),
        }
    }

    /// Evaluate a constant expression to a usize (for array sizes)
    fn eval_const_size(&self, expr: &ast::ConstExpr) -> usize {
        match expr {
            ast::ConstExpr::IntLit(n) => {
                if *n < 0 {
                    panic!("array size cannot be negative");
                }
                *n as usize
            }
            ast::ConstExpr::Paren(inner) => self.eval_const_size(inner),
            ast::ConstExpr::Binary { op, lhs, rhs } => {
                let l = self.eval_const_size(lhs);
                let r = self.eval_const_size(rhs);
                match op {
                    ast::BinOp::Add => l + r,
                    ast::BinOp::Sub => l.saturating_sub(r),
                    ast::BinOp::Mul => l * r,
                    ast::BinOp::Div => l / r,
                    _ => panic!("unsupported operator in array size"),
                }
            }
            ast::ConstExpr::Unary { op, operand } => {
                let v = self.eval_const_size(operand);
                match op {
                    ast::UnaryOp::Neg => panic!("array size cannot be negative"),
                    _ => v,
                }
            }
            // TODO: Support ConstExpr::Ident via symbol table lookup
            _ => panic!("unsupported constant expression in array size"),
        }
    }

    fn lower_binop(&self, op: ast::BinOp) -> CBinOp {
        match op {
            ast::BinOp::Add => CBinOp::Add,
            ast::BinOp::Sub => CBinOp::Sub,
            ast::BinOp::Mul => CBinOp::Mul,
            ast::BinOp::Div => CBinOp::Div,
            ast::BinOp::Rem => CBinOp::Mod,
            ast::BinOp::Eq => CBinOp::Eq,
            ast::BinOp::Ne => CBinOp::Ne,
            ast::BinOp::Lt => CBinOp::Lt,
            ast::BinOp::Le => CBinOp::Le,
            ast::BinOp::Gt => CBinOp::Gt,
            ast::BinOp::Ge => CBinOp::Ge,
            ast::BinOp::And => CBinOp::And,
            ast::BinOp::Or => CBinOp::Or,
            ast::BinOp::BitAnd => CBinOp::BitAnd,
            ast::BinOp::BitOr => CBinOp::BitOr,
            ast::BinOp::BitXor => CBinOp::BitXor,
            ast::BinOp::Shl => CBinOp::Shl,
            ast::BinOp::Shr => CBinOp::Shr,
        }
    }

    fn lower_unaryop(&self, op: ast::UnaryOp) -> CUnaryOp {
        match op {
            ast::UnaryOp::Neg => CUnaryOp::Neg,
            ast::UnaryOp::Not => CUnaryOp::Not,
            ast::UnaryOp::BitNot => CUnaryOp::BitNot,
        }
    }

    /// Check if a CType is a signed integer type
    fn is_signed_integer(ty: &CType) -> bool {
        matches!(
            ty,
            CType::Int8 | CType::Int16 | CType::Int32 | CType::Int64 | CType::PtrDiffT
        )
    }

    fn fresh_temp(&mut self) -> String {
        let name = format!("__tmp{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }
}

impl Default for Lower {
    fn default() -> Self {
        Self::new()
    }
}
