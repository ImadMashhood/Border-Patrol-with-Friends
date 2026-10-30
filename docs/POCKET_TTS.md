# Pocket TTS voice profile

The installed default is **Pocket TTS 4.0.1, English October 2026 model**, running locally on CPU. This is one of the engines listed in the **Scam With Your Friends 0.1.95** developer notes, alongside MOSS TTS Nano and Qwen3 TTS. Those notes do not identify a universal English default or publish the game's exact runtime integration/voice assignments. We use the official Pocket Python implementation through our existing authenticated local speech service. [Developer announcement](https://steamcommunity.com/app/4954910/allnews/), [official implementation](https://github.com/kyutai-labs/pocket-tts), [Python API](https://github.com/kyutai-labs/pocket-tts/blob/main/docs/API%20Reference/python-api.md).

## Launch

```bash
./scripts/setup-voice.sh
./play.sh
```

This workspace is already installed and warmed. Other checkouts need setup once. Public preset weights and cached speaker states work without a Hugging Face account or voice-cloning permission. Downloads are cached under ignored `.tools/huggingface`; the separate `.tools/pocket-speech` environment avoids dependency conflicts with the optional Qwen/Kokoro runtimes. Whisper Small remains on GPU. Always-on capture, 1.5-second silence detection, 20-second utterance submission and playback located at the NPC's face use the existing game path.

## Characters and delivery

All ten travelers have a separate stable Pocket voice assignment in [traveler_voices.json](../assets/settings/traveler_voices.json). Speaker states load once, are copied for generation and are protected by the service lock. Dialogue retains slang, contractions and punctuation; Pocket produces its own conversational inflection.

Pocket does **not** accept Qwen's natural-language emotion/accent instructions. The authored description and emotion metadata still reach the service, but they are not an instruction-control feature of Pocket. These preset accents can differ from the authored Qwen directions. Explicit screams are also unreliable: the synthetic `AAAH!` sample was recognized as “Appa,” so this change does not establish scream support. For direct delivery instructions and the earlier expressive voice implementation, select Qwen:

```bash
BORDER_TTS_ENGINE=qwen ./scripts/setup-voice.sh
BORDER_TTS_ENGINE=qwen ./play.sh
```

Kokoro remains selectable with `BORDER_TTS_ENGINE=kokoro` on both commands. Engine selection is explicit; inference errors use the existing local “Excuse me?” fallback rather than silently downloading another engine.

## Validation

On the available **six-core Ryzen 5 3600 / RTX 3080**, six short spoken lines generated in **1.24–1.82 seconds**, median **1.42 s**. The previous Qwen sample median was **11.58 s**. These measure complete WAV generation, not streaming first-audio latency or complete LLM response latency. The HTTP playback interface still waits for the whole WAV.

All six lines retained their wording when passed back through Whisper Small (with ordinary punctuation/spelling normalization). This is synthetic regression coverage, not a microphone accuracy score or a human assessment of naturalness. Combined recognition/voice process memory after the sample was **586 MiB GPU** and **1862 MiB RSS**. Pocket itself runs on CPU; GPU allocation is recognition/runtime overhead. The six-core/16 GiB/RTX 2060 design target remains unchanged and has not been certified on that GPU.

Checks passed: three authenticated HTTP boundary tests; real Rust STT/TTS adapters including a 20-second recognition input and two voice previews; seven-sample local synthesis/transcription benchmark; Python syntax, shell syntax and dependency compatibility. See [raw Pocket results](pocket-speech-validation.json). The timing run used one speech process; the adapter test separately exercised two coexisting services.

```bash
python3 scripts/test_speech_service.py
python3 scripts/smoke_gpu_speech.py
python3 scripts/benchmark_character_speech.py
```
