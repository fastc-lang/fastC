//! Expression parsing with single-binary-operator rule

use crate::ast::{BinOp, Expr, FieldInit, Param, TypeExpr, UnaryOp};
use crate::diag::CompileError;
use crate::lexer::Token;

use super::Parser;

impl Parser<'_> {
    /// Parse an expression
    pub fn parse_expr(&mut self) -> Result<Expr, CompileError> {
        self.parse_binary()
    }

    /// Parse binary expression with single-operator rule
    fn parse_binary(&mut self) -> Result<Expr, CompileError> {
        let start = self.current_span().start;
        let left = self.parse_unary()?;

        // Check for binary operator
        if let Some(op) = self.try_parse_binop() {
            let right = self.parse_unary()?;
            let end = self.previous_span().end;

            // Check for chained binary operator (not allowed without parens)
            if self.current().is_binary_op() {
                return Err(self.error("chained binary operators require parentheses"));
            }

            Ok(Expr::Binary {
                op,
                lhs: Box::new(left),
                rhs: Box::new(right),
                span: start..end,
            })
        } else {
            Ok(left)
        }
    }

    /// Try to parse a binary operator, returning None if not found
    pub fn try_parse_binop(&mut self) -> Option<BinOp> {
        let op = match self.current() {
            Token::Plus => Some(BinOp::Add),
            Token::Minus => Some(BinOp::Sub),
            Token::Star => Some(BinOp::Mul),
            Token::Slash => Some(BinOp::Div),
            Token::Percent => Some(BinOp::Rem),
            Token::EqEq => Some(BinOp::Eq),
            Token::NotEq => Some(BinOp::Ne),
            Token::Lt => Some(BinOp::Lt),
            Token::LtEq => Some(BinOp::Le),
            Token::Gt => Some(BinOp::Gt),
            Token::GtEq => Some(BinOp::Ge),
            Token::AndAnd => Some(BinOp::And),
            Token::OrOr => Some(BinOp::Or),
            Token::And => Some(BinOp::BitAnd),
            Token::Or => Some(BinOp::BitOr),
            Token::Caret => Some(BinOp::BitXor),
            Token::Shl => Some(BinOp::Shl),
            Token::Shr => Some(BinOp::Shr),
            _ => None,
        };

        if op.is_some() {
            self.advance();
        }
        op
    }

    /// Parse unary expression
    fn parse_unary(&mut self) -> Result<Expr, CompileError> {
        let start = self.current_span().start;

        match self.current() {
            Token::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                let end = self.previous_span().end;
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span: start..end,
                })
            }
            Token::Not => {
                self.advance();
                let operand = self.parse_unary()?;
                let end = self.previous_span().end;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span: start..end,
                })
            }
            Token::Tilde => {
                self.advance();
                let operand = self.parse_unary()?;
                let end = self.previous_span().end;
                Ok(Expr::Unary {
                    op: UnaryOp::BitNot,
                    operand: Box::new(operand),
                    span: start..end,
                })
            }
            _ => self.parse_postfix(),
        }
    }

    /// Parse postfix expressions (calls, field access)
    fn parse_postfix(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&Token::LParen) {
                expr = self.parse_call(expr)?;
            } else if self.check(&Token::Dot) {
                expr = self.parse_field_access(expr)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Parse a function call
    fn parse_call(&mut self, callee: Expr) -> Result<Expr, CompileError> {
        let start = callee.span().start;
        self.consume(&Token::LParen, "expected '('")?;

        let mut args = Vec::new();
        if !self.check(&Token::RParen) {
            args.push(self.parse_expr()?);
            while self.check(&Token::Comma) {
                self.advance();
                if self.check(&Token::RParen) {
                    break;
                }
                args.push(self.parse_expr()?);
            }
        }

        self.consume(&Token::RParen, "expected ')'")?;
        let end = self.previous_span().end;

        Ok(Expr::Call {
            callee: Box::new(callee),
            args,
            span: start..end,
        })
    }

    /// Parse field access
    fn parse_field_access(&mut self, base: Expr) -> Result<Expr, CompileError> {
        let start = base.span().start;
        self.consume(&Token::Dot, "expected '.'")?;
        let field = self.expect_ident()?;
        let end = self.previous_span().end;

        Ok(Expr::Field {
            base: Box::new(base),
            field,
            span: start..end,
        })
    }

    /// Parse primary expression
    fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        let start = self.current_span().start;

        match self.current().clone() {
            // Literals
            Token::IntLit(n) | Token::HexLit(n) | Token::BinLit(n) | Token::OctLit(n) => {
                self.advance();
                let end = self.previous_span().end;
                Ok(Expr::IntLit {
                    value: n,
                    span: start..end,
                })
            }
            Token::FloatLit(n) => {
                let raw = self.tokens[self.pos].span.clone();
                let raw_str = self.source[raw].to_string();
                self.advance();
                let end = self.previous_span().end;
                Ok(Expr::FloatLit {
                    value: n,
                    raw: raw_str,
                    span: start..end,
                })
            }
            Token::True => {
                self.advance();
                let end = self.previous_span().end;
                Ok(Expr::BoolLit {
                    value: true,
                    span: start..end,
                })
            }
            // Closure: `|name: Type, ...| -> Ret { body }`. `|` at
            // primary position can only mean closure-start — bit-or
            // is an infix operator and never appears at primary.
            Token::Or | Token::OrOr => self.parse_closure(start),
            Token::False => {
                self.advance();
                let end = self.previous_span().end;
                Ok(Expr::BoolLit {
                    value: false,
                    span: start..end,
                })
            }

            // Parenthesized expression
            Token::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                let end = self.previous_span().end;
                Ok(Expr::Paren {
                    inner: Box::new(inner),
                    span: start..end,
                })
            }

            // Builtins
            Token::Addr => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'addr'")?;
                let operand = self.parse_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                let end = self.previous_span().end;
                Ok(Expr::Addr {
                    operand: Box::new(operand),
                    span: start..end,
                })
            }
            Token::AddrM => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'addrm'")?;
                let operand = self.parse_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                let end = self.previous_span().end;
                Ok(Expr::AddrM {
                    operand: Box::new(operand),
                    span: start..end,
                })
            }
            Token::Deref => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'deref'")?;
                let operand = self.parse_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                let end = self.previous_span().end;
                Ok(Expr::Deref {
                    operand: Box::new(operand),
                    span: start..end,
                })
            }
            Token::At => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'at'")?;
                let base = self.parse_expr()?;
                self.consume(&Token::Comma, "expected ',' in 'at'")?;
                let index = self.parse_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                let end = self.previous_span().end;
                Ok(Expr::At {
                    base: Box::new(base),
                    index: Box::new(index),
                    span: start..end,
                })
            }
            Token::Cast => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'cast'")?;
                let ty = self.parse_type()?;
                self.consume(&Token::Comma, "expected ',' in 'cast'")?;
                let expr = self.parse_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                let end = self.previous_span().end;
                Ok(Expr::Cast {
                    ty,
                    expr: Box::new(expr),
                    span: start..end,
                })
            }
            Token::SizeOf => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'sizeof'")?;
                let ty = self.parse_type()?;
                self.consume(&Token::RParen, "expected ')'")?;
                let end = self.previous_span().end;
                Ok(Expr::SizeOf {
                    ty,
                    span: start..end,
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
                let end = self.previous_span().end;
                Ok(Expr::CStr {
                    value: s,
                    span: start..end,
                })
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
                let end = self.previous_span().end;
                Ok(Expr::Bytes {
                    value: s,
                    span: start..end,
                })
            }
            Token::None => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'none'")?;
                let ty = self.parse_type()?;
                self.consume(&Token::RParen, "expected ')'")?;
                let end = self.previous_span().end;
                Ok(Expr::None {
                    ty,
                    span: start..end,
                })
            }
            Token::Some => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'some'")?;
                let value = self.parse_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                let end = self.previous_span().end;
                Ok(Expr::Some {
                    value: Box::new(value),
                    span: start..end,
                })
            }
            Token::Ok_ => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'ok'")?;
                let value = self.parse_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                let end = self.previous_span().end;
                Ok(Expr::Ok {
                    value: Box::new(value),
                    span: start..end,
                })
            }
            Token::Err_ => {
                self.advance();
                self.consume(&Token::LParen, "expected '(' after 'err'")?;
                let value = self.parse_expr()?;
                self.consume(&Token::RParen, "expected ')'")?;
                let end = self.previous_span().end;
                Ok(Expr::Err {
                    value: Box::new(value),
                    span: start..end,
                })
            }

            // Identifier (possibly struct literal)
            Token::Ident(name) => {
                self.advance();

                // Check for struct literal
                if self.check(&Token::LBrace) {
                    self.advance();
                    let mut fields = Vec::new();

                    if !self.check(&Token::RBrace) {
                        loop {
                            let field_start = self.current_span().start;
                            let field_name = self.expect_ident()?;
                            self.consume(&Token::Colon, "expected ':' in struct literal")?;
                            let value = self.parse_expr()?;
                            let field_end = self.previous_span().end;

                            fields.push(FieldInit {
                                name: field_name,
                                value,
                                span: field_start..field_end,
                            });

                            if !self.check(&Token::Comma) {
                                break;
                            }
                            self.advance();
                            if self.check(&Token::RBrace) {
                                break;
                            }
                        }
                    }

                    self.consume(&Token::RBrace, "expected '}'")?;
                    let end = self.previous_span().end;

                    Ok(Expr::StructLit {
                        name,
                        fields,
                        span: start..end,
                    })
                } else {
                    let end = self.previous_span().end;
                    Ok(Expr::Ident {
                        name,
                        span: start..end,
                    })
                }
            }

            _ => Err(self.error("expected expression")),
        }
    }

    /// Parse a closure expression:
    ///   - zero args:   `|| -> Ret { body }`     (start tok: OrOr)
    ///   - non-zero:    `|x: T, y: U| -> Ret { body }`  (start tok: Or)
    ///
    /// `start` is the byte offset of the leading `|` (or `||`) — the
    /// caller has just consumed nothing yet; this method advances past
    /// the introducer itself. Return type is mandatory in v1 because
    /// type inference is too restricted to recover it from the body.
    fn parse_closure(&mut self, start: usize) -> Result<Expr, CompileError> {
        // Empty-param shortcut: `||` is the single-token form.
        if matches!(self.current(), crate::lexer::Token::OrOr) {
            self.advance();
            let ret = self.parse_closure_return()?;
            let body = self.parse_block()?;
            let end = self.previous_span().end;
            return Ok(Expr::Closure {
                params: Vec::new(),
                ret,
                body,
                span: start..end,
            });
        }

        // Otherwise `|` ... `|`.
        self.consume(
            &crate::lexer::Token::Or,
            "expected '|' to start closure parameters",
        )?;
        let mut params: Vec<Param> = Vec::new();
        if !self.check(&crate::lexer::Token::Or) {
            loop {
                let p_start = self.current_span().start;
                let name = self.expect_ident()?;
                self.consume(
                    &crate::lexer::Token::Colon,
                    "expected ':' after closure parameter name (type annotations are required in v1)",
                )?;
                let ty = self.parse_type()?;
                let p_end = self.previous_span().end;
                params.push(Param {
                    name,
                    ty,
                    span: p_start..p_end,
                });
                if !self.check(&crate::lexer::Token::Comma) {
                    break;
                }
                self.advance();
            }
        }
        self.consume(
            &crate::lexer::Token::Or,
            "expected '|' to close closure parameters",
        )?;
        let ret = self.parse_closure_return()?;
        let body = self.parse_block()?;
        let end = self.previous_span().end;
        Ok(Expr::Closure {
            params,
            ret,
            body,
            span: start..end,
        })
    }

    /// Closure return type — `-> Type` is required in v1. `void` is
    /// explicit; closures returning nothing must say `-> void`.
    fn parse_closure_return(&mut self) -> Result<TypeExpr, CompileError> {
        self.consume(
            &crate::lexer::Token::Arrow,
            "expected '->' before closure return type",
        )?;
        self.parse_type()
    }
}
