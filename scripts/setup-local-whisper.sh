#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODEL_DIR="$ROOT_DIR/backend/models"
MODEL_PATH="$MODEL_DIR/ggml-tiny.en.bin"
MODEL_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"

if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew is required for this setup script on macOS." >&2
  exit 1
fi

if ! command -v whisper-cli >/dev/null 2>&1 && ! command -v main >/dev/null 2>&1; then
  brew install whisper-cpp
fi

if ! command -v ffmpeg >/dev/null 2>&1; then
  brew install ffmpeg
fi

mkdir -p "$MODEL_DIR"
if [ ! -f "$MODEL_PATH" ]; then
  curl -L "$MODEL_URL" -o "$MODEL_PATH"
fi

echo "Local Whisper setup complete."
echo "Model: $MODEL_PATH"
if command -v whisper-cli >/dev/null 2>&1; then
  echo "Binary: $(command -v whisper-cli)"
elif command -v main >/dev/null 2>&1; then
  echo "Binary: $(command -v main)"
fi
echo
echo "The backend auto-detects backend/models/ggml-tiny.en.bin when whisper.cpp is on PATH."
