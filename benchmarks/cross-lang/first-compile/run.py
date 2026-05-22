#!/usr/bin/env python3
"""First-compile-success-rate harness.

For each (task, language, LLM) triple, send the prompt N times,
extract the code block from each response, attempt to compile it,
and record pass/fail. Writes per-cell aggregates to results.csv and
per-trial responses to responses/<task>/<lang>/<llm>/<trial>.txt.

LLM providers (pick whichever you have keys for; the script skips
providers with unset keys):
- Claude via ANTHROPIC_API_KEY (uses anthropic SDK)
- GPT-4o via OPENAI_API_KEY (uses openai SDK)
- Gemini 2.5 Pro via GOOGLE_API_KEY (uses google-genai SDK)
- Ollama Cloud models (GLM, Kimi, DeepSeek, Qwen, ...) via
  OLLAMA_API_KEY — one logical LLM per model, registered in
  `ollama_models.json` next to this script. No SDK install needed;
  the harness uses urllib.request to POST to
  https://ollama.com/api/chat.

Install:
    pip install anthropic openai google-genai

Run:
    ANTHROPIC_API_KEY=... python3 run.py --n 10
    # Or just a subset:
    python3 run.py --tasks T1 --langs fastc --llms claude --n 3
    # Or Ollama-only:
    OLLAMA_API_KEY=... python3 run.py --llms glm kimi --n 5

Cost guide for N=10, all proprietary LLMs, all 3 tasks, all 5 langs:
- 450 completions total
- ~$5-8 in API charges (Anthropic + OpenAI + Google)
- ~60-90 minutes wall-clock

Ollama Cloud pricing varies per model. With four default Ollama
models (glm / kimi / deepseek / qwen) added to the grid, total
completions roughly triple (1050) and time roughly triples. Check
ollama.com for current rates.

Dry-run (no API calls, just exercises the plumbing):
    python3 run.py --dry-run
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import re
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent.parent
RUNTIME = REPO / "runtime"
FASTC = REPO / "target" / "release" / "fastc"

TASKS = ["T1", "T2", "T3", "T4", "T5"]
LANGS = ["fastc", "c", "rust", "zig", "go"]
# Built-in providers; Ollama Cloud models from ollama_models.json
# are appended after the file loads.
LLMS = ["claude", "gpt", "gemini"]

OLLAMA_API_URL = "https://ollama.com/api/chat"
# Long fastC cheatsheet + json_tokenizer task (T3) regularly pushes
# Ollama Cloud's response time past 2 minutes. 5 min is generous
# enough that genuine network/model failures still bubble up.
OLLAMA_TIMEOUT_S = 300

LANG_FILE_EXT = {
    "fastc": "fc",
    "c": "c",
    "rust": "rs",
    "zig": "zig",
    "go": "go",
}

LANG_NAME = {
    "fastc": "fastC",
    "c": "C",
    "rust": "Rust",
    "zig": "Zig",
    "go": "Go",
}

CHEATSHEET = {
    "fastc": "fc.md",
    "c": "c.md",
    "rust": "rs.md",
    "zig": "zig.md",
    "go": "go.md",
}


# ----- Prompt assembly -------------------------------------------------------


def build_prompt(task: str, lang: str) -> str:
    task_text = (BASE / "prompts" / f"{task}.md").read_text()
    cheatsheet_text = (BASE / "cheatsheets" / CHEATSHEET[lang]).read_text()
    return (
        task_text.replace("{LANG}", LANG_NAME[lang])
        + "\n\n---\n\n"
        + cheatsheet_text
    )


# ----- Provider calls --------------------------------------------------------


def call_claude(prompt: str) -> str:
    import anthropic

    client = anthropic.Anthropic()
    msg = client.messages.create(
        model="claude-sonnet-4-6",
        max_tokens=4096,
        messages=[{"role": "user", "content": prompt}],
    )
    return msg.content[0].text


def call_gpt(prompt: str) -> str:
    from openai import OpenAI

    client = OpenAI()
    resp = client.chat.completions.create(
        model="gpt-4o",
        max_tokens=4096,
        messages=[{"role": "user", "content": prompt}],
    )
    return resp.choices[0].message.content


def call_gemini(prompt: str) -> str:
    from google import genai

    client = genai.Client()
    resp = client.models.generate_content(
        model="gemini-2.5-pro",
        contents=prompt,
    )
    return resp.text


def ollama_call(model_id: str, prompt: str) -> str:
    """POST to Ollama Cloud's /api/chat. Reads OLLAMA_API_KEY for
    auth. Uses stdlib urllib so no SDK install is required.

    Enforces a hard wall-clock cap via SIGALRM because urllib's
    `timeout=` parameter only governs per-socket-read timeouts —
    a slow-loris server that dribbles bytes can hang the call
    indefinitely. SIGALRM kills the whole call after
    OLLAMA_TIMEOUT_S regardless of socket behavior."""
    import urllib.request
    import signal

    api_key = os.environ["OLLAMA_API_KEY"]
    payload = json.dumps({
        "model": model_id,
        "messages": [{"role": "user", "content": prompt}],
        "stream": False,
    }).encode("utf-8")
    req = urllib.request.Request(
        OLLAMA_API_URL,
        data=payload,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )

    def _alarm_handler(signum, frame):
        raise TimeoutError(
            f"ollama_call wall-clock timeout after {OLLAMA_TIMEOUT_S}s"
        )

    prev_handler = signal.signal(signal.SIGALRM, _alarm_handler)
    signal.alarm(OLLAMA_TIMEOUT_S)
    try:
        with urllib.request.urlopen(req, timeout=OLLAMA_TIMEOUT_S) as resp:
            body = json.loads(resp.read().decode("utf-8"))
    finally:
        signal.alarm(0)
        signal.signal(signal.SIGALRM, prev_handler)
    # Ollama's non-streaming response: {"message": {"role":..., "content":...}, ...}
    return body["message"]["content"]


def make_ollama_caller(model_id: str) -> Callable[[str], str]:
    """Factory that binds a model ID into a `call(prompt)` closure
    matching the PROVIDERS callable signature."""
    def call(prompt: str) -> str:
        return ollama_call(model_id, prompt)
    return call


PROVIDERS: dict[str, Callable[[str], str]] = {
    "claude": call_claude,
    "gpt": call_gpt,
    "gemini": call_gemini,
}


# ----- Ollama model registry -------------------------------------------------


def load_ollama_models() -> dict[str, str]:
    """Read ollama_models.json next to this script. Each non-underscore
    key is a logical LLM name (appears in --llms / results.csv /
    responses/); the value is the Ollama Cloud model ID sent in the
    API request body. Returns {} if the file is missing — no Ollama
    models is a valid configuration."""
    cfg = BASE / "ollama_models.json"
    if not cfg.exists():
        return {}
    raw = json.loads(cfg.read_text())
    # Strip comment keys (any key starting with `_`) so users can leave
    # notes in the JSON file without polluting the provider list.
    return {k: v for k, v in raw.items() if not k.startswith("_")}


OLLAMA_MODELS = load_ollama_models()
for _logical, _model_id in OLLAMA_MODELS.items():
    PROVIDERS[_logical] = make_ollama_caller(_model_id)
    if _logical not in LLMS:
        LLMS.append(_logical)


# ----- Code extraction -------------------------------------------------------


CODE_BLOCK_RE = re.compile(r"```[a-zA-Z0-9_+\-]*\n(.*?)```", re.DOTALL)


def extract_code(response: str, lang: str) -> str | None:
    # Prefer fenced block tagged with the language. Fall back to any
    # fenced block. Fall back to the full response if no fences.
    for m in CODE_BLOCK_RE.finditer(response):
        head = response[: m.start()]
        if LANG_NAME[lang].lower() in head.lower()[-200:]:
            return m.group(1)
    matches = CODE_BLOCK_RE.findall(response)
    if matches:
        return matches[0]
    return response.strip() or None


# ----- Compile attempts ------------------------------------------------------


def try_compile(code: str, lang: str, workdir: Path) -> tuple[bool, str]:
    """Return (passed, stderr)."""
    ext = LANG_FILE_EXT[lang]
    src = workdir / f"prog.{ext}"
    src.write_text(code)
    out = workdir / "prog"

    if lang == "fastc":
        c_out = workdir / "prog.c"
        r = subprocess.run(
            [str(FASTC), "compile", str(src), "-o", str(c_out)],
            capture_output=True, text=True, timeout=30,
        )
        if r.returncode != 0:
            return False, r.stderr
        r = subprocess.run(
            ["cc", "-O2", f"-I{RUNTIME}", str(c_out), "-o", str(out)],
            capture_output=True, text=True, timeout=30,
        )
        return r.returncode == 0, r.stderr
    if lang == "c":
        r = subprocess.run(
            ["gcc", "-O2", str(src), "-o", str(out)],
            capture_output=True, text=True, timeout=30,
        )
        return r.returncode == 0, r.stderr
    if lang == "rust":
        r = subprocess.run(
            ["rustc", "-O", str(src), "-o", str(out)],
            capture_output=True, text=True, timeout=60,
        )
        return r.returncode == 0, r.stderr
    if lang == "zig":
        # Move source to workdir-local for zig's output behavior.
        r = subprocess.run(
            ["zig", "build-exe", "-O", "ReleaseFast", "-lc",
             "--name", "prog", str(src)],
            cwd=str(workdir), capture_output=True, text=True, timeout=60,
        )
        return r.returncode == 0, r.stderr
    if lang == "go":
        go_bin = os.environ.get("GO_BIN")
        if not go_bin:
            for cand in ["/opt/homebrew/Cellar/go/1.26.3/bin/go",
                         "/usr/local/go/bin/go"]:
                if Path(cand).exists():
                    go_bin = cand
                    break
        if not go_bin:
            return False, "go binary not found"
        # Go demands a directory of its own; copy into workdir.
        gosrc = workdir / "main.go"
        gosrc.write_text(code)
        r = subprocess.run(
            [go_bin, "build", "-o", str(out), str(gosrc)],
            cwd=str(workdir), capture_output=True, text=True, timeout=60,
        )
        return r.returncode == 0, r.stderr
    raise ValueError(f"unknown lang: {lang}")


# ----- Main loop -------------------------------------------------------------


@dataclass
class CellResult:
    task: str
    lang: str
    llm: str
    trials: int
    passes: int

    @property
    def pass_rate(self) -> float:
        return self.passes / self.trials if self.trials else 0.0


def run_cell(task: str, lang: str, llm: str, n: int,
             dry_run: bool, work_root: Path,
             sleep_ms: int = 0) -> CellResult:
    print(f"  {task} / {lang} / {llm}: ", end="", flush=True)
    prompt = build_prompt(task, lang)
    passes = 0
    actual = 0
    for trial in range(n):
        resp_dir = BASE / "responses" / task / lang / llm
        resp_dir.mkdir(parents=True, exist_ok=True)
        resp_file = resp_dir / f"{trial:02d}.txt"
        if dry_run:
            print(".", end="", flush=True)
            continue
        # Treat error-marked response files as cache misses so a
        # rerun with --sleep-ms (or after a key swap) picks them up.
        cached = resp_file.exists() and not resp_file.read_text().startswith("# ERROR:")
        if cached:
            response = resp_file.read_text()
        else:
            try:
                response = PROVIDERS[llm](prompt)
            except Exception as e:
                print(f"!", end="", flush=True)
                resp_file.write_text(f"# ERROR: {type(e).__name__}: {e}\n")
                if sleep_ms > 0:
                    time.sleep(sleep_ms / 1000.0)
                continue
            resp_file.write_text(response)
            if sleep_ms > 0:
                time.sleep(sleep_ms / 1000.0)
        code = extract_code(response, lang)
        if code is None:
            print("-", end="", flush=True)
            actual += 1
            continue
        work = work_root / f"trial_{trial:02d}"
        work.mkdir(parents=True, exist_ok=True)
        ok, _err = try_compile(code, lang, work)
        actual += 1
        if ok:
            passes += 1
            print("+", end="", flush=True)
        else:
            print("x", end="", flush=True)
    print(f"  ({passes}/{actual})")
    return CellResult(task=task, lang=lang, llm=llm,
                      trials=actual, passes=passes)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--n", type=int, default=10)
    ap.add_argument("--tasks", nargs="*", default=TASKS)
    ap.add_argument("--langs", nargs="*", default=LANGS)
    ap.add_argument("--llms", nargs="*", default=LLMS)
    ap.add_argument("--dry-run", action="store_true",
                    help="exercise the plumbing without calling LLMs")
    ap.add_argument("--sleep-ms", type=int, default=0,
                    help="sleep this many milliseconds between LLM calls "
                         "(useful for rate-limited providers like Ollama Cloud)")
    args = ap.parse_args()

    # Skip providers that don't have keys configured. Built-in
    # providers have their own per-provider env var; every Ollama
    # logical name defaults to OLLAMA_API_KEY.
    PROVIDER_KEY = {
        "claude": "ANTHROPIC_API_KEY",
        "gpt": "OPENAI_API_KEY",
        "gemini": "GOOGLE_API_KEY",
    }
    available_llms = []
    for llm in args.llms:
        if llm not in PROVIDERS:
            print(f"error: unknown LLM '{llm}'. Known: {', '.join(sorted(PROVIDERS))}",
                  file=sys.stderr)
            return 1
        key_var = PROVIDER_KEY.get(llm, "OLLAMA_API_KEY")
        if args.dry_run or os.environ.get(key_var):
            available_llms.append(llm)
        else:
            print(f"skipping {llm}: {key_var} not set", file=sys.stderr)
    if not available_llms and not args.dry_run:
        print("error: no LLM provider keys set, nothing to run", file=sys.stderr)
        return 1

    if not FASTC.exists():
        print(f"error: {FASTC} not found. run: cargo build --release -p fastc",
              file=sys.stderr)
        return 1

    work_root = Path("/tmp") / f"first_compile_{int(time.time())}"
    work_root.mkdir(parents=True, exist_ok=True)
    print(f"work dir: {work_root}")

    results: list[CellResult] = []
    for task in args.tasks:
        for lang in args.langs:
            for llm in available_llms:
                cell = run_cell(task, lang, llm, args.n, args.dry_run,
                                work_root / task / lang / llm,
                                sleep_ms=args.sleep_ms)
                results.append(cell)

    if args.dry_run:
        return 0

    out_csv = BASE / "results.csv"
    with out_csv.open("w", newline="") as f:
        f.write(f"# fastC first-compile-success-rate benchmark\n")
        f.write(f"# Generated: {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}\n")
        f.write(f"# Providers: {', '.join(available_llms)}\n")
        f.write(f"# Trials per cell: {args.n}\n")
        w = csv.writer(f)
        w.writerow(["task", "language", "llm", "trials", "passes", "pass_rate"])
        for r in results:
            w.writerow([r.task, r.lang, r.llm, r.trials, r.passes,
                        f"{r.pass_rate:.2f}"])
    print(f"results: {out_csv}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
