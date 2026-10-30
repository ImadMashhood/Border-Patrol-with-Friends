# Conversation diagnostics and provider cooldowns

Launch with `./play.sh`, then use another terminal in the repository:

```bash
python3 scripts/call_logs.py --follow
```

Without `--follow`, the viewer prints the current session once. `--json` prints JSONL for analysis. `--directory /absolute/path` selects an older session. The latest launch directory is recorded in ignored `output/latest-call-log-dir.txt`; actual files live under `output/call-logs/<UTC-start-time>-<launcher-pid>/`. Each process writes a separate `calls-<pid>.jsonl`, so game, gateway, host and local speech diagnostics can be merged by timestamp. Set `BORDER_CALL_LOG_DIR` to override the destination.

The tablet's conversation page retains the officer/traveler transcript and now displays the last reply error and retry countdown. A successful reply clears that error. This adds diagnostics without opening an automatic conversation window or preventing movement.

## What the log records

- Recognized words and whether the game accepted or discarded that transcript.
- Requests with correlation ID, session ID and officer utterance; validated traveler replies and emotions.
- HTTP status, sanitized failure message, provider rate limits and retry interval.
- Provider repair/validation stages, request latency and token usage.
- TTS model, generated text, synthesis time, audio duration, playback completion and speech errors.

Raw microphone recordings, authorization headers, API keys, system prompts and raw provider response bodies are not recorded. Dialogue logs are local, ignored by Git, created with owner-only file permissions on Unix and rotated after 10 MiB per process (one previous file retained). They contain the player's transcribed words and the NPC's spoken replies; delete an old session directory to remove it.

## The “Excuse me?” loop

The existing runtime log showed repeated upstream **HTTP 429 rate limits**. The host converted every gateway failure into an HTTP 200 pardon, hiding the actual error from the client. The host now preserves the failure status and retry metadata while retaining the neutral pardon in authoritative conversation history. The game can still play the cached “Excuse me?” once for a failed attempt.

For a 429, submissions and microphone capture processing pause for the provider's remaining retry period (bounded to 120 seconds). Other request failures use a short five-second cooldown. The next question must be a new player submission: no automatic replay or request flood. The officer can continue walking and using the tablet. A later player attempt may still fail if the external provider remains unavailable; this change does not raise provider/account quotas.

Use the logged stage to distinguish `provider_rate_limit` / HTTP 429 from `provider_validation_failed`, `microphone_error`, `stt`, `tts` or `speech_error`. Generation and playback timestamps show whether a delay is the LLM, local synthesis or listening endpoint.

## Verification

94 workspace Rust tests pass (one pre-existing microphone fixture ignored), including a real HTTP 429 client test that checks cooldown and credential omission. The two-peer host HTTP test checks gateway outages, preserved fallback history and forwarding of a simulated 429/retry interval. The graphical UI test passes, as do speech HTTP boundary tests, Clippy and syntax checks. Real local 7M speech samples are recorded separately in [fast voice validation](pocket-fast-speech-validation.json).
