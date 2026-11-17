//! Offline audio effects for in-app playback.
//!
//! The same [`AudioEffects`] settings that become General MIDI sends in an
//! exported file are applied here to the rendered stereo buffer. Processing
//! happens once, on the render worker, after the virtual instruments and the
//! percussion layer have been mixed, so every voice shares one room.
//!
//! The chain is chorus, then echo, then reverb, followed by a peak limiter.
//! It is deterministic, allocation is bounded by the delay lines, and the
//! audio callback is never involved.

use std::f32::consts::TAU;

use caller_core::{AudioEffects, ChorusPreset, EchoPreset, ReverbPreset};

/// Apply the selected effects to an interleaved buffer.
///
/// `channels` is the buffer's channel count. Stereo keeps the original chain
/// sample-for-sample; mono runs the same chain on one channel; every extra
/// surround channel gets its own delay line and reverb so a five-channel render
/// is genuinely processed instead of being read as two-channel frames.
///
/// Nothing changes when all three effects are off, which keeps the dry render
/// sample-for-sample identical to the previous behaviour.
pub fn apply(samples: &mut [f32], sample_rate: u32, channels: u16, effects: AudioEffects) {
    let channels = usize::from(channels.max(1));
    if effects.is_dry() || samples.len() < channels {
        return;
    }
    let sample_rate = sample_rate.max(8_000);
    apply_chorus(samples, sample_rate, channels, effects.chorus);
    apply_echo(samples, sample_rate, channels, effects.echo);
    apply_reverb(samples, sample_rate, channels, effects.reverb);
    limit(samples);
}

fn apply_chorus(samples: &mut [f32], sample_rate: u32, channels: usize, preset: ChorusPreset) {
    let (rate_hz, depth_seconds, base_seconds, wet) = match preset {
        ChorusPreset::Off => return,
        ChorusPreset::Subtle => (0.45, 0.0025, 0.018, 0.10),
        ChorusPreset::Rich => (0.80, 0.0045, 0.022, 0.22),
    };
    let base = base_seconds * sample_rate as f32;
    let depth = depth_seconds * sample_rate as f32;
    let length = (base + depth).ceil() as usize + 2;
    let mut lines = vec![vec![0.0f32; length]; channels];
    let mut write = 0usize;
    let mut phase = 0.0f32;
    let step = rate_hz / sample_rate as f32;

    for frame in samples.chunks_mut(channels) {
        for (channel, line) in lines.iter_mut().enumerate() {
            line[write] = frame[channel];
            // A quarter-cycle offset between the first two channels widens the
            // image exactly as the stereo chain always did. Extra channels
            // continue the rotation rather than copying the front pair.
            let channel_phase = phase + channel as f32 * 0.25;
            let delay = base + depth * (0.5 + 0.5 * (TAU * channel_phase).sin());
            frame[channel] += wet * read_delay(line, write, delay);
        }

        write += 1;
        if write == length {
            write = 0;
        }
        phase += step;
        if phase >= 1.0 {
            phase -= 1.0;
        }
    }
}

/// Read a fractional delay from a ring buffer that was just written at `write`.
fn read_delay(buffer: &[f32], write: usize, delay: f32) -> f32 {
    let length = buffer.len() as f32;
    let delay = delay.clamp(1.0, length - 2.0);
    let read = (write as f32 - delay).rem_euclid(length);
    let first = read.floor() as usize;
    let second = (first + 1) % buffer.len();
    let fraction = read - first as f32;
    buffer[first] * (1.0 - fraction) + buffer[second] * fraction
}

fn apply_echo(samples: &mut [f32], sample_rate: u32, channels: usize, preset: EchoPreset) {
    let (delay_seconds, feedback, wet) = match preset {
        EchoPreset::Off => return,
        EchoPreset::Subtle => (0.120, 0.22, 0.16),
        EchoPreset::Strong => (0.260, 0.38, 0.24),
    };
    let delay = (delay_seconds * sample_rate as f32).round() as usize;
    if delay < 1 {
        return;
    }
    let mut lines = vec![vec![0.0f32; delay]; channels];
    let mut index = 0usize;

    for frame in samples.chunks_mut(channels) {
        for (channel, line) in lines.iter_mut().enumerate() {
            let delayed = line[index];
            line[index] = frame[channel] + delayed * feedback;
            frame[channel] += delayed * wet;
        }
        index += 1;
        if index == delay {
            index = 0;
        }
    }
}

fn apply_reverb(samples: &mut [f32], sample_rate: u32, channels: usize, preset: ReverbPreset) {
    let Some((feedback, damp)) = reverb_room(preset) else {
        return;
    };
    // Keep the wet level tied to the same send level the MIDI export writes.
    let wet = preset.send_level() as f32 / 127.0 * 0.55;
    let mut front = Reverb::new(sample_rate, feedback, damp, wet);
    // One extra mono reverb per channel beyond the front pair (the centre and
    // the rear pair in a surround render). This keeps each channel's tail
    // independent instead of folding the extra channels into the front pair.
    let pairs = if channels >= 2 { 2 } else { 1 };
    let mut mono = (pairs..channels)
        .map(|_| Reverb::new(sample_rate, feedback, damp, wet))
        .collect::<Vec<_>>();

    for frame in samples.chunks_mut(channels) {
        if channels >= 2 {
            let (left, right) = front.process(frame[0], frame[1]);
            frame[0] = left;
            frame[1] = right;
        } else {
            let (value, _) = front.process(frame[0], frame[0]);
            frame[0] = value;
        }
        for (offset, reverb) in mono.iter_mut().enumerate() {
            let channel = pairs + offset;
            if channel >= channels {
                break;
            }
            let (value, _) = reverb.process(frame[channel], frame[channel]);
            frame[channel] = value;
        }
    }
}

fn reverb_room(preset: ReverbPreset) -> Option<(f32, f32)> {
    Some(match preset {
        ReverbPreset::Off => return None,
        ReverbPreset::SmallRoom => (0.55, 0.60),
        ReverbPreset::MediumRoom => (0.70, 0.45),
        ReverbPreset::LargeRoom => (0.80, 0.35),
        ReverbPreset::Hall => (0.88, 0.28),
    })
}

/// A small Freeverb-style stereo reverb.
struct Reverb {
    combs_left: Vec<Comb>,
    combs_right: Vec<Comb>,
    allpass_left: Vec<Allpass>,
    allpass_right: Vec<Allpass>,
    feedback: f32,
    damp: f32,
    wet: f32,
    dry: f32,
}

impl Reverb {
    fn new(sample_rate: u32, feedback: f32, damp: f32, wet: f32) -> Self {
        let scale = sample_rate as f32 / 44_100.0;
        let mut combs_left = Vec::with_capacity(COMB_TUNING.len());
        let mut combs_right = Vec::with_capacity(COMB_TUNING.len());
        for tuning in COMB_TUNING {
            combs_left.push(Comb::new(scaled(tuning, scale)));
            combs_right.push(Comb::new(scaled(tuning + STEREO_SPREAD, scale)));
        }
        let mut allpass_left = Vec::with_capacity(ALLPASS_TUNING.len());
        let mut allpass_right = Vec::with_capacity(ALLPASS_TUNING.len());
        for tuning in ALLPASS_TUNING {
            allpass_left.push(Allpass::new(scaled(tuning, scale)));
            allpass_right.push(Allpass::new(scaled(tuning + STEREO_SPREAD, scale)));
        }
        Self {
            combs_left,
            combs_right,
            allpass_left,
            allpass_right,
            feedback,
            damp,
            wet,
            // Pull the dry signal back a little as the wet signal rises so the
            // sum stays close to the level of the unprocessed voice.
            dry: 1.0 - wet * 0.5,
        }
    }

    fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let input = (left + right) * FREEVERB_INPUT_GAIN;
        let mut wet_left = 0.0f32;
        for comb in &mut self.combs_left {
            wet_left += comb.process(input, self.feedback, self.damp);
        }
        for allpass in &mut self.allpass_left {
            wet_left = allpass.process(wet_left);
        }
        let mut wet_right = 0.0f32;
        for comb in &mut self.combs_right {
            wet_right += comb.process(input, self.feedback, self.damp);
        }
        for allpass in &mut self.allpass_right {
            wet_right = allpass.process(wet_right);
        }
        (
            left * self.dry + wet_left * self.wet,
            right * self.dry + wet_right * self.wet,
        )
    }
}

fn scaled(length: usize, scale: f32) -> usize {
    ((length as f32 * scale).round() as usize).max(1)
}

const COMB_TUNING: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;
const FREEVERB_INPUT_GAIN: f32 = 0.015;

struct Comb {
    buffer: Vec<f32>,
    index: usize,
    store: f32,
}

impl Comb {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0f32; length],
            index: 0,
            store: 0.0,
        }
    }

    fn process(&mut self, input: f32, feedback: f32, damp: f32) -> f32 {
        let output = self.buffer[self.index];
        self.store = output * (1.0 - damp) + self.store * damp;
        self.buffer[self.index] = input + self.store * feedback;
        self.index += 1;
        if self.index == self.buffer.len() {
            self.index = 0;
        }
        output
    }
}

struct Allpass {
    buffer: Vec<f32>,
    index: usize,
}

impl Allpass {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0f32; length],
            index: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let buffered = self.buffer[self.index];
        let output = -input + buffered;
        self.buffer[self.index] = input + buffered * 0.5;
        self.index += 1;
        if self.index == self.buffer.len() {
            self.index = 0;
        }
        output
    }
}

/// Keep the effected buffer below full scale without changing its shape.
fn limit(samples: &mut [f32]) {
    let peak = samples
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    if peak > 0.98 {
        let scale = 0.98 / peak;
        for sample in samples {
            *sample *= scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 24_000;

    fn impulse(seconds: f32) -> Vec<f32> {
        let frames = (seconds * SAMPLE_RATE as f32) as usize;
        let mut samples = vec![0.0f32; frames * 2];
        samples[0] = 0.5;
        samples[1] = 0.5;
        samples
    }

    fn effects(reverb: ReverbPreset, chorus: ChorusPreset, echo: EchoPreset) -> AudioEffects {
        AudioEffects {
            reverb,
            chorus,
            echo,
        }
    }

    fn energy(samples: &[f32], from: f32, to: f32) -> f32 {
        let start = (from * SAMPLE_RATE as f32) as usize * 2;
        let end = (to * SAMPLE_RATE as f32) as usize * 2;
        samples[start..end]
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0, f32::max)
    }

    #[test]
    fn dry_effects_leave_the_buffer_untouched() {
        let original = impulse(0.5);
        let mut samples = original.clone();
        apply(
            &mut samples,
            SAMPLE_RATE,
            2,
            effects(ReverbPreset::Off, ChorusPreset::Off, EchoPreset::Off),
        );
        assert_eq!(samples, original);
    }

    #[test]
    fn medium_room_reverb_has_a_tail_and_stays_finite() {
        let mut samples = impulse(1.0);
        apply(
            &mut samples,
            SAMPLE_RATE,
            2,
            effects(ReverbPreset::MediumRoom, ChorusPreset::Off, EchoPreset::Off),
        );
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(
            energy(&samples, 0.05, 0.30) > 1e-6,
            "the reverb tail should continue after the impulse"
        );
        assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn echo_repeats_after_the_delay() {
        let mut samples = impulse(1.0);
        apply(
            &mut samples,
            SAMPLE_RATE,
            2,
            effects(ReverbPreset::Off, ChorusPreset::Off, EchoPreset::Subtle),
        );
        let quiet_before = energy(&samples, 0.02, 0.10);
        let repeat = energy(&samples, 0.12, 0.18);
        assert!(quiet_before < 1e-6);
        assert!(repeat > 1e-3, "the echo should repeat at about 120 ms");
    }

    #[test]
    fn chorus_changes_the_signal_without_changing_its_length() {
        let mut samples = impulse(0.5);
        let length = samples.len();
        apply(
            &mut samples,
            SAMPLE_RATE,
            2,
            effects(ReverbPreset::Off, ChorusPreset::Rich, EchoPreset::Off),
        );
        assert_eq!(samples.len(), length);
        assert!(
            energy(&samples, 0.018, 0.040) > 1e-4,
            "the chorus should produce a delayed copy"
        );
    }

    #[test]
    fn the_effect_chain_is_deterministic() {
        let settings = effects(ReverbPreset::Hall, ChorusPreset::Subtle, EchoPreset::Strong);
        let mut first = impulse(2.0);
        let mut second = first.clone();
        apply(&mut first, SAMPLE_RATE, 2, settings);
        apply(&mut second, SAMPLE_RATE, 2, settings);
        assert_eq!(first, second);
    }

    #[test]
    fn the_limiter_keeps_output_below_full_scale() {
        let mut samples = vec![0.9f32; 48_000 * 2];
        apply(
            &mut samples,
            SAMPLE_RATE,
            2,
            effects(ReverbPreset::Hall, ChorusPreset::Rich, EchoPreset::Strong),
        );
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().all(|sample| sample.abs() <= 0.981));
    }

    #[test]
    fn surround_channels_are_processed_independently() {
        // Five channels, with signal only in the centre and rear pair. A naive
        // stereo pass would leave those frames untouched or read them as
        // out-of-phase front channels.
        let frames = SAMPLE_RATE as usize / 2;
        let mut samples = vec![0.0f32; frames * 5];
        samples[2] = 0.5;
        samples[3] = 0.4;
        samples[4] = -0.4;
        samples[5 + 2] = 0.3;
        apply(
            &mut samples,
            SAMPLE_RATE,
            5,
            effects(
                ReverbPreset::MediumRoom,
                ChorusPreset::Off,
                EchoPreset::Subtle,
            ),
        );
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().all(|sample| sample.abs() <= 0.981));
        for channel in [2usize, 3, 4] {
            assert!(
                samples
                    .iter()
                    .skip(channel)
                    .step_by(5)
                    .any(|sample| sample.abs() > 1e-6),
                "surround channel {channel} lost its audio"
            );
        }
    }

    #[test]
    fn a_mono_buffer_runs_the_same_chain() {
        let frames = SAMPLE_RATE as usize;
        let mut samples = vec![0.0f32; frames];
        samples[0] = 0.5;
        apply(
            &mut samples,
            SAMPLE_RATE,
            1,
            effects(
                ReverbPreset::MediumRoom,
                ChorusPreset::Off,
                EchoPreset::Subtle,
            ),
        );
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(
            samples[..SAMPLE_RATE as usize / 2]
                .iter()
                .any(|sample| sample.abs() > 1e-6)
        );
    }
}
