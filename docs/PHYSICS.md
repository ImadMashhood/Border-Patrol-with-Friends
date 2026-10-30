# Movement, aiming, articulated bodies and vehicle variety

Run `./play.sh`; a second officer joins with `./play.sh --join`. The office ready hold is now **two seconds**. Two lanes and the existing ten travelers remain.

## Movement and weapon fixes

Player translation previously followed successive host snapshots, which made movement arrive in visible steps. Normal play now predicts walking/jogging every rendered frame, using the **same collision function** as the host (`crates/net/src/movement.rs`). Small network differences are reconciled gradually; shift respawns and large corrections reset the predicted pose. When standing still the camera converges to the authoritative position for accurate interaction/aiming. Prediction pauses if snapshots become stale. Positions, readiness, damage and rewards remain host-owned.

The graphical worker now sends input/receives snapshots at 20 Hz, matching the host tick. Vehicle roots and remote officers are interpolated, and vehicle glazing, doors and seated occupants use the displayed car pose together so a driver does not jump ahead of an interpolated car.

Hold **right mouse** with the pistol drawn to aim down the sights. Sight alignment, field of view (70° hip / 50° aimed) and gun placement blend smoothly; aiming reduces mouse sensitivity. The sight posts have been resized to keep the target readable. Right click can also capture mouse look. B draws/holsters, left click fires, and R reloads.

The doubled bullet appearance came from rendering both the travelling projectile and another full-length trail after impact. The impact trail is removed. Each shot now has one moving tracer, with impact-only marks/debris and the existing host-simulated travel/collision.

## Physical glass and transparency

Glass fragments are actual **Rapier rigid bodies on the host**. Thin colliders receive gravity, angular velocity, friction, restitution and continuous collision detection. They collide with the road, cabin, seats, vehicle bodywork, doors and intact glass. Full breakage releases fragments across the pane; a windshield hole releases a smaller burst. There are at most 256 live fragments; they expire after eight seconds. Fragment-to-fragment contacts are filtered to control cost.

Clients display replicated fragment transforms with smoothing. The old upward-only glass particle burst is disabled. Metal/world impact sparks remain short visual effects.

The camera uses Bevy's **order-independent transparency** with 16 layers and MSAA disabled. This handles overlapping transparent panes and shards without treating the first pane as the only visible glass layer. Both faces of the glazing are rendered. This is alpha-blended glass rather than a refraction renderer.

## Cabin ragdolls and extraction

Dead NPCs switch to an **eleven-body Rapier ragdoll**: pelvis, chest, head, upper/lower arms and thighs/shins. Limited spherical and hinge joints keep the body connected. A seated corpse begins in a seated pose, collides with the cabin and seat, and retains a releasable pelvis/seat constraint so it can slump without immediately falling through the car. Death uses the shot impulse; further shots can impart impulses to individual ragdoll bodies.

Standing NPCs collapse under the same solver. Clients render the host's bone poses instead of the earlier fixed death rotation. The solver runs at a fixed 60 Hz; the headless host publishes poses in normal snapshots.

Near the **driver's side**, press **Y** or use **Pull body out** in the duty roster to remove a dead seated traveler. The host checks range and lane ownership/assistance, opens the door, lowers the driver glass, releases the seat constraint and pulls the chest toward an outside point through a temporary physical anchor. Limbs follow under their existing joints and collision. Once clear, the anchor releases and the body settles on the ground. It does not teleport to a preselected prone pose. X still orders a living traveler out/back; T requests recovery of a disabled vehicle.

The character art remains a procedural prototype. The improved ragdoll uses jointed capsule/sphere body geometry with matching clothing colors; a full animated/skinned character asset is separate art work. Self-contact and joint limits are simplified for this game's small number of bodies.

## Six detailed vehicle bodies

| Kind | Distinguishing body/fittings |
| --- | --- |
| Sedan | Existing chamfered body, sloping glazing and short trunk |
| Pickup | Raised cab, open ribbed bed, running boards and larger wheels |
| SUV | Longer raised passenger roof, extended rear glazing and roof rails |
| Van | Longer/taller body, large rear side glazing, roof ribs and rear door detail |
| Hatchback | Shorter wheelbase/body, rear lip spoiler and rear wiper |
| Utility pickup | Pickup bed with metal toolbox and elevated cargo rack |

All types have close-up grille slats, plate characters, lamps, bumpers, hood/door seams, wipers, sills, fuel covers, dashboard/radio fittings, vents and steering-wheel detail. Paint and body variants remain independent of scenario guilt. Catalog dimensions drive queue spacing, access points and collision. Both lanes receive mixed types. Front doors/windows and existing hood/cargo inspections remain usable; this is a reusable procedural vehicle set, not a scan-quality asset pack.

## Reuse and validation

The implementation integrates Rapier and adapts its articulated-body example, plus Rusty Shooter's weapon offset/recovery pattern. [Pinned sources, modifications and retained license notices](THIRD_PARTY.md) are included.

```sh
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
python3 scripts/test_host.py
python3 scripts/test_graphical_shift.py --lifecycle
python3 scripts/test_graphical_shift.py --physics
```

The locked workspace suite passes **60 Rust tests**, with one existing microphone/model fixture ignored. Clippy, formatting, the HTTP regression and both two-client graphical modes pass. Tests cover smooth bounded prediction/reconciliation, the two-second host hold, the existing ten-minute clock and interviews, all vehicle types/queue paths, finite falling/colliding fragments and expiry, a connected cabin ragdoll, seat restraint, physical extraction and projectile impulses. The physics graphical check runs two actual clients, captures right-button aiming, glass breakage, cabin/outside ragdolls and extraction, and verifies all six vehicle kinds. Automated checks use offline AI and disable recording/audio. A human feel/performance review and RTX 2060 hardware certification remain outside these automated checks.

![Right-button sight alignment](physics-aim.webp)

![Body extracted from the cabin using the physical grab constraint](physics-extraction.webp)

![Mixed six-body vehicle catalog in the live two-lane queue](vehicle-fleet.webp)
