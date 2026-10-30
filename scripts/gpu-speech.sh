#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
export HF_HOME="$PWD/.tools/huggingface"
export HF_HUB_DISABLE_XET=1
export OMP_NUM_THREADS=4
border_speech_python=.tools/pocket-speech/bin/python
if [[ "${BORDER_TTS_ENGINE:-pocket}" == qwen ]]; then border_speech_python=.tools/expressive-speech/bin/python; fi
if [[ "${BORDER_TTS_ENGINE:-pocket}" == kokoro ]]; then border_speech_python=.tools/gpu-speech/bin/python; fi
if [[ ! -x "$border_speech_python" ]]; then echo 'Run ./scripts/setup-voice.sh to install the selected speech profile.' >&2; exit 1; fi
border_cuda_libs="$("$border_speech_python" -c 'import site,glob,os; print(":".join(p for root in site.getsitepackages() for p in glob.glob(os.path.join(root,"nvidia","*","lib"))))')"
export LD_LIBRARY_PATH="$border_cuda_libs${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
if [[ "${1:-}" == --benchmark ]]; then
  shift
  exec env -u OPENROUTER_API_KEY -u OPENROUTER_MODEL "$border_speech_python" scripts/benchmark_speech.py "$@"
fi
exec env -u OPENROUTER_API_KEY -u OPENROUTER_MODEL "$border_speech_python" scripts/speech_service.py "$@"
