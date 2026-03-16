//! Power of 10 Rule 3: No Dynamic Memory Allocation After Initialization
//!
//! "Do not use dynamic memory allocation after initialization."
//!
//! Rationale: Memory allocators, such as malloc, and garbage collectors
//! often have unpredictable behavior that can significantly impact
//! performance. A notable class of coding errors also stems from the
//! mishandling of memory allocation and free routines.
//!
//! Note: FastC doesn't have malloc as a built-in, so this rule primarily
//! checks for FFI calls to known allocation functions.

use crate::ast::Expr;
use super::{P10Config, P10Rule, P10Violation};
use crate::p10::config::SafetyLevel;

/// Known memory allocation functions to flag
const ALLOC_FUNCTIONS: &[&str] = &[
    "malloc",
    "calloc",
    "realloc",
    "free",
    "alloca",
    "aligned_alloc",
    "posix_memalign",
    "valloc",
    "pvalloc",
    "memalign",
    // C++ allocators (if called via FFI)
    "operator new",
    "operator delete",
];

/// Rule 3: No runtime dynamic memory allocation
pub struct MemoryRule;

impl MemoryRule {
    /// Check if a function name is a known allocator
    fn is_alloc_function(&self, name: &str) -> bool {
        ALLOC_FUNCTIONS.contains(&name)
    }
}

impl P10Rule for MemoryRule {
    fn rule_number(&self) -> u8 {
        3
    }

    fn name(&self) -> &'static str {
        "no-runtime-alloc"
    }

    fn description(&self) -> &'static str {
        "No dynamic memory allocation after initialization"
    }

    fn is_enabled(&self, config: &P10Config) -> bool {
        // Enabled by default in Standard and SafetyCritical modes
        config.level != SafetyLevel::Relaxed
    }

    fn check_expr(&self, expr: &Expr, _config: &P10Config, _source: &str) -> Vec<P10Violation> {
        if let Expr::Call { callee, span, .. } = expr {
            // Check if the callee is a simple identifier (function name)
            if let Expr::Ident { name, .. } = callee.as_ref() {
                if self.is_alloc_function(name) {
                    return vec![
                        P10Violation::new(
                            3,
                            format!("call to memory allocator '{}' is not allowed in safety-critical code", name),
                            span.clone(),
                        )
                        .with_help("Power of 10 Rule 3 forbids dynamic memory allocation after initialization")
                        .with_note("Use statically allocated memory or preallocate during initialization"),
                    ];
                }
            }
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_malloc_call_flagged() {
        let expr = Expr::Call {
            callee: Box::new(Expr::Ident {
                name: "malloc".to_string(),
                span: 0..6,
            }),
            args: vec![Expr::IntLit { value: 100, span: 7..10 }],
            span: 0..11,
        };
        let config = P10Config::safety_critical();
        let rule = MemoryRule;

        let violations = rule.check_expr(&expr, &config, "");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, 3);
        assert!(violations[0].message.contains("malloc"));
    }

    #[test]
    fn test_safe_call_passes() {
        let expr = Expr::Call {
            callee: Box::new(Expr::Ident {
                name: "printf".to_string(),
                span: 0..6,
            }),
            args: vec![],
            span: 0..8,
        };
        let config = P10Config::safety_critical();
        let rule = MemoryRule;

        let violations = rule.check_expr(&expr, &config, "");
        assert!(violations.is_empty());
    }

    #[test]
    fn test_free_call_flagged() {
        let expr = Expr::Call {
            callee: Box::new(Expr::Ident {
                name: "free".to_string(),
                span: 0..4,
            }),
            args: vec![Expr::Ident { name: "ptr".to_string(), span: 5..8 }],
            span: 0..9,
        };
        let config = P10Config::safety_critical();
        let rule = MemoryRule;

        let violations = rule.check_expr(&expr, &config, "");
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("free"));
    }
}
