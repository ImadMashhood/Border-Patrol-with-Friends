#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
mkdir -p .tools/voices
if [[ ! -x .tools/voice-env/bin/python ]]; then python3 -m venv .tools/voice-env; fi
.tools/voice-env/bin/pip install 'piper-tts==1.8.0'
.tools/voice-env/bin/python -m piper.download_voices --download-dir .tools/voices en_US-ryan-medium en_US-lessac-medium
printf '\nTwo local neural voices are ready. The launcher selects them automatically.\n'
