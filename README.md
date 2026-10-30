# Border Patrol with Friends

**Unabridged game design and development checklist — Draft 2, October 2026**

Rust / Bevy · Fictional desert checkpoint · Full game: 1–6 players and six lanes · Ten-minute shifts · Windows and Linux

This checklist preserves all 20 sections of the supplied *Border Patrol with Friends — Unabridged Design*, including the original complete gameplay concept, technical architecture, procedural traveler generation, and the expanded first-playable implementation and acceptance specification. Repeated requirements are retained where the design revisits them in greater detail.

**Checkbox convention:** every box starts unchecked for acceptance tracking against this new design. An unchecked box is not a claim that no prototype work exists, and no part is marked complete without its full acceptance being verified. The current single-lane prototype is not yet the complete two-player vertical slice or six-player game.

For the existing prototype's launch commands, controls, AI configuration, voice setup and validation links, see [Running the current prototype](docs/PLAYING.md).

## Part 1 — High-level concept

- [ ] Let up to six friends operate a dysfunctional fictional border crossing in a desert.
- [ ] Give each player a lane where they question travelers through AI conversation, check documents and cargo, and admit, deny or detain people.
- [ ] Give the team a shared quota and shared finances.
- [ ] Allow players to leave their lanes to investigate fence breaches, with a risk of suspicious vehicles escaping.
- [ ] Establish a stylized, low-poly, chaotic and comedic tone rather than a realistic policy simulation.
- [ ] Make entertaining improvised conversations a core pillar.
- [ ] Make evidence-based decisions a core pillar.
- [ ] Make tangible vehicle and cargo interactions a core pillar.
- [ ] Make multiplayer interference and coordination a core pillar.
- [ ] Include occasional surprising action as a core pillar.
- [ ] Provide meaningful shared upgrades as a core pillar.

## Part 2 — Setting and physical map

- [ ] Build a compact playable checkpoint with six booths and lanes.
- [ ] Include car queues, barriers and inspection bays.
- [ ] Include a detention area, administration office and equipment storage.
- [ ] Include a long border fence, watchtowers and a perimeter patrol strip.
- [ ] Use distant mountains, dunes, dust, haze and simplified scenery to make the fence appear to extend for miles without simulating miles of terrain.
- [ ] Keep the surrounding landscape mostly as set dressing and make actual navigable bounds explicit.
- [ ] Include sedans, SUVs, pickups, vans, box trucks and semi-trucks.
- [ ] Give every vehicle type appropriate doors, cargo points, inspection positions and departure paths.
- [ ] Use intentionally low-poly art with good lighting, cohesive materials and readable silhouettes.

## Part 3 — Shift start and ready-up

- [ ] Spawn everyone in the shared office, healed and equipped.
- [ ] Give the exit door a hold-E interaction with a visible three-second progress bar.
- [ ] Allow the host to complete the hold, or allow a majority of connected players to register ready.
- [ ] Have the host validate the ready threshold, open the door and start the authoritative ten-minute timer.
- [ ] Activate vehicle queues and random security events when the shift begins.
- [ ] Allow players to choose lanes, swap lanes and assist one another.

## Part 4 — Ten-minute core loop

- [ ] Queue each vehicle, approach a free lane and stop at its designated inspection point.
- [ ] Let the officer approach the driver and press E to start microphone or text conversation.
- [ ] Give the traveler authored hidden truth, a procedural personality and an initial story.
- [ ] Let the officer ask questions, examine documents, search the vehicle and cargo, and optionally perform secondary inspection.
- [ ] Offer admit, deny and detain decisions.
- [ ] Have the host validate evidence, apply rewards or penalties, and move the car along the correct exit path.
- [ ] Advance the next car while other lanes operate simultaneously.
- [ ] End the day after ten minutes, with a grace period for active interactions.
- [ ] Allow players to abandon lanes to assist with security events.
- [ ] Make unattended-lane escape opportunities depend on the traveler, barrier and time absent; do not make absence a guaranteed penalty.

## Part 5 — Traveler generation: 100 × 20 × 200 × 500

- [ ] Maintain four independent, versioned libraries: 100 personality profiles, 20 occupations or roles, 200 crossing reasons or cover stories, and 500 names.
- [ ] Recognize that the naive Cartesian product gives 200,000,000 combinations, but sample only valid combinations.
- [ ] Treat these library sizes as targets for the complete game, not a claim that the repository already contains them.

### Generation pipeline

- [ ] Sample an incident archetype and immutable legal or illegal status first.
- [ ] Select a role and plausible crossing purpose using compatibility tags.
- [ ] Choose a name from a diverse, appropriate fictional name pool; names must never predict guilt.
- [ ] Choose a primary personality and optionally one or two modifiers.
- [ ] Generate consistent cargo, travel documents, itinerary, hidden motive, evidence and contradictions.
- [ ] Assign voice style, emotional baseline, patience, trust, greed and escalation thresholds.
- [ ] Validate internal consistency and assign a deterministic seed and unique case ID.
- [ ] Pass only traveler-visible knowledge to the LLM; keep hidden truth and scoring on the trusted host.

### Suggested 20 roles

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

### Personality dimensions and examples

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

### 200 excuses and crossing reasons

- [ ] Organize the 200 entries into tagged categories: family visits, tourism, work, freight, relocation, repairs, events, study, medical visits, recreation, errands, emergency travel, deliveries, transit, personal purchases, business meetings, vehicle transfer, volunteering, temporary stays and suspicious cover stories.
- [ ] Give every reason required documents, plausible cargo, duration constraints and compatible roles.
- [ ] Allow false cover stories, but map them to specific immutable evidence.

### Example generated travelers

- [ ] Support **Hank Rudd**: an American rural contractor who is loud, abrasive, proud, impatient and occasionally prejudiced; he claims to be driving to a family barbecue with tools.
- [ ] Keep Hank's hidden truth a lawful crossing with ordinary tools; he may argue and insult the officer without becoming guilty merely because he is unpleasant.
- [ ] Support the contrasting **Elena Voss**: a polished business executive who offers a bribe to avoid inspection of undeclared commercial samples, with hidden evidence in the cargo manifest.

## Part 6 — Conversation AI design

- [ ] Treat the LLM as an improvisational actor, not the source of truth.
- [ ] Have it directly address player questions, recognize jokes, sarcasm, insults, repetition and accusations, and vary its tone.
- [ ] Allow emotional expression, refusal to answer, argument, deflection, bargaining, attempted bribery and requests for a supervisor.
- [ ] Prevent invention of new decisive cargo, documents or destinations.
- [ ] Do not force a confession when evidence is shown.
- [ ] Separate authoritative scenario facts, traveler knowledge, public claims, observed evidence and conversational state.
- [ ] Track frustration, confidence, suspicion and cooperation as evolving state variables.
- [ ] Permit innocuous conversational improvisation while keeping material claims coherent.
- [ ] Detect and reject unwanted exact duplicate output, test semantic repetition, and avoid hard-coded catchphrases appearing every turn.
- [ ] Run local speech-to-text and text-to-speech on each player's PC.
- [ ] Use a trusted provider gateway to call an inexpensive hosted LLM, validate structured output and maintain dialogue history.
- [ ] Support subtitles and typed input.
- [ ] Never let a client send its own guilt verdict or money award to the host.
- [ ] Protect shared API credentials on a trusted server rather than in game clients.

## Part 7 — Evidence, inspection and decisions

- [ ] Let players inspect documents and cargo manifests.
- [ ] Let players inspect trunks, glove compartments, trailers and containers.
- [ ] Make physical evidence matter more than forced AI admissions.
- [ ] Give evidence an ID, location, reveal condition and contradiction links.
- [ ] Penalize wrongful denial of honest travelers.
- [ ] Pay income for correctly admitting lawful travelers.
- [ ] Reward catching prohibited cargo.
- [ ] Allow bribery to offer immediate gains while risking audits and fines.
- [ ] Have the server determine actual outcomes.

## Part 8 — Security events, combat and health

- [ ] Occasionally have NPCs attempt fence crossings, creating alarms and a choice about abandoning a lane.
- [ ] Rarely have a traveler flee, attack or emerge armed.
- [ ] Extremely rarely have a truck explode, potentially incapacitating a guard for the shift.
- [ ] Keep these fictional arcade events infrequent, fairly telegraphed and subordinate to routine inspection.
- [ ] Start every guard with a pistol, taser and handcuffs.
- [ ] Heavily penalize unnecessary force.
- [ ] Have incapacitated players spectate for the remainder of the day.
- [ ] Heal and respawn all players for the next day.
- [ ] Add later upgrades for protective gear, additional weapons, better barriers, scanners and security systems.

## Part 9 — Economy and quotas

- [ ] Give each day a shared quota.
- [ ] Use **+$100 for a correct admission** as an illustrative starting value.
- [ ] Use **+$200 for correctly denying a smuggler** as an illustrative starting value.
- [ ] Use **+$250 for justified detention** as an illustrative starting value.
- [ ] Use **−$400 for admitting a smuggler** as an illustrative starting value.
- [ ] Use **−$150 for wrongful denial** as an illustrative starting value.
- [ ] Adjust these values through playtests rather than treating them as final balance.
- [ ] Apply a strike or setback for missing quota; end the run after repeated misses.
- [ ] Let the team spend shared money on upgrades at the end of each day.
- [ ] Provide **inspection upgrades**: document reader, cargo X-ray, drug detection and inspection mirrors.
- [ ] Provide **perimeter upgrades**: cameras, alarms, fence sensors and a watchtower.
- [ ] Provide **protection upgrades**: armor, taser, shotgun and rifle.
- [ ] Provide **operations upgrades**: faster barriers, cargo handling, communication and queue processing.
- [ ] Make upgrades change real interactions, not only numbers.
- [ ] Let players vote on purchases and have the host resolve ties.

## Part 10 — End-of-day review and reset

- [ ] Stop new vehicle arrivals at the ten-minute mark.
- [ ] Close the day after a short grace period.
- [ ] Return players to the office for review.
- [ ] Show travelers processed and correct admissions and denials.
- [ ] Show smugglers caught and missed, plus unauthorized crossings.
- [ ] Show money earned and lost, damage, player deaths and quota grade.
- [ ] With opt-in recording, replay the funniest AI exchanges and multiplayer mistakes.
- [ ] Hold the upgrade vote, heal everyone, and start the next day at the office door.

## Part 11 — Multiplayer architecture

- [ ] Make shifts, lane ownership, queues and vehicle motion host-authoritative.
- [ ] Make hidden truth, evidence and financial results host-authoritative.
- [ ] Make combat, health, border incidents and purchases host-authoritative.
- [ ] Have client PCs handle local capture, STT, TTS and UI.
- [ ] Use a trusted AI service to validate conversational output; forward only approved actions to the host.
- [ ] Use versioned network messages, replayable event IDs, timeouts and deterministic scenario seeds to aid debugging.
- [ ] Never trust client-provided rewards or disposition outcomes.
- [ ] Structure the Rust workspace with `crates/game` for Bevy rendering and interactions.
- [ ] Include `simulation` for headless authoritative rules and `net` for the network protocol.
- [ ] Include `scenarios` for content packs and validation.
- [ ] Include `caller` for prompt/state and `caller-server` for the provider gateway.
- [ ] Include `speech` for local audio adapters.
- [ ] Reuse design lessons from Wrong Number without reusing dispatch-specific game logic.
- [ ] Verify Rust/Bevy dependency versions before pinning them.

## Part 12 — Complete game versus vertical slice

- [ ] Deliver the **full game** scope: six lanes, 1–6 players, multiple vehicles, security incidents, combat, quota progression, a large procedural character library and upgrades.
- [ ] First deliver the deliberately smaller **two-player vertical slice** to prove the core checkpoint loop before implementing everything.

## Part 13 — First playable vertical slice: deliverables

- [ ] Deliver one administration office, a ready-up door, two lanes, two-player networking and a synchronized ten-minute shift.
- [ ] Deliver one sedan and one truck with correct queueing, stopping and exit paths.
- [ ] Deliver ten authored scenarios: **seven honest and three smuggling**, each with distinct personality and evidence.
- [ ] Deliver natural AI dialogue, local STT/TTS, typed input, subtitles, conversation memory and cost controls.
- [ ] Deliver document and cargo inspection, contradictions, admit/deny decisions, shared money and a quota.
- [ ] Deliver basic handcuffs, taser and pistol with placeholder models, plus at least one nonlethal fleeing or resistance encounter.
- [ ] Deliver one performance review and one functional upgrade affecting the next shift.
- [ ] Deliver a small procedural personality-combination proof of concept without requiring the full 100/20/200/500 library to ship the slice.

## Part 14 — Required vertical-slice playthrough

- [ ] Have two players join and both spawn in the office.
- [ ] Have the host or majority hold E for three seconds, open the door and begin the timer.
- [ ] Bring in cars and have each player interview a driver, search evidence and decide a disposition.
- [ ] Have the host evaluate outcomes and advance queues while players process multiple cars.
- [ ] End the shift and let both players review shared results.
- [ ] Let the players purchase an upgrade and start the next day.
- [ ] Complete this entire sequence without developer intervention.

## Part 15 — Milestones and acceptance

### Milestones

- [ ] **M1:** room, lanes, door, movement and simple low-poly environment.
- [ ] **M2:** vehicle queue, E interaction, inspection and disposition.
- [ ] **M3:** AI conversations, emotion and memory, local STT/TTS and typed fallback.
- [ ] **M4:** evidence and contradictions, immutable truth and scoring.
- [ ] **M5:** host-authoritative two-player co-op and reconnect handling.
- [ ] **M6:** quota, debrief, one upgrade and next-day reset.
- [ ] **M7:** playtest conversation quality and enjoyment; fix repetition, pacing and fairness before expanding.

### Acceptance

- [ ] Two players complete a full shift.
- [ ] AI responds to unexpected questions without repeating canned answers.
- [ ] Honest, nervous travelers are not systematically treated as guilty.
- [ ] Smugglers can be detected by evidence without confessing.
- [ ] Host and client agree on results.
- [ ] Audio processing does not freeze rendering.
- [ ] Money and upgrades persist between days within a run.
- [ ] At least one upgrade affects gameplay.

## Part 16 — Procedural generation validation and future content

- [ ] Store the target 100 personality profiles, 20 roles, 200 crossing reasons and 500 names in versioned data files with tags and schema validation.
- [ ] Generate a seedable traveler record before AI interaction.
- [ ] Detect incompatible role/reason/cargo combinations.
- [ ] Detect duplicate names within a session, contradictory documents and accidental leakage of hidden truth.
- [ ] Use authored templates plus controlled variation rather than raw unrestricted LLM world generation.
- [ ] Automatically test a large sample of generated travelers for consistency and content diversity.
- [ ] Plan future six-player expansion, the full procedural library, and more cars and cargo.
- [ ] Plan future fence breaches, rare combat events and deeper upgrades.
- [ ] Plan future Steam Workshop character packs and opt-in voice profiles.
- [ ] Require permission from the voice owner and a commercial-use-compatible TTS stack for voice cloning.

## Part 17 — Success criterion

- [ ] Prove that two friends can have fun questioning, inspecting and arguing over a small number of travelers before rare explosions, a large progression system or full six-player features exist.
- [ ] Build the social and physical core first.
- [ ] Add complexity only after real playtests demonstrate that the core is entertaining.

## Part 18 — Complete first playable vertical slice: expanded specification

This is the implementation-oriented continuation of the original gameplay design. It deliberately keeps the two-player scope and defines user-visible gameplay and pass/fail checks in full.

### 18.1 — Vertical slice objective

- [ ] Build a compact but complete two-player border checkpoint.
- [ ] Let two friends begin a shift, question travelers, physically inspect cargo, discover inconsistencies, decide who crosses, share money and purchase an upgrade.
- [ ] Prioritize entertaining interactions and convincing AI behavior over elaborate graphics.

### 18.2 — Required components

- [ ] One shared administration office and one working three-second hold-E ready-up door.
- [ ] Two independently operable inspection lanes with entry and exit barriers.
- [ ] Two-player host-authoritative multiplayer and a synchronized ten-minute shift clock.
- [ ] One sedan and one truck with stopping, inspection and departure paths.
- [ ] Ten authored traveler scenarios, including honest, deceptive, rude, anxious and bribery-capable personalities.
- [ ] Microphone-driven AI dialogue, local STT and TTS, subtitles and typed-input fallback.
- [ ] Document inspection, basic cargo search and secondary inspection.
- [ ] Admit, deny and detain decisions validated by the host.
- [ ] Shared earnings, a quota, a review screen and one purchasable gameplay-changing upgrade.

### 18.3 — Required complete gameplay sequence

- [ ] **Step 1:** two players join a lobby hosted by one player's PC.
- [ ] **Step 2:** both spawn in the administration office, healed and carrying starting equipment.
- [ ] **Step 3:** either the host or a majority holds E on the exit door for three seconds.
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

### 18.4 — AI acceptance requirements

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

### 18.5 — Multiplayer acceptance requirements

- [ ] Give the host ownership of shift progression, vehicle queues, scenario assignment, immutable truth, decisions and money.
- [ ] Give the host ownership of combat, upgrades and player health.
- [ ] Have clients capture and synthesize their own audio and submit requests.
- [ ] Reject duplicate dispositions, conflicting lane ownership, client-supplied reward totals and unauthorized state changes.
- [ ] Keep two connected clients synchronized on barrier states, vehicle positions, evidence discoveries and shared finances.
- [ ] Prevent a client disconnect from corrupting the shift.
- [ ] Let the host reassign the disconnected client's lane or leave it unattended.

### 18.6 — Equipment and physical interaction

- [ ] Start players with a pistol, taser, handcuffs and a basic inspection tool.
- [ ] Use placeholder models and simplified animations.
- [ ] Implement at least one scripted or scenario-driven attempt to flee or resist detention.
- [ ] Defer advanced weapons, complex combat and a full inventory beyond the vertical slice.

### 18.7 — Economy and upgrade demonstration

- [ ] Show current earnings, quota, correctly processed travelers, penalties and remaining time during the shift.
- [ ] Explain at review why each decision was rewarded or penalized.
- [ ] Implement one meaningful upgrade, such as a document scanner that reveals additional verification data during the next shift.
- [ ] Do not count an upgrade that only changes a displayed number as satisfying acceptance.

### 18.8 — Vertical slice success criteria

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

### 18.9 — Deferred features

- [ ] Expand to all six lanes and full six-player synchronization after the slice.
- [ ] Defer miles of playable terrain and additional vehicle classes.
- [ ] Defer large-scale fence breaches, advanced combat and vehicle explosions.
- [ ] Defer advanced X-ray, drug detection, watchtowers and permanent progression.
- [ ] Defer Steam Workshop, public character uploads and cloned voices.

### 18.10 — Implementation milestones

- [ ] **M1 — Environment blockout:** office, ready door, two lanes, barriers and basic cars.
- [ ] **M2 — Vehicle processing:** arrival, stopping, E interaction, inspection, decisions and departure.
- [ ] **M3 — AI conversations:** personality, memory, local STT/TTS, provider validation and latency controls.
- [ ] **M4 — Evidence:** documents, cargo, contradictions and immutable truth.
- [ ] **M5 — Multiplayer:** host authority, two-player state, independent local audio and reconnect handling.
- [ ] **M6 — Economy:** quota, penalties, review and one functional upgrade.
- [ ] **M7 — Playtesting:** human conversations, fair evidence, performance and clip potential.

### 18.11 — Final vertical slice requirement

- [ ] Prove that questioning unpredictable AI travelers while physically inspecting their vehicles is entertaining on its own.
- [ ] Let two friends argue about evidence, interrogate a rude or evasive traveler, search a trunk and make a meaningful decision.
- [ ] Achieve this without needing elaborate weapons, spectacular explosions or a large progression tree.

## Part 19 — Procedural content acceptance and data format

- [ ] Maintain 100 authored personality profiles, 20 roles, 200 crossing reasons and 500 names in versioned data files for the complete game.
- [ ] Treat these as explicit content targets, not a claim that those libraries already exist.
- [ ] Make each generated case deterministic under a supplied seed and pass compatibility checks before gameplay.
- [ ] Create `personality_profiles.json` with 100 records containing ID, traits, stress thresholds, conversational style and reaction rules.
- [ ] Create `roles.json` with 20 records containing occupation, typical documents, cargo compatibility and plausible reasons.
- [ ] Create `crossing_reasons.json` with 200 records containing tags, itinerary patterns, documents, cargo and potential cover-story conflicts.
- [ ] Create `names.json` with 500 diverse fictional names and no guilt correlation.
- [ ] Create `case_templates.json` with truthful and deceptive archetypes, canonical truth, evidence and contradiction rules.
- [ ] Unit-test at least **10,000 deterministic combinations**, checking that names, cargo, itinerary, documents and reason tags remain consistent.
- [ ] Measure repetition across recent cases.
- [ ] Ensure that a character's demographic background does not predict whether they are a smuggler.
- [ ] Never trust a client-submitted generated case on the server.

## Part 20 — Final game design principle

- [ ] Produce funny, unpredictable multiplayer moments through character conflict, physical inspections, shared quotas, bribery, unattended lanes, perimeter emergencies and occasional danger.
- [ ] Use AI to enrich these interactions rather than substitute for them.
- [ ] Make proving an enjoyable two-player checkpoint loop the first priority.
