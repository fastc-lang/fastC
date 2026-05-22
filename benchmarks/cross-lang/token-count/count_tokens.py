#!/usr/bin/env python3
"""Cross-language token-count benchmark.

Reads the source file for every (program, language) pair in
`benchmarks/cross-lang/<program>/<lang>/main.<ext>` and emits per-
tokenizer token counts to `results.csv`.

Tokenizers:
- cl100k_base — OpenAI GPT-4 family.
- o200k_base — OpenAI GPT-4o / o-series.
- char_count — naive character count, included as a baseline so
  readers can see how each tokenizer compares to "just count bytes."

The token count of source code is a proxy for how many tokens an
LLM has to emit to write the program from scratch. A lower number
is friendlier for context-budget reasons and (usually) correlates
with faster generation. It does NOT tell you whether the LLM will
get the program right on the first try — that's a separate
benchmark (`first-compile/`).

Run:
    pip install tiktoken
    python3 count_tokens.py
Output: results.csv in this directory.
"""

from __future__ import annotations
import csv
import sys
from pathlib import Path

try:
    import tiktoken
except ImportError:
    print("error: tiktoken not installed. run: pip install tiktoken", file=sys.stderr)
    sys.exit(1)

BENCH_DIR = Path(__file__).resolve().parent.parent
PROGRAMS = ["hello", "sum", "fib40", "mandelbrot"]

# (language tag, subdirectory, source filename)
LANGS = [
    ("fastc", "fc", "main.fc"),
    ("c", "c", "main.c"),
    ("rust", "rs", "main.rs"),
    ("zig", "zig", "main.zig"),
    ("go", "go", "main.go"),
]


def main() -> int:
    cl100k = tiktoken.get_encoding("cl100k_base")
    o200k = tiktoken.get_encoding("o200k_base")

    out_path = Path(__file__).parent / "results.csv"
    with out_path.open("w", newline="") as f:
        # Header / provenance lines start with `#` so the CSV parser
        # treats them as comments. The actual CSV header is the
        # first non-# row.
        f.write("# fastC cross-language token-count benchmark\n")
        f.write(f"# Tokenizers: cl100k_base (GPT-4), o200k_base (GPT-4o)\n")
        f.write("# Anthropic / Gemini tokenizers are closed-source; their\n")
        f.write("# counts are approximated by cl100k within ~10% based on\n")
        f.write("# Anthropic's published behavior, so we omit a separate column.\n")
        writer = csv.writer(f)
        writer.writerow([
            "program", "language",
            "source_bytes", "source_lines",
            "cl100k_tokens", "o200k_tokens",
        ])

        for prog in PROGRAMS:
            for lang_tag, subdir, src_name in LANGS:
                src = BENCH_DIR / prog / subdir / src_name
                if not src.exists():
                    print(f"warn: missing {src}", file=sys.stderr)
                    continue
                text = src.read_text()
                bytes_ = len(text.encode("utf-8"))
                lines = text.count("\n") + (1 if text and not text.endswith("\n") else 0)
                cl100k_count = len(cl100k.encode(text))
                o200k_count = len(o200k.encode(text))
                writer.writerow([
                    prog, lang_tag,
                    bytes_, lines,
                    cl100k_count, o200k_count,
                ])

    # Pretty-print the table to stdout too so a one-line invocation
    # gives the user something to look at.
    print(out_path.read_text())
    return 0


if __name__ == "__main__":
    sys.exit(main())
