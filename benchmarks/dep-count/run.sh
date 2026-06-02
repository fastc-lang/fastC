#!/usr/bin/env bash
# Dep-count benchmark runner (D3).
#
# Counts transitive dependencies and executable build-script
# invocations for the same toy program (HTTP fetch + JSON parse)
# across the language matrix. Outputs results.csv with columns:
#
#   language, total_deps, build_scripts, host, timestamp, git_sha
#
# This is a measurement script — it counts what's present in each
# language sub-dir's manifest / lock file / tree. The programs
# themselves can be skeletal as long as they declare the relevant
# deps.

set -uo pipefail

cd "$(dirname "$0")"

TS=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
HOST=$(uname -nrm 2>/dev/null || echo "unknown")
GIT_SHA=$(git -C ../.. rev-parse --short HEAD 2>/dev/null || echo "unknown")

CSV="results.csv"
if [ ! -f "$CSV" ]; then
  echo "language,total_deps,build_scripts,host,timestamp,git_sha" > "$CSV"
fi

count_fastc() {
  local lock="fastc/fastc.lock"
  if [ -f "$lock" ]; then
    # Count [[package]] sections.
    grep -c "^\[\[package\]\]" "$lock" 2>/dev/null || echo 0
  else
    echo 0
  fi
}

count_rust() {
  if [ -f "rust/Cargo.lock" ]; then
    grep -c "^\[\[package\]\]" "rust/Cargo.lock" 2>/dev/null || echo 0
  elif [ -d "rust" ] && command -v cargo >/dev/null 2>&1; then
    (cd rust && cargo tree 2>/dev/null | wc -l) || echo 0
  else
    echo 0
  fi
}

count_go() {
  if [ -f "go/go.sum" ]; then
    awk '{print $1}' "go/go.sum" | sort -u | wc -l | tr -d ' '
  else
    echo 0
  fi
}

count_zig() {
  if [ -f "zig/build.zig.zon" ]; then
    # Crude — counts dependency entries by .url occurrences.
    grep -c "\.url = " "zig/build.zig.zon" 2>/dev/null || echo 0
  else
    echo 0
  fi
}

count_c() {
  # C has no manifest; explicit deps live in the Makefile / source.
  # Count #include <foo.h> with non-libc names as a proxy.
  if [ -d "c" ]; then
    grep -h "^#include <" c/*.c 2>/dev/null \
      | sort -u \
      | grep -v "^#include <std" \
      | grep -v "^#include <string\.h>" \
      | grep -v "^#include <stdint\.h>" \
      | wc -l | tr -d ' '
  else
    echo 0
  fi
}

# Build-script counts. fastC's manifest is closed-schema with no
# build scripts — structurally 0. Other languages count their
# actual build-time executable hooks.
count_fastc_scripts() { echo 0; }
count_rust_scripts() {
  find rust -name build.rs -not -path "*/target/*" 2>/dev/null | wc -l | tr -d ' '
}
count_go_scripts() {
  # Go runs `go generate` directives. Count them.
  if [ -d "go" ]; then
    grep -rh "//go:generate" go/ 2>/dev/null | wc -l | tr -d ' '
  else
    echo 0
  fi
}
count_zig_scripts() { echo 0; }
count_c_scripts() { echo 0; }

for lang in fastc rust go zig c; do
  total=$("count_${lang}")
  scripts=$("count_${lang}_scripts")
  echo "$lang,$total,$scripts,$HOST,$TS,$GIT_SHA" >> "$CSV"
  printf "%-10s deps=%s build_scripts=%s\n" "$lang" "$total" "$scripts"
done

echo "Results appended to $CSV"
