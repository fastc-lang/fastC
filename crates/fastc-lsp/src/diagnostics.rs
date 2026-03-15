//! Convert FastC compilation errors to LSP diagnostics

use fastc::diag::CompileError;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

/// Convert a byte offset to an LSP Position (line/character)
pub fn byte_to_position(source: &str, byte_offset: usize) -> Position {
    let mut line = 0u32;
    let mut col = 0u32;
    let mut current_byte = 0usize;

    for ch in source.chars() {
        if current_byte >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        current_byte += ch.len_utf8();
    }

    Position::new(line, col)
}

/// Convert a byte span to an LSP Range
pub fn byte_span_to_range(source: &str, start: usize, end: usize) -> Range {
    Range::new(
        byte_to_position(source, start),
        byte_to_position(source, end),
    )
}

/// Convert a CompileError to LSP Diagnostics
pub fn compile_error_to_diagnostics(error: &CompileError, source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    match error {
        CompileError::Parse {
            message,
            span,
            hint,
            ..
        } => {
            diagnostics.push(Diagnostic {
                range: byte_span_to_range(source, span.start, span.end),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(tower_lsp::lsp_types::NumberOrString::String(
                    "fastc::parse".to_string(),
                )),
                source: Some("fastc".to_string()),
                message: format!("Parse error: {}", message),
                related_information: hint.as_ref().map(|h| {
                    vec![tower_lsp::lsp_types::DiagnosticRelatedInformation {
                        location: tower_lsp::lsp_types::Location {
                            uri: tower_lsp::lsp_types::Url::parse("file:///hint").unwrap(),
                            range: Range::default(),
                        },
                        message: h.clone(),
                    }]
                }),
                ..Default::default()
            });
        }
        CompileError::Resolve {
            message,
            span,
            hint,
            ..
        } => {
            diagnostics.push(Diagnostic {
                range: byte_span_to_range(source, span.start, span.end),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(tower_lsp::lsp_types::NumberOrString::String(
                    "fastc::resolve".to_string(),
                )),
                source: Some("fastc".to_string()),
                message: format!("Resolution error: {}", message),
                related_information: hint.as_ref().map(|h| {
                    vec![tower_lsp::lsp_types::DiagnosticRelatedInformation {
                        location: tower_lsp::lsp_types::Location {
                            uri: tower_lsp::lsp_types::Url::parse("file:///hint").unwrap(),
                            range: Range::default(),
                        },
                        message: h.clone(),
                    }]
                }),
                ..Default::default()
            });
        }
        CompileError::Type {
            message,
            span,
            hint,
            ..
        } => {
            diagnostics.push(Diagnostic {
                range: byte_span_to_range(source, span.start, span.end),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(tower_lsp::lsp_types::NumberOrString::String(
                    "fastc::typecheck".to_string(),
                )),
                source: Some("fastc".to_string()),
                message: format!("Type error: {}", message),
                related_information: hint.as_ref().map(|h| {
                    vec![tower_lsp::lsp_types::DiagnosticRelatedInformation {
                        location: tower_lsp::lsp_types::Location {
                            uri: tower_lsp::lsp_types::Url::parse("file:///hint").unwrap(),
                            range: Range::default(),
                        },
                        message: h.clone(),
                    }]
                }),
                ..Default::default()
            });
        }
        CompileError::Safety {
            message,
            span,
            hint,
            ..
        } => {
            diagnostics.push(Diagnostic {
                range: byte_span_to_range(source, span.start, span.end),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(tower_lsp::lsp_types::NumberOrString::String(
                    "fastc::safety".to_string(),
                )),
                source: Some("fastc".to_string()),
                message: format!("Safety error: {}", message),
                related_information: hint.as_ref().map(|h| {
                    vec![tower_lsp::lsp_types::DiagnosticRelatedInformation {
                        location: tower_lsp::lsp_types::Location {
                            uri: tower_lsp::lsp_types::Url::parse("file:///hint").unwrap(),
                            range: Range::default(),
                        },
                        message: h.clone(),
                    }]
                }),
                ..Default::default()
            });
        }
        CompileError::Multiple { errors } => {
            for error in errors {
                diagnostics.extend(compile_error_to_diagnostics(error, source));
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_to_position_ascii() {
        let source = "line1\nline2";
        assert_eq!(byte_to_position(source, 0), Position::new(0, 0));
        assert_eq!(byte_to_position(source, 6), Position::new(1, 0));
        assert_eq!(byte_to_position(source, source.len()), Position::new(1, 5));
    }

    #[test]
    fn test_byte_span_to_range() {
        let source = "abc\ndef";
        let range = byte_span_to_range(source, 1, 5);
        assert_eq!(range.start, Position::new(0, 1));
        assert_eq!(range.end, Position::new(1, 1));
    }
}
