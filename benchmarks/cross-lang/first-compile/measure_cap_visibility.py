#!/usr/bin/env python3
"""measure_cap_visibility.py — score signature-side-effect visibility.

For each T6 (load_config) response, extract the `load_config`
function's *signature line* (not body) and score whether a
reviewer reading only that signature can see the function does
filesystem I/O.

The structural wedge fastC makes is that capability arguments
appear in the type signature. In C/Rust/Zig/Go, file I/O is
ambient — a function with the signature `int load_config(const
char *path)` could be reading a file, or just returning 42, and
a reviewer cannot tell from the signature alone.

A response scores SIGNATURE-VISIBLE if the function signature
contains a capability token (any of: `Cap`, `CapFsRead`,
`capability`, or a language-specific equivalent we add later).
For now, fastC is the only language whose signatures can
contain these tokens; the score for the four other languages
will be 0/N by construction unless an LLM volunteers a non-
standard annotation (which would be itself interesting).

Output:
- Per-cell counts: total trials, compile-passes, signature-
  visible count.
- Bar chart sketch in stdout for quick visual reading.

Usage:
    python3 measure_cap_visibility.py
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
import subprocess
from pathlib import Path

BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent.parent
FASTC = REPO / "target" / "release" / "fastc"
RUNTIME = REPO / "runtime"

CODE_RE = re.compile(r"```[a-zA-Z0-9_+\-]*\n(.*?)```", re.DOTALL)

# Capability-token patterns we look for in the signature line.
# fastC's canonical pattern is `c: ref(CapFsRead)` or any
# parameter typed `ref(Cap*)`. Other languages don't have an
# equivalent today; if a model invented one (e.g., a Rust trait
# bound `<Fs: FileRead>`) we'd want to count it, so the pattern
# is broader than just "CapFsRead".
CAP_TOKEN_RE = re.compile(
    r"\b(CapFsRead|CapFsWrite|CapNetConnect|CapNetListen|"
    r"CapProcSpawn|CapTimeRead|CapRand|CapEnvRead|Caps|"
    r"capability|Capability)\b"
)

LANG_FILE_EXT = {
    "fastc": "fc", "c": "c", "rust": "rs", "zig": "zig", "go": "go",
}


def extract_code(text: str) -> str | None:
    m = CODE_RE.findall(text)
    return m[0] if m else (text.strip() or None)


def extract_signature(code: str, lang: str) -> str | None:
    """Pull the first line(s) of the function whose name contains
    'load_config'. We greedily match from the `fn`/`int`/`func`
    keyword to the opening brace of the body so multi-line
    signatures get folded into a single string."""
    # Language-specific function header detection. We match the
    # function name as `load_config` literally.
    if lang in ("fastc", "rust", "zig"):
        # `fn load_config(...) -> ... {` or `fn load_config(...) ! ... {`
        m = re.search(r"\bfn\s+load_config\b[^{]*\{", code, re.DOTALL)
    elif lang == "go":
        m = re.search(r"\bfunc\s+load_config\b[^{]*\{", code, re.DOTALL)
    elif lang == "c":
        # `int load_config(...) {` or similar
        m = re.search(r"\b\w+\s+load_config\b[^{]*\{", code, re.DOTALL)
    else:
        return None
    if not m:
        return None
    return m.group(0).rsplit("{", 1)[0].strip()


def signature_visible(sig: str) -> bool:
    return bool(CAP_TOKEN_RE.search(sig))


def try_compile(code: str, lang: str, workdir: Path) -> bool:
    ext = LANG_FILE_EXT[lang]
    src = workdir / f"prog.{ext}"
    src.write_text(code)
    out = workdir / "prog"
    if lang == "fastc":
        cout = workdir / "prog.c"
        r = subprocess.run(
            [str(FASTC), "compile", str(src), "-o", str(cout)],
            capture_output=True, text=True, timeout=30,
        )
        if r.returncode != 0:
            return False
        r = subprocess.run(
            ["cc", "-O2", f"-I{RUNTIME}", str(cout), "-o", str(out)],
            capture_output=True, text=True, timeout=30,
        )
        return r.returncode == 0
    if lang == "c":
        r = subprocess.run(
            ["gcc", "-O2", str(src), "-o", str(out)],
            capture_output=True, text=True, timeout=30,
        )
        return r.returncode == 0
    if lang == "rust":
        r = subprocess.run(
            ["rustc", "-O", str(src), "-o", str(out)],
            capture_output=True, text=True, timeout=60,
        )
        return r.returncode == 0
    if lang == "zig":
        r = subprocess.run(
            ["zig", "build-exe", "-O", "ReleaseFast", "-lc",
             "--name", "prog", str(src)],
            cwd=str(workdir), capture_output=True, text=True, timeout=60,
        )
        return r.returncode == 0
    if lang == "go":
        go_bin = "/opt/homebrew/Cellar/go/1.26.3/bin/go"
        gosrc = workdir / "main.go"
        gosrc.write_text(code)
        r = subprocess.run(
            [go_bin, "build", "-o", str(out), str(gosrc)],
            cwd=str(workdir), capture_output=True, text=True, timeout=60,
        )
        return r.returncode == 0
    return False


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--langs", nargs="*",
                    default=["fastc", "c", "rust", "zig", "go"])
    ap.add_argument("--llms", nargs="*", default=None)
    args = ap.parse_args()

    resp_root = BASE / "responses" / "T6"
    if not resp_root.is_dir():
        print("no T6 responses captured yet", file=sys.stderr)
        return 1

    cells: dict[tuple[str, str], dict[str, int]] = {}

    with tempfile.TemporaryDirectory() as work_root:
        work_root = Path(work_root)
        for lang in args.langs:
            cell_root = resp_root / lang
            if not cell_root.is_dir():
                continue
            llms = sorted(d.name for d in cell_root.iterdir() if d.is_dir())
            for llm in llms:
                if args.llms and llm not in args.llms:
                    continue
                key = (lang, llm)
                cells[key] = {"trials": 0, "compiled": 0, "visible": 0}
                for resp_file in sorted((cell_root / llm).glob("*.txt")):
                    text = resp_file.read_text()
                    if text.startswith("# ERROR:"):
                        continue
                    code = extract_code(text)
                    if code is None:
                        continue
                    cells[key]["trials"] += 1

                    sig = extract_signature(code, lang)
                    if sig and signature_visible(sig):
                        cells[key]["visible"] += 1

                    wd = work_root / f"T6_{lang}_{llm}_{resp_file.stem}"
                    wd.mkdir(parents=True, exist_ok=True)
                    if try_compile(code, lang, wd):
                        cells[key]["compiled"] += 1

    print(f"{'cell':<24} {'trials':>6} {'compiled':>8} {'cap_visible':>12} {'visible_pct':>12}")
    print("-" * 70)
    for (lang, llm), c in sorted(cells.items()):
        t = c["trials"]
        vis = c["visible"]
        pct = f"{100*vis/t:.0f}%" if t else "n/a"
        cell_id = f"T6/{lang}/{llm}"
        print(f"{cell_id:<24} {t:>6} {c['compiled']:>8} {vis:>12} {pct:>12}")
    print("-" * 70)
    print("cap_visible = signature contains a Cap* / capability token a reviewer can read.")
    print("Expectation: fastC ≈ 100%, C/Rust/Zig/Go ≈ 0% (unless an LLM volunteered an annotation).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
