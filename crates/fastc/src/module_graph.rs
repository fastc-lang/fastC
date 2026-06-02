//! Stage 1.3 module-graph validation.
//!
//! Validates `//! @module / @owns / @arch / @depends / @threading /
//! @invariants` headers attached to inline `mod foo { ... }` blocks.
//!
//! Two enforcement modes:
//!
//! - **Lenient** (default for back-compat): a `mod` block without a
//!   header parses and compiles. A `mod` block with even one `//!`
//!   line must have the *complete* required-key set; partial headers
//!   are rejected.
//!
//! - **Strict** (opt-in via `[package] strict_modules = true` in
//!   `fastc.toml`, default ON for new projects scaffolded by
//!   `fastc new`): every inline `mod` block MUST have a complete
//!   header.
//!
//! Cross-module checks (run in both modes when at least one module
//! has a header):
//!
//! - `@owns` namespaces must be globally unique. Two modules
//!   claiming `@owns = "logging"` is a compile error.
//! - `@depends` must be exhaustive: every `use mod::X` from a module
//!   whose header lists `@depends` must point at a module in that
//!   list.
//! - `@arch` layering forms a DAG. A module whose `@arch` ranks
//!   "lower" can't depend on a higher-arch module. Ordering is
//!   user-specified via the order modules appear in `@arch` layer
//!   declarations (default order: top-of-file first → "lowest"
//!   architectural layer).

use crate::ast::{File, Item, ModDecl, ModuleHeader};
use crate::diag::CompileError;
use std::collections::{HashMap, HashSet};

/// Validate every `mod foo { ... }` declaration in the file.
pub fn validate(file: &File, source: &str, strict: bool) -> Result<(), CompileError> {
    let mut all_mods: Vec<(Vec<String>, ModDecl)> = Vec::new();
    collect_mods(&file.items, &[], &mut all_mods);

    let mut errors: Vec<CompileError> = Vec::new();

    // Per-module header completeness check.
    for (_path, m) in &all_mods {
        match &m.header {
            None => {
                if strict {
                    errors.push(CompileError::resolve(
                        format!(
                            "module '{}' is missing the mandatory `//!` header. \
                             Add `//! @module / @owns / @arch / @depends / @threading \
                             / @invariants` at the top of the body. Disable with \
                             `strict_modules = false` in `fastc.toml`.",
                            m.name
                        ),
                        m.span.clone(),
                        source,
                    ));
                }
            }
            Some(h) => {
                for missing in missing_keys(h) {
                    errors.push(CompileError::resolve(
                        format!(
                            "module '{}' has a partial header (missing {}). \
                             A `//!` block must declare every required key.",
                            m.name, missing
                        ),
                        m.span.clone(),
                        source,
                    ));
                }
            }
        }
    }

    // Cross-module checks only fire when at least one module declared
    // a header (otherwise it's all legacy code with nothing to enforce).
    let any_header = all_mods.iter().any(|(_, m)| m.header.is_some());
    if !any_header && !strict {
        return finalize(errors);
    }

    // @owns uniqueness across all modules.
    let mut owns_to_module: HashMap<String, String> = HashMap::new();
    for (_path, m) in &all_mods {
        let Some(h) = &m.header else { continue };
        for owned in &h.owns {
            if let Some(prev) = owns_to_module.get(owned) {
                errors.push(CompileError::resolve(
                    format!(
                        "@owns namespace '{}' is claimed by both '{}' and '{}'. \
                         Each namespace must have exactly one owner module.",
                        owned, prev, m.name
                    ),
                    m.span.clone(),
                    source,
                ));
            } else {
                owns_to_module.insert(owned.clone(), m.name.clone());
            }
        }
    }

    // @depends exhaustiveness: every `use mod::X` from a module body
    // must point at a module in the declared depends list (or be the
    // module itself).
    for (_path, m) in &all_mods {
        let Some(h) = &m.header else { continue };
        let declared: HashSet<&str> = h.depends.iter().map(|s| s.as_str()).collect();
        let Some(body) = &m.body else { continue };
        let used = used_modules(body);
        for u in used {
            if u == m.name || u.starts_with("std::") || u.starts_with("core::") {
                continue;
            }
            if !declared.contains(u.as_str()) {
                errors.push(CompileError::resolve(
                    format!(
                        "module '{}' uses '{}' but does not declare it in @depends. \
                         Add '{}' to the @depends list at the top of the module.",
                        m.name, u, u
                    ),
                    m.span.clone(),
                    source,
                ));
            }
        }
    }

    // @arch DAG check: if module A has @arch = "lower" and module B
    // has @arch = "higher", A.depends cannot include B. The layer
    // ordering is given by the order modules first appear with a
    // distinct @arch value.
    let arch_order = build_arch_order(&all_mods);
    for (_path, m) in &all_mods {
        let Some(h) = &m.header else { continue };
        let Some(my_arch) = &h.arch else { continue };
        let Some(my_rank) = arch_order.get(my_arch) else {
            continue;
        };
        for dep in &h.depends {
            let dep_mod = all_mods.iter().find(|(_, mm)| &mm.name == dep);
            let Some((_, dep_decl)) = dep_mod else {
                continue;
            };
            let Some(dep_h) = &dep_decl.header else {
                continue;
            };
            let Some(dep_arch) = &dep_h.arch else {
                continue;
            };
            let Some(dep_rank) = arch_order.get(dep_arch) else {
                continue;
            };
            if dep_rank > my_rank {
                errors.push(CompileError::resolve(
                    format!(
                        "module '{}' (arch='{}') depends on '{}' (arch='{}'). \
                         Architecture layering is a DAG — lower layers cannot \
                         depend on higher layers.",
                        m.name, my_arch, dep, dep_arch
                    ),
                    m.span.clone(),
                    source,
                ));
            }
        }
    }

    finalize(errors)
}

fn finalize(errors: Vec<CompileError>) -> Result<(), CompileError> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileError::multiple(errors))
    }
}

fn missing_keys(h: &ModuleHeader) -> Vec<&'static str> {
    let mut out = Vec::new();
    if h.module_name.is_none() {
        out.push("@module");
    }
    if h.owns.is_empty() {
        out.push("@owns");
    }
    if h.arch.is_none() {
        out.push("@arch");
    }
    // @depends may legitimately be empty (a leaf module). We require
    // the *key* be present (raw_lines must contain a "@depends="
    // line) but allow an empty value.
    if !h.raw_lines.iter().any(|l| l.trim().starts_with("@depends")) {
        out.push("@depends");
    }
    if h.threading.is_none() {
        out.push("@threading");
    }
    if h.invariants.is_empty() {
        out.push("@invariants");
    }
    out
}

fn collect_mods(items: &[Item], path: &[String], out: &mut Vec<(Vec<String>, ModDecl)>) {
    for item in items {
        if let Item::Mod(m) = item {
            out.push((path.to_vec(), m.clone()));
            if let Some(body) = &m.body {
                let mut p = path.to_vec();
                p.push(m.name.clone());
                collect_mods(body, &p, out);
            }
        }
    }
}

/// Find every module name referenced in this module body via `use`.
fn used_modules(items: &[Item]) -> HashSet<String> {
    let mut out = HashSet::new();
    for item in items {
        if let Item::Use(u) = item {
            if let Some(first) = u.path.first() {
                out.insert(first.clone());
            }
        }
    }
    out
}

fn build_arch_order(all_mods: &[(Vec<String>, ModDecl)]) -> HashMap<String, usize> {
    let mut order: HashMap<String, usize> = HashMap::new();
    let mut next = 0usize;
    for (_, m) in all_mods {
        if let Some(h) = &m.header {
            if let Some(a) = &h.arch {
                if !order.contains_key(a) {
                    order.insert(a.clone(), next);
                    next += 1;
                }
            }
        }
    }
    order
}
