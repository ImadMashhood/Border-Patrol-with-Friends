# Field HUD, equipment wheel and tablet

Run `./play.sh`. The world stays visible during ordinary conversation and movement. This update supersedes the earlier Tab clipboard and reticle-only UI.

## Controls

| Input | Action |
| --- | --- |
| Hold Tab | Open the equipment wheel; move the mouse to a sector |
| Release Tab / left click | Equip the highlighted pistol, tablet or hands |
| 1 / 2 / 3 while wheel is held | Select pistol / tablet / hands |
| E with tablet equipped | Open the tablet |
| I | Open / close the tablet directly |
| Esc / tablet close button | Close the tablet; Esc outside it releases the mouse |
| B | Draw / holster the pistol |
| Left / right mouse, R | Fire / aim, reload |
| E on an outlined object | Perform the small label's action |
| Hold E on a body | Keep gripping until released |
| 1 / 2 outside the wheel | Claim a lane |
| F / G / J / M | Flashlight / assist / lane swap / site map |

The compact bottom-left loadout shows equipped item, pistol magazine/reserve and reload/holster state. A short label projects onto the current outlined target: talk/finish, open/close a particular door or panel, search a container, hold to grab a body, or call the porter. The hint uses the same target as E and the white outline, not a separate distance selector.

The top-right conversation feed shows speaker names and the latest four utterances. Lines fade in over 0.2 seconds, remain for 6–14 seconds according to length, then fade out over 1.2 seconds. The latest traveler line stays visible during voice playback. The full ordered transcript remains in the tablet after captions expire. Opening the wheel/tablet hides the transient captions and world-action hint.

## Tablet

Selecting the tablet opens a centered field device. Closing it leaves a handheld tablet model; E opens it again. Selecting pistol or hands puts it away. Mouse look and firing are disabled while using the wheel/tablet. Walking and conversation remain free during ordinary play; no interview panel opens automatically.

Use the vehicle selector to review a record. **Details & documents** shows vehicle/lane/status, declarations obtained during conversation, presented documents, physical search findings, search locations and recorded contradictions. It exposes only public or observed data, never unsearched evidence or hidden scenario truth. **Conversation** keeps the transcript and supports typed questions to your own active traveler. **Shift & audio** provides lane claims, funds, notices, shift reset when permitted, microphone settings, noise threshold and mute/retry controls.

**APPROVE ENTRY**, **DISAPPROVE ENTRY** and **DETAIN** dispatch the existing authoritative host decisions. The buttons require a living stopped driver, physical access at their window and assignment/assistance for that lane. Detention additionally needs recorded contradictory evidence. Pending decisions disable submission; the tablet displays the host-confirmed result and offers **Release vehicle / call next**. Reading a record remotely does not grant permission to decide remotely. Decisions are final.

## Face-positioned speech

Traveler voice now uses the actual rendered head bone's face position after transform propagation, including seat/vehicle transforms and exit/walking animation. The camera supplies listener position and right-ear direction. Mono Kokoro/Piper output becomes stereo with equal-power directional panning and smooth distance attenuation, fading to silence by 18 metres. The output updates gains every 10 milliseconds and interpolates between gains to avoid clicks as the officer moves or turns. It streams through the existing ffplay adapter and does not require additional native audio development packages.

Playback cancellation, microphone pausing, generation checks and the bundled “Excuse me?” clips are preserved. The source follows the face rather than the camera or screen center. This is stereo spatial placement with distance rolloff; it does not add HRTF processing, wall occlusion or voice chat between players. As before, each client's traveler playback belongs to that client's active conversation.

## Road clearance

The cactus/bush cluster at the service road, nearby barrels and junction cones were moved off the carriageway. Procedural scenery now applies the full requested road clearance to the western service route, matching the other roads. A dedicated `--scene-view=service-road --smoke-test` view supports visual inspection.

## Verification

The workspace passes **88 Rust tests**, with one existing physical-microphone fixture ignored. Clippy, formatting and the two-client UI acceptance check pass. See [validation record](hud-validation.json).

The two-client `--ui` acceptance mode renders the HUD, projected E label, fading captions, wheel, tablet findings/transcript and committed decisions. It also moves both officers around their cars, searches sedan/truck cargo and containers, then returns to each driver and dispatches approve/deny through the tablet's shared controller. The host's confirmed outcomes are asserted; automated captures do not constitute a physical mouse usability test. Each voice source is checked against its rendered face position.

The audio checks cover left/right placement, turning the listener, distance rolloff, valid cached WAV decoding, malformed input rejection, and a real silent ffplay streaming run. Automated playback uses SDL's dummy driver. Listening with headphones remains a manual check; these tests do not certify perceived spatial quality or RTX 2060 frame rate.

```bash
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
python3 scripts/test_graphical_shift.py --ui
SDL_AUDIODRIVER=dummy cargo run --locked -p border-speech --example spatial_smoke
target/debug/border-game --scene-view=service-road --smoke-test
```
