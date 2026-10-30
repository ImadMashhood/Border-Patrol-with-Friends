#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
export HF_HOME="$PWD/.tools/huggingface"
export HF_HUB_DISABLE_XET=1
export OMP_NUM_THREADS=4
border_cuda_libs="$(.tools/gpu-speech/bin/python -c 'import site,glob,os; print(":".join(p for root in site.getsitepackages() for p in glob.glob(os.path.join(root,"nvidia","*","lib"))))')"
export LD_LIBRARY_PATH="$border_cuda_libs${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
if [[ "${1:-}" == --benchmark ]]; then
  shift
  exec env -u OPENROUTER_API_KEY -u OPENROUTER_MODEL .tools/gpu-speech/bin/python scripts/benchmark_speech.py "$@"
fi
exec env -u OPENROUTER_API_KEY -u OPENROUTER_MODEL .tools/gpu-speech/bin/python scripts/speech_service.py "$@"
