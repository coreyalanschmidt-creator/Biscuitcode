#!/usr/bin/env bash
# scripts/test-all.sh — run every gate the CI runs, in the same order.
#
# Usage:
#   bash scripts/test-all.sh          # run everything; stop on first failure
#   bash scripts/test-all.sh --fast   # skip cargo audit (slow, rarely changes)
#   bash scripts/test-all.sh --all    # also run release-mode build (10+ min)
#
# Every gate this runs is a hard CI gate on PR #1's pattern. If this script
# passes locally, CI should pass too — and if it doesn't pass here, don't
# bother pushing.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

FAST=0
ALL=0
for arg in "$@"; do
  case "$arg" in
    --fast) FAST=1 ;;
    --all)  ALL=1 ;;
    -h|--help)
      sed -n '2,12p' "$0"
      exit 0
      ;;
  esac
done

step() { printf '\n\033[1;36m==>\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m[PASS]\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31m[FAIL]\033[0m %s\n' "$*"; exit 1; }

step "cargo fmt --check"
cargo fmt --all --manifest-path src-tauri/Cargo.toml -- --check \
  && ok "rustfmt clean" || fail "rustfmt diffs — run \`cargo fmt --all\` in src-tauri"

step "cargo clippy -D warnings"
cargo clippy --workspace --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings \
  && ok "clippy clean" || fail "clippy errors"

step "cargo test --workspace"
cargo test --workspace --manifest-path src-tauri/Cargo.toml \
  && ok "cargo tests pass" || fail "cargo tests"

step "pnpm typecheck"
pnpm typecheck && ok "tsc clean" || fail "typecheck"

step "pnpm lint"
pnpm lint && ok "eslint clean" || fail "eslint"

step "pnpm check:i18n"
pnpm check:i18n && ok "i18n clean" || fail "i18n gate"

step "pnpm test (vitest unit + integration)"
pnpm test && ok "vitest pass" || fail "vitest"

step "pnpm test:e2e (vitest e2e via second config)"
pnpm test:e2e && ok "e2e pass" || fail "e2e"

step "pnpm audit --prod"
pnpm audit --prod && ok "pnpm audit clean" || fail "pnpm audit"

if [ "$FAST" = "0" ]; then
  step "cargo audit"
  if command -v cargo-audit >/dev/null 2>&1; then
    (cd src-tauri && cargo audit) && ok "cargo audit clean" || fail "cargo audit"
  else
    printf '\033[1;33m[SKIP]\033[0m cargo-audit not installed; run \`cargo install cargo-audit --locked\`\n'
  fi

  step "license scan (cargo-license + jq)"
  if command -v cargo-license >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
    disallowed=$(cd src-tauri && cargo license --json 2>/dev/null | jq -r '
      def is_safe(lic): lic | test("^(MIT|Apache-2\\.0|BSD-[23]-Clause|ISC|Unlicense|Zlib|CC0-1\\.0|MPL-2\\.0|0BSD)$");
      def has_safe_alt(s): (s | split(" OR ") | map(is_safe(.)) | any);
      .[]
      | .license as $L
      | select(($L // "") != "")
      | select(($L | test("GPL|AGPL|LGPL|SSPL"; "i")))
      | select((has_safe_alt($L)) | not)
      | "\(.name) \(.version) (\($L))"')
    if [ -z "$disallowed" ]; then
      ok "license scan clean"
    else
      printf '%s\n' "$disallowed"
      fail "disallowed licenses"
    fi
  else
    printf '\033[1;33m[SKIP]\033[0m cargo-license or jq missing; install with \`cargo install cargo-license --locked && sudo apt install jq\`\n'
  fi
fi

if [ "$ALL" = "1" ]; then
  step "pnpm tauri build --bundles deb (release, 10+ min)"
  pnpm tauri build --bundles deb && ok ".deb built" || fail "tauri build"
fi

printf '\n\033[1;32mAll gates pass.\033[0m Safe to push.\n'
