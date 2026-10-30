#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
if ! command -v nvidia-smi >/dev/null; then
  echo 'The reference GPU speech profile needs NVIDIA CUDA. Use BORDER_SPEECH_PROFILE=lightweight ./scripts/setup-voice.sh for CPU speech.' >&2
  exit 1
fi
mkdir -p .tools
if [[ ! -x .tools/bootstrap/bin/python ]]; then python3 -m venv .tools/bootstrap; fi
.tools/bootstrap/bin/pip install 'uv==0.13.6'
export UV_PYTHON_INSTALL_DIR="$PWD/.tools/python"
.tools/bootstrap/bin/uv python install 3.11
border_speech_env=.tools/expressive-speech
border_speech_requirements=scripts/expressive-speech-requirements.txt
if [[ "${BORDER_TTS_ENGINE:-qwen}" == kokoro ]]; then border_speech_env=.tools/gpu-speech; border_speech_requirements=scripts/gpu-speech-requirements.txt; fi
if [[ ! -x "$border_speech_env/bin/python" ]]; then .tools/bootstrap/bin/uv venv --python 3.11 "$border_speech_env"; fi
.tools/bootstrap/bin/uv pip install --python "$border_speech_env/bin/python" -r "$border_speech_requirements"
./scripts/gpu-speech.sh --check
printf '\nSpeech profile installed. Run ./play.sh.\n'
