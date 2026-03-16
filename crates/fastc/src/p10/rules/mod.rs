//! Power of 10 rule definitions
//!
//! Each rule corresponds to one of NASA/JPL's Power of 10 rules:
//! 1. No goto, setjmp/longjmp, or recursion
//! 2. Fixed upper bounds on all loops
//! 3. No dynamic memory allocation after initialization
//! 4. Functions <= 60 lines
//! 5. Minimum 2 assertions per function
//! 6. Smallest possible scope for data (enforced by FastC design)
//! 7. Check all return values (partially enforced by opt/res types)
//! 8. Limited preprocessor (FastC has none - fully satisfied)
//! 9. Restricted pointer use (single dereference level)
//! 10. Zero warnings with strict analysis

pub mod control_flow;
pub mod function_size;
pub mod loop_bounds;
pub mod memory;
pub mod pointers;

use crate::ast::{Block, Expr, File, FnDecl, Item, Stmt};
use crate::lexer::Span;
use super::config::P10Config;

/// A Power of 10 rule violation
#[derive(Debug, Clone)]
pub struct P10Violation {
    /// Rule number (1-10)
    pub rule: u8,
    /// Error code (e.g., "P10-001")
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Source location
    pub span: Span,
    /// Optional help text
    pub help: Option<String>,
    /// Optional note with additional context
    pub note: Option<String>,
}

impl P10Violation {
    /// Create a new violation
    pub fn new(rule: u8, message: impl Into<String>, span: Span) -> Self {
        Self {
            rule,
            code: format!("P10-{:03}", rule),
            message: message.into(),
            span,
            help: None,
            note: None,
        }
    }

    /// Add help text
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Add a note
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// Trait for implementing Power of 10 rules
pub trait P10Rule: Send + Sync {
    /// Rule number (1-10)
    fn rule_number(&self) -> u8;

    /// Rule name for display
    fn name(&self) -> &'static str;

    /// Brief description of the rule
    fn description(&self) -> &'static str;

    /// Check if this rule is enabled for the given config
    fn is_enabled(&self, config: &P10Config) -> bool;

    /// Check a complete file
    fn check_file(&self, _file: &File, _config: &P10Config, _source: &str) -> Vec<P10Violation> {
        vec![]
    }

    /// Check a function declaration
    fn check_function(&self, _func: &FnDecl, _config: &P10Config, _source: &str) -> Vec<P10Violation> {
        vec![]
    }

    /// Check a statement
    fn check_stmt(&self, _stmt: &Stmt, _config: &P10Config, _source: &str) -> Vec<P10Violation> {
        vec![]
    }

    /// Check an expression
    fn check_expr(&self, _expr: &Expr, _config: &P10Config, _source: &str) -> Vec<P10Violation> {
        vec![]
    }

    /// Check a block
    fn check_block(&self, _block: &Block, _config: &P10Config, _source: &str) -> Vec<P10Violation> {
        vec![]
    }
}

/// Registry of all Power of 10 rules
pub struct RuleRegistry {
    rules: Vec<Box<dyn P10Rule>>,
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleRegistry {
    /// Create a new registry with all rules
    pub fn new() -> Self {
        Self {
            rules: vec![
                Box::new(control_flow::ControlFlowRule),
                Box::new(loop_bounds::LoopBoundsRule),
                Box::new(memory::MemoryRule),
                Box::new(function_size::FunctionSizeRule),
                Box::new(pointers::PointerDepthRule),
            ],
        }
    }

    /// Get all enabled rules for a config
    pub fn enabled_rules(&self, config: &P10Config) -> Vec<&dyn P10Rule> {
        self.rules
            .iter()
            .filter(|r| r.is_enabled(config))
            .map(|r| r.as_ref())
            .collect()
    }

    /// Check a file against all enabled rules
    pub fn check_file(&self, file: &File, config: &P10Config, source: &str) -> Vec<P10Violation> {
        let mut violations = Vec::new();

        for rule in self.enabled_rules(config) {
            violations.extend(rule.check_file(file, config, source));

            // Check each item
            for item in &file.items {
                if let Item::Fn(func) = item {
                    violations.extend(rule.check_function(func, config, source));
                    violations.extend(self.check_block_recursive(&func.body, rule, config, source));
                }
            }
        }

        violations
    }

    /// Recursively check a block and its nested statements
    fn check_block_recursive(
        &self,
        block: &Block,
        rule: &dyn P10Rule,
        config: &P10Config,
        source: &str,
    ) -> Vec<P10Violation> {
        let mut violations = Vec::new();
        violations.extend(rule.check_block(block, config, source));

        for stmt in &block.stmts {
            violations.extend(rule.check_stmt(stmt, config, source));
            violations.extend(self.check_stmt_recursive(stmt, rule, config, source));
        }

        violations
    }

    /// Recursively check a statement and its nested blocks/expressions
    fn check_stmt_recursive(
        &self,
        stmt: &Stmt,
        rule: &dyn P10Rule,
        config: &P10Config,
        source: &str,
    ) -> Vec<P10Violation> {
        let mut violations = Vec::new();

        match stmt {
            Stmt::If { cond, then_block, else_block, .. } => {
                violations.extend(rule.check_expr(cond, config, source));
                violations.extend(self.check_block_recursive(then_block, rule, config, source));
                if let Some(else_branch) = else_block {
                    match else_branch {
                        crate::ast::ElseBranch::ElseIf(else_if) => {
                            violations.extend(self.check_stmt_recursive(else_if, rule, config, source));
                        }
                        crate::ast::ElseBranch::Else(else_blk) => {
                            violations.extend(self.check_block_recursive(else_blk, rule, config, source));
                        }
                    }
                }
            }
            Stmt::While { cond, body, .. } => {
                violations.extend(rule.check_expr(cond, config, source));
                violations.extend(self.check_block_recursive(body, rule, config, source));
            }
            Stmt::For { body, .. } => {
                violations.extend(self.check_block_recursive(body, rule, config, source));
            }
            Stmt::Switch { expr, cases, default, .. } => {
                violations.extend(rule.check_expr(expr, config, source));
                for case in cases {
                    for case_stmt in &case.stmts {
                        violations.extend(rule.check_stmt(case_stmt, config, source));
                    }
                }
                if let Some(default_stmts) = default {
                    for default_stmt in default_stmts {
                        violations.extend(rule.check_stmt(default_stmt, config, source));
                    }
                }
            }
            Stmt::Block(block) => {
                violations.extend(self.check_block_recursive(block, rule, config, source));
            }
            Stmt::Unsafe { body, .. } => {
                violations.extend(self.check_block_recursive(body, rule, config, source));
            }
            Stmt::Defer { body, .. } => {
                violations.extend(self.check_block_recursive(body, rule, config, source));
            }
            Stmt::IfLet { expr, then_block, else_block, .. } => {
                violations.extend(rule.check_expr(expr, config, source));
                violations.extend(self.check_block_recursive(then_block, rule, config, source));
                if let Some(else_blk) = else_block {
                    violations.extend(self.check_block_recursive(else_blk, rule, config, source));
                }
            }
            Stmt::Let { init, .. } => {
                violations.extend(rule.check_expr(init, config, source));
            }
            Stmt::Assign { lhs, rhs, .. } => {
                violations.extend(rule.check_expr(lhs, config, source));
                violations.extend(rule.check_expr(rhs, config, source));
            }
            Stmt::Expr { expr, .. } | Stmt::Discard { expr, .. } => {
                violations.extend(rule.check_expr(expr, config, source));
            }
            Stmt::Return { value, .. } => {
                if let Some(val) = value {
                    violations.extend(rule.check_expr(val, config, source));
                }
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }

        violations
    }
}
