#!/usr/bin/env bash
# Install the native offline STT backend and a checksum-verified English model.
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This helper is for macOS. Install whisper-cli and set OVERSEER_WHISPER_MODEL to a GGML model on other platforms." >&2
  exit 1
fi

voice_brew="$(command -v brew || true)"
if [[ -z "$voice_brew" ]]; then
  for candidate in /opt/homebrew/bin/brew /usr/local/bin/brew; do
    if [[ -x "$candidate" ]]; then voice_brew="$candidate"; break; fi
  done
fi
if [[ -z "$voice_brew" ]]; then
  echo "Homebrew is required: https://brew.sh" >&2
  exit 1
fi
voice_prefix="$("$voice_brew" --prefix)"
if [[ ! -x "$voice_prefix/bin/whisper-cli" ]]; then
  "$voice_brew" install whisper.cpp
fi

voice_data_dir="${OVERSEER_DATA_DIR:-$HOME/Library/Application Support/overseer}"
voice_model="${OVERSEER_WHISPER_MODEL:-$voice_data_dir/assets/voice/models/ggml-tiny.en.bin}"
voice_sha="921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f"
voice_url="https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-tiny.en.bin"
if [[ -f "$voice_model" ]] && echo "$voice_sha  $voice_model" | shasum -a 256 -c - >/dev/null 2>&1; then
  echo "Voice model is already installed and verified: $voice_model"
else
  mkdir -p "$(dirname "$voice_model")"
  voice_download="$(mktemp "${voice_model}.download.XXXXXX")"
  trap 'rm -f "$voice_download"' EXIT
  curl --fail --location --retry 3 --output "$voice_download" "$voice_url"
  echo "$voice_sha  $voice_download" | shasum -a 256 -c -
  mv "$voice_download" "$voice_model"
  trap - EXIT
fi

echo "Offline voice is ready. Restart Overseer to use the rebuilt daemon."
echo "Engine: $voice_prefix/bin/whisper-cli"
echo "Model: $voice_model"
