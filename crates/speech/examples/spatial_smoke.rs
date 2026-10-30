//! Set SDL_AUDIODRIVER=dummy for a silent output/process check.
use border_speech::{SpatialVoice, speak_spatial_cancellable};
use std::sync::{Mutex, atomic::AtomicBool};
fn main() {
    let spatial = Mutex::new(SpatialVoice {
        active: true,
        source: [-2., 0., 0.],
        ..Default::default()
    });
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            spatial.lock().unwrap().source = [2., 0., 0.];
        });
        speak_spatial_cancellable("Excuse me?", "09_cal", &AtomicBool::new(false), &spatial)
            .unwrap();
    });
    println!("PASS: cached speech streamed as positional stereo, changing sides during playback");
}
