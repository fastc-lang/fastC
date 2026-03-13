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

/// Strip comment tokens from a list of spanned tokens
///
/// This is useful for the parser which doesn't need comments.
pub fn strip_comments(tokens: Vec<Spanned<Token>>) -> Vec<Spanned<Token>> {
    tokens
        .into_iter()
        .filter(|t| !matches!(t.node, Token::LineComment(_) | Token::BlockComment(_)))
        .collect()
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
        let source = "// comment\nfn main() {}";
        let tokens = Lexer::tokenize(source);
        let stripped = strip_comments(tokens);

        // Should not contain any comment tokens
        for token in &stripped {
            assert!(!matches!(
                token.node,
                Token::LineComment(_) | Token::BlockComment(_)
            ));
        }
    }
}
