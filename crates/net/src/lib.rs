//! Versioned wire contracts. Requests express intent; authoritative outcomes are server output.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u16 = 1;
pub const PEER_TIMEOUT_MS: u64 = 15_000;
pub const HTTP_TIMEOUT_SECS: u64 = 5;
pub const MAX_PLAYERS: usize = 2;
pub const LANE_COUNT: u8 = 2;
pub const MAX_REQUEST_BYTES: usize = 8192;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinRequest {
    pub protocol_version: u16,
    pub join_code: String,
    pub display_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinResponse {
    pub protocol_version: u16,
    pub session_id: String,
    pub peer_id: u64,
    pub peer_token: String,
    pub next_sequence: u64,
    pub snapshot: WorldSnapshot,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionRequest {
    pub protocol_version: u16,
    pub session_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandEnvelope {
    pub protocol_version: u16,
    pub session_id: String,
    pub request_id: String,
    pub sequence: u64,
    pub expected_revision: Option<u64>,
    pub command: ClientCommand,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientCommand {
    SetReady {
        ready: bool,
    },
    ClaimLane {
        lane_id: u8,
    },
    ReleaseLane {
        lane_id: u8,
    },
    InspectEvidence {
        vehicle_id: u64,
        evidence_id: String,
    },
    Decide {
        vehicle_id: u64,
        disposition: Disposition,
    },
    RequestPurchase {
        upgrade_id: String,
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Admit,
    Deny,
    Detain,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandReceipt {
    pub request_id: String,
    pub sequence: u64,
    pub revision: u64,
    pub event_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub event_id: String,
    pub sequence: u64,
    pub revision: u64,
    pub kind: String,
    pub data: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replay {
    pub session_id: String,
    pub latest_sequence: u64,
    pub events: Vec<DomainEvent>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ShiftPhase {
    Lobby,
    Active,
    Grace,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VehicleStage {
    Queued,
    Stopped,
    DepartingAdmit,
    DepartingDeny,
    Detained,
    Exited,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerView {
    pub peer_id: u64,
    pub display_name: String,
    pub ready: bool,
    pub connected: bool,
    pub health: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneView {
    pub lane_id: u8,
    pub owner: Option<u64>,
    pub barrier_open: bool,
    pub queue: Vec<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceView {
    pub id: String,
    pub description: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleView {
    pub vehicle_id: u64,
    pub lane_id: u8,
    pub kind: String,
    pub stage: VehicleStage,
    pub position_mm: u32,
    pub traveler_name: String,
    pub opening_line: String,
    pub observed_evidence: Vec<EvidenceView>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub vehicle_id: u64,
    pub disposition: Disposition,
    pub correct: bool,
    pub amount: i64,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseView {
    pub proposal_id: u64,
    pub peer_id: u64,
    pub upgrade_id: String,
    pub approved: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentView {
    pub incident_id: u64,
    pub kind: IncidentKind,
    pub resolved: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentKind {
    FenceBreach,
    Flight,
    ArmedAttack,
    Explosion,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub protocol_version: u16,
    pub session_id: String,
    pub revision: u64,
    pub latest_event_sequence: u64,
    pub seed: u64,
    pub phase: ShiftPhase,
    pub remaining_ms: u64,
    pub peers: Vec<PeerView>,
    pub lanes: Vec<LaneView>,
    pub vehicles: Vec<VehicleView>,
    pub money: i64,
    pub ledger: Vec<LedgerEntry>,
    pub purchases: Vec<PurchaseView>,
    pub upgrades: Vec<String>,
    pub incidents: Vec<IncidentView>,
}
/// Only the operator endpoint may deserialize these commands. Never put them in ClientCommand.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostCommand {
    StartShift,
    AdvanceWorld { delta_ms: u64 },
    SpawnNextVehicle { lane_id: u8, kind: String },
    ApplyDamage { peer_id: u64, amount: u8 },
    RestoreHealth,
    CreateIncident { kind: IncidentKind },
    ResolveIncident { incident_id: u64 },
    ApprovePurchase { proposal_id: u64 },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostEnvelope {
    pub protocol_version: u16,
    pub session_id: String,
    pub command: HostCommand,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorDelivery {
    pub expected_turn: u64,
    pub protocol_version: u16,
    pub session_id: String,
    pub vehicle_id: u64,
    pub officer_utterance: String,
    pub raw_reply: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_cannot_inject_authoritative_fields_or_commands() {
        for raw in [
            r#"{"type":"decide","vehicle_id":1,"disposition":"admit","reward":9999}"#,
            r#"{"type":"apply_damage","peer_id":2,"amount":100}"#,
            r#"{"type":"award_money","amount":9999}"#,
            r#"{"type":"claim_lane","lane_id":1,"peer_id":2}"#,
            r#"{"type":"request_purchase","upgrade_id":"document_scanner","cost":0}"#,
        ] {
            assert!(serde_json::from_str::<ClientCommand>(raw).is_err());
        }
    }
    #[test]
    fn wire_round_trip_preserves_protocol_and_command_identity() {
        let e = CommandEnvelope {
            protocol_version: PROTOCOL_VERSION,
            session_id: "room".into(),
            request_id: "r1".into(),
            sequence: 1,
            expected_revision: Some(0),
            command: ClientCommand::ClaimLane { lane_id: 1 },
        };
        let r: CommandEnvelope = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(r.command, e.command);
        assert_eq!(r.request_id, e.request_id);
    }
}

/// Only an authenticated local gateway caller can request provider work.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostTurnRequest {
    pub protocol_version: u16,
    pub session_id: String,
    pub vehicle_id: u64,
    pub utterance: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorContextRequest {
    pub protocol_version: u16,
    pub session_id: String,
    pub vehicle_id: u64,
}
