use serde::Deserialize;
use std::io::Read;
#[derive(Deserialize)]
struct Input {
    scenario_id: String,
    scenario: Option<border_scenarios::Scenario>,
    secondary: bool,
    history: Vec<border_caller::Turn>,
    utterance: String,
    raw_reply: Option<String>,
}
fn main() -> anyhow::Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let req: Input = serde_json::from_str(&input)?;
    let pack = border_scenarios::load_pack(std::path::Path::new("assets/scenarios"))?;
    let supplied = req.scenario.as_ref();
    let case = supplied
        .or_else(|| pack.iter().find(|s| s.scenario_id == req.scenario_id))
        .ok_or_else(|| anyhow::anyhow!("Unknown case"))?;
    let output = if std::env::args().any(|a| a == "--validate") {
        serde_json::to_value(border_caller::validate_turn_reply(
            case,
            req.secondary,
            &req.history,
            &req.utterance,
            req.raw_reply
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Missing reply"))?,
        )?)?
    } else if std::env::args().any(|a| a == "--request" || a == "--repair") {
        serde_json::json!({"messages": if std::env::args().any(|a| a == "--repair") {
            border_caller::repair_messages(case, req.secondary, &req.history, &req.utterance)
        } else { border_caller::conversation_messages(case, req.secondary, &req.history, &req.utterance) },
        "response_format": border_caller::response_format(case, req.secondary)})
    } else {
        serde_json::to_value(border_caller::conversation_messages(
            case,
            req.secondary,
            &req.history,
            &req.utterance,
        ))?
    };
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
