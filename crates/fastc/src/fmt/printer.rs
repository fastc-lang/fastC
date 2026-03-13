//! Pretty printer for FastC AST

use crate::ast::{
    BinOp, Block, Case, ConstDecl, ConstExpr, ElseBranch, EnumDecl, Expr, ExternBlock, ExternItem,
    Field, FieldInit, File, FnDecl, FnProto, ForInit, ForStep, Item, ModDecl, OpaqueDecl, Param,
    PrimitiveType, Repr, Stmt, StructDecl, TypeExpr, UnaryOp, UseDecl, UseItems, Variant,
};

/// Formatter for FastC source code
pub struct Formatter {
    output: String,
    indent: usize,
}

impl Formatter {
    const INDENT_WIDTH: usize = 4;

    /// Create a new formatter
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    /// Finish formatting and return the output
    pub fn finish(self) -> String {
        self.output
    }

    /// Write a string to the output
    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    /// Write indentation
    fn write_indent(&mut self) {
        for _ in 0..self.indent * Self::INDENT_WIDTH {
            self.output.push(' ');
        }
    }

    /// Write a newline followed by indentation
    fn newline(&mut self) {
        self.output.push('\n');
    }

    /// Write a line with indentation
    fn line(&mut self, s: &str) {
        self.write_indent();
        self.write(s);
        self.newline();
    }

    /// Write a comment (preserving original format)
    pub fn write_comment(&mut self, comment: &str) {
        self.write(comment);
        self.newline();
    }

    /// Format a complete file
    pub fn format_file(&mut self, file: &File) {
        for (i, item) in file.items.iter().enumerate() {
            if i > 0 {
                self.newline(); // Blank line between top-level items
            }
            self.format_item(item);
        }
    }

    /// Format a top-level item
    fn format_item(&mut self, item: &Item) {
        match item {
            Item::Fn(decl) => self.format_fn(decl),
            Item::Struct(decl) => self.format_struct(decl),
            Item::Enum(decl) => self.format_enum(decl),
            Item::Const(decl) => self.format_const(decl),
            Item::Opaque(decl) => self.format_opaque(decl),
            Item::Extern(block) => self.format_extern(block),
            Item::Use(decl) => self.format_use(decl),
            Item::Mod(decl) => self.format_mod(decl),
        }
    }

    /// Format a function declaration
    fn format_fn(&mut self, decl: &FnDecl) {
        self.write_indent();
        if decl.is_unsafe {
            self.write("unsafe ");
        }
        self.write("fn ");
        self.write(&decl.name);
        self.write("(");
        self.format_params(&decl.params);
        self.write(")");

        if !matches!(decl.return_type, TypeExpr::Void) {
            self.write(" -> ");
            self.format_type(&decl.return_type);
        }

        self.write(" ");
        self.format_block(&decl.body);
        self.newline();
    }

    /// Format function parameters
    fn format_params(&mut self, params: &[Param]) {
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write(&param.name);
            self.write(": ");
            self.format_type(&param.ty);
        }
    }

    /// Format a struct declaration
    fn format_struct(&mut self, decl: &StructDecl) {
        if let Some(repr) = &decl.repr {
            self.format_repr(repr);
        }
        self.write_indent();
        self.write("struct ");
        self.write(&decl.name);
        self.write(" {");
        self.newline();

        self.indent += 1;
        for field in &decl.fields {
            self.format_field(field);
        }
        self.indent -= 1;

        self.line("}");
    }

    /// Format a struct field
    fn format_field(&mut self, field: &Field) {
        self.write_indent();
        self.write(&field.name);
        self.write(": ");
        self.format_type(&field.ty);
        self.write(",");
        self.newline();
    }

    /// Format an enum declaration
    fn format_enum(&mut self, decl: &EnumDecl) {
        if let Some(repr) = &decl.repr {
            self.format_repr(repr);
        }
        self.write_indent();
        self.write("enum ");
        self.write(&decl.name);
        self.write(" {");
        self.newline();

        self.indent += 1;
        for variant in &decl.variants {
            self.format_variant(variant);
        }
        self.indent -= 1;

        self.line("}");
    }

    /// Format an enum variant
    fn format_variant(&mut self, variant: &Variant) {
        self.write_indent();
        self.write(&variant.name);
        if let Some(fields) = &variant.fields {
            self.write("(");
            for (i, ty) in fields.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.format_type(ty);
            }
            self.write(")");
        }
        self.write(",");
        self.newline();
    }

    /// Format a constant declaration
    fn format_const(&mut self, decl: &ConstDecl) {
        self.write_indent();
        self.write("const ");
        self.write(&decl.name);
        self.write(": ");
        self.format_type(&decl.ty);
        self.write(" = ");
        self.format_const_expr(&decl.value);
        self.write(";");
        self.newline();
    }

    /// Format an opaque type declaration
    fn format_opaque(&mut self, decl: &OpaqueDecl) {
        self.write_indent();
        self.write("opaque ");
        self.write(&decl.name);
        self.write(";");
        self.newline();
    }

    /// Format an extern block
    fn format_extern(&mut self, block: &ExternBlock) {
        self.write_indent();
        self.write("extern \"");
        self.write(&block.abi);
        self.write("\" {");
        self.newline();

        self.indent += 1;
        for item in &block.items {
            self.format_extern_item(item);
        }
        self.indent -= 1;

        self.line("}");
    }

    /// Format an extern item
    fn format_extern_item(&mut self, item: &ExternItem) {
        match item {
            ExternItem::Fn(proto) => self.format_fn_proto(proto),
            ExternItem::Struct(decl) => self.format_struct(decl),
            ExternItem::Enum(decl) => self.format_enum(decl),
            ExternItem::Opaque(decl) => self.format_opaque(decl),
        }
    }

    /// Format a use declaration
    fn format_use(&mut self, decl: &UseDecl) {
        self.write_indent();
        self.write("use ");

        // Write the path
        for (i, segment) in decl.path.iter().enumerate() {
            if i > 0 {
                self.write("::");
            }
            self.write(segment);
        }

        // Write the items
        match &decl.items {
            UseItems::Single(item) => {
                if !decl.path.is_empty() {
                    self.write("::");
                }
                self.write(item);
            }
            UseItems::Multiple(items) => {
                self.write("::{");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(item);
                }
                self.write("}");
            }
            UseItems::Glob => {
                self.write("::*");
            }
            UseItems::Module => {
                // Just the path, nothing extra
            }
        }

        self.write(";");
        self.newline();
    }

    /// Format a mod declaration
    fn format_mod(&mut self, decl: &ModDecl) {
        self.write_indent();
        if decl.is_pub {
            self.write("pub ");
        }
        self.write("mod ");
        self.write(&decl.name);

        match &decl.body {
            Some(items) => {
                self.write(" {");
                self.newline();
                self.indent += 1;
                for item in items {
                    self.format_item(item);
                }
                self.indent -= 1;
                self.line("}");
            }
            None => {
                self.write(";");
                self.newline();
            }
        }
    }

    /// Format a function prototype
    fn format_fn_proto(&mut self, proto: &FnProto) {
        self.write_indent();
        if proto.is_unsafe {
            self.write("unsafe ");
        }
        self.write("fn ");
        self.write(&proto.name);
        self.write("(");
        self.format_params(&proto.params);
        self.write(")");

        if !matches!(proto.return_type, TypeExpr::Void) {
            self.write(" -> ");
            self.format_type(&proto.return_type);
        }

        self.write(";");
        self.newline();
    }

    /// Format a repr attribute
    fn format_repr(&mut self, repr: &Repr) {
        self.write_indent();
        self.write("@repr(");
        self.write(match repr {
            Repr::C => "C",
            Repr::I8 => "i8",
            Repr::U8 => "u8",
            Repr::I16 => "i16",
            Repr::U16 => "u16",
            Repr::I32 => "i32",
            Repr::U32 => "u32",
            Repr::I64 => "i64",
            Repr::U64 => "u64",
        });
        self.write(")");
        self.newline();
    }

    /// Format a type expression
    fn format_type(&mut self, ty: &TypeExpr) {
        match ty {
            TypeExpr::Primitive(p) => self.write(primitive_name(*p)),
            TypeExpr::Named(name) => self.write(name),
            TypeExpr::Ref(inner) => {
                self.write("ref(");
                self.format_type(inner);
                self.write(")");
            }
            TypeExpr::Mref(inner) => {
                self.write("mref(");
                self.format_type(inner);
                self.write(")");
            }
            TypeExpr::Raw(inner) => {
                self.write("raw(");
                self.format_type(inner);
                self.write(")");
            }
            TypeExpr::Rawm(inner) => {
                self.write("rawm(");
                self.format_type(inner);
                self.write(")");
            }
            TypeExpr::Own(inner) => {
                self.write("own(");
                self.format_type(inner);
                self.write(")");
            }
            TypeExpr::Slice(inner) => {
                self.write("slice(");
                self.format_type(inner);
                self.write(")");
            }
            TypeExpr::Arr(inner, len) => {
                self.write("arr(");
                self.format_type(inner);
                self.write(", ");
                self.format_const_expr(len);
                self.write(")");
            }
            TypeExpr::Opt(inner) => {
                self.write("opt(");
                self.format_type(inner);
                self.write(")");
            }
            TypeExpr::Res(ok, err) => {
                self.write("res(");
                self.format_type(ok);
                self.write(", ");
                self.format_type(err);
                self.write(")");
            }
            TypeExpr::Fn {
                is_unsafe,
                params,
                ret,
            } => {
                if *is_unsafe {
                    self.write("unsafe ");
                }
                self.write("fn(");
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.format_type(param);
                }
                self.write(")");
                if !matches!(ret.as_ref(), TypeExpr::Void) {
                    self.write(" -> ");
                    self.format_type(ret);
                }
            }
            TypeExpr::Void => self.write("void"),
        }
    }

    /// Format a block
    fn format_block(&mut self, block: &Block) {
        self.write("{");
        if block.stmts.is_empty() {
            self.write("}");
            return;
        }
        self.newline();

        self.indent += 1;
        for stmt in &block.stmts {
            self.format_stmt(stmt);
        }
        self.indent -= 1;

        self.write_indent();
        self.write("}");
    }

    /// Format a statement
    fn format_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, ty, init, .. } => {
                self.write_indent();
                self.write("let ");
                self.write(name);
                self.write(": ");
                self.format_type(ty);
                self.write(" = ");
                self.format_expr(init);
                self.write(";");
                self.newline();
            }
            Stmt::Assign { lhs, rhs, .. } => {
                self.write_indent();
                self.format_expr(lhs);
                self.write(" = ");
                self.format_expr(rhs);
                self.write(";");
                self.newline();
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                self.write_indent();
                self.write("if (");
                self.format_expr(cond);
                self.write(") ");
                self.format_block(then_block);
                if let Some(else_branch) = else_block {
                    self.write(" else ");
                    match else_branch {
                        ElseBranch::ElseIf(stmt) => {
                            self.format_stmt_inline(stmt);
                        }
                        ElseBranch::Else(block) => {
                            self.format_block(block);
                        }
                    }
                }
                self.newline();
            }
            Stmt::IfLet {
                name,
                expr,
                then_block,
                else_block,
                ..
            } => {
                self.write_indent();
                self.write("if let ");
                self.write(name);
                self.write(" = ");
                self.format_expr(expr);
                self.write(" ");
                self.format_block(then_block);
                if let Some(else_blk) = else_block {
                    self.write(" else ");
                    self.format_block(else_blk);
                }
                self.newline();
            }
            Stmt::While { cond, body, .. } => {
                self.write_indent();
                self.write("while (");
                self.format_expr(cond);
                self.write(") ");
                self.format_block(body);
                self.newline();
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                self.write_indent();
                self.write("for (");
                if let Some(init) = init {
                    self.format_for_init(init);
                }
                self.write("; ");
                if let Some(cond) = cond {
                    self.format_expr(cond);
                }
                self.write("; ");
                if let Some(step) = step {
                    self.format_for_step(step);
                }
                self.write(") ");
                self.format_block(body);
                self.newline();
            }
            Stmt::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                self.write_indent();
                self.write("switch (");
                self.format_expr(expr);
                self.write(") {");
                self.newline();

                self.indent += 1;
                for case in cases {
                    self.format_case(case);
                }
                if let Some(stmts) = default {
                    self.write_indent();
                    self.write("default:");
                    self.newline();
                    self.indent += 1;
                    for stmt in stmts {
                        self.format_stmt(stmt);
                    }
                    self.indent -= 1;
                }
                self.indent -= 1;

                self.line("}");
            }
            Stmt::Return { value, .. } => {
                self.write_indent();
                self.write("return");
                if let Some(val) = value {
                    self.write(" ");
                    self.format_expr(val);
                }
                self.write(";");
                self.newline();
            }
            Stmt::Break { .. } => {
                self.line("break;");
            }
            Stmt::Continue { .. } => {
                self.line("continue;");
            }
            Stmt::Defer { body, .. } => {
                self.write_indent();
                self.write("defer ");
                self.format_block(body);
                self.newline();
            }
            Stmt::Expr { expr, .. } => {
                self.write_indent();
                self.format_expr(expr);
                self.write(";");
                self.newline();
            }
            Stmt::Discard { expr, .. } => {
                self.write_indent();
                self.write("discard(");
                self.format_expr(expr);
                self.write(");");
                self.newline();
            }
            Stmt::Unsafe { body, .. } => {
                self.write_indent();
                self.write("unsafe ");
                self.format_block(body);
                self.newline();
            }
            Stmt::Block(block) => {
                self.write_indent();
                self.format_block(block);
                self.newline();
            }
        }
    }

    /// Format a statement inline (for else-if chains)
    fn format_stmt_inline(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                self.write("if (");
                self.format_expr(cond);
                self.write(") ");
                self.format_block(then_block);
                if let Some(else_branch) = else_block {
                    self.write(" else ");
                    match else_branch {
                        ElseBranch::ElseIf(stmt) => {
                            self.format_stmt_inline(stmt);
                        }
                        ElseBranch::Else(block) => {
                            self.format_block(block);
                        }
                    }
                }
            }
            _ => self.format_stmt(stmt),
        }
    }

    /// Format a for loop initializer
    fn format_for_init(&mut self, init: &ForInit) {
        match init {
            ForInit::Let { name, ty, init } => {
                self.write("let ");
                self.write(name);
                self.write(": ");
                self.format_type(ty);
                self.write(" = ");
                self.format_expr(init);
            }
            ForInit::Assign { lhs, rhs } => {
                self.format_expr(lhs);
                self.write(" = ");
                self.format_expr(rhs);
            }
            ForInit::Call(expr) => {
                self.format_expr(expr);
            }
        }
    }

    /// Format a for loop step
    fn format_for_step(&mut self, step: &ForStep) {
        match step {
            ForStep::Assign { lhs, rhs } => {
                self.format_expr(lhs);
                self.write(" = ");
                self.format_expr(rhs);
            }
            ForStep::Call(expr) => {
                self.format_expr(expr);
            }
        }
    }

    /// Format a switch case
    fn format_case(&mut self, case: &Case) {
        self.write_indent();
        self.write("case ");
        self.format_const_expr(&case.value);
        self.write(":");
        self.newline();

        self.indent += 1;
        for stmt in &case.stmts {
            self.format_stmt(stmt);
        }
        self.indent -= 1;
    }

    /// Format an expression
    fn format_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::IntLit { value, .. } => {
                self.write(&value.to_string());
            }
            Expr::FloatLit { raw, .. } => {
                self.write(raw);
            }
            Expr::BoolLit { value, .. } => {
                self.write(if *value { "true" } else { "false" });
            }
            Expr::Ident { name, .. } => {
                self.write(name);
            }
            Expr::Binary { op, lhs, rhs, .. } => {
                self.format_expr(lhs);
                self.write(" ");
                self.write(binop_str(*op));
                self.write(" ");
                self.format_expr(rhs);
            }
            Expr::Unary { op, operand, .. } => {
                self.write(unaryop_str(*op));
                self.format_expr(operand);
            }
            Expr::Paren { inner, .. } => {
                self.write("(");
                self.format_expr(inner);
                self.write(")");
            }
            Expr::Call { callee, args, .. } => {
                self.format_expr(callee);
                self.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.format_expr(arg);
                }
                self.write(")");
            }
            Expr::Field { base, field, .. } => {
                self.format_expr(base);
                self.write(".");
                self.write(field);
            }
            Expr::Addr { operand, .. } => {
                self.write("addr(");
                self.format_expr(operand);
                self.write(")");
            }
            Expr::Deref { operand, .. } => {
                self.write("deref(");
                self.format_expr(operand);
                self.write(")");
            }
            Expr::At { base, index, .. } => {
                self.write("at(");
                self.format_expr(base);
                self.write(", ");
                self.format_expr(index);
                self.write(")");
            }
            Expr::Cast { ty, expr, .. } => {
                self.write("cast(");
                self.format_type(ty);
                self.write(", ");
                self.format_expr(expr);
                self.write(")");
            }
            Expr::CStr { value, .. } => {
                self.write("cstr(\"");
                self.write(&escape_string(value));
                self.write("\")");
            }
            Expr::Bytes { value, .. } => {
                self.write("bytes(\"");
                self.write(&escape_string(value));
                self.write("\")");
            }
            Expr::None { ty, .. } => {
                self.write("none(");
                self.format_type(ty);
                self.write(")");
            }
            Expr::Some { value, .. } => {
                self.write("some(");
                self.format_expr(value);
                self.write(")");
            }
            Expr::Ok { value, .. } => {
                self.write("ok(");
                self.format_expr(value);
                self.write(")");
            }
            Expr::Err { value, .. } => {
                self.write("err(");
                self.format_expr(value);
                self.write(")");
            }
            Expr::StructLit { name, fields, .. } => {
                self.write(name);
                self.write(" { ");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.format_field_init(field);
                }
                self.write(" }");
            }
        }
    }

    /// Format a field initializer
    fn format_field_init(&mut self, field: &FieldInit) {
        self.write(&field.name);
        self.write(": ");
        self.format_expr(&field.value);
    }

    /// Format a constant expression
    fn format_const_expr(&mut self, expr: &ConstExpr) {
        match expr {
            ConstExpr::IntLit(value) => {
                self.write(&value.to_string());
            }
            ConstExpr::FloatLit(value) => {
                self.write(&value.to_string());
            }
            ConstExpr::BoolLit(value) => {
                self.write(if *value { "true" } else { "false" });
            }
            ConstExpr::Ident(name) => {
                self.write(name);
            }
            ConstExpr::Binary { op, lhs, rhs } => {
                self.format_const_expr(lhs);
                self.write(" ");
                self.write(binop_str(*op));
                self.write(" ");
                self.format_const_expr(rhs);
            }
            ConstExpr::Unary { op, operand } => {
                self.write(unaryop_str(*op));
                self.format_const_expr(operand);
            }
            ConstExpr::Paren(inner) => {
                self.write("(");
                self.format_const_expr(inner);
                self.write(")");
            }
            ConstExpr::Cast { ty, expr } => {
                self.write("cast(");
                self.format_type(ty);
                self.write(", ");
                self.format_const_expr(expr);
                self.write(")");
            }
            ConstExpr::CStr(value) => {
                self.write("cstr(\"");
                self.write(&escape_string(value));
                self.write("\")");
            }
            ConstExpr::Bytes(value) => {
                self.write("bytes(\"");
                self.write(&escape_string(value));
                self.write("\")");
            }
        }
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the string representation of a binary operator
fn binop_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
    }
}

/// Get the string representation of a unary operator
fn unaryop_str(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "~",
    }
}

/// Get the string representation of a primitive type
fn primitive_name(p: PrimitiveType) -> &'static str {
    match p {
        PrimitiveType::I8 => "i8",
        PrimitiveType::I16 => "i16",
        PrimitiveType::I32 => "i32",
        PrimitiveType::I64 => "i64",
        PrimitiveType::U8 => "u8",
        PrimitiveType::U16 => "u16",
        PrimitiveType::U32 => "u32",
        PrimitiveType::U64 => "u64",
        PrimitiveType::F32 => "f32",
        PrimitiveType::F64 => "f64",
        PrimitiveType::Bool => "bool",
        PrimitiveType::Usize => "usize",
        PrimitiveType::Isize => "isize",
    }
}

/// Escape special characters in a string
fn escape_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\0' => result.push_str("\\0"),
            c => result.push(c),
        }
    }
    result
}
