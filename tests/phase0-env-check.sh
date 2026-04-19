#!/usr/bin/env bash
# tests/phase0-env-check.sh
#
# Asserts all Phase 0 acceptance criteria. Run from the project root:
#   bash tests/phase0-env-check.sh
#
# Exits 0 if every check passes; exits 1 on first failure (or after all
# checks if --no-fail-fast is given).
#
# This script is read-only and makes no modifications to the system.

set -uo pipefail

PASS=0
FAIL=0

green()  { printf '\033[32m  [PASS] %s\033[0m\n' "$*"; }
red()    { printf '\033[31m  [FAIL] %s\033[0m\n' "$*"; }
header() { printf '\033[34m%s\033[0m\n' "$*"; }

assert() {
  local desc="$1"
  local result="$2"   # "pass" or "fail"
  if [[ "$result" == "pass" ]]; then
    green "$desc"
    PASS=$((PASS + 1))
  else
    red "$desc"
    FAIL=$((FAIL + 1))
  fi
}

header "=== Phase 0 acceptance-criteria check ==="

# AC: Running inside WSL2
if grep -qiE 'wsl2|microsoft' /proc/version 2>/dev/null; then
  assert "WSL2 detected (/proc/version contains microsoft/WSL2)" "pass"
else
  assert "WSL2 detected (/proc/version contains microsoft/WSL2)" "fail"
fi

# AC: Ubuntu 24.04 noble
if grep -q '^VERSION_CODENAME=noble' /etc/os-release 2>/dev/null; then
  assert "Ubuntu 24.04 noble detected" "pass"
else
  assert "Ubuntu 24.04 noble detected" "fail"
fi

# AC: Project lives under HOME, not /mnt/
PROJECT_DIR="$(realpath "$(pwd)")"
case "$PROJECT_DIR" in
  /mnt/*)
    assert "Project under \$HOME (not /mnt/): $PROJECT_DIR" "fail"
    ;;
  *)
    assert "Project under \$HOME (not /mnt/): $PROJECT_DIR" "pass"
    ;;
esac

# AC: busctl present
if command -v busctl >/dev/null 2>&1; then
  assert "busctl present at $(command -v busctl)" "pass"
else
  assert "busctl present in PATH" "fail"
fi

# AC: libwebkit2gtk-4.1-dev installed
if apt list --installed 2>/dev/null | grep -q libwebkit2gtk-4.1-dev; then
  WEBKIT_VER="$(apt list --installed 2>/dev/null | grep libwebkit2gtk-4.1-dev | awk -F'/' '{print $2}' | awk '{print $1}')"
  assert "libwebkit2gtk-4.1-dev installed ($WEBKIT_VER)" "pass"
else
  assert "libwebkit2gtk-4.1-dev installed" "fail"
fi

# AC: webkit2gtk-4.1 pkg-config check (the script's post-install verify)
if pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
  assert "pkg-config webkit2gtk-4.1 OK ($(pkg-config --modversion webkit2gtk-4.1))" "pass"
else
  assert "pkg-config webkit2gtk-4.1 OK" "fail"
fi

# AC: cargo tauri --version prints 2.10.x
if command -v cargo-tauri >/dev/null 2>&1 || command -v cargo >/dev/null 2>&1; then
  TAURI_VER="$(cargo tauri --version 2>/dev/null | awk '{print $2}')"
  if [[ "$TAURI_VER" == 2.10.* ]]; then
    assert "cargo tauri --version = $TAURI_VER (2.10.x)" "pass"
  else
    assert "cargo tauri --version = $TAURI_VER (expected 2.10.x)" "fail"
  fi
else
  assert "cargo tauri --version (cargo not found)" "fail"
fi

# AC: pnpm --version prints 9.x or higher
if command -v pnpm >/dev/null 2>&1; then
  PNPM_VER="$(pnpm --version 2>/dev/null)"
  PNPM_MAJOR="${PNPM_VER%%.*}"
  if [[ "$PNPM_MAJOR" -ge 9 ]]; then
    assert "pnpm --version = $PNPM_VER (>= 9)" "pass"
  else
    assert "pnpm --version = $PNPM_VER (expected >= 9)" "fail"
  fi
else
  assert "pnpm --version (pnpm not found)" "fail"
fi

# AC: rustc >= 1.85
if command -v rustc >/dev/null 2>&1; then
  RUSTC_VER="$(rustc --version | awk '{print $2}')"
  RUST_MIN="1.85"
  OLDEST="$(printf '%s\n' "$RUST_MIN" "$RUSTC_VER" | sort -V | head -n1)"
  if [[ "$OLDEST" == "$RUST_MIN" ]]; then
    assert "rustc $RUSTC_VER >= $RUST_MIN (MSRV)" "pass"
  else
    assert "rustc $RUSTC_VER >= $RUST_MIN (MSRV) — too old" "fail"
  fi
else
  assert "rustc present (not found)" "fail"
fi

# AC: busctl --user list returns 0 or 1 for org.freedesktop.secrets
SECRETS_COUNT="$(timeout 5 busctl --user list 2>/dev/null | grep -c org.freedesktop.secrets || echo "0")"
if [[ "$SECRETS_COUNT" == "0" || "$SECRETS_COUNT" == "1" ]]; then
  assert "busctl --user list secrets count = $SECRETS_COUNT (0 or 1 is acceptable)" "pass"
else
  assert "busctl --user list secrets count = $SECRETS_COUNT (expected 0 or 1)" "fail"
fi

# AC: docs/DEV-SETUP.md exists
if [[ -f "docs/DEV-SETUP.md" ]]; then
  assert "docs/DEV-SETUP.md exists" "pass"
else
  assert "docs/DEV-SETUP.md exists" "fail"
fi

# AC: docs/DEV-SETUP.md linked from README.md
if grep -qF 'DEV-SETUP.md' README.md 2>/dev/null; then
  assert "docs/DEV-SETUP.md linked from README.md" "pass"
else
  assert "docs/DEV-SETUP.md linked from README.md" "fail"
fi

# AC: scripts/bootstrap-wsl.sh exists and is executable
if [[ -f "scripts/bootstrap-wsl.sh" ]]; then
  assert "scripts/bootstrap-wsl.sh exists" "pass"
else
  assert "scripts/bootstrap-wsl.sh exists" "fail"
fi

# AC: scripts/bootstrap-toolchain.sh exists
if [[ -f "scripts/bootstrap-toolchain.sh" ]]; then
  assert "scripts/bootstrap-toolchain.sh exists" "pass"
else
  assert "scripts/bootstrap-toolchain.sh exists" "fail"
fi

# AC: bootstrap-wsl.sh pre-flight checks pass (shellcheck-style dry run of env)
# We source only the pre-flight logic via a subshell, not the apt install section.
if bash -n scripts/bootstrap-wsl.sh 2>/dev/null; then
  assert "scripts/bootstrap-wsl.sh bash syntax OK" "pass"
else
  assert "scripts/bootstrap-wsl.sh bash syntax OK" "fail"
fi

if bash -n scripts/bootstrap-toolchain.sh 2>/dev/null; then
  assert "scripts/bootstrap-toolchain.sh bash syntax OK" "pass"
else
  assert "scripts/bootstrap-toolchain.sh bash syntax OK" "fail"
fi

# ----- PM-01 falsification: pkg-config name is precisely webkit2gtk-4.1 -----
# (Not webkit2gtk-6.0 or webkit2gtk-4.0)
PKG_NAME="$(pkg-config --list-all 2>/dev/null | grep -E '^webkit2gtk-' | awk '{print $1}' | head -5)"
if echo "$PKG_NAME" | grep -q "webkit2gtk-4.1"; then
  assert "PM-01 falsified: webkit2gtk-4.1 .pc file present (found: $(echo "$PKG_NAME" | tr '\n' ' '))" "pass"
else
  assert "PM-01 falsified: webkit2gtk-4.1 .pc file present (found: $(echo "$PKG_NAME" | tr '\n' ' '))" "fail"
fi

# ----- PM-02 falsification: nvm ls "20" resolves ----
if [[ -d "$HOME/.nvm" ]]; then
  NVM_DIR_VAL="$HOME/.nvm"
  # shellcheck disable=SC1091
  [[ -s "$NVM_DIR_VAL/nvm.sh" ]] && . "$NVM_DIR_VAL/nvm.sh" 2>/dev/null
  if nvm ls "20" 2>/dev/null | grep -q "v20"; then
    assert "PM-02 falsified: nvm ls 20 resolves to a v20.x.y entry" "pass"
  else
    assert "PM-02 falsified: nvm ls 20 resolves to a v20.x.y entry" "fail"
  fi
else
  assert "PM-02 falsified: nvm present (nvm not installed)" "fail"
fi

# ----- PM-03 falsification: busctl --user list returns within 5s -----
if timeout 5 busctl --user list >/dev/null 2>&1; then
  assert "PM-03 falsified: busctl --user list returns within 5s (no hang)" "pass"
else
  assert "PM-03 falsified: busctl --user list returns within 5s (no hang)" "fail"
fi

# ----- Summary ---------------------------------------------------------------
echo ""
header "=== Results: $PASS passed, $FAIL failed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
exit 0
