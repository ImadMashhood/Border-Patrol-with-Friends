# Reused Rust code and references

## Rapier — physics engine and articulated body example

- Repository: <https://github.com/dimforge/rapier>
- Integrated crate: **rapier3d 0.30.1**, Apache-2.0, pinned in Cargo.lock.
- Joint-layout reference: [examples3d/stress_tests/ragdolls3.rs](https://github.com/dimforge/rapier/blob/475caa3ff4646fd9a796b67d14116256f4962bc7/examples3d/stress_tests/ragdolls3.rs), commit `475caa3ff4646fd9a796b67d14116256f4962bc7`.
- Retained license: [Apache-2.0](licenses/rapier-APACHE-2.0.txt).

`crates/simulation/src/physics.rs` adapts the example's articulated rigid-body construction, spherical neck/shoulder/hip joints, limited elbow/knee hinges and disabled adjacent-body contacts. Changes include a separate pelvis, eleven bodies, seated starting poses, cabin/seat/door/glass collision, a releasable seat constraint, projectile impulses, a kinematic extraction anchor, fixed 60 Hz headless stepping, and networked body/shard poses. It uses the pinned Rapier API directly on the authoritative host, without changing the Bevy renderer or requiring the newer testbed API. The example's rendering/testbed code is not included.

## Rusty Shooter — weapon presentation

- Repository: <https://github.com/mrDIMAS/rusty-shooter>
- Source: [src/weapon.rs](https://github.com/mrDIMAS/rusty-shooter/blob/da7ba8c42fd86568e0445a4e23a7092159d63f9b/src/weapon.rs), commit `da7ba8c42fd86568e0445a4e23a7092159d63f9b`.
- Copyright (c) 2019 Dmitry Stepanov.
- Retained license: [MIT](licenses/rusty-shooter-MIT.txt).

The weapon offset/kick and follow/recovery pattern is adapted in `crates/game/src/weapon.rs`. Recovery is made frame-rate independent, with Bevy transforms, right-button sight alignment and field-of-view blending, the existing host-owned ammunition/cooldown checks, and one travelling projectile visual. Rusty Shooter's Fyrox scene graph, networking, sound/model assets and complete game are not copied.

## Original project work

Vehicle geometry, the six body variants and fittings, generated audio, movement prediction/shared collision, fracture meshes, client presentation and game-specific physics integration are authored for this project. No third-party car or character art is imported. The notices above cover the reused code/references and do not assign a new blanket license to the user's repository.
