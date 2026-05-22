#!/usr/bin/env bash
# Cross-language compile/size/runtime benchmark.
#
# For each of {hello, sum, fib40, mandelbrot} × {fc, c, rs, zig, go}:
#   1. Time the source-to-binary build (hyperfine --warmup 1 --runs 3).
#   2. Strip the binary, record its byte size.
#   3. Time runtime (hyperfine --warmup 1 --runs 5).
# Write everything to results.csv with a date stamp at the top.
#
# Required on PATH or in standard brew locations:
#   - fastc (target/release built via `cargo build --release -p fastc`)
#   - gcc, clang (Xcode CLT)
#   - rustc, zig
#   - hyperfine
# Go is optional — if `GO_BIN` is unset, the script auto-discovers
# brew's go install; if no Go is found, the Go column shows "N/A".
#
# Re-running:
#   $ ./run.sh
# The script writes results.csv in-place. The committed file is the
# golden run; per-run variance is typically ±15%.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BENCH_DIR="$REPO_ROOT/benchmarks/cross-lang"
FASTC="${FASTC:-$REPO_ROOT/target/release/fastc}"
RUNTIME="$REPO_ROOT/runtime"
GO_BIN="${GO_BIN:-}"

if [[ -z "$GO_BIN" ]]; then
    for cand in /opt/homebrew/Cellar/go/*/bin/go /usr/local/go/bin/go; do
        if [[ -x "$cand" ]]; then GO_BIN="$cand"; break; fi
    done
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

OUT="$BENCH_DIR/results.csv"
DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
HOST="$(uname -mnsr)"

{
    echo "# fastC cross-language benchmark"
    echo "# Generated: $DATE"
    echo "# Host: $HOST"
    echo "# Methodology: hyperfine --warmup 1 --runs 3 for compile, --runs 5 for runtime."
    echo "# Compile time includes source -> stripped binary. fastC compile = fastc + cc -O2."
    echo "program,language,compile_ms_median,strip_bytes,runtime_ms_median"
} > "$OUT"

build_fc() {
    local prog="$1" out_bin="$2"
    local src="$BENCH_DIR/$prog/fc/main.fc"
    local c_tmp="$WORK/${prog}.c"
    "$FASTC" compile "$src" -o "$c_tmp" >/dev/null 2>&1
    cc -O2 -I"$RUNTIME" "$c_tmp" -o "$out_bin" 2>/dev/null
}

build_c()   { gcc -O2 "$BENCH_DIR/$1/c/main.c" -o "$2"; }
build_rs()  { rustc -O "$BENCH_DIR/$1/rs/main.rs" -o "$2" 2>/dev/null; }
build_zig() {
    local prog="$1" out="$2"
    pushd "$WORK" >/dev/null
    zig build-exe -O ReleaseFast -lc --name "$(basename "$out")" "$BENCH_DIR/$prog/zig/main.zig" >/dev/null 2>&1
    mv "./$(basename "$out")" "$out"
    rm -f "./$(basename "$out").o" "./$(basename "$out").o.lock" 2>/dev/null || true
    popd >/dev/null
}
build_go()  {
    [[ -z "$GO_BIN" ]] && return 1
    pushd "$WORK" >/dev/null
    cp "$BENCH_DIR/$1/go/main.go" .
    "$GO_BIN" build -o "$2" main.go
    popd >/dev/null
}

# Get median compile time in ms by running hyperfine --export-json and parsing.
# Hyperfine emits seconds; we convert to ms.
time_compile() {
    local cmd="$1" bin="$2" json="$WORK/hf.json"
    rm -f "$bin"
    if ! hyperfine --warmup 1 --runs 3 --export-json "$json" \
            --prepare "rm -f '$bin'" "$cmd" >/dev/null 2>&1; then
        echo "FAIL"
        return
    fi
    python3 -c "import json,sys; d=json.load(open('$json')); print(int(d['results'][0]['median']*1000))"
}

time_runtime() {
    local bin="$1" json="$WORK/hf.json"
    if [[ ! -x "$bin" ]]; then echo "N/A"; return; fi
    # `sum` / `fib40` exit with their computed value (32 / 203) — hyperfine
    # treats any non-zero as failure by default. `--ignore-failure` keeps the
    # timing data; we already validated correctness in the sources by hand.
    if ! hyperfine --warmup 1 --runs 5 --ignore-failure --export-json "$json" \
            "$bin > /dev/null" >/dev/null 2>&1; then
        echo "FAIL"
        return
    fi
    python3 -c "import json,sys; d=json.load(open('$json')); print(int(d['results'][0]['median']*1000))"
}

strip_size() {
    local bin="$1"
    if [[ ! -x "$bin" ]]; then echo "N/A"; return; fi
    strip "$bin" 2>/dev/null || true
    stat -f%z "$bin" 2>/dev/null || stat -c%s "$bin"
}

# Build the compile-time invocation as a single shell string per
# language so hyperfine can re-run it. We use bash -c so the shell
# resolves the variables fresh each invocation.
fc_cmd()  { echo "$FASTC compile '$BENCH_DIR/$1/fc/main.fc' -o '$WORK/$1.c' && cc -O2 -I'$RUNTIME' '$WORK/$1.c' -o '$WORK/$1_fc'"; }
c_cmd()   { echo "gcc -O2 '$BENCH_DIR/$1/c/main.c' -o '$WORK/$1_c'"; }
rs_cmd()  { echo "rustc -O '$BENCH_DIR/$1/rs/main.rs' -o '$WORK/$1_rs' 2>/dev/null"; }
zig_cmd() { echo "cd '$WORK' && zig build-exe -O ReleaseFast -lc --name $1_zig '$BENCH_DIR/$1/zig/main.zig' >/dev/null 2>&1"; }
go_cmd()  { [[ -z "$GO_BIN" ]] && echo "" && return; echo "cd '$WORK' && cp '$BENCH_DIR/$1/go/main.go' . && '$GO_BIN' build -o '$1_go' main.go"; }

for prog in hello sum fib40 mandelbrot; do
    echo "--- $prog ---"

    # fastC
    fc_bin="$WORK/${prog}_fc"
    fc_compile=$(time_compile "$(fc_cmd "$prog")" "$fc_bin")
    fc_size=$(strip_size "$fc_bin")
    fc_run=$(time_runtime "$fc_bin")
    echo "$prog,fastc,$fc_compile,$fc_size,$fc_run" >> "$OUT"

    # C
    c_bin="$WORK/${prog}_c"
    c_compile=$(time_compile "$(c_cmd "$prog")" "$c_bin")
    c_size=$(strip_size "$c_bin")
    c_run=$(time_runtime "$c_bin")
    echo "$prog,c,$c_compile,$c_size,$c_run" >> "$OUT"

    # Rust
    rs_bin="$WORK/${prog}_rs"
    rs_compile=$(time_compile "$(rs_cmd "$prog")" "$rs_bin")
    rs_size=$(strip_size "$rs_bin")
    rs_run=$(time_runtime "$rs_bin")
    echo "$prog,rust,$rs_compile,$rs_size,$rs_run" >> "$OUT"

    # Zig
    zig_bin="$WORK/${prog}_zig"
    zig_compile=$(time_compile "$(zig_cmd "$prog")" "$zig_bin")
    zig_size=$(strip_size "$zig_bin")
    zig_run=$(time_runtime "$zig_bin")
    echo "$prog,zig,$zig_compile,$zig_size,$zig_run" >> "$OUT"

    # Go
    go_bin="$WORK/${prog}_go"
    if [[ -n "$GO_BIN" ]]; then
        go_compile=$(time_compile "$(go_cmd "$prog")" "$go_bin")
        go_size=$(strip_size "$go_bin")
        go_run=$(time_runtime "$go_bin")
        echo "$prog,go,$go_compile,$go_size,$go_run" >> "$OUT"
    else
        echo "$prog,go,N/A,N/A,N/A" >> "$OUT"
    fi
done

echo "--- results written to $OUT ---"
cat "$OUT"
