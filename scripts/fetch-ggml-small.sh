#!/usr/bin/env bash
# Download ggml-small.bin into the Luozi Application Support models dir (M3 manual placement helper).
set -euo pipefail

MODEL_NAME="ggml-small.bin"
# Official whisper.cpp GGML host (Hugging Face mirror via hf-mirror optional).
URL="${LUOZI_MODEL_URL:-https://huggingface.co/ggerganov/whisper.cpp/resolve/main/${MODEL_NAME}}"

DEST_DIR="${HOME}/Library/Application Support/app.luozi.desktop/models"
DEST="${DEST_DIR}/${MODEL_NAME}"

mkdir -p "$DEST_DIR"

if [[ -f "$DEST" && "${LUOZI_FORCE_MODEL:-0}" != "1" ]]; then
  echo "luozi: model already present: $DEST"
  ls -lh "$DEST"
  exit 0
fi

TMP="${DEST}.partial"
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
ls -lh "$DEST"
echo "luozi: done. Restart Luozi and hold Control+Alt+Space to transcribe."
