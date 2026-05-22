# Cross-language token-count benchmark

How many LLM tokens does the same program take to write in each
language? `count_tokens.py` reads the four programs from the perf
benchmark and reports per-tokenizer counts in `results.csv`.

## Setup

```bash
pip install tiktoken
python3 count_tokens.py
```

## Tokenizers

- `cl100k_base` — OpenAI GPT-4 family.
- `o200k_base` — OpenAI GPT-4o / o-series.

Anthropic and Google's tokenizers are closed-source. Both tokenize source code within roughly 10% of `cl100k_base` based on Anthropic's published behavior, so we report `cl100k` as the headline number and treat it as a proxy for "what most modern frontier LLMs see."

## Current results (cl100k tokens)

| Program | fastC | C | Rust | Zig | Go |
|---|---|---|---|---|---|
| hello | 25 | 21 | 10 | 38 | 17 |
| sum | 105 | 58 | 55 | 65 | 52 |
| fib40 | 117 | 69 | 66 | 63 | 60 |
| mandelbrot | 547 | 260 | 301 | 346 | 267 |

## What this tells us — honestly

**fastC's token count is the highest in three of four programs.** This was not the result we expected when planning the benchmark; the MANIFESTO's agent-friendliness claim implicitly suggested fastC would land between C and Rust on this axis. It does not.

The root cause is fastC's explicit-conversion style. Every `cast(f64, x)` is several tokens; the equivalent `(double)x` in C, `x as f64` in Rust, or `@as(f64, x)` in Zig is fewer. Multiply by every numeric expression in `mandelbrot` and the token count blows up.

This is a real finding and changes the agent-friendliness pitch:

- **Token count is not where fastC's agent story lives.** Writing 547 tokens of mandelbrot in fastC vs 260 in C is a real cost.
- **fastC's agent story lives in what the type system catches.** The same 547 tokens have explicit type information at every conversion point that the compiler can verify. A Rust or C version might be terser but lets implicit conversions slip past — fastC won't compile until every conversion is named.
- **The honest framing**, going forward: "fastC programs are longer to write but more of what's written is checked by the compiler. The trade is more typing for fewer first-compile failures."

The first-compile-success-rate benchmark (`../first-compile/`) measures the second half of that claim directly. If fastC's first-compile rate is meaningfully higher than the others' despite the higher token count, the trade is worth it. If not, the token-count cost isn't justified by the safety story and the language design needs revisiting.

## What this benchmark doesn't measure

- **Effective output rate.** Tokens/second varies per LLM and per language (token frequency tables differ). We measure raw count, not generation time.
- **Tokens to read a fastC program.** A reader (human or LLM) processing fastC source spends more tokens than reading C. This benchmark only measures the encoded source, not LLM context consumption during downstream tasks.
- **Correctness on first try.** That's the next benchmark.

## Re-running

The script is deterministic — same source, same tokenizer version, same numbers. If the committed `results.csv` doesn't match your local run, either `tiktoken` updated its encoding tables or someone edited a benchmark source.
