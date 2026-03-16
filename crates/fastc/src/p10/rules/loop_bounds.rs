//! Power of 10 Rule 2: Loop Bounds
//!
//! "Give all loops a fixed upper bound. It must be trivially possible for
//! a checking tool to prove statically that the loop cannot exceed a preset
//! upper bound on the number of iterations."
//!
//! Rationale: The absence of recursion and the presence of loop bounds
//! prevents runaway code. This rule does not apply to iterations that are
//! meant to be nonterminating (e.g., in a process scheduler).

use crate::ast::{Expr, Stmt};
use super::{P10Config, P10Rule, P10Violation};
use crate::p10::config::SafetyLevel;

/// Rule 2: All loops must have provable bounds
pub struct LoopBoundsRule;

impl LoopBoundsRule {
    /// Check if a while loop condition is always true (unbounded)
    fn is_unbounded_while(&self, cond: &Expr) -> bool {
        match cond {
            Expr::BoolLit { value: true, .. } => true,
            Expr::Paren { inner, .. } => self.is_unbounded_while(inner),
            Expr::IntLit { value, .. } if *value != 0 => true,
            _ => false,
        }
    }

    /// Check if a for loop has a clear termination condition
    fn is_bounded_for(&self, cond: &Option<Expr>) -> bool {
        // A for loop without a condition is unbounded
        if cond.is_none() {
            return false;
        }

        // For now, we consider any condition as potentially bounded
        // A more sophisticated analysis would verify the condition
        // involves a comparison with the loop variable
        true
    }
}

impl P10Rule for LoopBoundsRule {
    fn rule_number(&self) -> u8 {
        2
    }

    fn name(&self) -> &'static str {
        "loop-bounds"
    }

    fn description(&self) -> &'static str {
        "All loops must have provable upper bounds"
    }

    fn is_enabled(&self, config: &P10Config) -> bool {
        // Enabled by default in Standard and SafetyCritical modes
        config.level != SafetyLevel::Relaxed
    }

    fn check_stmt(&self, stmt: &Stmt, _config: &P10Config, _source: &str) -> Vec<P10Violation> {
        match stmt {
            Stmt::While { cond, span, .. } => {
                if self.is_unbounded_while(cond) {
                    vec![
                        P10Violation::new(
                            2,
                            "while loop has no provable upper bound",
                            span.clone(),
                        )
                        .with_help("Power of 10 Rule 2 requires all loops have provable termination")
                        .with_note("Consider using a for loop with explicit bounds or add a maximum iteration counter"),
                    ]
                } else {
                    vec![]
                }
            }
            Stmt::For { cond, span, .. } => {
                if !self.is_bounded_for(cond) {
                    vec![
                        P10Violation::new(
                            2,
                            "for loop has no termination condition",
                            span.clone(),
                        )
                        .with_help("Power of 10 Rule 2 requires all loops have provable termination")
                        .with_note("Add a condition to ensure the loop terminates"),
                    ]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Block;

    #[test]
    fn test_unbounded_while_true() {
        let stmt = Stmt::While {
            cond: Expr::BoolLit { value: true, span: 0..4 },
            body: Block { stmts: vec![], span: 5..7 },
            span: 0..7,
        };
        let config = P10Config::safety_critical();
        let rule = LoopBoundsRule;

        let violations = rule.check_stmt(&stmt, &config, "");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, 2);
    }

    #[test]
    fn test_bounded_while_passes() {
        let stmt = Stmt::While {
            cond: Expr::Binary {
                op: crate::ast::BinOp::Lt,
                lhs: Box::new(Expr::Ident { name: "i".to_string(), span: 0..1 }),
                rhs: Box::new(Expr::IntLit { value: 10, span: 4..6 }),
                span: 0..6,
            },
            body: Block { stmts: vec![], span: 7..9 },
            span: 0..9,
        };
        let config = P10Config::safety_critical();
        let rule = LoopBoundsRule;

        let violations = rule.check_stmt(&stmt, &config, "");
        assert!(violations.is_empty());
    }

    #[test]
    fn test_for_without_condition() {
        let stmt = Stmt::For {
            init: None,
            cond: None,
            step: None,
            body: Block { stmts: vec![], span: 10..12 },
            span: 0..12,
        };
        let config = P10Config::safety_critical();
        let rule = LoopBoundsRule;

        let violations = rule.check_stmt(&stmt, &config, "");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, 2);
    }
}
