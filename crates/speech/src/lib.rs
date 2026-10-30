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
/// Runs on a worker thread. Piper is preferred when configured; espeak-ng is a lightweight fallback.
pub fn speak(text: &str) -> Result<()> {
    if let Ok(model) = std::env::var("PIPER_MODEL") {
        let path = temp("speech", "wav");
        let result = (|| {
            let mut child =
                Command::new(std::env::var("PIPER_BIN").unwrap_or_else(|_| "piper".into()))
                    .args(["--model", &model, "--output_file"])
                    .arg(&path)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()?;
            child
                .stdin
                .take()
                .context("Piper input unavailable")?
                .write_all(text.as_bytes())?;
            wait_success(child, Duration::from_secs(20))?;
            let player = Command::new("ffplay")
                .args(["-nodisp", "-autoexit", "-loglevel", "quiet"])
                .arg(&path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .context("Piper playback needs ffplay")?;
            wait_success(player, Duration::from_secs(40))
        })();
        let _ = std::fs::remove_file(path);
        result
    } else {
        let child = Command::new("espeak-ng")
            .args(["-s", "165", "--", text])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Install espeak-ng or configure Piper for spoken replies")?;
        wait_success(child, Duration::from_secs(40))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn temporary_paths_are_unique() {
        assert_ne!(temp("test", "wav"), temp("test", "wav"));
    }
}
