#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
if [[ -f "$HOME/.cargo/env" ]]; then source "$HOME/.cargo/env"; fi
if ! command -v cargo >/dev/null; then
  echo 'Rust is required. Install rustup from https://rustup.rs and run ./play.sh again.' >&2
  exit 1
fi
# .env is a local shell file; never committed or passed to the game process.
if [[ -f .env ]]; then set -a; source .env; set +a; fi
export BORDER_SESSION_TOKEN="$(python3 -c 'import secrets;print(secrets.token_urlsafe(32))')"
export BORDER_SERVER_ADDR="127.0.0.1:${BORDER_PORT:-3847}"
export BORDER_SERVER_URL="http://$BORDER_SERVER_ADDR"
if [[ -x .tools/whisper/build/bin/whisper-cli ]]; then export WHISPER_BIN="$PWD/.tools/whisper/build/bin/whisper-cli"; fi
if [[ -f .tools/whisper/models/ggml-tiny.en.bin ]]; then export WHISPER_MODEL="$PWD/.tools/whisper/models/ggml-tiny.en.bin"; fi
if [[ -x .tools/voice-env/bin/piper ]]; then export PIPER_BIN="${PIPER_BIN:-$PWD/.tools/voice-env/bin/piper}"; fi
if [[ -f .tools/voices/en_US-ryan-medium.onnx ]]; then export PIPER_MODEL="${PIPER_MODEL:-$PWD/.tools/voices/en_US-ryan-medium.onnx}"; fi
if [[ -f .tools/voices/en_US-lessac-medium.onnx ]]; then export PIPER_MODEL_ALTERNATE="${PIPER_MODEL_ALTERNATE:-$PWD/.tools/voices/en_US-lessac-medium.onnx}"; fi
cargo build --locked -p border-caller-server -p border-game
mkdir -p output
./target/debug/border-caller-server >output/caller-server.log 2>&1 &
border_server_pid=$!
border_speech_pid=""
cleanup() {
  if [[ -n "$border_speech_pid" ]]; then kill "$border_speech_pid" 2>/dev/null || true; wait "$border_speech_pid" 2>/dev/null || true; fi
  kill "$border_server_pid" 2>/dev/null || true
  wait "$border_server_pid" 2>/dev/null || true
}
trap cleanup EXIT INT TERM
for _ in {1..100}; do
  if ! kill -0 "$border_server_pid" 2>/dev/null; then cat output/caller-server.log >&2; exit 1; fi
  if curl -fsS "$BORDER_SERVER_URL/health" >/dev/null 2>&1; then break; fi
  sleep 0.1
done
if ! kill -0 "$border_server_pid" 2>/dev/null; then cat output/caller-server.log >&2; exit 1; fi
if [[ "${BORDER_SPEECH_PROFILE:-reference}" != lightweight && -f .tools/gpu-speech-ready.json ]]; then
  export BORDER_SPEECH_TOKEN="$(python3 -c 'import secrets;print(secrets.token_urlsafe(32))')"
  export BORDER_SPEECH_URL="http://127.0.0.1:${BORDER_SPEECH_PORT:-3848}"
  echo 'Loading resident GPU recognition and voice models…'
  ./scripts/gpu-speech.sh --port "${BORDER_SPEECH_PORT:-3848}" >output/speech-service.log 2>&1 &
  border_speech_pid=$!
  border_speech_ready=false
  for _ in {1..900}; do
    if ! kill -0 "$border_speech_pid" 2>/dev/null; then cat output/speech-service.log >&2; exit 1; fi
    if curl -fsS "$BORDER_SPEECH_URL/health" >/dev/null 2>&1; then border_speech_ready=true; break; fi
    sleep 0.2
  done
  if [[ "$border_speech_ready" != true ]]; then echo 'GPU speech failed to become ready; see output/speech-service.log' >&2; exit 1; fi
fi
# Paid-provider credentials are explicitly removed from the game environment.
env -u OPENROUTER_API_KEY -u OPENROUTER_MODEL ./target/debug/border-game "$@"
