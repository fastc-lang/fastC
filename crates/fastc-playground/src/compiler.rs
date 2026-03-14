use serde::{Deserialize, Serialize};

/// Compilation request
#[derive(Debug, Deserialize)]
pub struct CompileRequest {
    pub code: String,
    #[serde(default)]
    pub emit_header: bool,
}

/// Check request
#[derive(Debug, Deserialize)]
pub struct CheckRequest {
    pub code: String,
}

/// Format request
#[derive(Debug, Deserialize)]
pub struct FormatRequest {
    pub code: String,
}

/// Run request
#[derive(Debug, Deserialize)]
pub struct RunRequest {
    pub code: String,
}

/// Compilation error details
#[derive(Debug, Serialize)]
pub struct CompileErrorInfo {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SpanInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Span information for errors
#[derive(Debug, Serialize)]
pub struct SpanInfo {
    pub start: usize,
    pub end: usize,
}

/// Convert byte offset to line number (1-indexed)
fn offset_to_line(code: &str, offset: usize) -> usize {
    let offset = offset.min(code.len());
    code[..offset].chars().filter(|&c| c == '\n').count() + 1
}

/// Extract span and hint from a CompileError by matching on variants
fn extract_error_info(code: &str, e: &fastc::diag::CompileError) -> CompileErrorInfo {
    use fastc::diag::CompileError;

    // Extract span and hint by matching on error variants
    let (span_range, hint) = match e {
        CompileError::Parse { span, hint, .. } => (Some(span.clone()), hint.clone()),
        CompileError::Resolve { span, hint, .. } => (Some(span.clone()), hint.clone()),
        CompileError::Type { span, hint, .. } => (Some(span.clone()), hint.clone()),
        CompileError::Safety { span, hint, .. } => (Some(span.clone()), hint.clone()),
        CompileError::Multiple { errors } => {
            // For multiple errors, use the first error's span
            if let Some(first) = errors.first() {
                return extract_error_info(code, first);
            }
            (None, None)
        }
    };

    let (span, line) = if let Some(range) = span_range {
        let line = offset_to_line(code, range.start);
        (
            Some(SpanInfo {
                start: range.start,
                end: range.end,
            }),
            Some(line),
        )
    } else {
        (None, None)
    };

    CompileErrorInfo {
        message: format!("{}", e),
        span,
        line,
        hint,
    }
}

/// Compilation response
#[derive(Debug, Serialize)]
pub struct CompileResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CompileErrorInfo>,
}

/// Check response
#[derive(Debug, Serialize)]
pub struct CheckResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<CompileErrorInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CompileErrorInfo>,
}

/// Format response
#[derive(Debug, Serialize)]
pub struct FormatResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CompileErrorInfo>,
}

/// Run response
#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CompileErrorInfo>,
}

/// Compile FastC code to C
pub fn compile(code: &str, emit_header: bool) -> CompileResponse {
    match fastc::compile_with_options(code, "playground.fc", emit_header) {
        Ok((c_code, header)) => CompileResponse {
            success: true,
            c_code: Some(c_code),
            header,
            error: None,
        },
        Err(e) => CompileResponse {
            success: false,
            c_code: None,
            header: None,
            error: Some(extract_error_info(code, &e)),
        },
    }
}

/// Type-check FastC code without compiling
pub fn check(code: &str) -> CheckResponse {
    match fastc::check(code, "playground.fc") {
        Ok(_) => CheckResponse {
            success: true,
            diagnostics: Some(vec![]),
            error: None,
        },
        Err(e) => CheckResponse {
            success: false,
            diagnostics: None,
            error: Some(extract_error_info(code, &e)),
        },
    }
}

/// Format FastC code
pub fn format(code: &str) -> FormatResponse {
    match fastc::format(code, "playground.fc") {
        Ok(formatted) => FormatResponse {
            success: true,
            formatted: Some(formatted),
            error: None,
        },
        Err(e) => FormatResponse {
            success: false,
            formatted: None,
            error: Some(extract_error_info(code, &e)),
        },
    }
}
