# Running the current Border Patrol with Friends prototype

Run terminal commands in this guide from the repository root. This guide describes the current prototype; the [technical build roadmap](../README.md) describes the complete target game.

A Rust / Bevy first-person checkpoint prototype. Interview ten fictional travelers at **San Ignacio, a fictional border crossing in Texas**, compare their accounts with documents and cargo, and decide who crosses. Exactly seven honest travelers and three smugglers. Built from primitive meshes; no art downloads required.

![San Ignacio checkpoint](checkpoint.png)

## Launch from a terminal

Linux / Fedora:

```bash
./play.sh
```

Windows PowerShell:

```powershell
.\play.ps1
```

First launch compiles the game and local interview service. Subsequent launches reuse the build. Rustup installs the pinned Rust 1.94.0 toolchain. Install Rust from https://rustup.rs if needed. Linux needs a C compiler, X11/XWayland and Vulkan-capable graphics drivers; Fedora development prerequisites are `gcc gcc-c++ make libX11-devel libxkbcommon-devel`. Windows needs the Visual Studio C++ build tools and Vulkan/DirectX drivers. Linux launcher also uses Python 3 and curl. Windows launcher uses PowerShell directly.

Choose a reproducible shift order: `./play.sh --seed=123`. The service binds only to loopback and the launcher stops it when the game closes. Logs: `output/caller-server.log`. Change `BORDER_PORT` if 3847 is occupied. Launch from any directory using the full path to the script.

## How to play

1. Click **Begin patrol**, walk with WASD and look with the mouse.
2. Aim at the teal sedan within 3 meters; press **E** to interview the driver.
3. Speak normally: the microphone listens automatically and sends your question after a short pause. You can also type and click **Send** or press Enter. Suggested topics fill the text box.
4. Open **View documents**, then **Request secondary inspection** to inspect physical cargo. Compare actual evidence with the traveler's account.
5. Choose **Admit** or **Deny**. The debrief shows immutable truth, contradictions, missed physical checks and your result.
6. **Close file & call next car** plays an exit animation and advances. Finish all ten for your shift score.

**Tab** switches mouse look / typing while the interview stays open. **Esc** closes the overlay and releases the pointer; aim and E resumes the same case. Click the scene to capture the mouse. No interview ends automatically. Live spoken compliance is character-specific: opportunistic or eager travelers can repeat a false denial to get through; principled or defiant travelers can refuse. These words never change physical evidence or guilt. Repeated and irrelevant questions change the actor's reaction; nervousness does not indicate guilt.

## AI modes

The default is clearly labeled **offline rehearsal**: authored topic responses with personality, repetition reactions and evidence-gated admissions. It is deterministic and requires no API key. It is useful for testing the complete gameplay loop, but does not understand arbitrary language like a live LLM.

For natural AI conversation on Linux, copy `.env.example` to `.env`, then configure:

```bash
OPENROUTER_API_KEY='your-key'
OPENROUTER_MODEL='google/gemini-2.5-flash-lite'
```

On Windows set those environment variables in PowerShell before running the launcher. Choose an available Qwen/Gemini or other OpenRouter model and benchmark character quality/latency; no model is silently selected or paid request made without your configuration. Optional input/output USD rates enable estimated cost logs (zero means no rate configured). A process cap defaults to 120 provider attempts including repairs; cases allow 40 turn attempts each and keep all successful turns. Provider requests time out at 10 seconds, with at most one contract repair. The configured local setup uses **Gemini 2.5 Flash-Lite**, as requested. The dialogue-context improvements remain enabled. A controlled comparison ran the same 20 officer utterances through Flash-Lite, Flash and GPT-4.1 mini with both the old and revised conversation context. Flash handled references, clarification and reactions more reliably than Flash-Lite in those runs. See [measured results and limitations](INTENT_BENCHMARK.md). No model is guaranteed to avoid hallucinations.

The trusted server owns scenario truth, evidence disclosure, history and dispositions. The game process receives public case data, actor replies and committed debriefs. Launch scripts remove provider key/model variables from the game environment. HTTP jobs run on worker threads; service handlers are asynchronous. Invalid JSON, unknown/hidden fact IDs, unsupported actions and automatic hangups are rejected. **Structural validation cannot prove every natural-language sentence is true**: live models can still hallucinate or disclose an untagged fact. Test your chosen model before treating it as reliable gameplay. The prototype is a local development service, not a hardened public multiplayer backend.

## Voice (Linux)

Install local speech recognition once:

```bash
./scripts/setup-voice.sh
```

The default installed profile now uses **GPU Whisper Large-v3 Turbo** for recognition and **Kokoro-82M** for more natural speech. Both stay loaded in a local service. The engineering reference is **6 CPU cores, 16 GB RAM and an RTX 2060 with 6 GB VRAM**; the speech VRAM target is at most 3 GB, leaving headroom for the game. See [setup, measured results and hardware limits](SPEECH_MODELS.md).

Installation creates an isolated Python 3.11/CUDA environment and downloads the models once. It needs NVIDIA drivers, Python 3 venv, curl, `arecord` (Fedora `alsa-utils`) and `ffplay` (FFmpeg). It does not change system Python or require a C++ compiler for the default profile. The launcher discovers the installed service, waits for warm-up and cleans it up when the game closes.

For the previous CPU Tiny/Piper profile, use `BORDER_SPEECH_PROFILE=lightweight ./scripts/setup-voice.sh` and the same prefix with `./play.sh`.

**There is no push-to-talk.** After E starts an interview, the microphone stays active throughout that interview, even while you look around or close the overlay. Speech detection waits for 1.5 seconds of silence, transcribes the phrase locally, and sends it to the live AI automatically. Listening pauses while the AI thinks or the traveler speaks, then resumes; it stops when you commit a decision. Traveler replies play aloud by default. Headphones reduce echo. Silent audio and short clicks are not sent for transcription.

**Audio & controls** lets you turn the microphone off, mute the traveler, select the ALSA capture device (default `default`), and adjust the noise threshold. Click **Apply settings / retry microphone** after changing the device or threshold. Each utterance is capped at 20 seconds and recognition has a 30-second timeout. Replies use the resident Kokoro voices in the reference profile. The CPU profile still uses Piper; `./scripts/setup-tts.sh` installs its voices. The reference profile requires no paid speech API. Typed play works even if microphone/speech tools are absent.

Current voice limitations: capture supports Linux only; Windows supports typed play. Speech uses a persistent local GPU service (or external CPU tools) rather than CPAL/Rodio and is not yet spatialized at the driver window. Automatic phrase detection followed by local whisper.cpp transcription was verified with generated speech ("What is the purpose of your visit?"). Both GPU voice profiles were synthesized through the actual Rust/HTTP adapter and their WAV outputs verified locally. Microphone and speaker hardware still require a real playtest. CPAL device enumeration, Windows capture and spatial audio remain follow-up Phase 1 work.

## Development and validation

```bash
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build --locked --workspace
python3 scripts/smoke_api.py
# Optional PAID live test: supply OPENROUTER_API_KEY and OPENROUTER_MODEL
python3 scripts/smoke_live.py
./play.sh --smoke-test
```

The API smoke test starts a temporary offline service on 13847, runs 30 exchanges per case, inspects evidence, commits all ten correct decisions, and tests auth/input/state/budget errors. The graphical smoke mode saves `output/checkpoint.png` then exits. `./play.sh --smoke-interview` also exercises an interview and secondary inspection, saving `output/interview.png` and `output/evidence.png`. It is a rendering smoke check, not a human playthrough. CI is configured for Linux and Windows; an actual Windows run is not yet verified.

Scenario data: `assets/scenarios/*.json`. Each contains personality, coherent immutable truth, initial cover stories for smugglers, document/cargo evidence, contradiction references and offline actor guidance. The live model receives only initial cover facts from the actor fields, not canned repetition/confession lines. The loader rejects malformed packs and requires exactly ten unique scenarios, three smugglers and fixed unique seeds. Unit tests run 300 authored exchanges and check disclosure/ground-truth invariants. See [design](DESIGN.md), [scenario contract](SCENARIO_SCHEMA.md), and [validation results](VALIDATION.md).

This is the initial playable slice, not the entire PDF Phase 1 acceptance. Sustained live model consistency, human playtesting, speech hardware, spatial audio and Windows execution still need verification. Multiplayer, driving, combat, economy and detailed searches are outside this slice. Inspired by the crate separation in [WrongNumber](https://github.com/ImadMashhood/WrongNumber); no reference project was modified.
