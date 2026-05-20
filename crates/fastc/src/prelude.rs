//! The built-in prelude: trait declarations and impls injected into every
//! fastC compilation.
//!
//! Stage 1.0 slice 3 ships:
//!
//! - `trait Eq`  — equality (`fn eq(self, other) -> bool`).
//! - `trait Ord` — ordering (`fn less_than(self, other) -> bool`).
//! - `trait Copy` — marker trait (no methods).
//!
//! And implementations for every primitive type with sensible semantics:
//!
//! - Every primitive implements `Eq` and `Copy`.
//! - Numeric primitives (everything except `bool`) implement `Ord`.
//!
//! The prelude is delivered as a fastC source string that is parsed once at
//! driver entry; the parsed items are prepended to the user's `File`
//! before desugar so the rest of the pipeline sees no special cases.

/// The prelude source. Kept as a literal string so adding a built-in trait
/// is just a textual addition — no AST surgery required.
///
/// All numeric primitives get both `Eq` and `Ord`. `bool` gets `Eq` and
/// `Copy` only (booleans have no total order in fastC).
pub const PRELUDE_SRC: &str = r#"
// --- Built-in traits (stage 1.0 slice 3) ---

trait Eq {
    fn eq(self: ref(Self), other: ref(Self)) -> bool;
}

trait Ord {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool;
}

trait Copy {
}

// --- Primitive impls ---

impl Eq for i8 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for i8 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for i8 {}

impl Eq for i16 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for i16 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for i16 {}

impl Eq for i32 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for i32 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for i32 {}

impl Eq for i64 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for i64 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for i64 {}

impl Eq for u8 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for u8 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for u8 {}

impl Eq for u16 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for u16 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for u16 {}

impl Eq for u32 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for u32 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for u32 {}

impl Eq for u64 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for u64 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for u64 {}

impl Eq for f32 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for f32 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for f32 {}

impl Eq for f64 {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for f64 {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for f64 {}

impl Eq for bool {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Copy for bool {}

impl Eq for usize {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for usize {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for usize {}

impl Eq for isize {
    fn eq(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) == deref(other));
    }
}

impl Ord for isize {
    fn less_than(self: ref(Self), other: ref(Self)) -> bool {
        return (deref(self) < deref(other));
    }
}

impl Copy for isize {}
"#;

/// Parse the prelude into a `Vec<Item>` ready to be prepended to a user
/// file. Parse errors here are programmer bugs in this file — they panic
/// rather than surface as user diagnostics.
pub fn prelude_items() -> Vec<crate::ast::Item> {
    let file = crate::driver::parse(PRELUDE_SRC, "<prelude>")
        .expect("prelude must always parse — fix prelude.rs");
    file.items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Item;

    #[test]
    fn prelude_parses_into_items() {
        let items = prelude_items();
        // Three traits + 13 primitives × ~3 impls each. Sanity-check we get
        // a non-trivial number without pinning a brittle exact count.
        assert!(items.len() >= 30, "got {} items", items.len());
        // First three should be the trait declarations.
        let mut trait_names: Vec<String> = items
            .iter()
            .filter_map(|i| match i {
                Item::Trait(t) => Some(t.name.clone()),
                _ => None,
            })
            .collect();
        trait_names.sort();
        assert_eq!(trait_names, vec!["Copy", "Eq", "Ord"]);
    }

    #[test]
    fn prelude_has_i32_eq_impl() {
        let items = prelude_items();
        let found = items.iter().any(|i| match i {
            Item::Impl(b) => b.target == "i32" && b.trait_name.as_deref() == Some("Eq"),
            _ => false,
        });
        assert!(found, "expected impl Eq for i32 in prelude");
    }
}
