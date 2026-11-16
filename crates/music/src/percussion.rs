//! Optional percussion layer: foot taps or a simple drum kit.
//!
//! The layer is generated on the same dance-beat grid as the music and rendered
//! with small procedural voices. That keeps it independent of the soundfont:
//! the bundled Fluid R3 subset has no drum kit, and a user-supplied font may
//! not have one either. MIDI export still uses the General MIDI percussion
//! notes so the exported file opens with the expected kit.

use std::f32::consts::TAU;

use contra_core::{Percussion, PercussionRhythm};

use crate::render::{AudioBuffer, RenderRequest};
use crate::score::{Meter, NoteEvent};

/// General MIDI percussion notes used by the export and the procedural voices.
pub const KICK: u8 = 36;
pub const SIDE_STICK: u8 = 37;
pub const SNARE: u8 = 38;
pub const CLOSED_HAT: u8 = 42;

/// The percussion pattern for one 64-beat loop pass.
pub fn notes(
    loop_beats: f64,
    meter: Meter,
    style: Percussion,
    rhythm: PercussionRhythm,
) -> Vec<NoteEvent> {
    let mut notes = Vec::new();
    match style {
        Percussion::Off => {}
        Percussion::FootTaps => foot_taps(&mut notes, loop_beats, rhythm),
        Percussion::Drumset => drumset(&mut notes, loop_beats, meter, rhythm),
    }
    notes.sort_by(|a, b| {
        a.start_beat
            .total_cmp(&b.start_beat)
            .then(a.midi.cmp(&b.midi))
    });
    notes
}

/// Resolve `Auto` to the pattern that suits the selected style.
fn resolve(style: Percussion, rhythm: PercussionRhythm) -> PercussionRhythm {
    match rhythm {
        PercussionRhythm::Auto => match style {
            Percussion::FootTaps => PercussionRhythm::Gallop,
            Percussion::Drumset => PercussionRhythm::Basic,
            Percussion::Off => PercussionRhythm::Auto,
        },
        explicit => explicit,
    }
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

/// Foot taps in the selected rhythm.
fn foot_taps(notes: &mut Vec<NoteEvent>, loop_beats: f64, rhythm: PercussionRhythm) {
    let rhythm = resolve(Percussion::FootTaps, rhythm);
    match rhythm {
        // `resolve` never leaves Auto for foot taps.
        PercussionRhythm::Auto | PercussionRhythm::Basic | PercussionRhythm::Beat => {
            let mut beat = 0.0;
            while beat < loop_beats - 1e-9 {
                let step = (beat * 2.0).round() as i64;
                push(
                    notes,
                    beat,
                    0.12,
                    SIDE_STICK,
                    if step.rem_euclid(4) == 0 { 92 } else { 76 },
                    loop_beats,
                );
                beat += 0.5;
            }
        }
        PercussionRhythm::Offbeat => {
            let mut beat = 0.25;
            while beat < loop_beats - 1e-9 {
                let step = (beat * 2.0).round() as i64;
                push(
                    notes,
                    beat,
                    0.1,
                    SIDE_STICK,
                    if step.rem_euclid(4) == 0 { 78 } else { 62 },
                    loop_beats,
                );
                beat += 0.5;
            }
        }
        PercussionRhythm::HalfTime => {
            let mut bar = 0.0;
            while bar < loop_beats - 1e-9 {
                push(notes, bar, 0.16, SIDE_STICK, 94, loop_beats);
                bar += 2.0;
            }
        }
        PercussionRhythm::DoubleTime => {
            let mut beat = 0.0;
            while beat < loop_beats - 1e-9 {
                let step = (beat * 4.0).round() as i64;
                push(
                    notes,
                    beat,
                    0.08,
                    SIDE_STICK,
                    if step.rem_euclid(8) == 0 {
                        88
                    } else if step.rem_euclid(2) == 0 {
                        72
                    } else {
                        58
                    },
                    loop_beats,
                );
                beat += 0.25;
            }
        }
        PercussionRhythm::Shuffle => {
            let mut bar = 0.0;
            while bar < loop_beats - 1e-9 {
                for offset in [0.0, 0.67, 1.0, 1.67] {
                    push(
                        notes,
                        bar + offset,
                        0.12,
                        SIDE_STICK,
                        if offset == 0.0 || offset == 1.0 {
                            86
                        } else {
                            66
                        },
                        loop_beats,
                    );
                }
                bar += 2.0;
            }
        }
        PercussionRhythm::Gallop => {
            // One and a: an eighth note and two sixteenths. Foot taps play one
            // group per dance beat, so the pattern stays light under the tune.
            let mut beat = 0.0;
            while beat < loop_beats - 1e-9 {
                for (index, offset) in [0.0, 0.50, 0.75].into_iter().enumerate() {
                    push(
                        notes,
                        beat + offset,
                        if index == 0 { 0.18 } else { 0.08 },
                        SIDE_STICK,
                        match index {
                            0 => 92,
                            1 => 72,
                            _ => 60,
                        },
                        loop_beats,
                    );
                }
                beat += 1.0;
            }
        }
        PercussionRhythm::Triplet => {
            let mut beat = 0.0;
            while beat < loop_beats - 1e-9 {
                for (index, offset) in [0.0, 1.0 / 3.0, 2.0 / 3.0].into_iter().enumerate() {
                    push(
                        notes,
                        beat + offset,
                        if index == 0 { 0.12 } else { 0.08 },
                        SIDE_STICK,
                        match index {
                            0 => 92,
                            1 => 72,
                            _ => 60,
                        },
                        loop_beats,
                    );
                }
                beat += 1.0;
            }
        }
        PercussionRhythm::Train => {
            let mut bar = 0.0;
            while bar < loop_beats - 1e-9 {
                for (index, offset) in [0.0, 0.5, 1.0, 1.5].into_iter().enumerate() {
                    push(
                        notes,
                        bar + offset,
                        0.1,
                        SIDE_STICK,
                        if index % 2 == 0 { 84 } else { 66 },
                        loop_beats,
                    );
                }
                bar += 2.0;
            }
        }
    }
}

/// Kick, snare, and hi-hat in the selected rhythm.
///
/// The hi-hat subdivides each dance beat in half for reels and in thirds for
/// compound meters, so a jig keeps its own lilt without moving the beat.
fn drumset(notes: &mut Vec<NoteEvent>, loop_beats: f64, meter: Meter, rhythm: PercussionRhythm) {
    let rhythm = resolve(Percussion::Drumset, rhythm);
    let hat_step = if meter.is_compound() { 1.0 / 3.0 } else { 0.5 };
    let mut bar = 0.0;
    while bar < loop_beats - 1e-9 {
        let bar_end = (bar + 2.0).min(loop_beats);
        match rhythm {
            // `resolve` never leaves Auto for the drumset.
            PercussionRhythm::Auto | PercussionRhythm::Basic => {
                push(notes, bar, 0.15, KICK, 108, bar_end);
                push(notes, bar + 1.0, 0.12, SNARE, 96, bar_end);
                push(notes, bar + 1.5, 0.12, KICK, 74, bar_end);
                hats(notes, bar, bar_end, hat_step, 58, 44);
            }
            PercussionRhythm::Beat => {
                push(notes, bar, 0.15, KICK, 108, bar_end);
                push(notes, bar + 1.0, 0.12, KICK, 92, bar_end);
                push(notes, bar + 1.0, 0.12, SNARE, 88, bar_end);
                hats(notes, bar, bar_end, hat_step, 56, 44);
            }
            PercussionRhythm::Offbeat => {
                push(notes, bar, 0.15, KICK, 104, bar_end);
                push(notes, bar + 0.5, 0.1, SNARE, 90, bar_end);
                push(notes, bar + 1.5, 0.1, SNARE, 90, bar_end);
                hats(notes, bar, bar_end, hat_step, 48, 38);
            }
            PercussionRhythm::HalfTime => {
                push(notes, bar, 0.18, KICK, 108, bar_end);
                push(notes, bar + 1.0, 0.12, SNARE, 96, bar_end);
                hats(notes, bar, bar_end, hat_step * 2.0, 54, 42);
            }
            PercussionRhythm::DoubleTime => {
                push(notes, bar, 0.15, KICK, 104, bar_end);
                push(notes, bar + 1.0, 0.12, KICK, 92, bar_end);
                push(notes, bar + 0.5, 0.1, SNARE, 88, bar_end);
                push(notes, bar + 1.5, 0.1, SNARE, 88, bar_end);
                hats(notes, bar, bar_end, hat_step * 0.5, 52, 40);
            }
            PercussionRhythm::Shuffle => {
                push(notes, bar, 0.15, KICK, 104, bar_end);
                push(notes, bar + 0.67, 0.1, SNARE, 92, bar_end);
                push(notes, bar + 1.0, 0.12, KICK, 86, bar_end);
                push(notes, bar + 1.67, 0.1, SNARE, 92, bar_end);
                for (offset, velocity) in [(0.0, 54), (0.67, 42), (1.0, 50), (1.67, 42)] {
                    push(notes, bar + offset, 0.05, CLOSED_HAT, velocity, bar_end);
                }
            }
            PercussionRhythm::Gallop => {
                let mut pulse = bar;
                while pulse < bar_end - 1e-9 {
                    push(notes, pulse, 0.14, KICK, 106, bar_end);
                    push(notes, pulse + 0.25, 0.08, SNARE, 88, bar_end);
                    push(notes, pulse + 0.375, 0.08, SNARE, 92, bar_end);
                    for (index, offset) in [0.0, 0.25, 0.375].into_iter().enumerate() {
                        push(
                            notes,
                            pulse + offset,
                            0.04,
                            CLOSED_HAT,
                            if index == 0 { 52 } else { 40 },
                            bar_end,
                        );
                    }
                    pulse += 0.5;
                }
            }
            PercussionRhythm::Triplet => {
                for base in [0.0, 1.0] {
                    push(notes, bar + base, 0.14, KICK, 106, bar_end);
                    push(notes, bar + base + 1.0 / 3.0, 0.08, SNARE, 88, bar_end);
                    push(notes, bar + base + 2.0 / 3.0, 0.08, SNARE, 92, bar_end);
                    for (index, offset) in [0.0, 1.0 / 3.0, 2.0 / 3.0].into_iter().enumerate() {
                        push(
                            notes,
                            bar + base + offset,
                            0.04,
                            CLOSED_HAT,
                            if index == 0 { 52 } else { 40 },
                            bar_end,
                        );
                    }
                }
            }
            PercussionRhythm::Train => {
                push(notes, bar, 0.15, KICK, 104, bar_end);
                push(notes, bar + 0.5, 0.1, SNARE, 92, bar_end);
                push(notes, bar + 1.0, 0.12, KICK, 96, bar_end);
                push(notes, bar + 1.5, 0.1, SNARE, 92, bar_end);
                hats(notes, bar, bar_end, hat_step, 46, 38);
            }
        }
        bar += 2.0;
    }
}

fn hats(notes: &mut Vec<NoteEvent>, start: f64, limit: f64, step: f64, accent: u8, normal: u8) {
    let mut hat = start;
    while hat < limit - 1e-9 {
        let index = ((hat - start) / step).round() as i64;
        push(
            notes,
            hat,
            0.05,
            CLOSED_HAT,
            if index.rem_euclid(4) == 0 {
                accent
            } else {
                normal
            },
            limit,
        );
        hat += step;
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
/// The count-in gets one clear note per beat, the loop passes use the selected
/// rhythm, and the once-only ending keeps its own arrangement.
pub(crate) fn mix(buffer: &mut AudioBuffer, request: &RenderRequest) {
    if !request.percussion.is_enabled() {
        return;
    }
    let notes = notes(
        request.arrangement.loop_beats,
        request.arrangement.meter,
        request.percussion,
        request.percussion_rhythm,
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
    let voices = crate::copies::voices(0.0, request.copies.percussion_count(), buffer.sample_rate);
    for note in count_in_notes(request.percussion, count_in_beats) {
        let beat_start = (note.start_beat * buffer.samples_per_beat).round() as u64;
        let duration = (note.duration_beats * buffer.samples_per_beat)
            .round()
            .max(1.0) as usize;
        for voice in &voices {
            render_note(
                &mut buffer.samples,
                (beat_start + voice.delay_samples) as usize,
                duration,
                note.midi,
                note.velocity,
                volume * voice.gain,
                buffer.sample_rate,
                voice.pan,
            );
        }
    }
    for pass in 0..passes {
        let offset = buffer.loop_start_sample + pass as u64 * pass_samples;
        for note in &notes {
            let start = offset + (note.start_beat * buffer.samples_per_beat).round() as u64;
            let duration = (note.duration_beats * buffer.samples_per_beat)
                .round()
                .max(1.0) as usize;
            for voice in &voices {
                render_note(
                    &mut buffer.samples,
                    (start + voice.delay_samples) as usize,
                    duration,
                    note.midi,
                    note.velocity,
                    volume * voice.gain,
                    buffer.sample_rate,
                    voice.pan,
                );
            }
        }
    }
    normalize(&mut buffer.samples);
}

/// Pan gains for the percussion voices.
///
/// Linear rather than constant-power so that a centred, single player keeps
/// exactly the level it had before copies existed. A panned copy only ever
/// lowers one side, which cannot add headroom pressure.
fn pan_gains(pan: f32) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    (1.0 - pan.max(0.0), 1.0 + pan.min(0.0))
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
    pan: f32,
) {
    let frames = out.len() / 2;
    if duration == 0 || start >= frames {
        return;
    }
    let duration = duration.min(frames - start);
    let (left_gain, right_gain) = pan_gains(pan);
    let sample_rate = sample_rate.max(8_000) as f32;
    let velocity = (velocity.max(1) as f32 / 127.0).clamp(0.05, 1.0);
    let gain = volume * velocity;
    // Deterministic noise keeps a render reproducible.
    let mut noise = Noise::new(start as u32 ^ (midi as u32) << 8);
    let mut phase = 0.0f32;
    let mut last_noise = 0.0f32;

    let (style_gain, decay) = match midi {
        KICK => (0.95, 24.0),
        SNARE => (0.65, 30.0),
        CLOSED_HAT => (0.35, 90.0),
        // Foot tap / side stick: a bright enough body to carry on small
        // speakers, with a short scuff for the attack.
        SIDE_STICK => (1.25, 42.0),
        _ => (1.0, 45.0),
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
            SIDE_STICK => {
                // A foot on a wooden floor: a short mid body, a low thump, and
                // a scuff. The midrange keeps it audible on small speakers.
                let body = (TAU * 190.0 * t).sin() * 0.55 + (TAU * 95.0 * t).sin() * 0.35;
                let scuff = noise.next() * (-t * 260.0).exp() * 0.6;
                body + scuff
            }
            _ => {
                // A fallback tap for any other percussion note.
                (TAU * 190.0 * t).sin() * 0.6 + noise.next() * 0.15
            }
        };
        let value = sample * gain * style_gain * envelope * 0.55;
        out[start * 2 + index * 2] += value * left_gain;
        out[start * 2 + index * 2 + 1] += value * right_gain;
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
        let notes = notes(
            64.0,
            Meter::default(),
            Percussion::FootTaps,
            PercussionRhythm::Beat,
        );
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
        let notes = notes(
            64.0,
            Meter::default(),
            Percussion::Drumset,
            PercussionRhythm::Auto,
        );
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
        let notes = notes(8.0, meter, Percussion::Drumset, PercussionRhythm::Auto);
        let hats = notes
            .iter()
            .filter(|note| note.midi == CLOSED_HAT)
            .map(|note| note.start_beat)
            .collect::<Vec<_>>();
        assert!(hats.iter().any(|beat| (beat - 1.0 / 3.0).abs() < 1e-9));
    }

    #[test]
    fn auto_rhythm_matches_each_styles_basic_pattern() {
        let meter = Meter::default();
        assert_eq!(
            notes(64.0, meter, Percussion::FootTaps, PercussionRhythm::Auto),
            notes(64.0, meter, Percussion::FootTaps, PercussionRhythm::Gallop)
        );
        assert_eq!(
            notes(64.0, meter, Percussion::Drumset, PercussionRhythm::Auto),
            notes(64.0, meter, Percussion::Drumset, PercussionRhythm::Basic)
        );
    }

    #[test]
    fn gallop_is_the_default_foot_tap_pattern() {
        let notes = notes(
            2.0,
            Meter::default(),
            Percussion::FootTaps,
            PercussionRhythm::Gallop,
        );
        let positions = notes.iter().map(|note| note.start_beat).collect::<Vec<_>>();
        let expected = [0.0, 0.5, 0.75, 1.0, 1.5, 1.75];
        assert_eq!(positions.len(), expected.len());
        for (position, expected) in positions.iter().zip(expected) {
            assert!(
                (position - expected).abs() < 1e-9,
                "gallop tap at {position}, expected {expected}"
            );
        }
    }

    #[test]
    fn triplet_is_three_even_taps_per_dance_beat() {
        let notes = notes(
            2.0,
            Meter::default(),
            Percussion::FootTaps,
            PercussionRhythm::Triplet,
        );
        let positions = notes.iter().map(|note| note.start_beat).collect::<Vec<_>>();
        let expected = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0, 4.0 / 3.0, 5.0 / 3.0];
        assert_eq!(positions.len(), expected.len());
        for (position, expected) in positions.iter().zip(expected) {
            assert!((position - expected).abs() < 1e-9);
        }
    }

    #[test]
    fn every_rhythm_stays_inside_the_loop() {
        for style in [Percussion::FootTaps, Percussion::Drumset] {
            for rhythm in PercussionRhythm::ALL {
                let notes = notes(64.0, Meter::default(), style, rhythm);
                assert!(!notes.is_empty(), "{style:?} with {rhythm:?} must play");
                assert!(
                    notes
                        .iter()
                        .all(|note| { note.start_beat >= 0.0 && note.end_beat() <= 64.0 + 1e-6 })
                );
            }
        }
    }

    #[test]
    fn rhythms_change_the_density_of_foot_taps() {
        let meter = Meter::default();
        let beat = notes(64.0, meter, Percussion::FootTaps, PercussionRhythm::Beat);
        let offbeat = notes(64.0, meter, Percussion::FootTaps, PercussionRhythm::Offbeat);
        let half = notes(
            64.0,
            meter,
            Percussion::FootTaps,
            PercussionRhythm::HalfTime,
        );
        let double = notes(
            64.0,
            meter,
            Percussion::FootTaps,
            PercussionRhythm::DoubleTime,
        );
        assert!(half.len() < beat.len());
        assert!(beat.len() < double.len());
        assert!(offbeat.iter().all(|note| {
            let position = (note.start_beat * 4.0).round() as i64;
            position.rem_euclid(2) == 1
        }));
    }

    #[test]
    fn foot_taps_are_audible_alone() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let request = RenderRequest {
            arrangement,
            bpm: 240.0,
            melody: contra_core::MelodyInstrument::Fiddle,
            accompaniment: contra_core::AccompanimentInstrument::Piano,
            accompaniment_rhythm: contra_core::AccompanimentRhythm::Auto,
            second_accompaniment: contra_core::Countermelody::Off,
            copies: contra_core::InstrumentCopies::default(),
            percussion: Percussion::FootTaps,
            percussion_rhythm: PercussionRhythm::Beat,
            melody_volume: 0.0,
            accompaniment_volume: 0.0,
            percussion_volume: 1.0,
            effects: contra_core::AudioEffects::default(),
            count_in_beats: 4.0,
            sample_rate: 24_000,
            variation_passes: 1,
            ending: false,
        };
        let buffer = crate::render::render(&request, None).expect("render taps");
        let peak = buffer
            .samples
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        let rms = (buffer
            .samples
            .iter()
            .map(|sample| sample * sample)
            .sum::<f32>()
            / buffer.samples.len() as f32)
            .sqrt();
        assert!(peak > 0.05, "foot taps peak at only {peak}");
        assert!(rms > 0.01, "foot taps RMS at only {rms}");
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
            second_accompaniment: contra_core::Countermelody::Accordion,
            copies: contra_core::InstrumentCopies::default(),
            percussion: Percussion::FootTaps,
            percussion_rhythm: PercussionRhythm::Auto,
            melody_volume: 0.75,
            accompaniment_volume: 0.65,
            percussion_volume: 0.5,
            effects: contra_core::AudioEffects::default(),
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

    #[test]
    fn percussion_copies_add_players_without_moving_the_grid() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let request = RenderRequest {
            arrangement,
            bpm: 240.0,
            melody: contra_core::MelodyInstrument::Fiddle,
            accompaniment: contra_core::AccompanimentInstrument::Piano,
            accompaniment_rhythm: contra_core::AccompanimentRhythm::Auto,
            second_accompaniment: contra_core::Countermelody::Off,
            copies: contra_core::InstrumentCopies {
                percussion: 2,
                ..contra_core::InstrumentCopies::default()
            },
            percussion: Percussion::Drumset,
            percussion_rhythm: PercussionRhythm::Basic,
            melody_volume: 0.0,
            accompaniment_volume: 0.0,
            percussion_volume: 0.6,
            effects: contra_core::AudioEffects::default(),
            count_in_beats: 4.0,
            sample_rate: 24_000,
            variation_passes: 1,
            ending: false,
        };
        let mut single = request.clone();
        single.copies.percussion = 1;
        let two = crate::render::render(&request, None).expect("render two players");
        let one = crate::render::render(&single, None).expect("render one player");

        assert_eq!(two.frames(), one.frames());
        assert_eq!(two.loop_pass_samples, one.loop_pass_samples);
        assert!(two.samples.iter().all(|sample| sample.is_finite()));
        assert!(two.samples.iter().all(|sample| sample.abs() <= 1.0));
        let difference = two
            .samples
            .iter()
            .zip(&one.samples)
            .fold(0.0f32, |worst, (a, b)| worst.max((a - b).abs()));
        assert!(
            difference > 1e-4,
            "a second percussion player must change the mix"
        );
        // The two players are panned apart, so the channels differ.
        let left: f32 = two.samples.iter().step_by(2).map(|s| s.abs()).sum();
        let right: f32 = two.samples.iter().skip(1).step_by(2).map(|s| s.abs()).sum();
        assert!(
            (left - right).abs() > 1.0,
            "the percussion copies should not sit dead centre together"
        );
    }
}
