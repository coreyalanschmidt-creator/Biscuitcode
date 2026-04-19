#!/usr/bin/env bash
# bootstrap-toolchain.sh — Phase 0 deliverable for BiscuitCode
#
# Installs the language toolchains (Rust, Node, pnpm) and tauri CLI required
# to build BiscuitCode. Runs AFTER scripts/bootstrap-wsl.sh has installed system
# dependencies.
#
# Usage:
#   bash scripts/bootstrap-toolchain.sh
#
# Idempotent: existing installations are detected and version-checked rather than
# overwritten. This script touches ~/.cargo, ~/.rustup, ~/.nvm, ~/.local/share/pnpm,
# and modifies ~/.bashrc / ~/.zshrc to source nvm and pnpm. Review before running
# if you have a non-standard shell setup.

set -euo pipefail

red()    { printf '\033[31m%s\033[0m\n' "$*"; }
green()  { printf '\033[32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[33m%s\033[0m\n' "$*"; }
blue()   { printf '\033[34m%s\033[0m\n' "$*"; }

# Required versions (pin to latest stable as of 2026-04-18).
RUST_MIN_VERSION="1.85"        # MSRV for Tauri 2.10.x
NODE_REQUIRED="20"             # Tauri 2.x requires >= 18; we use 20 LTS
PNPM_MIN_VERSION="9"
TAURI_CLI_VERSION="2.10.1"

blue "==> BiscuitCode toolchain bootstrap"

# ----- Rust ---------------------------------------------------------------

if command -v rustup >/dev/null 2>&1; then
  CURRENT_RUST="$(rustc --version 2>/dev/null | awk '{print $2}')"
  green "  [OK] rustup present; rustc ${CURRENT_RUST}"
  # Make sure stable is current.
  rustup update stable
else
  blue "==> Installing rustup (default profile, stable channel)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

# Verify rustc meets the MSRV.
RUSTC_VER="$(rustc --version | awk '{print $2}')"
if [[ "$(printf '%s\n' "$RUST_MIN_VERSION" "$RUSTC_VER" | sort -V | head -n1)" != "$RUST_MIN_VERSION" ]]; then
  red "ERROR: rustc ${RUSTC_VER} is older than required ${RUST_MIN_VERSION}."
  red "  Run: rustup update stable"
  exit 1
fi
green "  [OK] rustc ${RUSTC_VER} (>= ${RUST_MIN_VERSION})"

# ----- Node.js + nvm ------------------------------------------------------

if [[ -d "$HOME/.nvm" ]]; then
  green "  [OK] nvm directory present at ~/.nvm"
else
  blue "==> Installing nvm (Node Version Manager)"
  # Pinned to v0.40.x; verify checksum if you're security-conscious.
  curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
fi

# Source nvm for this script's session.
export NVM_DIR="$HOME/.nvm"
# shellcheck disable=SC1091
[[ -s "$NVM_DIR/nvm.sh" ]] && \. "$NVM_DIR/nvm.sh"

# Install/select Node 20.
if ! nvm ls "${NODE_REQUIRED}" >/dev/null 2>&1; then
  blue "==> Installing Node.js ${NODE_REQUIRED} via nvm"
  nvm install "${NODE_REQUIRED}"
fi
nvm use "${NODE_REQUIRED}"
nvm alias default "${NODE_REQUIRED}"

NODE_VER="$(node --version 2>/dev/null | sed 's/^v//')"
green "  [OK] node v${NODE_VER}"

# ----- pnpm ---------------------------------------------------------------

if command -v pnpm >/dev/null 2>&1; then
  PNPM_VER="$(pnpm --version)"
  green "  [OK] pnpm ${PNPM_VER}"
else
  blue "==> Installing pnpm via corepack"
  corepack enable
  corepack prepare "pnpm@latest-${PNPM_MIN_VERSION}" --activate
  PNPM_VER="$(pnpm --version)"
  green "  [OK] pnpm ${PNPM_VER}"
fi

# Verify pnpm meets minimum.
if [[ "$(printf '%s\n' "$PNPM_MIN_VERSION" "$PNPM_VER" | sort -V | head -n1)" != "$PNPM_MIN_VERSION" ]]; then
  red "ERROR: pnpm ${PNPM_VER} is older than required ${PNPM_MIN_VERSION}."
  red "  Run: corepack prepare pnpm@latest-${PNPM_MIN_VERSION} --activate"
  exit 1
fi

# ----- cargo-tauri-cli ----------------------------------------------------

if command -v cargo-tauri >/dev/null 2>&1; then
  TAURI_VER="$(cargo tauri --version 2>/dev/null | awk '{print $2}')"
  green "  [OK] cargo tauri ${TAURI_VER}"
else
  blue "==> Installing cargo-tauri-cli ${TAURI_CLI_VERSION}"
  cargo install tauri-cli --version "${TAURI_CLI_VERSION}" --locked
  TAURI_VER="$(cargo tauri --version | awk '{print $2}')"
  green "  [OK] cargo tauri ${TAURI_VER}"
fi

# ----- cargo audit (Phase 1+ CI) ------------------------------------------

if command -v cargo-audit >/dev/null 2>&1; then
  green "  [OK] cargo-audit present"
else
  blue "==> Installing cargo-audit (security audit for CI)"
  cargo install cargo-audit --locked
fi

# ----- Final summary ------------------------------------------------------

blue "==> Toolchain ready."
echo ""
echo "Versions installed:"
echo "  rustc:      $(rustc --version)"
echo "  cargo:      $(cargo --version)"
echo "  cargo tauri: $(cargo tauri --version)"
echo "  node:       $(node --version)"
echo "  pnpm:       $(pnpm --version)"
echo ""
echo "Next steps (from the project root, ~/biscuitcode):"
echo "  1. pnpm install"
echo "  2. pnpm tauri dev    # launches the dev window via WSLg"
echo "  3. Begin /run-phase 1 if Phase 0 acceptance criteria all pass."
echo ""
echo "If your shell does not pick up nvm or pnpm in a new terminal, ensure your"
echo "~/.bashrc (or ~/.zshrc) sources \$NVM_DIR/nvm.sh and includes ~/.local/share/pnpm"
echo "in PATH (corepack typically handles the latter)."
