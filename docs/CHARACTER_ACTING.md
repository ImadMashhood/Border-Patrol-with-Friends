# Character acting, voice and inspection repairs

Launch with `./play.sh`; restart an already running game/host/speech service to load the new code and profiles. Normal talking still uses E, free movement and continuous microphone capture. Hold Tab for the tablet.

## Animation

Living travelers, recovery porters and remote officers now use eleven connected bones, replacing the remote officer's three static blocks. The shared acting layer adds distance-driven leg/arm cycles, gradual acceleration and stopping, running lean, seated/standing blends, breathing, head movement and speech gestures. Individual blink timing avoids synchronized blinking. Actual speech RMS drives the mouth; mouths do not flap merely because synthesis is pending. Fear/anger changes gestures. Dead faces do not blink, and the existing authoritative Rapier ragdolls remain separate from cosmetic living-character poses.

Forward kinematics recomputes child positions from shared joint anchors, so turning or gesturing cannot pull arms/legs apart. The joint test checks every anchor over walking, sitting and talking blends. These are procedural prototype animations, not motion-captured performances or facial phoneme tracking.

## Speech recognition and personality

The developer's current **Scam With Your Friends 0.1.95** notes list **Whisper Small as the default**, with Base and Moonshine alternatives. The prototype now defaults to Whisper Small through the existing local faster-whisper adapter, with GPU `int8_float16`. [Official developer update](https://steamcommunity.com/app/4954910/allnews/). Set `BORDER_STT_MODEL=large-v3-turbo` to retain the earlier recognizer, or `BORDER_STT_DEVICE=cpu` for CPU recognition. Automatic 1.5-second endpointing and the 20-second utterance limit remain unchanged.

All ten authored travelers have expanded speech directions: contractions, cadence, slang, humor, bluntness, warmth, interruptions and distinct reactions to social pressure. The dialogue prompt permits swearing, laughter, shouting and frightened cries when appropriate without forcing every character to act that way. Accent instructions describe delivery rather than distorting spelling. The existing fear/survival/coercion state and immutable cargo/evidence system remain authoritative. A fresh live Gemini sample produced 12 valid replies across Cal, Rosa and Amir; see [the sample](acting-dialogue-sample.json).

## Expressive synthesis

**Update:** [Pocket TTS is now the installed default](POCKET_TTS.md). This section records the optional Qwen implementation and its measurements.

The instruction-controlled voice option is **Qwen3-TTS 12Hz 1.7B CustomVoice**, with ten authored character directions in [traveler_voices.json](../assets/settings/traveler_voices.json). Stable speaker assignments plus age, cadence, attitude and accent instructions distinguish voices. Validated reply emotions reach the synthesis service: anger, confusion, irritation, fear, panic and desperation no longer all use the same neutral delivery. Explicit alarmed cries can generate nonverbal screams. Emotion and accent are instructions to the model, not guaranteed human-rated performances.

The model's documented CustomVoice interface supports speaker and natural-language delivery instructions. [Official Qwen3-TTS implementation](https://github.com/QwenLM/Qwen3-TTS). The production path keeps its acoustic weights at FP16; text embedding lookup and final audio decoding run on CPU to reduce VRAM without the speech errors observed with aggressive weight compression. The configured speech allocation target is now **4.5 GiB**, replacing the former 3 GiB Kokoro target. The overall engineering reference remains six cores, 16 GiB RAM and a 6 GiB RTX 2060, with limited remaining VRAM headroom and no hardware certification yet.

Install/update the local profile once:

```bash
./scripts/setup-voice.sh
./play.sh
```

This downloads model weights into ignored `.tools/`, keeps inference local, and uses the existing authenticated loopback service. The selected speech profile is installed and warmed by setup. The face-positioned stereo playback, microphone echo suppression, cancellation and local “Excuse me?” fallback remain connected. HTTP synthesis allows enough time for the larger model; it does not block rendering.

For the faster, less expressive Kokoro path:

```bash
BORDER_TTS_ENGINE=kokoro ./scripts/setup-voice.sh
BORDER_TTS_ENGINE=kokoro ./play.sh
```

The CPU Tiny/Piper fallback remains available with `BORDER_SPEECH_PROFILE=lightweight`.

## Measured cost and limits

On the available RTX 3080 / six-core Ryzen 5 3600, the six spoken test lines took **7.42–15.94 seconds** to synthesize (median **11.58 s**). Recognition of those generated lines took **0.112–0.178 s** (median **0.134 s**). A separate short cry generated in 2.69 s. All six spoken lines retained their wording under transcription; the nonverbal cry appropriately has no recognized words. This is a small synthetic sample, not an accuracy percentage or proof of every accent.

The speech process measured **4266 MiB VRAM** and **5115 MiB RSS** after this sample. Whole-device deltas can include concurrently running graphical clients; per-process NVIDIA readings are used for the allocation check. These numbers fit the selected speech budget on this machine but leave much less headroom on a 6 GiB card. The expressive voice is substantially slower and heavier than Kokoro. A headset listening test, Windows audio acceptance and actual RTX 2060 frame-rate/memory testing remain outstanding. See [raw voice results](character-speech-validation.json).

## Cargo and navigation repairs

Physical opening state is now separate from search history. E can close and reopen the inner container/glovebox as well as the main cargo panel, without losing evidence. Closing the parent cargo panel also closes the inner lid. The target ray uses each displayed hinge's current transform; a reachable rear latch lets the officer close an elevated lid/tailgate. This covers sedan, pickup, SUV, van, hatchback and utility pickup geometry. Search findings appear directly in a small field card, remain visible when targeting an open searched compartment and stay in the tablet.

The porter resets to the chair's original facing direction when seated again. Walking porters and fleeing travelers use host-owned A* paths over a 0.5-metre grid, line-of-sight smoothing and swept capsule clearance. Checks include rotated vehicle bodies, open doors, buildings, closed gates, other pedestrians, map bounds and the bridge. Paths replan against moving obstacles; every step checks the current segment in 8-centimetre increments. A blocked actor waits/replans instead of advancing through a car. Dedicated vehicle entry/exit and recovery driving remain separate animations/controllers, not a full crowd or vehicle navigation system.

## Verification

```bash
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
python3 scripts/test_host.py
python3 scripts/test_speech_service.py
python3 scripts/test_graphical_shift.py --repairs
python3 scripts/test_graphical_shift.py --ui
python3 scripts/test_graphical_shift.py --physics
python3 scripts/benchmark_character_speech.py
```

The final workspace passes **93 Rust tests** (one existing microphone fixture ignored), Clippy and formatting. The three speech HTTP tests, host regression and UI/cargo/physics graphical modes pass. See [validation record](acting-validation.json).

The cargo acceptance mode runs two real clients, selects animated lids/containers through the normal E controller, closes/reopens them and checks retained findings. The UI and physics modes cover the earlier tablet, conversation, glass and extraction behavior with the new character rigs. Automated graphical checks use offline dialogue and disable recording/audio. The separate local speech check synthesizes and recognizes samples through the authenticated HTTP endpoints. Actual mouse/headset feel remains a human playtest.
