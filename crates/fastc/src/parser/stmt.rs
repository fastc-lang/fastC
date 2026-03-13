//! Statement parsing

use crate::ast::{Block, Case, ElseBranch, ForInit, ForStep, Stmt};
use crate::diag::CompileError;
use crate::lexer::Token;

use super::Parser;

impl Parser<'_> {
    /// Parse a statement
    pub fn parse_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.current_span().start;

        match self.current() {
            Token::Let => self.parse_let_stmt(),
            Token::If => self.parse_if_stmt(),
            Token::While => self.parse_while_stmt(),
            Token::For => self.parse_for_stmt(),
            Token::Switch => self.parse_switch_stmt(),
            Token::Return => self.parse_return_stmt(),
            Token::Break => {
                self.advance();
                self.consume(&Token::Semi, "expected ';' after 'break'")?;
                let end = self.previous_span().end;
                Ok(Stmt::Break { span: start..end })
            }
            Token::Continue => {
                self.advance();
                self.consume(&Token::Semi, "expected ';' after 'continue'")?;
                let end = self.previous_span().end;
                Ok(Stmt::Continue { span: start..end })
            }
            Token::Defer => self.parse_defer_stmt(),
            Token::Unsafe => self.parse_unsafe_block(),
            Token::Discard => self.parse_discard_stmt(),
            Token::LBrace => {
                let block = self.parse_block()?;
                Ok(Stmt::Block(block))
            }
            _ => self.parse_expr_or_assign_stmt(),
        }
    }

    /// Parse a let statement
    fn parse_let_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Let, "expected 'let'")?;
        let name = self.expect_ident()?;
        self.consume(&Token::Colon, "expected ':' after variable name")?;
        let ty = self.parse_type()?;
        self.consume(&Token::Eq, "expected '=' in let statement")?;
        let init = self.parse_expr()?;
        self.consume(&Token::Semi, "expected ';'")?;
        let end = self.previous_span().end;

        Ok(Stmt::Let {
            name,
            ty,
            init,
            span: start..end,
        })
    }

    /// Parse an if statement
    fn parse_if_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::If, "expected 'if'")?;

        // Check for if-let
        if self.check(&Token::Let) {
            self.advance();
            let name = self.expect_ident()?;
            self.consume(&Token::Eq, "expected '=' in if-let")?;
            self.consume(&Token::UnwrapChecked, "expected 'unwrap_checked' in if-let")?;
            self.consume(&Token::LParen, "expected '(' after 'unwrap_checked'")?;
            let expr = self.parse_expr()?;
            self.consume(&Token::RParen, "expected ')'")?;

            let then_block = self.parse_block()?;

            let else_block = if self.check(&Token::Else) {
                self.advance();
                Some(self.parse_block()?)
            } else {
                None
            };

            let end = self.previous_span().end;
            return Ok(Stmt::IfLet {
                name,
                expr,
                then_block,
                else_block,
                span: start..end,
            });
        }

        self.consume(&Token::LParen, "expected '(' after 'if'")?;
        let cond = self.parse_expr()?;
        self.consume(&Token::RParen, "expected ')'")?;

        let then_block = self.parse_block()?;

        let else_block = if self.check(&Token::Else) {
            self.advance();
            if self.check(&Token::If) {
                Some(ElseBranch::ElseIf(Box::new(self.parse_if_stmt()?)))
            } else {
                Some(ElseBranch::Else(self.parse_block()?))
            }
        } else {
            None
        };

        let end = self.previous_span().end;
        Ok(Stmt::If {
            cond,
            then_block,
            else_block,
            span: start..end,
        })
    }

    /// Parse a while statement
    fn parse_while_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::While, "expected 'while'")?;
        self.consume(&Token::LParen, "expected '(' after 'while'")?;
        let cond = self.parse_expr()?;
        self.consume(&Token::RParen, "expected ')'")?;
        let body = self.parse_block()?;
        let end = self.previous_span().end;

        Ok(Stmt::While {
            cond,
            body,
            span: start..end,
        })
    }

    /// Parse a for statement
    fn parse_for_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::For, "expected 'for'")?;
        self.consume(&Token::LParen, "expected '(' after 'for'")?;

        // Parse init (optional)
        let init = if self.check(&Token::Semi) {
            None
        } else if self.check(&Token::Let) {
            self.advance();
            let name = self.expect_ident()?;
            self.consume(&Token::Colon, "expected ':'")?;
            let ty = self.parse_type()?;
            self.consume(&Token::Eq, "expected '='")?;
            let init_expr = self.parse_expr()?;
            Some(ForInit::Let {
                name,
                ty,
                init: init_expr,
            })
        } else {
            let expr = self.parse_expr()?;
            if self.check(&Token::Eq) {
                self.advance();
                let rhs = self.parse_expr()?;
                Some(ForInit::Assign { lhs: expr, rhs })
            } else {
                Some(ForInit::Call(expr))
            }
        };
        self.consume(&Token::Semi, "expected ';' after for init")?;

        // Parse condition (optional)
        let cond = if self.check(&Token::Semi) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.consume(&Token::Semi, "expected ';' after for condition")?;

        // Parse step (optional)
        let step = if self.check(&Token::RParen) {
            None
        } else {
            let expr = self.parse_expr()?;
            if self.check(&Token::Eq) {
                self.advance();
                let rhs = self.parse_expr()?;
                Some(ForStep::Assign { lhs: expr, rhs })
            } else {
                Some(ForStep::Call(expr))
            }
        };
        self.consume(&Token::RParen, "expected ')'")?;

        let body = self.parse_block()?;
        let end = self.previous_span().end;

        Ok(Stmt::For {
            init,
            cond,
            step,
            body,
            span: start..end,
        })
    }

    /// Parse a switch statement
    fn parse_switch_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Switch, "expected 'switch'")?;
        self.consume(&Token::LParen, "expected '(' after 'switch'")?;
        let expr = self.parse_expr()?;
        self.consume(&Token::RParen, "expected ')'")?;
        self.consume(&Token::LBrace, "expected '{'")?;

        let mut cases = Vec::new();
        let mut default = None;

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if self.check(&Token::Case) {
                let case_start = self.current_span().start;
                self.advance();
                let value = self.parse_const_expr()?;
                self.consume(&Token::Colon, "expected ':' after case value")?;

                let mut stmts = Vec::new();
                while !self.check(&Token::Case)
                    && !self.check(&Token::Default)
                    && !self.check(&Token::RBrace)
                {
                    stmts.push(self.parse_stmt()?);
                }

                let case_end = self.previous_span().end;
                cases.push(Case {
                    value,
                    stmts,
                    span: case_start..case_end,
                });
            } else if self.check(&Token::Default) {
                self.advance();
                self.consume(&Token::Colon, "expected ':' after 'default'")?;

                let mut stmts = Vec::new();
                while !self.check(&Token::Case)
                    && !self.check(&Token::Default)
                    && !self.check(&Token::RBrace)
                {
                    stmts.push(self.parse_stmt()?);
                }

                default = Some(stmts);
            } else {
                return Err(self.error("expected 'case' or 'default'"));
            }
        }

        self.consume(&Token::RBrace, "expected '}'")?;
        let end = self.previous_span().end;

        Ok(Stmt::Switch {
            expr,
            cases,
            default,
            span: start..end,
        })
    }

    /// Parse a return statement
    fn parse_return_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Return, "expected 'return'")?;

        let value = if self.check(&Token::Semi) {
            None
        } else {
            Some(self.parse_expr()?)
        };

        self.consume(&Token::Semi, "expected ';'")?;
        let end = self.previous_span().end;

        Ok(Stmt::Return {
            value,
            span: start..end,
        })
    }

    /// Parse a defer statement
    fn parse_defer_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Defer, "expected 'defer'")?;
        let body = self.parse_block()?;
        let end = self.previous_span().end;

        Ok(Stmt::Defer {
            body,
            span: start..end,
        })
    }

    /// Parse an unsafe block
    fn parse_unsafe_block(&mut self) -> Result<Stmt, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Unsafe, "expected 'unsafe'")?;
        let body = self.parse_block()?;
        let end = self.previous_span().end;

        Ok(Stmt::Unsafe {
            body,
            span: start..end,
        })
    }

    /// Parse a discard statement
    fn parse_discard_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::Discard, "expected 'discard'")?;
        self.consume(&Token::LParen, "expected '(' after 'discard'")?;
        let expr = self.parse_expr()?;
        self.consume(&Token::RParen, "expected ')'")?;
        self.consume(&Token::Semi, "expected ';'")?;
        let end = self.previous_span().end;

        Ok(Stmt::Discard {
            expr,
            span: start..end,
        })
    }

    /// Parse expression statement or assignment
    fn parse_expr_or_assign_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.current_span().start;
        let expr = self.parse_expr()?;

        if self.check(&Token::Eq) {
            self.advance();
            let rhs = self.parse_expr()?;
            self.consume(&Token::Semi, "expected ';'")?;
            let end = self.previous_span().end;

            Ok(Stmt::Assign {
                lhs: expr,
                rhs,
                span: start..end,
            })
        } else {
            self.consume(&Token::Semi, "expected ';'")?;
            let end = self.previous_span().end;

            Ok(Stmt::Expr {
                expr,
                span: start..end,
            })
        }
    }

    /// Parse a block
    pub fn parse_block(&mut self) -> Result<Block, CompileError> {
        let start = self.current_span().start;
        self.consume(&Token::LBrace, "expected '{'")?;

        let mut stmts = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
        }

        self.consume(&Token::RBrace, "expected '}'")?;
        let end = self.previous_span().end;

        Ok(Block {
            stmts,
            span: start..end,
        })
    }
}
