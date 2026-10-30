# Part 4 — San Ignacio environment

San Ignacio now sits on the Texas bank of a desert river. The road crosses a supported bridge into the checkpoint through an opening in the border fence. The long border fence stays on the Texas bank; it does not run down the opposite bank or through the water.

![San Ignacio river checkpoint](san-ignacio-overview.webp)

## Explore

Launch the updated graphical prototype with:

```sh
./play.sh
```

Click **Begin patrol**, then use WASD and the mouse. Hold **left Shift** to jog. Press **M** for a site map with your position; press M again to close it. E still starts the existing interview when you aim at the active teal sedan within three metres. Tab switches between mouse look and typing, and Escape releases the mouse.

The administration office is west of the inspection lanes, storage is immediately farther west, the holding yard is behind the office, and secondary inspection is east of the lanes. The river bridge is south, behind the vehicle queue. Walk through the fence opening and across the bridge to the closed far-end gate. Closed gates and the yard fence mark the limits; the surrounding desert and reserved future lanes are scenery.

## Scene inventory

| Space | Implemented detail |
| --- | --- |
| Two-lane checkpoint | Separate lane markings/arrows, stop lines, canopy beams, hanging fixtures, cameras, protective bollards, striped barriers and static queued vehicles |
| Active interview stop | Existing detailed sedan, visible driver, animated barrier, AI/text/voice conversation and evidence UI retained |
| Administration office | Hollow enterable room, doorway, window trim, roof, desks, terminals, chairs, documents, filing cabinets, bulletin board, coffee station, ceiling fixtures and warm interior lighting |
| Equipment storage | Open-front shelter, shelves, stacked crates, adjacent utility cabinet, service pipes, condensers and barrels |
| Holding yard | Fenced enclosure, open east-side entrance, shaded benches, nearby water tank and location sign |
| Secondary inspection | Broad marked concrete apron, open-sided roof, structural posts and work tables |
| River crossing | Separate riverbanks, lowered river channel, animated surface streaks, road deck, piers, guardrails, posts and diagonal supports |
| Border | Tall Texas-bank fence with a bridge opening, detailed close slats and inexpensive distant sections fading into haze |
| Desert | Branching ribbed cacti with flower tips, sage-colored bushes, scattered rocks, bank stones, low dunes, angular distant mountains and slowly drifting dust |
| Readability | Warm cohesive palette, directional shadows, distance fog, occluded world location labels, humorous checkpoint notices and a site map |

The playable yard is approximately 51 × 67 metres, with an additional walkable bridge corridor. Movement stays at ground/deck level, with axis-aligned collision against structural walls and props. Enterable facilities and the driver approach are reachable from the spawn. Terrain outside the yard/bridge is intentionally non-navigable. The eastern reserved apron provides space for later expansion without adding operational lanes now.

Environment geometry uses four shared primitive meshes and a shared palette; the existing sedan retains its own geometry. No art downloads, external textures or new runtime dependencies are required. This is stylized placeholder art with intentional detail, not a claim of final production assets or measured RTX 2060 performance.

## Scope boundary

Part 4 completes the scene and navigable spaces. Queue cars and the second barrier are set dressing until Part 6 lifecycle integration. The office doorway is open; the ready-up door interaction and authoritative shift start remain Part 5. Work tables/crates are props; physical cargo searches and detention behavior remain their later Parts. Multiplayer rendering/replication is still tracked in Part 15. The current launch remains the working solo interview prototype in the expanded map.

## Validation

Validated on Linux with Rust 1.94.0 and the locked dependency graph:

- Full workspace tests: **34 passed**, one existing explicit microphone/model fixture test ignored.
- Clippy for the entire workspace/all targets with warnings denied, plus Rust formatting: passed.
- A headless connectivity test builds the actual scene and flood-fills collision-free walking routes from spawn to the office, storage, holding yard, secondary bay, driver and both ends of the bridge. It also rejects river shortcuts and the reserved future lanes.
- `./play.sh --smoke-test` passed with the resident GPU speech models loaded and a clean automatic exit.
- Graphical smoke captures reviewed for the normal driver view, office interior, river/bridge, site overview and site map.
- Graphical interview smoke exercised an offline AI exchange and secondary inspection with the expanded scene and exited successfully. No paid model call or microphone recording was needed for these checks.

Reproduce captures after `cargo build --locked -p border-game`:

```sh
./target/debug/border-game --smoke-test
./target/debug/border-game --smoke-test --scene-view=overview
./target/debug/border-game --smoke-test --scene-view=river
./target/debug/border-game --smoke-test --scene-view=office
./target/debug/border-game --smoke-test --scene-view=map --show-map
```

Other review viewpoints are `yard` and `desert`. Captures are written under ignored `output/`; the overview above is the checked-in review image. Smoke mode disables microphone recording and speech playback and exits automatically. These captures validate rendering and layout, not a full human multiplayer playtest.
