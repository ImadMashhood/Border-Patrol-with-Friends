//! Renderer-free authority. Hidden scenario truth stays server-side; observations are shared.
use anyhow::{Result, bail, ensure};
use border_caller::{Turn, validate_turn_reply};
use border_net::*;
use border_scenarios::{Scenario, seeded_order};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
mod combat;
#[cfg(test)]
mod combat_tests;
mod interactions;
#[cfg(test)]
mod interactions_tests;
#[cfg(test)]
mod lifecycle_tests;
mod navigation;
mod physics;
mod shift;

struct Case {
    epoch: u64,
    scenario: Scenario,
    history: Vec<Turn>,
    secondary: bool,
    waypoint: usize,
    departure_order: u64,
    conversation_epoch: u64,
    grab_started_ms: u64,
    misconduct_turn: Option<(usize, String)>,
    flee_requested: bool,
}
struct Peer {
    input: Option<InputFrame>,
    input_at: u64,
    input_frame: u64,
    hold_started: Option<u64>,
    last_seen: u64,
    next_sequence: u64,
    receipts: HashMap<String, (String, CommandReceipt)>,
}
/// Secret server-to-server context. Never return this from a peer route.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ActorContext {
    pub conversation_epoch: u64,
    pub scenario: Scenario,
    pub history: Vec<Turn>,
    pub expected_turn: u64,
    pub secondary: bool,
    pub case_epoch: u64,
}
pub struct Simulation {
    world: WorldSnapshot,
    pack: Vec<Scenario>,
    order: Vec<usize>,
    cursor: usize,
    cases: HashMap<u64, Case>,
    peers: HashMap<u64, Peer>,
    events: VecDeque<DomainEvent>,
    collision: Vec<border_net::map::Aabb>,
    next_notice_ms: u64,
    epoch: u64,
    next_projectile: u64,
    next_impact: u64,
    walkers: HashMap<navigation::Walker, navigation::Route>,
    physics: physics::PhysicalWorld,
}
impl Simulation {
    pub fn new(session_id: String, seed: u64, pack: Vec<Scenario>) -> Result<Self> {
        ensure!(!pack.is_empty(), "empty scenario pack");
        for s in &pack {
            s.validate()?;
        }
        Ok(Self {
            order: seeded_order(pack.len(), seed),
            pack,
            cursor: 0,
            cases: HashMap::new(),
            peers: HashMap::new(),
            events: VecDeque::new(),
            collision: border_net::map::collision().solids,
            next_notice_ms: 45_000 + seed % 30_000,
            epoch: 0,
            next_projectile: 1,
            next_impact: 1,
            walkers: HashMap::new(),
            physics: physics::PhysicalWorld::new(),
            world: WorldSnapshot {
                protocol_version: PROTOCOL_VERSION,
                session_id,
                revision: 0,
                latest_event_sequence: 0,
                seed,
                phase: ShiftPhase::Lobby,
                remaining_ms: 600_000,
                peers: vec![],
                lanes: (1..=LANE_COUNT)
                    .map(|lane_id| LaneView {
                        lane_id,
                        owner: None,
                        barrier_open: false,
                        entry_barrier_open: false,
                        queue: vec![],
                    })
                    .collect(),
                vehicles: vec![],
                money: 0,
                ledger: vec![],
                purchases: vec![],
                upgrades: vec![],
                incidents: vec![],
                host_peer_id: None,
                server_time_ms: 0,
                shift_started_ms: None,
                door_open: false,
                queues_active: false,
                events_active: false,
                lane_swap: None,
                notices: vec![],
                projectiles: vec![],
                impacts: vec![],
                shards: vec![],
                porters: (1..=2)
                    .map(|lane_id| PorterView {
                        lane_id,
                        position: porter_chair(lane_id),
                        yaw: 0.,
                        phase: PorterPhase::Seated,
                        vehicle_id: None,
                        progress: 0.,
                    })
                    .collect(),
            },
        })
    }
    pub fn snapshot(&self) -> WorldSnapshot {
        self.world.clone()
    }
    pub fn check_session(&self, version: u16, session: &str) -> Result<()> {
        ensure!(version == PROTOCOL_VERSION, "unsupported protocol version");
        ensure!(session == self.world.session_id, "wrong session");
        Ok(())
    }
    fn emit(&mut self, kind: &str, data: serde_json::Value) {
        self.world.revision += 1;
        self.world.latest_event_sequence += 1;
        let sequence = self.world.latest_event_sequence;
        self.events.push_back(DomainEvent {
            event_id: format!("{}:{sequence}", self.world.session_id),
            sequence,
            revision: self.world.revision,
            kind: kind.into(),
            data,
        });
        if self.events.len() > 512 {
            self.events.pop_front();
        }
    }
    pub fn join(&mut self, name: String, now: u64) -> Result<u64> {
        ensure!(self.world.peers.len() < 32, "session join limit reached");
        ensure!(
            self.world.phase == ShiftPhase::Lobby,
            "shift already started"
        );
        ensure!(
            !name.trim().is_empty() && name.len() <= 48,
            "name must be 1–48 bytes"
        );
        ensure!(
            self.world.peers.iter().filter(|p| p.connected).count() < MAX_PLAYERS,
            "session full"
        );
        let id = self.world.peers.len() as u64 + 1;
        self.world.peers.push(PeerView {
            peer_id: id,
            display_name: name,
            ready: false,
            connected: true,
            health: 100,
            position: border_net::map::SPAWNS[(id as usize - 1) % 2],
            yaw: std::f32::consts::PI,
            pitch: 0.,
            equipment: vec![
                "pistol".into(),
                "flashlight".into(),
                "radio".into(),
                "document_clipboard".into(),
            ],
            hold_ms: 0,
            assisting: None,
            next_command_sequence: 1,
            weapon: border_net::combat::WeaponState::default(),
        });
        self.peers.insert(
            id,
            Peer {
                last_seen: now,
                input: None,
                input_at: now,
                input_frame: 0,
                hold_started: None,
                next_sequence: 1,
                receipts: HashMap::new(),
            },
        );
        if self.world.host_peer_id.is_none() {
            self.world.host_peer_id = Some(id);
        }
        self.emit("peer_joined", json!({"peer_id":id}));
        Ok(id)
    }
    pub fn heartbeat(&mut self, id: u64, now: u64) -> Result<()> {
        ensure!(
            self.world
                .peers
                .iter()
                .any(|p| p.peer_id == id && p.connected),
            "peer disconnected"
        );
        self.peers.get_mut(&id).unwrap().last_seen = now;
        Ok(())
    }
    pub fn leave(&mut self, id: u64) {
        if let Some(p) = self
            .world
            .peers
            .iter_mut()
            .find(|p| p.peer_id == id && p.connected)
        {
            p.connected = false;
            p.ready = false;
            for l in &mut self.world.lanes {
                if l.owner == Some(id) {
                    l.owner = None;
                }
            }
            self.world.lane_swap = None;
            if self.world.host_peer_id == Some(id) {
                self.world.host_peer_id = self
                    .world
                    .peers
                    .iter()
                    .find(|p| p.connected)
                    .map(|p| p.peer_id);
            }
            self.emit("peer_left", json!({"peer_id":id}));
        }
    }
    pub fn expire(&mut self, now: u64) {
        let ids: Vec<_> = self
            .peers
            .iter()
            .filter(|(_, p)| now.saturating_sub(p.last_seen) >= PEER_TIMEOUT_MS)
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            self.leave(id);
        }
    }
    pub fn replay(&self, after: u64) -> Result<Replay> {
        ensure!(
            after <= self.world.latest_event_sequence,
            "future event cursor"
        );
        if let Some(first) = self.events.front() {
            ensure!(
                after.saturating_add(1) >= first.sequence,
                "resync required: event cursor expired"
            );
        }
        Ok(Replay {
            session_id: self.world.session_id.clone(),
            latest_sequence: self.world.latest_event_sequence,
            events: self
                .events
                .iter()
                .filter(|e| e.sequence > after)
                .cloned()
                .collect(),
        })
    }
    fn owns_vehicle(&self, peer: u64, id: u64) -> Result<usize> {
        ensure!(
            self.world.phase == ShiftPhase::Active || self.world.phase == ShiftPhase::Grace,
            "shift inactive"
        );
        let i = self
            .world
            .vehicles
            .iter()
            .position(|v| v.vehicle_id == id)
            .ok_or_else(|| anyhow::anyhow!("unknown vehicle"))?;
        let v = &self.world.vehicles[i];
        ensure!(
            v.stage == VehicleStage::Stopped
                && v.physics.npc.health > 0
                && !matches!(
                    v.physics.npc.mode,
                    border_net::combat::NpcMode::Exiting | border_net::combat::NpcMode::Entering
                ),
            "traveler unavailable"
        );
        ensure!(
            self.world
                .lanes
                .iter()
                .any(|l| l.lane_id == v.lane_id && l.owner == Some(peer))
                || self
                    .world
                    .peers
                    .iter()
                    .any(|p| p.peer_id == peer && p.assisting == Some(v.lane_id)),
            "claim or assist this lane first"
        );
        Ok(i)
    }
    pub fn command(&mut self, peer: u64, e: CommandEnvelope, now: u64) -> Result<CommandReceipt> {
        self.check_session(e.protocol_version, &e.session_id)?;
        ensure!(
            !e.request_id.is_empty() && e.request_id.len() <= 80,
            "invalid request ID"
        );
        let fingerprint = serde_json::to_string(&e)?;
        let p = self
            .peers
            .get(&peer)
            .ok_or_else(|| anyhow::anyhow!("unknown peer"))?;
        if let Some((old, r)) = p.receipts.get(&e.request_id) {
            ensure!(
                old == &fingerprint,
                "request ID reused with different content"
            );
            return Ok(r.clone());
        }
        ensure!(p.receipts.len() < 4096, "session command limit reached");
        ensure!(p.next_sequence == e.sequence, "unexpected command sequence");
        ensure!(
            e.expected_revision.is_none_or(|r| r == self.world.revision),
            "stale revision"
        );
        self.heartbeat(peer, now)?;
        ensure!(
            self.world
                .peers
                .iter()
                .any(|p| p.peer_id == peer && p.health > 0)
                || matches!(&e.command, ClientCommand::ResetShift),
            "peer incapacitated"
        );
        match &e.command {
            ClientCommand::ToggleCompartment {
                vehicle_id,
                location,
            } => {
                self.toggle_compartment(peer, *vehicle_id, *location)?;
            }
            ClientCommand::TogglePanel { vehicle_id, point } => {
                let i = self.inspection_access(peer, *vehicle_id)?;
                self.require_point(peer, i, *point)?;
                if self.world.vehicles[i].open_panels.contains(point) {
                    self.world.vehicles[i].open_panels.retain(|p| p != point);
                    if *point == vehicle::VehiclePoint::Cargo {
                        self.world.vehicles[i]
                            .open_compartments
                            .retain(|l| *l != border_scenarios::EvidenceLocation::Container);
                    }
                } else {
                    self.inspect_point(peer, *vehicle_id, *point)?;
                }
            }
            ClientCommand::EndConversation { vehicle_id } => {
                self.end_conversation(peer, *vehicle_id)?;
            }
            ClientCommand::GrabBody { vehicle_id, bone } => {
                self.grab_body(peer, *vehicle_id, *bone)?;
            }
            ClientCommand::ReleaseBody => {
                self.release_bodies(peer);
            }
            ClientCommand::RequestPorter { lane_id } => {
                self.request_porter(peer, *lane_id)?;
            }
            ClientCommand::SetWeaponDrawn { drawn } => {
                ensure!(
                    !drawn || self.world.phase != ShiftPhase::Lobby,
                    "weapons stay holstered before ready-up"
                );
                self.world
                    .peers
                    .iter_mut()
                    .find(|p| p.peer_id == peer)
                    .unwrap()
                    .weapon
                    .drawn = *drawn;
            }
            ClientCommand::Fire { direction } => self.fire(peer, *direction)?,
            ClientCommand::Reload => self.reload(peer)?,
            ClientCommand::ToggleDoor { vehicle_id, side } => {
                self.toggle_door(peer, *vehicle_id, *side)?
            }
            ClientCommand::ToggleWindow { vehicle_id, pane } => {
                self.toggle_window(peer, *vehicle_id, *pane)?
            }
            ClientCommand::OrderExit { vehicle_id } => self.order_exit(peer, *vehicle_id)?,
            ClientCommand::PullBody { vehicle_id } => self.pull_body(peer, *vehicle_id)?,
            ClientCommand::OrderEnter { vehicle_id } => self.order_enter(peer, *vehicle_id)?,
            ClientCommand::TowVehicle { vehicle_id } => self.tow_vehicle(peer, *vehicle_id)?,
            ClientCommand::SetReady { ready } => {
                ensure!(!ready, "hold E at the office door to register ready");
                self.world
                    .peers
                    .iter_mut()
                    .find(|p| p.peer_id == peer)
                    .unwrap()
                    .ready = false;
                self.peers.get_mut(&peer).unwrap().hold_started = None;
            }
            ClientCommand::AssistLane { lane_id } => {
                ensure!(
                    lane_id.is_none_or(|id| self
                        .world
                        .lanes
                        .iter()
                        .any(|l| l.lane_id == id && l.owner.is_some())),
                    "lane is not staffed"
                );
                self.world
                    .peers
                    .iter_mut()
                    .find(|p| p.peer_id == peer)
                    .unwrap()
                    .assisting = *lane_id;
            }
            ClientCommand::RequestLaneSwap => {
                let other = self
                    .world
                    .peers
                    .iter()
                    .find(|p| p.connected && p.peer_id != peer)
                    .ok_or_else(|| anyhow::anyhow!("no other officer"))?;
                ensure!(
                    self.world
                        .lanes
                        .iter()
                        .filter(|l| l.owner.is_some())
                        .count()
                        == 2,
                    "both lanes must be assigned"
                );
                self.world.lane_swap = Some([peer, other.peer_id]);
            }
            ClientCommand::AcceptLaneSwap => {
                let offer = self
                    .world
                    .lane_swap
                    .ok_or_else(|| anyhow::anyhow!("no swap offer"))?;
                ensure!(offer[1] == peer, "swap belongs to another officer");
                for lane in &mut self.world.lanes {
                    if lane.owner == Some(offer[0]) {
                        lane.owner = Some(offer[1]);
                    } else if lane.owner == Some(offer[1]) {
                        lane.owner = Some(offer[0]);
                    }
                }
                self.world.lane_swap = None;
            }
            ClientCommand::ReleaseVehicle { vehicle_id } => self.release_case(peer, *vehicle_id)?,
            ClientCommand::ResetShift => self.reset_shift(peer)?,
            ClientCommand::ClaimLane { lane_id } => {
                let l = self
                    .world
                    .lanes
                    .iter_mut()
                    .find(|l| l.lane_id == *lane_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown lane"))?;
                ensure!(l.owner.is_none() || l.owner == Some(peer), "lane occupied");
                l.owner = Some(peer);
                for other in &mut self.world.lanes {
                    if other.lane_id != *lane_id && other.owner == Some(peer) {
                        other.owner = None;
                    }
                }
            }
            ClientCommand::ReleaseLane { lane_id } => {
                let l = self
                    .world
                    .lanes
                    .iter_mut()
                    .find(|l| l.lane_id == *lane_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown lane"))?;
                ensure!(l.owner == Some(peer), "lane not owned");
                l.owner = None;
            }
            ClientCommand::InspectVehiclePoint { vehicle_id, point } => {
                self.inspect_point(peer, *vehicle_id, *point)?;
            }
            ClientCommand::RequestSecondary { vehicle_id } => {
                let i = self.authorize_case(peer, *vehicle_id)?;
                self.world.vehicles[i].secondary_requested = true;
                self.emit("secondary_requested", json!({"vehicle_id":vehicle_id}));
            }
            ClientCommand::SearchCompartment {
                vehicle_id,
                location,
            } => {
                self.search_compartment(peer, *vehicle_id, *location)?;
            }
            ClientCommand::InspectEvidence {
                vehicle_id,
                evidence_id,
            } => {
                let location = self
                    .cases
                    .get(vehicle_id)
                    .and_then(|c| c.scenario.evidence.iter().find(|e| e.id == *evidence_id))
                    .ok_or_else(|| anyhow::anyhow!("unknown evidence"))?
                    .location;
                self.search_compartment(peer, *vehicle_id, location)?;
            }
            ClientCommand::Decide {
                vehicle_id,
                disposition,
            } => {
                self.decide_case(peer, *vehicle_id, *disposition)?;
            }
            ClientCommand::RequestPurchase { upgrade_id } => {
                ensure!(upgrade_id == "document_scanner", "unknown upgrade");
                ensure!(
                    !self.world.upgrades.contains(upgrade_id),
                    "already purchased"
                );
                ensure!(
                    !self
                        .world
                        .purchases
                        .iter()
                        .any(|p| p.upgrade_id == *upgrade_id && !p.approved),
                    "purchase pending"
                );
                let proposal_id = self.world.purchases.len() as u64 + 1;
                self.world.purchases.push(PurchaseView {
                    proposal_id,
                    peer_id: peer,
                    upgrade_id: upgrade_id.clone(),
                    approved: false,
                });
            }
        }
        self.emit(
            "command_applied",
            json!({"peer_id":peer,"request_id":e.request_id,"command":e.command}),
        );
        let receipt = CommandReceipt {
            request_id: e.request_id.clone(),
            sequence: e.sequence,
            revision: self.world.revision,
            event_ids: vec![format!(
                "{}:{}",
                self.world.session_id, self.world.latest_event_sequence
            )],
        };
        let p = self.peers.get_mut(&peer).unwrap();
        p.next_sequence += 1;
        self.world
            .peers
            .iter_mut()
            .find(|p| p.peer_id == peer)
            .unwrap()
            .next_command_sequence = p.next_sequence;
        // Retain every receipt for the session; the command limit bounds memory.
        p.receipts
            .insert(e.request_id, (fingerprint, receipt.clone()));
        Ok(receipt)
    }
    pub fn operator(&mut self, command: HostCommand) -> Result<()> {
        match &command {
            HostCommand::StartShift => self.start_shift()?,
            HostCommand::SpawnNextVehicle { lane_id, kind } => {
                ensure!(self.world.phase == ShiftPhase::Active, "shift inactive");
                ensure!(
                    (kind == "car" || vehicle::KINDS.contains(&kind.as_str())),
                    "unknown vehicle kind"
                );
                ensure!(self.cursor < self.order.len(), "scenario pack exhausted");
                let l = self
                    .world
                    .lanes
                    .iter_mut()
                    .find(|l| l.lane_id == *lane_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown lane"))?;
                let vehicle_id = self.cursor as u64 + 1;
                let mut scenario = self.pack[self.order[self.cursor]]
                    .instantiate(self.world.seed ^ self.epoch.rotate_left(32) ^ vehicle_id);
                if kind == "utility_truck" {
                    scenario.evidence.push(border_scenarios::Evidence {
                        id: "trailer_manifest".into(), location: border_scenarios::EvidenceLocation::Trailer,
                        reveal_condition: border_scenarios::RevealCondition::Secondary, reveal_rule: "physical_search".into(),
                        description: format!("Trailer inspected: secured container and inventory card. {} No additional consignment listed.", scenario.truth.cargo),
                    });
                }
                self.cursor += 1;
                l.queue.push(vehicle_id);
                self.world.vehicles.push(VehicleView {
                    vehicle_id,
                    lane_id: *lane_id,
                    kind: vehicle::spec(kind).kind.into(),
                    stage: VehicleStage::Queued,
                    position_mm: 0,
                    traveler_name: scenario.character.name.clone(),
                    opening_line: scenario.opening_line.clone(),
                    conversation_owner: None,
                    observed_evidence: vec![],
                    position: [0.; 3],
                    yaw: 0.,
                    speed: 0.,
                    distance_to_stop: 18. + (l.queue.len() - 1) as f32 * 30.,
                    history: vec![],
                    outcome: None,
                    secondary_inspected: false,
                    secondary_requested: false,
                    searched_compartments: vec![],
                    inspected_points: vec![],
                    open_panels: vec![],
                    open_compartments: vec![],
                    physics: border_net::combat::VehiclePhysics::default(),
                });
                self.cases.insert(
                    vehicle_id,
                    Case {
                        scenario,
                        history: vec![],
                        secondary: false,
                        epoch: self.epoch,
                        waypoint: 0,
                        departure_order: 0,
                        conversation_epoch: 0,
                        grab_started_ms: 0,
                        misconduct_turn: None,
                        flee_requested: false,
                    },
                );
            }
            HostCommand::AdvanceWorld { delta_ms } => {
                ensure!(*delta_ms <= 1000, "tick exceeds 1000 ms");
                let now = self.world.server_time_ms + delta_ms;
                self.tick(now);
            }
            HostCommand::ApplyDamage { peer_id, amount } => {
                let p = self
                    .world
                    .peers
                    .iter_mut()
                    .find(|p| p.peer_id == *peer_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown peer"))?;
                p.health = p.health.saturating_sub(*amount);
            }
            HostCommand::RestoreHealth => {
                for p in &mut self.world.peers {
                    p.health = 100;
                }
            }
            HostCommand::CreateIncident { kind } => {
                let incident_id = self.world.incidents.len() as u64 + 1;
                self.world.incidents.push(IncidentView {
                    incident_id,
                    kind: kind.clone(),
                    resolved: false,
                });
            }
            HostCommand::ResolveIncident { incident_id } => {
                let i = self
                    .world
                    .incidents
                    .iter_mut()
                    .find(|i| i.incident_id == *incident_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown incident"))?;
                ensure!(!i.resolved, "incident already resolved");
                i.resolved = true;
            }
            HostCommand::ApprovePurchase { proposal_id } => {
                let p = self
                    .world
                    .purchases
                    .iter_mut()
                    .find(|p| p.proposal_id == *proposal_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown proposal"))?;
                ensure!(!p.approved, "already approved");
                ensure!(self.world.money >= 300, "insufficient shared funds");
                self.world.money -= 300;
                p.approved = true;
                self.world.upgrades.push(p.upgrade_id.clone());
            }
        }
        self.emit("host_command_applied", json!({"command":command}));
        Ok(())
    }
    pub fn actor_context(&self, vehicle: u64) -> Result<ActorContext> {
        let v = self
            .world
            .vehicles
            .iter()
            .find(|v| v.vehicle_id == vehicle)
            .ok_or_else(|| anyhow::anyhow!("unknown vehicle"))?;
        ensure!(
            v.stage == VehicleStage::Stopped && v.physics.npc.health > 0,
            "traveler unavailable"
        );
        let c = &self.cases[&vehicle];
        ensure!(c.history.len() < 128, "conversation limit reached");
        let mut scenario = c.scenario.with_observations(
            &v.observed_evidence
                .iter()
                .map(|e| e.id.clone())
                .collect::<Vec<_>>(),
        );
        scenario.encounter = self.physical_context(v);
        Ok(ActorContext {
            conversation_epoch: c.conversation_epoch,
            scenario,
            history: c.history.clone(),
            expected_turn: c.history.len() as u64,
            secondary: c.secondary,
            case_epoch: c.epoch,
        })
    }
    pub fn actor(&mut self, delivery: ActorDelivery) -> Result<()> {
        self.check_session(delivery.protocol_version, &delivery.session_id)?;
        let v = self
            .world
            .vehicles
            .iter()
            .find(|v| v.vehicle_id == delivery.vehicle_id)
            .ok_or_else(|| anyhow::anyhow!("unknown vehicle"))?;
        ensure!(
            v.stage == VehicleStage::Stopped && v.physics.npc.health > 0,
            "traveler unavailable"
        );
        ensure!(
            !delivery.officer_utterance.trim().is_empty()
                && delivery.officer_utterance.len() <= 2000,
            "invalid utterance"
        );
        let physical = self.physical_context(v);
        let c = self.cases.get_mut(&delivery.vehicle_id).unwrap();
        ensure!(
            delivery.conversation_epoch == c.conversation_epoch,
            "conversation ownership changed"
        );
        ensure!(
            delivery.case_epoch == c.epoch,
            "reply belongs to an earlier shift"
        );
        ensure!(
            delivery.expected_turn == c.history.len() as u64,
            "stale actor turn"
        );
        ensure!(c.history.len() < 128, "conversation limit reached");
        let observed = c.scenario.with_observations(
            &v.observed_evidence
                .iter()
                .map(|e| e.id.clone())
                .collect::<Vec<_>>(),
        );
        let reply = validate_turn_reply(
            &observed,
            c.secondary,
            &c.history,
            &delivery.officer_utterance,
            &delivery.raw_reply,
        )?;
        if reply.action != "none" {
            bail!("unsupported actor action");
        }
        c.history.push(Turn {
            officer: delivery.officer_utterance.clone(),
            traveler: reply.dialogue.clone(),
            emotion: reply.emotion.clone(),
            physical,
        });
        self.world
            .vehicles
            .iter_mut()
            .find(|v| v.vehicle_id == delivery.vehicle_id)
            .unwrap()
            .history = c.history.clone();
        self.emit("traveler_spoke",json!({"vehicle_id":delivery.vehicle_id,"officer_utterance":delivery.officer_utterance,"dialogue":reply.dialogue,"emotion":reply.emotion}));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sim() -> Simulation {
        Simulation::new(
            "test".into(),
            42,
            border_scenarios::load_pack(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scenarios"),
            )
            .unwrap(),
        )
        .unwrap()
    }
    fn send(s: &mut Simulation, p: u64, c: ClientCommand) -> Result<CommandReceipt> {
        let n = s.peers[&p].next_sequence;
        s.command(
            p,
            CommandEnvelope {
                protocol_version: PROTOCOL_VERSION,
                session_id: "test".into(),
                request_id: format!("{p}-{n}"),
                sequence: n,
                expected_revision: None,
                command: c,
            },
            0,
        )
    }
    fn active() -> Simulation {
        let mut s = sim();
        for p in 1..=2 {
            s.join(format!("Officer {p}"), 0).unwrap();
            s.world
                .peers
                .iter_mut()
                .find(|v| v.peer_id == p)
                .unwrap()
                .ready = true;
            send(&mut s, p, ClientCommand::ClaimLane { lane_id: p as u8 }).unwrap();
        }
        s.operator(HostCommand::StartShift).unwrap();
        for _ in 0..12 {
            s.operator(HostCommand::AdvanceWorld { delta_ms: 1000 })
                .unwrap();
        }
        for p in &mut s.world.peers {
            p.position = if p.peer_id == 1 {
                [-1.6, 1.7, 0.]
            } else {
                [5.4, 1.7, 0.]
            };
        }
        s
    }
    #[test]
    fn requires_two_ready_peers_and_exclusive_lanes() {
        let mut s = sim();
        s.join("A".into(), 0).unwrap();
        assert!(send(&mut s, 1, ClientCommand::SetReady { ready: true }).is_err());
        assert!(s.operator(HostCommand::StartShift).is_err());
        s.join("B".into(), 0).unwrap();
        send(&mut s, 1, ClientCommand::ClaimLane { lane_id: 1 }).unwrap();
        assert!(send(&mut s, 2, ClientCommand::ClaimLane { lane_id: 1 }).is_err());
        assert!(s.join("C".into(), 0).is_err());
    }
    #[test]
    fn private_truth_and_memory_never_serialize() {
        let s = active();
        let raw = serde_json::to_string(&s.snapshot()).unwrap();
        for key in [
            "truth",
            "smuggling",
            "contradictions",
            "actor",
            "fact_ids",
            "resolution_rules",
        ] {
            assert!(!raw.contains(&format!("\"{key}\"")));
        }
        assert!(s.snapshot().vehicles[0].observed_evidence.is_empty());
    }
    #[test]
    fn replay_cannot_double_award_or_mutate_request() {
        let mut s = active();
        let n = s.peers[&1].next_sequence;
        let e = CommandEnvelope {
            protocol_version: PROTOCOL_VERSION,
            session_id: "test".into(),
            request_id: "decision".into(),
            sequence: n,
            expected_revision: None,
            command: ClientCommand::Decide {
                vehicle_id: 1,
                disposition: Disposition::Admit,
            },
        };
        let a = s.command(1, e.clone(), 0).unwrap();
        let money = s.snapshot().money;
        let b = s.command(1, e.clone(), 0).unwrap();
        assert_eq!(a.revision, b.revision);
        assert_eq!(s.snapshot().money, money);
        assert_eq!(s.snapshot().ledger.len(), 1);
        let mut altered = e;
        altered.command = ClientCommand::Decide {
            vehicle_id: 1,
            disposition: Disposition::Deny,
        };
        assert!(s.command(1, altered, 0).is_err());
        send(
            &mut s,
            1,
            ClientCommand::Decide {
                vehicle_id: 1,
                disposition: Disposition::Admit,
            },
        )
        .unwrap();
        assert_eq!(s.world.ledger.len(), 1);
    }
    #[test]
    fn health_money_and_incidents_require_host_authority() {
        let mut s = active();
        assert!(
            send(
                &mut s,
                2,
                ClientCommand::Decide {
                    vehicle_id: 1,
                    disposition: Disposition::Admit
                }
            )
            .is_err()
        );
        s.operator(HostCommand::ApplyDamage {
            peer_id: 1,
            amount: 100,
        })
        .unwrap();
        assert!(
            send(
                &mut s,
                1,
                ClientCommand::InspectEvidence {
                    vehicle_id: 1,
                    evidence_id: "x".into()
                }
            )
            .is_err()
        );
        s.operator(HostCommand::RestoreHealth).unwrap();
        send(
            &mut s,
            1,
            ClientCommand::RequestPurchase {
                upgrade_id: "document_scanner".into(),
            },
        )
        .unwrap();
        assert!(
            s.operator(HostCommand::ApprovePurchase { proposal_id: 1 })
                .is_err()
        );
        assert!(s.snapshot().upgrades.is_empty());
        s.world.money = 300;
        s.operator(HostCommand::ApprovePurchase { proposal_id: 1 })
            .unwrap();
        assert_eq!(s.snapshot().money, 0);
        assert!(
            s.operator(HostCommand::ApprovePurchase { proposal_id: 1 })
                .is_err()
        );
        s.operator(HostCommand::CreateIncident {
            kind: IncidentKind::FenceBreach,
        })
        .unwrap();
        s.operator(HostCommand::ResolveIncident { incident_id: 1 })
            .unwrap();
        assert!(s.snapshot().incidents[0].resolved);
    }
    #[test]
    fn evidence_must_exist_and_be_observed_by_lane_owner() {
        let mut s = active();
        let id = s.cases[&1].scenario.evidence[0].id.clone();
        assert!(
            send(
                &mut s,
                2,
                ClientCommand::InspectEvidence {
                    vehicle_id: 1,
                    evidence_id: id.clone()
                }
            )
            .is_err()
        );
        send(
            &mut s,
            1,
            ClientCommand::InspectEvidence {
                vehicle_id: 1,
                evidence_id: id,
            },
        )
        .unwrap();
        assert_eq!(s.snapshot().vehicles[0].observed_evidence.len(), 1);
        assert!(
            send(
                &mut s,
                1,
                ClientCommand::InspectEvidence {
                    vehicle_id: 1,
                    evidence_id: "invented".into()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn timeout_releases_lane_and_replay_requires_resync() {
        let mut s = active();
        s.heartbeat(2, 14_000).unwrap();
        s.expire(15_000);
        assert_eq!(s.snapshot().lanes[0].owner, None);
        assert!(!s.snapshot().peers[0].connected);
        assert!(s.snapshot().peers[1].connected);
        assert!(s.heartbeat(1, 15_001).is_err());
        for _ in 0..520 {
            s.emit("test", json!({}));
        }
        assert!(s.replay(0).is_err());
        let r = s.replay(s.world.latest_event_sequence - 2).unwrap();
        assert_eq!(r.events.len(), 2);
        assert_ne!(r.events[0].event_id, r.events[1].event_id);
    }
    #[test]
    fn seeds_and_ticks_are_deterministic() {
        let a = active();
        let b = active();
        assert_eq!(
            serde_json::to_value(a.snapshot()).unwrap(),
            serde_json::to_value(b.snapshot()).unwrap()
        );
        assert!(a.check_session(999, "test").is_err());
        assert!(a.check_session(PROTOCOL_VERSION, "other").is_err());
    }
    #[test]
    fn actor_output_is_revalidated_before_broadcast() {
        let mut s = active();
        let d=ActorDelivery{conversation_epoch:0,case_epoch:0,expected_turn:0,protocol_version:PROTOCOL_VERSION,session_id:"test".into(),vehicle_id:1,officer_utterance:"Hello".into(),raw_reply:r#"{"dialogue":"Hello officer.","emotion":"cooperative","action":"award_money","fact_ids":[],"end_conversation":false}"#.into()};
        assert!(s.actor(d.clone()).is_err());
        assert!(s.cases[&1].history.is_empty());
        let mut d = d;
        d.raw_reply = d.raw_reply.replace("award_money", "none");
        s.actor(d).unwrap();
        assert_eq!(s.cases[&1].history.len(), 1);
        assert_eq!(s.events.back().unwrap().kind, "traveler_spoke");
    }
    #[test]
    fn stale_revision_and_out_of_order_commands_leave_state_unchanged() {
        let mut s = active();
        let revision = s.world.revision;
        let mut e = CommandEnvelope {
            protocol_version: PROTOCOL_VERSION,
            session_id: "test".into(),
            request_id: "stale".into(),
            sequence: s.peers[&1].next_sequence,
            expected_revision: Some(0),
            command: ClientCommand::ReleaseLane { lane_id: 1 },
        };
        assert!(s.command(1, e.clone(), 0).is_err());
        e.expected_revision = None;
        e.sequence += 10;
        assert!(s.command(1, e, 0).is_err());
        assert_eq!(s.world.revision, revision);
    }
    #[test]
    fn old_ai_reply_cannot_enter_a_new_shift() {
        let mut s = active();
        let old = s.actor_context(1).unwrap();
        s.world.phase = ShiftPhase::Grace;
        s.reset_shift(1).unwrap();
        s.world.peers[0].ready = true;
        s.operator(HostCommand::StartShift).unwrap();
        for _ in 0..12 {
            s.operator(HostCommand::AdvanceWorld { delta_ms: 1000 })
                .unwrap();
        }
        assert_ne!(old.case_epoch, s.actor_context(1).unwrap().case_epoch);
        let delivery=ActorDelivery{conversation_epoch:0,case_epoch:old.case_epoch,expected_turn:0,protocol_version:PROTOCOL_VERSION,session_id:"test".into(),vehicle_id:1,officer_utterance:"Hello".into(),raw_reply:r#"{"dialogue":"Hello officer.","emotion":"cooperative","action":"none","fact_ids":[],"end_conversation":false}"#.into()};
        assert!(s.actor(delivery).is_err());
        assert!(s.cases[&1].history.is_empty());
    }
}
