# San Ignacio environment and timed traffic

San Ignacio now sits on the Texas bank of a desert river. The road crosses a supported bridge into the checkpoint through an opening in the border fence. The long border fence stays on the Texas bank; it does not run down the opposite bank or through the water.

![San Ignacio river checkpoint](san-ignacio-overview.webp)

![Rocky approach road and layered clouds](san-ignacio-landscape.webp)

![Clean cinderblock administration office](san-ignacio-office.webp)

![Clean automotive paint and detailed car geometry](san-ignacio-cars.webp)

## Explore

Launch the updated graphical prototype with:

```sh
./play.sh
```

Click **Begin patrol**, then use WASD and the mouse. Hold **left Shift** to jog. Press **M** for a site map with your position; press M again to close it. E still starts the existing interview when you aim at the active teal sedan within three metres. Tab switches between mouse look and typing, and Escape releases the mouse.

The administration office is west of the inspection lanes, storage is immediately farther west, the holding yard is behind the office, and secondary inspection is east of the lanes. The river bridge is south, behind the vehicle queue. Walk through the fence opening and across the bridge to the posted pedestrian limit. Vehicle roads continue beyond the walkable yard, bend into the rocky landscape and fade into haze. Posted signs mark the pedestrian limits; the compound itself is open, with no enclosure fencing; the surrounding desert and reserved future lanes are scenery.

## Scene inventory

| Space | Implemented detail |
| --- | --- |
| Two-lane checkpoint | Separate lane markings/arrows, stop lines, canopy beams, hanging fixtures, cameras, protective bollards, moving striped barriers and timed vehicle queues |
| Active interview stop | Ten shared-mesh sedans with clean varied automotive paint, chamfered bodies, sloped windscreens and metal wheel hubs, visible drivers, braking/queue spacing, arrival HUD, AI/text/voice conversation and evidence UI |
| Administration office | Clean textured cinderblock exterior with staggered courses, mortar grooves, subtle concrete pores and consistent block scale; hollow enterable room, doorway, window trim, roof, desks, terminals, chairs, documents, filing cabinets, bulletin board, coffee station, ceiling fixtures and warm interior lighting |
| Equipment storage | Open-front shelter, shelves, stacked crates, adjacent utility cabinet, service pipes, condensers and barrels |
| Holding yard | Open concrete pad with painted markings, shaded benches, nearby water tank and location sign; no surrounding fence |
| Secondary inspection | Broad marked concrete apron, open-sided roof, structural posts and work tables |
| River crossing | Separate riverbanks, lowered river channel, animated surface streaks, road deck, piers, guardrails, posts and diagonal supports |
| Border | Tall Texas-bank fence with a bridge opening, detailed close slats and inexpensive distant sections fading into haze |
| Desert | Ribbed cacti with spine clusters/flower tips, scrub and dead trees, loose twigs, scree, chipped angular rock variants, irregular sandy terrain, layered drifting clouds and dust plumes; no dunes |
| Readability | Clean car paint is separated from gritty road/terrain materials. Warm palette, directional shadows, distance fog, occluded location labels, humorous notices and site map; procedural grit, normal maps, fissures, stains, tire wear, concrete joints, rust, roof vents, roadside service poles and sagging wires |

The playable yard is approximately 64 × 73 metres, with an additional walkable bridge corridor. Compound fences have been removed; only the riverbank border wall remains. Bridge guardrails and traffic barriers stay in place. Movement stays at ground/deck level, with axis-aligned collision against structural walls and props. Enterable facilities and the driver approach are reachable from the spawn. Terrain outside the yard/bridge is intentionally non-navigable. The eastern reserved apron provides space for later expansion without adding operational lanes now.

Environment meshes and material handles are shared across repeated props and the vehicle pool. Rocks use seven deterministically deformed, chipped mesh variants; clouds use overlapping detailed lobes with baked cool shading. Small procedural albedo/normal maps add grit and surface variation to sand, concrete, asphalt, stone and metal; a dedicated masonry material gives the office readable block courses, while car paint stays clean without downloading texture packs. Terrain relief is strongest beyond developed areas, while walking routes and road beds stay clear and flat enough to navigate. This remains stylized art and has not been performance-certified on an RTX 2060.

## Vehicle cadence

The first interview car drives toward the stop when the scene opens. Later cars enter at staggered times from the long approach road, brake behind the queue and maintain nine-metre centre spacing. The arrival HUD reports progress. E is enabled only after the current car has stopped and the officer aims within interview range.

After the server returns a decision and the player closes the case file, the barrier rises and that car drives away. Admitted cars follow the outbound road as it bends left. Denied cars turn onto the west return road; they do not reverse through the waiting queue. Only when the departing car clears the checkpoint does the next case advance and the following car pull up. New cases therefore do not teleport a replacement car into the booth. Vehicles yield to a person directly in their path, while an officer beside the window does not block arrivals. Moving vehicles raise small trailing dust puffs.

Lane 2 has a separate ambient queue and automatic barrier cycle. Its cars stop briefly, then continue out along the road; this is visual traffic, not an independent AI interview. Starting another shift resets the finite vehicle pool and timing. The original ten-case content order and authoritative provider decision results are unchanged.

## Scope boundary

The expanded scene and the solo traffic loop are implemented. Vehicle motion currently runs in the graphical prototype; the separate headless host rules remain unchanged. Full host replication, additional vehicle types and interaction points remain in Parts 6/15. The office ready-up door/shift interaction remains Part 5. Work tables and crates are props; physical searches and detention behavior remain their later Parts. The current launch is the working solo interview prototype in the expanded map.

## Validation

Validated on Linux with Rust 1.94.0 and the locked dependency graph:

- Full workspace tests: **38 passed**, one existing explicit microphone/model fixture test ignored.
- Clippy for the entire workspace/all targets with warnings denied, plus Rust formatting: passed.
- A headless connectivity test builds the actual scene and flood-fills collision-free walking routes from spawn to the office, storage, holding yard, secondary bay, driver and both ends of the bridge. It also rejects river shortcuts and the reserved future lanes.
- `./play.sh --smoke-test` passed with the resident GPU speech models loaded and a clean automatic exit.
- The traffic system test runs all ten cases with alternating admit/deny decisions, checks queue gaps and the west return route, then restarts the shift. Other tests cover braking, the bridge/bent approach route and yielding to a person in the driving lane.
- Rendered views reviewed for the overview, normal driver position, rocky riverbank and road/cloud landscape.
- Graphical interview smoke exercised an offline AI exchange and secondary inspection with the expanded scene and exited successfully. The smoke now fails if arrival, dialogue or secondary inspection does not finish. No paid model call or microphone recording was needed for these checks.

Reproduce captures after `cargo build --locked -p border-game`:

```sh
./target/debug/border-game --smoke-test
./target/debug/border-game --smoke-test --scene-view=overview
./target/debug/border-game --smoke-test --scene-view=river
./target/debug/border-game --smoke-test --scene-view=office
./target/debug/border-game --smoke-test --scene-view=map --show-map
./target/debug/border-game --smoke-test --scene-view=landscape
./target/debug/border-game --smoke-test --scene-view=rocks
```

Other review viewpoints are `yard`, `desert`, `north-road`, `cinematic`, `office-exterior` and `cars`. Smoke captures freeze camera movement to make visual review independent of mouse motion. Captures are written under ignored `output/`; the overview above is the checked-in review image. Smoke mode disables microphone recording and speech playback and exits automatically. These captures validate rendering and layout, not a full human multiplayer playtest.
