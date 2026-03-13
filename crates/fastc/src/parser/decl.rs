//! Declaration parsing

use crate::ast::{
    ConstDecl, EnumDecl, ExternBlock, ExternItem, Field, FnDecl, FnProto, Item, ModDecl,
    OpaqueDecl, Param, Repr, StructDecl, UseDecl, UseItems, Variant,
};
use crate::diag::CompileError;
use crate::lexer::Token;

use super::Parser;

impl Parser<'_> {
    /// Parse a top-level item
    pub fn parse_item(&mut self) -> Result<Item, CompileError> {
        // Check for attributes
        let repr = if self.check(&Token::AtRepr) {
            Some(self.parse_repr_attr()?)
        } else {
            None
        };

        // Check for visibility modifier
        let is_pub = if self.check(&Token::Pub) {
            self.advance();
            true
        } else {
            false
        };

        match self.current() {
            Token::Fn => Ok(Item::Fn(self.parse_fn_decl(false)?)),
            Token::Unsafe => {
                self.advance();
                if self.check(&Token::Fn) {
                    Ok(Item::Fn(self.parse_fn_decl(true)?))
                } else {
                    Err(self.error("expected 'fn' after 'unsafe'"))
                }
            }
            Token::Struct => Ok(Item::Struct(self.parse_struct_decl(repr)?)),
            Token::Enum => Ok(Item::Enum(self.parse_enum_decl(repr)?)),
            Token::Const => Ok(Item::Const(self.parse_const_decl()?)),
            Token::Opaque => Ok(Item::Opaque(self.parse_opaque_decl()?)),
            Token::Extern => Ok(Item::Extern(self.parse_extern_block()?)),
            Token::Use => Ok(Item::Use(self.parse_use_decl()?)),
            Token::Mod => Ok(Item::Mod(self.parse_mod_decl(is_pub)?)),
            _ => Err(self.error("expected top-level item")),
        }
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
        self.consume(&Token::Fn, "expected 'fn'")?;
        let name = self.expect_ident()?;
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
            params,
            return_type,
            body,
            span: start..end,
        })
    }

    /// Parse function prototype (no body)
    fn parse_fn_proto(&mut self, is_unsafe: bool) -> Result<FnProto, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Fn, "expected 'fn'")?;
        let name = self.expect_ident()?;
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
            params,
            return_type,
            span: start..end,
        })
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
            fields,
            span: start..end,
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
