# First-compile-success-rate benchmark

Measures whether an LLM writing fastC produces code that compiles
on the first try more reliably than the same LLM writing C, Rust,
Zig, or Go.

## How it works

For each (task, language, LLM) triple, the harness:

1. Builds a prompt that names the language and includes a per-language syntax cheat sheet (cheatsheets/`<lang>.md`).
2. Sends the prompt to the LLM `N` times (default 10).
3. Extracts the code block from each response.
4. Tries to compile it with the language's standard release-mode invocation.
5. Records pass/fail per trial and aggregates per cell.

Per-trial responses are archived to `responses/<task>/<lang>/<llm>/<trial>.txt` so reviewers can audit the LLM output without re-spending API budget.

## Tasks

- **T1 — sum_array.** Sum an array of i32 and return an i64. Output `55`.
- **T2 — is_prime.** Read N from stdin; output `YES` if prime else `NO`. Handles N up to 2^31-1.
- **T3 — json_token.** Tokenize a small JSON-like input. Six token kinds, one per line of output.

Prompts live in `prompts/`. The `{LANG}` placeholder is substituted at runtime so every language sees the same task description with only the language name (and cheat sheet) differing.

## Setup

```bash
# 1. Pick which providers to use. The harness skips any provider
#    whose key isn't set.
export ANTHROPIC_API_KEY=...
export OPENAI_API_KEY=...
export GOOGLE_API_KEY=...

# 2. Install the per-provider SDKs (only the ones you'll use).
pip install anthropic openai google-genai

# 3. Build fastc in release mode.
cargo build --release -p fastc

# 4. Run.
cd benchmarks/cross-lang/first-compile
python3 run.py --n 10
```

For a quick sanity check use:

```bash
python3 run.py --dry-run --n 2 --tasks T1 --langs fastc c
```

To run a single cell:

```bash
python3 run.py --tasks T1 --langs fastc --llms claude --n 5
```

## Cost & wall-clock

| Mode | Cells | Trials | API cost | Wall-clock |
|---|---|---|---|---|
| Full grid, N=10, 3 LLMs | 3 × 5 × 3 = 45 | 450 | ~$5–8 | ~60–90 min |
| Single LLM, N=10 | 3 × 5 × 1 = 15 | 150 | ~$2 | ~20 min |
| Sanity run, --dry-run | n/a | 0 | 0 | < 1 sec |

The harness is **idempotent on responses**: if a trial's response file already exists, the LLM is not re-called. This lets you resume a partial run without paying twice and lets you re-compile (e.g., after fixing a compiler bug) without re-prompting.

## Honest framing

- **N=10 is small.** Pass-rate uncertainty at N=10 is ±15 percentage points at 95% confidence. The benchmark gives a directional answer — "fastC pass rate is meaningfully above / below Rust" — not a research-grade one.
- **"First compile" means the language's release-mode invocation exits 0.** It does not check the program produces the right output. A program that compiles but returns the wrong answer counts as a pass here. Correctness-on-first-try is the next benchmark axis.
- **Prompt design matters.** We give every language the same task description plus a one-screen cheat sheet at the end. Cheat sheets are normalized for length and depth across languages so no language is unfairly helped or starved.
- **Cheat sheet quality directly affects pass rate.** A bad cheat sheet for fastC would tank its number unfairly. The current cheat sheets are the best honest summary we can write; if you spot a bug or missing detail, fix the cheat sheet in `cheatsheets/<lang>.md` and re-run.
- **LLM nondeterminism.** Two runs of the same cell can produce different pass rates. The committed `results.csv` is one run's golden data, with date and provider list at the top. Re-running gives results within ±20pp per cell.

## Current results

The harness is committed but the headline run hasn't been performed yet (no API keys configured in this build environment). To populate `results.csv`, set keys as above and run.

When populated, expected shape:

| Task | Lang | Claude | GPT-4o | Gemini | Mean |
|---|---|---|---|---|---|
| T1 sum_array | fastc | TBD | TBD | TBD | TBD |
| T1 sum_array | c | TBD | TBD | TBD | TBD |
| T1 sum_array | rust | TBD | TBD | TBD | TBD |
| ... | | | | | |

The headline number the project's launch claims rests on is the **fastC vs Rust delta** averaged across all three tasks. If fastC's first-compile pass rate is +15pp or more above Rust's, the agent-friendliness wedge is real and the higher token cost (see `../token-count/`) is worth paying. If it's not, we publish the number anyway and rethink.

## Reproducing without API keys

If you want to audit the harness logic without paying for LLM calls, every committed per-trial response under `responses/<task>/<lang>/<llm>/<trial>.txt` is re-compilable from local — the script's idempotency means you can edit a response file and re-run to see whether your edit compiles. This lets reviewers verify the compile-check logic independently of LLM behavior.

(Currently no responses are committed; once the headline run happens they'll be checked in or, for large runs, lz4-compressed and committed as one archive — decision in the commit that lands the data.)
