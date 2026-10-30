# Casual conversations and character psychology

Run `./play.sh` from the repository. For another officer before ready-up, run `./play.sh --join --name=Officer-B`. The configured live model remains Gemini 2.5 Flash-Lite; local recognition remains Whisper Large-v3 Turbo and speech remains Kokoro-82M.

## Physical interaction

Approach a stopped car. Its windows begin raised; the seated driver lowers their window when an officer comes within 3.2 metres, and raises it after everyone leaves. Aim at a nearby interactable object to see a white mesh outline. Press **E** on the traveler to start or end the conversation. No conversation panel opens automatically, and walking/mouse look stay available. **Tab** opens the optional clipboard for transcripts, typed questions, documents, searches and decisions.

The initiating officer exclusively owns speech until ending the conversation, leaving the 8-metre conversation radius, disconnecting or becoming incapacitated. Another officer cannot interleave questions or steal an active conversation. Transcript/evidence remain shared. The host increments a conversation epoch, rejecting late replies from prior owners. A resumed case retains its ordered conversation history and does not replay its opening line.

Press **E** on outlined doors, hood, trunk/tailgate and accessible containers to operate them. A door target is the lower panel; the traveler's head is the talk target. Driver decisions still require returning to the window. Search permissions and physical opening requirements remain authoritative.

Hold **E** on any reachable bone of a dead body to grip it. Walk and look while holding, release to drop, then grip again as many times as desired. There is no fixed hold duration. The host uses a force-limited spring and existing anatomical joints; a seated corpse is pulled through its open door before lifting above the roof. A lost/stale held-input stream releases the grip. Extracted bodies remain movable when their original car is recovered.

Each lane has a seated porter and chair. Press **E** on the outlined porter or **T** near a driverless car in your assigned/assisted lane. The porter approaches via the front corner, removes a seated corpse if necessary, enters, closes the doors, drives to a separate recovery parking space and walks back to his chair. He waits while an officer holds the body or blocks his driving path. A living traveler outside the car also allows recovery. Recovery clears the queue without inventing a legal disposition or reward.

## General psychology

Each of the ten authored travelers has individual 0–100 values for survival, boldness, integrity, aggression, sociability, greed and pride. These traits combine with ordered conversation memory and trusted physical observations to derive fear, threat severity, coerced compliance, rapport, embarrassment, humiliation, anger, bribery interest, frustration, confidence, suspicion and cooperation.

Insults, friendly persuasion, apologies, humiliation, bribe offers and threats have different effects. Fear and danger decay slowly across turns; reassurance cannot instantly erase intimidation. The live model receives the actual dialogue and contextual guidance, rather than a mandatory canned reaction. Normal compliance boundaries describe preferences, not an unconditional refusal to speak a falsehood. Possible reactions include pleading, bargaining, freezing, arguing, reluctant compliance or seeking help. An aggressive/bold traveler under severe fear can physically leave the vehicle and flee; the host controls this movement. Calling for help is currently dialogue, not a dispatched response team.

Threat cue matching is deliberately conservative and supplements model interpretation. Common negated, quoted, hypothetical and joking phrases avoid automatic verbal-threat penalties; an actually aimed weapon remains dangerous regardless of those words. It is a heuristic, not a complete language classifier. Aim uses the officer's position, yaw and pitch. Nearby weapons, injury and recent impacts are host observations, never claims accepted from the model.

Credible explicit verbal threats or aiming a gun at the traveler during a submitted turn record a complaint and cost **100** shared funds. Hitting an unarmed traveler costs **250** per hit. The penalties are prototype balance values. Complaints are recorded events/notices; there is no complete investigation or discipline system yet. An innocent speaking officer is not charged merely because another officer has a weapon aimed nearby.

Spoken concessions remain untrusted speech. Saying “I have no tools” under coercion does not remove tools, change identity/family, rewrite documents or alter guilt. No model output changes money, evidence, health, doors, vehicle movement or legal outcomes. Available evidence still gates detention. The new emotions are `afraid`, `panicked` and `desperate`, alongside the existing seven states. Schema validation protects the structured contract, but cannot prove every natural-language claim accurate.

## Provider and voice failures

The gateway serializes provider requests and spaces attempts, including a format repair, by at least 1.2 seconds. It permits same-model provider fallback. HTTP 429 establishes a cooldown from Retry-After (5–120 seconds, 30 seconds if unavailable), preventing rapid repeated upstream calls. There is one format repair and a configurable process budget, default **500 attempts**, plus the existing 40-attempt case limit. These controls reduce bursts; external account/provider quotas still apply. See [OpenRouter provider routing](https://openrouter.ai/docs/guides/routing/provider-selection).

On delivery failure the host records the original utterance and a neutral **“Excuse me?”**, without new facts or gameplay actions. The game plays bundled Kokoro fallback clips even if speech generation is down. A normal speech synthesis failure also attempts this local fallback once, without a retry loop. Technical errors stay in logs or the optional clipboard; no automatic banner interrupts normal play. Microphone initialization/recognition errors need the existing audio controls to retry; a complete microphone device outage cannot accept spoken input.

## Verification

The full suite passes **85 Rust tests**, with one existing physical-microphone fixture ignored. Clippy, formatting, the HTTP outage regression and both graphical acceptance modes pass. See [validation record](conversation-validation.json).

Automated Rust checks cover all ten psychology profiles, ordinary questions, insults, absurd requests, death threats, actual weapons/injury, trait differences, remembered fear, humor/negation, emotion validation and immutable scenario truth. Interaction checks cover exclusive owners, stale replies, approach windows, delivery fallback, two 30-second body holds with joint integrity checks, release/re-grab, abandoned grip cleanup, correct weapon direction/attribution and the porter's full return cycle.

The real two-client casual acceptance check exercises E targeting, white outlines, no automatic panel, walking during a conversation and physical searches. The physics acceptance check exercises glass, aiming, cabin/outside ragdolls, repeated body grips and the porter. Automated graphical checks use offline AI and disable audio/microphones.

A separate paid live Gemini sample produced **12/12 validated replies** across Cal, Rosa and Amir, with one repair. Cal agreed to an absurd demand and then pleaded under a gun; Rosa resisted ordinary pressure and changed her reaction under danger; Amir showed fear/desperation. Median successful exchange time was 0.56 seconds; the repaired exchange took 8.71 seconds. This is a small qualitative sample, not a guarantee of all future dialogue. See [raw live sample](psychology-live.json). Physical audio feel, Windows desktop testing and RTX 2060 frame-rate certification remain unverified.

```bash
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
python3 scripts/test_host.py
python3 scripts/test_graphical_shift.py --casual
python3 scripts/test_graphical_shift.py --physics
# Optional paid live sample; export credentials from your local .env first.
cargo build -p border-caller --example prompt_messages
python3 scripts/benchmark_psychology.py
```
