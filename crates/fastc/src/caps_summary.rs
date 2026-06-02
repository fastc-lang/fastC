//! Stage 1.4 — `caps.json` per-build artifact.
//!
//! Walks every function in the compilation unit and records which
//! capability tokens it accepts as parameters. The result is a
//! flat list that agents (and `fastc-mcp` consumers) can read to
//! answer "what can this function actually do?" without parsing
//! the source.
//!
//! ## What's a capability parameter
//!
//! Anything shaped `ref(CapX)` or `mref(CapX)` where `CapX` is in
//! the sealed-caps set from `cap_check.rs`. A function with a
//! `ref(CapFsRead)` parameter can — and only can — call functions
//! that take `ref(CapFsRead)` somewhere downstream. The structural
//! property (fabrication forbidden outside `mod caps`, `caps::init`
//! main-only) means every cap an end user sees was minted in `main`
//! and threaded explicitly to the call site.
//!
//! ## Shape of the artifact
//!
//! ```json
//! {
//!   "schema": "fastc.caps.v1",
//!   "functions": [
//!     { "name": "fs_check", "caps": ["CapFsRead"] },
//!     { "name": "main", "caps": [] }
//!   ],
//!   "summary": {
//!     "total_functions": 12,
//!     "capability_using": 4,
//!     "capabilities_seen": ["CapFsRead", "CapNetConnect"]
//!   }
//! }
//! ```
//!
//! Aggregate `capabilities_seen` is the headline number for an
//! agent: "this program reaches the network and the read-side of the
//! filesystem; it does not write files, spawn processes, or listen
//! on ports."

use std::collections::BTreeSet;

use crate::ast::{File, FnDecl, Item, TypeExpr};

/// The fixed set of capability struct names the language recognizes.
/// Mirrors `cap_check::SEALED_CAPS` minus the `Caps` bundle (which
/// isn't a per-function parameter shape).
const KNOWN_CAPS: &[&str] = &[
    "CapFsRead",
    "CapFsWrite",
    "CapNetConnect",
    "CapNetListen",
    "CapProcSpawn",
    "CapTimeRead",
    "CapRand",
    "CapEnvRead",
];

/// One function's cap surface.
#[derive(Debug, Clone)]
pub struct FnCapSurface {
    pub name: String,
    /// Capability struct names this function accepts as parameters,
    /// in declaration order with duplicates collapsed.
    pub caps: Vec<String>,
}

/// Whole-program cap surface.
#[derive(Debug, Clone, Default)]
pub struct CapsSummary {
    pub functions: Vec<FnCapSurface>,
}

impl CapsSummary {
    /// Walk a parsed `File` and collect every fn's cap surface.
    /// Recurses into `mod` bodies — every function reachable in the
    /// compilation unit shows up exactly once, mangled with its
    /// module path.
    pub fn from_file(file: &File) -> Self {
        let mut out = Self::default();
        collect_items(&file.items, &[], &mut out);
        out
    }

    pub fn total(&self) -> usize {
        self.functions.len()
    }

    pub fn capability_using(&self) -> usize {
        self.functions.iter().filter(|f| !f.caps.is_empty()).count()
    }

    pub fn capabilities_seen(&self) -> Vec<String> {
        let mut s: BTreeSet<String> = BTreeSet::new();
        for f in &self.functions {
            for c in &f.caps {
                s.insert(c.clone());
            }
        }
        s.into_iter().collect()
    }

    /// Serialize to the canonical `caps.json` shape documented at
    /// the top of this module.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str("  \"schema\": \"fastc.caps.v1\",\n");
        out.push_str("  \"functions\": [\n");
        for (i, f) in self.functions.iter().enumerate() {
            out.push_str("    {");
            out.push_str(&format!("\"name\": \"{}\"", json_escape(&f.name)));
            out.push_str(", \"caps\": [");
            for (j, c) in f.caps.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("\"{}\"", json_escape(c)));
            }
            out.push_str("]}");
            if i + 1 < self.functions.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ],\n");
        out.push_str("  \"summary\": {\n");
        out.push_str(&format!("    \"total_functions\": {},\n", self.total()));
        out.push_str(&format!(
            "    \"capability_using\": {},\n",
            self.capability_using()
        ));
        out.push_str("    \"capabilities_seen\": [");
        let seen = self.capabilities_seen();
        for (i, c) in seen.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("\"{}\"", json_escape(c)));
        }
        out.push_str("]\n");
        out.push_str("  }\n");
        out.push_str("}\n");
        out
    }
}

fn collect_items(items: &[Item], module_path: &[String], out: &mut CapsSummary) {
    for item in items {
        match item {
            Item::Fn(f) => out.functions.push(extract(f, module_path)),
            Item::Mod(m) => {
                if let Some(body) = &m.body {
                    let mut path = module_path.to_vec();
                    path.push(m.name.clone());
                    collect_items(body, &path, out);
                }
            }
            // Impl methods are interesting too but they're already
            // desugared to free `Type_method` functions by the time
            // the discharge / caps surfaces run. Skipping here
            // avoids double-counting.
            _ => {}
        }
    }
}

fn extract(f: &FnDecl, module_path: &[String]) -> FnCapSurface {
    let mut caps: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for p in &f.params {
        if let Some(c) = cap_name_in(&p.ty) {
            if seen.insert(c.clone()) {
                caps.push(c);
            }
        }
    }
    let name = if module_path.is_empty() {
        f.name.clone()
    } else {
        format!("{}::{}", module_path.join("::"), f.name)
    };
    FnCapSurface { name, caps }
}

/// Return the underlying `Cap*` struct name if `ty` is `ref(CapX)` or
/// `mref(CapX)` (or one nested through a single level of paren-like
/// wrapping). Everything else is `None`.
fn cap_name_in(ty: &TypeExpr) -> Option<String> {
    let inner = match ty {
        TypeExpr::Ref(inner) | TypeExpr::Mref(inner) => inner.as_ref(),
        _ => return None,
    };
    let name = match inner {
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::NamedGeneric(n, _) => n.clone(),
        _ => return None,
    };
    if KNOWN_CAPS.iter().any(|c| *c == name.as_str()) {
        Some(name)
    } else {
        None
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn empty_file_summary() {
        let f = parse("", "test.fc").expect("parse");
        let s = CapsSummary::from_file(&f);
        assert_eq!(s.total(), 0);
        assert_eq!(s.capability_using(), 0);
        assert!(s.capabilities_seen().is_empty());
    }

    #[test]
    fn function_with_no_caps_lists_zero() {
        let f = parse("fn id(x: i32) -> i32 { return x; }", "test.fc").expect("parse");
        let s = CapsSummary::from_file(&f);
        assert_eq!(s.total(), 1);
        assert_eq!(s.capability_using(), 0);
        assert!(s.functions[0].caps.is_empty());
    }

    #[test]
    fn function_with_capfsread_param_is_captured() {
        let src = r#"
            fn read_file(c: ref(CapFsRead), path: raw(u8)) -> i32 {
                return 0;
            }
        "#;
        let f = parse(src, "test.fc").expect("parse");
        let s = CapsSummary::from_file(&f);
        assert_eq!(s.capability_using(), 1);
        assert_eq!(s.functions[0].caps, vec!["CapFsRead".to_string()]);
        assert_eq!(s.capabilities_seen(), vec!["CapFsRead".to_string()]);
    }

    #[test]
    fn caps_dedupe_per_function() {
        let src = r#"
            fn dual(a: ref(CapFsRead), b: ref(CapFsRead)) -> i32 {
                return 0;
            }
        "#;
        let f = parse(src, "test.fc").expect("parse");
        let s = CapsSummary::from_file(&f);
        assert_eq!(s.functions[0].caps.len(), 1, "duplicate caps collapse");
    }

    #[test]
    fn json_contains_schema_and_summary() {
        let src = r#"
            fn net_call(c: ref(CapNetConnect)) -> i32 { return 0; }
            fn pure_fn(x: i32) -> i32 { return x; }
        "#;
        let f = parse(src, "test.fc").expect("parse");
        let s = CapsSummary::from_file(&f);
        let json = s.to_json();
        assert!(json.contains("\"schema\": \"fastc.caps.v1\""));
        assert!(json.contains("\"total_functions\": 2"));
        assert!(json.contains("\"capability_using\": 1"));
        assert!(json.contains("\"CapNetConnect\""));
    }

    #[test]
    fn unknown_struct_in_ref_is_not_a_cap() {
        let src = r#"
            struct Whatever {}
            fn take(_x: ref(Whatever)) -> i32 { return 0; }
        "#;
        let f = parse(src, "test.fc").expect("parse");
        let s = CapsSummary::from_file(&f);
        assert!(s.functions[0].caps.is_empty(), "only sealed caps count");
    }

    #[test]
    fn nested_mod_function_is_qualified() {
        let src = r#"
            mod inner {
                pub fn hi(c: ref(CapTimeRead)) -> i32 { return 0; }
            }
        "#;
        let f = parse(src, "test.fc").expect("parse");
        let s = CapsSummary::from_file(&f);
        let hi = s
            .functions
            .iter()
            .find(|f| f.name.ends_with("hi"))
            .expect("found");
        assert_eq!(hi.name, "inner::hi");
        assert_eq!(hi.caps, vec!["CapTimeRead".to_string()]);
    }
}
