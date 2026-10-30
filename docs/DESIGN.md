# Border Patrol with Friends — first playable

A fictional checkpoint at Alder Crossing. Walk to a car, press E, interview a traveler, inspect documents/cargo, admit or deny, and read a factual debrief. Ten authored cases: seven honest and three smugglers. No combat, driving or multiplayer in this slice.

## Architecture

WrongNumber inspires the separation of simulation, caller and trusted service. Its dispatch gameplay is not reused. Its current Rust 1.99.1 / Bevy 0.20 pins are not copied: this slice uses an independently compiled Rust 1.94.0 / Bevy 0.17.3 / bevy_egui 0.38.1 combination. No reference-project files are modified.

- `scenarios`: typed manifests, validation, deterministic ordering and truth-based dispositions. No renderer or provider dependency.
- `caller`: prompt contract, disclosure validation and deterministic offline actor.
- `caller-server`: loopback HTTP service, authoritative per-session history, optional OpenRouter adapter, timeouts, bounded repair, turn budgets. API secrets stay here.
- `speech`: bounded external recorder/whisper/Piper adapters; typed fallback always works.
- `game`: Bevy first-person blockout and egui inspection overlay, HTTP on worker threads, stale results discarded on cancellation.

Truth is immutable. Smugglers have false public claims contradicted by specific, inspectable evidence; honest uncertainty does not change guilt. Demographics and nervousness are never evidence. The LLM cannot decide dispositions or generate evidence. Local game assets are authoring data, not an anti-cheat boundary.

## State and contracts

CarPresent → PlayerNearCar → ConversationActive → DecisionPending (secondary evidence) → DecisionCommitted → Reveal → NextCar. E opens/resumes only when the camera ray intersects the car within 3m. Esc releases focus; mouse capture can be toggled while interviewing. Decisions require an active session, no pending reply, and are committed once. The barrier/car animate before the next case. One shift uses all ten cases exactly once in a seeded order.

JSON actor output: dialogue (1–1000 chars), emotion enum, action `none`, known and disclosed fact_ids, end_conversation=false. Prompts carry scenario knowledge, visible evidence, prior turns and the current utterance. One repair attempt is permitted; errors are shown, never converted into outcomes. Server keeps at most 30 exchanges, 40 turn attempts per case, 1024 output tokens per request. A process-wide attempt cap includes failed/repair requests. Logs report latency and token usage; dollar cost is configurable by model rates rather than guessed.

## Milestones and acceptance

1. P1A: workspace and checkpoint movement/E interaction.
2. P1B: ten manifests, text interviews, evidence, decisions, debrief and seeded progression.
3. P1C: offline actor and optional trusted live provider, validation/error states and budgets.
4. P1D: automatic open-microphone phrase detection through external local speech tools; subtitles, microphone controls, mute and transcript. Listening pauses for AI processing/playback, and resumes automatically. Tool/model installation and hardware acceptance are separate from typed play.
5. P1E: headless integration tests plus real graphical/voice/provider playtesting. Report untested hardware and paid-provider paths explicitly.

See README for commands, requirements and current acceptance evidence. Later co-op can place the existing authoritative session service behind authenticated networking; synchronization is not implemented here.
