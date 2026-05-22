#!/usr/bin/env python3
"""Safety-wedge measurement.

For each cached response, compile it AND (if compile succeeds)
run it and compare output to a golden value. Reports per-cell:

    compile_rate   — % of trials that produced a compilable binary
    correct_rate   — % of trials whose binary ALSO produced the
                     expected output

The wedge hypothesis: fastC's compile_rate may be lower than C
(see measure_fix.py for that finding), but the gap between
compile_rate and correct_rate should be smaller — fastC's strict
checks catch at compile time what C would let slip into a binary
that runs but returns the wrong value.

T1 only for now — it has the most coverage and an unambiguous
golden output. T2 needs stdin, T3 needs a complex string match;
both can be added as the data lands.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

BASE = Path(__file__).resolve().parent
RUNTIME = BASE.parent.parent.parent / "runtime"
FASTC = BASE.parent.parent.parent / "target" / "release" / "fastc"

CODE_RE = re.compile(r"```[a-zA-Z0-9_+\-]*\n(.*?)```", re.DOTALL)


# Task -> (stdin if any, expected stdout substring). Substring
# match is deliberately lenient — some LLMs print "Result: 55"
# instead of "55", which we count as correct because the answer
# is right and only the formatting differs from what the prompt
# asked for. A stricter benchmark would penalize the formatting.
GOLDEN = {
    "T1": (None, "55"),
}


LANG_FILE_EXT = {
    "fastc": "fc", "c": "c", "rust": "rs", "zig": "zig", "go": "go",
}


def extract_code(text: str) -> str | None:
    m = CODE_RE.findall(text)
    return m[0] if m else (text.strip() or None)


def build_and_run(code: str, lang: str, stdin: str | None,
                  workdir: Path) -> tuple[bool, bool, str]:
    """Returns (compiled, ran_and_correct, stdout_or_err)."""
    ext = LANG_FILE_EXT[lang]
    src = workdir / f"prog.{ext}"
    src.write_text(code)
    out = workdir / "prog"

    # Compile step (per-language).
    if lang == "fastc":
        cout = workdir / "prog.c"
        r = subprocess.run(
            [str(FASTC), "compile", str(src), "-o", str(cout)],
            capture_output=True, text=True, timeout=30,
        )
        if r.returncode != 0:
            return False, False, r.stderr
        r = subprocess.run(
            ["cc", "-O2", f"-I{RUNTIME}", str(cout), "-o", str(out)],
            capture_output=True, text=True, timeout=30,
        )
        if r.returncode != 0:
            return False, False, r.stderr
    elif lang == "c":
        r = subprocess.run(
            ["gcc", "-O2", str(src), "-o", str(out)],
            capture_output=True, text=True, timeout=30,
        )
        if r.returncode != 0:
            return False, False, r.stderr
    elif lang == "rust":
        r = subprocess.run(
            ["rustc", "-O", str(src), "-o", str(out)],
            capture_output=True, text=True, timeout=60,
        )
        if r.returncode != 0:
            return False, False, r.stderr
    elif lang == "zig":
        r = subprocess.run(
            ["zig", "build-exe", "-O", "ReleaseFast", "-lc",
             "--name", "prog", str(src)],
            cwd=str(workdir), capture_output=True, text=True, timeout=60,
        )
        if r.returncode != 0:
            return False, False, r.stderr
    elif lang == "go":
        go_bin = "/opt/homebrew/Cellar/go/1.26.3/bin/go"
        gosrc = workdir / "main.go"
        gosrc.write_text(code)
        r = subprocess.run(
            [go_bin, "build", "-o", str(out), str(gosrc)],
            cwd=str(workdir), capture_output=True, text=True, timeout=60,
        )
        if r.returncode != 0:
            return False, False, r.stderr
    else:
        return False, False, f"unknown lang {lang}"

    # Run step.
    try:
        r = subprocess.run(
            [str(out)], input=stdin if stdin else "",
            capture_output=True, text=True, timeout=10,
        )
    except subprocess.TimeoutExpired:
        return True, False, "runtime timeout"

    return True, True, r.stdout  # `True, True` is provisional — we still check output below


def check_output(stdout: str, expected_substring: str) -> bool:
    return expected_substring in stdout


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tasks", nargs="*", default=["T1"])
    ap.add_argument("--langs", nargs="*", default=["fastc", "c", "rust", "zig", "go"])
    ap.add_argument("--llms", nargs="*", default=None)
    args = ap.parse_args()

    # Per-cell counters.
    cells: dict[tuple[str, str, str], dict[str, int]] = {}

    with tempfile.TemporaryDirectory() as work_root:
        work_root = Path(work_root)
        for task in args.tasks:
            if task not in GOLDEN:
                print(f"skipping {task}: no golden defined", file=sys.stderr)
                continue
            stdin, expected = GOLDEN[task]
            for lang in args.langs:
                cell_root = BASE / "responses" / task / lang
                if not cell_root.is_dir():
                    continue
                llms = sorted(d.name for d in cell_root.iterdir() if d.is_dir())
                for llm in llms:
                    if args.llms and llm not in args.llms:
                        continue
                    key = (task, lang, llm)
                    cells[key] = {"trials": 0, "compiled": 0, "correct": 0}
                    for resp_file in sorted((cell_root / llm).glob("*.txt")):
                        text = resp_file.read_text()
                        if text.startswith("# ERROR:"):
                            continue
                        code = extract_code(text)
                        if code is None:
                            continue
                        cells[key]["trials"] += 1
                        wd = work_root / f"{task}_{lang}_{llm}_{resp_file.stem}"
                        wd.mkdir(parents=True, exist_ok=True)
                        compiled, ran, output = build_and_run(
                            code, lang, stdin, wd,
                        )
                        if compiled:
                            cells[key]["compiled"] += 1
                        if compiled and ran and check_output(output, expected):
                            cells[key]["correct"] += 1

    # Print summary table.
    print(f"{'cell':<32} {'trials':>6} {'compiled':>8} {'correct':>7} {'safety_gap':>10}")
    print("-" * 70)
    for (task, lang, llm), c in sorted(cells.items()):
        t, comp, corr = c["trials"], c["compiled"], c["correct"]
        gap = (comp - corr) if t else 0
        cell_id = f"{task}/{lang}/{llm}"
        print(f"{cell_id:<32} {t:>6} {comp:>8} {corr:>7} {gap:>10}")
    print("-" * 70)
    print("safety_gap = compiled - correct: trials that compiled but produced wrong output")
    return 0


if __name__ == "__main__":
    sys.exit(main())
