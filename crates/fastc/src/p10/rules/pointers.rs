//! Power of 10 Rule 9: Restricted Pointer Use
//!
//! "The use of pointers must be restricted. Specifically, no more than
//! one level of dereferencing should be used. Pointer dereference operations
//! may not be hidden in macro definitions or inside typedef declarations.
//! Function pointers are not permitted."
//!
//! Rationale: Pointers are easily misused, even by experienced programmers.
//! They can make it hard to follow or analyze the flow of data in a program,
//! especially by tool-based analyzers.

use crate::ast::Expr;
use super::{P10Config, P10Rule, P10Violation};
use crate::p10::config::SafetyLevel;

/// Rule 9: Restrict pointer dereference depth
pub struct PointerDepthRule;

impl PointerDepthRule {
    /// Calculate the dereference depth of an expression
    fn deref_depth(&self, expr: &Expr) -> usize {
        match expr {
            Expr::Deref { operand, .. } => 1 + self.deref_depth(operand),
            Expr::Paren { inner, .. } => self.deref_depth(inner),
            _ => 0,
        }
    }
}

impl P10Rule for PointerDepthRule {
    fn rule_number(&self) -> u8 {
        9
    }

    fn name(&self) -> &'static str {
        "pointer-depth"
    }

    fn description(&self) -> &'static str {
        "No more than one level of pointer dereferencing"
    }

    fn is_enabled(&self, config: &P10Config) -> bool {
        // Enabled by default in Standard and SafetyCritical modes
        config.level != SafetyLevel::Relaxed
    }

    fn check_expr(&self, expr: &Expr, config: &P10Config, _source: &str) -> Vec<P10Violation> {
        // Only check deref expressions
        if let Expr::Deref { span, .. } = expr {
            let depth = self.deref_depth(expr);
            if depth > config.max_pointer_depth {
                return vec![
                    P10Violation::new(
                        9,
                        format!(
                            "pointer dereference depth {} exceeds maximum of {}",
                            depth, config.max_pointer_depth
                        ),
                        span.clone(),
                    )
                    .with_help("Power of 10 Rule 9 restricts pointer use to single dereference")
                    .with_note("Consider using intermediate variables or restructuring data"),
                ];
            }
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_deref_passes() {
        let expr = Expr::Deref {
            operand: Box::new(Expr::Ident {
                name: "ptr".to_string(),
                span: 6..9,
            }),
            span: 0..10,
        };
        let config = P10Config::safety_critical();
        let rule = PointerDepthRule;

        let violations = rule.check_expr(&expr, &config, "");
        assert!(violations.is_empty());
    }

    #[test]
    fn test_double_deref_fails() {
        let expr = Expr::Deref {
            operand: Box::new(Expr::Deref {
                operand: Box::new(Expr::Ident {
                    name: "pptr".to_string(),
                    span: 12..16,
                }),
                span: 6..17,
            }),
            span: 0..18,
        };
        let config = P10Config::safety_critical();
        let rule = PointerDepthRule;

        let violations = rule.check_expr(&expr, &config, "");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, 9);
        assert!(violations[0].message.contains("2"));
    }

    #[test]
    fn test_triple_deref_fails() {
        let inner = Expr::Deref {
            operand: Box::new(Expr::Ident {
                name: "ppptr".to_string(),
                span: 18..23,
            }),
            span: 12..24,
        };
        let middle = Expr::Deref {
            operand: Box::new(inner),
            span: 6..25,
        };
        let expr = Expr::Deref {
            operand: Box::new(middle),
            span: 0..26,
        };
        let config = P10Config::safety_critical();
        let rule = PointerDepthRule;

        let violations = rule.check_expr(&expr, &config, "");
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("3"));
    }

    #[test]
    fn test_relaxed_mode_allows_deep_deref() {
        let expr = Expr::Deref {
            operand: Box::new(Expr::Deref {
                operand: Box::new(Expr::Ident {
                    name: "pptr".to_string(),
                    span: 12..16,
                }),
                span: 6..17,
            }),
            span: 0..18,
        };
        let config = P10Config::relaxed();
        let rule = PointerDepthRule;

        // Rule is disabled in relaxed mode
        assert!(!rule.is_enabled(&config));
    }
}
