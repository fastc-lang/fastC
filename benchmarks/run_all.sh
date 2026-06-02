#!/usr/bin/env bash
# Umbrella benchmark runner for the v1.x close-out (D4).
#
# Runs every benchmark harness fastC ships and aggregates results
# into a single SUMMARY.md at the repo root of `benchmarks/`. Each
# sub-harness has its own run script and CSV output; this script
# orchestrates the ordering and writes the timestamp / host / git
# sha header.
#
# Usage: bash benchmarks/run_all.sh
#
# Skips any harness whose dependencies aren't installed
# (`hyperfine` for cross-lang perf, `python3` + tiktoken for
# token-count, etc.).
#
# Exit code 0 even if individual harnesses are skipped — `run_all.sh`
# is documentation-quality, not CI-quality. Individual harnesses
# have their own pass/fail discipline.

set -uo pipefail

cd "$(dirname "$0")"

SUMMARY="$(pwd)/SUMMARY.md"
TS=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
HOST=$(uname -nrm 2>/dev/null || echo "unknown")
GIT_SHA=$(git -C .. rev-parse --short HEAD 2>/dev/null || echo "unknown")

{
  echo "# fastC benchmark summary"
  echo
  echo "- timestamp (UTC): \`$TS\`"
  echo "- host: \`$HOST\`"
  echo "- git sha: \`$GIT_SHA\`"
  echo
} > "$SUMMARY"

run_harness() {
  local name="$1"
  local cmd="$2"
  local cwd="$3"
  echo "==> $name"
  echo "## $name" >> "$SUMMARY"
  echo >> "$SUMMARY"
  if [ -d "$cwd" ]; then
    (cd "$cwd" && eval "$cmd") 2>&1 | tee -a "$SUMMARY"
    echo >> "$SUMMARY"
  else
    echo "_skipped — $cwd not found_" >> "$SUMMARY"
    echo >> "$SUMMARY"
  fi
}

# 1. Cross-language perf benchmarks (hello / sum / fib40 / mandelbrot)
run_harness "Cross-lang perf" "bash run.sh 2>&1 || true" "cross-lang"

# 2. Agent first-compile success rates
run_harness "First-compile rates" \
  "if command -v python3 >/dev/null && [ -f run.py ]; then python3 run.py --skip-network || true; fi" \
  "cross-lang/first-compile"

# 3. Token-count comparison
run_harness "Token counts" \
  "if command -v python3 >/dev/null && [ -f count_tokens.py ]; then python3 count_tokens.py || true; fi" \
  "cross-lang/token-count"

# 4. Dep-count benchmark (D3)
run_harness "Dep counts" "bash run.sh 2>&1 || true" "dep-count"

# 5. Internal monomorphization profile
run_harness "Monomorphization profile" "bash profile_mono.sh 2>&1 || true" "internal"

echo
echo "Summary written to $SUMMARY"
