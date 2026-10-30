# Host and protocol

The host is now connected to normal graphical play. See [Part 5 controls, launch/join and validation](SHIFT.md) for the end-to-end flow. `./play.sh` hosts; `./play.sh --join` opens a second local client. The setup defaults to two players, two lanes and loopback transport.

## Headless checks

```sh
python3 scripts/test_host.py
cargo test --locked -p border-net -p border-simulation -p border-caller-server
```

The test launches the actual host and provider gateway with separate credentials, two authenticated peers and a manual debug clock. It exercises the door hold, host/majority rules, movement authority, exact clock, lane swaps, assisted conversation/inspection, decision deduplication, event hooks and reset. No GPU or paid model call is needed.

For a desktop check with two real rendered clients:

```sh
python3 scripts/test_graphical_shift.py
```

## Manual service configuration

Run from the repository root with distinct `BORDER_HOST_OPERATOR_TOKEN` and `BORDER_HOST_GATEWAY_TOKEN` values of at least 24 characters, and a `BORDER_HOST_JOIN_CODE` of at least eight. `BORDER_SESSION_TOKEN` authenticates the local provider gateway. The launcher normally generates these service secrets and keeps them out of game-client environments.

```sh
cargo run --locked -p border-caller-server --bin border-host
```

Defaults: `127.0.0.1:8090`, seed 42, scenario pack `assets/scenarios`. Override with `BORDER_HOST_BIND`, `BORDER_HOST_SEED` and `BORDER_SCENARIOS`. `BORDER_GATEWAY_URL` defaults to `http://127.0.0.1:3847`; the provider gateway must have the same `BORDER_HOST_URL`, `BORDER_HOST_GATEWAY_TOKEN` and `BORDER_SESSION_TOKEN`. It keeps the existing OpenRouter/model configuration, including Gemini 2.5 Flash Lite when configured.

Normal host time advances automatically. Only explicitly setting `BORDER_HOST_MANUAL_CLOCK=1` enables trusted operator `advance_world` requests for deterministic debugging. Launchers force real-time mode. Public game clients cannot advance the clock.

## Wire contract, version 5

`POST /join` requires `protocol_version`, invite code and display name. It returns a room UUID, peer ID, random bearer token, next command sequence and public snapshot. Exactly two live peers may join in the lobby; active-shift reconnect is deferred.

`POST /input` accepts an `InputFrame` with version/session, increasing frame ID, normalized movement intent, finite yaw, jogging intent and `ready_held`. The host owns positions, speed/collision, input freshness, ready duration and the countdown. Unknown fields such as `position`, `held_ms`, `remaining_ms` or rewards are rejected. A snapshot is returned; the graphical worker sends input at twenty Hz, while the host ticks at twenty Hz.

`POST /command` uses the versioned session/request/sequence envelope. Commands express intent: claim/release a lane, assist, request/accept a swap, release a committed vehicle, request a new shift or propose an upgrade. `SetReady(true)` is rejected; readiness must be earned through the door hold. `SetReady(false)` withdraws a completed vote. Client-supplied guilt, rewards, prices, arbitrary world positions and privileged host commands are not accepted.

Identical accepted command retries return the original receipt. Altering an accepted request ID fails. A rejected command does not consume a sequence. Public snapshots acknowledge each peer's next accepted command sequence. There are up to 4096 accepted commands per peer/session and 32 lifetime lobby joins. Input frames use their own monotonic counter and do not consume the command budget.

`GET /snapshot` and `GET /events?after=N` require the peer token. Events have `room UUID:sequence` IDs and revisions. The host retains 512 events, including world ticks, so old cursors can require resynchronization: fetch a snapshot and resume from `latest_event_sequence`. Peer expiry is 15 seconds; graphical input refreshes liveness continuously. Expiry releases lane ownership and updates the host role.

The public snapshot contains officers/loadouts/positions, readiness, door state, the shared clock, lanes, vehicles, public conversation/evidence, resolved outcomes, ledger and dispatch hooks. Hidden scenario truth, actor instructions, contradictions and provider prompts are absent before a case is resolved. Only the committed debrief publishes that case's truth.

## Interview bridge

The host's peer-facing `/start`, `/turn`, `/inspect` and `/decide` routes use the existing caller DTOs; the session string identifies a host vehicle. Protocol 6 checks physical driver range for interviews/decisions and cargo range for `/inspect`, plus lane ownership/assistance. `search_compartment` validates its own physical location and opening conditions. `request_secondary` only marks a search request. The game requests a real vehicle, rather than choosing a scenario or supplying an outcome. Both officers see the same transcript and evidence.

For a turn, the host calls the local provider gateway's `/host-turn`. The gateway retrieves private `/actor-context` using its service credential, constructs the ordered prompt/history, validates the response and forwards `/actor`. The host validates it again against its case epoch and turn index. The only AI action currently allowed is `none`; a model cannot open barriers, change truth, purchase or award money. One request per traveler may be pending, and histories are limited to 128 turns. Cancelled requests clean up their pending guard. Old responses cannot cross shift resets.

The trusted `/operator` route remains available for debugging damage, incidents and purchases. Model output and peer tokens cannot invoke it. Prototype settlement constants remain +100 correct admit, +200 supported correct deny (zero for an unsupported guess), +250 evidence-supported correct detain, −400 admitting a smuggler and −150 wrong deny/detain; final balancing/review is Part 12–14. The debug document-scanner proposal costs 300 and requires operator approval.

## Crates and map

`game` renders/input; `simulation` owns headless rules; `net` defines versioned DTOs/shared geometry; `scenarios` validates immutable content; `caller` builds/validates actor conversations; `caller-server` runs gateway/host services; `speech` handles local audio. This keeps the design lessons from Wrong Number without dispatch-specific logic.

The renderer exports static solid bounds to `assets/maps/san-ignacio-collision.json`. After changing collision geometry:

```sh
cargo run --locked -p border-game -- --export-collision=assets/maps/san-ignacio-collision.json
```

The host includes this bake; a headless renderer test verifies it against the scene. The ready door is a separate dynamic host collider.

The optional command-line peer still works:

```sh
cargo run --locked -p border-net --bin border-peer -- http://127.0.0.1:8090 Officer-A
```

Set the invite through `BORDER_HOST_JOIN_CODE`. It keeps its token private and accepts `/move X Z`, `/stop`, `/hold`, `/release`, `/snapshot`, `/events` and newline JSON core commands. Movement and ready intent are pulsed automatically. It leaves on EOF.

Rust/Bevy pins remain Rust 1.94.0, Bevy 0.17.3 and bevy_egui 0.38.1 with the committed lockfile. These are project pins, not a claim to be the latest releases. Full replication/reconnect/audio polish and the complete multiplayer acceptance pass remain Part 15; production Internet transport remains later security work.

Part 6 adds physical `inspect_vehicle_point` commands, shared inspected-panel state, a vehicle-size catalog and separate entry/exit gate state. See [vehicle lifecycle and controls](VEHICLES.md).

Protocol 4 adds authoritative pistol/projectile state, door/window commands, physical traveler exit/return, damage and disabled-vehicle recovery. See [combat and vehicle mechanics](COMBAT.md).

Protocol 5 adds the six vehicle kinds, `pull_body`, per-bone ragdoll transforms and bounded physical shard snapshots. Rapier runs on the host at fixed 60 Hz. Readiness requires two seconds; graphical movement predicts with shared collision rules and reconciles against 20 Hz host snapshots. See [physics details](PHYSICS.md).

Protocol 6 adds compartment observations, locations, reveal rules, contradiction links, secondary requests and a distinct detain outcome. See [Parts 7–10](PARTS_7_10.md). Old clients must rebuild with their host.
