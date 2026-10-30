use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub biography: String,
    pub occupation: String,
    pub speech_style: String,
    pub emotion_baseline: String,
    #[serde(default)]
    pub authority_response: AuthorityResponse,
    #[serde(default)]
    pub compliance_bounds: String,
    #[serde(default)]
    pub variation: PersonalityVariation,
    #[serde(default)]
    pub bribery_capable: bool,
    #[serde(default)]
    pub psychology: PsychologyTraits,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsychologyTraits {
    pub survival: u8,
    pub boldness: u8,
    pub integrity: u8,
    pub aggression: u8,
    pub sociability: u8,
    pub greed: u8,
    pub pride: u8,
}
impl Default for PsychologyTraits {
    fn default() -> Self {
        Self {
            survival: 80,
            boldness: 40,
            integrity: 60,
            aggression: 20,
            sociability: 50,
            greed: 30,
            pride: 50,
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhysicalContext {
    pub officer_peer: Option<u64>,
    pub weapon_drawn: bool,
    pub weapon_aimed: bool,
    pub injured: bool,
    pub violence_nearby: bool,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityResponse {
    Eager,
    Pragmatic,
    Opportunistic,
    Playful,
    Reluctant,
    #[default]
    Principled,
    Defiant,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Truth {
    pub purpose: String,
    pub itinerary: String,
    pub cargo: String,
    pub smuggling: bool,
    pub documents: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RevealCondition {
    Documents,
    Secondary,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    #[serde(default)]
    pub location: EvidenceLocation,
    #[serde(default)]
    pub reveal_rule: String,
    pub id: String,
    pub description: String,
    pub reveal_condition: RevealCondition,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub claim_id: String,
    pub evidence_id: String,
    pub explanation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub purpose: String,
    pub itinerary: String,
    pub cargo: String,
    pub repeat: String,
    pub irrelevant: String,
    pub confront: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionRules {
    pub admit_if: String,
    pub deny_if: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    #[serde(default)]
    pub observation_scope: bool,
    #[serde(default)]
    pub encounter: PhysicalContext,
    #[serde(default)]
    pub metadata: Option<CaseMetadata>,
    #[serde(default)]
    pub record_seed: u64,
    pub schema_version: u32,
    pub scenario_id: String,
    pub seed: u64,
    pub character: Character,
    pub truth: Truth,
    pub public_claims: Vec<Claim>,
    pub evidence: Vec<Evidence>,
    pub contradictions: Vec<Contradiction>,
    pub allowed_actions: Vec<String>,
    pub opening_line: String,
    pub resolution_rules: ResolutionRules,
    pub actor: Actor,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalityVariation {
    pub mannerism: String,
    pub interest: String,
    pub stress_response: String,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLocation {
    #[default]
    Documents,
    Glovebox,
    Cargo,
    Container,
    Trailer,
}
impl EvidenceLocation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Documents => "Presented documents",
            Self::Glovebox => "Glove compartment",
            Self::Cargo => "Trunk / pickup bed",
            Self::Container => "Cargo container",
            Self::Trailer => "Utility trailer",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseMetadata {
    pub role: String,
    pub reason: String,
    pub cargo_kind: String,
    pub document_holder: String,
    pub document_purpose: String,
    pub document_itinerary: String,
    pub document_cargo: String,
}
#[derive(Deserialize)]
struct Templates {
    version: u32,
    templates: Vec<Archetype>,
    mannerisms: Vec<String>,
    interests: Vec<String>,
    stress_responses: Vec<String>,
}
#[derive(Deserialize)]
struct Archetype {
    occupation: String,
    scenario_id: String,
    role: String,
    allowed_reasons: Vec<String>,
    allowed_cargo: Vec<String>,
    smuggling: bool,
}
fn templates() -> Templates {
    serde_json::from_str(include_str!("../../../assets/case_templates.json"))
        .expect("bundled authored templates")
}
impl Scenario {
    pub fn with_observations(&self, ids: &[String]) -> Self {
        let mut actor_case = self.clone();
        actor_case.observation_scope = true;
        actor_case.evidence.retain(|e| ids.contains(&e.id));
        actor_case
            .contradictions
            .retain(|c| ids.contains(&c.evidence_id));
        actor_case
    }
    pub fn instantiate(&self, seed: u64) -> Self {
        let t = templates();
        let mut case = self.clone();
        case.record_seed = seed ^ self.seed.rotate_left(17);
        let order = seeded_order(64, case.record_seed);
        case.character.variation = PersonalityVariation {
            mannerism: t.mannerisms[order[0] % t.mannerisms.len()].clone(),
            interest: t.interests[order[1] % t.interests.len()].clone(),
            stress_response: t.stress_responses[order[2] % t.stress_responses.len()].clone(),
        };
        case
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema_version == 1, "Unsupported scenario schema");
        let p = &self.character.psychology;
        ensure!(
            [
                p.survival,
                p.boldness,
                p.integrity,
                p.aggression,
                p.sociability,
                p.greed,
                p.pride
            ]
            .iter()
            .all(|x| *x <= 100),
            "Psychology traits must be 0–100"
        );
        for value in [
            &self.scenario_id,
            &self.character.name,
            &self.character.biography,
            &self.character.speech_style,
            &self.truth.purpose,
            &self.truth.itinerary,
            &self.truth.cargo,
            &self.opening_line,
            &self.actor.purpose,
            &self.actor.cargo,
            &self.actor.itinerary,
            &self.actor.repeat,
            &self.actor.irrelevant,
            &self.actor.confront,
        ] {
            ensure!(!value.trim().is_empty(), "Empty required scenario field");
        }
        ensure!(!self.truth.documents.is_empty(), "Missing documents");
        let t = templates();
        ensure!(t.version == 1, "Unsupported archetype version");
        let m = self
            .metadata
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing canonical case metadata"))?;
        let archetype = t
            .templates
            .iter()
            .find(|a| a.scenario_id == self.scenario_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown authored archetype"))?;
        ensure!(
            self.character.occupation == archetype.occupation
                && m.role == archetype.role
                && archetype.allowed_reasons.contains(&m.reason)
                && archetype.allowed_cargo.contains(&m.cargo_kind),
            "Incompatible role/reason/cargo combination"
        );
        ensure!(
            self.truth.smuggling == archetype.smuggling,
            "Legal status conflicts with authored archetype"
        );
        ensure!(
            m.document_holder == self.character.name
                && m.document_purpose == self.truth.purpose
                && m.document_itinerary == self.truth.itinerary
                && m.document_cargo == self.truth.cargo,
            "Documents contradict canonical identity, purpose, itinerary or cargo"
        );
        ensure!(
            matches!(
                self.character.emotion_baseline.as_str(),
                "cooperative"
                    | "confused"
                    | "suspicious"
                    | "irritated"
                    | "defensive"
                    | "angry"
                    | "disengaged"
                    | "afraid"
                    | "panicked"
                    | "desperate"
            ),
            "Invalid emotion"
        );
        ensure!(
            self.allowed_actions == ["none"],
            "Unsupported actor actions"
        );
        ensure!(
            self.resolution_rules.admit_if == "no_undeclared_commercial_goods"
                && self.resolution_rules.deny_if == "undeclared_commercial_goods",
            "Unsupported resolution rule"
        );
        let mut ids: HashSet<&str> = ["purpose", "itinerary", "cargo"].into_iter().collect();
        for c in &self.public_claims {
            ensure!(
                !c.id.is_empty() && !c.text.is_empty() && ids.insert(&c.id),
                "Duplicate/empty claim ID"
            );
        }
        for e in &self.evidence {
            ensure!(
                (e.location == EvidenceLocation::Documents)
                    == (e.reveal_condition == RevealCondition::Documents),
                "Evidence location/reveal mismatch"
            );
            ensure!(
                e.reveal_rule
                    == if e.location == EvidenceLocation::Documents {
                        "presented_at_driver"
                    } else {
                        "physical_search"
                    },
                "Invalid evidence reveal rule"
            );
            ensure!(
                !e.id.is_empty() && !e.description.is_empty() && ids.insert(&e.id),
                "Duplicate/empty evidence ID"
            );
        }
        ensure!(
            self.evidence
                .iter()
                .any(|e| e.reveal_condition == RevealCondition::Documents),
            "No document evidence"
        );
        ensure!(
            self.evidence
                .iter()
                .any(|e| e.reveal_condition == RevealCondition::Secondary),
            "No cargo evidence"
        );
        for c in &self.contradictions {
            ensure!(
                self.public_claims.iter().any(|x| x.id == c.claim_id),
                "Dangling claim"
            );
            ensure!(
                self.evidence.iter().any(|x| x.id == c.evidence_id),
                "Dangling evidence"
            );
            ensure!(!c.explanation.is_empty(), "Empty contradiction");
        }
        ensure!(
            self.truth.smuggling != self.contradictions.is_empty(),
            "Guilt and contradiction mismatch"
        );
        Ok(())
    }
    pub fn visible_evidence(&self, secondary: bool) -> Vec<Evidence> {
        self.evidence
            .iter()
            .filter(|e| {
                self.observation_scope
                    || secondary
                    || e.reveal_condition == RevealCondition::Documents
            })
            .cloned()
            .collect()
    }
    pub fn fact_allowed(&self, id: &str, secondary: bool) -> bool {
        matches!(id, "purpose" | "itinerary")
            || (id == "cargo" && (!self.truth.smuggling || secondary))
            || self.public_claims.iter().any(|c| c.id == id)
            || self.visible_evidence(secondary).iter().any(|e| e.id == id)
    }
    pub fn resolve(&self, decision: Decision, secondary: bool) -> Reveal {
        let correct = match decision {
            Decision::Admit => !self.truth.smuggling,
            Decision::Deny | Decision::Detain => self.truth.smuggling,
        };
        Reveal {
            name: self.character.name.clone(),
            smuggling: self.truth.smuggling,
            correct,
            truth: self.truth.clone(),
            decision,
            clues: self
                .contradictions
                .iter()
                .map(|c| c.explanation.clone())
                .collect(),
            missed_clues: self
                .contradictions
                .iter()
                .filter(|c| {
                    self.evidence.iter().any(|e| {
                        e.id == c.evidence_id && e.reveal_condition == RevealCondition::Secondary
                    }) && !secondary
                })
                .map(|c| c.explanation.clone())
                .collect(),
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Admit,
    Deny,
    Detain,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reveal {
    pub name: String,
    pub smuggling: bool,
    pub correct: bool,
    pub truth: Truth,
    pub decision: Decision,
    pub clues: Vec<String>,
    pub missed_clues: Vec<String>,
}
pub fn load_pack(path: &Path) -> Result<Vec<Scenario>> {
    let mut files: Vec<_> = std::fs::read_dir(path)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let pack: Vec<Scenario> = files
        .iter()
        .map(|p| Ok(serde_json::from_slice(&std::fs::read(p)?)?))
        .collect::<Result<_>>()?;
    validate_pack(&pack)?;
    Ok(pack)
}
pub fn validate_pack(pack: &[Scenario]) -> Result<()> {
    ensure!(pack.len() == 10, "Expected exactly ten scenarios");
    ensure!(
        pack.iter().filter(|s| s.truth.smuggling).count() == 3,
        "Expected exactly three smugglers"
    );
    let mut ids = HashSet::new();
    let mut seeds = HashSet::new();
    let mut names = HashSet::new();
    for s in pack {
        ensure!(
            !s.observation_scope,
            "Authored pack cannot pre-disclose evidence"
        );
        s.validate()?;
        ensure!(
            names.insert(s.character.name.trim().to_lowercase()),
            "Duplicate traveler name"
        );
        if !ids.insert(&s.scenario_id) || !seeds.insert(s.seed) {
            bail!("Duplicate scenario ID/seed");
        }
    }
    Ok(())
}
/// Platform-independent Fisher-Yates order. Every case appears once per shift.
pub fn seeded_order(count: usize, seed: u64) -> Vec<usize> {
    let mut order: Vec<_> = (0..count).collect();
    let mut state = seed.max(1);
    for i in (1..count).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        order.swap(i, state as usize % (i + 1));
    }
    order
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pack() -> Vec<Scenario> {
        load_pack(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/scenarios"
        )))
        .unwrap()
    }
    #[test]
    fn pack_is_exact_and_fair() {
        let p = pack();
        validate_pack(&p).unwrap();
        for s in p {
            assert_eq!(s.resolve(Decision::Deny, false).correct, s.truth.smuggling);
            assert_eq!(s.resolve(Decision::Admit, true).correct, !s.truth.smuggling);
            if s.truth.smuggling {
                assert!(s.contradictions.iter().any(|c| {
                    s.visible_evidence(false)
                        .iter()
                        .any(|e| e.id == c.evidence_id)
                }));
            }
        }
    }
    #[test]
    fn order_is_reproducible_without_replacement() {
        let a = seeded_order(10, 42);
        assert_eq!(a, seeded_order(10, 42));
        assert_ne!(a, seeded_order(10, 43));
        let mut sorted = a;
        sorted.sort();
        assert_eq!(sorted, (0..10).collect::<Vec<_>>());
    }
    #[test]
    fn malformed_truth_and_references_rejected() {
        let mut p = pack();
        p[0].truth.smuggling = true;
        assert!(validate_pack(&p).is_err());
        let mut p = pack();
        p[2].contradictions[0].evidence_id = "invented".into();
        assert!(validate_pack(&p).is_err());
        let mut p = pack();
        p[0].evidence[1].id = p[0].evidence[0].id.clone();
        assert!(validate_pack(&p).is_err());
    }
    #[test]
    fn seeded_variation_changes_personality_but_never_truth_or_evidence() {
        let p = pack();
        let truth = serde_json::to_value(&p[0].truth).unwrap();
        let evidence = serde_json::to_value(&p[0].evidence).unwrap();
        let mut variations = HashSet::new();
        for seed in 1..100 {
            let c = p[0].instantiate(seed);
            c.validate().unwrap();
            assert_eq!(serde_json::to_value(&c.truth).unwrap(), truth);
            assert_eq!(serde_json::to_value(&c.evidence).unwrap(), evidence);
            assert_eq!(
                c.character.variation,
                p[0].instantiate(seed).character.variation
            );
            variations.insert(serde_json::to_string(&c.character.variation).unwrap());
        }
        assert!(variations.len() >= 20);
    }
    #[test]
    fn incompatible_combinations_documents_and_duplicate_names_are_rejected() {
        let p = pack();
        let mut bad = p[0].clone();
        bad.metadata.as_mut().unwrap().reason = "commercial_delivery".into();
        assert!(bad.validate().is_err());
        let mut bad = p[0].clone();
        bad.metadata.as_mut().unwrap().document_cargo = "Invented goods".into();
        assert!(bad.validate().is_err());
        let mut bad = p.clone();
        bad[1].character.name = bad[0].character.name.clone();
        bad[1].metadata.as_mut().unwrap().document_holder = bad[0].character.name.clone();
        assert!(
            validate_pack(&bad)
                .unwrap_err()
                .to_string()
                .contains("Duplicate traveler name")
        );
        let mut bad = p[0].clone();
        bad.evidence[1].location = EvidenceLocation::Documents;
        assert!(bad.validate().is_err());
    }
}
