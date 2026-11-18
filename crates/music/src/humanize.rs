//! Deterministic human performance for the audio renderers.
//!
//! A contra band does not hold a metronome. Each player drifts a little against
//! the grid, and a fiddler's fingers land a little off the nominal pitch. This
//! module reproduces that at a deliberately small scale:
//!
//! - every voice (melody, accompaniment, countermelody) and every copy of a
//!   voice has its own slow tempo deviation, bounded to
//!   [`TEMPO_DEVIATION_LIMIT`] (0.35 %, under two milliseconds per beat at
//!   240 BPM) times the timing amount;
//! - a PID controller pulls each voice's accumulated timing error back to the
//!   un-deviated beat grid, so a player never leaves a permanent offset. Its
//!   gains are the [`PidGains`] in the render request, so a caller can tune how
//!   quickly a wandering player is pulled back;
//! - notes that begin together as a chord are spread by a few milliseconds of
//!   pitch-level asynchrony, while a melody note sounding alone is left alone;
//! - an unfretted string player (a fiddle) places every finger slightly off the
//!   nominal pitch. Each note gets a small intonation error drawn from a normal
//!   distribution, bounded to [`PITCH_DEVIATION_LIMIT_CENTS`] at a tuning amount
//!   of 1. Only unfretted strings drift: frets, keys, and the instrument's own
//!   tuning set the pitch of every other voice. The tuning amount is separate
//!   from the timing amount.
//!
//! Everything is a pure function of a seed derived from the arrangement and the
//! voice, so two identical renders are sample-identical.
//!
//! Deliberate exclusions:
//!
//! - the count-in is not humanized, so the four potatoes state the tempo;
//! - percussion is not humanized, because it is the audible grid the rest of
//!   the band plays against;
//! - the once-only slow ending follows its own written ritardando and keeps its
//!   written pitch;
//! - MIDI export stays grid-exact and at the nominal pitch (see
//!   [`crate::midi`]).
//!
//! Offsets are bounded twice: the tempo deviation bounds the rate, and
//! [`ONSET_OFFSET_LIMIT_SECONDS`] times the timing amount bounds every
//! individual onset. An offset is also clamped inside the 64-beat loop, so a
//! note never crosses the loop boundary and the loop's sample count never
//! changes.

use std::f64::consts::TAU;

use caller_core::{MAX_HUMANIZE_AMOUNT, MAX_PID_KD, MAX_PID_KI, MAX_PID_KP};

use crate::arrange::Arrangement;
use crate::render::RenderRequest;
use crate::score::NoteEvent;

/// Largest slow tempo deviation at amount 1, as a fraction of the user-set
/// tempo.
///
/// 0.35 % is under two milliseconds per beat at 240 BPM. Without correction a
/// whole 64-beat pass would drift by about 112 ms; the PID controller keeps
/// each onset within a few milliseconds instead. The timing amount
/// scales the deviation.
pub const TEMPO_DEVIATION_LIMIT: f64 = 0.0035;

/// Absolute limit on one note's humanized onset at amount 1, in seconds.
///
/// The limit scales with the timing amount, so the setting stays linear
/// up to [`MAX_HUMANIZE_AMOUNT`].
pub const ONSET_OFFSET_LIMIT_SECONDS: f64 = 0.012;

/// Largest asynchrony applied to one note of a chord at amount 1, in seconds.
pub const CHORD_ASYNCHRONY_LIMIT_SECONDS: f64 = 0.006;

/// Standard deviation of one note's intonation error at tuning amount 1, in
/// cents.
///
/// Four cents is a little under a tenth of a semitone: enough that a fiddler's
/// line sounds played rather than tuned, without approaching a wrong note. The
/// tuning amount scales it, separately from the timing amount.
pub const PITCH_DEVIATION_SIGMA_CENTS: f64 = 4.0;

/// Hard bound on one note's intonation error at tuning amount 1, in cents.
///
/// Three standard deviations keeps the normal distribution's rare tail from
/// producing a note that reads as out of tune rather than human.
pub const PITCH_DEVIATION_LIMIT_CENTS: f64 = 12.0;

/// Control steps per dance beat for the deviation table.
pub const STEPS_PER_BEAT: usize = 8;

/// Default proportional gain of the per-voice PID controller (dimensionless).
pub const PID_KP: f64 = 0.5;
/// Default integral gain of the per-voice PID controller, per beat.
pub const PID_KI: f64 = 0.05;
/// Default derivative gain of the per-voice PID controller, in beats.
pub const PID_KD: f64 = 0.02;

/// The gains of a tempo controller, in beat units.
///
/// The request carries these so a caller can tune how quickly a wandering
/// player is pulled back to the grid; the [`Default`] values are the calibrated
/// response.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PidGains {
    /// Proportional gain (dimensionless).
    pub kp: f64,
    /// Integral gain, per beat.
    pub ki: f64,
    /// Derivative gain, in beats.
    pub kd: f64,
}

impl Default for PidGains {
    fn default() -> Self {
        Self {
            kp: PID_KP,
            ki: PID_KI,
            kd: PID_KD,
        }
    }
}

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
    gains: PidGains,
    error_beats: f64,
    integral: f64,
    previous_error: f64,
}

impl TempoController {
    /// A controller with the given gains.
    pub fn new(gains: PidGains) -> Self {
        Self {
            gains,
            ..Self::default()
        }
    }

    /// Advance one control step and return the new offset in beats.
    ///
    /// `deviation` is the fractional tempo deviation for this step and
    /// `dt_beats` is the step length. The result is the voice's onset offset
    /// from the grid at the end of the step.
    pub fn advance(&mut self, deviation: f64, dt_beats: f64) -> f64 {
        let dt = dt_beats.max(1e-9);
        self.integral += self.error_beats * dt;
        let derivative = (self.error_beats - self.previous_error) / dt;
        let correction = self.gains.kp * self.error_beats
            + self.gains.ki * self.integral
            + self.gains.kd * derivative;
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

/// The human performance for one 64-beat loop: timing and intonation, shared by
/// the synth and soundfont renderers.
#[derive(Clone, Debug)]
pub struct Humanizer {
    seed: u64,
    seconds_per_beat: f64,
    loop_beats: f64,
    enabled: bool,
    tempo_drift: bool,
    chord_spread: bool,
    pitch_drift: bool,
    amount: f64,
    pitch_amount: f64,
    gains: PidGains,
}

impl Humanizer {
    /// Build from a render request. The seed comes from the arrangement, so a
    /// tune keeps its feel across renders and voices.
    pub fn new(request: &RenderRequest) -> Self {
        let options = &request.humanize_options;
        Self {
            seed: arrangement_seed(&request.arrangement),
            seconds_per_beat: (120.0 / request.bpm.max(1.0)).max(1e-6),
            loop_beats: request.arrangement.loop_beats.max(1.0),
            enabled: request.humanize_timing,
            tempo_drift: options.tempo_drift,
            chord_spread: options.chord_spread,
            pitch_drift: options.pitch_drift,
            amount: if options.amount.is_finite() {
                f64::from(options.amount.clamp(0.0, MAX_HUMANIZE_AMOUNT))
            } else {
                0.0
            },
            pitch_amount: if options.pitch_amount.is_finite() {
                f64::from(options.pitch_amount.clamp(0.0, MAX_HUMANIZE_AMOUNT))
            } else {
                0.0
            },
            gains: PidGains {
                kp: gain(options.pid_kp, PID_KP, MAX_PID_KP),
                ki: gain(options.pid_ki, PID_KI, MAX_PID_KI),
                kd: gain(options.pid_kd, PID_KD, MAX_PID_KD),
            },
        }
    }

    /// Whether the setting is on.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// The timing for one copy of a voice, or `None` when no human timing
    /// applies.
    pub fn voice(&self, voice: Voice, copy: usize) -> Option<VoiceTiming> {
        if !self.enabled || self.amount <= 0.0 || (!self.tempo_drift && !self.chord_spread) {
            return None;
        }
        let seed = voice_seed(self.seed, voice, copy);
        let offsets = if self.tempo_drift {
            build_offsets(
                seed,
                self.seconds_per_beat,
                self.loop_beats,
                self.amount,
                self.gains,
            )
        } else {
            vec![0.0; ((self.loop_beats * STEPS_PER_BEAT as f64).ceil() as usize) + 1]
        };
        Some(VoiceTiming {
            offsets,
            seconds_per_beat: self.seconds_per_beat,
            loop_beats: self.loop_beats,
            seed,
            chord_spread: self.chord_spread,
            amount: self.amount,
            offset_limit_seconds: ONSET_OFFSET_LIMIT_SECONDS * self.amount,
        })
    }

    /// The intonation error for one copy of an unfretted-string voice, or
    /// `None` when pitch humanization is off.
    ///
    /// The caller decides whether the voice is an unfretted string (see
    /// [`caller_core::BandVoice::is_unfretted_strings`]); this module only
    /// supplies the distribution. Pitch drift has its own amount and is
    /// independent of the timing switches, so a band can keep its timing tight
    /// while the fiddle plays slightly out of tune.
    pub fn pitch(&self, voice: Voice, copy: usize) -> Option<PitchHumanizer> {
        if !self.enabled || !self.pitch_drift || self.pitch_amount <= 0.0 {
            return None;
        }
        Some(PitchHumanizer {
            seed: voice_seed(self.seed, voice, copy),
            amount: self.pitch_amount,
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
    chord_spread: bool,
    amount: f64,
    /// Largest onset offset in seconds at this amount.
    offset_limit_seconds: f64,
}

impl VoiceTiming {
    /// Humanized onset offset for a note, in seconds.
    ///
    /// `in_chord` adds the small pitch-level asynchrony; a note that sounds
    /// alone keeps the voice's timing without any extra smear. Notes before
    /// beat 0 (pickups) and non-finite positions get no offset. The result is
    /// clamped so the onset stays inside the loop and within the amount-scaled
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
        if in_chord && self.chord_spread {
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
        (beats * self.seconds_per_beat).clamp(-self.offset_limit_seconds, self.offset_limit_seconds)
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
        if !self.chord_spread || self.amount <= 0.0 {
            return 0.0;
        }
        let beat_key = (start_beat.max(0.0) * 1_000_000.0).round() as u64;
        let hash = splitmix64(self.seed ^ beat_key.rotate_left(17) ^ (u64::from(midi) << 40));
        let unit = (hash >> 11) as f64 / (1u64 << 53) as f64;
        (unit * 2.0 - 1.0) * CHORD_ASYNCHRONY_LIMIT_SECONDS * self.amount
    }
}

/// Deterministic intonation error for one unfretted-string player.
///
/// Each note's error is independent, as a fiddler's finger placement is, but it
/// is a pure function of the player, the note's start, and its pitch. Two
/// renders of the same tune therefore detune exactly the same notes by exactly
/// the same amounts.
#[derive(Clone, Debug)]
pub struct PitchHumanizer {
    seed: u64,
    amount: f64,
}

impl PitchHumanizer {
    /// Deviation from the nominal pitch for a note, in cents.
    ///
    /// The value comes from a standard normal distribution (Box-Muller). It is
    /// clamped to three standard deviations and scaled by the tuning amount, so
    /// tuning amount 0 is exactly in tune and amount 1 stays within
    /// [`PITCH_DEVIATION_LIMIT_CENTS`].
    pub fn deviation_cents(&self, start_beat: f64, midi: u8) -> f64 {
        if self.amount <= 0.0 || !start_beat.is_finite() {
            return 0.0;
        }
        // A signed key keeps a pickup before beat 0 distinct from beat 0.
        let beat_key = (start_beat * 1_000_000.0).round() as i64 as u64;
        let first = splitmix64(self.seed ^ beat_key.rotate_left(23) ^ (u64::from(midi) << 48));
        let second = splitmix64(first);
        // Both uniforms are inside (0, 1): the +0.5 keeps ln finite at the
        // bottom of the range.
        let first_unit = (((first >> 11) as f64) + 0.5) / (1u64 << 53) as f64;
        let second_unit = (second >> 11) as f64 / (1u64 << 53) as f64;
        let normal = (-2.0 * first_unit.ln()).sqrt() * (TAU * second_unit).cos();
        let limit = PITCH_DEVIATION_LIMIT_CENTS * self.amount;
        (normal.clamp(-3.0, 3.0) * PITCH_DEVIATION_SIGMA_CENTS * self.amount).clamp(-limit, limit)
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
fn build_offsets(
    seed: u64,
    seconds_per_beat: f64,
    loop_beats: f64,
    amount: f64,
    gains: PidGains,
) -> Vec<f64> {
    let steps = ((loop_beats.max(1.0) * STEPS_PER_BEAT as f64).ceil() as usize) + 1;
    let dt = 1.0 / STEPS_PER_BEAT as f64;
    // Two slow, seeded sines per voice. Their periods are roughly 4 and 8
    // bars at 240 BPM, which is the timescale a player's tempo wanders on.
    let (first_phase, second_phase, first_rate, second_rate) = seeded_phases(seed);
    let amount = amount.clamp(0.0, f64::from(MAX_HUMANIZE_AMOUNT));
    // The bounds scale with the amount, so the setting is linear all the way to
    // its maximum instead of saturating at the amount-1 limit.
    let limit_seconds = ONSET_OFFSET_LIMIT_SECONDS * amount;
    let mut controller = TempoController::new(gains);
    let mut offsets = Vec::with_capacity(steps);
    for step in 0..steps {
        let time = step as f64 * dt;
        let deviation = TEMPO_DEVIATION_LIMIT
            * amount
            * (0.65 * (TAU * first_rate * time + first_phase).sin()
                + 0.35 * (TAU * second_rate * time + second_phase).sin());
        let offset = controller.advance(deviation, dt);
        offsets.push((offset * seconds_per_beat).clamp(-limit_seconds, limit_seconds));
    }
    offsets
}

/// A finite gain inside `0.0..=max`, or the default when the stored value is
/// not finite.
fn gain(value: f32, default: f64, max: f32) -> f64 {
    if value.is_finite() {
        f64::from(value.clamp(0.0, max))
    } else {
        default
    }
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
            tempo_drift: true,
            chord_spread: true,
            pitch_drift: true,
            amount: 1.0,
            pitch_amount: 1.0,
            gains: PidGains::default(),
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

    #[test]
    fn disabling_tempo_drift_leaves_only_chord_spread() {
        let mut humanizer = humanizer();
        humanizer.tempo_drift = false;
        let timing = humanizer.voice(Voice::Melody, 0).expect("spread only");
        for beat in [0.0, 1.0, 8.0, 31.5, 63.0] {
            assert_eq!(timing.base_offset_seconds(beat), 0.0);
        }
        assert!(
            (0..8).any(|midi| timing.chord_asynchrony_seconds(4.0, 60 + midi) != 0.0),
            "chord spread should still move notes"
        );
    }

    #[test]
    fn disabling_chord_spread_removes_chord_asynchrony() {
        let mut humanizer = humanizer();
        humanizer.chord_spread = false;
        let timing = humanizer
            .voice(Voice::Accompaniment, 0)
            .expect("drift only");
        for midi in 55..75 {
            assert_eq!(timing.chord_asynchrony_seconds(4.0, midi), 0.0);
        }
        // The slow voice offset remains.
        assert!(timing.base_offset_seconds(8.0).abs() > 0.0);
    }

    #[test]
    fn amount_zero_is_identical_to_off() {
        let mut humanizer = humanizer();
        humanizer.amount = 0.0;
        assert!(humanizer.voice(Voice::Melody, 0).is_none());
    }

    #[test]
    fn amount_scales_the_calibrated_offsets() {
        let full = humanizer().voice(Voice::Melody, 0).expect("full");
        let mut half_humanizer = humanizer();
        half_humanizer.amount = 0.5;
        let half = half_humanizer.voice(Voice::Melody, 0).expect("half");
        for beat in [0.0, 1.0, 8.0, 31.5, 63.0] {
            let full_offset = full.base_offset_seconds(beat);
            let half_offset = half.base_offset_seconds(beat);
            assert!(
                (half_offset - full_offset * 0.5).abs() < 1e-12,
                "half amount should halve the drift at beat {beat}"
            );
        }
        for midi in 55..75 {
            assert!(
                (half.chord_asynchrony_seconds(4.0, midi)
                    - full.chord_asynchrony_seconds(4.0, midi) * 0.5)
                    .abs()
                    < 1e-12
            );
        }
    }

    #[test]
    fn amount_one_keeps_the_calibrated_output() {
        let humanizer = humanizer();
        let timing = humanizer.voice(Voice::Melody, 0).expect("timing");
        assert!(timing.base_offset_seconds(8.0).abs() > 0.0);
        assert!(timing.base_offset_seconds(8.0).abs() <= ONSET_OFFSET_LIMIT_SECONDS);
        assert_eq!(humanizer.amount, 1.0);
    }

    #[test]
    fn the_maximum_amount_stays_linear_past_the_calibrated_bound() {
        let full = humanizer().voice(Voice::Melody, 0).expect("full");
        let max = f64::from(MAX_HUMANIZE_AMOUNT);
        let mut maximum_humanizer = humanizer();
        maximum_humanizer.amount = max;
        let maximum = maximum_humanizer
            .voice(Voice::Melody, 0)
            .expect("maximum amount");
        for beat in [0.0, 1.0, 8.0, 31.5, 63.0] {
            let full_offset = full.base_offset_seconds(beat);
            let maximum_offset = maximum.base_offset_seconds(beat);
            assert!(
                (maximum_offset - full_offset * max).abs() < 1e-9,
                "twenty times the amount should move twenty times as far at beat {beat}: \
                 {maximum_offset} vs {full_offset}"
            );
        }
        // The onset bound scales too, so the maximum amount is not clipped back
        // to the amount-1 limit somewhere in the pass.
        let mut largest = 0.0_f64;
        let mut beat = 0.0;
        while beat < 64.0 {
            largest = largest.max(maximum.onset_offset_seconds(beat, 0.5, 60, false).abs());
            beat += 0.25;
        }
        assert!(
            largest > ONSET_OFFSET_LIMIT_SECONDS,
            "the amount-1 cap must not flatten the larger setting: {largest}"
        );
        assert!(
            largest <= ONSET_OFFSET_LIMIT_SECONDS * max + 1e-9,
            "the scaled cap should still bound the onset: {largest}"
        );
    }

    #[test]
    fn the_pid_gains_change_the_pull_back_to_the_grid() {
        let dt = 1.0 / STEPS_PER_BEAT as f64;
        let mut calibrated = TempoController::default();
        let mut stiff = TempoController::new(PidGains {
            kp: 2.0,
            ki: 0.5,
            kd: 0.5,
        });
        let mut soft = TempoController::new(PidGains {
            kp: 0.0,
            ki: 0.0,
            kd: 0.0,
        });
        for _ in 0..(8 * STEPS_PER_BEAT) {
            let deviation = TEMPO_DEVIATION_LIMIT;
            calibrated.advance(deviation, dt);
            stiff.advance(deviation, dt);
            soft.advance(deviation, dt);
        }
        // With no correction the soft controller keeps accumulating the
        // deviation, so its error is the largest; the stiff one resists most.
        assert!(
            soft.error_beats().abs() > calibrated.error_beats().abs(),
            "zero gains should drift furthest"
        );
        assert!(
            calibrated.error_beats().abs() > stiff.error_beats().abs(),
            "stiffer gains should hold the grid more tightly"
        );
    }

    #[test]
    fn pitch_errors_are_normal_and_bounded() {
        let pitch = humanizer().pitch(Voice::Melody, 0).expect("pitch");
        let values: Vec<f64> = (0..10_000)
            .map(|step| pitch.deviation_cents(step as f64 * 0.25, 60 + (step % 12) as u8))
            .collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values
            .iter()
            .map(|value| (value - mean) * (value - mean))
            .sum::<f64>()
            / values.len() as f64;
        let deviation = variance.sqrt();
        assert!(
            mean.abs() < 0.15,
            "the distribution should center on the nominal pitch, not {mean}"
        );
        assert!(
            (deviation - PITCH_DEVIATION_SIGMA_CENTS).abs() < 0.2,
            "the standard deviation should be the calibrated {PITCH_DEVIATION_SIGMA_CENTS} \
             cents, not {deviation}"
        );
        assert!(
            values
                .iter()
                .all(|value| value.abs() <= PITCH_DEVIATION_LIMIT_CENTS + 1e-9),
            "no note may leave the three-sigma bound"
        );
        assert!(
            values.iter().any(|value| *value < 0.0) && values.iter().any(|value| *value > 0.0),
            "a fiddler plays both sharp and flat"
        );
    }

    #[test]
    fn pitch_errors_are_reproducible_and_per_player() {
        let first = humanizer().pitch(Voice::Melody, 0).expect("first");
        let second = humanizer().pitch(Voice::Melody, 0).expect("second");
        let other_copy = humanizer().pitch(Voice::Melody, 1).expect("other copy");
        assert_eq!(
            first.deviation_cents(4.0, 60).to_bits(),
            second.deviation_cents(4.0, 60).to_bits(),
            "the same player must detune the same note the same way"
        );
        assert_ne!(
            first.deviation_cents(4.0, 60).to_bits(),
            other_copy.deviation_cents(4.0, 60).to_bits(),
            "each player in a section has a different hand"
        );
        assert_ne!(
            first.deviation_cents(4.0, 60).to_bits(),
            first.deviation_cents(4.5, 60).to_bits(),
            "every note is placed independently"
        );
    }

    #[test]
    fn pitch_humanization_follows_its_switch_and_its_own_amount() {
        let mut off = humanizer();
        off.pitch_drift = false;
        assert!(off.pitch(Voice::Melody, 0).is_none());

        let mut disabled = humanizer();
        disabled.enabled = false;
        assert!(disabled.pitch(Voice::Melody, 0).is_none());

        let mut zero = humanizer();
        zero.pitch_amount = 0.0;
        assert!(zero.pitch(Voice::Melody, 0).is_none());

        let full = humanizer().pitch(Voice::Melody, 0).expect("full");
        let mut half_humanizer = humanizer();
        half_humanizer.pitch_amount = 0.5;
        let half = half_humanizer.pitch(Voice::Melody, 0).expect("half");
        for step in 0..64 {
            let beat = step as f64 * 0.25;
            assert!(
                (half.deviation_cents(beat, 67) - full.deviation_cents(beat, 67) * 0.5).abs()
                    < 1e-12,
                "half the amount should halve the intonation error at beat {beat}"
            );
        }
    }

    #[test]
    fn timing_and_tuning_amounts_are_independent() {
        let base = humanizer();
        let base_timing = base.voice(Voice::Melody, 0).expect("timing");
        let base_pitch = base.pitch(Voice::Melody, 0).expect("pitch");

        // Turning the timing amount off leaves the intonation untouched.
        let mut timing_off = humanizer();
        timing_off.amount = 0.0;
        assert!(timing_off.voice(Voice::Melody, 0).is_none());
        let pitch = timing_off.pitch(Voice::Melody, 0).expect("pitch stays on");
        assert_eq!(
            pitch.deviation_cents(4.0, 60).to_bits(),
            base_pitch.deviation_cents(4.0, 60).to_bits()
        );

        // Turning the tuning amount off leaves the timing untouched.
        let mut tuning_off = humanizer();
        tuning_off.pitch_amount = 0.0;
        assert!(tuning_off.pitch(Voice::Melody, 0).is_none());
        let timing = tuning_off.voice(Voice::Melody, 0).expect("timing stays on");
        assert_eq!(
            timing.base_offset_seconds(8.0).to_bits(),
            base_timing.base_offset_seconds(8.0).to_bits()
        );
    }

    #[test]
    fn a_pickup_note_gets_its_own_pitch_error() {
        let pitch = humanizer().pitch(Voice::Melody, 0).expect("pitch");
        assert_ne!(
            pitch.deviation_cents(-0.5, 62).to_bits(),
            pitch.deviation_cents(0.0, 62).to_bits(),
            "a pickup must not borrow the downbeat's intonation"
        );
    }
}
