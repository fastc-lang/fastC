//! Source code formatter for FastC
//!
//! Parses source to AST and pretty-prints back to canonical format.
//! Comments are preserved by using the trivia-aware lexer.

mod printer;

pub use printer::Formatter;

use crate::diag::CompileError;
use crate::lexer::{strip_comments, Lexer, TriviaLexer};
use crate::parser::Parser;

/// Format FastC source code to canonical format
///
/// Returns the formatted source code. Comments are preserved.
pub fn format(source: &str, filename: &str) -> Result<String, CompileError> {
    // Get trivia tokens for comment preservation
    let trivia_tokens: Vec<_> = TriviaLexer::new(source).collect();

    // Parse source to AST (using stripped tokens)
    let lexer = Lexer::new(source);
    let tokens = strip_comments(lexer.collect());
    let mut parser = Parser::new(&tokens, source, filename);
    let ast = parser.parse_file()?;

    // Format AST back to source with comments
    let mut formatter = Formatter::new();

    // Output leading comments (comments before first item)
    if !trivia_tokens.is_empty() {
        for comment in &trivia_tokens[0].leading_comments {
            formatter.write_comment(&comment.text);
        }
    }

    formatter.format_file(&ast);
    Ok(formatter.finish())
}

/// Check if source is already canonically formatted
///
/// Returns `Ok(true)` if formatted, `Ok(false)` if not, or an error on parse failure.
pub fn check_formatted(source: &str, filename: &str) -> Result<bool, CompileError> {
    let formatted = format(source, filename)?;
    Ok(source == formatted)
}
