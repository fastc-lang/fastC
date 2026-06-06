//! Parser for FastC

mod decl;
mod expr;
mod stmt;
mod types;

use crate::ast::File;
use crate::diag::CompileError;
use crate::lexer::{Span, Spanned, Token};

/// Parser state
pub struct Parser<'a> {
    tokens: &'a [Spanned<Token>],
    pos: usize,
    source: &'a str,
    _filename: &'a str,
}

impl<'a> Parser<'a> {
    /// Create a new parser
    pub fn new(tokens: &'a [Spanned<Token>], source: &'a str, filename: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
            _filename: filename,
        }
    }

    /// Parse a complete file
    pub fn parse_file(&mut self) -> Result<File, CompileError> {
        let mut items = Vec::new();

        while !self.is_at_end() {
            // B3 contextual keyword: `test { fn ... fn ... }` flattens
            // into individual top-level fns each marked `is_test=true`.
            // We hijack `Ident("test") {` before falling through to
            // parse_item so user-defined `fn test() { ... }` still
            // parses correctly (parse_item handles the fn case).
            if matches!(self.current(), crate::lexer::Token::Ident(s) if s == "test") {
                let saved = self.pos;
                self.advance();
                if self.check(&crate::lexer::Token::LBrace) {
                    self.advance();
                    while !self.check(&crate::lexer::Token::RBrace) && !self.is_at_end() {
                        let it = self.parse_item()?;
                        match it {
                            crate::ast::Item::Fn(mut f) => {
                                f.is_test = true;
                                items.push(crate::ast::Item::Fn(f));
                            }
                            _ => {
                                return Err(self.error(
                                    "only `fn` declarations are allowed inside a `test { }` block",
                                ));
                            }
                        }
                    }
                    self.consume(
                        &crate::lexer::Token::RBrace,
                        "expected '}' to close test block",
                    )?;
                    continue;
                } else {
                    self.pos = saved;
                }
            }
            items.push(self.parse_item()?);
        }

        Ok(File { items })
    }

    // Token access helpers

    fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|t| &t.node)
            .unwrap_or(&Token::Eof)
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span.clone())
            .unwrap_or(self.source.len()..self.source.len())
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.pos += 1;
        }
        self.previous()
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.pos.saturating_sub(1)].node
    }

    fn previous_span(&self) -> Span {
        self.tokens[self.pos.saturating_sub(1)].span.clone()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.current(), Token::Eof)
    }

    fn check(&self, token: &Token) -> bool {
        std::mem::discriminant(self.current()) == std::mem::discriminant(token)
    }

    fn consume(&mut self, expected: &Token, message: &str) -> Result<(), CompileError> {
        if self.check(expected) {
            self.advance();
            Ok(())
        } else {
            // C-phase fix-it backfill: when the parser was expecting
            // a semicolon and didn't find one, register a structured
            // Fixit that inserts `;` at the end of the previous
            // token's span. `fastc fix` will apply this mechanically.
            // The missing-`;` case is the single highest-frequency
            // parser diagnostic in agent-loop usage; a one-keystroke
            // mechanical fix is the wedge here.
            if matches!(expected, Token::Semi) {
                let prev = self.previous_span();
                crate::fixit::registry::push(crate::fixit::Fixit::new(
                    prev.end..prev.end,
                    ";",
                    "insert missing ';'",
                ));
            }
            Err(self.error(message))
        }
    }

    /// Consume any consecutive `///` doc-comment tokens at the current
    /// position. Returns the doc text (with `///` and one optional leading
    /// space stripped) for each line, in source order.
    fn collect_doc_comments(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        while let Token::LineComment(text) = self.current().clone() {
            if crate::lexer::is_doc_comment(&text) {
                out.push(crate::lexer::doc_comment_text(&text));
                self.advance();
            } else {
                // Should not occur — non-doc line comments are filtered out
                // before the parser sees them.
                self.advance();
            }
        }
        out
    }

    fn error(&self, message: &str) -> CompileError {
        CompileError::parse(message, self.current_span(), self.source)
    }

    fn expect_ident(&mut self) -> Result<String, CompileError> {
        match self.current().clone() {
            Token::Ident(name) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.error("expected identifier")),
        }
    }

    /// Like `expect_ident` but also accepts primitive type keywords, useful
    /// for positions where either a user-named type or a built-in primitive
    /// is valid (e.g. the target of `impl Trait for ___`).
    fn expect_type_name(&mut self) -> Result<String, CompileError> {
        let name = match self.current() {
            Token::Ident(name) => name.clone(),
            Token::I8 => "i8".to_string(),
            Token::I16 => "i16".to_string(),
            Token::I32 => "i32".to_string(),
            Token::I64 => "i64".to_string(),
            Token::U8 => "u8".to_string(),
            Token::U16 => "u16".to_string(),
            Token::U32 => "u32".to_string(),
            Token::U64 => "u64".to_string(),
            Token::F32 => "f32".to_string(),
            Token::F64 => "f64".to_string(),
            Token::Bool => "bool".to_string(),
            Token::Usize => "usize".to_string(),
            Token::Isize => "isize".to_string(),
            _ => return Err(self.error("expected type name")),
        };
        self.advance();
        Ok(name)
    }
}
