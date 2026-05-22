#!/usr/bin/env python3
"""fastc-fix prototype: auto-correct LLM-generated fastC.

Applies a small set of text-level transforms to convert common
LLM mistakes (Rust-style array types, Zig-style array types,
missing main return type, bracket indexing) into valid fastC.

This is a sidecar prototype. A production `fastc fix` subcommand
in the Rust binary would do these transforms at the parser level
with proper scope/type awareness. The prototype runs fast enough
to validate the rescue-rate hypothesis before investing in the
Rust implementation.

Transforms (each is a single-purpose regex):

  T1. Rust-style array type `[T; N]` → `arr(T, N)`
      Catches `fn f(a: [i32; 10])`, `let x: [u8; 4]`.

  T2. Zig-style array type `[N]T` → `arr(T, N)`
      Catches `let x: [10]i32`.

  T3. `fn main()` → `fn main() -> i32`
      Adds the i32 return type that fastC requires.

  T4. Bracket indexing `expr[i]` → `at(expr, i)`
      Only applies to indexing inside expression context, not
      type annotations (which T1/T2 already handle).

  T5. Force `return 0;` if `main` has been transformed to -> i32
      and the body ends without a return.
      Skipped in v1 — most models include return 0 already.

Usage:
    python3 fastc_fix.py < input.fc > output.fc
    python3 fastc_fix.py input.fc                # in-place to stdout
    python3 fastc_fix.py --report input.fc       # show what changed
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


# T1: Rust-style array type. `[T; N]` where T is an identifier or
# nested type, N is a positive integer. Captures the surrounding
# context to avoid touching slice / array literals.
#
# We only transform when the `[T; N]` follows `:` or `->`, i.e.
# in a type position. This avoids accidentally transforming
# expression-level slice notation that fastC might support.
#
# Side effect: if the binding being typed is literally named `arr`
# (which collides with the `arr(...)` type constructor), the
# transform also captures the surrounding `arr:` binding and we
# emit a follow-up rename of `arr` → `data` to avoid the
# self-reference. Handled in a second pass below.
RE_RUST_ARRAY_TYPE = re.compile(
    r"(:\s*|->\s*)\[(\w+)\s*;\s*(\d+)\]"
)


def transform_rust_array_type(src: str) -> tuple[str, int]:
    def repl(m: re.Match) -> str:
        prefix, ty, n = m.group(1), m.group(2), m.group(3)
        return f"{prefix}arr({ty}, {n})"
    new, n = RE_RUST_ARRAY_TYPE.subn(repl, src)
    return new, n


# T1b: when a parameter (or let-binding) named `arr` ends up
# typed as `arr(T, N)` after T1, fastC sees `arr: arr(T, N)`
# and the parser collides (parameter name shadows the type
# constructor). The fix is to rename every word-boundary `arr`
# in the source to `data`. Only fires if both the collision
# pattern is present.
# `arr` is a reserved keyword in fastC (the array type
# constructor). Any parameter, let-binding, or local named
# literally `arr` fails to parse. The most common LLM pattern is
# `fn sum_array(arr: SOMETHING)` because the task description
# says "array of i32". We detect `arr:` in any binding position
# and globally rename `arr` → `data`.
RE_ARR_AS_BINDING = re.compile(r"\barr\s*:")


def transform_rename_arr_param(src: str) -> tuple[str, int]:
    if not RE_ARR_AS_BINDING.search(src):
        return src, 0
    # Replace every standalone `arr` identifier with `data`.
    # Use `\b` word boundary; do NOT touch `arr(`, `arr_*`,
    # `_arr`, or method calls like `.arr_*`. The `(?!\()`
    # negative-lookahead skips `arr(` (the type constructor).
    new, n = re.subn(r"\barr\b(?!\()", "data", src)
    return new, n


# T2: Zig-style array type. `[N]T` where N is a positive integer
# literal, T is an identifier. Like T1, only in type position
# (after `:` or `->`).
RE_ZIG_ARRAY_TYPE = re.compile(
    r"(:\s*|->\s*)\[(\d+)\](\w+)"
)


def transform_zig_array_type(src: str) -> tuple[str, int]:
    def repl(m: re.Match) -> str:
        prefix, n, ty = m.group(1), m.group(2), m.group(3)
        return f"{prefix}arr({ty}, {n})"
    new, n = RE_ZIG_ARRAY_TYPE.subn(repl, src)
    return new, n


# T3: main with no return type. `fn main()` or `fn main(...)` not
# followed by `->`. The fastC return type is always i32.
RE_MAIN_NO_RET = re.compile(
    r"fn\s+main\s*\(([^)]*)\)\s*\{"
)


def transform_main_return(src: str) -> tuple[str, int]:
    def repl(m: re.Match) -> str:
        return f"fn main({m.group(1)}) -> i32 {{"
    new, n = RE_MAIN_NO_RET.subn(repl, src)
    return new, n


# T4: bracket indexing. `name[expr]` → `at(name, expr)`. By the
# time we run this, T1/T2 have already rewritten any type-position
# brackets — `[T; N]` and `[N]T` — but parametric types like
# `Vec[i32]`, `Opt[T]`, `Res[T, E]`, `HashMap[K, V]` still look
# the same as expression-level indexing. We use a capitalization
# heuristic: fastC types are conventionally capitalized
# (`Vec`, `Opt`, `Str`, `Cap*`) and variables are lowercase. The
# transform only fires when the identifier before `[` starts with
# a lowercase letter or underscore.
#
# The index expression can be anything from a literal to a nested
# function call (`data[cast(usize, i)]`). We match brackets with
# balanced parens inside by accepting any non-bracket characters
# plus one level of balanced `()` groups.
RE_INDEXING = re.compile(
    r"\b([a-z_]\w*)\[((?:[^\[\]]|\([^()]*\))+)\]"
)


def transform_indexing(src: str) -> tuple[str, int]:
    def repl(m: re.Match) -> str:
        ident, idx = m.group(1), m.group(2)
        # Skip if this looks like a leftover type annotation.
        if ";" in idx:
            return m.group(0)
        return f"at({ident}, {idx})"
    new, n = RE_INDEXING.subn(repl, src)
    return new, n


# T5: fastC's for-loop syntax requires parens around init/cond/step:
#   for (let i: i32 = 0; (i < N); i = (i + 1)) { ... }
# LLMs trained on C/Rust idioms drop the outer parens. Detect a
# `for` token NOT followed by `(`, capture up to the body brace,
# wrap.
RE_FOR_NO_PARENS = re.compile(
    r"\bfor\s+(?!\()([^{]+?)\s*\{",
    re.DOTALL,
)


def transform_for_parens(src: str) -> tuple[str, int]:
    def repl(m: re.Match) -> str:
        body = m.group(1).rstrip()
        return f"for ({body}) {{"
    new, n = RE_FOR_NO_PARENS.subn(repl, src)
    return new, n


# T6: hoist `use` statements from inside function bodies to the
# top of the file. fastC's `use` is top-level only; the parser
# rejects `use io::println;` inside `fn main() { ... }`.
# We also handle Rust-style `use io::{print_int, put_char}` by
# expanding it into separate `use` lines.
RE_USE_IN_FN = re.compile(r"^\s*use\s+([\w:]+(?:::\{[^}]+\})?)\s*;\s*$", re.MULTILINE)
RE_USE_GROUP = re.compile(r"^use\s+([\w:]+)::\{([^}]+)\}\s*;\s*$", re.MULTILINE)


def transform_hoist_use(src: str) -> tuple[str, int]:
    """Collect every `use X;` line that sits inside a function body
    (rough heuristic: line starts with whitespace + `use`), expand
    `use mod::{a, b, c}` into one `use mod::a` per item, prepend
    every collected line to the top of the file, and remove the
    originals from the body."""
    use_lines = []
    out_lines = []
    for line in src.splitlines(keepends=True):
        stripped = line.strip()
        if stripped.startswith("use ") and stripped.endswith(";"):
            use_lines.append(line.lstrip())
            continue
        out_lines.append(line)

    if not use_lines:
        return src, 0

    # Expand grouped uses.
    expanded = []
    for u in use_lines:
        m = RE_USE_GROUP.match(u.strip() + "\n")
        if m:
            mod, items = m.group(1), m.group(2)
            for it in (x.strip() for x in items.split(",")):
                if it:
                    expanded.append(f"use {mod}::{it};\n")
        else:
            expanded.append(u if u.endswith("\n") else u + "\n")

    # Dedupe while preserving order.
    seen = set()
    deduped = []
    for u in expanded:
        if u not in seen:
            seen.add(u)
            deduped.append(u)

    new = "".join(deduped) + "\n" + "".join(out_lines)
    return new, len(use_lines)


# T7: integer literal in let-binding with non-i32 annotated type.
# fastC infers literal `0` / `1` / etc as i32 by default and
# refuses to silently widen. The common pattern is
#   let total: i64 = 0;
# which produces a `expected i64, got i32` type mismatch. Wrap
# the literal in `cast(<annotated_type>, <literal>)`.
# We match: `let NAME: NONI32_INT = INT_LITERAL;`
RE_LET_INT_LITERAL = re.compile(
    r"(let\s+\w+\s*:\s*(i8|i16|i64|u8|u16|u32|u64|usize|isize|size_t|ptrdiff_t)\s*=\s*)(-?\d+)(\s*;)"
)


def transform_int_literal_cast(src: str) -> tuple[str, int]:
    def repl(m: re.Match) -> str:
        prefix, ty, lit, tail = m.group(1), m.group(2), m.group(3), m.group(4)
        return f"{prefix}cast({ty}, {lit}){tail}"
    new, n = RE_LET_INT_LITERAL.subn(repl, src)
    return new, n


TRANSFORMS = [
    ("rust_array_type", transform_rust_array_type),
    ("zig_array_type", transform_zig_array_type),
    ("rename_arr_param", transform_rename_arr_param),
    ("main_return", transform_main_return),
    ("for_parens", transform_for_parens),
    ("indexing", transform_indexing),
    ("hoist_use", transform_hoist_use),
    ("int_literal_cast", transform_int_literal_cast),
]


def apply_all(src: str) -> tuple[str, dict[str, int]]:
    counts: dict[str, int] = {}
    cur = src
    for name, fn in TRANSFORMS:
        cur, n = fn(cur)
        if n:
            counts[name] = n
    return cur, counts


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("input", nargs="?", help="input file (default: stdin)")
    ap.add_argument("--report", action="store_true",
                    help="report transform counts on stderr; emit fixed source on stdout")
    args = ap.parse_args()

    if args.input:
        src = Path(args.input).read_text()
    else:
        src = sys.stdin.read()

    fixed, counts = apply_all(src)
    sys.stdout.write(fixed)
    if args.report:
        if counts:
            for k, v in counts.items():
                print(f"  applied {k}: {v}", file=sys.stderr)
        else:
            print("  no transforms applied", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
