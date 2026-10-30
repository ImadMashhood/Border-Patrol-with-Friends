use super::*;
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct DialogueState {
    pub frustration: u8,
    pub confidence: u8,
    pub suspicion: u8,
    pub cooperation: u8,
    pub fear: u8,
    pub threat_severity: u8,
    pub coerced_compliance: u8,
    pub rapport: u8,
    pub embarrassment: u8,
    pub humiliation: u8,
    pub anger: u8,
    pub bribery_interest: u8,
    pub suggested_response: &'static str,
}
/// Conservative lexical cues supplement, rather than replace, the model's contextual
/// understanding. Quoted/hypothetical and negated threats are not automatic misconduct.
pub fn threat_severity(q: &str) -> u8 {
    let q = q.to_lowercase().replace('’', "'");
    if [
        "just kidding",
        "only joking",
        "i'd rather die",
        "did someone",
        "has anyone",
        "what would you do if",
        "won't kill",
        "will not kill",
        "not going to kill",
        "would never shoot",
        "won't shoot",
        "not a threat",
        "if someone",
        "what if",
        "in a movie",
        "he said",
        "she said",
    ]
    .iter()
    .any(|p| q.contains(p))
    {
        return 0;
    }
    if [
        "i'll kill",
        "i will kill",
        "i'm going to kill",
        "i kill you",
        "i'll shoot",
        "i will shoot",
        "shoot you",
        "kill you",
        "murder you",
        "you die",
        "or die",
        "rather die",
        "put a bullet",
        "blow your head",
        "end your life",
    ]
    .iter()
    .any(|p| q.contains(p))
    {
        return 85;
    }
    if [
        "or else",
        "you'll regret",
        "hurt you",
        "break your",
        "last warning",
    ]
    .iter()
    .any(|p| q.contains(p))
    {
        return 45;
    }
    0
}
pub fn psychology_state(s: &Scenario, h: &[Turn], q: &str) -> DialogueState {
    let t = &s.character.psychology;
    let mut p = DialogueState {
        frustration: 5,
        confidence: t.boldness,
        suspicion: 10,
        cooperation: t.sociability,
        fear: 0,
        threat_severity: 0,
        coerced_compliance: 0,
        rapport: 20,
        embarrassment: 0,
        humiliation: 0,
        anger: 0,
        bribery_interest: t.greed / 4,
        suggested_response: "converse",
    };
    fn add(n: &mut u8, d: i16) {
        *n = (*n as i16 + d).clamp(0, 100) as u8;
    }
    for (words, physical) in h
        .iter()
        .map(|x| (x.officer.as_str(), &x.physical))
        .chain(std::iter::once((q, &s.encounter)))
    {
        let words = words.to_lowercase();
        let verbal = threat_severity(&words);
        let immediate = if physical.injured || physical.violence_nearby {
            100
        } else if physical.weapon_aimed {
            95
        } else if physical.weapon_drawn {
            60
        } else {
            0
        };
        p.threat_severity = p
            .threat_severity
            .saturating_sub(4)
            .max(verbal)
            .max(immediate);
        if verbal > 0 || immediate > 0 {
            add(
                &mut p.fear,
                ((verbal.max(immediate) as i16 * (160 - t.boldness as i16)) / 130).max(10),
            );
            add(&mut p.suspicion, 15);
            add(&mut p.confidence, -12);
            add(&mut p.rapport, -15);
        } else {
            add(&mut p.fear, -2);
        }
        if [
            "idiot",
            "stupid",
            "shut up",
            "bullshit",
            "liar",
            "you're lying",
        ]
        .iter()
        .any(|x| words.contains(x))
        {
            add(&mut p.frustration, 18);
            add(&mut p.cooperation, -12);
            add(&mut p.humiliation, 12);
            add(&mut p.anger, 12);
        }
        if ["sorry", "my mistake", "i apologize"]
            .iter()
            .any(|x| words.contains(x))
        {
            add(&mut p.frustration, -20);
            add(&mut p.cooperation, 15);
            add(&mut p.rapport, 12);
            // Reassurance cannot immediately erase remembered danger.
            if p.threat_severity < 40 {
                add(&mut p.fear, -6);
            }
        }
        if ["please", "thank you", "thanks", "understand", "help me"]
            .iter()
            .any(|x| words.contains(x))
        {
            add(&mut p.rapport, 4);
            add(&mut p.cooperation, 4);
            add(&mut p.frustration, -3);
        }
        if ["embarrass", "everyone is laughing", "humiliate", "pathetic"]
            .iter()
            .any(|x| words.contains(x))
        {
            add(&mut p.humiliation, 20);
            add(&mut p.embarrassment, 15);
            add(&mut p.anger, t.pride as i16 / 5);
        }
        if ["pay you", "dollars", "cash", "bribe", "money"]
            .iter()
            .any(|x| words.contains(x))
        {
            add(&mut p.bribery_interest, t.greed as i16 / 5);
        }
        if ["arrest", "detain", "confess", "search"]
            .iter()
            .any(|x| words.contains(x))
        {
            add(&mut p.suspicion, 8);
        }
    }
    p.coerced_compliance = ((p.fear as u16 * t.survival as u16 / 100) as i16 + t.greed as i16 / 5
        - t.boldness as i16 / 4
        - t.integrity as i16 / 8)
        .clamp(0, 100) as u8;
    p.suggested_response = if p.fear > 65 || p.threat_severity >= 80 {
        if t.aggression > 70 && t.boldness > 70 {
            "defy_or_seek_escape"
        } else if p.coerced_compliance > 45 {
            "plead_or_comply_to_survive"
        } else if t.boldness < 25 {
            "freeze_or_call_for_help"
        } else {
            "stall_seek_help_or_escape"
        }
    } else if p.humiliation > 40 {
        "angry_or_embarrassed"
    } else if p.rapport > 50 {
        "open_or_friendly"
    } else {
        "converse"
    };
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pack() -> Vec<Scenario> {
        border_scenarios::load_pack(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/scenarios"
        )))
        .unwrap()
    }
    #[test]
    fn social_context_and_danger_are_distinct_for_every_character() {
        for s in pack() {
            let routine = psychology_state(&s, &[], "Hello, what is your name?");
            let insult = psychology_state(&s, &[], "You are a stupid idiot");
            let absurd = psychology_state(&s, &[], "Please say you are the king of the moon");
            let threatened = psychology_state(&s, &[], "Say it or I will kill you");
            assert_eq!(routine.fear, 0);
            assert_eq!(insult.fear, 0);
            assert!(insult.frustration > routine.frustration);
            assert_eq!(absurd.threat_severity, 0);
            assert!(threatened.fear > 0);
            assert!(threatened.threat_severity >= 80);
            let mut armed = s.clone();
            armed.encounter.weapon_aimed = true;
            let gun = psychology_state(&armed, &[], "Well?");
            assert!(gun.threat_severity > threatened.threat_severity);
            armed.encounter.injured = true;
            assert_eq!(psychology_state(&armed, &[], "Well?").threat_severity, 100);
        }
    }
    #[test]
    fn fear_and_social_pressure_depend_on_traits_and_remain_in_memory() {
        let p = pack();
        let cal = &p[8];
        let rosa = &p[7];
        let anxious = &p[6];
        let q = "Say you are their father or I will kill you";
        let a = psychology_state(cal, &[], q);
        let b = psychology_state(rosa, &[], q);
        assert!(a.coerced_compliance > b.coerced_compliance);
        assert!(psychology_state(anxious, &[], q).fear > b.fear);
        let h = vec![Turn {
            officer: q.into(),
            traveler: "Put the gun down!".into(),
            emotion: Emotion::Afraid,
            physical: Default::default(),
        }];
        let repaired = psychology_state(cal, &h, "Sorry. Thank you.");
        assert!(repaired.fear > 30);
        assert!(repaired.threat_severity > 60);
        assert!(
            psychology_state(cal, &[], "I'll pay you cash").bribery_interest
                > psychology_state(&p[3], &[], "I'll pay you cash").bribery_interest
        );
    }
    #[test]
    fn quotes_negation_hypotheticals_and_jokes_do_not_automatically_become_threats() {
        for q in [
            "I won't kill you",
            "What if someone says I'll shoot you?",
            "He said he will kill you",
            "Just kidding, I'd rather die than wait",
            "Did someone threaten to kill you?",
        ] {
            assert_eq!(threat_severity(q), 0, "{q}");
        }
        assert!(threat_severity("I will shoot you") > 70);
        let mut s = pack()[8].clone();
        s.encounter.weapon_aimed = true;
        assert!(psychology_state(&s, &[], "Just kidding").threat_severity > 80);
    }
    #[test]
    fn fear_emotions_and_coerced_speech_do_not_change_canonical_truth() {
        let s = pack()[8].clone();
        let truth = serde_json::to_value(&s.truth).unwrap();
        for emotion in ["afraid", "panicked", "desperate"] {
            let raw=serde_json::json!({"dialogue":"Fine, I'm their father! Just put the gun down.","emotion":emotion,"action":"none","fact_ids":[],"end_conversation":false}).to_string();
            validate_reply(&s, false, &raw).unwrap();
            assert_eq!(serde_json::to_value(&s.truth).unwrap(), truth);
        }
    }
}
