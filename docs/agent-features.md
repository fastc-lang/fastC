# Agent-First Features Specification

fastC is designed to be the best systems language for AI coding agents. This document specifies what "agent-friendly" means, the features that support it, and how we measure success.

Agent-first features are not a single milestone. They span four stages of the [roadmap](roadmap.md):

- **Stage 1.3** — Annotation grammar. Every function signature becomes a typed operating manual (`@mem`, `@panics`, `@purity`, `@complexity` + mandatory module headers). See [docs/annotations.md](annotations.md).
- **Stage 1.4** — Capability system. I/O permissions become typed function arguments; an agent reading a fastC signature sees the full I/O surface without reading the body. See [docs/capabilities.md](capabilities.md).
- **Stage 1.5** — Contracts (runtime tier). `@requires` / `@ensures` become compile-time obligations, lowered to runtime asserts. See [docs/contracts.md](contracts.md).
- **Stage 1.6** — The features in this document: `--output-format=json` everywhere, `fastc fix`, `fastc context`, `fastc diff`, `fastc explain`, the unified diagnostic stream, and the **`fastc-mcp` server** (see [docs/mcp.md](mcp.md)) that exposes everything above to Claude Code / Cursor / Codex over Model Context Protocol.

Together these stages turn the compiler from "produces text errors" into "exposes a typed protocol surface that an agent reasons against."

## Compiler Constraints as Tooling Feedback

Agent-first features are not bolted on — they are a direct consequence of fastC's compiler constraints. Every rule the compiler enforces is a rule that tooling can report on, auto-fix, and verify.

| Compiler Constraint | Feedback It Enables | Existing Infrastructure |
|---------------------|---------------------|------------------------|
| Unambiguous grammar | Parse errors have one location, one fix | Miette spans with `hint: Option<String>` |
| Explicit types on signatures | Type errors show expected vs. actual | `CompileError::TypeCheck` with spans |
| No implicit conversions | Every mismatch → `cast(T, expr)` fix-it | `.with_help()` on `CompileError` |
| `unsafe` block requirement | "Wrap in unsafe" fix-it with span | `safety_with_hint()` in `errors.rs` |
| P10 rules (001–010) | Structured violations with codes + help | `ViolationDetail { code, message, location, help, note }` |
| Deterministic C output | Diff-verifiable changes | Stable ordering in C emission |
| Bounds/null/overflow checks | Runtime traps at known locations | `fc_trap()` with source context |
| Mandatory annotations on `pub` functions (stage 1.3) | Signature is the operating manual | `manifest.json` build artifact |
| Capability-typed I/O (stage 1.4) | I/O surface visible in signatures | `caps.json` build artifact |
| Contract discharge (stage 1.5 / 2.1) | Proven pre/postconditions | `discharge.json` build artifact |

**What already ships (v0.6):**
- `fastc cert-report --format json` — structured P10 compliance output with `ComplianceReport`, `ViolationDetail`, and `SourceLocation`
- `fastc cert-report --format text` — human-readable output with Unicode formatting and pass/fail icons
- `CliReportFormat::Json | Compact | Text` — three output modes already implemented
- All `CompileError` variants carry `hint: Option<String>` for fix-it suggestions
- DO-178C / ISO 26262 certification metadata in compliance reports

The stage 1.6 work extends this from `cert-report` to all commands, from display-only hints to auto-applicable fixes via `fastc fix`, and — most importantly — from a CLI surface to a **native MCP protocol surface** via `fastc-mcp`. Agents no longer text-parse `fastc check` output; they query typed resources (see [docs/mcp.md](mcp.md)).

## Problem Statement

Current systems languages were designed for human programmers using text editors:

- **C** has ambiguous grammar (declaration vs. expression), implicit conversions, undefined behavior that silently produces wrong results, and error messages that require deep expertise to interpret.
- **Rust** has excellent error messages for humans but complex lifetime diagnostics that confuse agents. Macro-heavy code is difficult for agents to generate correctly. The trait system produces cascading errors that are hard to resolve automatically.
- **Zig** has good explicit syntax but `comptime` patterns require understanding evaluation phases. Error messages assume human context.

AI coding agents have fundamentally different needs from human programmers:

| Human need | Agent need |
|-----------|------------|
| Concise syntax (less typing) | Explicit syntax (less ambiguity) |
| Flexible style | Canonical style (one way to write it) |
| Rich error prose | Structured error data (JSON, spans, fix-its) |
| IDE integration | CLI-first tooling with machine-readable output |
| Manual debugging | Automated fix-iterate loops |

## What Makes a Language Agent-Friendly

### 1. Unambiguous Grammar

FastC's grammar has no context-dependent parsing. Every token sequence has exactly one parse. There is no "most vexing parse," no ambiguity between declarations and expressions, and no need for semantic information during parsing.

This means an agent can generate syntactically valid code from grammar rules alone, without needing a type checker in the loop.

### 2. Deterministic Output

`fastc fmt` produces byte-identical output for semantically identical input. `fastc build` produces byte-identical C for identical `.fc` input. This means agents can verify their changes by diffing output — if the C output didn't change, the semantics didn't change.

### 3. Structured Diagnostics

All compiler errors include:
- A unique error code (e.g., `E0042`)
- Source span (file, line, column, length)
- Human-readable message
- Machine-readable category (type error, syntax error, safety violation, etc.)
- Fix-it hints where applicable (the suggested replacement text and its span)

### 4. Single Canonical Style

`fastc fmt` enforces one style. There are no style options, no configuration, no debates. An agent generating FastC code can run `fastc fmt` and know the output matches project conventions.

### 5. Explicit Over Implicit

FastC requires explicit types on function signatures, explicit error handling (no exceptions), explicit unsafe blocks, and explicit ownership annotations. This reduces the amount of context an agent needs to generate correct code.

## Feature Specifications

### `--output-format=json`

All CLI commands support `--output-format=json` to produce machine-readable output.

> **Existing foundation:** `fastc cert-report` already supports `--format json|compact|text` using the `CliReportFormat` enum. The `ViolationDetail` struct already serializes `code`, `message`, `SourceLocation { line, column, offset, length }`, `help`, and `note` fields. The 1.6 work extends this pattern to `compile`, `check`, `fmt`, and `explain` commands, and unifies `CompileError` diagnostics with P10 violations, capability errors (stage 1.4), and contract violations (stage 1.5) into a single JSON stream — served over MCP via `fastc-mcp`.

**Diagnostics format:**

```json
{
  "diagnostics": [
    {
      "code": "E0042",
      "severity": "error",
      "message": "type mismatch: expected `i32`, found `bool`",
      "file": "src/main.fc",
      "span": {
        "start": {"line": 10, "column": 5},
        "end": {"line": 10, "column": 12}
      },
      "category": "type_error",
      "fixits": [
        {
          "message": "convert bool to i32",
          "span": {
            "start": {"line": 10, "column": 5},
            "end": {"line": 10, "column": 12}
          },
          "replacement": "if (x) { 1 } else { 0 }"
        }
      ]
    }
  ],
  "summary": {
    "errors": 1,
    "warnings": 0
  }
}
```

**Build output format:**

```json
{
  "success": true,
  "artifacts": [
    {"type": "c_source", "path": "build/main.c"},
    {"type": "c_header", "path": "build/main.h"},
    {"type": "binary", "path": "build/main"}
  ],
  "timings": {
    "parse_ms": 12,
    "typecheck_ms": 8,
    "lower_ms": 5,
    "cc_ms": 340
  }
}
```

### `fastc fix`

Auto-apply all fix-it hints from diagnostics.

> **Existing foundation:** The compiler already generates fix-it hints via `parse_with_hint()`, `resolve_with_hint()`, `type_error_with_hint()`, and `safety_with_hint()` helpers. P10 violations carry `.with_help()` and `.with_note()` suggestions. `fastc fix` promotes these from display-only hints to auto-applicable source transformations.

```
$ fastc check src/main.fc --output-format=json
# 3 errors with fix-its

$ fastc fix src/main.fc
Applied 3 fixes:
  src/main.fc:10:5  — wrapped expression in unsafe block
  src/main.fc:22:1  — added missing return type annotation
  src/main.fc:35:10 — replaced `==` with `.eq()` for struct comparison

$ fastc check src/main.fc
No errors.
```

**Behavior:**
- Only applies fixes that are unambiguous (one suggested replacement).
- When multiple fixes conflict (overlapping spans), applies none and reports the conflict.
- `--dry-run` flag shows what would be applied without modifying files.
- Exit code 0 if all fixes applied successfully, 1 if conflicts remain.

### `fastc context`

Dump the public API surface of a project in a compact format suitable for LLM context windows.

```
$ fastc context src/
```

**Output:**

```
// Project: my_project (v0.1.0)
// Modules: 3 files, 12 public items

mod math {
    fn add(a: i32, b: i32) -> i32;
    fn multiply(a: i32, b: i32) -> i32;
    struct Vector2 { x: f64, y: f64 }
}

mod io {
    fn read_file(path: ref(str)) -> res(String, IoError);
    fn write_file(path: ref(str), data: ref(str)) -> res(void, IoError);
    enum IoError { NotFound, PermissionDenied, Other }
}

mod main {
    fn main() -> i32;
}
```

**Behavior:**
- Shows only public function signatures, struct definitions, and enum variants.
- Omits function bodies.
- Includes doc comments (`///`) if present.
- `--include-private` flag shows all items.
- `--max-tokens=N` flag truncates output to fit a context window budget.

### `fastc diff`

Semantic diff between two versions of a file or project.

```
$ fastc diff src/main.fc~1 src/main.fc
```

**Output:**

```
Changed: fn process(data: slice(i32)) -> i32
  Parameter added: `len: i32`
  Return type changed: i32 -> res(i32, Error)

Added: struct Error { code: i32, message: arr(u8, 256) }

Removed: fn old_helper(x: i32) -> i32
```

**Behavior:**
- Compares at the AST level, not the text level.
- Ignores formatting changes (whitespace, comment movement).
- Reports semantic changes: added/removed/changed functions, types, fields.
- `--output-format=json` produces machine-readable diff.

### Inline Test Blocks

```fastc
fn add(a: i32, b: i32) -> i32 {
    return (a + b);
}

test {
    assert(add(2, 3) == 5);
    assert(add(-1, 1) == 0);
    assert(add(0, 0) == 0);
}
```

**Behavior:**
- `test { }` blocks are only compiled when running `fastc test`.
- Tests are co-located with the code they test — agents don't need to navigate to a separate test file.
- Test blocks have access to all items in the same module (including private items).
- `fastc test --filter=add` runs only tests in the scope of matching items.

## Agent Workflow: Check-Fix-Check Loop

The canonical agent workflow for FastC:

```
1. Agent generates .fc code
2. Run: fastc check --output-format=json
3. If errors with fix-its:
     Run: fastc fix
     Go to step 2
4. If errors without fix-its:
     Agent reads diagnostics JSON
     Agent modifies code based on error spans and messages
     Go to step 2
5. If no errors:
     Run: fastc build --output-format=json
     Done.
```

This loop converges because:
- Fix-its resolve the most common errors automatically.
- Structured diagnostics give agents precise information about remaining errors.
- Deterministic output means the same fix always produces the same result.
- No hidden state (no caches, no incremental compilation artifacts that affect behavior).

## Comparison: Agent Experience by Language

| Capability | C | Zig | Rust | FastC |
|-----------|---|-----|------|-------|
| Unambiguous grammar | No | Mostly | Mostly | Yes |
| Deterministic formatting | clang-format (configurable) | zig fmt | rustfmt | fastc fmt (zero config) |
| Structured error JSON | No (text only) | No | Partial (--error-format=json) | Yes (all commands) |
| Auto-fix command | No | No | cargo fix (limited) | fastc fix |
| Context dump | No | No | cargo doc (HTML) | fastc context (compact text) |
| Semantic diff | No | No | No | fastc diff |
| Inline tests | No | Yes (Zig tests) | No (separate mod test) | Yes (test blocks) |
| Single canonical style | No | Yes | Mostly (configurable) | Yes (zero config) |

## Measuring Agent Friendliness

We track these metrics to validate that FastC is genuinely more agent-friendly:

### Error Recovery Rate

- **Protocol**: Give an agent 50 broken programs with known fixes. Measure how many are fixed in one `check → fix → check` round-trip.
- **Target**: > 80% recovery rate (vs. ~40% for C, ~60% for Rust).

### Code Generation Accuracy

- **Protocol**: Give an agent 50 natural-language specifications. Measure how many generated programs compile and pass tests on first try.
- **Target**: > 70% first-try accuracy (vs. ~30% for C, ~50% for Rust).

### Diagnostic Parsability

- **Protocol**: Feed 100 compiler error messages to an LLM. Measure how often it correctly identifies the file, line, error category, and suggested fix.
- **Target**: > 95% parsability for JSON diagnostics.

### Round-Trip Consistency

- **Protocol**: `parse → format → parse → format` produces identical output.
- **Target**: 100% (already guaranteed by deterministic output).

These benchmarks are part of the [benchmarking infrastructure](benchmarking.md).
