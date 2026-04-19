#!/usr/bin/env bash
# bootstrap-wsl.sh — Phase 0 deliverable for BiscuitCode
#
# Idempotent install of the WSL2 + Ubuntu 24.04 system dependencies needed
# to develop and build BiscuitCode (Tauri 2.x, WebKitGTK 4.1, Linux .deb target).
#
# Usage:
#   bash scripts/bootstrap-wsl.sh
#
# Safe to re-run: apt-get install is idempotent; this script does not modify
# user dotfiles. After this completes, run scripts/bootstrap-toolchain.sh.
#
# Hard requirements (script aborts if missing):
#   - Running inside WSL2 (uname -r contains "WSL2" or "microsoft")
#   - Project working directory is under $HOME, NOT under /mnt/c/
#     (Tauri builds in /mnt/c suffer from inotify exhaustion and 10x slower IO)
#   - busctl is available (ships with systemd; pre-installed on Ubuntu 24.04)
#
# This script does NOT install Rust, Node, or pnpm — that's bootstrap-toolchain.sh.

set -euo pipefail

# ----- Pre-flight checks --------------------------------------------------

red()    { printf '\033[31m%s\033[0m\n' "$*"; }
green()  { printf '\033[32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[33m%s\033[0m\n' "$*"; }
blue()   { printf '\033[34m%s\033[0m\n' "$*"; }

blue "==> BiscuitCode WSL2 bootstrap"

# 1. Confirm we're in WSL2 (not Windows-native, not WSL1).
if ! grep -qiE 'wsl2|microsoft' /proc/version 2>/dev/null; then
  red "ERROR: This script must run inside WSL2 + Ubuntu 24.04."
  red "  /proc/version: $(cat /proc/version 2>/dev/null || echo '<unreadable>')"
  red "See docs/DEV-SETUP.md for WSL2 install instructions."
  exit 1
fi
green "  [OK] Running inside WSL2"

# 2. Confirm Ubuntu 24.04 (noble). Mint 22 base = noble; 24.04 = matching toolchain.
if ! grep -q '^VERSION_CODENAME=noble' /etc/os-release 2>/dev/null; then
  yellow "  [WARN] /etc/os-release does not report VERSION_CODENAME=noble (Ubuntu 24.04)."
  yellow "         BiscuitCode targets Mint 22 (Ubuntu 24.04 base). Other Ubuntu versions"
  yellow "         may have different webkit2gtk / libfuse2 packaging. Continuing anyway,"
  yellow "         but expect package-name surprises."
else
  green "  [OK] Ubuntu 24.04 noble detected"
fi

# 3. Confirm we are NOT under /mnt/c/ (or /mnt/<any-drive>/).
PROJECT_DIR="$(realpath "$(pwd)")"
case "$PROJECT_DIR" in
  /mnt/*)
    red "ERROR: Project is at $PROJECT_DIR — this is on the Windows filesystem."
    red "  Tauri builds in /mnt/c/ are 5-10x slower and trigger inotify-watch exhaustion."
    red "  Move the repo to ~/biscuitcode/ inside WSL and re-run from there."
    red "  Example: cp -r /mnt/c/Users/super/Documents/GitHub/BiscuitCode ~/biscuitcode"
    exit 1
    ;;
esac
green "  [OK] Project lives under \$HOME ($PROJECT_DIR)"

# 4. Confirm busctl is available (ships with systemd).
if ! command -v busctl >/dev/null 2>&1; then
  red "ERROR: busctl not found in PATH. busctl ships with systemd and should be"
  red "  pre-installed on Ubuntu 24.04. If you are on a minimal/headless WSL image,"
  red "  install systemd: sudo apt-get install -y systemd"
  exit 1
fi
green "  [OK] busctl present at $(command -v busctl)"

# ----- Apt installs -------------------------------------------------------

blue "==> Updating apt index"
sudo apt-get update -qq

blue "==> Installing Tauri 2.x build dependencies (WebKitGTK 4.1, GTK 3, libsoup 3)"
APT_PACKAGES=(
  # Build essentials
  build-essential
  pkg-config
  curl
  file
  patchelf

  # GTK 3 + WebKitGTK 4.1 (Tauri v2 Linux webview)
  libgtk-3-dev
  libwebkit2gtk-4.1-dev
  libjavascriptcoregtk-4.1-dev
  libsoup-3.0-dev

  # System integration
  libdbus-1-dev
  libssl-dev
  libayatana-appindicator3-dev
  librsvg2-dev

  # AppImage runtime (libfuse2 was renamed to libfuse2t64 on noble)
  libfuse2t64

  # Secret Service (libsecret) for the keyring crate
  gnome-keyring
  libsecret-1-0
  libsecret-tools
  libsecret-1-dev
)

# Install in one transaction so apt resolves conflicts together.
sudo apt-get install -y "${APT_PACKAGES[@]}"

# ----- Post-install verification ------------------------------------------

blue "==> Verifying installed packages"

# WebKitGTK 4.1 must be present at the version Tauri 2.x links against.
if pkg-config --exists webkit2gtk-4.1; then
  WEBKIT_VER="$(pkg-config --modversion webkit2gtk-4.1)"
  green "  [OK] webkit2gtk-4.1 ${WEBKIT_VER}"
else
  red "ERROR: webkit2gtk-4.1 pkg-config check failed after install."
  red "  Tauri v2 links against this; without it, tauri build will fail."
  exit 1
fi

# GTK 3
if pkg-config --exists gtk+-3.0; then
  GTK_VER="$(pkg-config --modversion gtk+-3.0)"
  green "  [OK] gtk+-3.0 ${GTK_VER}"
else
  red "ERROR: gtk+-3.0 pkg-config check failed."
  exit 1
fi

# libsoup 3 (replaces libsoup 2 in Tauri v2)
if pkg-config --exists libsoup-3.0; then
  green "  [OK] libsoup-3.0 $(pkg-config --modversion libsoup-3.0)"
else
  red "ERROR: libsoup-3.0 pkg-config check failed."
  exit 1
fi

# libsecret for keyring
if pkg-config --exists libsecret-1; then
  green "  [OK] libsecret-1 $(pkg-config --modversion libsecret-1)"
else
  red "ERROR: libsecret-1 pkg-config check failed."
  exit 1
fi

# Secret Service daemon presence (acceptable to be missing on headless WSL).
if busctl --user list 2>/dev/null | grep -q org.freedesktop.secrets; then
  green "  [OK] org.freedesktop.secrets is on the user DBus session"
else
  yellow "  [WARN] org.freedesktop.secrets is NOT on the user DBus right now."
  yellow "         This is normal in a headless WSL session. To activate gnome-keyring:"
  yellow "           dbus-launch --exit-with-session gnome-keyring-daemon --start --components=secrets"
  yellow "         BiscuitCode's onboarding will block API-key entry until this is reachable."
fi

# AppImage runtime sanity
if dpkg -s libfuse2t64 >/dev/null 2>&1; then
  green "  [OK] libfuse2t64 installed (AppImage runtime)"
else
  yellow "  [WARN] libfuse2t64 missing — AppImages will refuse to launch."
fi

# ----- Done ---------------------------------------------------------------

blue "==> WSL2 system dependencies installed."
echo ""
echo "Next steps:"
echo "  1. Run scripts/bootstrap-toolchain.sh to install Rust, Node, pnpm, cargo-tauri-cli."
echo "  2. Then: cd ${PROJECT_DIR} && pnpm install"
echo "  3. Then: pnpm tauri dev   (opens a WSLg window)"
echo ""
echo "If anything failed above, fix the specific failure before re-running this script."
