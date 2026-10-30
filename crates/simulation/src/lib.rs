//! Renderer-free authority. Hidden cases and conversation history never enter public snapshots.
use anyhow::{Result, bail, ensure};
use border_caller::{Turn, validate_turn_reply};
use border_net::*;
use border_scenarios::{Scenario, seeded_order};
use serde_json::json;
use std::collections::{HashMap, VecDeque};

struct Case {
    scenario: Scenario,
    history: Vec<Turn>,
}
struct Peer {
    last_seen: u64,
    next_sequence: u64,
    receipts: HashMap<String, (String, CommandReceipt)>,
}
/// Secret server-to-server context. Never return this from a peer route.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ActorContext {
    pub scenario: Scenario,
    pub history: Vec<Turn>,
    pub expected_turn: u64,
}
pub struct Simulation {
    world: WorldSnapshot,
    pack: Vec<Scenario>,
    order: Vec<usize>,
    cursor: usize,
    cases: HashMap<u64, Case>,
    peers: HashMap<u64, Peer>,
    events: VecDeque<DomainEvent>,
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
                        queue: vec![],
                    })
                    .collect(),
                vehicles: vec![],
                money: 0,
                ledger: vec![],
                purchases: vec![],
                upgrades: vec![],
                incidents: vec![],
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
        });
        self.peers.insert(
            id,
            Peer {
                last_seen: now,
                next_sequence: 1,
                receipts: HashMap::new(),
            },
        );
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
        ensure!(v.stage == VehicleStage::Stopped, "vehicle not stopped");
        ensure!(
            self.world
                .lanes
                .iter()
                .any(|l| l.lane_id == v.lane_id && l.owner == Some(peer)),
            "lane not owned"
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
                .any(|p| p.peer_id == peer && p.health > 0),
            "peer incapacitated"
        );
        match &e.command {
            ClientCommand::SetReady { ready } => {
                ensure!(
                    self.world.phase == ShiftPhase::Lobby,
                    "shift already started"
                );
                self.world
                    .peers
                    .iter_mut()
                    .find(|p| p.peer_id == peer)
                    .unwrap()
                    .ready = *ready;
            }
            ClientCommand::ClaimLane { lane_id } => {
                ensure!(
                    !self
                        .world
                        .lanes
                        .iter()
                        .any(|l| l.owner == Some(peer) && l.lane_id != *lane_id),
                    "release current lane first"
                );
                let l = self
                    .world
                    .lanes
                    .iter_mut()
                    .find(|l| l.lane_id == *lane_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown lane"))?;
                ensure!(l.owner.is_none() || l.owner == Some(peer), "lane occupied");
                l.owner = Some(peer);
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
            ClientCommand::InspectEvidence {
                vehicle_id,
                evidence_id,
            } => {
                let i = self.owns_vehicle(peer, *vehicle_id)?;
                let evidence = self.cases[vehicle_id]
                    .scenario
                    .evidence
                    .iter()
                    .find(|e| e.id == *evidence_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown evidence"))?;
                if !self.world.vehicles[i]
                    .observed_evidence
                    .iter()
                    .any(|e| e.id == *evidence_id)
                {
                    self.world.vehicles[i].observed_evidence.push(EvidenceView {
                        id: evidence.id.clone(),
                        description: evidence.description.clone(),
                    });
                }
            }
            ClientCommand::Decide {
                vehicle_id,
                disposition,
            } => {
                let i = self.owns_vehicle(peer, *vehicle_id)?;
                let guilty = self.cases[vehicle_id].scenario.truth.smuggling;
                let correct = (*disposition == Disposition::Admit) != guilty;
                let amount = match (*disposition, correct) {
                    (Disposition::Admit, true) => 100,
                    (Disposition::Admit, false) => -400,
                    (Disposition::Deny, true) => 200,
                    (Disposition::Detain, true) => 250,
                    _ => -150,
                };
                self.world.money += amount;
                self.world.ledger.push(LedgerEntry {
                    vehicle_id: *vehicle_id,
                    disposition: *disposition,
                    correct,
                    amount,
                    reason: if correct {
                        "correct disposition"
                    } else {
                        "incorrect disposition"
                    }
                    .into(),
                });
                self.world.vehicles[i].stage = match disposition {
                    Disposition::Admit => VehicleStage::DepartingAdmit,
                    Disposition::Deny => VehicleStage::DepartingDeny,
                    Disposition::Detain => VehicleStage::Detained,
                };
                let lane = self.world.vehicles[i].lane_id;
                self.world
                    .lanes
                    .iter_mut()
                    .find(|l| l.lane_id == lane)
                    .unwrap()
                    .barrier_open = *disposition == Disposition::Admit;
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
        // Retain every receipt for the session; the command limit bounds memory.
        p.receipts
            .insert(e.request_id, (fingerprint, receipt.clone()));
        Ok(receipt)
    }
    pub fn operator(&mut self, command: HostCommand) -> Result<()> {
        match &command {
            HostCommand::StartShift => {
                ensure!(
                    self.world.phase == ShiftPhase::Lobby,
                    "shift already started"
                );
                let peers: Vec<_> = self.world.peers.iter().filter(|p| p.connected).collect();
                ensure!(
                    peers.len() == 2 && peers.iter().all(|p| p.ready),
                    "two ready peers required"
                );
                self.world.phase = ShiftPhase::Active;
            }
            HostCommand::SpawnNextVehicle { lane_id, kind } => {
                ensure!(self.world.phase == ShiftPhase::Active, "shift inactive");
                ensure!(
                    ["car", "van", "truck"].contains(&kind.as_str()),
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
                let scenario = self.pack[self.order[self.cursor]].clone();
                self.cursor += 1;
                l.queue.push(vehicle_id);
                self.world.vehicles.push(VehicleView {
                    vehicle_id,
                    lane_id: *lane_id,
                    kind: kind.clone(),
                    stage: VehicleStage::Queued,
                    position_mm: 0,
                    traveler_name: scenario.character.name.clone(),
                    opening_line: scenario.opening_line.clone(),
                    observed_evidence: vec![],
                });
                self.cases.insert(
                    vehicle_id,
                    Case {
                        scenario,
                        history: vec![],
                    },
                );
            }
            HostCommand::AdvanceWorld { delta_ms } => {
                ensure!(*delta_ms <= 1000, "tick exceeds 1000 ms");
                ensure!(self.world.phase != ShiftPhase::Lobby, "shift inactive");
                self.world.remaining_ms = self.world.remaining_ms.saturating_sub(*delta_ms);
                if self.world.remaining_ms == 0 {
                    self.world.phase = ShiftPhase::Grace;
                }
                for l in &mut self.world.lanes {
                    if let Some(id) = l.queue.first() {
                        let v = self
                            .world
                            .vehicles
                            .iter_mut()
                            .find(|v| v.vehicle_id == *id)
                            .unwrap();
                        match v.stage {
                            VehicleStage::Queued => {
                                v.position_mm =
                                    (v.position_mm + (*delta_ms as u32) * 5).min(10_000);
                                if v.position_mm == 10_000 {
                                    v.stage = VehicleStage::Stopped;
                                }
                            }
                            VehicleStage::DepartingAdmit | VehicleStage::DepartingDeny => {
                                v.position_mm += (*delta_ms as u32) * 5;
                                if v.position_mm >= 20_000 {
                                    v.stage = VehicleStage::Exited;
                                }
                            }
                            _ => {}
                        }
                        if matches!(v.stage, VehicleStage::Exited | VehicleStage::Detained) {
                            l.queue.remove(0);
                            l.barrier_open = false;
                        }
                    }
                }
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
        ensure!(v.stage == VehicleStage::Stopped, "vehicle not stopped");
        let c = &self.cases[&vehicle];
        ensure!(c.history.len() < 128, "conversation limit reached");
        Ok(ActorContext {
            scenario: c.scenario.clone(),
            history: c.history.clone(),
            expected_turn: c.history.len() as u64,
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
        ensure!(v.stage == VehicleStage::Stopped, "vehicle not stopped");
        ensure!(
            !delivery.officer_utterance.trim().is_empty()
                && delivery.officer_utterance.len() <= 2000,
            "invalid utterance"
        );
        let c = self.cases.get_mut(&delivery.vehicle_id).unwrap();
        // This foundation supports public facts only. Evidence-aware conversation replication is Part 15.
        ensure!(
            delivery.expected_turn == c.history.len() as u64,
            "stale actor turn"
        );
        ensure!(c.history.len() < 128, "conversation limit reached");
        let reply = validate_turn_reply(
            &c.scenario,
            false,
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
        });
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
            send(&mut s, p, ClientCommand::SetReady { ready: true }).unwrap();
            send(&mut s, p, ClientCommand::ClaimLane { lane_id: p as u8 }).unwrap();
        }
        s.operator(HostCommand::StartShift).unwrap();
        s.operator(HostCommand::SpawnNextVehicle {
            lane_id: 1,
            kind: "car".into(),
        })
        .unwrap();
        for _ in 0..2 {
            s.operator(HostCommand::AdvanceWorld { delta_ms: 1000 })
                .unwrap();
        }
        s
    }
    #[test]
    fn requires_two_ready_peers_and_exclusive_lanes() {
        let mut s = sim();
        s.join("A".into(), 0).unwrap();
        send(&mut s, 1, ClientCommand::SetReady { ready: true }).unwrap();
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
            "history",
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
        assert!(
            send(
                &mut s,
                1,
                ClientCommand::Decide {
                    vehicle_id: 1,
                    disposition: Disposition::Admit
                }
            )
            .is_err()
        );
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
        let d=ActorDelivery{expected_turn:0,protocol_version:PROTOCOL_VERSION,session_id:"test".into(),vehicle_id:1,officer_utterance:"Hello".into(),raw_reply:r#"{"dialogue":"Hello officer.","emotion":"cooperative","action":"award_money","fact_ids":[],"end_conversation":false}"#.into()};
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
}
