#!/usr/bin/env bash
# Profile fastc's monomorphization pass against the largest generic-heavy
# examples in the repo, recording per-pass milliseconds via --timing.
#
# Background: stage 0.9 of the compile-time plan flagged `mono` as the
# pass most likely to harbor a quadratic algorithm. This script captures
# the empirical numbers so we can detect a regression if mono grows
# super-linearly as the stdlib expands.
#
# Usage:
#   bash benchmarks/internal/profile_mono.sh
#
# Output: a markdown table on stdout summarizing per-pass timing for each
# example, plus an aggregate "mono share of total" percentage.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FASTC="$REPO_ROOT/target/release/fastc"

EXAMPLES=(
    "examples/json_tokenizer_demo.fc"
    "examples/hashmap_demo.fc"
    "examples/word_count_demo.fc"
    "examples/str_repeat_hm_clone_demo.fc"
    "examples/str_split_demo.fc"
    "examples/hashmap_iter_demo.fc"
    "examples/vec_higher_order_demo.fc"
)

if [[ ! -x "$FASTC" ]]; then
    echo "error: $FASTC not found. run: cargo build --release -p fastc" >&2
    exit 1
fi

echo "| Example | Total ms | Mono ms | Mono % |"
echo "|---|---|---|---|"

for src in "${EXAMPLES[@]}"; do
    [[ ! -f "$REPO_ROOT/$src" ]] && continue
    json=$("$FASTC" compile "$REPO_ROOT/$src" -o /tmp/_mono_profile.c --timing 2>&1)
    summary=$(printf "%s" "$json" | python3 -c '
import json, sys
data = json.load(sys.stdin)
total = data["total_ms"]
mono = sum(p["ms"] for p in data["passes"] if p["pass"] == "mono")
pct = (100 * mono / total) if total else 0
print(f"{total} {mono} {pct:.0f}%")
')
    read -r total mono pct <<< "$summary"
    name=$(basename "$src")
    printf "| %s | %s | %s | %s |\n" "$name" "$total" "$mono" "$pct"
done
