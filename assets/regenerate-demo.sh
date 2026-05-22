#!/usr/bin/env bash
# Regenerate the README demo GIF.
#
# Storyboard (≤45 seconds):
#  1. Show a fastC program that calls time::now() without a cap.
#  2. Try to compile → type error names the missing argument.
#  3. Show the fixed version that takes a cap argument.
#  4. Compile → success.
#
# Prerequisites:
#   brew install vhs       # also pulls in ttyd
#   cargo build --release -p fastc
#
# Run:
#   cd /Volumes/Github/fastC
#   ./assets/regenerate-demo.sh
#
# Outputs:
#   assets/demo.gif  ← embedded in README.md
#   assets/demo.tape ← editable source for the recording

set -euo pipefail

if ! command -v vhs >/dev/null; then
    echo "error: vhs not installed. run: brew install vhs" >&2
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

if [[ ! -x "target/release/fastc" ]]; then
    echo "fastc binary missing — building..."
    cargo build --release -p fastc
fi

vhs assets/demo.tape

echo "Wrote: assets/demo.gif ($(stat -f%z assets/demo.gif 2>/dev/null || stat -c%s assets/demo.gif) bytes)"
