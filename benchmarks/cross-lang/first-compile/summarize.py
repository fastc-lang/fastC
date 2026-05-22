#!/usr/bin/env python3
"""Summarize cached first-compile responses to a results.csv.

Walks responses/<task>/<lang>/<llm>/<NN>.txt, runs each response
through the same code-extraction + compile pipeline run.py uses,
and writes per-cell aggregates. Reads only what's on disk — no LLM
API calls.

Useful when:
- A long run.py invocation hasn't finished yet but you want to see
  the partial picture.
- You changed the cheat sheet or the task description and want to
  re-evaluate without re-prompting.

Output:
- results.csv (overwrites in place; same shape as run.py emits).

Usage:
    python3 summarize.py
    python3 summarize.py --tasks T1 T2     # subset
"""

from __future__ import annotations

import argparse
import csv
import subprocess
import sys
import time
from pathlib import Path

# Reuse run.py's helpers via direct import so the compile logic
# stays in one place.
sys.path.insert(0, str(Path(__file__).resolve().parent))
import run as runpy   # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tasks", nargs="*", default=runpy.TASKS)
    ap.add_argument("--langs", nargs="*", default=runpy.LANGS)
    ap.add_argument("--llms", nargs="*", default=None,
                    help="default: every LLM with at least one response on disk")
    args = ap.parse_args()

    resp_root = runpy.BASE / "responses"

    if args.llms is None:
        # Discover from disk.
        llms = set()
        for p in resp_root.glob("*/*/*/*.txt"):
            llms.add(p.parent.name)
        args.llms = sorted(llms)

    if not runpy.FASTC.exists():
        print(f"error: {runpy.FASTC} not found", file=sys.stderr)
        return 1

    work_root = Path("/tmp") / f"summarize_{int(time.time())}"
    work_root.mkdir(parents=True, exist_ok=True)

    rows = []
    for task in args.tasks:
        for lang in args.langs:
            for llm in args.llms:
                cell_dir = resp_root / task / lang / llm
                if not cell_dir.is_dir():
                    continue
                trials = 0
                passes = 0
                for resp_file in sorted(cell_dir.glob("*.txt")):
                    text = resp_file.read_text()
                    if text.startswith("# ERROR:"):
                        continue
                    code = runpy.extract_code(text, lang)
                    if code is None:
                        trials += 1
                        continue
                    work = work_root / task / lang / llm / resp_file.stem
                    work.mkdir(parents=True, exist_ok=True)
                    ok, _err = runpy.try_compile(code, lang, work)
                    trials += 1
                    if ok:
                        passes += 1
                rate = passes / trials if trials else 0.0
                rows.append((task, lang, llm, trials, passes, rate))

    out_csv = runpy.BASE / "results.csv"
    with out_csv.open("w", newline="") as f:
        f.write("# fastC first-compile-success-rate benchmark\n")
        f.write(f"# Generated: {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())} (via summarize.py)\n")
        f.write(f"# LLMs included: {', '.join(args.llms)}\n")
        w = csv.writer(f)
        w.writerow(["task", "language", "llm", "trials", "passes", "pass_rate"])
        for r in rows:
            w.writerow([r[0], r[1], r[2], r[3], r[4], f"{r[5]:.2f}"])

    print(f"wrote {out_csv} ({len(rows)} cells)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
