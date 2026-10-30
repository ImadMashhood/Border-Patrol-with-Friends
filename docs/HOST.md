# Headless host and protocol — Parts 2–3

The foundation runs without Bevy, a GPU, microphone or paid model call. It supports two real peers against one authoritative in-memory host. The existing `./play.sh` launches the graphical solo prototype; connecting its world to this host is Part 15. Physical inspections, combat mechanics, upgrade voting, reconnect and complete shifts remain in their later roadmap Parts.

## Verify the foundation

From the repository root, with the pinned Rust toolchain and Python 3 installed:

```sh
python3 scripts/test_host.py
```

The script builds and launches a host, two separate `border-peer` processes and the AI gateway in offline mode. It checks ready states, exclusive lane ownership, host-owned movement, hidden-state isolation, authentication boundaries, accepted/stale/invalid actor delivery, the gateway-to-host conversation bridge, one settlement and disconnect cleanup. Credentials are generated for the test and kept out of its output. Every child is stopped on success or failure. CI runs it on Linux and Windows.

Headless unit tests:

```sh
cargo test --locked -p border-scenarios -p border-caller -p border-net -p border-simulation -p border-caller-server
```

## Start a host manually

Use distinct temporary values for these three credentials. Only the join code goes to peers; the operator and gateway credentials stay on the host/service machine.

```sh
export BORDER_HOST_JOIN_CODE='choose-an-invite-code'
export BORDER_HOST_OPERATOR_TOKEN='choose-a-distinct-secret-at-least-24-characters'
export BORDER_HOST_GATEWAY_TOKEN='choose-another-distinct-secret-at-least-24-characters'
cargo run --locked -p border-caller-server --bin border-host
```

Defaults: `127.0.0.1:8090`, seed `42`, scenario pack `assets/scenarios`, protocol `1`. Run from the repository root. `BORDER_HOST_BIND` explicitly changes the listen address and `BORDER_HOST_SEED` changes scenario order. This is a development HTTP transport; remote encryption, Internet matchmaking, production admission controls and reconnect are later work. A session exists only in memory and restarts with a new UUID.

In each of two other terminals, set the same `BORDER_HOST_JOIN_CODE`, then run:

```sh
cargo run --locked -p border-net --bin border-peer -- http://127.0.0.1:8090 Officer-A
```

Use `Officer-B` in the second terminal. The client prints public join state, keeps its random peer token in memory, and accepts newline-delimited commands:

```json
{"type":"set_ready","ready":true}
{"type":"claim_lane","lane_id":1}
```

The second officer claims lane `2`. Type `/snapshot` to fetch current public state or `/events` to read events since the last snapshot/replay. Both peers see the same world and ledger. The CLI sends a heartbeat every five seconds and leaves on EOF. A 15-second heartbeat timeout disconnects a peer and releases their lane. Rejoining an active shift is deliberately deferred to Part 15.

To exercise host operations, copy the public `session_id` printed at join, then POST an envelope to `/operator` with `Authorization: Bearer <operator token>`:

```json
{"protocol_version":1,"session_id":"SESSION-ID","command":{"type":"start_shift"}}
```

Other operator commands: `spawn_next_vehicle` (`lane_id`, `kind`: car/van/truck), `advance_world` (`delta_ms`, at most 1000), `apply_damage` (`peer_id`, `amount`), `restore_health`, `create_incident` (`kind`), `resolve_incident` (`incident_id`) and `approve_purchase` (`proposal_id`). This foundation deliberately advances simulation time through explicit operator ticks, which makes replay/debugging deterministic. Runtime clock scheduling is Part 5.

A spawned vehicle queues at position zero and moves at five metres per second to a stop ten metres along the lane. Only the front vehicle advances. Its lane owner may submit `inspect_evidence` (`vehicle_id`, an authored `evidence_id`) or `decide` (`vehicle_id`, `disposition`: admit/deny/detain). Evidence IDs are validated against the hidden case; distance checks and physical search points are Part 10. After a decision, departing vehicles move to twenty metres and exit; detention removes the vehicle from the processing queue on the next tick.

Current foundation settlement constants: correct admit +100, correct deny +200, correct detain +250; admitting a smuggler −400, wrong deny/detain −150. The host alone evaluates hidden truth. These are provisional rules for authority testing, not the final Part 12 balance. A peer can propose `document_scanner` with `request_purchase`; only the operator can approve it, at the host’s fixed cost of 300, once. The later gameplay system will replace this operator harness with the intended review/vote flow.

## Crate and information boundaries

| Crate | Responsibility |
| --- | --- |
| `game` | Bevy rendering, input and existing solo prototype interaction |
| `simulation` | Headless rules, private cases/history, peer/lane ownership, clock, queue/motion, evidence, dispositions, ledger, health, incidents, purchases |
| `net` | Versioned request and public snapshot/event DTOs; standalone peer client |
| `scenarios` | Validated authored truth, public claims, character knowledge/cover story, evidence/reveal rules and deterministic ordering |
| `caller` | Prompt construction, ordered conversation state, allowed fact/action schema and response validation |
| `caller-server` | Existing provider gateway plus the new separate headless host executable |
| `speech` | Independent local audio adapters; no world authority |

`Scenario.truth` is immutable authority. `Scenario.actor`, character and public claims define what a traveler knows and says; evidence/reveal rules constrain disclosure. A private simulation case stores that scenario separately from conversation history. A public vehicle stores only observed evidence, traveler identity and opening dialogue. Public snapshots and replay events never serialize the private case, hidden truth, smuggling flag, contradictions or provider prompt. A settled ledger reveals correctness only after the decision.

The reuse from Wrong Number is architectural: separate content, actor prompt/state, validated gateway and audio adapters. No dispatch call lifecycle or dispatch-specific rules enter the new simulation.

Rust is pinned to 1.94.0 in `rust-toolchain.toml`, Bevy to 0.17.3 and bevy_egui to 0.38.1; the lockfile pins the resolved graph. Verified with the installed compiler, a full locked workspace build/tests and Clippy. Release references: [Rust 1.94.0 official announcement](https://blog.rust-lang.org/2026/03/05/Rust-1.94.0/) and [Bevy 0.17.3 API](https://docs.rs/bevy/0.17.3/bevy/). These are reproducible project pins, not a claim to be the newest releases.

## Protocol and authority

All state-changing envelopes include `protocol_version` and `session_id`; peer command envelopes also contain a unique `request_id`, strictly increasing `sequence` and optional `expected_revision`. A rejected command does not consume a sequence. Retry a transport failure with the exact same envelope: an identical accepted request returns its original receipt without mutation; reusing its ID with changed content fails. Accepted commands are bounded to 4096 per peer per session; all receipts are retained for that session. At most 32 total joins may occur before restarting the lobby.

Peer identity comes from the token, never a command field. Unknown JSON fields and privileged client command variants fail deserialization. A peer may set readiness, claim/release a lane, request an inspection/decision or propose an upgrade. It cannot send guilt, rewards, balances, prices, arbitrary positions, damage, another peer’s identity or an authoritative decision result. Incapacitated peers cannot act. Host operators own world ticks, incident resolution and damage; ordinary peer tokens cannot call those routes.

Each mutation produces a monotonically numbered event with an ID of `session UUID:sequence` and a world revision. Authenticated peers can GET `/snapshot` or `/events?after=N`. The replay window retains the last 512 events. An expired cursor returns `resync required`; fetch a snapshot and resume at its `latest_event_sequence`. A future cursor, wrong session/version or stale expected revision fails. HTTP clients have a five-second timeout, request bodies are limited to 8192 bytes, and private histories are limited to 128 turns.

## Trusted AI bridge

Run the existing `border-caller-server` gateway with these extra environment settings:

```sh
export BORDER_HOST_URL='http://127.0.0.1:8090'
# Same server-only BORDER_HOST_GATEWAY_TOKEN as the host.
cargo run --locked -p border-caller-server
```

The existing provider configuration still selects live OpenRouter/Gemini or offline rehearsal. A trusted local caller POSTs `/host-turn` to the gateway with its existing `BORDER_SESSION_TOKEN` bearer credential:

```json
{"protocol_version":1,"session_id":"SESSION-ID","vehicle_id":1,"utterance":"What is your name?"}
```

The gateway retrieves private `/actor-context` using the separate server-only credential, runs the provider with the host-owned scenario and history, validates the reply, and forwards it to `/actor`. The host validates it again against its own case/history and expected turn index before emitting `traveler_spoke`. Concurrent or retried stale actor results cannot append the same turn twice. The model currently has only the approved action `none`: it can speak but cannot award, purchase, open barriers, change truth or make a disposition. Future action types require explicit host rules.

Peer tokens and the invite code cannot retrieve actor context or deliver model output. Provider work happens outside host locks. Invalid output, stale context or provider failure creates no transcript event. The bridge uses public-fact disclosure for now; evidence-aware replicated interviews and peer-to-gateway routing are integrated in Parts 8/15. The service bridge smoke uses offline rehearsal to avoid paid API calls; it is not a new live-model quality benchmark.
