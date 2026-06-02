//! Declaration parsing

use crate::ast::{
    ConstDecl, EnumDecl, ExternBlock, ExternItem, Field, FnDecl, FnProto, ImplBlock, Item, ModDecl,
    OpaqueDecl, Param, Repr, StructDecl, TraitDecl, TypeParam, UseDecl, UseItems, Variant,
};
use crate::diag::CompileError;
use crate::lexer::Token;

use super::Parser;

impl Parser<'_> {
    /// Parse a top-level item
    pub fn parse_item(&mut self) -> Result<Item, CompileError> {
        // Doc comments (`///`) attach to the next item. Collected here at
        // parse_item entry so every item kind benefits.
        let docs = self.collect_doc_comments();

        // Check for attributes
        let repr = if self.check(&Token::AtRepr) {
            Some(self.parse_repr_attr()?)
        } else {
            None
        };

        // Function-level annotations (`@noalloc` / `@nodiverg` /
        // `@pure`) precede `fn` and accumulate into a Vec attached
        // to the FnDecl. Collected here so they survive a `pub`
        // modifier or a leading `unsafe`. Same treatment for the
        // `@requires(...)` / `@ensures(...)` contract syntax.
        let annotations = self.parse_fn_annotations();
        let requires = self.parse_fn_requires()?;
        let ensures = self.parse_fn_ensures()?;
        // Allow interleaved order — annotations then requires then
        // ensures, or any permutation. Loop until all three stabilize
        // so a `@pure @requires(x>0) @ensures(result>=0) @noalloc`
        // chain parses regardless of token order.
        let mut annotations = annotations;
        let mut requires = requires;
        let mut ensures = ensures;
        loop {
            let more_ann = self.parse_fn_annotations();
            let more_req = self.parse_fn_requires()?;
            let more_ens = self.parse_fn_ensures()?;
            if more_ann.is_empty() && more_req.is_empty() && more_ens.is_empty() {
                break;
            }
            annotations.extend(more_ann);
            requires.extend(more_req);
            ensures.extend(more_ens);
        }

        // Check for visibility modifier
        let is_pub = if self.check(&Token::Pub) {
            self.advance();
            true
        } else {
            false
        };

        let item = match self.current() {
            Token::Fn => {
                let mut f = self.parse_fn_decl(false)?;
                f.annotations = merge_annotations(&annotations, &f.annotations);
                let mut rs = requires.clone();
                rs.extend(f.requires);
                f.requires = rs;
                let mut es = ensures.clone();
                es.extend(f.ensures);
                f.ensures = es;
                Item::Fn(f)
            }
            Token::Unsafe => {
                self.advance();
                if self.check(&Token::Fn) {
                    let mut f = self.parse_fn_decl(true)?;
                    f.annotations = merge_annotations(&annotations, &f.annotations);
                    let mut rs = requires.clone();
                    rs.extend(f.requires);
                    f.requires = rs;
                    let mut es = ensures.clone();
                    es.extend(f.ensures);
                    f.ensures = es;
                    Item::Fn(f)
                } else {
                    return Err(self.error("expected 'fn' after 'unsafe'"));
                }
            }
            Token::Struct => Item::Struct(self.parse_struct_decl(repr)?),
            Token::Enum => Item::Enum(self.parse_enum_decl(repr)?),
            Token::Const => Item::Const(self.parse_const_decl()?),
            Token::Opaque => Item::Opaque(self.parse_opaque_decl()?),
            Token::Extern => Item::Extern(self.parse_extern_block()?),
            Token::Use => Item::Use(self.parse_use_decl()?),
            Token::Mod => Item::Mod(self.parse_mod_decl(is_pub)?),
            Token::Impl => Item::Impl(self.parse_impl_block()?),
            Token::Trait => Item::Trait(self.parse_trait_decl()?),
            _ => return Err(self.error("expected top-level item")),
        };

        Ok(attach_doc_comments(item, docs))
    }

    /// Parse `impl Type { ... }` (inherent) or `impl Trait for Type { ... }`
    /// (trait impl). The two are distinguished by the `for` keyword after
    /// the first identifier.
    fn parse_impl_block(&mut self) -> Result<ImplBlock, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Impl, "expected 'impl'")?;
        // The first name may be a trait (Ident) or, for inherent impls of
        // a primitive type (`impl i32 { ... }`, although unusual), a
        // primitive keyword. Accept both.
        let first = self.expect_type_name()?;
        let (trait_name, target) = if self.check(&Token::For) {
            self.advance();
            // The target after `for` is a type — primitive keywords are
            // valid here (`impl Eq for i32`).
            let target = self.expect_type_name()?;
            (Some(first), target)
        } else {
            (None, first)
        };
        self.consume(&Token::LBrace, "expected '{' after impl target")?;

        let mut methods = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let is_unsafe = if self.check(&Token::Unsafe) {
                self.advance();
                true
            } else {
                false
            };
            // K1: an impl method may carry `@requires(...)` / `@ensures(...)`
            // / `@pure` / `@noalloc` / `@nodiverg` annotations before the
            // `fn` keyword — the same set top-level fns accept. We hand
            // off to `parse_fn_decl`, which absorbs the annotations
            // itself and then requires `fn`. The legacy hard check on
            // `Token::Fn` rejected impl methods that had contracts on
            // them and forced users to declare every method's
            // precondition externally.
            if !is_impl_method_start(self.current()) {
                return Err(self.error("expected 'fn' (or contract annotation) inside impl block"));
            }
            methods.push(self.parse_fn_decl(is_unsafe)?);
        }

        self.consume(&Token::RBrace, "expected '}' to close impl block")?;
        let end = self.previous_span().end;
        Ok(ImplBlock {
            target,
            trait_name,
            methods,
            span: start..end,
            doc_comments: Vec::new(),
        })
    }

    /// Parse `trait Foo { fn method(...) -> T; ... }`. Bodies are not
    /// permitted on trait method prototypes in v1 (no default methods).
    fn parse_trait_decl(&mut self) -> Result<TraitDecl, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Trait, "expected 'trait'")?;
        let name = self.expect_ident()?;
        self.consume(&Token::LBrace, "expected '{' after trait name")?;

        let mut methods = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let is_unsafe = if self.check(&Token::Unsafe) {
                self.advance();
                true
            } else {
                false
            };
            if !self.check(&Token::Fn) {
                return Err(self.error("expected 'fn' inside trait declaration"));
            }
            methods.push(self.parse_fn_proto(is_unsafe)?);
        }

        self.consume(&Token::RBrace, "expected '}' to close trait declaration")?;
        let end = self.previous_span().end;
        Ok(TraitDecl {
            name,
            methods,
            span: start..end,
            doc_comments: Vec::new(),
        })
    }

    /// Parse @repr attribute
    fn parse_repr_attr(&mut self) -> Result<Repr, CompileError> {
        self.consume(&Token::AtRepr, "expected '@repr'")?;
        self.consume(&Token::LParen, "expected '(' after '@repr'")?;

        let repr = match self.current() {
            Token::Ident(s) if s == "C" => {
                self.advance();
                Repr::C
            }
            Token::I8 => {
                self.advance();
                Repr::I8
            }
            Token::U8 => {
                self.advance();
                Repr::U8
            }
            Token::I16 => {
                self.advance();
                Repr::I16
            }
            Token::U16 => {
                self.advance();
                Repr::U16
            }
            Token::I32 => {
                self.advance();
                Repr::I32
            }
            Token::U32 => {
                self.advance();
                Repr::U32
            }
            Token::I64 => {
                self.advance();
                Repr::I64
            }
            Token::U64 => {
                self.advance();
                Repr::U64
            }
            _ => return Err(self.error("expected repr kind (C, i8, u8, etc.)")),
        };

        self.consume(&Token::RParen, "expected ')'")?;
        Ok(repr)
    }

    /// Parse function declaration
    fn parse_fn_decl(&mut self, is_unsafe: bool) -> Result<FnDecl, CompileError> {
        let start = self.current_span().start;
        // Collect any `@noalloc` / `@nodiverg` / `@pure` annotations
        // attached to the fn declaration. They precede `fn`.
        let annotations = self.parse_fn_annotations();
        // Collect any `@requires(...)` / `@ensures(...)` clauses. These
        // can interleave with the bool-flag annotations above; all
        // three are accumulated independently and merged onto the FnDecl.
        let requires = self.parse_fn_requires()?;
        let ensures = self.parse_fn_ensures()?;
        self.consume(&Token::Fn, "expected 'fn'")?;
        let name = self.expect_ident()?;
        let generics = self.parse_optional_type_params()?;
        self.consume(&Token::LParen, "expected '('")?;

        let params = self.parse_param_list()?;

        self.consume(&Token::RParen, "expected ')'")?;
        self.consume(&Token::Arrow, "expected '->'")?;
        let return_type = self.parse_type()?;
        let body = self.parse_block()?;
        let end = self.previous_span().end;

        Ok(FnDecl {
            is_unsafe,
            name,
            generics,
            params,
            return_type,
            body,
            span: start..end,
            doc_comments: Vec::new(),
            annotations,
            requires,
            ensures,
        })
    }

    /// Parse zero-or-more `@requires(<expr>)` clauses. Each clause's
    /// expression is the boolean condition the caller must satisfy.
    /// Lower turns each into `if (!cond) fc_trap();` at body entry.
    fn parse_fn_requires(&mut self) -> Result<Vec<crate::ast::Expr>, CompileError> {
        let mut out = Vec::new();
        while matches!(self.current(), Token::AtRequires) {
            self.advance();
            self.consume(&Token::LParen, "expected '(' after '@requires'")?;
            let cond = self.parse_expr()?;
            self.consume(&Token::RParen, "expected ')' to close '@requires'")?;
            out.push(cond);
        }
        Ok(out)
    }

    /// Parse zero-or-more `@ensures(<expr>)` postcondition clauses.
    /// Inside the expression, the identifier `result` is reserved
    /// for the value the function returns; the lower pass binds it
    /// at every `return EXPR;` site. v1 checks at runtime via
    /// `if (!cond) fc_trap();`; v2.1 SMT-discharges what it can.
    fn parse_fn_ensures(&mut self) -> Result<Vec<crate::ast::Expr>, CompileError> {
        let mut out = Vec::new();
        while matches!(self.current(), Token::AtEnsures) {
            self.advance();
            self.consume(&Token::LParen, "expected '(' after '@ensures'")?;
            let cond = self.parse_expr()?;
            self.consume(&Token::RParen, "expected ')' to close '@ensures'")?;
            out.push(cond);
        }
        Ok(out)
    }

    /// Parse zero-or-more `@flag` attributes that precede a fn decl.
    /// Each well-known flag becomes a string in the returned vec;
    /// unknown attributes are caller-rejected at the lexer level
    /// because they show up as `Token::At` followed by something
    /// the parser doesn't recognize here.
    fn parse_fn_annotations(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        loop {
            match self.current() {
                Token::AtNoalloc => {
                    self.advance();
                    out.push("noalloc".to_string());
                }
                Token::AtNodiverg => {
                    self.advance();
                    out.push("nodiverg".to_string());
                }
                Token::AtPure => {
                    self.advance();
                    out.push("pure".to_string());
                }
                _ => break,
            }
        }
        out
    }

    /// Parse function prototype (no body)
    fn parse_fn_proto(&mut self, is_unsafe: bool) -> Result<FnProto, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Fn, "expected 'fn'")?;
        let name = self.expect_ident()?;
        let generics = self.parse_optional_type_params()?;
        self.consume(&Token::LParen, "expected '('")?;

        let params = self.parse_param_list()?;

        self.consume(&Token::RParen, "expected ')'")?;
        self.consume(&Token::Arrow, "expected '->'")?;
        let return_type = self.parse_type()?;
        self.consume(&Token::Semi, "expected ';'")?;
        let end = self.previous_span().end;

        Ok(FnProto {
            is_unsafe,
            name,
            generics,
            params,
            return_type,
            span: start..end,
        })
    }

    /// Parse `[T, U, V]` after a function name, returning an empty vec if no
    /// type parameter list is present. Each parameter may carry trait
    /// bounds: `T: Bound1 + Bound2`.
    fn parse_optional_type_params(&mut self) -> Result<Vec<TypeParam>, CompileError> {
        if !self.check(&Token::LBracket) {
            return Ok(Vec::new());
        }
        self.advance(); // consume '['

        let mut params = Vec::new();
        if !self.check(&Token::RBracket) {
            loop {
                let start = self.current_span().start;
                let name = self.expect_ident()?;
                let mut bounds = Vec::new();
                if self.check(&Token::Colon) {
                    self.advance();
                    loop {
                        bounds.push(self.expect_ident()?);
                        if !self.check(&Token::Plus) {
                            break;
                        }
                        self.advance();
                    }
                }
                let end = self.previous_span().end;
                params.push(TypeParam {
                    name,
                    bounds,
                    span: start..end,
                });
                if !self.check(&Token::Comma) {
                    break;
                }
                self.advance();
            }
        }
        self.consume(&Token::RBracket, "expected ']'")?;
        Ok(params)
    }

    /// Parse parameter list
    fn parse_param_list(&mut self) -> Result<Vec<Param>, CompileError> {
        let mut params = Vec::new();

        if !self.check(&Token::RParen) {
            loop {
                let param_start = self.current_span().start;
                let name = self.expect_ident()?;
                self.consume(&Token::Colon, "expected ':'")?;
                let ty = self.parse_type()?;
                let param_end = self.previous_span().end;

                params.push(Param {
                    name,
                    ty,
                    span: param_start..param_end,
                });

                if !self.check(&Token::Comma) {
                    break;
                }
                self.advance();
            }
        }

        Ok(params)
    }

    /// Parse struct declaration
    fn parse_struct_decl(&mut self, repr: Option<Repr>) -> Result<StructDecl, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Struct, "expected 'struct'")?;
        let name = self.expect_ident()?;
        let generics = self.parse_optional_type_params()?;
        self.consume(&Token::LBrace, "expected '{'")?;

        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let field_start = self.current_span().start;
            let field_name = self.expect_ident()?;
            self.consume(&Token::Colon, "expected ':'")?;
            let ty = self.parse_type()?;
            let field_end = self.previous_span().end;

            fields.push(Field {
                name: field_name,
                ty,
                span: field_start..field_end,
            });

            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.consume(&Token::RBrace, "expected '}'")?;
        let end = self.previous_span().end;

        Ok(StructDecl {
            repr,
            name,
            generics,
            fields,
            span: start..end,
            doc_comments: Vec::new(),
        })
    }

    /// Parse enum declaration
    fn parse_enum_decl(&mut self, repr: Option<Repr>) -> Result<EnumDecl, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Enum, "expected 'enum'")?;
        let name = self.expect_ident()?;
        self.consume(&Token::LBrace, "expected '{'")?;

        let mut variants = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let var_start = self.current_span().start;
            let var_name = self.expect_ident()?;

            let fields = if self.check(&Token::LParen) {
                self.advance();
                let mut types = Vec::new();
                if !self.check(&Token::RParen) {
                    types.push(self.parse_type()?);
                    while self.check(&Token::Comma) {
                        self.advance();
                        if self.check(&Token::RParen) {
                            break;
                        }
                        types.push(self.parse_type()?);
                    }
                }
                self.consume(&Token::RParen, "expected ')'")?;
                Some(types)
            } else {
                None
            };

            let var_end = self.previous_span().end;
            variants.push(Variant {
                name: var_name,
                fields,
                span: var_start..var_end,
            });

            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.consume(&Token::RBrace, "expected '}'")?;
        let end = self.previous_span().end;

        Ok(EnumDecl {
            repr,
            name,
            variants,
            span: start..end,
            doc_comments: Vec::new(),
        })
    }

    /// Parse const declaration
    fn parse_const_decl(&mut self) -> Result<ConstDecl, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Const, "expected 'const'")?;
        let name = self.expect_ident()?;
        self.consume(&Token::Colon, "expected ':'")?;
        let ty = self.parse_type()?;
        self.consume(&Token::Eq, "expected '='")?;
        let value = self.parse_const_expr()?;
        self.consume(&Token::Semi, "expected ';'")?;
        let end = self.previous_span().end;

        Ok(ConstDecl {
            name,
            ty,
            value,
            span: start..end,
            doc_comments: Vec::new(),
        })
    }

    /// Parse opaque declaration
    fn parse_opaque_decl(&mut self) -> Result<OpaqueDecl, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Opaque, "expected 'opaque'")?;
        let name = self.expect_ident()?;
        self.consume(&Token::Semi, "expected ';'")?;
        let end = self.previous_span().end;

        Ok(OpaqueDecl {
            name,
            span: start..end,
        })
    }

    /// Parse extern block
    fn parse_extern_block(&mut self) -> Result<ExternBlock, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Extern, "expected 'extern'")?;

        let abi = match self.current().clone() {
            Token::StringLit(s) => {
                self.advance();
                s
            }
            _ => return Err(self.error("expected ABI string (e.g., \"C\")")),
        };

        self.consume(&Token::LBrace, "expected '{'")?;

        let mut items = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // Check for attributes
            let repr = if self.check(&Token::AtRepr) {
                Some(self.parse_repr_attr()?)
            } else {
                None
            };

            let item = match self.current() {
                Token::Fn => ExternItem::Fn(self.parse_fn_proto(false)?),
                Token::Unsafe => {
                    self.advance();
                    ExternItem::Fn(self.parse_fn_proto(true)?)
                }
                Token::Struct => ExternItem::Struct(self.parse_struct_decl(repr)?),
                Token::Enum => ExternItem::Enum(self.parse_enum_decl(repr)?),
                Token::Opaque => ExternItem::Opaque(self.parse_opaque_decl()?),
                _ => return Err(self.error("expected extern item")),
            };
            items.push(item);
        }

        self.consume(&Token::RBrace, "expected '}'")?;
        let end = self.previous_span().end;

        Ok(ExternBlock {
            abi,
            items,
            span: start..end,
        })
    }

    /// Parse use declaration
    /// Syntax: use path::to::item;
    ///         use path::to::{item1, item2};
    ///         use path::to::*;
    ///         use module;
    fn parse_use_decl(&mut self) -> Result<UseDecl, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Use, "expected 'use'")?;

        // Parse the path
        let mut path = Vec::new();
        path.push(self.expect_ident()?);

        // Check for :: path continuation or ;
        while self.check(&Token::ColonColon) {
            self.advance();

            // Check for special endings
            if self.check(&Token::Star) {
                // use path::*;
                self.advance();
                self.consume(&Token::Semi, "expected ';'")?;
                let end = self.previous_span().end;
                return Ok(UseDecl {
                    path,
                    items: UseItems::Glob,
                    span: start..end,
                });
            }

            if self.check(&Token::LBrace) {
                // use path::{item1, item2};
                self.advance();
                let mut items = Vec::new();

                if !self.check(&Token::RBrace) {
                    items.push(self.expect_ident()?);
                    while self.check(&Token::Comma) {
                        self.advance();
                        if self.check(&Token::RBrace) {
                            break;
                        }
                        items.push(self.expect_ident()?);
                    }
                }

                self.consume(&Token::RBrace, "expected '}'")?;
                self.consume(&Token::Semi, "expected ';'")?;
                let end = self.previous_span().end;
                return Ok(UseDecl {
                    path,
                    items: UseItems::Multiple(items),
                    span: start..end,
                });
            }

            // Regular path segment
            path.push(self.expect_ident()?);
        }

        self.consume(&Token::Semi, "expected ';'")?;
        let end = self.previous_span().end;

        // If path has only one element, it's a module import
        // Otherwise, the last element is the item being imported
        let (path, items) = if path.len() == 1 {
            (path, UseItems::Module)
        } else {
            let item = path.pop().unwrap();
            (path, UseItems::Single(item))
        };

        Ok(UseDecl {
            path,
            items,
            span: start..end,
        })
    }

    /// Parse module declaration
    /// Syntax: mod name;           (load from file)
    ///         mod name { ... }    (inline module)
    fn parse_mod_decl(&mut self, is_pub: bool) -> Result<ModDecl, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Mod, "expected 'mod'")?;
        let name = self.expect_ident()?;

        // Check for inline module body or external file reference
        let body = if self.check(&Token::LBrace) {
            self.advance();
            let mut items = Vec::new();
            while !self.check(&Token::RBrace) && !self.is_at_end() {
                items.push(self.parse_item()?);
            }
            self.consume(&Token::RBrace, "expected '}'")?;
            Some(items)
        } else {
            self.consume(&Token::Semi, "expected ';' or '{'")?;
            None
        };

        let end = self.previous_span().end;

        Ok(ModDecl {
            is_pub,
            name,
            body,
            span: start..end,
        })
    }
}

/// Attach a `Vec<String>` of doc-comment lines to whichever item variant
/// supports them. Items that don't carry docs (Use, Opaque, Extern) drop
/// the docs silently — they predate the doc-comment feature and have no
/// natural home for them yet.
fn attach_doc_comments(item: Item, docs: Vec<String>) -> Item {
    if docs.is_empty() {
        return item;
    }
    match item {
        Item::Fn(mut d) => {
            d.doc_comments = docs;
            Item::Fn(d)
        }
        Item::Struct(mut d) => {
            d.doc_comments = docs;
            Item::Struct(d)
        }
        Item::Enum(mut d) => {
            d.doc_comments = docs;
            Item::Enum(d)
        }
        Item::Const(mut d) => {
            d.doc_comments = docs;
            Item::Const(d)
        }
        Item::Trait(mut d) => {
            d.doc_comments = docs;
            Item::Trait(d)
        }
        Item::Impl(mut d) => {
            d.doc_comments = docs;
            Item::Impl(d)
        }
        // Use/Opaque/Extern/Mod don't carry docs in v1.
        other => other,
    }
}

/// K1: the legal start tokens for a method declaration inside an
/// `impl Type { … }` block. `Fn` is the historical case; the four
/// annotation tokens cover `@requires` / `@ensures` / `@pure` /
/// `@noalloc` / `@nodiverg` interleaved before the `fn` keyword.
/// `parse_fn_decl` consumes the annotations itself; this helper
/// only needs to recognize their token shape so the impl-block
/// loop doesn't bail out before the handoff.
fn is_impl_method_start(tok: &crate::lexer::Token) -> bool {
    use crate::lexer::Token;
    matches!(
        tok,
        Token::Fn
            | Token::AtRequires
            | Token::AtEnsures
            | Token::AtPure
            | Token::AtNoalloc
            | Token::AtNodiverg
    )
}

/// Merge two annotation vectors, keeping order and deduplicating
/// case-sensitively. Used so `@pure` written before `pub fn` and
/// `@noalloc` written between `pub` and `fn` both end up on the
/// final FnDecl.annotations.
fn merge_annotations(a: &[String], b: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    for x in a.iter().chain(b.iter()) {
        if !out.iter().any(|s| s == x) {
            out.push(x.clone());
        }
    }
    out
}
