# Parts 1–3 validation

Validated on Linux, 29 October 2026, using Rust 1.94.0 and the committed lockfile.

| Check | Result |
| --- | --- |
| `cargo test --locked --workspace` | 33 passed; one existing microphone/Whisper fixture test ignored because it requires explicit hardware/model inputs |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `python3 scripts/test_host.py` | Passed: real host, two CLI peer processes and offline provider gateway; identical public snapshots, host-only operations, actor validation/stale rejection and disconnect cleanup |
| `python3 scripts/smoke_api.py` | Passed: all ten cases with 30 exchanges each, evidence, decisions, auth, malformed input, hidden-fact validation, committed-state guards and turn budget |
| README preservation check | All 306 original checklist items preserved in the same order; only the 20 items in Parts 1–3 checked |

New tests cover exclusive lanes, two ready peers, private state isolation, exact request replay, altered request-ID rejection, no double settlement, stale revisions, invalid command sequence, deterministic seeds/ticks, evidence authorization, host-owned health/incidents/purchases, heartbeat expiry, bounded event replay/resync and rejected actor actions.

These results certify the Parts 1–3 contract and headless foundations. They do not certify Windows execution, RTX 2060 performance, a new live-model benchmark, graphical two-player gameplay, physical search distance checks, weapons, production Internet security or reconnect. Those remain visible in their later roadmap Parts. The host uses explicit deterministic operator ticks until the Part 5 clock is connected; the new multiplayer provider bridge currently uses public-fact disclosure until evidence-aware interviews are integrated.
