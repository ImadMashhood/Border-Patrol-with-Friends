# Speech profile and reference hardware

**Current profile:** [Pocket TTS on CPU plus Whisper Small on GPU](POCKET_TTS.md). Qwen remains available for instruction-controlled delivery. The Turbo/Kokoro measurements below are historical.


The user-selected reference is **six physical CPU cores, 16 GB RAM, and an NVIDIA RTX 2060 with 6 GB VRAM**. The target rendering workload is 1080p / 60 FPS. This is an engineering target, not a published or certified minimum specification. Available measurements use an RTX 3080 and Ryzen 5 3600; the CPU has the reference six physical cores, while the GPU is faster and has more memory.

## Earlier Turbo/Kokoro reference profile

- Recognition: **Whisper Large-v3 Turbo**, using faster-whisper / CTranslate2 with `int8_float16`, beam size 3 and neural silence filtering. This replaces CPU Whisper Tiny for the default installed profile.
- Voice: **Kokoro-82M**, using resident GPU inference with Michael and Heart voice profiles. This replaces Piper as the installed default; character voice assignments remain unchanged and character expansion is deferred.
- A persistent loopback service keeps both models in memory, preloads both voices and warms inference before the game opens. Requests run away from the Bevy main thread.
- Only one GPU speech operation runs at a time. Four CPU threads are used for speech, preserving CPU headroom on the six-core reference.
- The speech allocation target is at most 3 GiB VRAM, leaving space for rendering and the desktop in a 6 GiB card. Setup checks observed resident GPU process memory against that budget. It does not enforce a hardware-level GPU allocation ceiling.
- Phrase completion uses 1.5 seconds of silence, with automatic submission at 20 seconds of elapsed utterance audio. The local backend accepts up to 22 seconds to allow pre-roll and frame padding. Always-on listening, automatic submission and playback echo suppression remain enabled.
- Speech recognition and synthesis are local after models are downloaded. The conversation model is separately configured as **Gemini 2.5 Flash-Lite**, per user preference.

## Install and launch

```bash
./scripts/setup-voice.sh
./play.sh
```

The reference installer creates a private Python 3.11 environment and installs pinned CUDA-compatible dependencies. NVIDIA drivers, Python 3 with venv, curl and ffplay must be available. It downloads several GB once; it does not install a system CUDA toolkit or change system Python. Model files and runtimes live in ignored `.tools/`. On launch the service waits for warm-up, then the game starts. Closing the game stops both local services.

The smaller CPU profile remains available:

```bash
BORDER_SPEECH_PROFILE=lightweight ./scripts/setup-voice.sh
BORDER_SPEECH_PROFILE=lightweight ./play.sh
```

That profile retains Whisper Tiny and Piper and is useful without NVIDIA CUDA. The GPU speech upgrade is currently integrated with the Linux launcher/capture path; Windows microphone acceptance remains outstanding.

## Verification

```bash
python3 scripts/test_speech_service.py
./scripts/gpu-speech.sh --benchmark
python3 scripts/smoke_gpu_speech.py
python3 scripts/benchmark_reference_runtime.py
```

The synthetic benchmark runs exactly the same six utterances through both recognizers, including three samples with deterministic 12 dB added noise. It records word error rate, synthesis time, recognition time, process RAM and GPU memory. Both models got the initial six samples correct, so that sample establishes a latency improvement, not a proven accuracy advantage. Broader recorded speech and real microphones are needed before claiming an accuracy percentage.

The optional human-speech test accepts the public LibriSpeech dummy Parquet fixture with `./scripts/gpu-speech.sh --benchmark --human-parquet /path/to/file.parquet` (requires the optional `pyarrow==17.0.0` package). It selects the first six recordings between one and twelve seconds without selecting by recognition outcome. Its results are separate from the synthetic benchmark.

The Rust adapter check starts a fresh service, verifies real recognition over HTTP, and synthesizes both voices through the same Rust functions used by the game. The 1080p runtime check samples combined game/service RAM and whole-device VRAM while rendering. These measurements cannot substitute for an actual RTX 2060 playtest or confirm 60 FPS on that card.

Upstream sources: [faster-whisper](https://github.com/SYSTRAN/faster-whisper), [Kokoro](https://github.com/hexgrad/kokoro), and the [LibriSpeech fixture](https://huggingface.co/datasets/hf-internal-testing/librispeech_asr_dummy).

## Measured results on the available RTX 3080

| Check | Previous CPU Tiny path | Resident GPU path |
|---|---:|---:|
| Six synthetic questions: median recognition | 8.282 s | 0.162 s |
| Six synthetic questions: normalized WER | 0% | 0% |
| First six eligible human recordings: median recognition | 8.911 s | 0.233 s |
| Human recordings: normalized WER | 1.96% | 1.96% |
| Median complete-utterance voice generation | Not compared | 0.100 s |

The human set uses the first six LibriSpeech fixture recordings lasting 1–12 seconds. Punctuation, case, `Mr`/`Mister`, and the number forty are normalized consistently. This small set tied in accuracy; these observations do not justify claiming a measured accuracy gain or predict performance for every accent/noise condition. The larger recognition model provides a higher-capacity path for further playtesting.

Observed resident speech GPU process snapshots were about 2.0 GiB, with process RAM around 2.0–2.5 GiB. During a concurrent game/interview check, all child processes peaked at 2,686 MiB RSS and the whole GPU—including an existing desktop baseline of 1,972 MiB—peaked at 4,734 MiB. The game/speech incremental device peak was 2,762 MiB. The 1080p test mode requested 1920×1080, but the desktop window manager constrained the captured window to **1920×973**. This is recorded explicitly rather than treated as a certified 1080p/60 FPS result. The resource headroom is encouraging; actual RTX 2060 frame rate and microphone acceptance remain unverified.

Traveler playback now follows the rendered face with dynamic stereo panning and distance rolloff. See [current positional audio and HUD](HUD.md). Recognition and synthesis models are unchanged.
