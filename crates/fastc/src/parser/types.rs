//! Type parsing

use crate::ast::{ConstExpr, PrimitiveType, TypeExpr};
use crate::diag::CompileError;
use crate::lexer::Token;

use super::Parser;

impl Parser<'_> {
    /// Parse a type expression
    pub fn parse_type(&mut self) -> Result<TypeExpr, CompileError> {
        match self.current().clone() {
            // Primitive types
            Token::I8 => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::I8))
            }
            Token::I16 => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::I16))
            }
            Token::I32 => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::I32))
            }
            Token::I64 => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::I64))
            }
            Token::U8 => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::U8))
            }
            Token::U16 => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::U16))
            }
            Token::U32 => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::U32))
            }
            Token::U64 => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::U64))
            }
            Token::F32 => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::F32))
            }
            Token::F64 => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::F64))
            }
            Token::Bool => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::Bool))
            }
            Token::Usize => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::Usize))
            }
            Token::Isize => {
                self.advance();
                Ok(TypeExpr::Primitive(PrimitiveType::Isize))
            }
            Token::Void => {
                self.advance();
                Ok(TypeExpr::Void)
            }

            // Type constructors
            Token::Ref => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'ref'")?;
                let inner = self.parse_type()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(TypeExpr::Ref(Box::new(inner)))
            }
            Token::Mref => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'mref'")?;
                let inner = self.parse_type()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(TypeExpr::Mref(Box::new(inner)))
            }
            Token::Raw => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'raw'")?;
                let inner = self.parse_type()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(TypeExpr::Raw(Box::new(inner)))
            }
            Token::Rawm => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'rawm'")?;
                let inner = self.parse_type()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(TypeExpr::Rawm(Box::new(inner)))
            }
            Token::Own => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'own'")?;
                let inner = self.parse_type()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(TypeExpr::Own(Box::new(inner)))
            }
            Token::Slice => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'slice'")?;
                let inner = self.parse_type()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(TypeExpr::Slice(Box::new(inner)))
            }
            Token::Arr => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'arr'")?;
                let elem_type = self.parse_type()?;
                self.consume(&Token::Comma, "expected ',' in arr type")?;
                let size = self.parse_const_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(TypeExpr::Arr(Box::new(elem_type), Box::new(size)))
            }
            Token::Opt => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'opt'")?;
                let inner = self.parse_type()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(TypeExpr::Opt(Box::new(inner)))
            }
            Token::Res => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'res'")?;
                let ok_type = self.parse_type()?;
                self.consume(&Token::Comma, "expected ',' in res type")?;
                let err_type = self.parse_type()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(TypeExpr::Res(Box::new(ok_type), Box::new(err_type)))
            }

            // Function type
            Token::Fn | Token::Unsafe => {
                let is_unsafe = if self.check(&Token::Unsafe) {
                    self.advance();
                    true
                } else {
                    false
                };
                self.consume(&Token::Fn, "expected 'fn'")?;
                self.consume(&Token::LParen, "expected '(' in function type")?;

                let mut params = Vec::new();
                if !self.check(&Token::RParen) {
                    params.push(self.parse_type()?);
                    while self.check(&Token::Comma) {
                        self.advance();
                        if self.check(&Token::RParen) {
                            break;
                        }
                        params.push(self.parse_type()?);
                    }
                }
                self.consume(&Token::RParen, "expected ')'")?;
                self.consume(&Token::Arrow, "expected '->' in function type")?;
                let ret = self.parse_type()?;

                Ok(TypeExpr::Fn {
                    is_unsafe,
                    params,
                    ret: Box::new(ret),
                })
            }

            // Named type
            Token::Ident(name) => {
                self.advance();
                Ok(TypeExpr::Named(name))
            }

            _ => Err(self.error("expected type")),
        }
    }

    /// Parse a constant expression
    pub fn parse_const_expr(&mut self) -> Result<ConstExpr, CompileError> {
        self.parse_const_binary()
    }

    fn parse_const_binary(&mut self) -> Result<ConstExpr, CompileError> {
        let left = self.parse_const_unary()?;

        // Check for binary operator
        if let Some(op) = self.try_parse_binop() {
            let right = self.parse_const_unary()?;

            // Check for chained binary operator (not allowed)
            if self.current().is_binary_op() {
                return Err(self.error("chained binary operators require parentheses"));
            }

            Ok(ConstExpr::Binary {
                op,
                lhs: Box::new(left),
                rhs: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_const_unary(&mut self) -> Result<ConstExpr, CompileError> {
        use crate::ast::UnaryOp;

        match self.current() {
            Token::Minus => {
                self.advance();
                let operand = self.parse_const_unary()?;
                Ok(ConstExpr::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                })
            }
            Token::Not => {
                self.advance();
                let operand = self.parse_const_unary()?;
                Ok(ConstExpr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                })
            }
            Token::Tilde => {
                self.advance();
                let operand = self.parse_const_unary()?;
                Ok(ConstExpr::Unary {
                    op: UnaryOp::BitNot,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_const_primary(),
        }
    }

    fn parse_const_primary(&mut self) -> Result<ConstExpr, CompileError> {
        match self.current().clone() {
            Token::IntLit(n) | Token::HexLit(n) | Token::BinLit(n) | Token::OctLit(n) => {
                self.advance();
                Ok(ConstExpr::IntLit(n))
            }
            Token::FloatLit(n) => {
                self.advance();
                Ok(ConstExpr::FloatLit(n))
            }
            Token::True => {
                self.advance();
                Ok(ConstExpr::BoolLit(true))
            }
            Token::False => {
                self.advance();
                Ok(ConstExpr::BoolLit(false))
            }
            Token::Ident(name) => {
                self.advance();
                Ok(ConstExpr::Ident(name))
            }
            Token::LParen => {
                self.advance();
                let inner = self.parse_const_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(ConstExpr::Paren(Box::new(inner)))
            }
            Token::Cast => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'cast'")?;
                let ty = self.parse_type()?;
                self.consume(&Token::Comma, "expected ',' in cast")?;
                let expr = self.parse_const_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(ConstExpr::Cast {
                    ty,
                    expr: Box::new(expr),
                })
            }
            Token::Cstr => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'cstr'")?;
                let s = match self.current().clone() {
                    Token::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => return Err(self.error("expected string literal")),
                };
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(ConstExpr::CStr(s))
            }
            Token::Bytes => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'bytes'")?;
                let s = match self.current().clone() {
                    Token::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => return Err(self.error("expected string literal")),
                };
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(ConstExpr::Bytes(s))
            }
            _ => Err(self.error("expected constant expression")),
        }
    }
}
