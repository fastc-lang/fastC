//! Unified JSON diagnostic envelope.
//!
//! Every fastC diagnostic — compile errors, P10 violations, capability
//! violations, contract violations, discharge failures — funnels
//! through `Diagnostic` so MCP servers, CI tooling, and editors get
//! a single shape to parse.
//!
//! The shape is additive: new fields appear over time, existing
//! fields keep their meaning. Consumers are expected to ignore
//! unknown fields.

use crate::diag::CompileError;
use crate::lexer::Span;

/// One diagnostic in the unified envelope.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// What kind of diagnostic this is. Determines which `rule_id`
    /// values are valid.
    pub kind: DiagnosticKind,
    /// Stable rule identifier (e.g. `E_UNRESOLVED_NAME`, `p10-rule-4`).
    pub rule_id: String,
    /// `error` or `warning`.
    pub severity: Severity,
    /// Source file + byte span. None when the diagnostic isn't tied
    /// to a specific location.
    pub span: Option<SpanInfo>,
    /// One-line human-readable message.
    pub message: String,
    /// Optional fix-it hint (free text).
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticKind {
    CompileError,
    P10Violation,
    CapabilityViolation,
    ContractViolation,
    DischargeFailure,
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct SpanInfo {
    pub file: String,
    pub start: usize,
    pub end: usize,
}

impl Diagnostic {
    pub fn to_json(&self) -> String {
        let kind = match self.kind {
            DiagnosticKind::CompileError => "compile_error",
            DiagnosticKind::P10Violation => "p10_violation",
            DiagnosticKind::CapabilityViolation => "capability_violation",
            DiagnosticKind::ContractViolation => "contract_violation",
            DiagnosticKind::DischargeFailure => "discharge_failure",
        };
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let span_json = match &self.span {
            Some(s) => format!(
                "{{\"file\": \"{}\", \"start\": {}, \"end\": {}}}",
                escape(&s.file),
                s.start,
                s.end
            ),
            None => "null".to_string(),
        };
        let hint_json = match &self.hint {
            Some(h) => format!("\"{}\"", escape(h)),
            None => "null".to_string(),
        };
        format!(
            "{{\n  \"kind\": \"{}\",\n  \"rule_id\": \"{}\",\n  \"severity\": \"{}\",\n  \"span\": {},\n  \"message\": \"{}\",\n  \"hint\": {}\n}}",
            kind,
            escape(&self.rule_id),
            severity,
            span_json,
            escape(&self.message),
            hint_json
        )
    }
}

/// Convert a `CompileError` to a unified `Diagnostic`. The mapping
/// uses each variant's natural rule_id; `Multiple` recursively
/// flattens into a Vec<Diagnostic> via `diagnostics_from_error`.
pub fn from_compile_error(err: &CompileError, file: &str) -> Diagnostic {
    match err {
        CompileError::Parse {
            message,
            span,
            hint,
            ..
        } => Diagnostic {
            kind: DiagnosticKind::CompileError,
            rule_id: "E_PARSE".to_string(),
            severity: Severity::Error,
            span: Some(span_info(file, span)),
            message: message.clone(),
            hint: hint.clone(),
        },
        CompileError::Resolve {
            message,
            span,
            hint,
            ..
        } => Diagnostic {
            kind: DiagnosticKind::CompileError,
            rule_id: "E_RESOLVE".to_string(),
            severity: Severity::Error,
            span: Some(span_info(file, span)),
            message: message.clone(),
            hint: hint.clone(),
        },
        CompileError::Type {
            message,
            span,
            hint,
            ..
        } => Diagnostic {
            kind: DiagnosticKind::CompileError,
            rule_id: "E_TYPE".to_string(),
            severity: Severity::Error,
            span: Some(span_info(file, span)),
            message: message.clone(),
            hint: hint.clone(),
        },
        CompileError::Safety {
            message,
            span,
            hint,
            ..
        } => Diagnostic {
            kind: DiagnosticKind::CompileError,
            rule_id: "E_SAFETY".to_string(),
            severity: Severity::Error,
            span: Some(span_info(file, span)),
            message: message.clone(),
            hint: hint.clone(),
        },
        CompileError::P10 {
            code,
            message,
            span,
            hint,
            ..
        } => Diagnostic {
            kind: DiagnosticKind::P10Violation,
            rule_id: code.clone(),
            severity: Severity::Error,
            span: Some(span_info(file, span)),
            message: message.clone(),
            hint: hint.clone(),
        },
        CompileError::Multiple { errors } => {
            // Multiple at the top level: roll up into a single
            // diagnostic that summarizes the count. Callers wanting
            // individual diagnostics should use `flatten_compile_error`.
            Diagnostic {
                kind: DiagnosticKind::CompileError,
                rule_id: "E_MULTIPLE".to_string(),
                severity: Severity::Error,
                span: None,
                message: format!("{} errors occurred", errors.len()),
                hint: None,
            }
        }
    }
}

/// Recursively flatten a `Multiple` error into a list of individual
/// diagnostics. Single errors return a one-element vec.
pub fn flatten_compile_error(err: &CompileError, file: &str) -> Vec<Diagnostic> {
    match err {
        CompileError::Multiple { errors } => errors
            .iter()
            .flat_map(|e| flatten_compile_error(e, file))
            .collect(),
        other => vec![from_compile_error(other, file)],
    }
}

/// Serialize a list of diagnostics as a JSON array.
pub fn diagnostics_array_json(diags: &[Diagnostic]) -> String {
    let mut out = String::from("[\n");
    for (i, d) in diags.iter().enumerate() {
        out.push_str(&d.to_json());
        if i + 1 < diags.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push(']');
    out
}

fn span_info(file: &str, span: &Span) -> SpanInfo {
    SpanInfo {
        file: file.to_string(),
        start: span.start,
        end: span.end,
    }
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_serializes_to_canonical_json() {
        let d = Diagnostic {
            kind: DiagnosticKind::CompileError,
            rule_id: "E_PARSE".to_string(),
            severity: Severity::Error,
            span: Some(SpanInfo {
                file: "foo.fc".to_string(),
                start: 10,
                end: 12,
            }),
            message: "expected ;".to_string(),
            hint: Some("add a semicolon".to_string()),
        };
        let json = d.to_json();
        assert!(json.contains("\"kind\": \"compile_error\""));
        assert!(json.contains("\"rule_id\": \"E_PARSE\""));
        assert!(json.contains("\"severity\": \"error\""));
        assert!(json.contains("\"file\": \"foo.fc\""));
        assert!(json.contains("\"start\": 10"));
        assert!(json.contains("\"hint\": \"add a semicolon\""));
    }

    #[test]
    fn diagnostic_with_no_hint_or_span_is_null_serialized() {
        let d = Diagnostic {
            kind: DiagnosticKind::DischargeFailure,
            rule_id: "smt_timeout".to_string(),
            severity: Severity::Warning,
            span: None,
            message: "obligation timed out".to_string(),
            hint: None,
        };
        let json = d.to_json();
        assert!(json.contains("\"span\": null"));
        assert!(json.contains("\"hint\": null"));
    }

    #[test]
    fn flatten_multiple_error_produces_individual_diagnostics() {
        let inner_a = CompileError::parse("oops 1", 0..1, "src");
        let inner_b = CompileError::resolve("oops 2", 5..6, "src");
        let multi = CompileError::multiple(vec![inner_a, inner_b]);
        let diags = flatten_compile_error(&multi, "x.fc");
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].rule_id, "E_PARSE");
        assert_eq!(diags[1].rule_id, "E_RESOLVE");
    }
}
