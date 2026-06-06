#!/usr/bin/env bash
# Run the full benchmark suite. Wraps benchmarks/run_all.sh with
# friendlier framing.
#
#   bash scripts/bench.sh

set -uo pipefail

cd "$(dirname "$0")/.."

if [ -t 1 ]; then
  BOLD='\033[1m'; BLUE='\033[0;34m'; RESET='\033[0m'
else
  BOLD=''; BLUE=''; RESET=''
fi

echo -e "${BOLD}${BLUE}fastC benchmark harness${RESET}"
echo "Output → benchmarks/SUMMARY.md"
echo
exec bash benchmarks/run_all.sh
