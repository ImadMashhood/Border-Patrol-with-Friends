# Validation — October 29, 2026

Verified locally on Linux using the pinned Rust 1.94.0 toolchain:

- `cargo build --locked --workspace` passed for all five crates.
- `cargo test --locked --workspace` passed sixteen tests (plus one explicitly ignored local-speech integration test), including 300 headless character exchanges, immutable truth/disclosure invariants, exactly 10 cases / 3 smugglers, malformed output, repetition reactions, deterministic order, interaction-ray range and nonblocking provider timeout behavior.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` passed.
- `python3 scripts/smoke_api.py` passed 300 HTTP dialogue turns across all ten cases, evidence inspection and correct decisions for every case. Additional checks cover authentication, empty utterances, duplicate decisions, turns after a decision and the 40-attempt budget.
- `./play.sh --smoke-test` launched Bevy, rendered the scene, saved a screenshot and exited cleanly. The screenshot was visually inspected and the lighting/start view adjusted.
- `./play.sh --smoke-interview` exercised live local HTTP start, conversation and secondary evidence updates inside the rendered game. Interview and evidence screenshots were visually inspected. This scripted check bypasses physical movement and button clicks and is not a human playthrough.
- Built whisper.cpp v1.8.3 locally and installed the tiny.en model. The Rust speech adapter correctly transcribed synthesized audio as “What is the purpose of your visit?”. Temporary test recordings remained outside the repository.

## Still unverified or deferred

Live OpenRouter is now configured locally through an ignored owner-readable `.env` file. Gemini 2.5 Flash Lite passed 23 live dialogue turns over all ten scenarios with a measured median service latency of 0.61 seconds (maximum 1.02 seconds in that run). All three smugglers maintained their public cover story until secondary evidence confrontation, then acknowledged the authored goods. Exact verbatim replay is rejected and gets one repair attempt. Qwen was tried first and improvised additional facts; Gemini was selected after prompt tuning. Some free-form wording still added an unprovided family detail, so the semantic limitations below remain real. JSON/fact-ID validation does not prove semantic truth in arbitrary model prose. A human should play all ten cases against a chosen model before Phase 1 is considered accepted.

A headless game test verifies that a detected phrase posts a turn to an HTTP service and receives its reply without any keypress or Send click. Automatic microphone segmentation was separately tested with silence, clicks, bounded long phrases, pause resets, and a generated PCM question passed through the same segmenter and real whisper.cpp adapter. No push-to-talk key or Send click is needed for spoken questions.

No real microphone/hardware playthrough, Piper voice-model acceptance or Windows execution was performed. Windows CI is configured, not yet observed passing. Capture currently supports Linux through arecord; CPAL/Rodio integration, microphone device enumeration, spatial playback and Windows speech capture remain work for full PDF Phase 1 acceptance. Audio subprocesses finish independently; UI/network work remains responsive, but there is not yet immediate interruption of an already spoken reply.

No multiplayer, combat, driving, detailed cargo searching or economy is implemented. The source includes separate design and playable implementation commits. Local build outputs, downloaded speech tools/models, and credentials are excluded from version control.

## Intent and neural speech follow-up

See INTENT_BENCHMARK.md for the 20-utterance comparison across three models and both prompt formats. The local live model is now Gemini 2.5 Flash. A regression test verifies that the opening and all twenty prior exchanges are included as ordered role messages and canned reaction/confession lines are absent from system context. The server no longer drops early exchanges within its 40-attempt budget.

Piper 1.8.0 and the Ryan/ Lessac medium neural voice models were installed locally. Both were synthesized through the Rust adapter to valid 22,050 Hz WAV files (5.12 s and 6.10 s test clips). Audio quality is previewable; hardware playback and microphone echo still need a human playtest. No synthetic voice model is committed to the repository.

After rebuilding the actual game/server binaries with the intent revision, the live service passed 23 turns over all ten cases with Gemini Flash (median 0.88 s, maximum 1.26 s in that run). Cal could argue about the resale goods rather than reciting the old confession. All truth-based dispositions stayed correct. The updated graphical interview smoke check also exited successfully; smoke mode deliberately disables room recording and spoken playback.

## Reference speech model revision

The user selected a six-core CPU, 16 GB RAM and standard 6 GB RTX 2060 as the design reference, and explicitly deferred further character work. The dialogue configuration was switched back to Gemini 2.5 Flash-Lite as requested; context fixes remain in place.

Whisper Large-v3 Turbo with int8/float16 CTranslate2 and Kokoro-82M now run in a persistent local CUDA service. Both models/voices were downloaded, warmed, and exercised through the actual Rust recognition/synthesis functions over authenticated loopback HTTP. The always-on microphone pipeline remains automatic, with a reduced 480 ms phrase-ending pause. Setup checks resident speech memory against a 3 GiB reference budget. The CPU Tiny/Piper profile remains selectable.

Six synthetic/noisy questions and six deterministic human fixture recordings were compared against the prior Tiny path. Recognition latency improved substantially on the available RTX 3080; normalized accuracy tied on these small sets. Both Kokoro voice profiles were rendered to valid WAV previews. Two Python HTTP boundary tests, all sixteen Rust tests, lint and formatting checks passed. A concurrent renderer/service check completed with memory results and the desktop-constrained actual capture size recorded in SPEECH_MODELS.md. This is not a certified RTX 2060 or 60 FPS playthrough.

## Character compliance, phrase timing and provider repair

Added authored responses to authority and personal bounds for every traveler. A 30-turn live Flash-Lite check across all ten characters showed cooperative/opportunistic denials, principled/defiant refusals, and a reluctant traveler yielding after reassurance. The final decisions and immutable truth matched every original manifest. This is observed model behavior, not a guarantee for every paraphrase.

Provider generation now requests a strict JSON schema. A failed candidate is discarded and a bounded retry uses trusted system delivery guidance with the actual officer utterance still last; repair instructions are never inserted as fake officer dialogue. Known internal diagnostic phrases are rejected from traveler text. A single exact JSON code fence is accepted, while trailing prose, malformed JSON, unknown facts and gameplay controls are still rejected. Benign emotion aliases such as playful are normalized to the existing UI emotion categories. Deliberately repeating an officer-requested phrase is exempted from the immediate repetition guard.

Speech now waits for 1.5 seconds of silence and forces submission at 20 seconds of elapsed utterance audio (30 ms capture-frame granularity, plus pre-roll). Tests cover a 1.2-second internal pause, exact silence closure, sustained-speech cutoff, reset behavior and discarded clicks. The backend audio duration limit is 22 seconds for capture padding. There are 22 passing Rust tests plus the separate Python HTTP tests; the local-model integration check remains opt-in.

The updated GPU speech smoke test also submitted an actual 20-second WAV through the Rust adapter and received a valid transcription, confirming the backend no longer rejects the new recording length.
