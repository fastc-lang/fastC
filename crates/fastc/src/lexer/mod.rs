//! Lexer for FastC source code

pub mod token;
pub mod trivia;

pub use token::{Span, Spanned, Token};
pub use trivia::{strip_comments, Comment, TriviaLexer, TriviaToken};

use logos::Logos;

/// Lexer wraps the logos lexer and provides spanned tokens
pub struct Lexer<'a> {
    inner: logos::Lexer<'a, Token>,
    finished: bool,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given source
    pub fn new(source: &'a str) -> Self {
        Self {
            inner: Token::lexer(source),
            finished: false,
        }
    }

    /// Get all tokens from the source
    pub fn tokenize(source: &str) -> Vec<Spanned<Token>> {
        Lexer::new(source).collect()
    }
}

impl Iterator for Lexer<'_> {
    type Item = Spanned<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match self.inner.next() {
            Some(Ok(token)) => {
                let span = self.inner.span();
                Some(Spanned::new(token, span))
            }
            Some(Err(())) => {
                // Skip invalid tokens and continue
                // In a real implementation we'd report errors
                let span = self.inner.span();
                Some(Spanned::new(
                    Token::Ident(self.inner.slice().to_string()),
                    span,
                ))
            }
            None => {
                self.finished = true;
                let end = self.inner.span().end;
                Some(Spanned::new(Token::Eof, end..end))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function() {
        let source = r#"fn main() -> void {
    return;
}"#;
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_let_statement() {
        let source = "let x: i32 = 42;";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_binary_operators() {
        let source = "(a + b) (c - d) (e * f) (g / h) (i % j)";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_comparison_operators() {
        let source = "(a == b) (c != d) (e < f) (g <= h) (i > j) (k >= l)";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_logical_operators() {
        let source = "(a && b) (c || d) !e";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_bitwise_operators() {
        let source = "(a & b) (c | d) (e ^ f) ~g (h << i) (j >> k)";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_type_keywords() {
        let source = "i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 bool usize isize void";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_type_constructors() {
        let source = "ref(i32) mref(i32) raw(i32) rawm(i32) own(i32) slice(i32) arr(i32, 10) opt(i32) res(i32, Error)";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_builtins() {
        let source = "addr(x) deref(p) at(arr, i) cast(i32, x) cstr(\"hello\") bytes(\"hello\") discard(x) none(i32) some(x)";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_literals() {
        let source = "42 0xFF 0b1010 0o77 3.14 2.5e10 true false \"hello\\nworld\"";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_struct_definition() {
        let source = r#"@repr(C)
struct Point {
    x: i32,
    y: i32,
}"#;
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_control_flow() {
        let source = "if while for switch case default break continue return defer";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_unsafe_keywords() {
        let source = "unsafe fn extern opaque";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_comments() {
        let source = r#"// single line comment
fn foo() -> void {
    /* multi
       line
       comment */
    return;
}"#;
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn test_underscore_numbers() {
        let source = "1_000_000 0xFF_FF 0b1010_1010";
        let tokens: Vec<_> = Lexer::new(source).map(|t| t.node).collect();
        insta::assert_debug_snapshot!(tokens);
    }
}
