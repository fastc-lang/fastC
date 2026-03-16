//! Power of 10 Rules for Safety-Critical Code
//!
//! This module implements NASA/JPL's "Power of 10: Rules for Developing
//! Safety-Critical Code" by Gerard J. Holzmann. These rules are designed
//! to make critical software more analyzable and verifiable.
//!
//! # The Power of 10 Rules
//!
//! 1. **Simple Control Flow** - No goto, setjmp/longjmp, or recursion
//! 2. **Loop Bounds** - All loops must have provable upper bounds
//! 3. **No Dynamic Allocation** - No malloc after initialization
//! 4. **Function Size** - Functions must fit on one page (60 lines max)
//! 5. **Assertion Density** - Minimum 2 assertions per function
//! 6. **Data Scope** - Declare data at smallest possible scope
//! 7. **Return Checking** - Check all return values
//! 8. **Limited Preprocessor** - Minimal macro use (N/A for FastC)
//! 9. **Pointer Restriction** - Single level of dereferencing only
//! 10. **Zero Warnings** - Compile with all warnings, pass all analyzers
//!
//! # Usage
//!
//! ```ignore
//! use fastc::p10::{P10Checker, P10Config, SafetyLevel};
//!
//! let config = P10Config::safety_critical();
//! let checker = P10Checker::new(config);
//! let violations = checker.check(&ast, source);
//! ```

pub mod config;
pub mod report;
pub mod rules;

pub use config::{P10Config, SafetyLevel};
pub use report::{ComplianceReport, ComplianceStatus, ProjectReport, ReportSummary};
pub use rules::{P10Rule, P10Violation, RuleRegistry};

use crate::ast::File;
use crate::diag::CompileError;
use crate::lexer::Span;

/// Power of 10 rule checker
pub struct P10Checker {
    config: P10Config,
    registry: RuleRegistry,
}

impl P10Checker {
    /// Create a new checker with the given configuration
    pub fn new(config: P10Config) -> Self {
        Self {
            config,
            registry: RuleRegistry::new(),
        }
    }

    /// Create a checker for safety-critical code
    pub fn safety_critical() -> Self {
        Self::new(P10Config::safety_critical())
    }

    /// Create a checker with standard settings
    pub fn standard() -> Self {
        Self::new(P10Config::standard())
    }

    /// Check an AST for Power of 10 violations
    pub fn check(&self, ast: &File, source: &str) -> Vec<P10Violation> {
        if !self.config.is_enabled() {
            return vec![];
        }

        self.registry.check_file(ast, &self.config, source)
    }

    /// Check an AST and convert violations to compile errors
    pub fn check_and_report(&self, ast: &File, source: &str) -> Result<(), CompileError> {
        let violations = self.check(ast, source);

        if violations.is_empty() {
            return Ok(());
        }

        // Convert violations to compile errors
        let errors: Vec<CompileError> = violations
            .into_iter()
            .map(|v| self.violation_to_error(v, source))
            .collect();

        Err(CompileError::multiple(errors))
    }

    /// Convert a P10 violation to a CompileError
    fn violation_to_error(&self, violation: P10Violation, source: &str) -> CompileError {
        let hint = match (&violation.help, &violation.note) {
            (Some(help), Some(note)) => Some(format!("{}\nNote: {}", help, note)),
            (Some(help), None) => Some(help.clone()),
            (None, Some(note)) => Some(format!("Note: {}", note)),
            (None, None) => None,
        };

        CompileError::P10 {
            code: violation.code,
            message: violation.message,
            span: violation.span,
            src: source.to_string(),
            hint,
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &P10Config {
        &self.config
    }

    /// Get list of enabled rules
    pub fn enabled_rules(&self) -> Vec<&dyn P10Rule> {
        self.registry.enabled_rules(&self.config)
    }

    /// Print a summary of enabled rules
    pub fn print_rules_summary(&self) {
        println!("Power of 10 Rules (Safety Level: {:?}):", self.config.level);
        println!();
        for rule in self.enabled_rules() {
            println!(
                "  Rule {:2}: {} - {}",
                rule.rule_number(),
                rule.name(),
                rule.description()
            );
        }
    }
}

/// Extension trait for CompileError to support P10 violations
impl CompileError {
    /// Create a P10 rule violation error
    pub fn p10(code: impl Into<String>, message: impl Into<String>, span: Span, src: &str) -> Self {
        CompileError::P10 {
            code: code.into(),
            message: message.into(),
            span,
            src: src.to_string(),
            hint: None,
        }
    }

    /// Create a P10 rule violation error with hint
    pub fn p10_with_hint(
        code: impl Into<String>,
        message: impl Into<String>,
        span: Span,
        src: &str,
        hint: impl Into<String>,
    ) -> Self {
        CompileError::P10 {
            code: code.into(),
            message: message.into(),
            span,
            src: src.to_string(),
            hint: Some(hint.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Block, FnDecl, Item, TypeExpr};

    fn make_test_file(funcs: Vec<FnDecl>) -> File {
        File {
            items: funcs.into_iter().map(Item::Fn).collect(),
        }
    }

    fn make_func(name: &str, line_count: usize) -> FnDecl {
        // Create a function body that spans the given number of lines
        let body_content: String = (0..line_count)
            .map(|i| format!("    let x{}: i32 = {};\n", i, i))
            .collect();

        let body_span = 0..body_content.len();

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
    fn test_checker_disabled_in_relaxed_mode() {
        let config = P10Config::relaxed();
        let checker = P10Checker::new(config);
        let file = make_test_file(vec![make_func("foo", 100)]);

        let violations = checker.check(&file, "");
        assert!(violations.is_empty());
    }

    #[test]
    fn test_safety_critical_checker() {
        let checker = P10Checker::safety_critical();
        assert_eq!(checker.config().level, SafetyLevel::SafetyCritical);
    }

    #[test]
    fn test_enabled_rules_in_critical_mode() {
        let checker = P10Checker::safety_critical();
        let rules = checker.enabled_rules();

        // Should have rules 1, 2, 3, 4, 9 enabled
        assert!(rules.len() >= 4);
    }
}
