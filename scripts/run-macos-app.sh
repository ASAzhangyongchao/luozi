#!/usr/bin/env bash
# Build (if needed) and open the macOS Luozi.app bundle for stable TCC testing.
# Accessibility / Microphone should be granted to this Luozi.app — not to target/debug/luozi.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

APP_CANDIDATES=(
  "$ROOT/src-tauri/target/release/bundle/macos/Luozi.app"
  "$ROOT/target/release/bundle/macos/Luozi.app"
  "$ROOT/src-tauri/target/debug/bundle/macos/Luozi.app"
  "$ROOT/target/debug/bundle/macos/Luozi.app"
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

APP=""
FORCE_BUILD="${LUOZI_FORCE_BUILD:-0}"

if [[ "$FORCE_BUILD" != "1" ]] && APP="$(find_app)"; then
  echo "luozi: reusing existing bundle: $APP"
else
  echo "luozi: building macOS bundle (npm run tauri -- build)…"
  export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
  # shellcheck disable=SC1091
  [[ -s "$NVM_DIR/nvm.sh" ]] && . "$NVM_DIR/nvm.sh"
  npm run tauri -- build
  if ! APP="$(find_app)"; then
    echo "luozi: Luozi.app not found after build. Checked:" >&2
    printf '  %s\n' "${APP_CANDIDATES[@]}" >&2
    exit 1
  fi
fi

echo "luozi: opening $APP"
open "$APP"
