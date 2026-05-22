#!/usr/bin/env python3
"""check_cheatsheet.py — verify every code block in fc.md compiles.

The cheatsheet is a load-bearing artifact: LLMs follow it, and if it
lies they ship broken code. This script extracts every fenced
``fastc`` block from `cheatsheets/fc.md`, wraps each one (if needed)
into a minimal compilable program, and invokes `fastc compile`. If
any block fails, the script exits non-zero with the offending
snippet annotated.

Snippets that are intentionally NOT compilable (the ❌ entries in
the common-mistakes section) are skipped: lines tagged with ❌ are
expected to break, lines tagged with ✓ are expected to compile.

Usage:
    python3 check_cheatsheet.py
    python3 check_cheatsheet.py --verbose       # show every snippet's verdict
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent.parent
FASTC = REPO / "target" / "release" / "fastc"
RUNTIME = REPO / "runtime"
CHEATSHEET = BASE / "cheatsheets" / "fc.md"

# Match fenced fastc blocks. Only blocks explicitly tagged
# `fastc` or `fc` are verified — `text` / `bash` / no-tag fences
# are treated as prose and skipped.
BLOCK_RE = re.compile(r"```(?:fastc|fc)\n(.*?)```", re.DOTALL)


def is_snippet_negative(snippet: str) -> bool:
    """Heuristic: snippets in the common-mistakes section are
    bracketed by `❌`/`✓` lines. We treat any block whose first
    non-whitespace line starts with `❌` or contains `❌` markers
    as a "do not compile this" demo. Such blocks are skipped."""
    return "❌" in snippet


def is_complete_program(snippet: str) -> bool:
    """A snippet is complete if it defines `fn main`. Otherwise we
    wrap it inside a synthetic main to test syntax."""
    return bool(re.search(r"\bfn\s+main\b", snippet))


def wrap_snippet(snippet: str) -> str:
    """If the snippet isn't a complete program, drop it inside an
    ad-hoc main. Wrapping is only needed for fragments like the
    type-table examples; the canonical worked example wraps itself."""
    if is_complete_program(snippet):
        return snippet
    # Heuristic: if the snippet looks like a let or call expression,
    # put it in main. If it has `fn` definitions but no main, append
    # main returning 0.
    has_fn = "fn " in snippet
    if has_fn:
        return snippet + "\nfn main() -> i32 { return 0; }\n"
    body_indented = "\n".join("    " + line if line.strip() else line
                              for line in snippet.splitlines())
    return f"fn main() -> i32 {{\n{body_indented}\n    return 0;\n}}\n"


def try_compile(src_text: str, workdir: Path) -> tuple[bool, str]:
    src = workdir / "snippet.fc"
    src.write_text(src_text)
    out_c = workdir / "snippet.c"
    r = subprocess.run(
        [str(FASTC), "compile", str(src), "-o", str(out_c)],
        capture_output=True, text=True, timeout=30,
    )
    if r.returncode != 0:
        return False, r.stderr.strip() or r.stdout.strip()
    # Also test that the produced C compiles. If snippet is wrapped,
    # the resulting C might pull in runtime symbols we need to link.
    out_bin = workdir / "snippet"
    r2 = subprocess.run(
        ["cc", "-O2", f"-I{RUNTIME}", str(out_c), "-o", str(out_bin)],
        capture_output=True, text=True, timeout=30,
    )
    if r2.returncode != 0:
        return False, f"cc failed: {r2.stderr.strip()}"
    return True, ""


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    if not FASTC.exists():
        print(f"error: {FASTC} not found. run: cargo build --release -p fastc",
              file=sys.stderr)
        return 1

    text = CHEATSHEET.read_text()
    blocks = BLOCK_RE.findall(text)
    if not blocks:
        print("error: no fenced code blocks found in fc.md", file=sys.stderr)
        return 1

    failures: list[tuple[int, str, str]] = []
    checked = 0
    skipped_negative = 0

    with tempfile.TemporaryDirectory() as work_root:
        work_root = Path(work_root)
        for idx, snippet in enumerate(blocks):
            label = f"#{idx + 1}"
            if is_snippet_negative(snippet):
                if args.verbose:
                    first_line = snippet.strip().splitlines()[0][:60] if snippet.strip() else "<empty>"
                    print(f"  {label} ❌-negative skipped: {first_line}")
                skipped_negative += 1
                continue
            wrapped = wrap_snippet(snippet)
            wd = work_root / f"snippet_{idx:03d}"
            wd.mkdir(parents=True, exist_ok=True)
            ok, err = try_compile(wrapped, wd)
            checked += 1
            if ok:
                if args.verbose:
                    first_line = snippet.strip().splitlines()[0][:60] if snippet.strip() else "<empty>"
                    print(f"  {label} ✓ {first_line}")
            else:
                failures.append((idx + 1, snippet, err))
                first_line = snippet.strip().splitlines()[0][:60] if snippet.strip() else "<empty>"
                print(f"  {label} ✗ {first_line}")

    print(f"\nchecked {checked} snippets, skipped {skipped_negative} negative examples")
    if failures:
        print(f"\n{len(failures)} snippet(s) FAILED:")
        for idx, snippet, err in failures:
            print(f"\n--- snippet #{idx} ---")
            print(snippet.rstrip())
            print(f"--- error ---")
            print(err[:1000])
        return 1
    print("all snippets compile")
    return 0


if __name__ == "__main__":
    sys.exit(main())
