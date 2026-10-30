fn main() -> anyhow::Result<()> {
    let file = std::env::args().nth(1).ok_or_else(|| {
        anyhow::anyhow!("Usage: cargo run -p border-speech --example transcribe -- recording.wav")
    })?;
    println!(
        "{}",
        border_speech::transcribe(std::path::Path::new(&file))?
    );
    Ok(())
}
