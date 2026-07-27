#!/usr/bin/env bash
# Install Luozi.app into a real Applications folder for TCC / Accessibility pickers.
# Dev builds under target/... are invisible when System Settings opens /Applications.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

APP_CANDIDATES=(
  "$ROOT/target/release/bundle/macos/Luozi.app"
  "$ROOT/src-tauri/target/release/bundle/macos/Luozi.app"
  "$ROOT/target/debug/bundle/macos/Luozi.app"
  "$ROOT/src-tauri/target/debug/bundle/macos/Luozi.app"
)

find_app() {
  local p
  for p in "${APP_CANDIDATES[@]}"; do
    if [[ -d "$p" ]]; then
      printf '%s\n' "$p"
      return 0
    fi
  done
  return 1
}

SRC=""
FORCE_BUILD="${LUOZI_FORCE_BUILD:-0}"

if [[ "$FORCE_BUILD" == "1" ]] || ! SRC="$(find_app)"; then
  echo "luozi: building macOS bundle…"
  export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
  # shellcheck disable=SC1091
  [[ -s "$NVM_DIR/nvm.sh" ]] && . "$NVM_DIR/nvm.sh"
  npm run tauri -- build
  if ! SRC="$(find_app)"; then
    echo "luozi: Luozi.app not found after build" >&2
    exit 1
  fi
fi

# Prefer /Applications (what System Settings「应用程序」lists). Fallback: ~/Applications.
if [[ -n "${LUOZI_INSTALL_DIR:-}" ]]; then
  DEST_DIR="$LUOZI_INSTALL_DIR"
elif [[ -w /Applications ]] || [[ "$(id -u)" -eq 0 ]]; then
  DEST_DIR="/Applications"
else
  # Try /Applications with ditto; if permission denied, use ~/Applications.
  DEST_DIR="/Applications"
fi

DEST="$DEST_DIR/Luozi.app"

install_to() {
  local dir="$1"
  local dest="$dir/Luozi.app"
  mkdir -p "$dir"
  rm -rf "$dest"
  ditto "$SRC" "$dest"
  # Keep adhoc signature consistent for the installed copy.
  codesign --force --deep --sign - "$dest" >/dev/null 2>&1 || true
  printf '%s\n' "$dest"
}

echo "luozi: source  $SRC"
if DEST_PATH="$(install_to "$DEST_DIR" 2>/dev/null)"; then
  :
else
  echo "luozi: cannot write $DEST_DIR — installing to ~/Applications instead"
  DEST_PATH="$(install_to "$HOME/Applications")"
fi

echo "luozi: installed $DEST_PATH"
echo "luozi: for Accessibility, pick Luozi from Applications (path: $DEST_PATH)"
echo "luozi: after each reinstall, toggle Accessibility off/on (adhoc signature changes)"

if [[ "${LUOZI_OPEN:-1}" == "1" ]]; then
  pkill -f 'Luozi.app/Contents/MacOS/luozi' 2>/dev/null || true
  sleep 0.3
  open "$DEST_PATH"
fi
