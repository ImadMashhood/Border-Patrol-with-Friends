#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
if [[ "${BORDER_SPEECH_PROFILE:-reference}" == lightweight ]]; then
  exec ./scripts/setup-lightweight-voice.sh
fi
exec ./scripts/setup-gpu-speech.sh
