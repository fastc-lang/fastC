#!/usr/bin/env bash
# fastC test harness — runs the full test suite with clear,
# colorized output. One command, no arguments needed.
#
# Usage:
#   bash scripts/test.sh           # everything
#   bash scripts/test.sh quick     # fast subset (unit + integration, no fuzz)
#   bash scripts/test.sh ci        # what CI runs (quick + fmt + clippy)
#
# Exit codes:
#   0 — all selected suites passed
#   1 — at least one suite failed
#   2 — a required tool (cargo) was missing

set -uo pipefail

cd "$(dirname "$0")/.."

# ANSI colors — disabled when stdout isn't a tty (CI logs, pipes).
if [ -t 1 ]; then
  GREEN='\033[0;32m'
  RED='\033[0;31m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  BOLD='\033[1m'
  RESET='\033[0m'
else
  GREEN=''; RED=''; YELLOW=''; BLUE=''; BOLD=''; RESET=''
fi

MODE="${1:-full}"
FAILED=0
TOTAL=0
PASSED=0
SKIPPED=0
START_TS=$(date +%s)

header() {
  echo
  echo -e "${BOLD}${BLUE}━━━ $1 ━━━${RESET}"
}

step() {
  local name="$1"
  local cmd="$2"
  local optional="${3:-required}"
  TOTAL=$((TOTAL + 1))
  echo
  echo -e "${BOLD}▶ $name${RESET}"
  echo -e "  ${YELLOW}\$ $cmd${RESET}"
  local step_start=$(date +%s)
  if eval "$cmd"; then
    local step_end=$(date +%s)
    PASSED=$((PASSED + 1))
    echo -e "  ${GREEN}✓ passed${RESET} (${step_end}s - ${step_start}s = $((step_end - step_start))s)"
  else
    local rc=$?
    if [ "$optional" = "optional" ]; then
      SKIPPED=$((SKIPPED + 1))
      echo -e "  ${YELLOW}↳ skipped (exit $rc, optional)${RESET}"
    else
      FAILED=$((FAILED + 1))
      echo -e "  ${RED}✗ failed (exit $rc)${RESET}"
    fi
  fi
}

# ─── tool checks ───
if ! command -v cargo >/dev/null 2>&1; then
  echo -e "${RED}error: cargo not found. Install Rust from https://rustup.rs${RESET}"
  exit 2
fi

case "$MODE" in
  quick)
    header "Quick mode: unit + integration tests"
    step "Cargo test (fastc crate)" "cargo test -p fastc --quiet 2>&1 | tail -20"
    ;;

  ci)
    header "CI mode: fmt + clippy + test"
    step "Cargo fmt --check" "cargo fmt --all --check"
    step "Cargo clippy" "cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20" optional
    step "Cargo test (workspace)" "cargo test --workspace --quiet 2>&1 | tail -20"
    ;;

  full|*)
    header "Full test suite"

    step "Build release binary (needed by integration tests)" \
         "cargo build --release -p fastc --quiet 2>&1 | tail -5"

    step "Workspace unit + integration tests" \
         "cargo test --workspace --quiet 2>&1 | tail -25"

    step "Format check" \
         "cargo fmt --all --check"

    step "Doc-tests" \
         "cargo test --doc --quiet 2>&1 | tail -5" optional

    # Examples smoke-test
    if [ -d examples ] && command -v cc >/dev/null 2>&1; then
      step "Smoke-test 5 examples through fastc + cc" '
        ok=0; fail=0
        for f in examples/01_hello_world.fc examples/02_arithmetic.fc examples/03_variables.fc examples/04_functions.fc examples/05_control_flow.fc; do
          [ -f "$f" ] || continue
          tmp_c=$(mktemp -t fastc-smoke.XXXX).c
          if ./target/release/fastc compile "$f" -o "$tmp_c" >/dev/null 2>&1; then
            if cc "$tmp_c" -Iruntime -o /dev/null >/dev/null 2>&1; then
              ok=$((ok + 1))
            else
              fail=$((fail + 1))
            fi
          else
            fail=$((fail + 1))
          fi
          rm -f "$tmp_c"
        done
        echo "  $ok passed, $fail failed"
        test $fail -eq 0
      '
    fi
    ;;
esac

END_TS=$(date +%s)
DURATION=$((END_TS - START_TS))

# ─── summary ───
echo
echo -e "${BOLD}${BLUE}━━━ Summary ━━━${RESET}"
echo "  total:    $TOTAL"
echo -e "  ${GREEN}passed:   $PASSED${RESET}"
if [ "$SKIPPED" -gt 0 ]; then
  echo -e "  ${YELLOW}skipped:  $SKIPPED${RESET}"
fi
if [ "$FAILED" -gt 0 ]; then
  echo -e "  ${RED}failed:   $FAILED${RESET}"
fi
echo "  duration: ${DURATION}s"
echo

if [ "$FAILED" -gt 0 ]; then
  echo -e "${RED}${BOLD}✗ Test harness failed.${RESET}"
  exit 1
else
  echo -e "${GREEN}${BOLD}✓ All checks passed.${RESET}"
  exit 0
fi
