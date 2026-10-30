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
if [[ ! -x .tools/gpu-speech/bin/python ]]; then .tools/bootstrap/bin/uv venv --python 3.11 .tools/gpu-speech; fi
.tools/bootstrap/bin/uv pip install --python .tools/gpu-speech/bin/python -r scripts/gpu-speech-requirements.txt
./scripts/gpu-speech.sh --check
printf '\nReference speech profile installed. Run ./play.sh.\n'
