#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
if [[ -f "$HOME/.cargo/env" ]]; then source "$HOME/.cargo/env"; fi
if ! command -v cargo >/dev/null; then echo 'Rust is required. Install rustup from https://rustup.rs.' >&2; exit 1; fi
if [[ -f .env ]]; then set -a; source .env; set +a; fi
border_log_session="${BORDER_CALL_LOG_DIR:-$PWD/output/call-logs/$(date -u +%Y%m%dT%H%M%SZ)-$$}"
mkdir -p output "$border_log_session"
export BORDER_CALL_LOG_DIR="$border_log_session"
printf '%s\n' "$border_log_session" > output/latest-call-log-dir.txt
echo 'Conversation diagnostics: python3 scripts/call_logs.py --follow'
border_join=false
border_join_url=""
border_game_args=()
for border_arg in "$@"; do
  case "$border_arg" in
    --join) border_join=true ;;
    --join=*) border_join=true; border_join_url="${border_arg#--join=}" ;;
    --seed=*) export BORDER_HOST_SEED="${border_arg#--seed=}"; border_game_args+=("$border_arg") ;;
    *) border_game_args+=("$border_arg") ;;
  esac
done
cargo build --locked -p border-caller-server -p border-game
mkdir -p output
border_server_pid=""
border_host_pid=""
border_speech_pid=""
cleanup() {
  for border_pid in "$border_speech_pid" "$border_host_pid" "$border_server_pid"; do
    if [[ -n "$border_pid" ]]; then kill "$border_pid" 2>/dev/null || true; wait "$border_pid" 2>/dev/null || true; fi
  done
}
trap cleanup EXIT INT TERM
if [[ "$border_join" == true ]]; then
  if [[ -z "$border_join_url" ]]; then
    if [[ ! -f output/local-session.json ]]; then echo 'Start ./play.sh first, or use --join=http://HOST:8090 with BORDER_HOST_JOIN_CODE.' >&2; exit 1; fi
    export BORDER_HOST_URL="$(python3 -c 'import json;print(json.load(open("output/local-session.json"))["url"])')"
    export BORDER_HOST_JOIN_CODE="$(python3 -c 'import json;print(json.load(open("output/local-session.json"))["join_code"])')"
    if [[ -z "${BORDER_SPEECH_URL:-}" ]]; then
      export BORDER_SPEECH_URL="$(python3 -c 'import json;print(json.load(open("output/local-session.json")).get("speech_url",""))')"
      export BORDER_SPEECH_TOKEN="$(python3 -c 'import json;print(json.load(open("output/local-session.json")).get("speech_token",""))')"
    fi
  else
    export BORDER_HOST_URL="$border_join_url"
    : "${BORDER_HOST_JOIN_CODE:?Set BORDER_HOST_JOIN_CODE to the host invite code}"
  fi
else
  export BORDER_HOST_MANUAL_CLOCK=0
  export BORDER_SESSION_TOKEN="$(python3 -c 'import secrets;print(secrets.token_urlsafe(32))')"
  export BORDER_HOST_OPERATOR_TOKEN="$(python3 -c 'import secrets;print(secrets.token_urlsafe(32))')"
  export BORDER_HOST_GATEWAY_TOKEN="$(python3 -c 'import secrets;print(secrets.token_urlsafe(32))')"
  export BORDER_HOST_JOIN_CODE="${BORDER_HOST_JOIN_CODE:-$(python3 -c 'import secrets;print(secrets.token_hex(6))')}"
  export BORDER_SERVER_ADDR="127.0.0.1:${BORDER_PORT:-3847}"
  export BORDER_GATEWAY_URL="http://$BORDER_SERVER_ADDR"
  export BORDER_HOST_URL="http://127.0.0.1:${BORDER_HOST_PORT:-8090}"
  export BORDER_HOST_BIND="${BORDER_HOST_BIND:-127.0.0.1:${BORDER_HOST_PORT:-8090}}"
  ./target/debug/border-caller-server >output/caller-server.log 2>&1 &
  border_server_pid=$!
  ./target/debug/border-host >output/host.log 2>&1 &
  border_host_pid=$!
  for _ in {1..100}; do
    if ! kill -0 "$border_server_pid" 2>/dev/null; then cat output/caller-server.log >&2; exit 1; fi
    if ! kill -0 "$border_host_pid" 2>/dev/null; then cat output/host.log >&2; exit 1; fi
    if curl -fsS "$BORDER_HOST_URL/health" >/dev/null 2>&1 && curl -fsS "$BORDER_GATEWAY_URL/health" >/dev/null 2>&1; then break; fi
    sleep 0.1
  done
  if ! curl -fsS "$BORDER_HOST_URL/health" >/dev/null; then echo 'Shift host failed to start.' >&2; exit 1; fi
  echo "San Ignacio host ready. Local second player: ./play.sh --join --name=Officer-2"
  echo "Invite code: $BORDER_HOST_JOIN_CODE"
fi
if [[ -x .tools/whisper/build/bin/whisper-cli ]]; then export WHISPER_BIN="$PWD/.tools/whisper/build/bin/whisper-cli"; fi
if [[ -f .tools/whisper/models/ggml-tiny.en.bin ]]; then export WHISPER_MODEL="$PWD/.tools/whisper/models/ggml-tiny.en.bin"; fi
if [[ -x .tools/voice-env/bin/piper ]]; then export PIPER_BIN="${PIPER_BIN:-$PWD/.tools/voice-env/bin/piper}"; fi
if [[ -f .tools/voices/en_US-ryan-medium.onnx ]]; then export PIPER_MODEL="${PIPER_MODEL:-$PWD/.tools/voices/en_US-ryan-medium.onnx}"; fi
if [[ -f .tools/voices/en_US-lessac-medium.onnx ]]; then export PIPER_MODEL_ALTERNATE="${PIPER_MODEL_ALTERNATE:-$PWD/.tools/voices/en_US-lessac-medium.onnx}"; fi
if [[ "${BORDER_SPEECH_PROFILE:-reference}" != lightweight && -f .tools/gpu-speech-ready.json && -z "${BORDER_SPEECH_URL:-}" ]]; then
  export BORDER_SPEECH_TOKEN="$(python3 -c 'import secrets;print(secrets.token_urlsafe(32))')"
  border_voice_port="${BORDER_SPEECH_PORT:-3848}"
  if [[ "$border_join" == true ]]; then border_voice_port="${BORDER_SPEECH_PORT:-3849}"; fi
  export BORDER_SPEECH_URL="http://127.0.0.1:$border_voice_port"
  echo 'Loading resident GPU recognition and voice models…'
  ./scripts/gpu-speech.sh --port "$border_voice_port" >output/speech-service.log 2>&1 &
  border_speech_pid=$!
  border_speech_ready=false
  for _ in {1..900}; do
    if ! kill -0 "$border_speech_pid" 2>/dev/null; then cat output/speech-service.log >&2; exit 1; fi
    if curl -fsS "$BORDER_SPEECH_URL/health" >/dev/null 2>&1; then border_speech_ready=true; break; fi
    sleep 0.2
  done
  if [[ "$border_speech_ready" != true ]]; then echo 'GPU speech failed to become ready.' >&2; exit 1; fi
fi
if [[ -z "${BORDER_SPEECH_URL:-}" ]]; then unset BORDER_SPEECH_URL BORDER_SPEECH_TOKEN; fi
if [[ "$border_join" == false ]]; then
  python3 - <<'PY'
import json,os
from pathlib import Path
p=Path('output/local-session.json')
p.write_text(json.dumps({'url':os.environ['BORDER_HOST_URL'],'join_code':os.environ['BORDER_HOST_JOIN_CODE'],'speech_url':os.getenv('BORDER_SPEECH_URL',''),'speech_token':os.getenv('BORDER_SPEECH_TOKEN','')}))
p.chmod(0o600)
PY
fi
# Provider/operator credentials belong to services, never to a game client.
env -u OPENROUTER_API_KEY -u OPENROUTER_MODEL -u BORDER_SESSION_TOKEN -u BORDER_HOST_OPERATOR_TOKEN -u BORDER_HOST_GATEWAY_TOKEN ./target/debug/border-game "${border_game_args[@]}"
