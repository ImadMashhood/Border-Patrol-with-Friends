//! Low-latency stereo placement over the existing ffplay output adapter.
//! Mono speech is panned at the current rendered face, not at the screen/camera.
use super::*;
use std::sync::Mutex;
#[derive(Clone, Copy, Debug)]
pub struct SpatialVoice {
    pub source: [f32; 3],
    pub listener: [f32; 3],
    pub right: [f32; 3],
    pub active: bool,
    pub energy: f32,
}
impl Default for SpatialVoice {
    fn default() -> Self {
        Self {
            source: [0.; 3],
            listener: [0.; 3],
            right: [1., 0., 0.],
            active: false,
            energy: 0.,
        }
    }
}
impl SpatialVoice {
    pub fn gains(self) -> [f32; 2] {
        if !self.active {
            return [0.; 2];
        }
        let delta: [f32; 3] = std::array::from_fn(|i| self.source[i] - self.listener[i]);
        let distance = delta.iter().map(|v| v * v).sum::<f32>().sqrt();
        if !distance.is_finite() || distance >= 18. {
            return [0.; 2];
        }
        let pan = (delta
            .iter()
            .zip(self.right)
            .map(|(d, r)| d * r)
            .sum::<f32>()
            / distance.max(0.01))
        .clamp(-1., 1.);
        // Smooth rolloff, full near-field loudness, and a fade to silence at 18m.
        let volume = (1. / (1. + (distance / 5.).powi(2))) * ((18. - distance) / 3.).clamp(0., 1.);
        let angle = (pan + 1.) * std::f32::consts::FRAC_PI_4;
        [angle.cos() * volume, angle.sin() * volume]
    }
}
fn mono_pcm(bytes: &[u8]) -> Result<(u32, Vec<i16>)> {
    ensure!(
        bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        "Invalid voice WAV"
    );
    let mut format = None;
    let mut data = None;
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let length = u32::from_le_bytes(bytes[i + 4..i + 8].try_into()?) as usize;
        let start = i + 8;
        let end = start.checked_add(length).context("Invalid WAV chunk")?;
        ensure!(end <= bytes.len(), "Truncated voice WAV");
        if &bytes[i..i + 4] == b"fmt " {
            ensure!(length >= 16, "Missing WAV format");
            let u16_at = |n| u16::from_le_bytes([bytes[start + n], bytes[start + n + 1]]);
            format = Some((
                u16_at(0),
                u16_at(2),
                u32::from_le_bytes(bytes[start + 4..start + 8].try_into()?),
                u16_at(14),
            ));
        } else if &bytes[i..i + 4] == b"data" {
            data = Some(&bytes[start..end]);
        }
        i = end + (length % 2);
    }
    let (format, channels, rate, bits) = format.context("Missing WAV format")?;
    let data = data.context("Missing WAV samples")?;
    ensure!(
        (1..=2).contains(&channels)
            && (8000..=96000).contains(&rate)
            && ((format == 1 && bits == 16) || (format == 3 && bits == 32)),
        "Voice output needs PCM16 or float32 WAV"
    );
    let sample_bytes = (bits / 8) as usize;
    let frame = sample_bytes * channels as usize;
    ensure!(data.len() % frame == 0, "Incomplete WAV frame");
    let samples = data
        .chunks_exact(frame)
        .map(|chunk| {
            let sum = chunk
                .chunks_exact(sample_bytes)
                .map(|sample| {
                    if format == 1 {
                        i16::from_le_bytes([sample[0], sample[1]]) as f32
                    } else {
                        f32::from_le_bytes(sample.try_into().unwrap()).clamp(-1., 1.) * 32767.
                    }
                })
                .sum::<f32>();
            (sum / channels as f32).clamp(-32768., 32767.) as i16
        })
        .collect();
    Ok((rate, samples))
}
pub fn speak_spatial_cancellable(
    text: &str,
    scenario: &str,
    cancel: &AtomicBool,
    spatial: &Mutex<SpatialVoice>,
) -> Result<()> {
    speak_spatial_expressive(text, scenario, "cooperative", cancel, spatial)
}
pub fn speak_spatial_expressive(
    text: &str,
    scenario: &str,
    emotion: &str,
    cancel: &AtomicBool,
    spatial: &Mutex<SpatialVoice>,
) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        return Ok(());
    }
    let path = temp("spatial-voice", "wav");
    let result = (|| {
        voice_file(text, &path, scenario, emotion)?;
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let (rate, pcm) = mono_pcm(&std::fs::read(&path)?)?;
        let mut player = Command::new("ffplay")
            .args([
                "-nodisp",
                "-autoexit",
                "-loglevel",
                "quiet",
                "-f",
                "wav",
                "-probesize",
                "32",
                "-analyzeduration",
                "0",
                "-i",
                "pipe:0",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Voice playback needs ffplay")?;
        let streamed = (|| -> Result<()> {
            let mut input = player.stdin.take().context("Voice output pipe missing")?;
            // A WAV header carries rate/channels across FFmpeg versions (recent ffplay
            // releases removed the older raw-input -ac option).
            let data_length = (pcm.len() * 4) as u32;
            let mut header = Vec::with_capacity(44);
            header.extend_from_slice(b"RIFF");
            header.extend_from_slice(&(36 + data_length).to_le_bytes());
            header.extend_from_slice(b"WAVEfmt ");
            header.extend_from_slice(&16u32.to_le_bytes());
            header.extend_from_slice(&1u16.to_le_bytes());
            header.extend_from_slice(&2u16.to_le_bytes());
            header.extend_from_slice(&rate.to_le_bytes());
            header.extend_from_slice(&(rate * 4).to_le_bytes());
            header.extend_from_slice(&4u16.to_le_bytes());
            header.extend_from_slice(&16u16.to_le_bytes());
            header.extend_from_slice(b"data");
            header.extend_from_slice(&data_length.to_le_bytes());
            input.write_all(&header)?;
            let chunk = (rate / 100) as usize; // 10ms; never queue the whole line with a stale position.
            let started = Instant::now();
            let mut previous = spatial.lock().unwrap().gains();
            let mut frames = 0;
            for samples in pcm.chunks(chunk) {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(());
                }
                let next = {
                    let mut voice = spatial.lock().unwrap();
                    voice.energy = (samples
                        .iter()
                        .map(|s| (*s as f32 / 32768.).powi(2))
                        .sum::<f32>()
                        / samples.len() as f32)
                        .sqrt()
                        .mul_add(6., 0.)
                        .clamp(0., 1.);
                    voice.gains()
                };
                let mut stereo = Vec::with_capacity(samples.len() * 4);
                for (i, &sample) in samples.iter().enumerate() {
                    let mix = (i + 1) as f32 / samples.len() as f32;
                    for channel in 0..2 {
                        let gain = previous[channel] + (next[channel] - previous[channel]) * mix;
                        stereo.extend_from_slice(&((sample as f32 * gain) as i16).to_le_bytes());
                    }
                }
                input.write_all(&stereo)?;
                previous = next;
                frames += samples.len();
                let deadline = Duration::from_secs_f64(frames as f64 / rate as f64);
                if let Some(remaining) = deadline.checked_sub(started.elapsed()) {
                    std::thread::sleep(remaining);
                }
            }
            drop(input);
            Ok(())
        })();
        if streamed.is_err() || cancel.load(Ordering::Relaxed) {
            let _ = player.kill();
            let _ = player.wait();
            return streamed;
        }
        let end = Instant::now();
        loop {
            if cancel.load(Ordering::Relaxed) || end.elapsed() > Duration::from_secs(4) {
                let _ = player.kill();
                let _ = player.wait();
                return Ok(());
            }
            if let Some(status) = player.try_wait()? {
                ensure!(status.success(), "Voice playback failed");
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    })();
    spatial.lock().unwrap().energy = 0.;
    let _ = std::fs::remove_file(path);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_face_and_listener_change_pan_and_distance_gain() {
        let mut s = SpatialVoice {
            active: true,
            source: [-2., 0., 0.],
            ..Default::default()
        };
        let left = s.gains();
        assert!(left[0] > left[1] * 10.);
        s.source = [2., 0., 0.];
        let right = s.gains();
        assert!(right[1] > right[0] * 10.);
        s.right = [-1., 0., 0.];
        assert!(s.gains()[0] > s.gains()[1] * 10.);
        s.right = [1., 0., 0.];
        s.source = [0., 0., -2.];
        let near = s.gains();
        assert!((near[0] - near[1]).abs() < 0.001);
        s.listener = [0., 0., 12.];
        assert!(s.gains()[0] < near[0] * 0.2);
        s.source = [0., 0., 50.];
        assert_eq!(s.gains(), [0.; 2]);
    }
    #[test]
    fn cached_voice_wavs_decode_and_truncation_is_rejected() {
        for bytes in [
            include_bytes!("../../../assets/audio/excuse-me.wav").as_slice(),
            include_bytes!("../../../assets/audio/excuse-me-alternate.wav").as_slice(),
        ] {
            let (rate, samples) = mono_pcm(bytes).unwrap();
            assert_eq!(rate, 24000);
            assert!(samples.len() > 24000);
            assert!(mono_pcm(&bytes[..bytes.len() - 2]).is_err());
        }
    }
}
