//! Dancer sounds: the stomps and claps the dancers make during a figure.
//!
//! The events come from the dance card (see `contra_core::dancer_sounds`) and
//! are already on the dance-beat grid. This module only renders them, with
//! small procedural voices so no new asset or soundfont preset is needed. The
//! layer is mixed after the effects chain, so reverb or echo cannot smear a
//! clap past its beat.

use std::f32::consts::TAU;

use contra_core::{DancerSound, DancerSoundEvent};

use crate::render::{AudioBuffer, RenderRequest};

/// Length of a rendered foot stomp, in seconds.
pub const STOMP_SECONDS: f64 = 0.12;
/// Length of a rendered hand clap, in seconds.
pub const CLAP_SECONDS: f64 = 0.06;

/// Mix the dancer sound layer into every loop pass.
///
/// Nothing happens when the feature is off, the events are empty, or the level
/// is zero. The sounds never move a beat and never replace the band: they are
/// added under it, so the layer cannot introduce a silent bar.
pub(crate) fn mix(buffer: &mut AudioBuffer, request: &RenderRequest) {
    if !request.dancer_sounds {
        return;
    }
    let volume = request.dancer_sound_volume.clamp(0.0, 2.0);
    if volume <= 0.0 || request.dancer_sound_events.is_empty() {
        return;
    }
    let pass_samples = buffer.loop_pass_samples.max(1);
    let passes = ((buffer
        .loop_end_sample
        .saturating_sub(buffer.loop_start_sample))
        / pass_samples)
        .max(1) as u32;

    for pass in 0..passes {
        let pass_start = buffer.loop_start_sample + pass as u64 * pass_samples;
        let pass_end = pass_start + pass_samples;
        for event in &request.dancer_sound_events {
            render_event(buffer, event, pass_start, pass_end, volume);
        }
    }
    limit(&mut buffer.samples);
}

fn render_event(
    buffer: &mut AudioBuffer,
    event: &DancerSoundEvent,
    pass_start: u64,
    pass_end: u64,
    volume: f32,
) {
    if !event.beat.is_finite() || event.beat < 0.0 {
        return;
    }
    let start = pass_start + (event.beat * buffer.samples_per_beat).round() as u64;
    if start >= pass_end || start >= buffer.loop_end_sample {
        return;
    }
    let seconds = match event.sound {
        DancerSound::Stomp => STOMP_SECONDS,
        DancerSound::Clap => CLAP_SECONDS,
    };
    let duration = ((seconds * buffer.sample_rate as f64).round() as usize).max(1);
    let duration = duration.min((pass_end - start) as usize);
    if duration == 0 {
        return;
    }
    let gain = volume * event.gain.clamp(0.0, 1.0);
    let pan = match event.sound {
        DancerSound::Stomp => -0.14,
        DancerSound::Clap => 0.14,
    };
    render_voice(
        &mut buffer.samples,
        start as usize,
        duration,
        event.sound,
        gain,
        buffer.sample_rate,
        pan,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_voice(
    out: &mut [f32],
    start: usize,
    duration: usize,
    sound: DancerSound,
    gain: f32,
    sample_rate: u32,
    pan: f32,
) {
    let frames = out.len() / 2;
    if start >= frames || duration == 0 {
        return;
    }
    let duration = duration.min(frames - start);
    let (left_gain, right_gain) = pan_gains(pan);
    let sample_rate = sample_rate.max(8_000) as f32;
    let mut noise = Noise::new((start as u32).wrapping_mul(31) ^ sound_tag(sound));
    let mut phase = 0.0f32;
    let mut last_noise = 0.0f32;
    let attack_samples = (sample_rate * 0.002).max(1.0);
    let release_samples = (sample_rate * 0.004).max(1.0);

    for index in 0..duration {
        let t = index as f32 / sample_rate;
        let attack = (index as f32 / attack_samples).min(1.0);
        let release = ((duration - index) as f32 / release_samples).clamp(0.0, 1.0);
        let sample = match sound {
            DancerSound::Stomp => {
                // A foot on a wooden floor: a low body that drops in pitch,
                // plus a short scuff for the attack.
                let frequency = 92.0 + 40.0 * (-t * 40.0).exp();
                phase += frequency / sample_rate;
                phase -= phase.floor();
                let body = (TAU * phase).sin() * 0.8 + (TAU * phase * 2.0).sin() * 0.2;
                let scuff = noise.next() * (-t * 240.0).exp() * 0.35;
                (body + scuff) * (-t * 30.0).exp()
            }
            DancerSound::Clap => {
                // Three quick noise bursts a few milliseconds apart read as a
                // hand clap. The envelope closes fast so it never smears into
                // the next beat.
                let value = noise.next();
                let high = value - last_noise * 0.9;
                last_noise = value;
                let burst = if t < 0.012 {
                    1.0
                } else if (0.018..0.030).contains(&t) {
                    0.55
                } else {
                    0.25
                };
                high * burst * (-t * 65.0).exp() * 2.2
            }
        };
        let value = sample * attack * release * gain * 0.5;
        out[start * 2 + index * 2] += value * left_gain;
        out[start * 2 + index * 2 + 1] += value * right_gain;
    }
}

fn sound_tag(sound: DancerSound) -> u32 {
    match sound {
        DancerSound::Stomp => 0x5354_4f4d,
        DancerSound::Clap => 0x434c_4150,
    }
}

/// Linear pan, so a centred sound keeps the level it had.
fn pan_gains(pan: f32) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    (1.0 - pan.max(0.0), 1.0 + pan.min(0.0))
}

fn limit(samples: &mut [f32]) {
    let peak = samples
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    if peak > 0.95 {
        let scale = 0.95 / peak;
        for sample in samples {
            *sample *= scale;
        }
    }
}

/// Small deterministic white-noise generator (xorshift32).
struct Noise {
    state: u32,
}

impl Noise {
    fn new(seed: u32) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abc::parse_abc;
    use crate::arrange::Arrangement;

    const TUNE: &str = "X:1\nT:T\nM:4/4\nL:1/8\nK:D\n\"D\"A2 A2 A2 A2 | \"G\"B2 B2 B2 B2|\n";

    fn buffer() -> AudioBuffer {
        AudioBuffer {
            samples: vec![0.0; 48_000 * 2],
            sample_rate: 24_000,
            channels: 2,
            count_in_samples: 0,
            loop_start_sample: 0,
            loop_end_sample: 48_000,
            loop_pass_samples: 48_000,
            samples_per_beat: 12_000.0,
            ending_from_beat: 0.0,
            ending_start_sample: None,
        }
    }

    fn request() -> RenderRequest {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        RenderRequest {
            arrangement,
            bpm: 240.0,
            melody: contra_core::MelodyInstrument::Fiddle,
            accompaniment: contra_core::AccompanimentInstrument::Piano,
            accompaniment_rhythm: contra_core::AccompanimentRhythm::Auto,
            second_accompaniment: contra_core::Countermelody::Off,
            copies: contra_core::InstrumentCopies::default(),
            band: contra_core::default_band(),
            output: contra_core::OutputMode::Stereo,
            variations: contra_core::VariationOptions::default(),
            humanize_options: contra_core::HumanizeOptions::default(),
            ending_options: contra_core::EndingOptions::default(),
            dancer_sounds: true,
            dancer_sound_volume: 0.35,
            dancer_sound_events: vec![
                DancerSoundEvent {
                    beat: 0.0,
                    sound: DancerSound::Stomp,
                    gain: 0.9,
                },
                DancerSoundEvent {
                    beat: 2.0,
                    sound: DancerSound::Clap,
                    gain: 0.8,
                },
            ],
            percussion: contra_core::Percussion::Off,
            percussion_rhythm: contra_core::PercussionRhythm::Auto,
            melody_volume: 0.75,
            accompaniment_volume: 0.65,
            percussion_volume: 0.0,
            effects: contra_core::AudioEffects::default(),
            count_in_beats: 0.0,
            sample_rate: 24_000,
            variation_passes: 1,
            ending: false,
            humanize_timing: false,
        }
    }

    #[test]
    fn disabled_leaves_the_buffer_silent() {
        let mut buffer = buffer();
        let mut request = request();
        request.dancer_sounds = false;
        mix(&mut buffer, &request);
        assert!(buffer.samples.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn stomps_and_claps_are_mixed_under_the_band() {
        let mut buffer = buffer();
        mix(&mut buffer, &request());
        assert!(buffer.samples.iter().any(|sample| sample.abs() > 0.0));
        assert!(buffer.samples.iter().all(|sample| sample.is_finite()));
        assert!(buffer.samples.iter().all(|sample| sample.abs() <= 1.0));
        // The stomp starts at beat 0, the clap at beat 2 (halfway through this
        // synthetic one-pass buffer).
        assert!(energy(&buffer.samples, 0, 2_000) > 0.0);
        assert!(energy(&buffer.samples, 24_000, 26_000) > 0.0);
    }

    #[test]
    fn a_clap_does_not_smear_past_its_beat() {
        let mut buffer = buffer();
        mix(&mut buffer, &request());
        let clap_start = 24_000;
        let after = energy(&buffer.samples, clap_start + 2_000, clap_start + 4_000);
        assert!(after < 1e-6, "clap tail leaked: {after}");
    }

    #[test]
    fn mixing_is_deterministic() {
        let mut first = buffer();
        let mut second = buffer();
        mix(&mut first, &request());
        mix(&mut second, &request());
        assert_eq!(first.samples, second.samples);
    }

    #[test]
    fn every_pass_gets_the_sounds() {
        let mut buffer = buffer();
        buffer.samples = vec![0.0; 96_000 * 2];
        buffer.loop_end_sample = 96_000;
        let mut request = request();
        request.variation_passes = 2;
        mix(&mut buffer, &request);
        assert!(energy(&buffer.samples, 0, 2_000) > 0.0);
        assert!(energy(&buffer.samples, 48_000, 50_000) > 0.0);
    }

    fn energy(samples: &[f32], start: usize, end: usize) -> f32 {
        samples[start * 2..end * 2]
            .iter()
            .fold(0.0f32, |sum, sample| sum + sample.abs())
    }
}
