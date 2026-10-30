//! Local external-tool adapters. No speech/cloud key reaches the game.
use anyhow::{Context, Result, bail, ensure};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
mod open_mic;
pub use open_mic::{OpenMicrophone, VoiceEvent};

static COUNTER: AtomicU64 = AtomicU64::new(0);
fn temp(stem: &str, ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "border-{}-{}-{stem}.{ext}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}
fn wait_success(mut child: Child, limit: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            ensure!(status.success(), "Speech tool failed (exit {status})");
            return Ok(());
        }
        if start.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Speech tool timed out");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
pub fn transcribe(audio: &Path) -> Result<String> {
    let executable = std::env::var("WHISPER_BIN").unwrap_or_else(|_| "whisper-cli".into());
    let model=std::env::var("WHISPER_MODEL").context("Voice input needs WHISPER_MODEL pointing to a local whisper.cpp ggml model; typed input is available")?;
    let output = temp("transcript", "out");
    let txt = output.with_extension("out.txt");
    let result = (|| {
        let child = Command::new(executable)
            .args(["-m", &model, "-f"])
            .arg(audio)
            .args(["-otxt", "-of"])
            .arg(&output)
            .args(["-nt", "-l", "en", "-t", "4"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Could not start whisper-cli")?;
        wait_success(child, Duration::from_secs(30))?;
        let text = std::fs::read_to_string(&txt)?.trim().to_string();
        ensure!(!text.is_empty(), "No speech recognized. Try again or type.");
        Ok(text.chars().take(500).collect())
    })();
    let _ = std::fs::remove_file(txt);
    result
}
/// Authored voice assignment, independent of gameplay evidence or guilt.
pub fn alternate_voice(scenario_id: &str) -> bool {
    matches!(
        scenario_id,
        "01_mara" | "02_nadia" | "04_evelyn" | "08_rosa" | "10_priya"
    )
}
fn voice_model(alternate: bool) -> Result<String> {
    if alternate && let Ok(model) = std::env::var("PIPER_MODEL_ALTERNATE") {
        return Ok(model);
    }
    std::env::var("PIPER_MODEL")
        .context("Neural speech needs Piper voices. Run scripts/setup-tts.sh and restart the game.")
}
/// Also used by the preview utility; synthesis never opens an audio device.
pub fn synthesize_to_file(text: &str, path: &Path, alternate: bool) -> Result<()> {
    let model = voice_model(alternate)?;
    let clean = text.replace(['*', '_', '`'], "");
    let mut child = Command::new(std::env::var("PIPER_BIN").unwrap_or_else(|_| "piper".into()))
        .args(["--model", &model, "--output_file"])
        .arg(path)
        .args(["--sentence-silence", "0.18", "--length-scale", "1.02"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Could not start neural speech. Run scripts/setup-tts.sh.")?;
    child
        .stdin
        .take()
        .context("Piper input unavailable")?
        .write_all(clean.as_bytes())?;
    wait_success(child, Duration::from_secs(20))?;
    ensure!(
        std::fs::metadata(path).is_ok_and(|m| m.len() > 44),
        "Piper produced no audio"
    );
    Ok(())
}
/// Runs on a worker thread. Microphone stays paused until synthesis and playback finish.
pub fn speak(text: &str, scenario_id: &str) -> Result<()> {
    let path = temp("speech", "wav");
    let result = (|| {
        synthesize_to_file(text, &path, alternate_voice(scenario_id))?;
        let player = Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-loglevel", "quiet"])
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Neural voice playback needs ffplay")?;
        wait_success(player, Duration::from_secs(40))
    })();
    let _ = std::fs::remove_file(path);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn temporary_paths_are_unique() {
        assert_ne!(temp("test", "wav"), temp("test", "wav"));
    }
}
