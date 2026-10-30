# Border Patrol with Friends — Technical Build Roadmap

**Setting:** San Ignacio, a fictional border crossing in Texas, with stylized desert-border scenery.

**Reference hardware:** 6 CPU cores, 16 GB RAM and an RTX 2060 with 6 GB VRAM. This is the engineering target, not a certified minimum specification.

The Parts below are **technical work packages in implementation order**, not the chapter order of the design PDF. **Parts 1–16 build and validate the complete two-player vertical slice. Parts 17–20 expand it into the full six-lane, 1–6-player game.** Follow the dependencies and exit gates before moving to the next scope.

Every checklist detail from the unabridged design is retained, including repeated requirements, the full 20-step playthrough, all acceptance criteria, numerical targets, examples and deferred features. Original M1–M7 labels remain as source milestone checks; the numbered Parts define this roadmap’s build order.

**Checkbox convention:** boxes track the deliverable of each technical Part. Part 1 checks the frozen scope contract; Parts 2–3 check executable foundations. Later gameplay acceptance remains unchecked until integrated and tested. They do not imply that no prototype code exists. For the current prototype’s terminal commands, controls, AI/voice setup and validation links, see [Running the current prototype](docs/PLAYING.md).


**Build status:** Parts 1–6 complete. See the [slice contract](docs/SCOPE_CONTRACT.md) and [host commands, protocol and validation](docs/HOST.md). Run `python3 scripts/test_host.py` to exercise a real host, two peers and the AI gateway bridge. Part 4 adds the [explorable San Ignacio river checkpoint](docs/SCENE.md) to `./play.sh`. Part 5 connects [shared office spawning, the ready door, clock and two-player lane controls](docs/SHIFT.md). Part 6 adds [mixed sedan/pickup queues, physical inspection points and four barriers](docs/VEHICLES.md). [Projectile combat and interactive vehicle glass/doors](docs/COMBAT.md) are also available. [Smooth movement, aiming, physical glass/ragdolls and six vehicle bodies](docs/PHYSICS.md) extend that prototype. Full multiplayer acceptance/hardening remains Part 15.

## Part 1 — Freeze the technical scope and acceptance contract

**Depends on:** None.

**Technical deliverable:** Lock the San Ignacio setting, the two-player first release and the conditions that must be met before full-game expansion. The full-game six-player target is implemented in Part 17, not here.

- [x] Set the game at **San Ignacio**, a fictional border crossing in **Texas**. This theming update supersedes the original design's unnamed location.
- [x] First deliver the deliberately smaller **two-player vertical slice** to prove the core checkpoint loop before implementing everything.
- [x] Build a compact but complete two-player border checkpoint.
- [x] Let two friends begin a shift, question travelers, physically inspect cargo, discover inconsistencies, decide who crosses, share money and purchase an upgrade.
- [x] Prioritize entertaining interactions and convincing AI behavior over elaborate graphics.

**Completion meaning:** The requirements below are frozen scope commitments, not a claim that the full checkpoint is already playable. Actual feature delivery is tracked in Parts 4–16.

**Exit gate:** A written slice contract and testable acceptance gates exist; the team agrees to prove the core loop before expanding.

## Part 2 — Establish the Rust workspace and headless data contracts

**Depends on:** Part 1.

**Technical deliverable:** Define the Bevy client, authoritative simulation, network protocol, scenario types, provider gateway and local speech boundaries before connecting gameplay systems.

- [x] Separate authoritative scenario facts, traveler knowledge, public claims, observed evidence and conversational state.
- [x] Use versioned network messages, replayable event IDs, timeouts and deterministic scenario seeds to aid debugging.
- [x] Structure the Rust workspace with `crates/game` for Bevy rendering and interactions.
- [x] Include `simulation` for headless authoritative rules and `net` for the network protocol.
- [x] Include `scenarios` for content packs and validation.
- [x] Include `caller` for prompt/state and `caller-server` for the provider gateway.
- [x] Include `speech` for local audio adapters.
- [x] Reuse design lessons from Wrong Number without reusing dispatch-specific game logic.
- [x] Verify Rust/Bevy dependency versions before pinning them.

**Exit gate:** The workspace builds with verified dependency pins, and the core world/scenario/event contracts can be exercised without the renderer.

## Part 3 — Build host sessions, command authority and protocol foundations

**Depends on:** Part 2.

**Technical deliverable:** Implement the host/client session and command boundary early. Establish who may change world state and how approved commands/events are versioned. The complete replicated gameplay is hardened in Part 15.

- [x] Never let a client send its own guilt verdict or money award to the host.
- [x] Make shifts, lane ownership, queues and vehicle motion host-authoritative.
- [x] Make hidden truth, evidence and financial results host-authoritative.
- [x] Make combat, health, border incidents and purchases host-authoritative.
- [x] Use a trusted AI service to validate conversational output; forward only approved actions to the host.
- [x] Never trust client-provided rewards or disposition outcomes.

**Exit gate:** Two peers can join a host-owned session; clients submit requests rather than trusted outcomes or rewards.

## Part 4 — Block out the San Ignacio checkpoint and interaction spaces

**Depends on:** Parts 1–3.

**Technical deliverable:** Build the initial office and two-lane scene, inspection areas, navigable bounds and placeholder art. Reserve space for later perimeter and six-lane expansion without building the whole world yet.

- [x] Establish a stylized, low-poly, chaotic and comedic tone rather than a realistic policy simulation.
- [x] Include car queues, barriers and inspection bays.
- [x] Include a detention area, administration office and equipment storage.
- [x] Use distant rocky ridges and outcrops, dust, haze and simplified scenery to make the fence appear to extend for miles without simulating miles of terrain.
- [x] Keep the surrounding landscape mostly as set dressing and make actual navigable bounds explicit.
- [x] Use intentionally low-poly art with good lighting, cohesive materials and readable silhouettes.
- [x] Span the river with a supported road bridge and place the long border fence on the Texas bank, with a checkpoint entrance at the bridge.
- [x] Add branching cacti, desert bushes, rocks, riverbank detail and navigable exploration routes through the checkpoint and bridge.

**Art direction update:** Keep the compound open with no enclosure fences; retain only the riverbank border wall and bridge guardrails. Use a clean textured cinderblock office and clean automotive paint. The latest direction supersedes the original dunes: use rough sandy terrain, chipped rocks, dry trees/scrub, weathered surfaces and detailed clouds. Roads continue beyond the site and bend into distant haze.

**Validation:** [Scene inventory, traffic, controls, screenshots and checks](docs/SCENE.md). The shared ready door and host-driven gameplay are implemented in Part 5.

**Exit gate:** The compact scene loads with clear lane, office, inspection and detention spaces; the art and map boundaries are readable.

## Part 5 — Implement player interaction, ready-up and the shift clock

**Depends on:** Parts 3–4.

**Technical deliverable:** Connect movement and E/hold-E interactions to the office door, ready votes, spawning and the host-owned shift state machine. Queue/event activation hooks are established here; their content follows later.

- [x] Spawn everyone in the shared office, healed and equipped.
- [x] Give the exit door a hold-E interaction with a visible two-second progress bar.
- [x] Allow the host to complete the hold, or allow a majority of connected players to register ready.
- [x] Have the host validate the ready threshold, open the door and start the authoritative ten-minute timer.
- [x] Activate vehicle queues and random security events when the shift begins.
- [x] Allow players to choose lanes, swap lanes and assist one another.
- [x] **M1:** room, lanes, door, movement and simple low-poly environment.
- [x] One shared administration office and one working two-second hold-E ready-up door.
- [x] **M1 — Environment blockout:** office, ready door, two lanes, barriers and basic cars.

**Validation:** [Launch/join, controls, authority rules and two-client checks](docs/SHIFT.md). Random security scheduling currently activates radio/dispatch hooks; its combat/breach content stays in later Parts.

**Exit gate:** The host or ready majority can start a shift through the two-second door interaction, and both clients agree on the ten-minute clock.

## Part 6 — Implement lane and vehicle lifecycle systems

**Implemented:** Two host-driven lanes each process mixed sedan/pickup queues, with vehicle-sized spacing, separate entry/exit barriers, driver-window conversation and physical passenger/hood/cargo inspection points.

**Depends on:** Parts 3–5.

**Technical deliverable:** Build reusable queue, lane reservation, arrival, stopping, driver-window interaction and departure components for the sedan and truck. Use test disposition commands until the complete inspection/decision loop lands in Part 10.

- [x] Give every vehicle type appropriate doors, cargo points, inspection positions and departure paths.
- [x] Queue each vehicle, approach a free lane and stop at its designated inspection point.
- [x] Let the officer approach the driver and press E to start microphone or text conversation.
- [x] Advance the next car while other lanes operate simultaneously.
- [x] Deliver one sedan and one truck with correct queueing, stopping and exit paths.
- [x] Two independently operable inspection lanes with entry and exit barriers.
- [x] One sedan and one truck with stopping, inspection and departure paths.

**Validation:** [Vehicle catalog, controls, lifecycle rules and two-client acceptance](docs/VEHICLES.md). Both vehicle types complete both departure paths; the full decision workflow remains Part 10.

**Exit gate:** Two lanes can process vehicle lifecycle transitions without blocking one another; each initial vehicle has valid interaction and exit points.

## Part 7 — Author the first ten cases and immutable truth model

**Depends on:** Parts 2 and 6.

**Technical deliverable:** Create the ten authored cases, case IDs, seeds, hidden truth and initial personality/knowledge data before the LLM sees them. Include a small generation proof of concept; defer the full content-library counts to Part 18.

- [ ] Give the traveler authored hidden truth, a procedural personality and an initial story.
- [ ] Deliver ten authored scenarios: **seven honest and three smuggling**, each with distinct personality and evidence.
- [ ] Deliver a small procedural personality-combination proof of concept without requiring the full 100/20/200/500 library to ship the slice.
- [ ] Generate a seedable traveler record before AI interaction.
- [ ] Detect incompatible role/reason/cargo combinations.
- [ ] Detect duplicate names within a session, contradictory documents and accidental leakage of hidden truth.
- [ ] Use authored templates plus controlled variation rather than raw unrestricted LLM world generation.
- [ ] Ten authored traveler scenarios, including honest, deceptive, rude, anxious and bribery-capable personalities.
- [ ] Create `case_templates.json` with truthful and deceptive archetypes, canonical truth, evidence and contradiction rules.

**Exit gate:** All ten cases load and validate; legal status, evidence and compatibility are fixed before an interview begins.

## Part 8 — Implement text AI, personality state and the trusted gateway

**Depends on:** Parts 3 and 7.

**Technical deliverable:** Connect traveler-facing knowledge and real dialogue history to the hosted LLM through the trusted caller service. Keep material truth authoritative and test conversation intent before adding audio complexity.

- [ ] Make entertaining improvised conversations a core pillar.
- [ ] Treat the LLM as an improvisational actor, not the source of truth.
- [ ] Have it directly address player questions, recognize jokes, sarcasm, insults, repetition and accusations, and vary its tone.
- [ ] Allow emotional expression, refusal to answer, argument, deflection, bargaining, attempted bribery and requests for a supervisor.
- [ ] Prevent invention of new decisive cargo, documents or destinations.
- [ ] Do not force a confession when evidence is shown.
- [ ] Track frustration, confidence, suspicion and cooperation as evolving state variables.
- [ ] Permit innocuous conversational improvisation while keeping material claims coherent.
- [ ] Detect and reject unwanted exact duplicate output, test semantic repetition, and avoid hard-coded catchphrases appearing every turn.
- [ ] Use a trusted provider gateway to call an inexpensive hosted LLM, validate structured output and maintain dialogue history.
- [ ] Protect shared API credentials on a trusted server rather than in game clients.
- [ ] Give every traveler a distinct biography, a coherent purpose for crossing, documents and cargo consistent with hidden truth, a personality, and memory of the current interview.
- [ ] Answer the officer's actual question instead of simply matching keywords or repeating a canned cargo line.
- [ ] Recognize sarcasm, accusations, insults, absurd questions and attempts to provoke a reaction.
- [ ] Let a repeatedly insulted traveler become angry, sarcastic, silent or evasive, or demand a supervisor.
- [ ] Let a smuggler lie convincingly, make mistakes or deny evidence; never make confession mandatory.
- [ ] Keep a rude or prejudiced honest traveler honest.
- [ ] Keep material scenario facts authoritative and prevent model output from changing them.
- [ ] Detect exact repetition and evaluate semantic repetition in testing.
- [ ] Avoid forced-confession instructions, exhaustive factual dialogue lists and scripted fallback text disguised as generative AI.
- [ ] Let the LLM improvise harmless personality and conversational details.
- [ ] Use AI to enrich these interactions rather than substitute for them.

**Exit gate:** Typed interviews preserve facts and memory, support varied reactions and expose provider failures without inventing a successful conversation.

## Part 9 — Integrate local speech, subtitles and responsive client UI

**Depends on:** Part 8.

**Technical deliverable:** Add persistent local STT/TTS, microphone capture, playback, subtitles and typed fallback on each client. Preserve the current always-on interaction, 1.5-second silence endpoint and 20-second utterance limit. Validate against the six-core/16 GB/RTX 2060 reference budget.

- [ ] Run local speech-to-text and text-to-speech on each player's PC.
- [ ] Support subtitles and typed input.
- [ ] Have client PCs handle local capture, STT, TTS and UI.
- [ ] **M3:** AI conversations, emotion and memory, local STT/TTS and typed fallback.
- [ ] Audio processing does not freeze rendering.
- [ ] Microphone-driven AI dialogue, local STT and TTS, subtitles and typed-input fallback.
- [ ] Have clients capture and synthesize their own audio and submit requests.
- [ ] **M3 — AI conversations:** personality, memory, local STT/TTS, provider validation and latency controls.

**Exit gate:** Each player can speak and hear their own interview without freezing rendering or mixing up another player’s audio.

## Part 10 — Build physical inspections, evidence and disposition rules

**Depends on:** Parts 6–9.

**Technical deliverable:** Connect document/cargo locations and reveal rules to physical searches, contradictions and host-validated admit/deny/detain commands. Finish the vehicle-processing and evidence milestone gates here.

- [ ] Make evidence-based decisions a core pillar.
- [ ] Make tangible vehicle and cargo interactions a core pillar.
- [ ] Let the officer ask questions, examine documents, search the vehicle and cargo, and optionally perform secondary inspection.
- [ ] Offer admit, deny and detain decisions.
- [ ] Have the host validate evidence, apply rewards or penalties, and move the car along the correct exit path.
- [ ] Let players inspect documents and cargo manifests.
- [ ] Let players inspect trunks, glove compartments, trailers and containers.
- [ ] Make physical evidence matter more than forced AI admissions.
- [ ] Give evidence an ID, location, reveal condition and contradiction links.
- [ ] Have the server determine actual outcomes.
- [ ] **M2:** vehicle queue, E interaction, inspection and disposition.
- [ ] **M4:** evidence and contradictions, immutable truth and scoring.
- [ ] Smugglers can be detected by evidence without confessing.
- [ ] Document inspection, basic cargo search and secondary inspection.
- [ ] Admit, deny and detain decisions validated by the host.
- [ ] **M2 — Vehicle processing:** arrival, stopping, E interaction, inspection, decisions and departure.
- [ ] **M4 — Evidence:** documents, cargo, contradictions and immutable truth.

**Exit gate:** A player can discover a fair evidence path, commit one valid disposition and send the vehicle down the correct exit route.

## Part 11 — Add starting equipment, health and one resistance encounter

**Implemented ahead of this milestone:** A [projectile pistol, articulated doors/windows, destructible glazing and physical NPC exit/death states](docs/COMBAT.md) are available now. The complete taser/handcuff, resistance, spectator and force-accounting requirements below remain open.

**Depends on:** Part 10.

**Technical deliverable:** Implement placeholder equipment interactions, restraint/nonlethal response and host-owned health for the slice. Demonstrate one fleeing/resistance event; defer advanced combat and rare explosions to Part 19.

- [ ] Start every guard with a pistol, taser and handcuffs.
- [ ] Heavily penalize unnecessary force.
- [ ] Have incapacitated players spectate for the remainder of the day.
- [ ] Heal and respawn all players for the next day.
- [ ] Deliver basic handcuffs, taser and pistol with placeholder models, plus at least one nonlethal fleeing or resistance encounter.
- [ ] Start players with a pistol, taser, handcuffs and a basic inspection tool.
- [ ] Use placeholder models and simplified animations.
- [ ] Implement at least one scripted or scenario-driven attempt to flee or resist detention.
- [ ] Defer advanced weapons, complex combat and a full inventory beyond the vertical slice.

**Exit gate:** The basic equipment set works in a bounded encounter, unnecessary force is recorded, and incapacitation/respawn rules have authoritative state.

## Part 12 — Implement the shared ledger, quota and live performance HUD

**Depends on:** Parts 10–11.

**Technical deliverable:** Apply host-calculated income, rewards, penalties and quota progress from verified disposition/incident events. Keep the illustrative values configurable for playtesting.

- [ ] Give the team a shared quota and shared finances.
- [ ] Penalize wrongful denial of honest travelers.
- [ ] Pay income for correctly admitting lawful travelers.
- [ ] Reward catching prohibited cargo.
- [ ] Allow bribery to offer immediate gains while risking audits and fines.
- [ ] Give each day a shared quota.
- [ ] Use **+$100 for a correct admission** as an illustrative starting value.
- [ ] Use **+$200 for correctly denying a smuggler** as an illustrative starting value.
- [ ] Use **+$250 for justified detention** as an illustrative starting value.
- [ ] Use **−$400 for admitting a smuggler** as an illustrative starting value.
- [ ] Use **−$150 for wrongful denial** as an illustrative starting value.
- [ ] Adjust these values through playtests rather than treating them as final balance.
- [ ] Apply a strike or setback for missing quota; end the run after repeated misses.
- [ ] Show current earnings, quota, correctly processed travelers, penalties and remaining time during the shift.

**Exit gate:** Both players see consistent earnings, penalties, quota and time; clients cannot supply their own reward totals.

## Part 13 — Implement shift closure and the shared review screen

**Depends on:** Parts 5 and 12.

**Technical deliverable:** Stop new arrivals, resolve the grace period and build the office review from authoritative events. Include explanations for reward and penalty decisions.

- [ ] End the day after ten minutes, with a grace period for active interactions.
- [ ] Stop new vehicle arrivals at the ten-minute mark.
- [ ] Close the day after a short grace period.
- [ ] Return players to the office for review.
- [ ] Show travelers processed and correct admissions and denials.
- [ ] Show smugglers caught and missed, plus unauthorized crossings.
- [ ] Show money earned and lost, damage, player deaths and quota grade.
- [ ] Explain at review why each decision was rewarded or penalized.

**Exit gate:** The timed shift closes cleanly and both players can review the same complete results.

## Part 14 — Implement one real upgrade, voting and next-day persistence

**Depends on:** Part 13.

**Technical deliverable:** Build the shared purchase/vote flow, one gameplay-changing inspection upgrade, within-run persistence and a clean return to ready-up with restored health. Save the full upgrade catalog for Part 20.

- [ ] Provide meaningful shared upgrades as a core pillar.
- [ ] Let the team spend shared money on upgrades at the end of each day.
- [ ] Make upgrades change real interactions, not only numbers.
- [ ] Let players vote on purchases and have the host resolve ties.
- [ ] Hold the upgrade vote, heal everyone, and start the next day at the office door.
- [ ] **M6:** quota, debrief, one upgrade and next-day reset.
- [ ] Money and upgrades persist between days within a run.
- [ ] At least one upgrade affects gameplay.
- [ ] Implement one meaningful upgrade, such as a document scanner that reveals additional verification data during the next shift.
- [ ] Do not count an upgrade that only changes a displayed number as satisfying acceptance.
- [ ] **M6 — Economy:** quota, penalties, review and one functional upgrade.

**Exit gate:** An upgrade purchased with shared funds changes an interaction in the next shift; a new day can start without developer intervention.

## Part 15 — Integrate the complete two-player loop and harden replication

**Depends on:** Parts 3–14.

**Technical deliverable:** Exercise every implemented subsystem together, validate replicated state under simultaneous actions, and handle disconnect/reassignment. Run the complete office-to-next-day sequence before expanding the game.

### Complete slice feature integration

- [ ] Give each player a lane where they question travelers through AI conversation, check documents and cargo, and admit, deny or detain people.
- [ ] Make multiplayer interference and coordination a core pillar.
- [ ] Deliver one administration office, a ready-up door, two lanes, two-player networking and a synchronized ten-minute shift.
- [ ] Deliver natural AI dialogue, local STT/TTS, typed input, subtitles, conversation memory and cost controls.
- [ ] Deliver document and cargo inspection, contradictions, admit/deny decisions, shared money and a quota.
- [ ] Deliver one performance review and one functional upgrade affecting the next shift.
- [ ] Two-player host-authoritative multiplayer and a synchronized ten-minute shift clock.
- [ ] Shared earnings, a quota, a review screen and one purchasable gameplay-changing upgrade.

### Playthrough summary without developer intervention

- [ ] Have two players join and both spawn in the office.
- [ ] Have the host or majority hold E for two seconds, open the door and begin the timer.
- [ ] Bring in cars and have each player interview a driver, search evidence and decide a disposition.
- [ ] Have the host evaluate outcomes and advance queues while players process multiple cars.
- [ ] End the shift and let both players review shared results.
- [ ] Let the players purchase an upgrade and start the next day.
- [ ] Complete this entire sequence without developer intervention.

### Original multiplayer milestone checkpoints

- [ ] **M5:** host-authoritative two-player co-op and reconnect handling.
- [ ] **M5 — Multiplayer:** host authority, two-player state, independent local audio and reconnect handling.

### Original cross-client acceptance checks

- [ ] Two players complete a full shift.
- [ ] AI responds to unexpected questions without repeating canned answers.
- [ ] Honest, nervous travelers are not systematically treated as guilty.
- [ ] Host and client agree on results.

### Required 20-step end-to-end playthrough

- [ ] **Step 1:** two players join a lobby hosted by one player's PC.
- [ ] **Step 2:** both spawn in the administration office, healed and carrying starting equipment.
- [ ] **Step 3:** either the host or a majority holds E on the exit door for two seconds.
- [ ] **Step 4:** the door opens and the authoritative ten-minute shift begins.
- [ ] **Step 5:** both players move to inspection lanes.
- [ ] **Step 6:** vehicles arrive, queue and stop at inspection positions.
- [ ] **Step 7:** each player walks to the driver window and presses E.
- [ ] **Step 8:** travelers begin AI-powered conversations.
- [ ] **Step 9:** players speak naturally through microphones, with text input still available.
- [ ] **Step 10:** travelers respond using locally synthesized speech and subtitles.
- [ ] **Step 11:** players inspect travel documents and vehicle cargo.
- [ ] **Step 12:** players compare statements with evidence and decide admit, deny or detain.
- [ ] **Step 13:** the host evaluates decisions against immutable hidden truth.
- [ ] **Step 14:** vehicles leave through the correct exit route.
- [ ] **Step 15:** the next vehicles advance and the cycle repeats.
- [ ] **Step 16:** the shift stops accepting new vehicles after ten minutes.
- [ ] **Step 17:** the game calculates income, penalties and quota results.
- [ ] **Step 18:** players return to the office for a shared performance review.
- [ ] **Step 19:** the team purchases an upgrade with shared funds.
- [ ] **Step 20:** the next shift becomes available with restored player health.

### Replication, authority and disconnect acceptance

- [ ] Give the host ownership of shift progression, vehicle queues, scenario assignment, immutable truth, decisions and money.
- [ ] Give the host ownership of combat, upgrades and player health.
- [ ] Reject duplicate dispositions, conflicting lane ownership, client-supplied reward totals and unauthorized state changes.
- [ ] Keep two connected clients synchronized on barrier states, vehicle positions, evidence discoveries and shared finances.
- [ ] Prevent a client disconnect from corrupting the shift.
- [ ] Let the host reassign the disconnected client's lane or leave it unattended.

**Exit gate:** Two real clients finish the entire 20-step playthrough with matching barriers, vehicles, evidence, money and host outcomes.

## Part 16 — Pass vertical-slice playtests, performance and fairness gates

**Depends on:** Part 15.

**Technical deliverable:** Run the full acceptance suite and real two-friend playtests. Fix intent, repetition, pacing, fairness, failure diagnostics and performance before beginning Parts 17–20.

### Original playtesting milestone checkpoints

- [ ] **M7:** playtest conversation quality and enjoyment; fix repetition, pacing and fairness before expanding.
- [ ] **M7 — Playtesting:** human conversations, fair evidence, performance and clip potential.

### Core enjoyment and expansion gate

- [ ] Prove that two friends can have fun questioning, inspecting and arguing over a small number of travelers before rare explosions, a large progression system or full six-player features exist.
- [ ] Build the social and physical core first.
- [ ] Add complexity only after real playtests demonstrate that the core is entertaining.
- [ ] Prove that questioning unpredictable AI travelers while physically inspecting their vehicles is entertaining on its own.
- [ ] Let two friends argue about evidence, interrogate a rude or evasive traveler, search a trunk and make a meaningful decision.
- [ ] Achieve this without needing elaborate weapons, spectacular explosions or a large progression tree.
- [ ] Make proving an enjoyable two-player checkpoint loop the first priority.

### The 15 final vertical-slice acceptance criteria

- [ ] **Criterion 1:** two players complete an entire shift together.
- [ ] **Criterion 2:** the ready-up system and countdown work reliably.
- [ ] **Criterion 3:** vehicles arrive, stop and depart without blocking lanes.
- [ ] **Criterion 4:** both players can interrogate separate travelers simultaneously.
- [ ] **Criterion 5:** AI responds naturally to unexpected and provocative questions.
- [ ] **Criterion 6:** travelers maintain scenario truth across 15–30 exchanges.
- [ ] **Criterion 7:** contradictions can be found through questioning and inspection.
- [ ] **Criterion 8:** correct and incorrect decisions update shared money consistently.
- [ ] **Criterion 9:** the quota system works and the review explains results.
- [ ] **Criterion 10:** the shift ends at ten minutes and a new day can begin.
- [ ] **Criterion 11:** an upgrade changes a later gameplay interaction.
- [ ] **Criterion 12:** networked state remains synchronized after simultaneous actions.
- [ ] **Criterion 13:** AI, STT and TTS work asynchronously without freezing Bevy.
- [ ] **Criterion 14:** two friends find the loop entertaining without requiring rare explosions.
- [ ] **Criterion 15:** failure modes are explicit; unavailable AI does not fabricate a successful conversation.

**Exit gate:** The two-player loop is entertaining and all listed acceptance checks pass; rare action and large content libraries are not needed to make it fun.

## Part 17 — Scale to six players, six lanes and the full vehicle roster

**Depends on:** Part 16.

**Technical deliverable:** Expand the already-proven session/lane systems to the full player and lane counts, then extend reusable vehicle components to the additional vehicle classes. Keep scenery mostly non-navigable set dressing.

- [ ] Let up to six friends operate San Ignacio, a dysfunctional fictional border crossing in Texas with desert-border scenery.
- [ ] Build a compact playable checkpoint with six booths and lanes.
- [ ] Include sedans, SUVs, pickups, vans, box trucks and semi-trucks.
- [ ] Plan future six-player expansion, the full procedural library, and more cars and cargo.
- [ ] Expand to all six lanes and full six-player synchronization after the slice.
- [ ] Defer miles of playable terrain and additional vehicle classes.

**Exit gate:** The complete six-lane layout and 1–6-player flow work with the expanded vehicle roster and appropriate interaction/departure paths.

## Part 18 — Build the full procedural libraries and generation validation

**Depends on:** Parts 7, 8 and 16; scheduled after the proven slice and initial scale-up.

**Technical deliverable:** Populate the four versioned libraries, implement tagged deterministic case construction and run large-sample consistency/diversity tests. Keep the examples and content rules from the design, rather than replacing them with unrestricted LLM generation.

### Library targets and combination count

- [ ] Maintain four independent, versioned libraries: 100 personality profiles, 20 occupations or roles, 200 crossing reasons or cover stories, and 500 names.
- [ ] Recognize that the naive Cartesian product gives 200,000,000 combinations, but sample only valid combinations.
- [ ] Treat these library sizes as targets for the complete game, not a claim that the repository already contains them.

### Deterministic generation pipeline

- [ ] Sample an incident archetype and immutable legal or illegal status first.
- [ ] Select a role and plausible crossing purpose using compatibility tags.
- [ ] Choose a name from a diverse, appropriate fictional name pool; names must never predict guilt.
- [ ] Choose a primary personality and optionally one or two modifiers.
- [ ] Generate consistent cargo, travel documents, itinerary, hidden motive, evidence and contradictions.
- [ ] Assign voice style, emotional baseline, patience, trust, greed and escalation thresholds.
- [ ] Validate internal consistency and assign a deterministic seed and unique case ID.
- [ ] Pass only traveler-visible knowledge to the LLM; keep hidden truth and scoring on the trusted host.

### The 20 suggested roles

- [ ] Tourist.
- [ ] Commuter.
- [ ] Delivery driver.
- [ ] Trucker.
- [ ] Mechanic.
- [ ] Student.
- [ ] Academic.
- [ ] Salesperson.
- [ ] Business executive.
- [ ] Parent.
- [ ] Retiree.
- [ ] Musician.
- [ ] Athlete.
- [ ] Construction worker.
- [ ] Farm worker.
- [ ] Medical worker.
- [ ] Contractor.
- [ ] Courier.
- [ ] Moving driver.
- [ ] Unemployed traveler.
- [ ] Ensure that role does not establish innocence or guilt.

### Personality dimensions and authored examples

- [ ] Author 100 distinctive profiles rather than 100 near-identical adjectives.
- [ ] Combine sociability, humor, pride, patience, defensiveness, anxiety, assertiveness, trust in authority, greed, honesty style and conflict response.
- [ ] Include a loudmouthed redneck with abrasive prejudices.
- [ ] Include an arrogant executive demanding exceptions.
- [ ] Include a sarcastic truck driver.
- [ ] Include a charming but deceptive courier.
- [ ] Include an anxious honest student.
- [ ] Include an overly talkative retiree.
- [ ] Include an aggressive bribe-offering criminal.
- [ ] Include a cooperative family traveler.
- [ ] Include a confused but honest tourist.
- [ ] Include a composed professional smuggler.
- [ ] Allow a fictional character to hold prejudiced or casually racist views and clash with a player, treating these as contextual individual flaws rather than characteristics of an entire demographic.
- [ ] Have characters respond naturally to criticism, teasing and unreasonable questions.
- [ ] Never use player or NPC demographics as a guilt classifier.

### The 200 crossing reasons and cover stories

- [ ] Organize the 200 entries into tagged categories: family visits, tourism, work, freight, relocation, repairs, events, study, medical visits, recreation, errands, emergency travel, deliveries, transit, personal purchases, business meetings, vehicle transfer, volunteering, temporary stays and suspicious cover stories.
- [ ] Give every reason required documents, plausible cargo, duration constraints and compatible roles.
- [ ] Allow false cover stories, but map them to specific immutable evidence.

### Example travelers: Hank Rudd and Elena Voss

- [ ] Support **Hank Rudd**: an American rural contractor who is loud, abrasive, proud, impatient and occasionally prejudiced; he claims to be driving to a family barbecue with tools.
- [ ] Keep Hank's hidden truth a lawful crossing with ordinary tools; he may argue and insult the officer without becoming guilty merely because he is unpleasant.
- [ ] Support the contrasting **Elena Voss**: a polished business executive who offers a bribe to avoid inspection of undeclared commercial samples, with hidden evidence in the cargo manifest.

### Versioned file formats, consistency and acceptance

- [ ] Store the target 100 personality profiles, 20 roles, 200 crossing reasons and 500 names in versioned data files with tags and schema validation.
- [ ] Automatically test a large sample of generated travelers for consistency and content diversity.
- [ ] Maintain 100 authored personality profiles, 20 roles, 200 crossing reasons and 500 names in versioned data files for the complete game.
- [ ] Treat these as explicit content targets, not a claim that those libraries already exist.
- [ ] Make each generated case deterministic under a supplied seed and pass compatibility checks before gameplay.
- [ ] Create `personality_profiles.json` with 100 records containing ID, traits, stress thresholds, conversational style and reaction rules.
- [ ] Create `roles.json` with 20 records containing occupation, typical documents, cargo compatibility and plausible reasons.
- [ ] Create `crossing_reasons.json` with 200 records containing tags, itinerary patterns, documents, cargo and potential cover-story conflicts.
- [ ] Create `names.json` with 500 diverse fictional names and no guilt correlation.
- [ ] Unit-test at least **10,000 deterministic combinations**, checking that names, cargo, itinerary, documents and reason tags remain consistent.
- [ ] Measure repetition across recent cases.
- [ ] Ensure that a character's demographic background does not predict whether they are a smuggler.
- [ ] Never trust a client-submitted generated case on the server.

**Exit gate:** The target 100/20/200/500 libraries exist; at least 10,000 sampled combinations pass the specified consistency and demographic-independence checks.

## Part 19 — Add perimeter incidents and expanded arcade combat

**Depends on:** Parts 11, 16 and 17.

**Technical deliverable:** Add the long-fence patrol loop, unattended-lane opportunities and infrequent escalating security events. Keep these additions subordinate to routine inspection.

- [ ] Allow players to leave their lanes to investigate fence breaches, with a risk of suspicious vehicles escaping.
- [ ] Include occasional surprising action as a core pillar.
- [ ] Include a long border fence, watchtowers and a perimeter patrol strip.
- [ ] Allow players to abandon lanes to assist with security events.
- [ ] Make unattended-lane escape opportunities depend on the traveler, barrier and time absent; do not make absence a guaranteed penalty.
- [ ] Occasionally have NPCs attempt fence crossings, creating alarms and a choice about abandoning a lane.
- [ ] Rarely have a traveler flee, attack or emerge armed.
- [ ] Extremely rarely have a truck explode, potentially incapacitating a guard for the shift.
- [ ] Keep these fictional arcade events infrequent, fairly telegraphed and subordinate to routine inspection.
- [ ] Plan future fence breaches, rare combat events and deeper upgrades.
- [ ] Defer large-scale fence breaches, advanced combat and vehicle explosions.

**Exit gate:** Security events create meaningful choices, share host-authoritative state and preserve fair warning, low frequency and force penalties.

## Part 20 — Expand upgrades, optional sharing features and full-game validation

**Depends on:** Parts 14 and 17–19.

**Technical deliverable:** Finish the advanced upgrade categories, optional exchange replays, Workshop/character content and opt-in voice features after the core game has passed its gates. Preserve the slice’s deferred-feature boundaries and verify the complete full-game experience.

### Advanced upgrade catalog and progression

- [ ] Add later upgrades for protective gear, additional weapons, better barriers, scanners and security systems.
- [ ] Provide **inspection upgrades**: document reader, cargo X-ray, drug detection and inspection mirrors.
- [ ] Provide **perimeter upgrades**: cameras, alarms, fence sensors and a watchtower.
- [ ] Provide **protection upgrades**: armor, taser, shotgun and rifle.
- [ ] Provide **operations upgrades**: faster barriers, cargo handling, communication and queue processing.
- [ ] Defer advanced X-ray, drug detection, watchtowers and permanent progression.

### Opt-in exchange and mistake replays

- [ ] With opt-in recording, replay the funniest AI exchanges and multiplayer mistakes.

### Complete-game acceptance

- [ ] Deliver the **full game** scope: six lanes, 1–6 players, multiple vehicles, security incidents, combat, quota progression, a large procedural character library and upgrades.
- [ ] Produce funny, unpredictable multiplayer moments through character conflict, physical inspections, shared quotas, bribery, unattended lanes, perimeter emergencies and occasional danger.

### Workshop, character uploads and permitted voice profiles

- [ ] Plan future Steam Workshop character packs and opt-in voice profiles.
- [ ] Require permission from the voice owner and a commercial-use-compatible TTS stack for voice cloning.
- [ ] Defer Steam Workshop, public character uploads and cloned voices.

**Exit gate:** The expanded systems change real interactions and produce the intended multiplayer moments without replacing the physical/social core with AI or spectacle.
