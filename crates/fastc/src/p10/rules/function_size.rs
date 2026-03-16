//! Power of 10 Rule 4: Function Size Limit
//!
//! "No function should be longer than what can be printed on a single sheet
//! of paper in a standard format with one line per statement and one line
//! per declaration. Typically, this means no more than about 60 lines of
//! code per function."
//!
//! Rationale: Each function should be a logical unit in the code that is
//! understandable and verifiable as a unit. It is much harder to understand
//! a logical unit that spans multiple pages. Excessively long functions are
//! often a sign of poorly structured code.

use crate::ast::FnDecl;
use super::{P10Config, P10Rule, P10Violation};
use crate::p10::config::SafetyLevel;

/// Rule 4: Function size limit
pub struct FunctionSizeRule;

impl FunctionSizeRule {
    /// Count the number of lines in a function body
    fn count_lines(&self, func: &FnDecl, source: &str) -> usize {
        let start = func.body.span.start;
        let end = func.body.span.end;

        if end <= start || end > source.len() {
            return 0;
        }

        let body_text = &source[start..end];

        // Count non-empty, non-comment-only lines
        body_text
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty()
                    && !trimmed.starts_with("//")
                    && trimmed != "{"
                    && trimmed != "}"
            })
            .count()
    }
}

impl P10Rule for FunctionSizeRule {
    fn rule_number(&self) -> u8 {
        4
    }

    fn name(&self) -> &'static str {
        "function-size"
    }

    fn description(&self) -> &'static str {
        "Functions must not exceed 60 lines (fit on one printed page)"
    }

    fn is_enabled(&self, config: &P10Config) -> bool {
        // Enabled by default in Standard and SafetyCritical modes
        config.level != SafetyLevel::Relaxed
    }

    fn check_function(&self, func: &FnDecl, config: &P10Config, source: &str) -> Vec<P10Violation> {
        let line_count = self.count_lines(func, source);

        if line_count > config.max_function_lines {
            vec![
                P10Violation::new(
                    4,
                    format!(
                        "function '{}' has {} lines, exceeds {} line limit",
                        func.name, line_count, config.max_function_lines
                    ),
                    func.span.clone(),
                )
                .with_help("Power of 10 Rule 4 requires functions fit on one printed page")
                .with_note("Consider breaking this function into smaller, focused functions"),
            ]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Block, TypeExpr};
    use crate::lexer::Span;

    fn make_func(name: &str, body_span: Span) -> FnDecl {
        FnDecl {
            is_unsafe: false,
            name: name.to_string(),
            params: vec![],
            return_type: TypeExpr::Void,
            body: Block {
                stmts: vec![],
                span: body_span,
            },
            span: 0..100,
        }
    }

    #[test]
    fn test_short_function_passes() {
        let source = r#"fn foo() -> void {
    let x: i32 = 1;
    let y: i32 = 2;
    return;
}"#;
        let func = make_func("foo", 17..source.len());
        let config = P10Config::safety_critical();
        let rule = FunctionSizeRule;

        let violations = rule.check_function(&func, &config, source);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_long_function_fails() {
        // Create a source with more than 60 lines
        let mut lines = vec!["fn foo() -> void {".to_string()];
        for i in 0..70 {
            lines.push(format!("    let x{}: i32 = {};", i, i));
        }
        lines.push("}".to_string());
        let source = lines.join("\n");

        let func = make_func("foo", 17..source.len());
        let config = P10Config::safety_critical();
        let rule = FunctionSizeRule;

        let violations = rule.check_function(&func, &config, source.as_str());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, 4);
        assert!(violations[0].message.contains("foo"));
        assert!(violations[0].message.contains("70"));
    }
}
