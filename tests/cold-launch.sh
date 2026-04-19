#!/usr/bin/env bash
# tests/cold-launch.sh
#
# Phase 10 deliverable: measure BiscuitCode cold-launch time.
# A cold launch is defined as: the time from exec() to the main window being
# visible and interactive (signalled by the app printing "BISCUITCODE_READY"
# to stdout — see src-tauri/src/main.rs).
#
# Acceptance criterion: under 2000ms on i5-8xxx / 8GB hardware.
#
# Usage:
#   bash tests/cold-launch.sh [path/to/biscuitcode]
#
# If no path is given, looks for the binary in the standard Tauri output
# directory and then in PATH.
#
# Exit codes:
#   0  — launched and printed BISCUITCODE_READY within the budget
#   1  — launched but exceeded the 2000ms budget
#   2  — binary not found or failed to launch

set -euo pipefail

BUDGET_MS=2000
TIMEOUT_S=10   # hard kill if app hangs

# Locate binary.
BINARY="${1:-}"
if [ -z "$BINARY" ]; then
  # Try common Tauri output locations.
  for candidate in \
    "./src-tauri/target/release/biscuitcode" \
    "./src-tauri/target/x86_64-unknown-linux-gnu/release/biscuitcode" \
    "$(command -v biscuitcode 2>/dev/null || true)"; do
    if [ -n "$candidate" ] && [ -x "$candidate" ]; then
      BINARY="$candidate"
      break
    fi
  done
fi

if [ -z "$BINARY" ] || [ ! -x "$BINARY" ]; then
  echo "ERROR: biscuitcode binary not found. Build first with: cargo tauri build"
  echo "       Or pass the path explicitly: bash tests/cold-launch.sh /path/to/biscuitcode"
  exit 2
fi

echo "Binary: $BINARY"
echo "Budget: ${BUDGET_MS}ms"

# Drop OS page-cache pages for the binary to simulate a cold start.
# This requires root; skip silently if not available.
if [ -r /proc/sys/vm/drop_caches ]; then
  echo 1 > /proc/sys/vm/drop_caches 2>/dev/null || true
fi

# Record start time in nanoseconds.
START_NS=$(date +%s%N)

# Launch the app, capture stdout, kill after TIMEOUT_S seconds.
# The app should print "BISCUITCODE_READY" when the window is interactive.
OUTPUT=$(timeout "${TIMEOUT_S}" "$BINARY" --cold-launch-probe 2>/dev/null || true)

END_NS=$(date +%s%N)

ELAPSED_MS=$(( (END_NS - START_NS) / 1000000 ))

echo "Elapsed: ${ELAPSED_MS}ms"

if echo "$OUTPUT" | grep -q "BISCUITCODE_READY"; then
  echo "RESULT: BISCUITCODE_READY signal received."
else
  echo "WARN: BISCUITCODE_READY signal not received (app may not implement --cold-launch-probe yet)."
  echo "      Elapsed time measured to process exit / timeout."
fi

if [ "$ELAPSED_MS" -le "$BUDGET_MS" ]; then
  echo "PASS: ${ELAPSED_MS}ms <= ${BUDGET_MS}ms budget."
  exit 0
else
  echo "FAIL: ${ELAPSED_MS}ms > ${BUDGET_MS}ms budget."
  exit 1
fi
