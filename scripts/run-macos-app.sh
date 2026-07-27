#!/usr/bin/env bash
# Build (if needed), install into Applications, then open — stable TCC identity.
# Prefer: npm run install:macos-app
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export LUOZI_FORCE_BUILD="${LUOZI_FORCE_BUILD:-0}"
export LUOZI_OPEN=1
exec bash "$ROOT/scripts/install-macos-app.sh"
