# San Ignacio slice contract — Part 1

Status: scope frozen for the first two-player slice. This document commits the build order and acceptance conditions; it does not certify that the complete slice already exists.

## Product boundary

San Ignacio is a fictional Texas border crossing, with a stylized desert setting. The first release proves a compact, entertaining checkpoint with exactly two players and two lanes. Convincing traveler conversations and interactions take priority over elaborate graphics. The engineering reference is six CPU cores, 16 GB system RAM and an RTX 2060 with 6 GB VRAM; performance on that machine remains an acceptance test, not a result inferred from another GPU.

Two friends must be able to begin a shift, interview travelers, physically inspect documents and cargo, identify inconsistencies, admit/deny/detain travelers, earn or lose shared money and purchase an upgrade. A traveler’s personality and spoken claims may change; the underlying case truth must remain immutable. The initial authored pack contains ten cases, seven honest and three smuggling.

## Acceptance contract

Each condition has an observable pass/fail test and an implementation home in the README. Expansion is gated on all of them passing.

| Gate | Required observation | Build Parts |
| --- | --- | --- |
| Environment | Both players can navigate the office and two lanes, reach inspection areas, and interact with vehicle and cargo points. | 4, 6 |
| Shift | The host or ready majority holds E at the exit door for three seconds; both clients show the same ten-minute clock. | 5, 15 |
| Cases | Ten valid authored cases load; a fixed seed produces the same order. Hidden truth is absent from peer snapshots. | 2, 3, 7 |
| Conversation | Travelers answer the actual question, remember earlier exchanges, maintain personalities and disclose only permitted information. Model output cannot award money or decide a case. | 3, 8 |
| Voice | Each player has independent continuous local speech recognition and natural speech playback. Pauses shorter than 1.5 seconds retain the turn; 20 seconds forces submission. | 9, 15 |
| Inspection | Players physically inspect documents/cargo, discover evidence and compare it to claims. Inspection changes knowledge, never truth. | 10 |
| Disposition | The host accepts an authorized action once, computes the outcome from hidden truth, moves the vehicle and updates both clients. | 3, 10, 15 |
| Incidents | Health, incapacitation, fleeing and the initial incident responses resolve under host authority. | 11 |
| Finances | Both players see the same ledger, quota, settlement and upgrade cost; duplicated requests cannot duplicate rewards or purchases. | 3, 12–14 |
| Full loop | Two graphical clients complete every step of the README’s 20-step playthrough with matching world state, including reconnect handling. | 15 |
| Quality | The README Part 16 conversation, fairness, pacing, usability and reference-hardware acceptance checks pass in human playtests. | 16 |

## Stop line

Parts 1–3 establish the contract, crate boundaries and executable host foundation. Parts 4–16 deliver and validate the complete graphical two-player loop. Parts 17–20 (six-player expansion, full procedural library, production security and advanced systems) begin only after that slice passes. Any unfinished acceptance condition stays visible; a headless protocol test does not replace the full two-client playthrough.

Normal play through `./play.sh` now uses the shared host session; a second local officer joins with `./play.sh --join`. Parts 1–5 provide the two-player office, ready door, host clock and two lanes; full multiplayer acceptance remains Part 15. New scope should update this contract and the relevant roadmap gate before implementation.
