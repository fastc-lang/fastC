//! Structured fix-it suggestions.
//!
//! A `Fixit` carries the metadata needed to mechanically apply a
//! diagnostic's suggested fix: where in the source to edit (span),
//! what text to substitute (replacement), and a human-readable
//! label. The `fastc fix` subcommand collects fixits from a check
//! run and applies them in reverse-span order so earlier edits
//! don't shift later spans.
//!
//! ## How emitters register fixits
//!
//! Diagnostic emitters opt in via `registry::push(Fixit::new(...))`
//! alongside the existing `.with_help(...)` text. The registry is a
//! thread-local accumulator drained by `fastc fix` after the check
//! pipeline runs. This sidesteps an enum-wide refactor of
//! `CompileError`: the variants don't carry fixits as a field;
//! instead, the registry is the side-channel.
//!
//! ## How `fastc fix` consumes them
//!
//! 1. `registry::clear()` at the start of the run.
//! 2. Run `fastc check`.
//! 3. `registry::drain()` returns every Fixit pushed during the run.
//! 4. `apply_all` writes the rewritten source back to disk (or
//!    prints a unified diff under `--dry-run`).

use crate::lexer::Span;

/// A mechanical source-text edit that resolves a diagnostic.
#[derive(Debug, Clone)]
pub struct Fixit {
    /// Byte span in the source to replace.
    pub span: Span,
    /// The replacement text. May be empty (deletion).
    pub replacement: String,
    /// One-line label shown to the user in editor / CLI contexts.
    pub label: String,
}

impl Fixit {
    pub fn new(span: Span, replacement: impl Into<String>, label: impl Into<String>) -> Self {
        Fixit {
            span,
            replacement: replacement.into(),
            label: label.into(),
        }
    }
}

/// Apply a batch of fixits to source text. Sorts by span end in
/// descending order so applying an earlier edit doesn't shift the
/// indices of a later one. Returns the new source.
///
/// Overlapping fixits: the first wins (post-sort, that's the one
/// with the larger end). Callers should de-duplicate before
/// invoking if they want different semantics.
pub fn apply_all(source: &str, mut fixits: Vec<Fixit>) -> String {
    // Sort by end DESC, then start ASC. The end-DESC order means we
    // edit from the back of the source forward, so earlier edits
    // don't shift later spans. The start-ASC tiebreak means that
    // when two fixits share an end, the one covering the larger
    // span wins (smaller start = larger span = more inclusive fix).
    fixits.sort_by(|a, b| {
        b.span
            .end
            .cmp(&a.span.end)
            .then(a.span.start.cmp(&b.span.start))
    });
    let mut out = source.to_string();
    let mut last_start = usize::MAX;
    for fix in fixits {
        if fix.span.end > last_start {
            // Overlapping with a fix we already applied — skip.
            continue;
        }
        if fix.span.end > out.len() {
            continue;
        }
        out.replace_range(fix.span.start..fix.span.end, &fix.replacement);
        last_start = fix.span.start;
    }
    out
}

/// Thread-local fix-it registry.
///
/// Diagnostic emitters call `push()` to record a structured fix
/// alongside their text hint. `fastc fix` drains the registry after
/// the check pipeline and applies via `apply_all`. The registry is
/// per-thread because the compile pipeline is single-threaded per
/// file; multiple compile invocations on different threads each
/// have their own accumulator.
pub mod registry {
    use super::Fixit;
    use std::cell::RefCell;

    thread_local! {
        static FIXITS: RefCell<Vec<Fixit>> = const { RefCell::new(Vec::new()) };
    }

    /// Record a fix-it for the current compile pipeline.
    pub fn push(fix: Fixit) {
        FIXITS.with(|cell| cell.borrow_mut().push(fix));
    }

    /// Drain every recorded fix-it. The registry is empty after.
    pub fn drain() -> Vec<Fixit> {
        FIXITS.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
    }

    /// Clear the registry without returning the values. Call at the
    /// start of a fresh compile.
    pub fn clear() {
        FIXITS.with(|cell| cell.borrow_mut().clear());
    }

    /// Read the registry without consuming it. Used by tests and
    /// diagnostic introspection.
    pub fn len() -> usize {
        FIXITS.with(|cell| cell.borrow().len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_single_fixit() {
        let src = "fn foo() { return 0 }";
        let fix = Fixit::new(19..19, ";", "missing semicolon");
        let out = apply_all(src, vec![fix]);
        assert_eq!(out, "fn foo() { return 0; }");
    }

    #[test]
    fn applies_multiple_in_reverse_order() {
        let src = "fn a() { } fn b() { }";
        let fixes = vec![
            Fixit::new(0..2, "pub fn", "add pub"),
            Fixit::new(11..13, "pub fn", "add pub"),
        ];
        let out = apply_all(src, fixes);
        assert_eq!(out, "pub fn a() { } pub fn b() { }");
    }

    #[test]
    fn skips_overlapping_fixits() {
        let src = "let x: i32 = 5;";
        let fixes = vec![
            Fixit::new(4..10, "y: i64", "rename"),
            Fixit::new(7..10, "u32", "retype"), // overlaps with the first
        ];
        let out = apply_all(src, fixes);
        // First fix (larger span at same end) wins; overlap skipped.
        assert_eq!(out, "let y: i64 = 5;");
    }
}
