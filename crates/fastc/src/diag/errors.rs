//! Error types for FastC compilation

use miette::Diagnostic;
use thiserror::Error;

use crate::lexer::Span;

/// Main compilation error type
#[derive(Debug, Error, Diagnostic)]
pub enum CompileError {
    #[error("Parse error: {message}")]
    #[diagnostic(code(fastc::parse), help("{}", hint.as_deref().unwrap_or("")))]
    Parse {
        message: String,
        #[label("here")]
        span: Span,
        #[source_code]
        src: String,
        hint: Option<String>,
    },

    #[error("Resolution error: {message}")]
    #[diagnostic(code(fastc::resolve), help("{}", hint.as_deref().unwrap_or("")))]
    Resolve {
        message: String,
        #[label("here")]
        span: Span,
        #[source_code]
        src: String,
        hint: Option<String>,
    },

    #[error("Type error: {message}")]
    #[diagnostic(code(fastc::typecheck), help("{}", hint.as_deref().unwrap_or("")))]
    Type {
        message: String,
        #[label("here")]
        span: Span,
        #[source_code]
        src: String,
        hint: Option<String>,
    },

    #[error("Safety error: {message}")]
    #[diagnostic(code(fastc::safety), help("{}", hint.as_deref().unwrap_or("")))]
    Safety {
        message: String,
        #[label("here")]
        span: Span,
        #[source_code]
        src: String,
        hint: Option<String>,
    },

    #[error("Multiple errors occurred")]
    #[diagnostic(code(fastc::multiple))]
    Multiple {
        #[related]
        errors: Vec<CompileError>,
    },
}

impl CompileError {
    pub fn parse(message: impl Into<String>, span: Span, src: &str) -> Self {
        CompileError::Parse {
            message: message.into(),
            span,
            src: src.to_string(),
            hint: None,
        }
    }

    pub fn parse_with_hint(
        message: impl Into<String>,
        span: Span,
        src: &str,
        hint: impl Into<String>,
    ) -> Self {
        CompileError::Parse {
            message: message.into(),
            span,
            src: src.to_string(),
            hint: Some(hint.into()),
        }
    }

    pub fn resolve(message: impl Into<String>, span: Span, src: &str) -> Self {
        CompileError::Resolve {
            message: message.into(),
            span,
            src: src.to_string(),
            hint: None,
        }
    }

    pub fn resolve_with_hint(
        message: impl Into<String>,
        span: Span,
        src: &str,
        hint: impl Into<String>,
    ) -> Self {
        CompileError::Resolve {
            message: message.into(),
            span,
            src: src.to_string(),
            hint: Some(hint.into()),
        }
    }

    pub fn type_error(message: impl Into<String>, span: Span, src: &str) -> Self {
        CompileError::Type {
            message: message.into(),
            span,
            src: src.to_string(),
            hint: None,
        }
    }

    pub fn type_error_with_hint(
        message: impl Into<String>,
        span: Span,
        src: &str,
        hint: impl Into<String>,
    ) -> Self {
        CompileError::Type {
            message: message.into(),
            span,
            src: src.to_string(),
            hint: Some(hint.into()),
        }
    }

    pub fn safety(message: impl Into<String>, span: Span, src: &str) -> Self {
        CompileError::Safety {
            message: message.into(),
            span,
            src: src.to_string(),
            hint: None,
        }
    }

    pub fn safety_with_hint(
        message: impl Into<String>,
        span: Span,
        src: &str,
        hint: impl Into<String>,
    ) -> Self {
        CompileError::Safety {
            message: message.into(),
            span,
            src: src.to_string(),
            hint: Some(hint.into()),
        }
    }

    /// Create an error from multiple errors
    /// If there's only one error, returns that error directly
    /// If there are multiple errors, wraps them in a Multiple variant
    pub fn multiple(mut errors: Vec<CompileError>) -> Self {
        match errors.len() {
            0 => panic!("Cannot create error from empty error list"),
            1 => errors.remove(0),
            _ => CompileError::Multiple { errors },
        }
    }
}
