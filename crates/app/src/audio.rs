//! cpal playback for the pre-rendered loop.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use arc_swap::ArcSwapOption;
use contra_music::AudioBuffer;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioEngine {
    stream: Option<cpal::Stream>,
    shared: Arc<Shared>,
    sample_rate: u32,
}

struct Shared {
    buffer: ArcSwapOption<AudioBuffer>,
    position: AtomicU64,
    playing: AtomicBool,
    looping: AtomicBool,
    finished: AtomicBool,
    volume: AtomicU32,
    speech: ArcSwapOption<Vec<f32>>,
    speech_position: AtomicU64,
    speech_playing: AtomicBool,
}

impl AudioEngine {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "no audio output device found".to_string())?;
        let supported = device
            .default_output_config()
            .map_err(|error| format!("default audio config: {error}"))?;
        let sample_rate = supported.sample_rate();
        let config = supported.config();
        let shared = Arc::new(Shared::new());
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config, Arc::clone(&shared)),
            cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config, Arc::clone(&shared)),
            cpal::SampleFormat::U16 => build_stream::<u16>(&device, &config, Arc::clone(&shared)),
            cpal::SampleFormat::F64 => build_stream::<f64>(&device, &config, Arc::clone(&shared)),
            cpal::SampleFormat::I32 => build_stream::<i32>(&device, &config, Arc::clone(&shared)),
            other => Err(format!("unsupported audio sample format {other}")),
        }?;
        stream
            .play()
            .map_err(|error| format!("start audio stream: {error}"))?;
        Ok(Self {
            stream: Some(stream),
            shared,
            sample_rate,
        })
    }

    pub fn silent(sample_rate: u32) -> Self {
        Self {
            stream: None,
            shared: Arc::new(Shared::new()),
            sample_rate,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn set_buffer(&self, buffer: AudioBuffer) {
        self.shared.buffer.store(Some(Arc::new(buffer)));
        self.shared.position.store(0, Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
    }

    /// Mix a mono speech buffer into the next output frames. The buffer is
    /// already at the device sample rate. Replacing a playing buffer starts
    /// the new call immediately.
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    pub fn play_speech(&self, samples: Vec<f32>) {
        if samples.is_empty() {
            return;
        }
        self.shared.speech.store(Some(Arc::new(samples)));
        self.shared.speech_position.store(0, Ordering::Relaxed);
        self.shared.speech_playing.store(true, Ordering::Relaxed);
    }

    pub fn stop_speech(&self) {
        self.shared.speech_playing.store(false, Ordering::Relaxed);
        self.shared.speech_position.store(0, Ordering::Relaxed);
    }

    pub fn play(&self) {
        if self.shared.finished.load(Ordering::Relaxed) {
            self.shared.position.store(0, Ordering::Relaxed);
            self.shared.finished.store(false, Ordering::Relaxed);
        }
        self.shared.playing.store(true, Ordering::Relaxed);
        if let Some(stream) = &self.stream {
            let _ = stream.play();
        }
    }

    pub fn pause(&self) {
        self.shared.playing.store(false, Ordering::Relaxed);
        self.stop_speech();
    }

    pub fn stop(&self) {
        self.shared.playing.store(false, Ordering::Relaxed);
        self.shared.position.store(0, Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
        self.stop_speech();
    }

    pub fn restart(&self) {
        self.shared.position.store(0, Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
        self.stop_speech();
        self.play();
    }

    pub fn set_looping(&self, looping: bool) {
        self.shared.looping.store(looping, Ordering::Relaxed);
    }

    pub fn set_volume(&self, volume: f32) {
        self.shared
            .volume
            .store(volume.clamp(0.0, 2.0).to_bits(), Ordering::Relaxed);
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    pub fn is_finished(&self) -> bool {
        self.shared.finished.load(Ordering::Relaxed)
    }

    pub fn position_samples(&self) -> u64 {
        self.shared.position.load(Ordering::Relaxed)
    }

    pub fn position_beat(&self) -> f64 {
        let Some(buffer) = self.shared.buffer.load_full() else {
            return 0.0;
        };
        buffer.beat_at_sample(self.position_samples())
    }

    /// Move the transport to a musical beat without changing play state.
    ///
    /// Used when a re-render swaps the buffer: the caller keeps the beat from
    /// the old buffer so playback continues instead of restarting.
    pub fn seek_beat(&self, beat: f64) {
        let Some(buffer) = self.shared.buffer.load_full() else {
            return;
        };
        let last_sample = buffer.loop_end_sample.saturating_sub(1) as i64;
        let sample = buffer.sample_at_beat(beat).clamp(0, last_sample) as u64;
        self.shared.position.store(sample, Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
    }

    pub fn has_buffer(&self) -> bool {
        self.shared.buffer.load().is_some()
    }
}

impl Shared {
    fn new() -> Self {
        Self {
            buffer: ArcSwapOption::empty(),
            position: AtomicU64::new(0),
            playing: AtomicBool::new(false),
            looping: AtomicBool::new(true),
            finished: AtomicBool::new(false),
            volume: AtomicU32::new(1.0f32.to_bits()),
            speech: ArcSwapOption::empty(),
            speech_position: AtomicU64::new(0),
            speech_playing: AtomicBool::new(false),
        }
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    device
        .build_output_stream(
            *config,
            move |data: &mut [T], _| {
                write_data(data, channels, &shared);
            },
            move |error| {
                eprintln!("audio stream error: {error}");
            },
            None,
        )
        .map_err(|error| format!("build audio stream: {error}"))
}

fn write_data<T>(data: &mut [T], channels: usize, shared: &Shared)
where
    T: cpal::Sample + cpal::FromSample<f32>,
{
    let channels = channels.max(1);
    let volume = f32::from_bits(shared.volume.load(Ordering::Relaxed));
    let music_buffer = shared.buffer.load();
    let music_playing = shared.playing.load(Ordering::Relaxed);
    let music = music_buffer
        .as_ref()
        .filter(|_| music_playing)
        .map(|buffer| buffer.as_ref());
    let looping = shared.looping.load(Ordering::Relaxed);
    // The buffer holds one pass, or several when the accompaniment varies.
    // One-shot playback plays the first pass, not every variation.
    let end_sample = music
        .map(|buffer| {
            if looping {
                buffer.loop_end_sample
            } else {
                buffer
                    .loop_start_sample
                    .saturating_add(buffer.loop_pass_samples)
                    .min(buffer.loop_end_sample)
            }
        })
        .unwrap_or(0);
    let loop_start = music.map(|buffer| buffer.loop_start_sample).unwrap_or(0);
    let mut music_position = shared.position.load(Ordering::Relaxed);
    let mut music_finished = false;

    let speech_buffer = shared.speech.load();
    let mut speech_playing = shared.speech_playing.load(Ordering::Relaxed);
    let speech = speech_buffer.as_ref().map(|samples| samples.as_slice());
    let mut speech_position = shared.speech_position.load(Ordering::Relaxed);

    for frame in data.chunks_mut(channels) {
        let (mut left, mut right) = (0.0f32, 0.0f32);

        if let Some(buffer) = music {
            if music_position >= end_sample {
                if looping {
                    music_position = loop_start;
                } else {
                    music_finished = true;
                }
            }
            if !music_finished {
                let index = music_position as usize * 2;
                left = buffer.samples.get(index).copied().unwrap_or(0.0) * volume;
                right = buffer.samples.get(index + 1).copied().unwrap_or(left) * volume;
                music_position += 1;
            }
        }

        if speech_playing && let Some(samples) = speech {
            if let Some(sample) = samples.get(speech_position as usize) {
                let sample = sample * volume;
                left += sample;
                right += sample;
                speech_position += 1;
            } else {
                speech_playing = false;
            }
        }

        left = left.clamp(-1.0, 1.0);
        right = right.clamp(-1.0, 1.0);
        match frame.len() {
            0 => {}
            1 => frame[0] = T::from_sample((left + right) * 0.5),
            _ => {
                frame[0] = T::from_sample(left);
                frame[1] = T::from_sample(right);
                for sample in frame.iter_mut().skip(2) {
                    *sample = T::from_sample(0.0);
                }
            }
        }
    }

    shared.position.store(music_position, Ordering::Relaxed);
    if music_finished {
        shared.playing.store(false, Ordering::Relaxed);
        shared.finished.store(true, Ordering::Relaxed);
    }
    shared
        .speech_position
        .store(speech_position, Ordering::Relaxed);
    shared
        .speech_playing
        .store(speech_playing, Ordering::Relaxed);
}

// Keep `cpal::Sample` in scope for the generic conversion above.
#[allow(unused_imports)]
use cpal::Sample as _;

#[cfg(test)]
mod tests {
    use super::*;

    fn test_buffer() -> AudioBuffer {
        AudioBuffer {
            samples: vec![0.0; 3_000 * 2],
            sample_rate: 1_000,
            channels: 2,
            count_in_samples: 1_000,
            loop_start_sample: 1_000,
            loop_end_sample: 3_000,
            loop_pass_samples: 2_000,
            samples_per_beat: 500.0,
        }
    }

    #[test]
    fn seek_beat_moves_the_transport_without_playing() {
        let audio = AudioEngine::silent(1_000);
        audio.set_buffer(test_buffer());

        audio.seek_beat(2.0);

        assert_eq!(audio.position_samples(), 2_000);
        assert!(!audio.is_playing());
        assert!((audio.position_beat() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn seek_beat_handles_the_count_in_and_clamps() {
        let audio = AudioEngine::silent(1_000);
        audio.set_buffer(test_buffer());

        audio.seek_beat(-1.0);
        assert_eq!(audio.position_samples(), 500);

        audio.seek_beat(-100.0);
        assert_eq!(audio.position_samples(), 0);

        audio.seek_beat(1_000.0);
        assert_eq!(audio.position_samples(), 2_999);
    }

    #[test]
    fn speech_is_mixed_without_music() {
        let shared = Shared::new();
        shared
            .speech
            .store(Some(Arc::new(vec![0.5, -0.5, 0.25, 0.0])));
        shared.speech_playing.store(true, Ordering::Relaxed);
        let mut data = [0.0f32; 10];
        write_data(&mut data, 2, &shared);
        assert!(data[0] > 0.4);
        assert!(data[1] > 0.4);
        assert!(data[2] < -0.4);
        assert_eq!(data[6], 0.0);
        assert!(!shared.speech_playing.load(Ordering::Relaxed));
    }

    #[test]
    fn stopping_speech_clears_the_playback_position() {
        let engine = AudioEngine::silent(48_000);
        engine.shared.speech.store(Some(Arc::new(vec![0.5, 0.5])));
        engine.shared.speech_playing.store(true, Ordering::Relaxed);
        engine.shared.speech_position.store(1, Ordering::Relaxed);
        engine.stop_speech();
        assert!(!engine.shared.speech_playing.load(Ordering::Relaxed));
        assert_eq!(engine.shared.speech_position.load(Ordering::Relaxed), 0);
    }
}
