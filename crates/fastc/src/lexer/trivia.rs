//! Trivia-aware tokenization for comment preservation
//!
//! This module provides a wrapper around the lexer that captures comments
//! as "trivia" attached to the next non-comment token. This allows the
//! formatter to preserve comments while the parser ignores them.

use super::{Lexer, Span, Spanned, Token};

/// A comment captured as trivia
#[derive(Debug, Clone)]
pub struct Comment {
    /// The comment text (including // or /* */)
    pub text: String,
    /// The span of the comment in the source
    pub span: Span,
}

/// A token with its associated leading trivia (comments)
#[derive(Debug, Clone)]
pub struct TriviaToken {
    /// The actual token
    pub token: Spanned<Token>,
    /// Comments that appeared before this token
    pub leading_comments: Vec<Comment>,
}

impl TriviaToken {
    pub fn new(token: Spanned<Token>) -> Self {
        Self {
            token,
            leading_comments: Vec::new(),
        }
    }

    pub fn with_comments(token: Spanned<Token>, comments: Vec<Comment>) -> Self {
        Self {
            token,
            leading_comments: comments,
        }
    }
}

/// Iterator that produces tokens with attached leading trivia
pub struct TriviaLexer<'a> {
    inner: Lexer<'a>,
    pending_comments: Vec<Comment>,
}

impl<'a> TriviaLexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inner: Lexer::new(source),
            pending_comments: Vec::new(),
        }
    }

    /// Tokenize source with trivia information
    pub fn tokenize(source: &str) -> Vec<TriviaToken> {
        TriviaLexer::new(source).collect()
    }
}

impl Iterator for TriviaLexer<'_> {
    type Item = TriviaToken;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let spanned = self.inner.next()?;

            match &spanned.node {
                // Doc comments (`///`) are *not* trivia — they're parsed
                // into the AST as `doc_comments` on the next item. Skip
                // them entirely here so the formatter doesn't double-emit.
                Token::LineComment(text) if is_doc_comment(text) => continue,
                Token::LineComment(text) | Token::BlockComment(text) => {
                    // Collect comment as trivia
                    self.pending_comments.push(Comment {
                        text: text.clone(),
                        span: spanned.span,
                    });
                    continue;
                }
                Token::Eof => {
                    // Attach any remaining comments to EOF token
                    let comments = std::mem::take(&mut self.pending_comments);
                    return Some(TriviaToken::with_comments(spanned, comments));
                }
                _ => {
                    // Non-comment token: attach pending comments
                    let comments = std::mem::take(&mut self.pending_comments);
                    return Some(TriviaToken::with_comments(spanned, comments));
                }
            }
        }
    }
}

/// Strip non-doc comment tokens from a list of spanned tokens.
///
/// Block comments and regular `//` line comments are dropped. Doc comments
/// (lines beginning with `///`) are kept so the parser can attach them to
/// the following item as `doc_comments`.
pub fn strip_comments(tokens: Vec<Spanned<Token>>) -> Vec<Spanned<Token>> {
    tokens
        .into_iter()
        .filter(|t| match &t.node {
            Token::BlockComment(_) => false,
            Token::LineComment(text) => is_doc_comment(text),
            _ => true,
        })
        .collect()
}

/// Does this `//`-style comment start with `///` (i.e. is it a doc comment)?
/// Note that `////` (four or more slashes) is treated as a regular comment,
/// matching the Rust convention.
pub fn is_doc_comment(text: &str) -> bool {
    text.starts_with("///") && !text.starts_with("////")
}

/// Strip the leading `///` (and an optional single space) from a doc-comment
/// line, returning just the human-readable text. `/// foo bar` → `"foo bar"`.
pub fn doc_comment_text(text: &str) -> String {
    let after = text.strip_prefix("///").unwrap_or(text);
    after.strip_prefix(' ').unwrap_or(after).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trivia_simple_comment() {
        let source = "// comment\nfn main() {}";
        let tokens: Vec<_> = TriviaLexer::new(source).collect();

        assert_eq!(tokens.len(), 7); // fn, main, (, ), {, }, Eof
        assert_eq!(tokens[0].leading_comments.len(), 1);
        assert_eq!(tokens[0].leading_comments[0].text, "// comment");
        assert!(matches!(tokens[0].token.node, Token::Fn));
    }

    #[test]
    fn test_trivia_multiple_comments() {
        let source = "// first\n// second\nfn main() {}";
        let tokens: Vec<_> = TriviaLexer::new(source).collect();

        assert_eq!(tokens[0].leading_comments.len(), 2);
        assert_eq!(tokens[0].leading_comments[0].text, "// first");
        assert_eq!(tokens[0].leading_comments[1].text, "// second");
    }

    #[test]
    fn test_trivia_block_comment() {
        let source = "/* block */\nfn main() {}";
        let tokens: Vec<_> = TriviaLexer::new(source).collect();

        assert_eq!(tokens[0].leading_comments.len(), 1);
        assert_eq!(tokens[0].leading_comments[0].text, "/* block */");
    }

    #[test]
    fn test_strip_comments() {
        // `//` regular comments and block comments are stripped, but `///`
        // doc comments are kept for the parser to attach to the next item.
        let source = "// regular\n/// doc\n/* block */ fn main() {}";
        let tokens = Lexer::tokenize(source);
        let stripped = strip_comments(tokens);

        let line_comments: Vec<_> = stripped
            .iter()
            .filter_map(|t| match &t.node {
                Token::LineComment(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(line_comments, vec!["/// doc".to_string()]);
        // Block comments are still stripped.
        assert!(
            !stripped
                .iter()
                .any(|t| matches!(t.node, Token::BlockComment(_)))
        );
    }

    #[test]
    fn test_doc_comment_text_strips_prefix() {
        assert_eq!(doc_comment_text("/// hello"), "hello");
        assert_eq!(doc_comment_text("///hello"), "hello");
        assert_eq!(doc_comment_text("///  hello"), " hello");
    }

    #[test]
    fn test_is_doc_comment() {
        assert!(is_doc_comment("/// doc"));
        assert!(!is_doc_comment("// regular"));
        assert!(!is_doc_comment("//// not a doc")); // four slashes — Rust convention
    }
}
