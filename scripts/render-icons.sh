#!/usr/bin/env bash
# scripts/render-icons.sh
#
# Phase 8 deliverable: render all BiscuitCode icon sizes from the master SVG.
#
# Requires: librsvg2-bin (for rsvg-convert), imagemagick (for .ico assembly).
#   sudo apt install librsvg2-bin imagemagick
#
# Usage:
#   bash scripts/render-icons.sh
#
# Output: packaging/icons/biscuitcode-{16,32,48,64,128,256,512}.png
#         src-tauri/icons/{32x32,128x128,128x128@2x}.png
#         src-tauri/icons/icon.ico
#
# Special note for 16px:
#   The 16x16 render uses the hand-tuned 16px SVG inline in the reference HTML
#   (stroke-width 72, corner radius 96) rather than downscaling the master.
#   See: packaging/icons/biscuitcode-icon-concepts.html
#   The reference HTML's 16px-optimised SVG is extracted below.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ICONS_DIR="$REPO_ROOT/packaging/icons"
TAURI_ICONS="$REPO_ROOT/src-tauri/icons"
MASTER_SVG="$ICONS_DIR/biscuitcode.svg"

# 16px hand-tuned variant (stroke-width 72, radius 96 per the reference HTML).
ICON_16_SVG="$ICONS_DIR/biscuitcode-16.svg"

cat > "$ICON_16_SVG" << 'SVG16'
<svg xmlns="http://www.w3.org/2000/svg"
     width="16" height="16"
     viewBox="0 0 512 512"
     role="img" aria-label="BiscuitCode">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#241A13"/>
      <stop offset="100%" stop-color="#0E0906"/>
    </linearGradient>
  </defs>
  <rect width="512" height="512" rx="96" fill="url(#bg)"/>
  <rect x="3" y="3" width="506" height="506" rx="93"
        fill="none" stroke="#3A2F24" stroke-width="2" opacity="0.7"/>
  <polyline points="148,148 256,256 148,364"
            fill="none"
            stroke="#E8B04C"
            stroke-width="72"
            stroke-linecap="round"
            stroke-linejoin="round"/>
  <rect x="280" y="328" width="88" height="28" rx="14" fill="#E8B04C"/>
</svg>
SVG16

echo "Rendering icons from SVG master..."

# Check tools.
if ! command -v rsvg-convert &>/dev/null; then
  echo "ERROR: rsvg-convert not found. Install: sudo apt install librsvg2-bin"
  exit 1
fi

if ! command -v convert &>/dev/null; then
  echo "ERROR: convert (ImageMagick) not found. Install: sudo apt install imagemagick"
  exit 1
fi

mkdir -p "$ICONS_DIR" "$TAURI_ICONS"

# Render standard sizes from master.
for SIZE in 32 48 64 128 256 512; do
  echo "  Rendering ${SIZE}x${SIZE}..."
  rsvg-convert -w "$SIZE" -h "$SIZE" "$MASTER_SVG" -o "$ICONS_DIR/biscuitcode-${SIZE}.png"
done

# Render 16px from the hand-tuned SVG.
echo "  Rendering 16x16 (hand-tuned)..."
rsvg-convert -w 16 -h 16 "$ICON_16_SVG" -o "$ICONS_DIR/biscuitcode-16.png"

# Copy into Tauri icons directory.
cp "$ICONS_DIR/biscuitcode-32.png"  "$TAURI_ICONS/32x32.png"
cp "$ICONS_DIR/biscuitcode-128.png" "$TAURI_ICONS/128x128.png"
cp "$ICONS_DIR/biscuitcode-256.png" "$TAURI_ICONS/128x128@2x.png"

# Assemble .ico (Windows future / cross-platform).
echo "  Assembling icon.ico..."
convert \
  "$ICONS_DIR/biscuitcode-16.png" \
  "$ICONS_DIR/biscuitcode-32.png" \
  "$ICONS_DIR/biscuitcode-48.png" \
  "$ICONS_DIR/biscuitcode-256.png" \
  "$TAURI_ICONS/icon.ico"

echo "Done. Icons in $ICONS_DIR and $TAURI_ICONS"
echo ""
echo "16x16 legibility check: inspect $ICONS_DIR/biscuitcode-16.png in an image viewer."
echo "If the >_ glyph is not legible, switch to Concept C from the reference HTML."

# Clean up temp SVG.
rm "$ICON_16_SVG"
