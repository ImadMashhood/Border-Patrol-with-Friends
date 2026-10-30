#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
# Local opt-in tools. No sudo or global Python package changes.
if [[ -x .tools/whisper/build/bin/whisper-cli && -f .tools/whisper/models/ggml-tiny.en.bin ]]; then
  echo "Local voice recognition is already installed. Run ./play.sh."
  exit 0
fi
if ! command -v "${CXX:-g++}" >/dev/null; then
  echo 'A C++ compiler is required. On Fedora install gcc-c++ make, then rerun this script.' >&2
  exit 1
fi
mkdir -p .tools
if [[ ! -d .tools/whisper/.git ]]; then git clone --depth 1 --branch v1.8.3 https://github.com/ggml-org/whisper.cpp.git .tools/whisper; fi
python3 -m venv .tools/voice-env
.tools/voice-env/bin/pip install 'cmake==4.2.3'
.tools/voice-env/bin/cmake -S .tools/whisper -B .tools/whisper/build -DCMAKE_BUILD_TYPE=Release -DGGML_NATIVE=OFF -DWHISPER_BUILD_TESTS=OFF
.tools/voice-env/bin/cmake --build .tools/whisper/build --config Release -j 4
bash .tools/whisper/models/download-ggml-model.sh tiny.en
printf '\nVoice recognition installed locally. Run ./play.sh; press E on the car, then speak normally.\n'
