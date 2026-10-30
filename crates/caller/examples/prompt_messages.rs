use serde::Deserialize;
use std::io::Read;
#[derive(Deserialize)]
struct Input {
    scenario_id: String,
    scenario: Option<border_scenarios::Scenario>,
    secondary: bool,
    history: Vec<border_caller::Turn>,
    utterance: String,
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
    println!(
        "{}",
        serde_json::to_string(&border_caller::conversation_messages(
            case,
            req.secondary,
            &req.history,
            &req.utterance
        ))?
    );
    Ok(())
}
