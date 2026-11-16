//! Deterministic human timing for the audio renderers.
//!
//! A contra band does not hold a metronome. Each player drifts a little against
//! the grid, and the section breathes together without losing the dance. This
//! module reproduces that at a deliberately small scale:
//!
//! - every voice (melody, accompaniment, countermelody) and every copy of a
//!   voice has its own slow tempo deviation, bounded to
//!   [`TEMPO_DEVIATION_LIMIT`] (0.35 %, under two milliseconds per beat at
//!   240 BPM);
//! - a PID controller pulls each voice's accumulated timing error back to the
//!   un-deviated beat grid, so a player never leaves a permanent offset;
//! - notes that begin together as a chord are spread by a few milliseconds of
//!   pitch-level asynchrony, while a melody note sounding alone is left alone.
//!
//! Everything is a pure function of a seed derived from the arrangement and the
//! voice, so two identical renders are sample-identical.
//!
//! Deliberate exclusions:
//!
//! - the count-in is not humanized, so the four potatoes state the tempo;
//! - percussion is not humanized, because it is the audible grid the rest of
//!   the band plays against;
//! - the once-only slow ending follows its own written ritardando;
//! - MIDI export stays grid-exact (see [`crate::midi`]).
//!
//! Offsets are bounded twice: the tempo deviation bounds the rate, and
//! [`ONSET_OFFSET_LIMIT_SECONDS`] bounds every individual onset. An offset is
//! also clamped inside the 64-beat loop, so a note never crosses the loop
//! boundary and the loop's sample count never changes.

use std::f64::consts::TAU;

use crate::arrange::Arrangement;
use crate::render::RenderRequest;
use crate::score::NoteEvent;

/// Largest slow tempo deviation, as a fraction of the user-set tempo.
///
/// 0.35 % is under two milliseconds per beat at 240 BPM. Without correction a
/// whole 64-beat pass would drift by about 112 ms; the PID controller keeps
/// each onset within a few milliseconds instead.
pub const TEMPO_DEVIATION_LIMIT: f64 = 0.0035;

/// Absolute limit on one note's humanized onset, in seconds.
pub const ONSET_OFFSET_LIMIT_SECONDS: f64 = 0.012;

/// Largest asynchrony applied to one note of a chord, in seconds.
pub const CHORD_ASYNCHRONY_LIMIT_SECONDS: f64 = 0.006;

/// Control steps per dance beat for the deviation table.
pub const STEPS_PER_BEAT: usize = 8;

/// Proportional gain of the per-voice PID controller (dimensionless).
pub const PID_KP: f64 = 0.5;
/// Integral gain of the per-voice PID controller, per beat.
pub const PID_KI: f64 = 0.05;
/// Derivative gain of the per-voice PID controller, in beats.
pub const PID_KD: f64 = 0.02;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// One pitched part of the virtual band.
///
/// Percussion is intentionally absent: see the module docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Voice {
    Melody,
    Accompaniment,
    Countermelody,
}

impl Voice {
    /// Every humanized voice, in display order.
    pub const ALL: [Self; 3] = [Self::Melody, Self::Accompaniment, Self::Countermelody];

    /// Stable seed tag; changing a renderer's part assignment does not change
    /// the tag, so a voice keeps its character.
    const fn seed_tag(self) -> u64 {
        match self {
            Self::Melody => 0x6d65_6c6f_6479_0001,
            Self::Accompaniment => 0x6163_636f_6d70_0002,
            Self::Countermelody => 0x636f_756e_7465_7203,
        }
    }

    /// The humanized voice for a copy-plan part.
    pub fn of(part: crate::copies::Part) -> Self {
        match part {
            crate::copies::Part::Melody => Self::Melody,
            crate::copies::Part::Accompaniment => Self::Accompaniment,
            crate::copies::Part::Countermelody => Self::Countermelody,
        }
    }
}

/// A small PID controller whose setpoint is the un-deviated beat grid.
///
/// The state is the voice's accumulated timing error in beats (positive means
/// the voice is ahead of the grid). A slow tempo deviation feeds the error and
/// the controller's correction pulls it back. Gains are in beat units, so the
/// response is the same at every tempo.
#[derive(Clone, Copy, Debug, Default)]
pub struct TempoController {
    error_beats: f64,
    integral: f64,
    previous_error: f64,
}

impl TempoController {
    /// Advance one control step and return the new offset in beats.
    ///
    /// `deviation` is the fractional tempo deviation for this step and
    /// `dt_beats` is the step length. The result is the voice's onset offset
    /// from the grid at the end of the step.
    pub fn advance(&mut self, deviation: f64, dt_beats: f64) -> f64 {
        let dt = dt_beats.max(1e-9);
        self.integral += self.error_beats * dt;
        let derivative = (self.error_beats - self.previous_error) / dt;
        let correction = PID_KP * self.error_beats + PID_KI * self.integral + PID_KD * derivative;
        self.previous_error = self.error_beats;
        // A runaway would mean the controller failed; the clamp keeps a bad
        // input from producing a non-finite or absurd onset.
        self.error_beats = (self.error_beats + (deviation - correction) * dt).clamp(-1.0, 1.0);
        self.error_beats
    }

    /// Current accumulated timing error in beats.
    pub fn error_beats(&self) -> f64 {
        self.error_beats
    }
}

/// The human timing for one 64-beat loop, shared by the synth and soundfont
/// renderers.
#[derive(Clone, Debug)]
pub struct Humanizer {
    seed: u64,
    seconds_per_beat: f64,
    loop_beats: f64,
    enabled: bool,
}

impl Humanizer {
    /// Build from a render request. The seed comes from the arrangement, so a
    /// tune keeps its feel across renders and voices.
    pub fn new(request: &RenderRequest) -> Self {
        Self {
            seed: arrangement_seed(&request.arrangement),
            seconds_per_beat: (120.0 / request.bpm.max(1.0)).max(1e-6),
            loop_beats: request.arrangement.loop_beats.max(1.0),
            enabled: request.humanize_timing,
        }
    }

    /// Whether the setting is on.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// The timing for one copy of a voice, or `None` when the setting is off.
    pub fn voice(&self, voice: Voice, copy: usize) -> Option<VoiceTiming> {
        if !self.enabled {
            return None;
        }
        let seed = voice_seed(self.seed, voice, copy);
        Some(VoiceTiming {
            offsets: build_offsets(seed, self.seconds_per_beat, self.loop_beats),
            seconds_per_beat: self.seconds_per_beat,
            loop_beats: self.loop_beats,
            seed,
        })
    }
}

/// One player's timing curve.
#[derive(Clone, Debug)]
pub struct VoiceTiming {
    /// Onset offset in seconds at each control step.
    offsets: Vec<f64>,
    seconds_per_beat: f64,
    loop_beats: f64,
    seed: u64,
}

impl VoiceTiming {
    /// Humanized onset offset for a note, in seconds.
    ///
    /// `in_chord` adds the small pitch-level asynchrony; a note that sounds
    /// alone keeps the voice's timing without any extra smear. Notes before
    /// beat 0 (pickups) and non-finite positions get no offset. The result is
    /// clamped so the onset stays inside the loop and within
    /// [`ONSET_OFFSET_LIMIT_SECONDS`].
    pub fn onset_offset_seconds(
        &self,
        start_beat: f64,
        duration_beats: f64,
        midi: u8,
        in_chord: bool,
    ) -> f64 {
        if !start_beat.is_finite() || start_beat < 0.0 {
            return 0.0;
        }
        let mut seconds = self.base_offset_seconds(start_beat);
        if in_chord {
            seconds += self.chord_asynchrony_seconds(start_beat, midi);
        }
        let mut beats = seconds / self.seconds_per_beat;
        // Keep the onset inside the loop: no note may start before the pass or
        // run past its end, so the sample count and the loop seam never change.
        let lower = -start_beat;
        let upper = (self.loop_beats - start_beat - duration_beats.max(0.0)).max(0.0);
        if lower <= upper {
            beats = beats.clamp(lower, upper);
        } else {
            beats = 0.0;
        }
        (beats * self.seconds_per_beat)
            .clamp(-ONSET_OFFSET_LIMIT_SECONDS, ONSET_OFFSET_LIMIT_SECONDS)
    }

    /// The voice's slow offset at `start_beat`, before chord asynchrony.
    pub fn base_offset_seconds(&self, start_beat: f64) -> f64 {
        if self.offsets.is_empty() {
            return 0.0;
        }
        let position = (start_beat.max(0.0) * STEPS_PER_BEAT as f64)
            .clamp(0.0, (self.offsets.len() - 1) as f64);
        let lower = position.floor() as usize;
        let upper = (lower + 1).min(self.offsets.len() - 1);
        let fraction = position - lower as f64;
        self.offsets[lower] * (1.0 - fraction) + self.offsets[upper] * fraction
    }

    /// Deterministic pitch-level asynchrony for one chord note, in seconds.
    ///
    /// It is a hash of the voice seed, the chord's start, and the pitch, so
    /// every player spreads the same chord a little differently without any
    /// state or randomness at render time.
    pub fn chord_asynchrony_seconds(&self, start_beat: f64, midi: u8) -> f64 {
        let beat_key = (start_beat.max(0.0) * 1_000_000.0).round() as u64;
        let hash = splitmix64(self.seed ^ beat_key.rotate_left(17) ^ (u64::from(midi) << 40));
        let unit = (hash >> 11) as f64 / (1u64 << 53) as f64;
        (unit * 2.0 - 1.0) * CHORD_ASYNCHRONY_LIMIT_SECONDS
    }
}

/// Mark which notes begin together as a chord.
///
/// A note is a chord member when another note of the same section starts within
/// a microsecond of it. A melody note on its own is therefore not smeared.
pub fn chord_members(notes: &[NoteEvent]) -> Vec<bool> {
    let mut members = vec![false; notes.len()];
    for (index, note) in notes.iter().enumerate() {
        if members[index] {
            continue;
        }
        for (other_index, other) in notes.iter().enumerate().skip(index + 1) {
            if (note.start_beat - other.start_beat).abs() < 1e-6 {
                members[index] = true;
                members[other_index] = true;
            }
        }
    }
    members
}

/// Build the per-voice offset table for one loop pass.
fn build_offsets(seed: u64, seconds_per_beat: f64, loop_beats: f64) -> Vec<f64> {
    let steps = ((loop_beats.max(1.0) * STEPS_PER_BEAT as f64).ceil() as usize) + 1;
    let dt = 1.0 / STEPS_PER_BEAT as f64;
    // Two slow, seeded sines per voice. Their periods are roughly 4 and 8
    // bars at 240 BPM, which is the timescale a player's tempo wanders on.
    let (first_phase, second_phase, first_rate, second_rate) = seeded_phases(seed);
    let mut controller = TempoController::default();
    let mut offsets = Vec::with_capacity(steps);
    let limit_beats = ONSET_OFFSET_LIMIT_SECONDS / seconds_per_beat.max(1e-6);
    for step in 0..steps {
        let time = step as f64 * dt;
        let deviation = TEMPO_DEVIATION_LIMIT
            * (0.65 * (TAU * first_rate * time + first_phase).sin()
                + 0.35 * (TAU * second_rate * time + second_phase).sin());
        let offset = controller.advance(deviation, dt);
        offsets.push((offset * seconds_per_beat).clamp(-limit_beats, limit_beats));
    }
    offsets
}

/// Seeded phases and two slow rates, all deterministic.
fn seeded_phases(seed: u64) -> (f64, f64, f64, f64) {
    let a = splitmix64(seed);
    let b = splitmix64(a);
    let phase_a = (a >> 11) as f64 / (1u64 << 53) as f64 * TAU;
    let phase_b = (b >> 11) as f64 / (1u64 << 53) as f64 * TAU;
    // 0.05–0.09 and 0.11–0.16 cycles per beat.
    let rate_a = 0.05 + ((a & 0xffff) as f64 / 65535.0) * 0.04;
    let rate_b = 0.11 + ((b & 0xffff) as f64 / 65535.0) * 0.05;
    (phase_a, phase_b, rate_a, rate_b)
}

/// Stable seed from the arrangement, so a tune keeps its timing feel.
fn arrangement_seed(arrangement: &Arrangement) -> u64 {
    let mut hash = FNV_OFFSET;
    hash = fnv(hash, arrangement.loop_beats.to_bits());
    hash = fnv(hash, u64::from(arrangement.meter.numerator));
    hash = fnv(hash, u64::from(arrangement.meter.denominator));
    hash = fnv(hash, arrangement.key.tonic_pitch_class as u64);
    for note in arrangement.melody.iter().chain(&arrangement.pickup) {
        hash = fnv(hash, note.start_beat.to_bits());
        hash = fnv(hash, note.duration_beats.to_bits());
        hash = fnv(hash, u64::from(note.midi));
    }
    for chord in &arrangement.chords {
        hash = fnv(hash, chord.start_beat.to_bits());
        for byte in chord.symbol.bytes() {
            hash = fnv(hash, u64::from(byte));
        }
    }
    hash
}

fn voice_seed(tune_seed: u64, voice: Voice, copy: usize) -> u64 {
    let hash = fnv(tune_seed, voice.seed_tag());
    fnv(hash, copy as u64)
}

fn fnv(hash: u64, value: u64) -> u64 {
    let mut hash = hash ^ value;
    for _ in 0..8 {
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= hash >> 32;
    }
    hash
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abc::parse_abc;

    const TUNE: &str = "X:1\nT:Test\nM:4/4\nL:1/8\nK:D\n\"D\"A2 A2 A2 A2 | \"G\"B2 B2 B2 B2|\n";

    fn humanizer() -> Humanizer {
        let score = parse_abc(TUNE).expect("parse");
        Humanizer {
            seed: arrangement_seed(&Arrangement::from_score(&score, 64.0)),
            seconds_per_beat: 0.5,
            loop_beats: 64.0,
            enabled: true,
        }
    }

    #[test]
    fn offsets_stay_within_the_configured_bound() {
        let humanizer = humanizer();
        for voice in Voice::ALL {
            for copy in 0..4 {
                let timing = humanizer.voice(voice, copy).expect("timing");
                let mut beat = 0.0;
                while beat < 64.0 {
                    let offset = timing.onset_offset_seconds(beat, 0.5, 60, true);
                    assert!(
                        offset.abs() <= ONSET_OFFSET_LIMIT_SECONDS + 1e-9,
                        "{voice:?} copy {copy} at beat {beat} moved {offset}s"
                    );
                    assert!(offset.is_finite());
                    beat += 0.25;
                }
            }
        }
    }

    #[test]
    fn one_note_never_crosses_the_loop_boundary() {
        let humanizer = humanizer();
        for voice in Voice::ALL {
            let timing = humanizer.voice(voice, 0).expect("timing");
            let first = timing.onset_offset_seconds(0.0, 1.0, 60, true);
            assert!(first >= 0.0, "beat 0 may not move earlier: {first}");
            let last_start = 63.5;
            let offset = timing.onset_offset_seconds(last_start, 0.5, 60, true);
            let end = last_start + offset / timing.seconds_per_beat + 0.5;
            assert!(end <= 64.0 + 1e-9, "note ends at beat {end}");
        }
    }

    #[test]
    fn a_perturbed_controller_error_decays_toward_zero() {
        let dt = 1.0 / STEPS_PER_BEAT as f64;
        let mut controller = TempoController::default();
        for _ in 0..8 {
            controller.advance(TEMPO_DEVIATION_LIMIT, dt);
        }
        let perturbed = controller.error_beats().abs();
        assert!(perturbed > 1e-5, "the deviation should displace it");

        // Sixteen bars at 240 BPM: 32 dance beats. The integral unwinds
        // steadily, so the residual error is negligible well before a tune's
        // second pass.
        for _ in 0..(32 * STEPS_PER_BEAT) {
            controller.advance(0.0, dt);
        }
        assert!(
            controller.error_beats().abs() < perturbed * 0.02,
            "error {} did not decay from {perturbed}",
            controller.error_beats()
        );
    }

    #[test]
    fn different_voices_and_copies_get_different_offsets() {
        let humanizer = humanizer();
        let melody = humanizer.voice(Voice::Melody, 0).expect("melody");
        let accompaniment = humanizer
            .voice(Voice::Accompaniment, 0)
            .expect("accompaniment");
        let melody_two = humanizer.voice(Voice::Melody, 1).expect("second melody");
        assert_ne!(
            melody.base_offset_seconds(8.0).to_bits(),
            accompaniment.base_offset_seconds(8.0).to_bits()
        );
        assert_ne!(
            melody.base_offset_seconds(8.0).to_bits(),
            melody_two.base_offset_seconds(8.0).to_bits()
        );
    }

    #[test]
    fn simultaneous_chord_notes_are_spread_but_bounded() {
        let notes = [
            NoteEvent {
                start_beat: 4.0,
                duration_beats: 1.0,
                midi: 60,
                velocity: 80,
            },
            NoteEvent {
                start_beat: 4.0,
                duration_beats: 1.0,
                midi: 64,
                velocity: 80,
            },
            NoteEvent {
                start_beat: 4.0,
                duration_beats: 1.0,
                midi: 67,
                velocity: 80,
            },
            NoteEvent {
                start_beat: 5.0,
                duration_beats: 1.0,
                midi: 72,
                velocity: 80,
            },
        ];
        let members = chord_members(&notes);
        assert_eq!(members, vec![true, true, true, false]);
        assert!(!members[3], "a lone melody note must not be smeared");

        let timing = humanizer().voice(Voice::Accompaniment, 0).expect("timing");
        let spread: Vec<f64> = notes[..3]
            .iter()
            .map(|note| timing.chord_asynchrony_seconds(note.start_beat, note.midi))
            .collect();
        assert!(
            spread.windows(2).any(|pair| pair[0] != pair[1]),
            "the chord should not move as one block: {spread:?}"
        );
        for offset in &spread {
            assert!(
                offset.abs() <= CHORD_ASYNCHRONY_LIMIT_SECONDS + 1e-12,
                "{offset}"
            );
        }
        // A note on its own keeps the plain voice offset.
        let alone = timing.onset_offset_seconds(5.0, 1.0, 72, false);
        assert_eq!(alone, timing.base_offset_seconds(5.0));
    }

    #[test]
    fn offsets_are_reproducible_for_the_same_tune() {
        let first = humanizer();
        let second = humanizer();
        for voice in Voice::ALL {
            for copy in 0..3 {
                let a = first.voice(voice, copy).expect("a");
                let b = second.voice(voice, copy).expect("b");
                for beat in [0.0, 1.0, 8.0, 31.5, 63.0] {
                    assert_eq!(
                        a.onset_offset_seconds(beat, 0.5, 60, true).to_bits(),
                        b.onset_offset_seconds(beat, 0.5, 60, true).to_bits()
                    );
                }
            }
        }
    }

    #[test]
    fn the_setting_off_returns_no_timing() {
        let mut humanizer = humanizer();
        humanizer.enabled = false;
        assert!(humanizer.voice(Voice::Melody, 0).is_none());
    }
}
