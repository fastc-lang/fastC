//! J1 (v2.0 hardening): source-map `#line` directives.
//!
//! The lowered C should carry a `#line N "<file>"` preprocessor
//! directive immediately before every user-authored function so
//! gdb / lldb stack traces and breakpoints land on the originating
//! `.fc` source line.
//!
//! Tests:
//!   1. Each user fn gets a `#line` referencing the correct line.
//!   2. Synthetic (closure-lifted, generated drop / clone) fns
//!      have no source line and the emitter must not invent one.
//!   3. Paths with embedded `"` characters are escaped safely.

use fastc::P10Config;
use fastc::compile_with_p10;

fn compile(src: &str, file: &str) -> String {
    let (c, _h) =
        compile_with_p10(src, file, false, P10Config::standard()).expect("compile succeeded");
    c
}

#[test]
fn user_fn_gets_correct_source_line_directive() {
    // The `fn` keyword for `first` lives at line 3; the source-map
    // pass keys off the FnDecl span, so the emitted directive must
    // be `#line 3` (the parser reports 1-based lines).
    let src = "// line 1\n\
               // line 2\n\
               fn first() -> i32 { return 1; }\n\
               \n\
               fn second(x: i32) -> i32 { return (x + 1); }\n";
    let c = compile(src, "/tmp/source_map_user.fc");

    // The directive precedes the lowered fn def.
    assert!(
        c.contains("#line 3 \"/tmp/source_map_user.fc\"\nint32_t first(void) {"),
        "expected #line 3 immediately before `first`, got:\n{}",
        excerpt(&c, "first(void)")
    );
    assert!(
        c.contains("#line 5 \"/tmp/source_map_user.fc\"\nint32_t second(int32_t x) {"),
        "expected #line 5 immediately before `second`, got:\n{}",
        excerpt(&c, "second(int32_t x)")
    );
}

#[test]
fn main_wrapper_carries_line_for_the_user_main() {
    // The auto-generated `int main(int argc, char** argv)` wrapper
    // comes from the emit pass and has no source line; only the
    // user-authored `fn main` (renamed to `fc_user_main`) gets one.
    let src = "fn main() -> i32 { return 0; }\n";
    let c = compile(src, "/tmp/source_map_main.fc");
    assert!(
        c.contains("#line 1 \"/tmp/source_map_main.fc\"\nint32_t fc_user_main(void) {"),
        "expected #line 1 before fc_user_main"
    );
    // The generated `int main(int argc, char** argv)` wrapper sits
    // outside the source-map machinery and must not carry a directive
    // on the *immediately preceding* line. (Earlier in the file a
    // user-fn `#line` is fine.)
    let wrapper_idx = c.find("int main(int argc, char** argv)").expect("wrapper");
    let preceding_lines: Vec<&str> = c[..wrapper_idx].lines().rev().take(3).collect();
    assert!(
        !preceding_lines.iter().any(|l| l.starts_with("#line")),
        "synthetic main wrapper should not carry a `#line` directive on \
         the lines immediately preceding it; got:\n{}",
        preceding_lines
            .iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn source_map_passes_through_cc_without_error() {
    // The C compiler must accept the emitted `#line` directives.
    let src = "fn main() -> i32 { return 7; }\n";
    let c = compile(src, "/tmp/source_map_cc.fc");
    let tmp = std::env::temp_dir().join("fastc_source_map_cc");
    std::fs::create_dir_all(&tmp).unwrap();
    let c_path = tmp.join("out.c");
    let bin = tmp.join("out_bin");
    std::fs::write(&c_path, &c).unwrap();
    let runtime = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("runtime");
    let out = std::process::Command::new("cc")
        .arg(&c_path)
        .arg("-I")
        .arg(&runtime)
        .arg("-g")
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "cc rejected the emitted source maps:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let run = std::process::Command::new(&bin).output().expect("run");
    assert_eq!(run.status.code(), Some(7));
}

#[test]
fn per_statement_source_marks_inside_user_main() {
    // J2: each user statement inside a fn body gets its own
    // `#line N "<file>"` directive so debugger breakpoints land on
    // the right .fc line, not just the function boundary.
    let src =
        "fn main() -> i32 {\n    let a: i32 = 1;\n    let b: i32 = 2;\n    return (a + b);\n}\n";
    let c = compile(src, "/tmp/per_stmt.fc");
    // J1 fn-level directive (#line 1) is there, plus per-stmt
    // directives for lines 2, 3, 4.
    for line in [1, 2, 3, 4] {
        assert!(
            c.contains(&format!("#line {} \"/tmp/per_stmt.fc\"", line)),
            "expected `#line {}` directive in output:\n{}",
            line,
            c
        );
    }
}

#[test]
fn path_with_quote_is_escaped_safely() {
    // A file path containing a `"` would otherwise break the C
    // string literal. The escaper doubles the quote.
    let src = "fn id(x: i32) -> i32 { return x; }\n";
    let c = compile(src, "/tmp/source_map_with\"quote.fc");
    assert!(
        c.contains("#line 1 \"/tmp/source_map_with\\\"quote.fc\""),
        "quote in path must be escaped, got:\n{}",
        excerpt(&c, "int32_t id")
    );
}

fn excerpt(c: &str, needle: &str) -> String {
    if let Some(idx) = c.find(needle) {
        let start = idx.saturating_sub(120);
        let end = (idx + needle.len() + 120).min(c.len());
        c[start..end].to_string()
    } else {
        format!("(needle '{}' not found)", needle)
    }
}
