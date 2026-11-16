//! cpal playback for the pre-rendered loop.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use arc_swap::ArcSwapOption;
#[cfg(feature = "headless")]
use contra_core::COUNT_IN_BEATS;
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
    /// Play the once-only ending at the end of the current pass instead of
    /// looping. The app sets this from the **End tune** control.
    ending: AtomicBool,
    /// Band level from Preferences::master_volume.
    volume: AtomicU32,
    /// Caller/speech level from Preferences::caller_volume. Independent of the
    /// band, so turning the band down never hides a call.
    caller_volume: AtomicU32,
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

    /// A silent transport with a 64-beat loop and four-beat count-in.
    ///
    /// The headless renderer uses this so the real UI can seek to any beat
    /// without an audio device.
    #[cfg(feature = "headless")]
    pub fn headless_clock(sample_rate: u32, bpm: f64) -> Self {
        let samples_per_beat = sample_rate as f64 * 120.0 / bpm.max(1.0);
        let count_in_samples = (COUNT_IN_BEATS * samples_per_beat).round() as u64;
        let loop_end_sample = count_in_samples + (64.0 * samples_per_beat).round() as u64;
        let buffer = AudioBuffer {
            samples: vec![0.0; loop_end_sample as usize * 2],
            sample_rate,
            channels: 2,
            count_in_samples,
            loop_start_sample: count_in_samples,
            loop_end_sample,
            loop_pass_samples: (64.0 * samples_per_beat).round() as u64,
            samples_per_beat,
            ending_from_beat: 62.0,
            ending_start_sample: None,
        };
        let audio = Self::silent(sample_rate);
        audio.set_buffer(buffer);
        audio
    }

    /// Force the transport into its finished state for headless screenshots.
    #[cfg(feature = "headless")]
    pub fn force_finished(&self) {
        if let Some(buffer) = self.shared.buffer.load_full() {
            self.shared
                .position
                .store(buffer.loop_end_sample, Ordering::Relaxed);
        }
        self.shared.playing.store(false, Ordering::Relaxed);
        self.shared.finished.store(true, Ordering::Relaxed);
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn set_buffer(&self, buffer: AudioBuffer) {
        self.shared.buffer.store(Some(Arc::new(buffer)));
        self.shared.position.store(0, Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
        self.shared.ending.store(false, Ordering::Relaxed);
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
            self.shared.ending.store(false, Ordering::Relaxed);
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

    /// Stop music output without touching speech.
    ///
    /// A tune change swaps the pre-rendered buffer at a pass boundary while the
    /// caller may still be speaking the call that straddles the boundary.
    pub fn pause_music(&self) {
        self.shared.playing.store(false, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        self.shared.playing.store(false, Ordering::Relaxed);
        self.shared.position.store(0, Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
        self.shared.ending.store(false, Ordering::Relaxed);
        self.stop_speech();
    }

    pub fn restart(&self) {
        self.shared.position.store(0, Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
        self.shared.ending.store(false, Ordering::Relaxed);
        self.stop_speech();
        self.play();
    }

    /// Make the transport take the once-only ending at the end of this pass.
    ///
    /// The audio thread splices into the ending at the start of the tune's last
    /// bar, so the ending can stretch that bar rather than repeat it.
    pub fn request_ending(&self) {
        self.shared.ending.store(true, Ordering::Relaxed);
    }

    pub fn cancel_ending(&self) {
        self.shared.ending.store(false, Ordering::Relaxed);
    }

    pub fn ending_requested(&self) -> bool {
        self.shared.ending.load(Ordering::Relaxed)
    }

    /// Whether the transport is currently inside the once-only ending.
    pub fn in_ending(&self) -> bool {
        let Some(buffer) = self.shared.buffer.load_full() else {
            return false;
        };
        buffer.in_ending(self.position_samples())
    }

    pub fn set_looping(&self, looping: bool) {
        self.shared.looping.store(looping, Ordering::Relaxed);
    }

    pub fn set_volume(&self, volume: f32) {
        self.shared
            .volume
            .store(volume.clamp(0.0, 2.0).to_bits(), Ordering::Relaxed);
    }

    /// Set the caller/speech level without touching the band mix.
    pub fn set_caller_volume(&self, volume: f32) {
        self.shared
            .caller_volume
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
            ending: AtomicBool::new(false),
            volume: AtomicU32::new(1.0f32.to_bits()),
            caller_volume: AtomicU32::new(1.0f32.to_bits()),
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
    let caller_volume = f32::from_bits(shared.caller_volume.load(Ordering::Relaxed));
    let music_buffer = shared.buffer.load();
    let music_playing = shared.playing.load(Ordering::Relaxed);
    let music = music_buffer
        .as_ref()
        .filter(|_| music_playing)
        .map(|buffer| buffer.as_ref());
    let output_channels = music.map(|buffer| buffer.channels.max(1)).unwrap_or(2);
    let output_mode = mode_of(output_channels);
    let caller_gains = contra_music::layout::caller_gains(output_mode);
    let looping = shared.looping.load(Ordering::Relaxed);
    let ending_requested = shared.ending.load(Ordering::Relaxed);
    let mut music_position = shared.position.load(Ordering::Relaxed);
    let mut music_finished = false;

    let speech_buffer = shared.speech.load();
    let mut speech_playing = shared.speech_playing.load(Ordering::Relaxed);
    let speech = speech_buffer.as_ref().map(|samples| samples.as_slice());
    let mut speech_position = shared.speech_position.load(Ordering::Relaxed);

    for frame in data.chunks_mut(channels) {
        let mut staged = [0.0f32; contra_music::layout::MAX_CHANNELS];

        if let Some(buffer) = music {
            if ending_requested {
                // The ending replaces the tune's last bar. Splice into the
                // appended region as soon as playback reaches that bar, so the
                // bar is heard once, stretched, instead of twice.
                if let Some(ending_start) = buffer.ending_start_sample {
                    if music_position < ending_start {
                        let pass_samples = buffer.loop_pass_samples.max(1);
                        let position_in_pass =
                            music_position.saturating_sub(buffer.loop_start_sample) % pass_samples;
                        let jump_at = (buffer.ending_from_beat.max(0.0) * buffer.samples_per_beat)
                            .round() as u64;
                        let jump_at = jump_at.min(pass_samples.saturating_sub(1));
                        if music_position >= buffer.loop_start_sample && position_in_pass >= jump_at
                        {
                            music_position = ending_start;
                        }
                    }
                    if music_position >= buffer.frames() as u64 {
                        music_finished = true;
                    }
                } else {
                    // No ending was rendered (for example the synthetic
                    // headless clock); stop cleanly at the end of this pass
                    // instead of looping.
                    let pass_end = buffer
                        .loop_start_sample
                        .saturating_add(buffer.loop_pass_samples)
                        .min(buffer.loop_end_sample);
                    if music_position >= pass_end {
                        music_finished = true;
                    }
                }
            } else {
                let end_sample = if looping {
                    buffer.loop_end_sample
                } else {
                    buffer
                        .loop_start_sample
                        .saturating_add(buffer.loop_pass_samples)
                        .min(buffer.loop_end_sample)
                };
                if music_position >= end_sample {
                    if looping {
                        music_position = buffer.loop_start_sample;
                    } else {
                        music_finished = true;
                    }
                }
            }
            if !music_finished {
                let source_channels = buffer.channels.max(1) as usize;
                let index = music_position as usize * source_channels;
                for channel in 0..source_channels.min(staged.len()) {
                    staged[channel] =
                        buffer.samples.get(index + channel).copied().unwrap_or(0.0) * volume;
                }
                music_position += 1;
            }
        }

        if speech_playing && let Some(samples) = speech {
            if let Some(sample) = samples.get(speech_position as usize) {
                let sample = sample * caller_volume;
                for (channel, gain) in caller_gains
                    .iter()
                    .enumerate()
                    .take(output_channels as usize)
                {
                    if channel < staged.len() {
                        staged[channel] += sample * gain;
                    }
                }
                speech_position += 1;
            } else {
                speech_playing = false;
            }
        }

        let mut mixed = [0.0f32; 8];
        let length = frame.len().min(mixed.len());
        downmix(&staged[..output_channels as usize], &mut mixed[..length]);
        for (index, sample) in frame.iter_mut().enumerate() {
            *sample = T::from_sample(mixed.get(index).copied().unwrap_or(0.0));
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

/// The layout a rendered buffer uses, from its channel count.
fn mode_of(channels: u16) -> contra_core::OutputMode {
    match channels {
        1 => contra_core::OutputMode::Mono,
        5 => contra_core::OutputMode::Surround,
        _ => contra_core::OutputMode::Stereo,
    }
}

/// Map one rendered frame onto the device's channels without allocating.
///
/// A mono device hears the energy average of every rendered channel. A stereo
/// device folds a surround frame to its front pair plus a little centre and
/// rear, so no instrument disappears. A device with more channels than the
/// render keeps the layout's channel order and leaves the rest silent.
fn downmix(src: &[f32], dst: &mut [f32]) {
    for sample in dst.iter_mut() {
        *sample = 0.0;
    }
    match (src.len(), dst.len()) {
        (5, 1) => {
            dst[0] = (src[0] + src[1] + src[2] + src[3] + src[4]) / 5.0;
        }
        (5, 2) => {
            dst[0] = src[0] + 0.5 * src[2] + 0.707 * src[3];
            dst[1] = src[1] + 0.5 * src[2] + 0.707 * src[4];
        }
        (5, _) => {
            for (channel, sample) in src.iter().enumerate().take(dst.len()) {
                dst[channel] = *sample;
            }
        }
        (2, 1) => dst[0] = (src[0] + src[1]) * 0.5,
        (2, _) => {
            dst[0] = src[0];
            if dst.len() > 1 {
                dst[1] = src[1];
            }
        }
        (1, _) => {
            for sample in dst.iter_mut() {
                *sample = src[0];
            }
        }
        (_, _) => {
            for (channel, sample) in src.iter().enumerate().take(dst.len()) {
                dst[channel] = *sample;
            }
        }
    }
    for sample in dst.iter_mut() {
        *sample = sample.clamp(-1.0, 1.0);
    }
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
            ending_from_beat: 0.0,
            ending_start_sample: None,
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

    #[test]
    fn pausing_music_for_a_tune_change_keeps_speech() {
        let engine = AudioEngine::silent(48_000);
        engine.play_speech(vec![0.5, 0.5]);
        engine.play();
        engine.pause_music();
        assert!(!engine.is_playing());
        assert!(engine.shared.speech_playing.load(Ordering::Relaxed));
    }

    fn ending_buffer() -> AudioBuffer {
        let mut buffer = test_buffer();
        // Extend the buffer with a one-second ending region at sample 3000.
        buffer.samples.resize(4_000 * 2, 0.5);
        buffer.ending_start_sample = Some(3_000);
        buffer.ending_from_beat = 1.0;
        buffer
    }

    #[test]
    fn requesting_the_ending_splices_in_at_the_last_bar() {
        let shared = Shared::new();
        shared.buffer.store(Some(Arc::new(ending_buffer())));
        shared.playing.store(true, Ordering::Relaxed);
        shared.ending.store(true, Ordering::Relaxed);
        // `ending_from_beat = 1` means the jump happens one beat into a pass;
        // position 1_100 is only 0.2 beats in, so playback must continue.
        shared.position.store(1_100, Ordering::Relaxed);
        let mut data = [0.0f32; 200];
        write_data(&mut data, 2, &shared);
        assert_eq!(shared.position.load(Ordering::Relaxed), 1_200);
        assert!(shared.ending.load(Ordering::Relaxed));
        assert!(!shared.finished.load(Ordering::Relaxed));

        // Position 2_500 is 3 beats into the pass (past the jump), so the
        // callback must move to the appended ending and play it.
        shared.position.store(2_500, Ordering::Relaxed);
        let mut data = [0.0f32; 200];
        write_data(&mut data, 2, &shared);
        assert!(
            data[0] > 0.4,
            "the first frame should come from the ending region"
        );
        assert_eq!(shared.position.load(Ordering::Relaxed), 3_100);
    }

    #[test]
    fn the_ending_plays_to_the_end_and_finishes() {
        let shared = Shared::new();
        shared.buffer.store(Some(Arc::new(ending_buffer())));
        shared.playing.store(true, Ordering::Relaxed);
        shared.ending.store(true, Ordering::Relaxed);
        shared.position.store(3_900, Ordering::Relaxed);
        let mut data = [0.0f32; 400];
        write_data(&mut data, 2, &shared);
        assert!(shared.finished.load(Ordering::Relaxed));
        assert!(!shared.playing.load(Ordering::Relaxed));
        assert_eq!(shared.position.load(Ordering::Relaxed), 4_000);
        assert!(data[0] > 0.4);
    }

    fn constant_buffer(value: f32, channels: u16) -> AudioBuffer {
        let mut buffer = test_buffer();
        buffer.channels = channels;
        buffer.samples = vec![value; buffer.loop_end_sample as usize * channels as usize + 200];
        buffer
    }

    #[test]
    fn the_caller_level_is_independent_of_the_band_level() {
        // Silent band, audible caller.
        let shared = Shared::new();
        shared.buffer.store(Some(Arc::new(constant_buffer(0.0, 2))));
        shared.playing.store(true, Ordering::Relaxed);
        shared.speech.store(Some(Arc::new(vec![0.5; 64])));
        shared.speech_playing.store(true, Ordering::Relaxed);
        shared.volume.store(0.0f32.to_bits(), Ordering::Relaxed);
        shared
            .caller_volume
            .store(1.0f32.to_bits(), Ordering::Relaxed);
        let mut data = [0.0f32; 8];
        write_data(&mut data, 2, &shared);
        assert!(data[0] > 0.4, "the caller must survive a muted band");

        // Audible band, silent caller.
        let shared = Shared::new();
        shared.buffer.store(Some(Arc::new(constant_buffer(0.5, 2))));
        shared.playing.store(true, Ordering::Relaxed);
        shared.speech.store(Some(Arc::new(vec![0.5; 64])));
        shared.speech_playing.store(true, Ordering::Relaxed);
        shared.volume.store(1.0f32.to_bits(), Ordering::Relaxed);
        shared
            .caller_volume
            .store(0.0f32.to_bits(), Ordering::Relaxed);
        let mut data = [0.0f32; 8];
        write_data(&mut data, 2, &shared);
        assert!(
            (data[0] - 0.5).abs() < 1e-6,
            "the band is unaffected by the caller"
        );
        assert!((data[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_surround_buffer_plays_on_a_stereo_device_without_missing_parts() {
        let shared = Shared::new();
        let mut buffer = constant_buffer(0.0, 5);
        // Front left, front right, centre, rear left, rear right.
        for frame in buffer.samples.as_chunks_mut::<5>().0.iter_mut() {
            frame[0] = 0.2;
            frame[1] = -0.2;
            frame[2] = 0.1;
            frame[3] = 0.1;
            frame[4] = -0.1;
        }
        shared.buffer.store(Some(Arc::new(buffer)));
        shared.playing.store(true, Ordering::Relaxed);
        let mut data = [0.0f32; 10];
        write_data(&mut data, 2, &shared);
        assert!(
            data[0] > 0.1,
            "front and rear left reach the left device channel"
        );
        assert!(
            data[1] < -0.1,
            "front and rear right reach the right channel"
        );
        assert!(data.iter().all(|sample| sample.is_finite()));
        assert!(data.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn a_surround_buffer_keeps_all_five_channels_on_a_surround_device() {
        let shared = Shared::new();
        let mut buffer = constant_buffer(0.0, 5);
        for frame in buffer.samples.as_chunks_mut::<5>().0.iter_mut() {
            frame[0] = 0.1;
            frame[1] = 0.2;
            frame[2] = 0.3;
            frame[3] = 0.4;
            frame[4] = 0.5;
        }
        shared.buffer.store(Some(Arc::new(buffer)));
        shared.playing.store(true, Ordering::Relaxed);
        let mut data = [0.0f32; 10];
        write_data(&mut data, 5, &shared);
        assert!((data[0] - 0.1).abs() < 1e-6);
        assert!((data[1] - 0.2).abs() < 1e-6);
        assert!((data[2] - 0.3).abs() < 1e-6);
        assert!((data[3] - 0.4).abs() < 1e-6);
        assert!((data[4] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_stereo_buffer_plays_on_a_mono_device_as_its_sum() {
        let shared = Shared::new();
        shared.buffer.store(Some(Arc::new(constant_buffer(0.4, 2))));
        shared.playing.store(true, Ordering::Relaxed);
        let mut data = [0.0f32; 4];
        write_data(&mut data, 1, &shared);
        assert!((data[0] - 0.4).abs() < 1e-6);
    }
}
