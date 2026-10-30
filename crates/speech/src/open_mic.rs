//! Continuous local capture with phrase detection. Silent audio never reaches STT.
use super::*;
#[cfg(any(target_os = "linux", test))]
use std::collections::VecDeque;
#[cfg(target_os = "linux")]
use std::io::Read;
use std::sync::{Arc, Mutex, atomic::AtomicBool};

#[cfg(any(target_os = "linux", test))]
const SAMPLE_RATE: usize = 16_000;
#[cfg(any(target_os = "linux", test))]
const FRAME_SAMPLES: usize = 480; // 30ms at 16kHz
#[cfg(any(target_os = "linux", test))]
const END_SILENCE_FRAMES: usize = 50; // 1.5 seconds of silence closes a phrase
#[cfg(any(target_os = "linux", test))]
const MIN_VOICE_FRAMES: usize = 6; // reject clicks and short noise bursts
#[cfg(any(target_os = "linux", test))]
const MAX_PHRASE_SAMPLES: usize = SAMPLE_RATE * 20;

pub enum VoiceEvent {
    Listening,
    Activity(bool),
    Transcribing,
    Transcript(String),
    Error(String),
}
/// Owns the recorder. Dropping it closes capture immediately; workers terminate off-thread.
pub struct OpenMicrophone {
    child: Mutex<Child>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}
impl OpenMicrophone {
    #[cfg(target_os = "linux")]
    pub fn start(
        device: &str,
        threshold: f32,
        emit: impl Fn(VoiceEvent) + Send + 'static,
    ) -> Result<Self> {
        ensure!(
            std::env::var("BORDER_SPEECH_URL").is_ok() || std::env::var("WHISPER_MODEL").is_ok(),
            "Automatic voice needs a local speech model. Run scripts/setup-voice.sh, then restart the game."
        );
        let mut child = Command::new("arecord")
            .args([
                "-q", "-D", device, "-t", "raw", "-f", "S16_LE", "-r", "16000", "-c", "1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Microphone capture needs arecord (alsa-utils)")?;
        let mut audio = child.stdout.take().context("Microphone pipe unavailable")?;
        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(true));
        let (tx, rx) = std::sync::mpsc::sync_channel(2);
        let reader_stop = stop.clone();
        std::thread::spawn(move || {
            while !reader_stop.load(Ordering::Relaxed) {
                let mut bytes = vec![0; FRAME_SAMPLES * 2];
                if audio.read_exact(&mut bytes).is_err() {
                    break;
                }
                let frame: Vec<i16> = bytes
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]]))
                    .collect();
                // A tiny queue prevents old audio playing back into recognition after an AI reply.
                if let Err(std::sync::mpsc::TrySendError::Disconnected(_)) = tx.try_send(frame) {
                    break;
                }
            }
        });
        let worker_stop = stop.clone();
        let worker_pause = paused.clone();
        std::thread::spawn(move || {
            let mut segmenter = Segmenter::new(threshold);
            let mut active = false;
            emit(VoiceEvent::Listening);
            while !worker_stop.load(Ordering::Relaxed) {
                let frame = match rx.recv_timeout(Duration::from_millis(150)) {
                    Ok(frame) => frame,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(_) => {
                        if !worker_stop.load(Ordering::Relaxed) {
                            emit(VoiceEvent::Error("Microphone capture stopped. Check the selected input device and click Retry microphone.".into()));
                        }
                        break;
                    }
                };
                if worker_pause.load(Ordering::Relaxed) {
                    segmenter.reset();
                    if active {
                        active = false;
                        emit(VoiceEvent::Activity(false));
                    }
                    continue;
                }
                let phrase = segmenter.feed(&frame);
                let now_active = segmenter.is_active();
                if now_active != active {
                    active = now_active;
                    emit(VoiceEvent::Activity(active));
                }
                if let Some(samples) = phrase {
                    worker_pause.store(true, Ordering::Relaxed);
                    emit(VoiceEvent::Transcribing);
                    let result = transcribe_samples(&samples);
                    if worker_stop.load(Ordering::Relaxed) {
                        break;
                    }
                    match result {
                        Ok(text) if speech_text(&text) => emit(VoiceEvent::Transcript(text)),
                        Ok(_) => emit(VoiceEvent::Listening),
                        Err(e) => {
                            emit(VoiceEvent::Error(e.to_string()));
                            break;
                        }
                    }
                }
            }
        });
        Ok(Self {
            child: Mutex::new(child),
            stop,
            paused,
        })
    }
    #[cfg(not(target_os = "linux"))]
    pub fn start(
        _device: &str,
        _threshold: f32,
        _emit: impl Fn(VoiceEvent) + Send + 'static,
    ) -> Result<Self> {
        bail!(
            "Automatic microphone capture currently supports Linux. Typed questions remain available."
        )
    }
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }
}
impl Drop for OpenMicrophone {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.paused.store(true, Ordering::Relaxed);
        if let Ok(child) = self.child.get_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// RMS gate and silence segmentation, independent of microphone hardware.
#[cfg(any(target_os = "linux", test))]
struct Segmenter {
    threshold: f32,
    pre_roll: VecDeque<Vec<i16>>,
    samples: Vec<i16>,
    voice_frames: usize,
    silence_frames: usize,
    elapsed_samples: usize,
}
#[cfg(any(target_os = "linux", test))]
impl Segmenter {
    fn new(threshold: f32) -> Self {
        Self {
            threshold: threshold.clamp(0.001, 0.25),
            pre_roll: VecDeque::new(),
            samples: vec![],
            voice_frames: 0,
            silence_frames: 0,
            elapsed_samples: 0,
        }
    }
    fn reset(&mut self) {
        self.pre_roll.clear();
        self.samples.clear();
        self.voice_frames = 0;
        self.silence_frames = 0;
        self.elapsed_samples = 0;
    }
    fn is_active(&self) -> bool {
        self.voice_frames > 0
    }
    fn feed(&mut self, frame: &[i16]) -> Option<Vec<i16>> {
        let energy = frame
            .iter()
            .map(|s| (*s as f64 / 32768.).powi(2))
            .sum::<f64>()
            / frame.len().max(1) as f64;
        let voiced = energy.sqrt() as f32 >= self.threshold;
        if voiced {
            if self.voice_frames == 0 {
                for prior in self.pre_roll.drain(..) {
                    self.samples.extend(prior);
                }
            }
            self.voice_frames += 1;
            self.silence_frames = 0;
        } else if self.is_active() {
            self.silence_frames += 1;
        }
        if self.is_active() {
            self.elapsed_samples += frame.len();
            self.samples.extend_from_slice(frame);
        } else {
            self.pre_roll.push_back(frame.to_vec());
            if self.pre_roll.len() > 6 {
                self.pre_roll.pop_front();
            }
        }
        if self.silence_frames >= END_SILENCE_FRAMES || self.elapsed_samples >= MAX_PHRASE_SAMPLES {
            let accepted = self.voice_frames >= MIN_VOICE_FRAMES;
            let samples = std::mem::take(&mut self.samples);
            self.reset();
            return accepted.then_some(samples);
        }
        None
    }
}
#[cfg(any(target_os = "linux", test))]
fn speech_text(text: &str) -> bool {
    let text = text.trim().to_lowercase();
    !text.is_empty()
        && !matches!(
            text.as_str(),
            "[blank_audio]" | "[silence]" | "[music]" | "(silence)" | "[inaudible]"
        )
}
#[cfg(any(target_os = "linux", test))]
fn transcribe_samples(samples: &[i16]) -> Result<String> {
    let path = temp("phrase", "wav");
    let result = (|| {
        write_wav(&path, samples)?;
        transcribe(&path)
    })();
    let _ = std::fs::remove_file(path);
    result
}
#[cfg(any(target_os = "linux", test))]
fn write_wav(path: &Path, samples: &[i16]) -> Result<()> {
    let bytes = (samples.len() * 2) as u32;
    let mut f = std::fs::File::create(path)?;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + bytes).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&(SAMPLE_RATE as u32).to_le_bytes())?;
    f.write_all(&((SAMPLE_RATE * 2) as u32).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&bytes.to_le_bytes())?;
    for sample in samples {
        f.write_all(&sample.to_le_bytes())?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires WHISPER_BIN, WHISPER_MODEL and BORDER_SPEECH_TEST_PCM"]
    fn local_whisper_transcribes_an_automatically_detected_phrase() {
        let bytes = std::fs::read(std::env::var("BORDER_SPEECH_TEST_PCM").unwrap()).unwrap();
        let mut samples: Vec<i16> = bytes
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        samples.extend(vec![0; SAMPLE_RATE * 2]);
        let mut gate = Segmenter::new(0.015);
        let phrases: Vec<_> = samples
            .chunks(FRAME_SAMPLES)
            .filter_map(|f| gate.feed(f))
            .collect();
        assert_eq!(phrases.len(), 1);
        let transcript = transcribe_samples(&phrases[0]).unwrap().to_lowercase();
        assert!(
            transcript.contains("purpose") && transcript.contains("visit"),
            "{transcript}"
        );
    }
    #[test]
    fn silence_and_clicks_do_not_trigger_transcription() {
        let mut gate = Segmenter::new(0.015);
        for _ in 0..100 {
            assert!(gate.feed(&[0; FRAME_SAMPLES]).is_none());
        }
        gate.feed(&[2000; FRAME_SAMPLES]);
        for _ in 0..30 {
            assert!(gate.feed(&[0; FRAME_SAMPLES]).is_none());
        }
    }
    #[test]
    fn phrase_closes_after_silence_and_does_not_repeat() {
        let mut gate = Segmenter::new(0.015);
        for _ in 0..8 {
            assert!(gate.feed(&[2000; FRAME_SAMPLES]).is_none());
        }
        for _ in 0..END_SILENCE_FRAMES - 1 {
            assert!(gate.feed(&[0; FRAME_SAMPLES]).is_none());
        }
        assert!(gate.feed(&[0; FRAME_SAMPLES]).is_some());
        for _ in 0..50 {
            assert!(gate.feed(&[0; FRAME_SAMPLES]).is_none());
        }
    }
    #[test]
    fn conversational_pause_does_not_cut_off_the_next_clause() {
        let mut gate = Segmenter::new(0.015);
        for _ in 0..10 {
            assert!(gate.feed(&[2000; FRAME_SAMPLES]).is_none());
        }
        for _ in 0..40 {
            assert!(gate.feed(&[0; FRAME_SAMPLES]).is_none());
        } // 1.2s pause
        for _ in 0..10 {
            assert!(gate.feed(&[2000; FRAME_SAMPLES]).is_none());
        }
        for _ in 0..49 {
            assert!(gate.feed(&[0; FRAME_SAMPLES]).is_none());
        }
        let phrase = gate.feed(&[0; FRAME_SAMPLES]).unwrap();
        assert_eq!(
            phrase.iter().filter(|s| **s == 2000).count(),
            20 * FRAME_SAMPLES
        );
    }
    #[test]
    fn continuous_speech_forces_submission_at_twenty_seconds() {
        let mut gate = Segmenter::new(0.015);
        for _ in 0..666 {
            assert!(gate.feed(&[2000; FRAME_SAMPLES]).is_none());
        }
        let phrase = gate.feed(&[2000; FRAME_SAMPLES]).unwrap();
        assert!(phrase.len() >= SAMPLE_RATE * 20);
        assert!(phrase.len() < SAMPLE_RATE * 20 + FRAME_SAMPLES);
    }
    #[test]
    fn reset_discards_paused_audio_and_long_phrases_are_bounded() {
        let mut gate = Segmenter::new(0.015);
        gate.feed(&[2000; FRAME_SAMPLES]);
        gate.reset();
        for _ in 0..30 {
            assert!(gate.feed(&[0; FRAME_SAMPLES]).is_none());
        }
        let mut chunks = 0;
        for _ in 0..1400 {
            if let Some(samples) = gate.feed(&[2000; FRAME_SAMPLES]) {
                assert!(samples.len() <= MAX_PHRASE_SAMPLES + FRAME_SAMPLES * 7);
                chunks += 1;
            }
        }
        assert_eq!(chunks, 2);
    }
    #[test]
    fn wav_header_and_silence_tokens_are_valid() {
        let path = temp("test", "wav");
        write_wav(&path, &[1, -1, 0]).unwrap();
        let wav = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(path);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(wav.len(), 50);
        assert_eq!(&wav[44..], &[1, 0, 255, 255, 0, 0]);
        assert!(!speech_text("[BLANK_AUDIO]"));
        assert!(speech_text("What is your purpose?"));
    }
}
