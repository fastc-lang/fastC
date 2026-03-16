//! Power of 10 Rule 1: Simple Control Flow
//!
//! "Restrict all code to very simple control flow constructs—do not use
//! goto statements, setjmp or longjmp constructs, or direct or indirect
//! recursion."
//!
//! Rationale: Simpler control flow translates into stronger capabilities
//! for analysis and often results in improved code clarity. Avoiding
//! recursion results in having an acyclic function call graph, which code
//! analyzers can exploit to prove limits on stack use and boundedness of
//! executions.

use std::collections::{HashMap, HashSet};

use crate::ast::{Expr, File, Item, Stmt};
use super::{P10Config, P10Rule, P10Violation};
use crate::p10::config::SafetyLevel;

/// Rule 1: No recursion (goto/setjmp not in FastC)
pub struct ControlFlowRule;

impl ControlFlowRule {
    /// Build a call graph from the AST
    fn build_call_graph(&self, file: &File) -> HashMap<String, HashSet<String>> {
        let mut graph: HashMap<String, HashSet<String>> = HashMap::new();

        // First pass: collect all function names
        for item in &file.items {
            if let Item::Fn(func) = item {
                graph.insert(func.name.clone(), HashSet::new());
            }
        }

        // Second pass: collect all calls
        for item in &file.items {
            if let Item::Fn(func) = item {
                let calls = self.collect_calls(&func.body.stmts, &graph);
                if let Some(edges) = graph.get_mut(&func.name) {
                    edges.extend(calls);
                }
            }
        }

        graph
    }

    /// Collect all function calls from a list of statements
    fn collect_calls(&self, stmts: &[Stmt], known_fns: &HashMap<String, HashSet<String>>) -> HashSet<String> {
        let mut calls = HashSet::new();

        for stmt in stmts {
            self.collect_calls_from_stmt(stmt, known_fns, &mut calls);
        }

        calls
    }

    fn collect_calls_from_stmt(
        &self,
        stmt: &Stmt,
        known_fns: &HashMap<String, HashSet<String>>,
        calls: &mut HashSet<String>,
    ) {
        match stmt {
            Stmt::Let { init, .. } => {
                self.collect_calls_from_expr(init, known_fns, calls);
            }
            Stmt::Assign { lhs, rhs, .. } => {
                self.collect_calls_from_expr(lhs, known_fns, calls);
                self.collect_calls_from_expr(rhs, known_fns, calls);
            }
            Stmt::If { cond, then_block, else_block, .. } => {
                self.collect_calls_from_expr(cond, known_fns, calls);
                calls.extend(self.collect_calls(&then_block.stmts, known_fns));
                if let Some(else_branch) = else_block {
                    match else_branch {
                        crate::ast::ElseBranch::ElseIf(else_if) => {
                            self.collect_calls_from_stmt(else_if, known_fns, calls);
                        }
                        crate::ast::ElseBranch::Else(else_blk) => {
                            calls.extend(self.collect_calls(&else_blk.stmts, known_fns));
                        }
                    }
                }
            }
            Stmt::IfLet { expr, then_block, else_block, .. } => {
                self.collect_calls_from_expr(expr, known_fns, calls);
                calls.extend(self.collect_calls(&then_block.stmts, known_fns));
                if let Some(else_blk) = else_block {
                    calls.extend(self.collect_calls(&else_blk.stmts, known_fns));
                }
            }
            Stmt::While { cond, body, .. } => {
                self.collect_calls_from_expr(cond, known_fns, calls);
                calls.extend(self.collect_calls(&body.stmts, known_fns));
            }
            Stmt::For { init, cond, step, body, .. } => {
                if let Some(init) = init {
                    match init {
                        crate::ast::ForInit::Let { init, .. } => {
                            self.collect_calls_from_expr(init, known_fns, calls);
                        }
                        crate::ast::ForInit::Assign { lhs, rhs } => {
                            self.collect_calls_from_expr(lhs, known_fns, calls);
                            self.collect_calls_from_expr(rhs, known_fns, calls);
                        }
                        crate::ast::ForInit::Call(expr) => {
                            self.collect_calls_from_expr(expr, known_fns, calls);
                        }
                    }
                }
                if let Some(cond) = cond {
                    self.collect_calls_from_expr(cond, known_fns, calls);
                }
                if let Some(step) = step {
                    match step {
                        crate::ast::ForStep::Assign { lhs, rhs } => {
                            self.collect_calls_from_expr(lhs, known_fns, calls);
                            self.collect_calls_from_expr(rhs, known_fns, calls);
                        }
                        crate::ast::ForStep::Call(expr) => {
                            self.collect_calls_from_expr(expr, known_fns, calls);
                        }
                    }
                }
                calls.extend(self.collect_calls(&body.stmts, known_fns));
            }
            Stmt::Switch { expr, cases, default, .. } => {
                self.collect_calls_from_expr(expr, known_fns, calls);
                for case in cases {
                    calls.extend(self.collect_calls(&case.stmts, known_fns));
                }
                if let Some(default_stmts) = default {
                    calls.extend(self.collect_calls(default_stmts, known_fns));
                }
            }
            Stmt::Return { value, .. } => {
                if let Some(val) = value {
                    self.collect_calls_from_expr(val, known_fns, calls);
                }
            }
            Stmt::Expr { expr, .. } | Stmt::Discard { expr, .. } => {
                self.collect_calls_from_expr(expr, known_fns, calls);
            }
            Stmt::Block(block) => {
                calls.extend(self.collect_calls(&block.stmts, known_fns));
            }
            Stmt::Unsafe { body, .. } => {
                calls.extend(self.collect_calls(&body.stmts, known_fns));
            }
            Stmt::Defer { body, .. } => {
                calls.extend(self.collect_calls(&body.stmts, known_fns));
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }

    fn collect_calls_from_expr(
        &self,
        expr: &Expr,
        known_fns: &HashMap<String, HashSet<String>>,
        calls: &mut HashSet<String>,
    ) {
        match expr {
            Expr::Call { callee, args, .. } => {
                // Check if callee is a simple identifier (function name)
                if let Expr::Ident { name, .. } = callee.as_ref() {
                    if known_fns.contains_key(name) {
                        calls.insert(name.clone());
                    }
                }
                // Recurse into callee and arguments
                self.collect_calls_from_expr(callee, known_fns, calls);
                for arg in args {
                    self.collect_calls_from_expr(arg, known_fns, calls);
                }
            }
            Expr::Binary { lhs, rhs, .. } => {
                self.collect_calls_from_expr(lhs, known_fns, calls);
                self.collect_calls_from_expr(rhs, known_fns, calls);
            }
            Expr::Unary { operand, .. } => {
                self.collect_calls_from_expr(operand, known_fns, calls);
            }
            Expr::Paren { inner, .. } => {
                self.collect_calls_from_expr(inner, known_fns, calls);
            }
            Expr::Field { base, .. } => {
                self.collect_calls_from_expr(base, known_fns, calls);
            }
            Expr::Addr { operand, .. } | Expr::Deref { operand, .. } => {
                self.collect_calls_from_expr(operand, known_fns, calls);
            }
            Expr::At { base, index, .. } => {
                self.collect_calls_from_expr(base, known_fns, calls);
                self.collect_calls_from_expr(index, known_fns, calls);
            }
            Expr::Cast { expr, .. } => {
                self.collect_calls_from_expr(expr, known_fns, calls);
            }
            Expr::Some { value, .. } | Expr::Ok { value, .. } | Expr::Err { value, .. } => {
                self.collect_calls_from_expr(value, known_fns, calls);
            }
            Expr::StructLit { fields, .. } => {
                for field in fields {
                    self.collect_calls_from_expr(&field.value, known_fns, calls);
                }
            }
            // Leaf nodes
            Expr::IntLit { .. }
            | Expr::FloatLit { .. }
            | Expr::BoolLit { .. }
            | Expr::Ident { .. }
            | Expr::CStr { .. }
            | Expr::Bytes { .. }
            | Expr::None { .. } => {}
        }
    }

    /// Find strongly connected components using Tarjan's algorithm
    /// Returns list of SCCs with more than one node (cycles)
    fn find_recursive_cycles(&self, graph: &HashMap<String, HashSet<String>>) -> Vec<Vec<String>> {
        let mut index_counter = 0;
        let mut stack = Vec::new();
        let mut lowlinks: HashMap<String, usize> = HashMap::new();
        let mut indices: HashMap<String, usize> = HashMap::new();
        let mut on_stack: HashSet<String> = HashSet::new();
        let mut sccs: Vec<Vec<String>> = Vec::new();

        fn strongconnect(
            v: &str,
            graph: &HashMap<String, HashSet<String>>,
            index_counter: &mut usize,
            stack: &mut Vec<String>,
            lowlinks: &mut HashMap<String, usize>,
            indices: &mut HashMap<String, usize>,
            on_stack: &mut HashSet<String>,
            sccs: &mut Vec<Vec<String>>,
        ) {
            indices.insert(v.to_string(), *index_counter);
            lowlinks.insert(v.to_string(), *index_counter);
            *index_counter += 1;
            stack.push(v.to_string());
            on_stack.insert(v.to_string());

            if let Some(successors) = graph.get(v) {
                for w in successors {
                    if !indices.contains_key(w) {
                        strongconnect(w, graph, index_counter, stack, lowlinks, indices, on_stack, sccs);
                        let low_v = *lowlinks.get(v).unwrap();
                        let low_w = *lowlinks.get(w).unwrap();
                        lowlinks.insert(v.to_string(), low_v.min(low_w));
                    } else if on_stack.contains(w) {
                        let low_v = *lowlinks.get(v).unwrap();
                        let idx_w = *indices.get(w).unwrap();
                        lowlinks.insert(v.to_string(), low_v.min(idx_w));
                    }
                }
            }

            let low_v = *lowlinks.get(v).unwrap();
            let idx_v = *indices.get(v).unwrap();
            if low_v == idx_v {
                let mut scc = Vec::new();
                loop {
                    let w = stack.pop().unwrap();
                    on_stack.remove(&w);
                    scc.push(w.clone());
                    if w == v {
                        break;
                    }
                }
                // Only report SCCs with cycles (self-loops or multiple nodes)
                if scc.len() > 1 {
                    sccs.push(scc);
                } else if scc.len() == 1 {
                    // Check for self-loop
                    let node = &scc[0];
                    if let Some(edges) = graph.get(node) {
                        if edges.contains(node) {
                            sccs.push(scc);
                        }
                    }
                }
            }
        }

        for node in graph.keys() {
            if !indices.contains_key(node) {
                strongconnect(
                    node,
                    graph,
                    &mut index_counter,
                    &mut stack,
                    &mut lowlinks,
                    &mut indices,
                    &mut on_stack,
                    &mut sccs,
                );
            }
        }

        sccs
    }

    /// Get the span for a function by name
    fn get_function_span(&self, file: &File, name: &str) -> Option<crate::lexer::Span> {
        for item in &file.items {
            if let Item::Fn(func) = item {
                if func.name == name {
                    return Some(func.span.clone());
                }
            }
        }
        None
    }
}

impl P10Rule for ControlFlowRule {
    fn rule_number(&self) -> u8 {
        1
    }

    fn name(&self) -> &'static str {
        "no-recursion"
    }

    fn description(&self) -> &'static str {
        "No goto, setjmp/longjmp, or direct/indirect recursion"
    }

    fn is_enabled(&self, config: &P10Config) -> bool {
        !config.allow_recursion && config.level == SafetyLevel::SafetyCritical
    }

    fn check_file(&self, file: &File, _config: &P10Config, _source: &str) -> Vec<P10Violation> {
        let mut violations = Vec::new();

        // Build call graph and find recursive cycles
        let call_graph = self.build_call_graph(file);
        let cycles = self.find_recursive_cycles(&call_graph);

        for cycle in cycles {
            // Report violation for each function in the cycle
            for func_name in &cycle {
                if let Some(span) = self.get_function_span(file, func_name) {
                    let cycle_str = cycle.join(" -> ");
                    violations.push(
                        P10Violation::new(
                            1,
                            format!("function '{}' is part of a recursive call cycle", func_name),
                            span,
                        )
                        .with_help("Power of 10 Rule 1 forbids recursion; use iteration instead")
                        .with_note(format!("Recursive cycle: {}", cycle_str)),
                    );
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Block, FnDecl, TypeExpr};

    fn make_simple_file(funcs: Vec<(&str, Vec<&str>)>) -> File {
        // Create a simple file with functions that call each other
        // This is a simplified test - in reality we'd need proper AST construction
        let items = funcs
            .into_iter()
            .map(|(name, _calls)| {
                Item::Fn(FnDecl {
                    is_unsafe: false,
                    name: name.to_string(),
                    params: vec![],
                    return_type: TypeExpr::Void,
                    body: Block {
                        stmts: vec![],
                        span: 0..10,
                    },
                    span: 0..20,
                })
            })
            .collect();

        File { items }
    }

    #[test]
    fn test_no_recursion_passes() {
        let file = make_simple_file(vec![("foo", vec![]), ("bar", vec![])]);
        let config = P10Config::safety_critical();
        let rule = ControlFlowRule;

        let violations = rule.check_file(&file, &config, "");
        assert!(violations.is_empty());
    }
}
