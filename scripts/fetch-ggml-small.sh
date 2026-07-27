#!/usr/bin/env bash
# Download + SHA256-verify ggml-small.bin into Luozi Application Support (M4).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="${ROOT}/src-tauri/resources/models-manifest.json"

MODEL_NAME="ggml-small.bin"
URL="${LUOZI_MODEL_URL:-https://huggingface.co/ggerganov/whisper.cpp/resolve/main/${MODEL_NAME}}"
EXPECTED_SHA256="${LUOZI_MODEL_SHA256:-1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b}"
EXPECTED_BYTES="${LUOZI_MODEL_BYTES:-487601967}"

if [[ -f "$MANIFEST" ]] && command -v python3 >/dev/null 2>&1; then
  _url="$(python3 -c 'import json,sys; m=json.load(open(sys.argv[1])); r=next((x for x in m["models"] if x.get("recommended")), m["models"][0]); print(r["url"])' "$MANIFEST")"
  _sha="$(python3 -c 'import json,sys; m=json.load(open(sys.argv[1])); r=next((x for x in m["models"] if x.get("recommended")), m["models"][0]); print(r["sha256"])' "$MANIFEST")"
  _bytes="$(python3 -c 'import json,sys; m=json.load(open(sys.argv[1])); r=next((x for x in m["models"] if x.get("recommended")), m["models"][0]); print(r["bytes"])' "$MANIFEST")"
  _name="$(python3 -c 'import json,sys; m=json.load(open(sys.argv[1])); r=next((x for x in m["models"] if x.get("recommended")), m["models"][0]); print(r["fileName"])' "$MANIFEST")"
  URL="${LUOZI_MODEL_URL:-$_url}"
  EXPECTED_SHA256="${LUOZI_MODEL_SHA256:-$_sha}"
  EXPECTED_BYTES="${LUOZI_MODEL_BYTES:-$_bytes}"
  MODEL_NAME="$_name"
fi

DEST_DIR="${HOME}/Library/Application Support/app.luozi.desktop/models"
DEST="${DEST_DIR}/${MODEL_NAME}"

mkdir -p "$DEST_DIR"

sha256_of() {
  local f="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$f" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" | awk '{print $1}'
  else
    echo "luozi: need shasum or sha256sum" >&2
    exit 1
  fi
}

verify_dest() {
  local got size
  got="$(sha256_of "$DEST")"
  if [[ "${got}" != "${EXPECTED_SHA256}" ]]; then
    echo "luozi: checksum failed: got ${got} expected ${EXPECTED_SHA256}" >&2
    return 1
  fi
  if [[ -n "${EXPECTED_BYTES}" ]]; then
    size="$(wc -c <"$DEST" | tr -d ' ')"
    if [[ "$size" != "$EXPECTED_BYTES" ]]; then
      echo "luozi: size mismatch: got ${size} expected ${EXPECTED_BYTES}" >&2
      return 1
    fi
  fi
  return 0
}

if [[ -f "$DEST" && "${LUOZI_FORCE_MODEL:-0}" != "1" ]]; then
  if verify_dest; then
    echo "luozi: model already present and verified: $DEST"
    ls -lh "$DEST"
    exit 0
  fi
  echo "luozi: existing file failed verify; re-downloading…"
  rm -f "$DEST"
fi

TMP="${DEST}.partial"
rm -f "$TMP"
echo "luozi: downloading ${MODEL_NAME} → ${DEST}"
echo "luozi: from ${URL}"

if command -v curl >/dev/null 2>&1; then
  curl -L --fail --retry 3 --retry-delay 2 -o "$TMP" "$URL"
elif command -v wget >/dev/null 2>&1; then
  wget -O "$TMP" "$URL"
else
  echo "luozi: need curl or wget" >&2
  exit 1
fi

mv "$TMP" "$DEST"
verify_dest
ls -lh "$DEST"
echo "luozi: done. Restart Luozi (or use tray 「下载推荐模型…」) then hold Control+Alt+Space."
