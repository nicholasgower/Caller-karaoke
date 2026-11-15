//! Small built-in synthesizer.
//!
//! This is not a sampled instrument library. It provides recognisable fiddle,
//! accordion, piano, and guitar timbres from procedural oscillators, and acts as
//! the fallback when the bundled Fluid R3 GM subset or a user-supplied SF2
//! cannot be loaded (see `soundfont.rs`).

use std::f32::consts::TAU;

use contra_core::{AccompanimentInstrument, MelodyInstrument};

use crate::render::{AudioBuffer, RenderRequest};
use crate::score::NoteEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Timbre {
    Fiddle,
    Accordion,
    Piano,
    Guitar,
}

pub fn render_builtin(request: &RenderRequest) -> AudioBuffer {
    let sample_rate = request.sample_rate.max(8_000);
    let beat_seconds = 120.0 / request.bpm.max(1.0);
    let samples_per_beat = beat_seconds * sample_rate as f64;
    let count_in_samples = (request.count_in_beats.max(0.0) * samples_per_beat).round() as u64;
    let pass_samples = (request.arrangement.loop_beats.max(1.0) * samples_per_beat).round() as u64;
    let passes = request.variation_passes.max(1);
    let loop_samples = pass_samples * passes as u64;
    let total_samples = count_in_samples + loop_samples;

    let mut samples = vec![0.0f32; total_samples as usize * 2];

    render_count_in(
        &mut samples,
        request,
        count_in_samples,
        samples_per_beat,
        sample_rate,
    );

    let melody_voice = melody_timbre(request.melody);
    let accompaniment_timbre = accompaniment_timbre(request.accompaniment);
    for pass in 0..passes {
        let offset = count_in_samples as i64 + pass as i64 * pass_samples as i64;
        if pass == 0 {
            render_notes(
                &mut samples,
                &request.arrangement.pickup,
                melody_voice,
                request.melody_volume,
                offset,
                samples_per_beat,
                sample_rate,
                -0.10,
            );
        }
        render_notes(
            &mut samples,
            &request.arrangement.melody,
            melody_voice,
            request.melody_volume,
            offset,
            samples_per_beat,
            sample_rate,
            -0.10,
        );

        let accompaniment = request
            .arrangement
            .accompaniment_notes_for_pass_with_rhythm(
                request.accompaniment,
                request.accompaniment_rhythm,
                pass,
            );
        render_notes(
            &mut samples,
            &accompaniment,
            accompaniment_timbre,
            request.accompaniment_volume,
            offset,
            samples_per_beat,
            sample_rate,
            0.10,
        );

        // Second accompaniment: a countermelody on a melody instrument, mixed a
        // little quieter and to the other side so the trio stays legible.
        if let Some(second) = request.second_accompaniment.instrument() {
            let countermelody = request.arrangement.countermelody_notes_for_pass(pass);
            render_notes(
                &mut samples,
                &countermelody,
                melody_timbre(second),
                request.accompaniment_volume * 0.8,
                offset,
                samples_per_beat,
                sample_rate,
                0.30,
            );
        }
    }

    apply_fades(&mut samples, sample_rate, total_samples as usize);
    normalize(&mut samples);

    AudioBuffer {
        samples,
        sample_rate,
        channels: 2,
        count_in_samples,
        loop_start_sample: count_in_samples,
        loop_end_sample: total_samples,
        loop_pass_samples: pass_samples,
        samples_per_beat,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_count_in(
    samples: &mut [f32],
    request: &RenderRequest,
    count_in_samples: u64,
    samples_per_beat: f64,
    sample_rate: u32,
) {
    if count_in_samples == 0 {
        return;
    }
    let chord = request.arrangement.count_in_chord();
    let timbre = accompaniment_timbre(request.accompaniment);
    let chord_notes = chord.voicing(55);
    let duration = (0.40 * samples_per_beat) as usize;
    let count = request.count_in_beats.round().clamp(0.0, 4.0) as usize;
    for beat in 0..count {
        let start = (beat as f64 * samples_per_beat).round() as usize;
        render_note(
            samples,
            start,
            duration,
            chord.bass_midi(),
            62,
            timbre,
            sample_rate as f32,
            request.accompaniment_volume * 0.8,
            0.10,
        );
        for midi in &chord_notes {
            render_note(
                samples,
                start,
                duration,
                *midi,
                55,
                timbre,
                sample_rate as f32,
                request.accompaniment_volume * 0.6,
                0.10,
            );
        }
    }
}

fn melody_timbre(instrument: MelodyInstrument) -> Timbre {
    match instrument {
        MelodyInstrument::Fiddle => Timbre::Fiddle,
        MelodyInstrument::Accordion => Timbre::Accordion,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_notes(
    samples: &mut [f32],
    notes: &[NoteEvent],
    timbre: Timbre,
    volume: f32,
    offset: i64,
    samples_per_beat: f64,
    sample_rate: u32,
    pan: f32,
) {
    for note in notes {
        let start = offset + (note.start_beat * samples_per_beat).round() as i64;
        let duration = (note.duration_beats * samples_per_beat).round().max(1.0) as usize;
        if start < 0 {
            continue;
        }
        render_note(
            samples,
            start as usize,
            duration,
            note.midi,
            note.velocity,
            timbre,
            sample_rate as f32,
            volume,
            pan,
        );
    }
}

fn accompaniment_timbre(instrument: AccompanimentInstrument) -> Timbre {
    match instrument {
        AccompanimentInstrument::Piano => Timbre::Piano,
        AccompanimentInstrument::Guitar => Timbre::Guitar,
        AccompanimentInstrument::Accordion => Timbre::Accordion,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_note(
    out: &mut [f32],
    start: usize,
    duration: usize,
    midi: u8,
    velocity: u8,
    timbre: Timbre,
    sample_rate: f32,
    gain: f32,
    pan: f32,
) {
    if duration == 0 || start >= out.len() / 2 {
        return;
    }
    let velocity = (velocity.max(1) as f32 / 127.0).clamp(0.05, 1.0);
    let gain = gain * velocity;
    let freq = midi_frequency(midi);
    let pan = pan.clamp(-1.0, 1.0);
    let left_gain = (0.5 * (1.0 - pan)).sqrt();
    let right_gain = (0.5 * (1.0 + pan)).sqrt();
    let duration = duration.min(out.len() / 2 - start);

    match timbre {
        Timbre::Fiddle => render_fiddle(
            out,
            start,
            duration,
            freq,
            gain,
            left_gain,
            right_gain,
            sample_rate,
        ),
        Timbre::Accordion => render_accordion(
            out,
            start,
            duration,
            freq,
            gain,
            left_gain,
            right_gain,
            sample_rate,
        ),
        Timbre::Piano => render_piano(
            out,
            start,
            duration,
            freq,
            gain,
            left_gain,
            right_gain,
            sample_rate,
        ),
        Timbre::Guitar => render_guitar(
            out,
            start,
            duration,
            freq,
            gain,
            left_gain,
            right_gain,
            sample_rate,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_fiddle(
    out: &mut [f32],
    start: usize,
    duration: usize,
    freq: f32,
    gain: f32,
    left_gain: f32,
    right_gain: f32,
    sample_rate: f32,
) {
    let mut phase = 0.0f32;
    let mut lowpass = 0.0f32;
    for index in 0..duration {
        let t = index as f32 / sample_rate;
        let vibrato = 1.0 + 0.006 * (TAU * 5.2 * t).sin() * (t * 4.0).min(1.0);
        phase += freq * vibrato / sample_rate;
        phase -= phase.floor();
        let mut saw = 2.0 * phase - 1.0;
        // A gentle body resonance and bow noise.
        lowpass += 0.20 * (saw - lowpass);
        saw = lowpass + 0.08 * (TAU * freq * 0.5 * t).sin();
        let envelope = adsr(
            index as f32 / sample_rate,
            duration as f32 / sample_rate,
            0.045,
            0.06,
            0.82,
            0.10,
        );
        let sample = saw * gain * envelope * 0.55;
        out[(start + index) * 2] += sample * left_gain;
        out[(start + index) * 2 + 1] += sample * right_gain;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_accordion(
    out: &mut [f32],
    start: usize,
    duration: usize,
    freq: f32,
    gain: f32,
    left_gain: f32,
    right_gain: f32,
    sample_rate: f32,
) {
    let mut phase_a = 0.0f32;
    let mut phase_b = 0.25f32;
    for index in 0..duration {
        let t = index as f32 / sample_rate;
        phase_a += freq * 0.996 / sample_rate;
        phase_b += freq * 1.004 / sample_rate;
        phase_a -= phase_a.floor();
        phase_b -= phase_b.floor();
        let saw = ((2.0 * phase_a - 1.0) * 0.55) + ((2.0 * phase_b - 1.0) * 0.45);
        let tremolo = 0.85 + 0.15 * (TAU * 5.5 * t).sin();
        let envelope = adsr(
            index as f32 / sample_rate,
            duration as f32 / sample_rate,
            0.02,
            0.04,
            0.85,
            0.08,
        );
        let sample = saw * gain * envelope * tremolo * 0.45;
        out[(start + index) * 2] += sample * left_gain;
        out[(start + index) * 2 + 1] += sample * right_gain;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_piano(
    out: &mut [f32],
    start: usize,
    duration: usize,
    freq: f32,
    gain: f32,
    left_gain: f32,
    right_gain: f32,
    sample_rate: f32,
) {
    let partials = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
    for index in 0..duration {
        let t = index as f32 / sample_rate;
        let attack = (t / 0.004).min(1.0);
        let mut sample = 0.0f32;
        for (partial_index, partial) in partials.iter().enumerate() {
            let n = *partial;
            let amplitude = 1.0 / (1.6 + partial_index as f32).powf(1.25);
            let inharmonic = 1.0 + 0.0004 * n * n;
            let decay = (-t * (2.2 + partial_index as f32 * 0.55)).exp();
            sample += (TAU * freq * n * inharmonic * t).sin() * amplitude * decay;
        }
        let release = ((duration as f32 / sample_rate - t) / 0.08).clamp(0.0, 1.0);
        let sample = sample * gain * attack * release * 0.35;
        out[(start + index) * 2] += sample * left_gain;
        out[(start + index) * 2 + 1] += sample * right_gain;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_guitar(
    out: &mut [f32],
    start: usize,
    duration: usize,
    freq: f32,
    gain: f32,
    left_gain: f32,
    right_gain: f32,
    sample_rate: f32,
) {
    let delay_len = ((sample_rate / freq).round() as usize).clamp(4, 4096);
    let mut delay = vec![0.0f32; delay_len];
    // Deterministic pluck excitation (xorshift).
    let mut state = 0x1234_5678u32 ^ freq.to_bits();
    for value in &mut delay {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        *value = ((state as f32 / u32::MAX as f32) * 2.0 - 1.0) * 0.4;
    }
    let mut index = 0usize;
    for sample_index in 0..duration {
        let current = delay[index];
        let next = (current + delay[(index + 1) % delay_len]) * 0.5 * 0.997;
        delay[index] = next;
        index = (index + 1) % delay_len;
        let envelope = ((duration as f32 / sample_rate - sample_index as f32 / sample_rate) / 0.12)
            .clamp(0.0, 1.0);
        let sample = current * gain * envelope * 0.55;
        out[(start + sample_index) * 2] += sample * left_gain;
        out[(start + sample_index) * 2 + 1] += sample * right_gain;
    }
}

fn adsr(t: f32, duration: f32, attack: f32, decay: f32, sustain: f32, release: f32) -> f32 {
    if t < attack {
        t / attack
    } else if t < attack + decay {
        let progress = (t - attack) / decay;
        1.0 + (sustain - 1.0) * progress
    } else if t > duration - release {
        ((duration - t) / release).clamp(0.0, 1.0) * sustain
    } else {
        sustain
    }
}

fn midi_frequency(midi: u8) -> f32 {
    440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0)
}

fn apply_fades(samples: &mut [f32], sample_rate: u32, frames: usize) {
    let fade = ((sample_rate as f32 * 0.004).round() as usize).max(1);
    for index in 0..fade.min(frames) {
        let gain = index as f32 / fade as f32;
        samples[index * 2] *= gain;
        samples[index * 2 + 1] *= gain;
        let tail = frames - 1 - index;
        samples[tail * 2] *= gain;
        samples[tail * 2 + 1] *= gain;
    }
}

fn normalize(samples: &mut [f32]) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abc::parse_abc;
    use crate::arrange::Arrangement;

    const TUNE: &str = "X:1\nT:Test\nM:4/4\nL:1/8\nK:D\n\"D\"A2 A2 A2 A2 | \"G\"B2 B2 B2 B2|\n";

    fn request() -> RenderRequest {
        let score = parse_abc(TUNE).expect("parse");
        RenderRequest {
            arrangement: Arrangement::from_score(&score, 64.0),
            bpm: 240.0,
            melody: MelodyInstrument::Fiddle,
            accompaniment: AccompanimentInstrument::Piano,
            accompaniment_rhythm: contra_core::AccompanimentRhythm::Auto,
            second_accompaniment: contra_core::SecondAccompaniment::Accordion,
            melody_volume: 0.75,
            accompaniment_volume: 0.65,
            count_in_beats: 3.0,
            sample_rate: 24_000,
            variation_passes: 1,
        }
    }

    #[test]
    fn renders_count_in_and_loop() {
        let buffer = render_builtin(&request());
        assert_eq!(buffer.samples.len() % 2, 0);
        assert!(buffer.count_in_samples > 0);
        assert!(buffer.frames() > buffer.count_in_samples as usize);
        assert!(buffer.samples.iter().any(|sample| sample.abs() > 0.001));
        assert!(buffer.samples.iter().all(|sample| sample.is_finite()));
        assert!(buffer.samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn buffer_length_matches_tempo() {
        let buffer = render_builtin(&request());
        // 3 count-in beats + 64 loop beats at 240 BPM = 33.5 s.
        let expected = 33.5 * 24_000.0;
        assert!((buffer.frames() as f64 - expected).abs() < 2.0);
    }

    #[test]
    fn variation_passes_extend_the_buffer_but_not_the_beat_clock() {
        let mut request = request();
        request.variation_passes = crate::VARIATION_PASSES;
        let buffer = render_builtin(&request);

        let pass_seconds = 64.0 * 0.5;
        let expected = (3.0 * 0.5 + pass_seconds * crate::VARIATION_PASSES as f64) * 24_000.0;
        assert!((buffer.frames() as f64 - expected).abs() < 2.0);
        assert_eq!(
            buffer.loop_end_sample - buffer.loop_start_sample,
            buffer.loop_pass_samples * crate::VARIATION_PASSES as u64
        );

        // The transport wraps every pass, so later passes still read as 0..64.
        let beat = buffer.beat_at_sample(buffer.loop_start_sample + buffer.loop_pass_samples + 1);
        assert!((0.0..0.1).contains(&beat), "beat {beat}");
        assert!(buffer.samples.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn second_accompaniment_adds_a_third_voice() {
        let mut trio = request();
        let with = render_builtin(&trio);
        trio.second_accompaniment = contra_core::SecondAccompaniment::Off;
        let without = render_builtin(&trio);

        assert_eq!(with.frames(), without.frames());
        let difference = with
            .samples
            .iter()
            .zip(&without.samples)
            .fold(0.0f32, |worst, (a, b)| worst.max((a - b).abs()));
        assert!(
            difference > 1e-3,
            "the second accompaniment should change the mix, not {difference}"
        );
        assert!(with.samples.iter().all(|sample| sample.is_finite()));
        assert!(with.samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn all_instruments_render() {
        for melody in [MelodyInstrument::Fiddle, MelodyInstrument::Accordion] {
            for accompaniment in [
                AccompanimentInstrument::Piano,
                AccompanimentInstrument::Guitar,
                AccompanimentInstrument::Accordion,
            ] {
                let mut request = request();
                request.melody = melody;
                request.accompaniment = accompaniment;
                let buffer = render_builtin(&request);
                assert!(buffer.samples.iter().any(|sample| sample.abs() > 0.001));
            }
        }
    }
}
