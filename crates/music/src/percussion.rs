//! Optional percussion layer: foot taps or a simple drum kit.
//!
//! The layer is generated on the same dance-beat grid as the music and rendered
//! with small procedural voices. That keeps it independent of the soundfont:
//! the bundled Fluid R3 subset has no drum kit, and a user-supplied font may
//! not have one either. MIDI export still uses the General MIDI percussion
//! notes so the exported file opens with the expected kit.

use std::f32::consts::TAU;

use contra_core::Percussion;

use crate::render::{AudioBuffer, RenderRequest};
use crate::score::{Meter, NoteEvent};

/// General MIDI percussion notes used by the export and the procedural voices.
pub const KICK: u8 = 36;
pub const SIDE_STICK: u8 = 37;
pub const SNARE: u8 = 38;
pub const CLOSED_HAT: u8 = 42;

/// The percussion pattern for one 64-beat loop pass.
pub fn notes(loop_beats: f64, meter: Meter, style: Percussion) -> Vec<NoteEvent> {
    let mut notes = Vec::new();
    match style {
        Percussion::Off => {}
        Percussion::FootTaps => foot_taps(&mut notes, loop_beats),
        Percussion::Drumset => drumset(&mut notes, loop_beats, meter),
    }
    notes.sort_by(|a, b| {
        a.start_beat
            .total_cmp(&b.start_beat)
            .then(a.midi.cmp(&b.midi))
    });
    notes
}

/// The percussion pattern for the count-in, one note per count-in beat.
pub fn count_in_notes(style: Percussion, beats: usize) -> Vec<NoteEvent> {
    let mut notes = Vec::new();
    if !style.is_enabled() {
        return notes;
    }
    for beat in 0..beats {
        notes.push(NoteEvent {
            start_beat: beat as f64,
            duration_beats: 0.1,
            midi: if style == Percussion::Drumset {
                CLOSED_HAT
            } else {
                SIDE_STICK
            },
            velocity: if beat == 0 { 76 } else { 66 },
        });
    }
    if style == Percussion::Drumset && beats > 0 {
        notes.push(NoteEvent {
            start_beat: (beats - 1) as f64,
            duration_beats: 0.12,
            midi: KICK,
            velocity: 96,
        });
    }
    notes.sort_by(|a, b| {
        a.start_beat
            .total_cmp(&b.start_beat)
            .then(a.midi.cmp(&b.midi))
    });
    notes
}

/// One soft tap per dance beat, accented at the start of each bar.
fn foot_taps(notes: &mut Vec<NoteEvent>, loop_beats: f64) {
    let mut beat = 0.0;
    while beat < loop_beats - 1e-9 {
        let step = (beat * 2.0).round() as i64;
        notes.push(NoteEvent {
            start_beat: beat,
            duration_beats: 0.12,
            midi: SIDE_STICK,
            velocity: if step.rem_euclid(4) == 0 { 80 } else { 62 },
        });
        beat += 0.5;
    }
}

/// Kick, snare, and hi-hat on the dance-beat grid.
///
/// The hi-hat subdivides each dance beat in half for reels and in thirds for
/// compound meters, so a jig keeps its own lilt without moving the beat.
fn drumset(notes: &mut Vec<NoteEvent>, loop_beats: f64, meter: Meter) {
    let hat_step = if meter.is_compound() { 1.0 / 3.0 } else { 0.5 };
    let mut bar = 0.0;
    while bar < loop_beats - 1e-9 {
        let bar_end = (bar + 2.0).min(loop_beats);
        push(notes, bar, 0.15, KICK, 108, bar_end);
        push(notes, bar + 1.0, 0.12, SNARE, 96, bar_end);
        // A lighter kick on the second half of the bar gives the pattern a
        // little forward motion without turning it into a march.
        push(notes, bar + 1.5, 0.12, KICK, 74, bar_end);

        let mut hat = bar;
        while hat < bar_end - 1e-9 {
            let step = ((hat - bar) / hat_step).round() as i64;
            push(
                notes,
                hat,
                0.05,
                CLOSED_HAT,
                if step.rem_euclid(4) == 0 { 58 } else { 44 },
                bar_end,
            );
            hat += hat_step;
        }
        bar += 2.0;
    }
}

fn push(
    notes: &mut Vec<NoteEvent>,
    start_beat: f64,
    duration_beats: f64,
    midi: u8,
    velocity: u8,
    limit: f64,
) {
    if start_beat >= limit - 1e-9 {
        return;
    }
    notes.push(NoteEvent {
        start_beat,
        duration_beats: duration_beats.min(limit - start_beat),
        midi,
        velocity,
    });
}

/// Mix the percussion pattern into a rendered buffer.
///
/// Only the loop passes are covered: the count-in stays accompaniment-only and
/// the once-only ending keeps its own arrangement.
pub(crate) fn mix(buffer: &mut AudioBuffer, request: &RenderRequest) {
    if !request.percussion.is_enabled() {
        return;
    }
    let notes = notes(
        request.arrangement.loop_beats,
        request.arrangement.meter,
        request.percussion,
    );
    if notes.is_empty() {
        return;
    }
    let pass_samples = buffer.loop_pass_samples.max(1);
    let passes = ((buffer
        .loop_end_sample
        .saturating_sub(buffer.loop_start_sample))
        / pass_samples)
        .max(1) as u32;
    let volume = request.percussion_volume.clamp(0.0, 2.0);
    let count_in_beats = request.count_in_beats.round().clamp(0.0, 8.0) as usize;
    for note in count_in_notes(request.percussion, count_in_beats) {
        let start = (note.start_beat * buffer.samples_per_beat).round() as u64;
        let duration = (note.duration_beats * buffer.samples_per_beat)
            .round()
            .max(1.0) as usize;
        render_note(
            &mut buffer.samples,
            start as usize,
            duration,
            note.midi,
            note.velocity,
            volume,
            buffer.sample_rate,
        );
    }
    for pass in 0..passes {
        let offset = buffer.loop_start_sample + pass as u64 * pass_samples;
        for note in &notes {
            let start = offset + (note.start_beat * buffer.samples_per_beat).round() as u64;
            let duration = (note.duration_beats * buffer.samples_per_beat)
                .round()
                .max(1.0) as usize;
            render_note(
                &mut buffer.samples,
                start as usize,
                duration,
                note.midi,
                note.velocity,
                volume,
                buffer.sample_rate,
            );
        }
    }
    normalize(&mut buffer.samples);
}

#[allow(clippy::too_many_arguments)]
fn render_note(
    out: &mut [f32],
    start: usize,
    duration: usize,
    midi: u8,
    velocity: u8,
    volume: f32,
    sample_rate: u32,
) {
    let frames = out.len() / 2;
    if duration == 0 || start >= frames {
        return;
    }
    let duration = duration.min(frames - start);
    let sample_rate = sample_rate.max(8_000) as f32;
    let velocity = (velocity.max(1) as f32 / 127.0).clamp(0.05, 1.0);
    let gain = volume * velocity;
    // Deterministic noise keeps a render reproducible.
    let mut noise = Noise::new(start as u32 ^ (midi as u32) << 8);
    let mut phase = 0.0f32;
    let mut last_noise = 0.0f32;

    let (style_gain, decay) = match midi {
        KICK => (0.95, 24.0),
        SNARE => (0.55, 30.0),
        CLOSED_HAT => (0.30, 90.0),
        _ => (0.55, 38.0), // foot tap / side stick
    };
    let release_samples = (sample_rate * 0.005).max(1.0);
    let attack_samples = (sample_rate * 0.002).max(1.0);

    for index in 0..duration {
        let t = index as f32 / sample_rate;
        let attack = (index as f32 / attack_samples).min(1.0);
        let envelope = attack
            * (-t * decay).exp()
            * ((duration - index) as f32 / release_samples).clamp(0.0, 1.0);
        let sample = match midi {
            KICK => {
                // A short pitch drop from the beater to the drum body.
                let frequency = 50.0 + 90.0 * (-t * 35.0).exp();
                phase += frequency / sample_rate;
                phase -= phase.floor();
                (TAU * phase).sin() * 0.9 + noise.next() * 0.1
            }
            SNARE => noise.next() * 0.75 + (TAU * 190.0 * t).sin() * 0.25,
            CLOSED_HAT => {
                // First difference is a cheap high-pass for a hat.
                let value = noise.next();
                let high = value - last_noise * 0.92;
                last_noise = value;
                high * 0.7
            }
            _ => {
                // A dull, low foot tap with a little scuff.
                (TAU * 84.0 * t).sin() * 0.8 + noise.next() * 0.12
            }
        };
        let value = sample * gain * style_gain * envelope * 0.55;
        out[start * 2 + index * 2] += value;
        out[start * 2 + index * 2 + 1] += value;
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

    #[test]
    fn foot_taps_land_on_every_dance_beat() {
        let notes = notes(64.0, Meter::default(), Percussion::FootTaps);
        assert_eq!(notes.len(), 128);
        assert!(notes.iter().all(|note| note.midi == SIDE_STICK));
        assert!(notes.iter().all(|note| {
            (note.start_beat * 2.0).fract().abs() < 1e-9 && note.end_beat() <= 64.0 + 1e-6
        }));
    }

    #[test]
    fn count_in_has_a_tap_for_every_beat() {
        let taps = count_in_notes(Percussion::FootTaps, 4);
        assert_eq!(taps.len(), 4);
        assert!(taps.iter().all(|note| note.midi == SIDE_STICK));

        let drums = count_in_notes(Percussion::Drumset, 4);
        assert_eq!(
            drums.iter().filter(|note| note.midi == CLOSED_HAT).count(),
            4
        );
        assert!(
            drums
                .iter()
                .any(|note| note.midi == KICK && (note.start_beat - 3.0).abs() < 1e-9)
        );
        assert!(count_in_notes(Percussion::Off, 4).is_empty());
    }

    #[test]
    fn drumset_has_a_kick_snare_and_hat_pattern() {
        let notes = notes(64.0, Meter::default(), Percussion::Drumset);
        for midi in [KICK, SNARE, CLOSED_HAT] {
            assert!(
                notes.iter().any(|note| note.midi == midi),
                "drumset is missing note {midi}"
            );
        }
        assert!(
            notes
                .iter()
                .all(|note| note.start_beat >= 0.0 && note.end_beat() <= 64.0 + 1e-6)
        );
        assert!(
            notes
                .iter()
                .any(|note| note.midi == SNARE && (note.start_beat - 1.0).abs() < 1e-9),
            "the snare should answer on the second dance beat"
        );
    }

    #[test]
    fn compound_meter_subdivides_the_hat_in_thirds() {
        let meter = Meter::parse("6/8").expect("6/8");
        let notes = notes(8.0, meter, Percussion::Drumset);
        let hats = notes
            .iter()
            .filter(|note| note.midi == CLOSED_HAT)
            .map(|note| note.start_beat)
            .collect::<Vec<_>>();
        assert!(hats.iter().any(|beat| (beat - 1.0 / 3.0).abs() < 1e-9));
    }

    #[test]
    fn rendering_percussion_changes_the_mix() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let mut request = RenderRequest {
            arrangement,
            bpm: 240.0,
            melody: contra_core::MelodyInstrument::Fiddle,
            accompaniment: contra_core::AccompanimentInstrument::Piano,
            accompaniment_rhythm: contra_core::AccompanimentRhythm::Auto,
            second_accompaniment: contra_core::SecondAccompaniment::Accordion,
            percussion: Percussion::FootTaps,
            melody_volume: 0.75,
            accompaniment_volume: 0.65,
            percussion_volume: 0.5,
            count_in_beats: 4.0,
            sample_rate: 24_000,
            variation_passes: 1,
            ending: false,
        };
        let with = crate::render::render(&request, None).expect("render trio");
        request.percussion = Percussion::Off;
        let without = crate::render::render(&request, None).expect("render trio");
        assert_eq!(with.frames(), without.frames());
        let difference = with
            .samples
            .iter()
            .zip(&without.samples)
            .fold(0.0f32, |worst, (a, b)| worst.max((a - b).abs()));
        assert!(difference > 1e-3, "percussion did not change the mix");
        assert!(with.samples.iter().all(|sample| sample.is_finite()));
        assert!(with.samples.iter().all(|sample| sample.abs() <= 1.0));
    }
}
