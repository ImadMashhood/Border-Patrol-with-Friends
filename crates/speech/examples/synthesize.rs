fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("Usage: synthesize output.wav scenario_id text"))?;
    let case = args.next().unwrap_or_else(|| "09_cal".into());
    let text = args.collect::<Vec<_>>().join(" ");
    anyhow::ensure!(!text.is_empty(), "Speech text is required");
    border_speech::synthesize_to_file(
        &text,
        std::path::Path::new(&path),
        border_speech::alternate_voice(&case),
    )?;
    Ok(())
}
