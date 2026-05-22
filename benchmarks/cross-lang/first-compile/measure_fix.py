#!/usr/bin/env python3
"""Measure fastc_fix.py's rescue rate.

For each cached fastC response, apply fastc_fix and report
whether the original compiles, the fixed version compiles, and
which transforms fired. Outputs a small table on stdout.

Usage:
    python3 measure_fix.py
    python3 measure_fix.py --tasks T1            # T1 only
    python3 measure_fix.py --llms glm kimi       # subset of models
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

BASE = Path(__file__).resolve().parent
FIX = BASE / "fastc_fix.py"
RUNTIME = BASE.parent.parent.parent / "runtime"
FASTC = BASE.parent.parent.parent / "target" / "release" / "fastc"

CODE_RE = re.compile(r"```[a-zA-Z0-9_+\-]*\n(.*?)```", re.DOTALL)


def extract_code(text: str) -> str | None:
    matches = CODE_RE.findall(text)
    return matches[0] if matches else (text.strip() or None)


def try_compile(src_text: str, workdir: Path) -> tuple[bool, str]:
    src = workdir / "prog.fc"
    src.write_text(src_text)
    cout = workdir / "prog.c"
    r1 = subprocess.run(
        [str(FASTC), "compile", str(src), "-o", str(cout)],
        capture_output=True, text=True, timeout=30,
    )
    if r1.returncode != 0:
        return False, r1.stderr or r1.stdout
    r2 = subprocess.run(
        ["cc", "-O2", f"-I{RUNTIME}", str(cout), "-o", str(workdir / "prog")],
        capture_output=True, text=True, timeout=30,
    )
    return r2.returncode == 0, r2.stderr


def apply_fix(src_text: str) -> tuple[str, str]:
    r = subprocess.run(
        [sys.executable, str(FIX), "--report"],
        input=src_text, capture_output=True, text=True, timeout=10,
    )
    return r.stdout, r.stderr


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tasks", nargs="*", default=["T1", "T2", "T3"])
    ap.add_argument("--llms", nargs="*", default=None)
    ap.add_argument("--lang", default="fastc")
    args = ap.parse_args()

    print(f"{'cell':<32} {'raw':<5} {'fixed':<5} transforms")
    print("-" * 78)

    raw_pass = 0
    fixed_pass = 0
    total = 0
    rescued = 0

    with tempfile.TemporaryDirectory() as work_root:
        work_root = Path(work_root)
        for task in args.tasks:
            cell_root = BASE / "responses" / task / args.lang
            if not cell_root.is_dir():
                continue
            llms = sorted(d.name for d in cell_root.iterdir() if d.is_dir())
            for llm in llms:
                if args.llms and llm not in args.llms:
                    continue
                for resp_file in sorted((cell_root / llm).glob("*.txt")):
                    text = resp_file.read_text()
                    if text.startswith("# ERROR:"):
                        continue
                    code = extract_code(text)
                    if code is None:
                        continue
                    total += 1
                    cell_id = f"{task}/{args.lang}/{llm}/{resp_file.stem}"

                    wd_raw = work_root / cell_id.replace("/", "_") / "raw"
                    wd_raw.mkdir(parents=True, exist_ok=True)
                    raw_ok, _ = try_compile(code, wd_raw)
                    if raw_ok:
                        raw_pass += 1

                    fixed_code, fix_report = apply_fix(code)
                    transforms = [
                        ln.strip().replace("applied ", "")
                        for ln in fix_report.splitlines()
                        if "applied" in ln
                    ]

                    wd_fix = work_root / cell_id.replace("/", "_") / "fixed"
                    wd_fix.mkdir(parents=True, exist_ok=True)
                    fixed_ok, _ = try_compile(fixed_code, wd_fix)
                    if fixed_ok:
                        fixed_pass += 1
                        if not raw_ok:
                            rescued += 1

                    raw_mark = "✓" if raw_ok else "✗"
                    fix_mark = "✓" if fixed_ok else "✗"
                    print(f"{cell_id:<32} {raw_mark:<5} {fix_mark:<5} {', '.join(transforms) or '-'}")

    print("-" * 78)
    print(f"total: {total}")
    print(f"raw pass:   {raw_pass}/{total} ({100*raw_pass/total if total else 0:.0f}%)")
    print(f"fixed pass: {fixed_pass}/{total} ({100*fixed_pass/total if total else 0:.0f}%)")
    print(f"rescued:    {rescued}/{total} (raw-fail then fixed-pass)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
