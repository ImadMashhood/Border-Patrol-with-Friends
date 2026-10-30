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
impl Scenario {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema_version == 1, "Unsupported scenario schema");
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
            .filter(|e| secondary || e.reveal_condition == RevealCondition::Documents)
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
            Decision::Deny => self.truth.smuggling,
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
    for s in pack {
        s.validate()?;
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
}
