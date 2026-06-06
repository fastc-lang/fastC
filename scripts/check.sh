#!/usr/bin/env bash
# Pre-commit / pre-push check. Runs fmt + clippy + quick tests.
# Use this before opening a PR.
#
#   bash scripts/check.sh

exec bash "$(dirname "$0")/test.sh" ci
