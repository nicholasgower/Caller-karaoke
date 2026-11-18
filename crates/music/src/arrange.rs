//! Turn a parsed score into a dance-length music loop and generate the
//! accompaniment.

use caller_core::{
    AccompanimentInstrument, AccompanimentRhythm, BowingOptions, EndingOptions, VariationOptions,
};

use crate::chord::{Chord, resolve_chords};
use crate::score::{ChordEvent, Key, Meter, NoteEvent, Score};

/// Number of distinct passes rendered back to back for one practice tune. Pass
/// 0 is the plain pattern; the others carry small variations so a looping
/// session does not repeat note for note.
pub const VARIATION_PASSES: u32 = 4;

/// How much longer the tune's last bar takes in the slow ending than at the
/// session tempo. Three times is a deliberate, audible ritardando, not a
/// subtle easing.
pub const ENDING_LAST_BAR_STRETCH: f64 = 3.0;

/// Number of beats before the loop end that the gradual ending begins.
///
/// Four beats is the last two bars: enough time for the accompaniment to thin
/// and the tempo to ease before the final chord, without making the ending so
/// long that it reads as a separate section.
pub const ENDING_GRADUAL_BEATS: f64 = 4.0;

/// How much longer the four-beat gradual region takes than at session tempo.
pub const ENDING_GRADUAL_STRETCH: f64 = 2.0;

/// Shape of the ritardando inside the ending's last bar. Above 1 the bar starts
/// at the session tempo and the following beats stretch more and more, so the
/// ending eases into the slowdown instead of lurching.
pub const ENDING_SLOWDOWN_CURVE: f64 = 2.0;

/// Dance beats the final harmony is held before the audio fades out.
pub const ENDING_LINGER_BEATS: f64 = 8.0;

/// Minimum melody coverage, in dance beats, for a bar to be left to the melody.
///
/// A two-beat bar needs a real tune line to carry it. Requiring this much
/// sounding melody means a variation rest never turns a written rest into a
/// silent measure.
pub const MIN_MELODY_ALONE_BEATS: f64 = 0.75;

/// Smallest interval, in semitones, the chordal accompaniment keeps from any
/// melody note it sounds against.
///
/// A minor third or more reads as two independent voices. Anything closer — a
/// unison, a minor second, or a major second — thickens the melody instead of
/// supporting it, so the accompaniment note is moved down an octave.
pub const MIN_MELODY_SEPARATION_SEMITONES: i32 = 3;

/// Lowest pitch an accompaniment note may reach when it is moved down an
/// octave to clear the melody. The floor keeps the fix from pushing a chord
/// into inaudible sub-bass on small speakers.
pub const MIN_ACCOMPANIMENT_MIDI: u8 = 24;

/// One note of the once-only ending.
///
/// Unlike [`NoteEvent`], an ending note is timed in seconds because the ending
/// deliberately leaves the fixed dance-beat grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EndingNote {
    /// Seconds from the start of the ending.
    pub start_seconds: f64,
    pub duration_seconds: f64,
    pub midi: u8,
    pub velocity: u8,
}

/// The once-only slow finish appended to the practice loop.
#[derive(Clone, Debug, PartialEq)]
pub struct Ending {
    /// The stretched closing melody.
    pub melody: Vec<EndingNote>,
    /// The ending region's backing plus the held final chord.
    pub accompaniment: Vec<EndingNote>,
    /// Beat in a normal 64-beat pass where the ending replaces the remaining
    /// music. It is four beats before the loop end for a gradual ending, and
    /// the start of the tune's last bar otherwise.
    pub from_beat: f64,
    /// When the held final chord begins, in seconds from the ending's start.
    pub linger_start_seconds: f64,
    /// Total length of the ending in seconds.
    pub seconds: f64,
}

/// The melody is lifted to at least this velocity so it sits above the
/// accompaniment, whose notes are written in the 46-60 range.
const MELODY_VELOCITY_FLOOR: u8 = 92;
/// Leave a little headroom below full velocity.
const MELODY_VELOCITY_CEILING: u8 = 124;

/// The music for one practice loop.
#[derive(Clone, Debug)]
pub struct Arrangement {
    /// Melody notes with times relative to the start of the loop.
    pub melody: Vec<NoteEvent>,
    /// Notes that belong before beat 0 (a pickup, played in the count-in).
    pub pickup: Vec<NoteEvent>,
    pub chords: Vec<ChordEvent>,
    pub loop_beats: f64,
    /// Natural length of the tune body in dance beats, excluding any pickup
    /// and before padding or trimming to [`Self::loop_beats`].
    pub body_beats: f64,
    pub meter: Meter,
    pub key: Key,
    pub warnings: Vec<String>,
    /// Which fiddle bowing rules shape the melody and countermelody lines.
    pub bowing: BowingOptions,
}

/// The dance and tune disagree about how many beats the figure set lasts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LengthMismatch {
    pub dance_beats: f64,
    pub tune_beats: f64,
}

impl std::fmt::Display for LengthMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the dance is {:.1} beats but the tune is {:.1} beats",
            self.dance_beats, self.tune_beats
        )
    }
}

impl std::error::Error for LengthMismatch {}

impl Arrangement {
    pub fn from_score(score: &Score, loop_beats: f64) -> Self {
        Self::from_score_with_bowing(score, loop_beats, BowingOptions::default())
    }

    /// Build an arrangement with explicit fiddle bowing rules.
    ///
    /// [`Self::from_score`] enables both rules; this variant lets a caller
    /// carry the user's settings into every melody and countermelody pass.
    pub fn from_score_with_bowing(score: &Score, loop_beats: f64, bowing: BowingOptions) -> Self {
        let loop_beats = loop_beats.max(4.0);
        let mut warnings = Vec::new();

        let first_bar = score.bar_lines.first().copied().unwrap_or(0.0);
        let start = if first_bar > 0.005 && first_bar < 2.0 {
            first_bar
        } else {
            0.0
        };

        let body_end = score.duration_beats.max(start + 1.0);
        let body_len = (body_end - start).max(1.0);

        if (body_len - loop_beats).abs() > 0.01 {
            if body_len < loop_beats {
                warnings.push(format!(
                    "Tune is {:.1} bars; extended to the 32-bar practice loop.",
                    body_len / 2.0
                ));
            } else {
                warnings.push(format!(
                    "Tune is {:.1} bars; trimmed to the 32-bar practice loop.",
                    body_len / 2.0
                ));
            }
        }

        let mut pickup = Vec::new();
        for note in &score.notes {
            if note.start_beat < start {
                let end = note.end_beat().min(start);
                if end > note.start_beat {
                    pickup.push(NoteEvent {
                        start_beat: note.start_beat - start,
                        duration_beats: end - note.start_beat,
                        ..*note
                    });
                }
            }
        }

        let mut body = Vec::new();
        for note in &score.notes {
            let note_start = note.start_beat.max(start);
            let note_end = note.end_beat().min(body_end);
            if note_end <= note_start {
                continue;
            }
            body.push(NoteEvent {
                start_beat: note_start - start,
                duration_beats: note_end - note_start,
                ..*note
            });
        }

        let repeats = (loop_beats / body_len).ceil().max(1.0) as usize;
        let mut melody = Vec::new();
        for repeat in 0..repeats {
            let offset = repeat as f64 * body_len;
            for note in &body {
                let note_start = note.start_beat + offset;
                let note_end = note.end_beat() + offset;
                if note_start >= loop_beats {
                    continue;
                }
                let note_end = note_end.min(loop_beats);
                if note_end <= note_start {
                    continue;
                }
                melody.push(NoteEvent {
                    start_beat: note_start,
                    duration_beats: note_end - note_start,
                    ..*note
                });
            }
        }
        melody.sort_by(|a, b| a.start_beat.total_cmp(&b.start_beat));

        let source_chords = resolve_chords(score);
        let mut body_chords = Vec::new();
        for chord in &source_chords {
            let chord_start = chord.start_beat.max(start);
            let chord_end = chord.end_beat().min(body_end);
            if chord_end <= chord_start {
                continue;
            }
            body_chords.push(ChordEvent {
                start_beat: chord_start - start,
                duration_beats: chord_end - chord_start,
                ..chord.clone()
            });
        }

        let mut chords = Vec::new();
        for repeat in 0..repeats {
            let offset = repeat as f64 * body_len;
            for chord in &body_chords {
                let chord_start = chord.start_beat + offset;
                if chord_start >= loop_beats {
                    continue;
                }
                let chord_end = (chord.end_beat() + offset).min(loop_beats);
                if chord_end <= chord_start {
                    continue;
                }
                chords.push(ChordEvent {
                    start_beat: chord_start,
                    duration_beats: chord_end - chord_start,
                    ..chord.clone()
                });
            }
        }
        fill_chord_gaps(&mut chords, loop_beats, &score.key);

        Self {
            melody,
            pickup,
            chords,
            loop_beats,
            body_beats: body_len,
            meter: score.meter,
            key: score.key.clone(),
            warnings,
            bowing,
        }
    }

    pub fn bars(&self) -> f64 {
        self.loop_beats / 2.0
    }

    /// Check that the dance and the tune body last the same number of beats.
    ///
    /// This is deliberately a check rather than a clamp: a dance card that is
    /// not 64 beats is valid (and the caller may choose a shorter or longer
    /// tune), but silently stretching or trimming the tune would hide a real
    /// mismatch and let the calls drift.
    pub fn check_dance_length(&self, dance_beats: f64) -> Result<(), LengthMismatch> {
        const TOLERANCE: f64 = 1e-6;
        if (dance_beats - self.body_beats).abs() > TOLERANCE {
            Err(LengthMismatch {
                dance_beats,
                tune_beats: self.body_beats,
            })
        } else {
            Ok(())
        }
    }

    pub fn count_in_chord(&self) -> Chord {
        self.chords
            .first()
            .and_then(|chord| Chord::parse(&chord.symbol))
            .unwrap_or_else(|| {
                Chord::from_root_quality(
                    self.key.tonic_pitch_class,
                    match self.key.mode {
                        crate::score::Mode::Minor
                        | crate::score::Mode::Dorian
                        | crate::score::Mode::Phrygian => crate::chord::ChordQuality::Minor,
                        _ => crate::chord::ChordQuality::Major,
                    },
                )
            })
    }

    /// Build the once-only slow ending.
    ///
    /// With [`EndingOptions::gradual`] the ending begins
    /// [`ENDING_GRADUAL_BEATS`] before the loop end, so the last two bars are
    /// replaced by a thinning accompaniment and an easing tempo instead of the
    /// transport jumping into a stretched final bar. Turning the option off
    /// keeps the older shape: only the last bar is replaced and it is stretched
    /// three times. [`EndingOptions::final_chord_down_fifth`] resolves the final
    /// harmony a perfect fifth below the written chord, and the closing melody
    /// note resolves with it.
    ///
    /// The ending replaces its region when the transport reaches
    /// [`Ending::from_beat`], so the tune's own closing music is not heard
    /// twice.
    pub fn ending(
        &self,
        bpm: f64,
        instrument: AccompanimentInstrument,
        rhythm: AccompanimentRhythm,
        options: &EndingOptions,
    ) -> Ending {
        let beat_seconds = 120.0 / bpm.max(1.0);
        let bar_beats = self.meter.bar_dance_beats().clamp(1.0, self.loop_beats);
        let region_beats = if options.gradual {
            ENDING_GRADUAL_BEATS
                .min(self.loop_beats)
                .max(bar_beats.min(self.loop_beats))
        } else {
            bar_beats
        };
        let region_start = (self.loop_beats - region_beats).max(0.0);
        let normal_seconds = region_beats * beat_seconds;
        let stretch = if options.gradual {
            ENDING_GRADUAL_STRETCH
        } else {
            ENDING_LAST_BAR_STRETCH
        };
        let ritard_seconds = normal_seconds * stretch;
        let linger_seconds = ENDING_LINGER_BEATS * beat_seconds;
        // Start at the session tempo, then stretch each beat more than the one
        // before: the linear term keeps the opening of the region honest and
        // the power term supplies the ritardando.
        let slowdown = (ritard_seconds - normal_seconds).max(0.0);
        let warp = |beat: f64| {
            let progress = ((beat - region_start) / region_beats).clamp(0.0, 1.0);
            normal_seconds * progress + slowdown * progress.powf(ENDING_SLOWDOWN_CURVE)
        };

        let mut melody = Vec::new();
        for note in self
            .melody
            .iter()
            .filter(|note| note.start_beat >= region_start - 1e-9)
        {
            let start = warp(note.start_beat);
            let end = warp(note.end_beat());
            melody.push(EndingNote {
                start_seconds: start,
                duration_seconds: (end - start).max(beat_seconds * 0.12),
                midi: note.midi,
                velocity: melody_velocity(note, false),
            });
        }
        melody.sort_by(|a, b| a.start_seconds.total_cmp(&b.start_seconds));

        // A tune can rest through its written last region. Hold the last pitch
        // it did play, under the final chord, rather than ending with no melody.
        if melody.is_empty()
            && let Some(note) = self.melody.last()
        {
            melody.push(EndingNote {
                start_seconds: ritard_seconds,
                duration_seconds: linger_seconds * 0.96,
                midi: note.midi,
                velocity: melody_velocity(note, false),
            });
        }

        // Hold the closing melody note into the final chord. When the final
        // chord resolves a fifth below the written chord, the melody resolves
        // with it: the written note holds until the chord arrives, then the
        // same voice re-attacks a perfect fifth lower under the held harmony.
        if let Some(last) = melody.last_mut() {
            if options.final_chord_down_fifth {
                if last.start_seconds < ritard_seconds {
                    let resolved = EndingNote {
                        start_seconds: ritard_seconds,
                        duration_seconds: (linger_seconds * 0.96).max(0.3),
                        midi: last.midi.saturating_sub(7),
                        velocity: last.velocity,
                    };
                    last.duration_seconds = (ritard_seconds - last.start_seconds).max(0.05);
                    melody.push(resolved);
                } else {
                    // The fallback already begins on the final chord, so the
                    // written note is the resolution itself.
                    last.midi = last.midi.saturating_sub(7);
                }
            } else if last.start_seconds < ritard_seconds {
                last.duration_seconds =
                    (ritard_seconds - last.start_seconds).max(0.05) + linger_seconds;
            }
        }

        let mut accompaniment =
            self.ending_region_accompaniment(instrument, rhythm, region_start, region_beats);
        if options.gradual {
            thin_ending_texture(&mut accompaniment, region_beats);
        }
        for note in &mut accompaniment {
            let start = warp(note.start_seconds);
            let end = warp(note.start_seconds + note.duration_seconds);
            note.start_seconds = start;
            note.duration_seconds = (end - start).max(beat_seconds * 0.08);
        }
        accompaniment.extend(final_chord_notes(
            &self.chord_at((self.loop_beats - 1e-6).max(0.0)),
            ritard_seconds,
            linger_seconds,
            options.final_chord_down_fifth,
        ));
        separate_ending_accompaniment(&melody, &mut accompaniment);
        accompaniment.sort_by(|a, b| a.start_seconds.total_cmp(&b.start_seconds));

        Ending {
            melody,
            accompaniment,
            from_beat: region_start,
            linger_start_seconds: ritard_seconds,
            seconds: ritard_seconds + linger_seconds,
        }
    }

    /// The ending region's backing notes, expressed in beats from its start.
    fn ending_region_accompaniment(
        &self,
        instrument: AccompanimentInstrument,
        rhythm: AccompanimentRhythm,
        region_start: f64,
        region_beats: f64,
    ) -> Vec<EndingNote> {
        let rhythm = resolve_rhythm(instrument, rhythm);
        let chord = self.chord_at(region_start);
        let mut notes = Vec::new();
        self.render_rhythm(
            &mut notes,
            rhythm,
            &chord,
            region_start,
            (region_start + region_beats).min(self.loop_beats),
        );
        notes
            .into_iter()
            .map(|note| EndingNote {
                start_seconds: note.start_beat - region_start,
                duration_seconds: note.duration_beats,
                midi: note.midi,
                velocity: note.velocity,
            })
            .collect()
    }

    /// Generate the plain backing track for the selected instrument.
    ///
    /// This is pass 0 with the instrument's default rhythm. MIDI export and
    /// callers that want one unchanged loop use it.
    pub fn accompaniment_notes(&self, instrument: AccompanimentInstrument) -> Vec<NoteEvent> {
        self.accompaniment_notes_with_rhythm(instrument, AccompanimentRhythm::Auto)
    }

    /// Generate the plain backing track for one instrument and rhythm.
    pub fn accompaniment_notes_with_rhythm(
        &self,
        instrument: AccompanimentInstrument,
        rhythm: AccompanimentRhythm,
    ) -> Vec<NoteEvent> {
        self.accompaniment_notes_for_pass_with_rhythm(instrument, rhythm, 0)
    }

    /// Generate the backing track for one loop pass using the instrument's
    /// default rhythm.
    pub fn accompaniment_notes_for_pass(
        &self,
        instrument: AccompanimentInstrument,
        pass: u32,
    ) -> Vec<NoteEvent> {
        self.accompaniment_notes_for_pass_with_rhythm(instrument, AccompanimentRhythm::Auto, pass)
    }

    /// Generate the backing track for one loop pass.
    ///
    /// `rhythm` chooses how the chord track is realised. `Auto` keeps each
    /// instrument's signature pattern (boom-chuck piano, strummed guitar,
    /// sustained accordion). Pass 0 is the plain pattern. Later passes are the
    /// same pattern with deterministic variations allowed by `options`: a few
    /// extra chord stabs for emphasis, and now and then one bar where the
    /// accompaniment drops out so the melody plays alone. The variations never
    /// move existing notes, so the pulse and the 64-beat grid are unchanged.
    pub fn accompaniment_notes_for_pass_with_rhythm(
        &self,
        instrument: AccompanimentInstrument,
        rhythm: AccompanimentRhythm,
        pass: u32,
    ) -> Vec<NoteEvent> {
        self.accompaniment_notes_for_pass_with_options(
            instrument,
            rhythm,
            pass,
            &VariationOptions::default(),
        )
    }

    /// Generate the backing track for one loop pass with explicit variation
    /// choices.
    pub fn accompaniment_notes_for_pass_with_options(
        &self,
        instrument: AccompanimentInstrument,
        rhythm: AccompanimentRhythm,
        pass: u32,
        options: &VariationOptions,
    ) -> Vec<NoteEvent> {
        let mut notes = self.plain_accompaniment(instrument, rhythm);
        if pass > 0 {
            self.vary_accompaniment(&mut notes, pass, options);
        }
        self.separate_accompaniment_from_melody(&mut notes);
        notes.sort_by(|a, b| {
            a.start_beat
                .total_cmp(&b.start_beat)
                .then(a.midi.cmp(&b.midi))
        });
        notes
    }

    /// The countermelody voice: a melody instrument under the tune.
    ///
    /// This is what makes the default session a trio. It places one note per
    /// dance beat, a diatonic third below the melody note sounding at that beat,
    /// snapped to a nearby chord tone, and leaves the beat open rather than
    /// repeating a pitch. It is derived from the melody, key, and chord track
    /// only, so it stays on the same 64-beat grid and adds no new timing.
    pub fn countermelody_notes(&self) -> Vec<NoteEvent> {
        self.countermelody_notes_for_pass(0)
    }

    /// The countermelody for one loop pass.
    ///
    /// When the accompaniment rests a bar on a variation pass, the
    /// countermelody rests with it so the melody really is alone.
    pub fn countermelody_notes_for_pass(&self, pass: u32) -> Vec<NoteEvent> {
        self.countermelody_notes_for_pass_with_options(pass, &VariationOptions::default())
    }

    /// The countermelody for one loop pass with explicit variation choices.
    pub fn countermelody_notes_for_pass_with_options(
        &self,
        pass: u32,
        options: &VariationOptions,
    ) -> Vec<NoteEvent> {
        if self.melody.is_empty() {
            return Vec::new();
        }
        let rest_bar = self.variation_rest_bar(pass, options);
        let mut notes = Vec::new();
        let mut previous = None;
        let mut beat = 0.0;
        while beat < self.loop_beats - 1e-9 {
            let step = 0.5_f64.min(self.loop_beats - beat);
            let resting = rest_bar.is_some_and(|bar| {
                let start = bar as f64 * 2.0;
                beat >= start - 1e-9 && beat < start + 2.0 - 1e-9
            });
            if resting {
                // Re-enter cleanly after the melody-only bar.
                previous = None;
                beat += step;
                continue;
            }
            if let Some(melody) = self.melody_note_at(beat)
                && let Some(midi) = self.counter_pitch(&self.chord_at(beat), melody.midi)
            {
                if Some(midi) == previous {
                    // Leave the beat open rather than re-articulating the same
                    // pitch; the line places notes, it does not drone.
                    previous = None;
                } else {
                    notes.push(NoteEvent {
                        start_beat: beat,
                        duration_beats: (step * 0.9).min(self.loop_beats - beat),
                        midi,
                        velocity: 62,
                    });
                    previous = Some(midi);
                }
            }
            beat += step;
        }
        // The countermelody is a fiddle line too, so the same bowing rules
        // shape it when its rhythm happens to contain the figures.
        crate::bowing::apply_with_options(&mut notes, self.meter, &self.bowing);
        notes
    }

    /// The melody note sounding at `beat`, or the next one inside the step.
    fn melody_note_at(&self, beat: f64) -> Option<&NoteEvent> {
        self.melody
            .iter()
            .find(|note| beat >= note.start_beat - 1e-9 && beat < note.end_beat() - 1e-9)
            .or_else(|| {
                self.melody
                    .iter()
                    .find(|note| note.start_beat > beat && note.start_beat < beat + 0.5)
            })
    }

    /// A diatonic third below `melody`, snapped to the chord when a chord tone
    /// is within a whole tone. The line therefore moves with the tune instead of
    /// holding a pedal while the melody passes by.
    fn counter_pitch(&self, chord: &Chord, melody: u8) -> Option<u8> {
        let scale = self.key.mode.intervals();
        let relative = (melody as i32 - self.key.tonic_pitch_class).rem_euclid(12);
        let Some(degree) = scale.iter().position(|interval| *interval == relative) else {
            return self.chord_tone_below(chord, melody);
        };
        // Two scale degrees down, wrapping into the octave below for the tonic
        // and second degrees.
        let offset = if degree >= 2 {
            scale[degree] - scale[degree - 2]
        } else {
            scale[degree] + 12 - scale[degree + 5]
        };
        let mut midi = melody as i32 - offset;
        let tones = chord_tone_classes(chord);
        if !tones.contains(&midi.rem_euclid(12)) {
            for delta in [1, -1, 2, -2] {
                let candidate = midi + delta;
                if candidate <= melody as i32 - 3
                    && (45..=86).contains(&candidate)
                    && tones.contains(&candidate.rem_euclid(12))
                {
                    midi = candidate;
                    break;
                }
            }
        }
        ((45..=86).contains(&midi) && midi <= melody as i32 - 3).then_some(midi as u8)
    }

    /// The highest chord tone within an octave below a melody note that is not
    /// on the key's scale.
    fn chord_tone_below(&self, chord: &Chord, melody: u8) -> Option<u8> {
        let tones = chord_tone_classes(chord);
        (melody as i32 - 12..=melody as i32 - 3)
            .rev()
            .find(|midi| (45..=86).contains(midi) && tones.contains(&midi.rem_euclid(12)))
            .map(|midi| midi as u8)
    }

    /// Whether `midi` sits closer than [`MIN_MELODY_SEPARATION_SEMITONES`] to a
    /// melody note sounding anywhere between `start` and `end`.
    fn crowds_melody(&self, midi: u8, start: f64, end: f64) -> bool {
        self.melody.iter().any(|melody| {
            melody.start_beat < end - 1e-9
                && melody.end_beat() > start + 1e-9
                && (melody.midi as i32 - midi as i32).abs() < MIN_MELODY_SEPARATION_SEMITONES
        })
    }

    /// Move accompaniment notes that crowd the melody down an octave.
    ///
    /// The melody is the line the dancers follow; a chord tone within a whole
    /// tone of it blurs into the tune instead of supporting it. Any note that
    /// lands closer than a minor third to an overlapping melody note is
    /// dropped by an octave, repeating while a lower melody note is still too
    /// close. The bass already sits below the melody and the countermelody is
    /// deliberately a third below it, so both are left where they are.
    fn separate_accompaniment_from_melody(&self, notes: &mut [NoteEvent]) {
        for note in notes {
            while note.midi.saturating_sub(MIN_ACCOMPANIMENT_MIDI) >= 12
                && self.crowds_melody(note.midi, note.start_beat, note.end_beat())
            {
                note.midi -= 12;
            }
        }
    }

    /// The melody for one loop pass.
    ///
    /// The plain tune is lifted above the accompaniment and accented on the
    /// dance pulse. A quarter followed by two eighths before another quarter is
    /// bowed as a fiddle figure: the eighths are re-struck and accented (see
    /// [`crate::bowing`]). A run of four to eight successive eighths is instead
    /// taken in one bow, each one ringing into the next. Later passes add
    /// deterministic melody variation allowed by `options`: a little
    /// articulation, chord-tone harmony doubles on strong beats, diatonic grace
    /// notes, and an occasional octave lift. Existing note starts never move,
    /// so the dance timing and the 64-beat loop are unchanged.
    pub fn melody_notes_for_pass(&self, pass: u32) -> Vec<NoteEvent> {
        self.melody_notes_for_pass_with_options(pass, &VariationOptions::default())
    }

    /// The melody for one loop pass with explicit variation choices.
    pub fn melody_notes_for_pass_with_options(
        &self,
        pass: u32,
        options: &VariationOptions,
    ) -> Vec<NoteEvent> {
        let mut notes = Vec::with_capacity(self.melody.len());
        for (index, note) in self.melody.iter().enumerate() {
            notes.push(NoteEvent {
                velocity: melody_velocity(note, index == 0),
                ..*note
            });
        }
        // Shape the plain line before the ornaments: the bowing rules belong to
        // the tune, and a grace note added later must not hide a figure.
        crate::bowing::apply_with_options(&mut notes, self.meter, &self.bowing);
        if pass > 0 {
            self.vary_melody(&mut notes, pass, options);
        }
        notes.sort_by(|a, b| {
            a.start_beat
                .total_cmp(&b.start_beat)
                .then(a.midi.cmp(&b.midi))
        });
        notes
    }

    fn plain_accompaniment(
        &self,
        instrument: AccompanimentInstrument,
        rhythm: AccompanimentRhythm,
    ) -> Vec<NoteEvent> {
        let rhythm = resolve_rhythm(instrument, rhythm);
        let mut notes = Vec::new();
        for chord_event in &self.chords {
            let chord = Chord::parse(&chord_event.symbol).unwrap_or_else(|| self.count_in_chord());
            let start = chord_event.start_beat;
            let end = chord_event.end_beat().min(self.loop_beats);
            if end <= start {
                continue;
            }
            self.render_rhythm(&mut notes, rhythm, &chord, start, end);
        }
        notes
    }

    fn render_rhythm(
        &self,
        notes: &mut Vec<NoteEvent>,
        rhythm: AccompanimentRhythm,
        chord: &Chord,
        start: f64,
        end: f64,
    ) {
        match rhythm {
            // Resolved to a concrete pattern in `plain_accompaniment`.
            AccompanimentRhythm::Auto => {}
            AccompanimentRhythm::Block => {
                let mut beat = start;
                while beat < end {
                    push_note(notes, beat, 0.9, chord.bass_midi(), 58, end);
                    push_chord(notes, beat, 0.9, chord, 55, 52, end);
                    beat += 1.0;
                }
            }
            AccompanimentRhythm::BoomChuck => {
                let mut step = 0;
                let mut beat = start;
                while beat < end {
                    if step % 2 == 0 {
                        push_note(notes, beat, 0.45, chord.bass_midi(), 58, end);
                    } else {
                        push_chord(notes, beat, 0.45, chord, 55, 52, end);
                    }
                    beat += 0.5;
                    step += 1;
                }
            }
            AccompanimentRhythm::Offbeat => {
                // A held bass anchors each bar while the chords answer between
                // the beats.
                let mut bar = start;
                while bar < end {
                    push_note(notes, bar, 1.8, chord.bass_midi(), 56, end);
                    push_chord(notes, bar + 0.5, 0.3, chord, 55, 56, end);
                    push_chord(notes, bar + 1.5, 0.3, chord, 55, 56, end);
                    bar += 2.0;
                }
            }
            AccompanimentRhythm::Arpeggio => {
                let voicing = chord.voicing(55);
                let mut step = 0;
                let mut beat = start;
                while beat < end {
                    let midi = if step % 4 == 0 || voicing.is_empty() {
                        chord.bass_midi()
                    } else {
                        voicing[(step - 1) % voicing.len()]
                    };
                    push_note(notes, beat, 0.45, midi, 52, end);
                    beat += 0.5;
                    step += 1;
                }
            }
            AccompanimentRhythm::Shuffle => {
                // Long-short pairs: the offbeat chord lands two thirds of the
                // way through each beat.
                let mut bar = start;
                while bar < end {
                    push_note(notes, bar, 0.4, chord.bass_midi(), 58, end);
                    push_chord(notes, bar + 0.67, 0.3, chord, 55, 52, end);
                    push_note(notes, bar + 1.0, 0.4, chord.bass_midi(), 58, end);
                    push_chord(notes, bar + 1.67, 0.3, chord, 55, 52, end);
                    bar += 2.0;
                }
            }
            AccompanimentRhythm::Strum => {
                let voicing = chord.voicing(55);
                let mut beat = start;
                let mut down = true;
                while beat < end {
                    if down {
                        // Down strum: the bass string leads and the pick rolls
                        // up through the chord.
                        for (index, midi) in voicing.iter().enumerate() {
                            push_note(notes, beat + index as f64 * 0.025, 0.42, *midi, 58, end);
                        }
                    } else {
                        // Up strum: the treble leads and the pick falls back
                        // toward the bass. Dropping the lowest string and
                        // easing the attack keeps the offbeat lighter than the
                        // downbeat.
                        for (index, midi) in voicing.iter().skip(1).rev().enumerate() {
                            push_note(notes, beat + index as f64 * 0.018, 0.30, *midi, 44, end);
                        }
                    }
                    down = !down;
                    beat += 0.5;
                }
            }
            AccompanimentRhythm::Sustained => {
                let duration = (end - start).min(2.0) * 0.95;
                push_note(notes, start, duration, chord.bass_midi(), 55, end);
                push_chord(notes, start, duration, chord, 60, 50, end);
            }
        }
    }

    /// Apply one pass of melody variations in place.
    ///
    /// Additions sit behind the plain melody and use their own seed stream, so
    /// they do not change which accompaniment variation a pass receives.
    fn vary_melody(&self, notes: &mut Vec<NoteEvent>, pass: u32, options: &VariationOptions) {
        let mut rng = VariationRng::new(self.variation_seed(), pass ^ 0x5EED_1234);
        let mut additions = Vec::new();

        for note in notes.iter_mut() {
            // Alternate passes are a touch more detached; the others stay
            // legato. This changes articulation without moving any start.
            if options.articulation && pass % 2 == 1 && note.duration_beats >= 0.75 {
                note.duration_beats = (note.duration_beats * 0.82).max(0.25);
            }

            if note.duration_beats < 0.4 {
                continue;
            }

            let strong = is_pulse(note.start_beat);
            if options.double_stops
                && strong
                && rng.chance(3)
                && let Some(harmony) =
                    self.chord_tone_below(&self.chord_at(note.start_beat), note.midi)
            {
                additions.push(NoteEvent {
                    start_beat: note.start_beat,
                    duration_beats: note.duration_beats,
                    midi: harmony,
                    velocity: (note.velocity as u16).saturating_sub(18).max(72) as u8,
                });
            }

            // A short diatonic grace note leads into some longer notes.
            if options.grace_notes
                && note.duration_beats >= 0.75
                && note.start_beat >= 0.25
                && rng.chance(5)
            {
                let up = rng.chance(2);
                if let Some(pitch) = diatonic_neighbor(note.midi, &self.key, up) {
                    additions.push(NoteEvent {
                        start_beat: note.start_beat - 0.125,
                        duration_beats: 0.125,
                        midi: pitch,
                        velocity: 84,
                    });
                }
            }

            // Later passes occasionally lift a held note an octave, which adds
            // sparkle without disturbing the underlying line.
            if options.octave_lift
                && pass >= 2
                && note.duration_beats >= 1.0
                && note.midi + 12 <= 100
                && rng.chance(4)
            {
                additions.push(NoteEvent {
                    start_beat: note.start_beat,
                    duration_beats: note.duration_beats,
                    midi: note.midi + 12,
                    velocity: (note.velocity as u16).saturating_sub(24).max(68) as u8,
                });
            }
        }

        for note in additions {
            if note.start_beat >= 0.0 && note.end_beat() <= self.loop_beats + 1e-9 {
                notes.push(note);
            }
        }
    }

    /// Apply one pass of variations in place.
    ///
    /// The seed comes from the arrangement itself, so a given tune always
    /// produces the same variations and the audio, like the rest of `music`,
    /// stays deterministic and offline.
    fn vary_accompaniment(
        &self,
        notes: &mut Vec<NoteEvent>,
        pass: u32,
        options: &VariationOptions,
    ) {
        let mut rng = VariationRng::new(self.variation_seed(), pass);
        let bars = self.bars().floor() as i64;

        let rest_bar = self.variation_rest_bar(pass, options);
        if let Some(bar) = rest_bar {
            let rest_start = bar as f64 * 2.0;
            let rest_end = (rest_start + 2.0).min(self.loop_beats);
            notes.retain(|note| note.start_beat < rest_start - 1e-9 || note.start_beat >= rest_end);
        }

        // A few short chord stabs on the back half of the bar, above the
        // regular pattern, add emphasis without disturbing the pulse.
        if !options.chord_stabs {
            return;
        }
        for bar in 0..bars {
            if rest_bar == Some(bar) {
                continue;
            }
            if !rng.chance(6) {
                continue;
            }
            let at = bar as f64 * 2.0 + 1.5;
            if at >= self.loop_beats {
                continue;
            }
            for midi in self.chord_at(at).voicing(64) {
                notes.push(NoteEvent {
                    start_beat: at,
                    duration_beats: 0.3,
                    midi,
                    velocity: 46,
                });
            }
        }
    }

    /// The bar whose accompaniment drops out on `pass`, if any.
    ///
    /// Rarely leave one bar to the melody. Skip the opening bars so the
    /// dancers always get a clear start, and only consider bars where the tune
    /// itself has enough notes to carry the bar alone. A rest bar chosen over a
    /// written rest would leave a silent measure, which is the one thing a
    /// dance cannot use. The choice is deterministic and shared by the
    /// accompaniment and the countermelody.
    fn variation_rest_bar(&self, pass: u32, options: &VariationOptions) -> Option<i64> {
        if !options.melody_alone_bar {
            return None;
        }
        let bars = self.bars().floor() as i64;
        if pass < 2 || bars <= 7 {
            return None;
        }
        let mut rng = VariationRng::new(self.variation_seed(), pass);
        if !rng.chance(3) {
            return None;
        }
        let first = 4;
        let last = bars - 3;
        let candidates: Vec<i64> = (first..=last)
            .filter(|bar| {
                let start = *bar as f64 * 2.0;
                self.melody_coverage(start, start + 2.0) >= MIN_MELODY_ALONE_BEATS
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        Some(candidates[rng.below(candidates.len() as u64) as usize])
    }

    /// How much of `[start, end)` the melody fills, in dance beats.
    ///
    /// Overlapping and adjacent notes are merged, so a rest in the written
    /// tune is measured as a rest rather than as part of the surrounding notes.
    fn melody_coverage(&self, start: f64, end: f64) -> f64 {
        let mut spans: Vec<(f64, f64)> = self
            .melody
            .iter()
            .map(|note| (note.start_beat.max(start), note.end_beat().min(end)))
            .filter(|(note_start, note_end)| *note_end > *note_start + 1e-9)
            .collect();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut covered = 0.0;
        let mut cursor = start;
        for (span_start, span_end) in spans {
            if span_end <= cursor + 1e-9 {
                continue;
            }
            let span_start = span_start.max(cursor);
            covered += span_end - span_start;
            cursor = span_end;
        }
        covered.min(end - start).max(0.0)
    }

    fn chord_at(&self, beat: f64) -> Chord {
        self.chords
            .iter()
            .find(|chord| beat >= chord.start_beat && beat < chord.end_beat())
            .and_then(|chord| Chord::parse(&chord.symbol))
            .unwrap_or_else(|| self.count_in_chord())
    }

    /// FNV-1a over the chord track and the first part of the melody. The tune
    /// content selects the variations; nothing external is involved.
    fn variation_seed(&self) -> u64 {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        let mut hash = OFFSET;
        let mut feed = |value: u64| {
            hash ^= value;
            hash = hash.wrapping_mul(PRIME);
        };
        for chord in &self.chords {
            for byte in chord.symbol.bytes() {
                feed(byte as u64);
            }
            feed((chord.start_beat * 1000.0).round() as u64);
        }
        for note in self.melody.iter().take(64) {
            feed(note.midi as u64);
            feed((note.start_beat * 1000.0).round() as u64);
        }
        hash
    }
}

/// Pitch classes of a chord's tones, relative to C.
fn chord_tone_classes(chord: &Chord) -> Vec<i32> {
    chord
        .quality
        .intervals()
        .iter()
        .map(|interval| (chord.root + interval).rem_euclid(12))
        .collect()
}

/// Lift a melody note above the accompaniment and accent the dance pulse.
///
/// A bar is two dance beats: the bar start gets the strongest accent, the
/// second beat a smaller one. The first melody note gets a little extra so the
/// tune announces itself.
fn melody_velocity(note: &NoteEvent, first: bool) -> u8 {
    let beat_in_bar = note.start_beat.rem_euclid(2.0);
    let accent = if beat_in_bar < 0.01 {
        14
    } else if (beat_in_bar - 1.0).abs() < 0.01 {
        5
    } else {
        0
    };
    let lift = if first { 4 } else { 0 };
    (note.velocity.max(MELODY_VELOCITY_FLOOR) as u16 + accent + lift)
        .min(MELODY_VELOCITY_CEILING as u16) as u8
}

/// True for notes that begin on one of the two dance beats in a bar.
fn is_pulse(beat: f64) -> bool {
    let in_bar = beat.rem_euclid(2.0);
    in_bar < 0.01 || (in_bar - 1.0).abs() < 0.01
}

/// The nearest scale note above or below `midi`, used for grace notes.
fn diatonic_neighbor(midi: u8, key: &Key, up: bool) -> Option<u8> {
    let scale: Vec<i32> = key
        .mode
        .intervals()
        .iter()
        .map(|interval| (key.tonic_pitch_class + interval).rem_euclid(12))
        .collect();
    let direction = if up { 1 } else { -1 };
    for step in 1..=2 {
        let candidate = midi as i32 + direction * step;
        if !(24..=108).contains(&candidate) {
            return None;
        }
        if scale.contains(&candidate.rem_euclid(12)) {
            return Some(candidate as u8);
        }
    }
    None
}

/// Add one accompaniment note, clipped to the harmony's end.
fn push_note(
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
    let duration_beats = duration_beats.min(limit - start_beat);
    if duration_beats <= 1e-6 {
        return;
    }
    notes.push(NoteEvent {
        start_beat,
        duration_beats,
        midi,
        velocity,
    });
}

/// Add every note of a close-position voicing at the same time.
fn push_chord(
    notes: &mut Vec<NoteEvent>,
    start_beat: f64,
    duration_beats: f64,
    chord: &Chord,
    low: i32,
    velocity: u8,
    limit: f64,
) {
    for midi in chord.voicing(low) {
        push_note(notes, start_beat, duration_beats, midi, velocity, limit);
    }
}

/// Resolve an instrument's signature pattern when the user chose "match
/// instrument".
fn resolve_rhythm(
    instrument: AccompanimentInstrument,
    rhythm: AccompanimentRhythm,
) -> AccompanimentRhythm {
    match rhythm {
        AccompanimentRhythm::Auto => match instrument {
            AccompanimentInstrument::Piano => AccompanimentRhythm::BoomChuck,
            AccompanimentInstrument::Guitar => AccompanimentRhythm::Strum,
            AccompanimentInstrument::Accordion => AccompanimentRhythm::Sustained,
        },
        explicit => explicit,
    }
}

/// The held final harmony: a low bass note plus a full close-position chord.
///
/// With `down_fifth` the whole chord is resolved a perfect fifth below the
/// written final chord (root minus seven semitones). The written slash bass is
/// dropped, because the resolving chord's own root is the bass the cadence
/// wants.
fn final_chord_notes(
    chord: &Chord,
    start: f64,
    linger_seconds: f64,
    down_fifth: bool,
) -> Vec<EndingNote> {
    let resolving;
    let chord = if down_fifth {
        resolving = Chord {
            root: (chord.root - 7).rem_euclid(12),
            quality: chord.quality,
            bass: None,
        };
        &resolving
    } else {
        chord
    };
    let duration_seconds = (linger_seconds * 0.96).max(0.3);
    let mut notes = vec![EndingNote {
        start_seconds: start,
        duration_seconds,
        midi: chord.bass_midi(),
        velocity: 66,
    }];
    for midi in chord.voicing(55) {
        notes.push(EndingNote {
            start_seconds: start,
            duration_seconds,
            midi,
            velocity: 58,
        });
    }
    for midi in chord.voicing(67) {
        notes.push(EndingNote {
            start_seconds: start,
            duration_seconds,
            midi,
            velocity: 48,
        });
    }
    notes
}

/// Thin the ending's accompaniment as it approaches the final chord.
///
/// This is what makes the gradual ending read as an ending rather than a
/// sudden swap: the upper chord tones leave first, then the bass pulse, so the
/// tune is left alone before the final chord arrives. Notes that have already
/// started keep their full length; nothing is cut off mid-sustain.
fn thin_ending_texture(notes: &mut Vec<EndingNote>, region_beats: f64) {
    let region = region_beats.max(1e-9);
    let original = notes.clone();
    notes.retain(|note| {
        let progress = (note.start_seconds / region).clamp(0.0, 1.0);
        if progress >= 0.75 {
            return false;
        }
        if progress >= 0.5 {
            // Keep only the lowest note of each attack: the chord thins to a
            // single walking bass line.
            let lowest = original
                .iter()
                .filter(|other| (other.start_seconds - note.start_seconds).abs() < 1e-9)
                .map(|other| other.midi)
                .min();
            return lowest == Some(note.midi);
        }
        true
    });
}

/// Apply the melody-separation rule to the once-only ending.
///
/// The ending is timed in seconds rather than dance beats, so it needs its own
/// overlap test; the rule itself is the same as for the loop.
fn separate_ending_accompaniment(melody: &[EndingNote], accompaniment: &mut [EndingNote]) {
    for note in accompaniment {
        while note.midi.saturating_sub(MIN_ACCOMPANIMENT_MIDI) >= 12
            && melody.iter().any(|melody| {
                melody.start_seconds < note.start_seconds + note.duration_seconds - 1e-9
                    && melody.start_seconds + melody.duration_seconds > note.start_seconds + 1e-9
                    && (melody.midi as i32 - note.midi as i32).abs()
                        < MIN_MELODY_SEPARATION_SEMITONES
            })
        {
            note.midi -= 12;
        }
    }
}

/// Small deterministic generator (SplitMix64) used for the variation choices.
struct VariationRng {
    state: u64,
}

impl VariationRng {
    fn new(seed: u64, pass: u32) -> Self {
        Self {
            state: seed ^ (pass as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
        }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// True roughly once in `n`.
    fn chance(&mut self, n: u64) -> bool {
        n > 0 && self.next().is_multiple_of(n)
    }

    fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
}

fn fill_chord_gaps(chords: &mut Vec<ChordEvent>, loop_beats: f64, key: &Key) {
    chords.sort_by(|a, b| a.start_beat.total_cmp(&b.start_beat));
    if chords.is_empty() {
        let tonic = Chord::from_root_quality(
            key.tonic_pitch_class,
            match key.mode {
                crate::score::Mode::Minor
                | crate::score::Mode::Dorian
                | crate::score::Mode::Phrygian => crate::chord::ChordQuality::Minor,
                _ => crate::chord::ChordQuality::Major,
            },
        );
        chords.push(ChordEvent {
            start_beat: 0.0,
            duration_beats: loop_beats,
            symbol: tonic.symbol(),
        });
        return;
    }

    if chords[0].start_beat > 0.005 {
        let symbol = chords[0].symbol.clone();
        let duration = chords[0].start_beat;
        chords.insert(
            0,
            ChordEvent {
                start_beat: 0.0,
                duration_beats: duration,
                symbol,
            },
        );
    }

    for index in 0..chords.len() {
        let desired_end = chords
            .get(index + 1)
            .map(|next| next.start_beat)
            .unwrap_or(loop_beats);
        if chords[index].end_beat() < desired_end {
            chords[index].duration_beats = (desired_end - chords[index].start_beat).max(0.125);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::Meter;

    fn legacy_ending() -> EndingOptions {
        EndingOptions {
            gradual: false,
            final_chord_down_fifth: false,
        }
    }

    fn simple_score() -> Score {
        let mut notes = Vec::new();
        for index in 0..32 {
            notes.push(NoteEvent {
                start_beat: index as f64,
                duration_beats: 1.0,
                midi: 62 + (index % 3) as u8,
                velocity: 80,
            });
        }
        Score {
            title: "Simple".into(),
            meter: Meter::default(),
            key: Key::parse("D").unwrap(),
            unit_note_length: 0.125,
            notes,
            chords: vec![ChordEvent {
                start_beat: 0.0,
                duration_beats: 64.0,
                symbol: "D".into(),
            }],
            bar_lines: (0..32).map(|index| index as f64 * 2.0).collect(),
            duration_beats: 64.0,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn produces_a_sixty_four_beat_loop() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        assert_eq!(arrangement.loop_beats, 64.0);
        assert!(!arrangement.melody.is_empty());
        assert!(arrangement.melody.iter().all(|note| note.start_beat < 64.0));
        assert_eq!(arrangement.chords[0].start_beat, 0.0);
    }

    #[test]
    fn extends_a_short_tune() {
        let mut score = simple_score();
        score.duration_beats = 32.0;
        score.notes.retain(|note| note.start_beat < 32.0);
        score.chords = Vec::new();
        let arrangement = Arrangement::from_score(&score, 64.0);
        assert!(
            arrangement
                .melody
                .iter()
                .any(|note| note.start_beat >= 32.0)
        );
        assert!(
            arrangement
                .chords
                .iter()
                .any(|chord| chord.start_beat >= 32.0),
            "chords: {:?}",
            arrangement
                .chords
                .iter()
                .map(|chord| chord.start_beat)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn countermelody_fills_the_loop_below_the_tune() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let notes = arrangement.countermelody_notes();
        assert!(!notes.is_empty(), "the countermelody must play");
        assert!(
            notes
                .iter()
                .all(|note| { note.start_beat >= 0.0 && note.end_beat() <= 64.0 + 1e-6 })
        );
        assert!(
            notes
                .iter()
                .all(|note| { (note.start_beat * 2.0).fract().abs() < 1e-6 }),
            "the countermelody sits on the dance-beat grid"
        );
        for note in &notes {
            let melody = arrangement
                .melody
                .iter()
                .find(|melody| {
                    note.start_beat >= melody.start_beat - 1e-9
                        && note.start_beat < melody.end_beat() - 1e-9
                })
                .or_else(|| {
                    arrangement.melody.iter().find(|melody| {
                        melody.start_beat > note.start_beat
                            && melody.start_beat < note.start_beat + 0.5
                    })
                })
                .expect("every countermelody note has a melody note");
            assert!(
                melody.midi as i32 - note.midi as i32 >= 3,
                "countermelody note {} must sit below melody {}",
                note.midi,
                melody.midi
            );
        }
    }

    #[test]
    fn countermelody_sits_on_the_tunes_scale_and_moves() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let notes = arrangement.countermelody_notes();
        let key = &arrangement.key;
        let scale = key.mode.intervals();
        for note in &notes {
            let relative = (note.midi as i32 - key.tonic_pitch_class).rem_euclid(12);
            let absolute = (note.midi as i32).rem_euclid(12);
            let chord = chord_tone_classes(&arrangement.chord_at(note.start_beat));
            assert!(
                scale.contains(&relative) || chord.contains(&absolute),
                "countermelody note {} should belong to {} or its chord",
                note.midi,
                key.tonic_name
            );
        }
        assert!(
            notes.windows(2).any(|pair| pair[0].midi != pair[1].midi),
            "the countermelody must move with the tune"
        );
    }

    #[test]
    fn countermelody_handles_a_compound_meter() {
        let abc = "X:1\nT:Jig\nM:6/8\nL:1/8\nK:D\n\"D\"A2 A A2 A | \"G\"B2 B B2 B |\n";
        let score = crate::abc::parse_abc(abc).expect("parse jig");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let notes = arrangement.countermelody_notes();
        assert!(!notes.is_empty());
        assert!(notes.iter().all(|note| {
            note.start_beat >= 0.0
                && note.end_beat() <= arrangement.loop_beats + 1e-6
                && (note.start_beat * 2.0).fract().abs() < 1e-6
        }));
    }

    #[test]
    fn countermelody_is_deterministic_and_rests_with_the_variations() {
        let first = Arrangement::from_score(&simple_score(), 64.0);
        let second = Arrangement::from_score(&simple_score(), 64.0);
        let mut saw_rest = false;
        for pass in 0..16 {
            assert_eq!(
                first.countermelody_notes_for_pass(pass),
                second.countermelody_notes_for_pass(pass),
            );
            let Some(bar) = first.variation_rest_bar(pass, &VariationOptions::default()) else {
                continue;
            };
            saw_rest = true;
            let start = bar as f64 * 2.0;
            assert!(
                first
                    .countermelody_notes_for_pass(pass)
                    .iter()
                    .all(|note| note.start_beat < start - 1e-9 || note.start_beat >= start + 2.0),
                "pass {pass} must leave bar {bar} to the melody"
            );
        }
        assert!(saw_rest, "the variation pass set should include a rest bar");
    }

    #[test]
    fn melody_is_lifted_and_accented_on_the_pulse() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let plain = arrangement.melody_notes_for_pass(0);
        assert_eq!(plain.len(), arrangement.melody.len());
        assert!(
            plain.iter().all(|note| note.velocity >= 92),
            "the melody must sit above the accompaniment"
        );
        assert!(
            plain
                .iter()
                .zip(arrangement.melody.iter())
                .all(|(lifted, source)| lifted.velocity >= source.velocity)
        );
        let bar_start = plain
            .iter()
            .find(|note| note.start_beat.abs() < 1e-9)
            .expect("opening note");
        let second_beat = plain
            .iter()
            .find(|note| (note.start_beat - 1.0).abs() < 1e-9)
            .expect("second dance beat");
        assert!(
            bar_start.velocity > second_beat.velocity,
            "the bar start should carry the strongest accent"
        );
    }

    #[test]
    fn a_quarter_two_eighths_quarter_is_bowed_with_a_new_stroke() {
        let abc = "X:1\nT:T\nM:4/4\nL:1/8\nK:D\n\"D\"A2 AA A2 A2|\n";
        let score = crate::abc::parse_abc(abc).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let melody = arrangement.melody_notes_for_pass(0);

        let at = |beat: f64| {
            melody
                .iter()
                .find(|note| (note.start_beat - beat).abs() < 1e-9)
                .unwrap_or_else(|| panic!("no melody note at beat {beat}"))
        };
        let first_quarter = at(0.0);
        let first_eighth = at(0.5);
        let second_eighth = at(0.75);
        let last_quarter = at(1.0);
        for eighth in [first_eighth, second_eighth] {
            assert!(
                eighth.velocity > first_quarter.velocity && eighth.velocity > last_quarter.velocity,
                "the two eighths must be accented above the surrounding quarters \
                 ({} and {} vs {} and {})",
                first_eighth.velocity,
                second_eighth.velocity,
                first_quarter.velocity,
                last_quarter.velocity
            );
            assert!(
                eighth.duration_beats < 0.25,
                "the eighth at beat {} must be re-struck, not slurred",
                eighth.start_beat
            );
        }
        // The plain line keeps its written grid and the quarters keep their
        // written length.
        assert_eq!(first_quarter.duration_beats, 0.5);
        assert_eq!(last_quarter.duration_beats, 0.5);
        assert!(
            melody
                .iter()
                .zip(arrangement.melody.iter())
                .all(|(shaped, written)| (shaped.start_beat - written.start_beat).abs() < 1e-9)
        );
    }

    #[test]
    fn four_successive_eighths_are_tied_in_the_arranged_line() {
        // Quarter, four eighths, quarter: one bar of running eighths between
        // two quarter notes.
        let abc = "X:1\nT:T\nM:4/4\nL:1/8\nK:D\n\"D\"A2 Bcdc A2|\n";
        let score = crate::abc::parse_abc(abc).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let melody = arrangement.melody_notes_for_pass(0);

        let run: Vec<&NoteEvent> = melody
            .iter()
            .filter(|note| note.start_beat >= 0.5 - 1e-9 && note.start_beat < 1.5 - 1e-9)
            .collect();
        assert_eq!(run.len(), 4, "the bar should hold four eighth notes");
        assert!(
            run[..3].iter().all(|note| note.duration_beats > 0.25),
            "the first three eighths should ring into the next one"
        );
        assert_eq!(run[3].duration_beats, 0.25, "the last eighth ends the bow");
        assert!(
            melody
                .iter()
                .zip(arrangement.melody.iter())
                .all(|(shaped, written)| (shaped.start_beat - written.start_beat).abs() < 1e-9)
        );
    }

    #[test]
    fn the_arrangement_honours_the_bowing_switches() {
        let abc = "X:1\nT:T\nM:4/4\nL:1/8\nK:D\n\"D\"A2 AA A2 A2|\n";
        let score = crate::abc::parse_abc(abc).expect("parse");
        let plain = Arrangement::from_score_with_bowing(
            &score,
            64.0,
            BowingOptions {
                bow_change: false,
                tied_runs: false,
            },
        )
        .melody_notes_for_pass(0);
        let eighth = plain
            .iter()
            .find(|note| (note.start_beat - 0.5).abs() < 1e-9)
            .expect("eighth note");
        assert_eq!(
            eighth.velocity, 92,
            "the un-bowed eighth keeps its velocity"
        );
        assert_eq!(eighth.duration_beats, 0.25, "and its written length");

        let bowed = Arrangement::from_score(&score, 64.0).melody_notes_for_pass(0);
        let eighth = bowed
            .iter()
            .find(|note| (note.start_beat - 0.5).abs() < 1e-9)
            .expect("eighth note");
        assert_eq!(eighth.velocity, 92 + crate::bowing::BOW_CHANGE_ACCENT);
        assert!(eighth.duration_beats < 0.25);
    }

    #[test]
    fn melody_variation_keeps_the_tune_and_the_loop_grid() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let plain = arrangement.melody_notes_for_pass(0);
        let mut seen = Vec::new();
        for pass in 1..VARIATION_PASSES {
            let varied = arrangement.melody_notes_for_pass(pass);
            assert!(
                varied.len() >= plain.len(),
                "pass {pass} should add ornaments, not remove the tune"
            );
            for base in &plain {
                assert!(
                    varied.iter().any(|note| {
                        (note.start_beat - base.start_beat).abs() < 1e-9 && note.midi == base.midi
                    }),
                    "pass {pass} moved or removed a melody note at beat {}",
                    base.start_beat
                );
            }
            assert!(
                varied
                    .iter()
                    .all(|note| note.start_beat >= 0.0 && note.end_beat() <= 64.0 + 1e-6),
                "pass {pass} must stay inside the loop"
            );
            assert_ne!(varied, plain, "pass {pass} must vary the melody");
            assert_eq!(
                varied,
                arrangement.melody_notes_for_pass(pass),
                "melody variation must be deterministic"
            );
            assert!(
                !seen.contains(&varied),
                "variation passes should differ from one another"
            );
            seen.push(varied);
        }
    }

    #[test]
    fn accompaniment_is_generated() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        for instrument in [
            AccompanimentInstrument::Piano,
            AccompanimentInstrument::Guitar,
            AccompanimentInstrument::Accordion,
        ] {
            let notes = arrangement.accompaniment_notes(instrument);
            assert!(!notes.is_empty());
            assert!(notes.iter().all(|note| note.midi <= 108));
        }
    }

    #[test]
    fn accompaniment_keeps_a_minor_third_from_the_melody() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        for instrument in [
            AccompanimentInstrument::Piano,
            AccompanimentInstrument::Guitar,
            AccompanimentInstrument::Accordion,
        ] {
            for rhythm in AccompanimentRhythm::ALL {
                let notes = arrangement.accompaniment_notes_with_rhythm(instrument, rhythm);
                for note in &notes {
                    for melody in &arrangement.melody {
                        let overlaps = melody.start_beat < note.end_beat() - 1e-9
                            && melody.end_beat() > note.start_beat + 1e-9;
                        if overlaps {
                            assert!(
                                (melody.midi as i32 - note.midi as i32).abs()
                                    >= MIN_MELODY_SEPARATION_SEMITONES,
                                "{} {} note {} crowds melody {} at beat {}",
                                instrument.label(),
                                rhythm.label(),
                                note.midi,
                                melody.midi,
                                note.start_beat
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_low_melody_pushes_the_accompaniment_down_an_octave() {
        let low = Arrangement::from_score(&simple_score(), 64.0);
        let mut high_score = simple_score();
        for note in &mut high_score.notes {
            note.midi += 24;
        }
        let high = Arrangement::from_score(&high_score, 64.0);
        let low_notes = low.accompaniment_notes(AccompanimentInstrument::Piano);
        let high_notes = high.accompaniment_notes(AccompanimentInstrument::Piano);

        // The high tune sits clear of the piano's close-position voicing, so
        // nothing moves and its top note stays the voicing's top.
        assert!(
            high_notes.iter().all(|note| note.midi <= 74),
            "a high melody should leave the voicing alone: {:?}",
            high_notes.iter().map(|note| note.midi).collect::<Vec<_>>()
        );
        // With the tune in the same octave as the chord tones, the nearest
        // tones are dropped an octave and the pattern reaches lower.
        assert!(
            low_notes.iter().any(|note| note.midi < 55),
            "a low melody should move the voicing down: {:?}",
            low_notes.iter().map(|note| note.midi).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_ending_keeps_its_accompaniment_clear_of_the_melody() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let ending = arrangement.ending(
            240.0,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            &legacy_ending(),
        );
        for note in &ending.accompaniment {
            let note_end = note.start_seconds + note.duration_seconds;
            for melody in &ending.melody {
                let melody_end = melody.start_seconds + melody.duration_seconds;
                let overlaps = melody.start_seconds < note_end - 1e-9
                    && melody_end > note.start_seconds + 1e-9;
                if overlaps {
                    assert!(
                        (melody.midi as i32 - note.midi as i32).abs()
                            >= MIN_MELODY_SEPARATION_SEMITONES,
                        "ending accompaniment {} crowds melody {} at {:.3}s",
                        note.midi,
                        melody.midi,
                        note.start_seconds
                    );
                }
            }
        }
    }

    #[test]
    fn auto_rhythm_matches_each_instruments_signature_pattern() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        for (instrument, rhythm) in [
            (
                AccompanimentInstrument::Piano,
                AccompanimentRhythm::BoomChuck,
            ),
            (AccompanimentInstrument::Guitar, AccompanimentRhythm::Strum),
            (
                AccompanimentInstrument::Accordion,
                AccompanimentRhythm::Sustained,
            ),
        ] {
            assert_eq!(
                arrangement.accompaniment_notes(instrument),
                arrangement.accompaniment_notes_with_rhythm(instrument, rhythm),
                "{} should default to {}",
                instrument.label(),
                rhythm.label()
            );
        }
    }

    #[test]
    fn every_rhythm_stays_inside_the_loop_for_every_instrument() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        for instrument in [
            AccompanimentInstrument::Piano,
            AccompanimentInstrument::Guitar,
            AccompanimentInstrument::Accordion,
        ] {
            for rhythm in AccompanimentRhythm::ALL {
                let notes = arrangement.accompaniment_notes_with_rhythm(instrument, rhythm);
                assert!(
                    !notes.is_empty(),
                    "{} with {} must play",
                    instrument.label(),
                    rhythm.label()
                );
                assert!(
                    notes
                        .iter()
                        .all(|note| note.start_beat >= 0.0 && note.end_beat() <= 64.0 + 1e-6),
                    "{} with {} must stay inside the loop",
                    instrument.label(),
                    rhythm.label()
                );
            }
        }
    }

    #[test]
    fn rhythms_have_distinct_pulses() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let piano = AccompanimentInstrument::Piano;

        let block = arrangement.accompaniment_notes_with_rhythm(piano, AccompanimentRhythm::Block);
        assert!(
            block
                .iter()
                .any(|note| (note.start_beat - 1.0).abs() < 1e-9),
            "block chords land on each dance beat"
        );

        let shuffle =
            arrangement.accompaniment_notes_with_rhythm(piano, AccompanimentRhythm::Shuffle);
        assert!(
            shuffle
                .iter()
                .any(|note| (note.start_beat - 0.67).abs() < 1e-9),
            "shuffle places the offbeat chord two thirds of the way through the beat"
        );

        let arpeggio =
            arrangement.accompaniment_notes_with_rhythm(piano, AccompanimentRhythm::Arpeggio);
        let sustained =
            arrangement.accompaniment_notes_with_rhythm(piano, AccompanimentRhythm::Sustained);
        assert!(
            arpeggio.len() > sustained.len(),
            "the arpeggio should move through the chord rather than hold it"
        );

        assert_ne!(
            arpeggio, block,
            "choosing a rhythm must change the generated accompaniment"
        );
    }

    #[test]
    fn strum_alternates_down_and_up() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let notes = arrangement.accompaniment_notes_with_rhythm(
            AccompanimentInstrument::Guitar,
            AccompanimentRhythm::Strum,
        );

        // Within a strum, the attack order tells the direction: a down strum
        // goes from the bass up through the chord, an up strum comes back down
        // from the treble.
        for strum in 0..4 {
            let low = strum as f64 * 0.5;
            let high = low + 0.5;
            let mut attack: Vec<&NoteEvent> = notes
                .iter()
                .filter(|note| note.start_beat >= low && note.start_beat < high)
                .collect();
            attack.sort_by(|a, b| a.start_beat.total_cmp(&b.start_beat));
            let pitches: Vec<u8> = attack.iter().map(|note| note.midi).collect();
            if strum % 2 == 0 {
                assert!(
                    pitches.windows(2).all(|pair| pair[0] < pair[1]),
                    "strum {strum} should sweep up from the bass: {pitches:?}"
                );
            } else {
                assert!(
                    pitches.windows(2).all(|pair| pair[0] > pair[1]),
                    "strum {strum} should sweep back down from the treble: {pitches:?}"
                );
            }
        }

        // The upstroke is lighter and leaves out the lowest string.
        let down: Vec<&NoteEvent> = notes.iter().filter(|note| note.start_beat < 0.5).collect();
        let up: Vec<&NoteEvent> = notes
            .iter()
            .filter(|note| (0.5..1.0).contains(&note.start_beat))
            .collect();
        assert!(down.iter().all(|note| note.velocity > 44));
        assert!(up.iter().all(|note| note.velocity == 44));
        assert_eq!(up.len(), down.len() - 1);
        assert!(up.iter().map(|note| note.midi).min() > down.iter().map(|note| note.midi).min());
    }

    #[test]
    fn variations_keep_the_selected_rhythm_on_the_grid() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        for rhythm in AccompanimentRhythm::ALL {
            let plain =
                arrangement.accompaniment_notes_with_rhythm(AccompanimentInstrument::Piano, rhythm);
            for pass in 1..VARIATION_PASSES {
                let notes = arrangement.accompaniment_notes_for_pass_with_rhythm(
                    AccompanimentInstrument::Piano,
                    rhythm,
                    pass,
                );
                assert!(
                    notes
                        .iter()
                        .all(|note| note.start_beat >= 0.0 && note.end_beat() <= 64.0 + 1e-6),
                    "{} pass {pass} must stay inside the loop",
                    rhythm.label()
                );
                assert!(
                    notes.iter().any(|note| note.start_beat < 2.0),
                    "{} pass {pass} must keep the opening bar",
                    rhythm.label()
                );
            }
            assert!(
                arrangement
                    .accompaniment_notes_for_pass_with_rhythm(
                        AccompanimentInstrument::Piano,
                        rhythm,
                        1,
                    )
                    .len()
                    >= plain.len(),
                "{} variations should add emphasis, not remove the pattern",
                rhythm.label()
            );
        }
    }

    #[test]
    fn variation_passes_add_notes_without_leaving_the_loop() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        for instrument in [
            AccompanimentInstrument::Piano,
            AccompanimentInstrument::Guitar,
            AccompanimentInstrument::Accordion,
        ] {
            let plain = arrangement.accompaniment_notes(instrument);
            let mut changed = false;
            for pass in 1..VARIATION_PASSES {
                let notes = arrangement.accompaniment_notes_for_pass(instrument, pass);
                assert!(!notes.is_empty(), "pass {pass} must keep playing");
                assert!(
                    notes
                        .iter()
                        .all(|note| note.start_beat >= 0.0 && note.end_beat() <= 64.0 + 1e-6),
                    "pass {pass} must stay inside the 64-beat loop"
                );
                assert!(
                    notes.iter().any(|note| note.start_beat < 2.0),
                    "pass {pass} must keep the opening bar so dancers get a clear start"
                );
                changed |= notes != plain;
            }
            assert!(changed, "variation passes must differ from the plain loop");
        }
    }

    #[test]
    fn variation_passes_are_deterministic() {
        let first = Arrangement::from_score(&simple_score(), 64.0);
        let second = Arrangement::from_score(&simple_score(), 64.0);
        for pass in 0..VARIATION_PASSES {
            assert_eq!(
                first.accompaniment_notes_for_pass(AccompanimentInstrument::Piano, pass),
                second.accompaniment_notes_for_pass(AccompanimentInstrument::Piano, pass),
            );
        }
    }

    #[test]
    fn first_variation_emphasises_some_bars() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        // Pass 1 never rests, so its only change is extra emphasis notes.
        let plain = arrangement.accompaniment_notes(AccompanimentInstrument::Piano);
        let varied = arrangement.accompaniment_notes_for_pass(AccompanimentInstrument::Piano, 1);
        assert!(
            varied.len() > plain.len(),
            "{} should have more notes than {}",
            varied.len(),
            plain.len()
        );
    }

    #[test]
    fn variations_keep_the_bundled_tune_on_the_grid() {
        let abc = include_str!("../../../assets/defaults/sandy_boys.abc");
        let score = crate::abc::parse_abc(abc).expect("parse bundled tune");
        let arrangement = Arrangement::from_score(&score, 64.0);
        for pass in 0..VARIATION_PASSES {
            for instrument in [
                AccompanimentInstrument::Piano,
                AccompanimentInstrument::Guitar,
                AccompanimentInstrument::Accordion,
            ] {
                let notes = arrangement.accompaniment_notes_for_pass(instrument, pass);
                assert!(!notes.is_empty());
                assert!(notes.iter().all(|note| {
                    note.start_beat >= 0.0 && note.end_beat() <= arrangement.loop_beats + 1e-6
                }));
            }
        }
    }

    #[test]
    fn bundled_melody_variations_stay_on_the_grid() {
        let abc = include_str!("../../../assets/defaults/sandy_boys.abc");
        let score = crate::abc::parse_abc(abc).expect("parse bundled tune");
        let arrangement = Arrangement::from_score(&score, 64.0);
        for pass in 0..VARIATION_PASSES {
            let notes = arrangement.melody_notes_for_pass(pass);
            assert!(!notes.is_empty(), "pass {pass} must keep the melody");
            assert!(notes.iter().all(|note| {
                note.start_beat >= 0.0
                    && note.end_beat() <= arrangement.loop_beats + 1e-6
                    && note.velocity <= 124
            }));
        }
    }

    #[test]
    fn count_in_uses_first_chord() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        assert_eq!(arrangement.count_in_chord().symbol(), "D");
    }

    #[test]
    fn separates_a_pickup_from_the_loop() {
        let abc = "X:1\nT:Pickup\nM:4/4\nL:1/8\nK:D\neg |: A2 A2 A2 A2 | B2 B2 B2 B2 :|\n";
        let score = crate::abc::parse_abc(abc).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        assert!(!arrangement.pickup.is_empty());
        assert!(arrangement.pickup.iter().all(|note| note.start_beat < 0.0));
        assert!(arrangement.melody.iter().all(|note| note.start_beat >= 0.0));
        assert_eq!(arrangement.loop_beats, 64.0);
    }

    #[test]
    fn bundled_sandy_boys_arranges_to_a_full_loop() {
        let abc = include_str!("../../../assets/defaults/sandy_boys.abc");
        let score = crate::abc::parse_abc(abc).expect("parse bundled tune");
        let arrangement = Arrangement::from_score(&score, 64.0);
        assert_eq!(score.duration_beats, 64.5);
        // `bar_lines` records the written pass (the B repeat is expanded into
        // notes, not bar lines): the final written bar now ends at 48.5, so the
        // B part is a full 16 beats and its repeat stays on the bar grid.
        assert_eq!(score.bar_lines.last().copied(), Some(48.5));
        assert_eq!(arrangement.loop_beats, 64.0);
        assert_eq!(
            arrangement.body_beats, 64.0,
            "the AABB body must fill the 64-beat dance, with the pickup outside it"
        );
        assert!(arrangement.check_dance_length(64.0).is_ok());
        assert!(!arrangement.pickup.is_empty(), "Sandy Boys has a pickup");
        assert!(!arrangement.melody.is_empty());
        assert!(!arrangement.chords.is_empty());
        assert!(
            arrangement
                .melody
                .iter()
                .all(|note| note.start_beat >= 0.0 && note.start_beat < 64.0)
        );
        assert!(
            arrangement
                .melody
                .iter()
                .all(|note| note.end_beat() <= 64.0 + 1e-6)
        );
        let last_start = arrangement
            .melody
            .iter()
            .map(|note| note.start_beat)
            .fold(0.0_f64, f64::max);
        assert!(
            (last_start - 63.0).abs() < 1e-6,
            "the final 2-beat bar should begin at beat 63, not {last_start}"
        );
        assert_eq!(arrangement.count_in_chord().symbol(), "A");
        assert!(
            !arrangement.countermelody_notes().is_empty(),
            "the bundled tune must produce the trio's countermelody"
        );
    }

    #[test]
    fn detects_a_dance_and_tune_length_mismatch() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        assert!(arrangement.check_dance_length(64.0).is_ok());

        let mismatch = arrangement
            .check_dance_length(63.5)
            .expect_err("63.5 beats should not match a 64-beat tune");
        assert_eq!(mismatch.dance_beats, 63.5);
        assert_eq!(mismatch.tune_beats, 64.0);
        assert!(mismatch.to_string().contains("63.5"));
        assert!(mismatch.to_string().contains("64.0"));
    }

    #[test]
    fn the_ending_replaces_the_last_bar_and_holds_the_final_chord() {
        let mut score = simple_score();
        score.notes.push(NoteEvent {
            start_beat: 62.0,
            duration_beats: 1.0,
            midi: 64,
            velocity: 80,
        });
        score.notes.push(NoteEvent {
            start_beat: 63.0,
            duration_beats: 1.0,
            midi: 66,
            velocity: 80,
        });
        let arrangement = Arrangement::from_score(&score, 64.0);
        let ending = arrangement.ending(
            240.0,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            &legacy_ending(),
        );

        // A two-beat bar at 240 BPM lasts 1 s; stretched three times, then an
        // eight-beat (4 s) hold.
        assert!((ending.from_beat - 62.0).abs() < 1e-9);
        assert!((ending.linger_start_seconds - 3.0).abs() < 1e-9);
        assert!((ending.seconds - 7.0).abs() < 1e-9);
        assert!(!ending.melody.is_empty());
        assert!(!ending.accompaniment.is_empty());

        let mut seconds: Vec<f64> = ending
            .melody
            .iter()
            .chain(ending.accompaniment.iter())
            .map(|note| note.start_seconds + note.duration_seconds)
            .collect();
        seconds.sort_by(f64::total_cmp);
        assert!(
            seconds
                .last()
                .is_some_and(|end| *end <= ending.seconds + 1e-9)
        );
        assert!(seconds.last().is_some_and(|end| *end > 6.5));
        assert!(
            ending
                .melody
                .iter()
                .chain(ending.accompaniment.iter())
                .all(|note| note.midi <= 108 && note.start_seconds >= 0.0)
        );

        // The last melody note rings through the final chord instead of being
        // cut when the bar ends.
        assert!(
            ending
                .melody
                .iter()
                .any(|note| note.duration_seconds >= 5.0),
            "the closing melody note should linger: {:?}",
            ending.melody
        );
        assert!(
            ending
                .accompaniment
                .iter()
                .any(|note| note.start_seconds >= 2.99 && note.start_seconds <= 3.01),
            "the final chord should arrive at the end of the ritardando"
        );
    }

    #[test]
    fn the_ending_slows_down_more_as_the_last_bar_closes() {
        let mut score = simple_score();
        for (beat, midi) in [(62.0, 64), (62.5, 65), (63.0, 66), (63.5, 67)] {
            score.notes.push(NoteEvent {
                start_beat: beat,
                duration_beats: 0.5,
                midi,
                velocity: 80,
            });
        }
        let arrangement = Arrangement::from_score(&score, 64.0);
        let ending = arrangement.ending(
            240.0,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            &legacy_ending(),
        );

        let mut starts: Vec<f64> = ending
            .melody
            .iter()
            .filter(|note| note.start_seconds < ending.linger_start_seconds)
            .map(|note| note.start_seconds)
            .collect();
        starts.sort_by(f64::total_cmp);
        assert!(starts.len() >= 4, "starts: {starts:?}");
        let first_gap = starts[1] - starts[0];
        let last_gap = starts[starts.len() - 1] - starts[starts.len() - 2];
        assert!(
            last_gap > first_gap * 1.5,
            "gaps should grow as the tempo falls: {starts:?}"
        );
    }

    #[test]
    fn the_ending_follows_the_session_tempo() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let slow = arrangement.ending(
            120.0,
            AccompanimentInstrument::Guitar,
            AccompanimentRhythm::Auto,
            &legacy_ending(),
        );
        let fast = arrangement.ending(
            240.0,
            AccompanimentInstrument::Guitar,
            AccompanimentRhythm::Auto,
            &legacy_ending(),
        );
        assert!((slow.seconds - fast.seconds * 2.0).abs() < 1e-9);
        assert!(slow.melody[0].duration_seconds > fast.melody[0].duration_seconds);
    }

    #[test]
    fn every_instrument_and_rhythm_can_play_the_ending() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        for instrument in [
            AccompanimentInstrument::Piano,
            AccompanimentInstrument::Guitar,
            AccompanimentInstrument::Accordion,
        ] {
            for rhythm in AccompanimentRhythm::ALL {
                let ending =
                    arrangement.ending(240.0, instrument, rhythm, &EndingOptions::default());
                assert!(!ending.melody.is_empty());
                assert!(
                    !ending.accompaniment.is_empty(),
                    "{} with {} produced no ending backing",
                    instrument.label(),
                    rhythm.label()
                );
                assert!(
                    ending
                        .melody
                        .iter()
                        .chain(ending.accompaniment.iter())
                        .all(|note| note.start_seconds >= 0.0
                            && note.start_seconds + note.duration_seconds <= ending.seconds + 1e-9)
                );
            }
        }
    }

    #[test]
    fn the_ending_uses_the_same_two_beat_bar_for_a_jig() {
        let mut score = simple_score();
        score.meter = Meter {
            numerator: 6,
            denominator: 8,
        };
        score.notes.push(NoteEvent {
            start_beat: 62.0,
            duration_beats: 1.0,
            midi: 64,
            velocity: 80,
        });
        score.notes.push(NoteEvent {
            start_beat: 63.0,
            duration_beats: 1.0,
            midi: 66,
            velocity: 80,
        });
        let arrangement = Arrangement::from_score(&score, 64.0);
        let ending = arrangement.ending(
            240.0,
            AccompanimentInstrument::Accordion,
            AccompanimentRhythm::Auto,
            &legacy_ending(),
        );
        assert!((ending.from_beat - 62.0).abs() < 1e-9);
        assert!((ending.seconds - 7.0).abs() < 1e-9);
        assert!(!ending.melody.is_empty());
        assert!(!ending.accompaniment.is_empty());
    }

    #[test]
    fn the_gradual_ending_starts_four_beats_before_the_loop_end() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let ending = arrangement.ending(
            240.0,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            &EndingOptions::default(),
        );
        assert!((ending.from_beat - 60.0).abs() < 1e-9);
        // Four beats at 240 BPM are 2 s; doubled by the easing, then an
        // eight-beat (4 s) hold.
        assert!((ending.linger_start_seconds - 4.0).abs() < 1e-9);
        assert!((ending.seconds - 8.0).abs() < 1e-9);
        assert!(ending.seconds.is_finite() && ending.seconds > 0.0);
    }

    #[test]
    fn the_gradual_ending_thins_before_the_final_chord() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let ending = arrangement.ending(
            240.0,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            &EndingOptions::default(),
        );
        // Notes are in seconds; the region is four beats (2 s at 240 BPM).
        let region_seconds = 2.0;
        let before_final: Vec<&EndingNote> = ending
            .accompaniment
            .iter()
            .filter(|note| note.start_seconds < ending.linger_start_seconds - 1e-9)
            .collect();
        assert!(
            !before_final.is_empty(),
            "the ending must still have backing"
        );
        assert!(
            before_final
                .iter()
                .all(|note| { note.start_seconds / region_seconds < 0.75 + 1e-9 }),
            "the backing should drop out before the final chord: {:?}",
            before_final
                .iter()
                .map(|note| note.start_seconds)
                .collect::<Vec<_>>()
        );
        // In the last half of the region only the lowest note of each attack
        // remains.
        let late: Vec<&EndingNote> = before_final
            .iter()
            .copied()
            .filter(|note| note.start_seconds / region_seconds >= 0.5)
            .collect();
        for note in late {
            let lowest = before_final
                .iter()
                .filter(|other| (other.start_seconds - note.start_seconds).abs() < 1e-9)
                .map(|other| other.midi)
                .min();
            assert_eq!(lowest, Some(note.midi));
        }
        for note in ending.melody.iter().chain(ending.accompaniment.iter()) {
            assert!(note.start_seconds.is_finite() && note.duration_seconds.is_finite());
            assert!(note.start_seconds >= 0.0);
            assert!(note.start_seconds + note.duration_seconds <= ending.seconds + 1e-9);
            assert!(note.midi <= 108);
        }
    }

    #[test]
    fn the_final_chord_resolves_a_fifth_below_by_default() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let written_root = arrangement.chord_at(arrangement.loop_beats - 1e-6).root;
        let ending = arrangement.ending(
            240.0,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            &EndingOptions::default(),
        );
        let bass = ending
            .accompaniment
            .iter()
            .filter(|note| (note.start_seconds - ending.linger_start_seconds).abs() < 1e-9)
            .map(|note| note.midi)
            .min()
            .expect("the final chord has a bass note");
        assert_eq!(
            i32::from(bass).rem_euclid(12),
            (written_root - 7).rem_euclid(12),
            "the final bass should be a perfect fifth below the written root"
        );
    }

    #[test]
    fn the_closing_melody_resolves_a_fifth_below_with_the_final_chord() {
        let mut score = simple_score();
        score.notes.push(NoteEvent {
            start_beat: 63.0,
            duration_beats: 1.0,
            midi: 66,
            velocity: 80,
        });
        let arrangement = Arrangement::from_score(&score, 64.0);
        let written_last = arrangement
            .melody
            .last()
            .expect("the score has a closing melody note")
            .midi;
        let ending = arrangement.ending(
            240.0,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            &EndingOptions::default(),
        );

        let closing: Vec<&EndingNote> = ending
            .melody
            .iter()
            .filter(|note| (note.start_seconds - ending.linger_start_seconds).abs() < 1e-9)
            .collect();
        assert_eq!(closing.len(), 1, "the final chord has one melody note");
        assert_eq!(
            closing[0].midi,
            written_last.saturating_sub(7),
            "the closing melody should resolve a perfect fifth below the written note"
        );
        assert!(
            ending.melody.iter().any(|note| {
                note.start_seconds < ending.linger_start_seconds - 1e-9 && note.midi == written_last
            }),
            "the written melody note should hold until the final chord arrives"
        );
    }

    #[test]
    fn turning_the_ending_options_off_restores_the_previous_shape() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let written_root = arrangement.chord_at(arrangement.loop_beats - 1e-6).root;
        let ending = arrangement.ending(
            240.0,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            &legacy_ending(),
        );
        assert!((ending.from_beat - 62.0).abs() < 1e-9);
        assert!((ending.linger_start_seconds - 3.0).abs() < 1e-9);
        assert!((ending.seconds - 7.0).abs() < 1e-9);
        let bass = ending
            .accompaniment
            .iter()
            .filter(|note| (note.start_seconds - ending.linger_start_seconds).abs() < 1e-9)
            .map(|note| note.midi)
            .min()
            .expect("the final chord has a bass note");
        assert_eq!(i32::from(bass).rem_euclid(12), written_root.rem_euclid(12));
    }

    /// A varied pass may not turn a written rest into an extra silent bar.
    #[test]
    fn varied_passes_never_leave_a_silent_bar() {
        const SILENT_BELOW_BEATS: f64 = 0.5;
        let bundled = include_str!("../../../assets/defaults/sandy_boys.abc");
        let bundled = crate::abc::parse_abc(bundled).expect("parse bundled tune");
        let short = short_resting_score();
        for score in [bundled, short] {
            let arrangement = Arrangement::from_score(&score, 64.0);
            let options = VariationOptions::default();
            let plain = pass_notes(&arrangement, 0, &options);
            let bars = arrangement.bars().floor() as i64;
            let plain_silent: Vec<bool> = (0..bars)
                .map(|bar| {
                    let start = bar as f64 * 2.0;
                    sounding_coverage(&plain, start, start + 2.0) < SILENT_BELOW_BEATS
                })
                .collect();
            for pass in 1..VARIATION_PASSES {
                let notes = pass_notes(&arrangement, pass, &options);
                for bar in 0..bars {
                    if plain_silent[bar as usize] {
                        continue;
                    }
                    let start = bar as f64 * 2.0;
                    let coverage = sounding_coverage(&notes, start, start + 2.0);
                    assert!(
                        coverage >= SILENT_BELOW_BEATS,
                        "pass {pass} leaves bar {bar} silent ({coverage:.2} beats sound)"
                    );
                }
            }
        }
    }

    fn short_resting_score() -> Score {
        let mut score = simple_score();
        score.duration_beats = 16.0;
        score.notes.retain(|note| note.start_beat < 16.0);
        // A written four-beat rest in the middle of the short tune. Repeating
        // it fills the 64-beat loop with rests that a variation must not
        // extend.
        score
            .notes
            .retain(|note| !(8.0..12.0).contains(&note.start_beat));
        score.chords = vec![ChordEvent {
            start_beat: 0.0,
            duration_beats: 16.0,
            symbol: "D".into(),
        }];
        score
    }

    fn pass_notes(
        arrangement: &Arrangement,
        pass: u32,
        options: &VariationOptions,
    ) -> Vec<NoteEvent> {
        let mut notes = arrangement.melody_notes_for_pass_with_options(pass, options);
        notes.extend(arrangement.accompaniment_notes_for_pass_with_options(
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            pass,
            options,
        ));
        notes.extend(arrangement.countermelody_notes_for_pass_with_options(pass, options));
        notes
    }

    fn sounding_coverage(notes: &[NoteEvent], start: f64, end: f64) -> f64 {
        let mut spans: Vec<(f64, f64)> = notes
            .iter()
            .map(|note| (note.start_beat.max(start), note.end_beat().min(end)))
            .filter(|(note_start, note_end)| *note_end > *note_start + 1e-9)
            .collect();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut covered = 0.0;
        let mut cursor = start;
        for (span_start, span_end) in spans {
            if span_end <= cursor + 1e-9 {
                continue;
            }
            let span_start = span_start.max(cursor);
            covered += span_end - span_start;
            cursor = span_end;
        }
        covered.min(end - start).max(0.0)
    }

    #[test]
    fn every_variation_option_gates_its_own_variation() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let all = VariationOptions::default();
        let variants = [
            (
                "articulation",
                VariationOptions {
                    articulation: false,
                    ..all.clone()
                },
            ),
            (
                "double_stops",
                VariationOptions {
                    double_stops: false,
                    ..all.clone()
                },
            ),
            (
                "grace_notes",
                VariationOptions {
                    grace_notes: false,
                    ..all.clone()
                },
            ),
            (
                "octave_lift",
                VariationOptions {
                    octave_lift: false,
                    ..all.clone()
                },
            ),
            (
                "chord_stabs",
                VariationOptions {
                    chord_stabs: false,
                    ..all.clone()
                },
            ),
            (
                "melody_alone_bar",
                VariationOptions {
                    melody_alone_bar: false,
                    ..all.clone()
                },
            ),
        ];
        let plain_melody = arrangement.melody_notes_for_pass_with_options(0, &all);
        for (name, off) in variants {
            let mut changed = false;
            for pass in 1..12 {
                let with = pass_notes(&arrangement, pass, &all);
                let without = pass_notes(&arrangement, pass, &off);
                assert!(
                    with.iter()
                        .all(|note| note.start_beat >= 0.0 && note.end_beat() <= 64.0 + 1e-6),
                    "{name} pass {pass} left the loop"
                );
                assert!(
                    plain_melody.iter().all(|base| without.iter().any(|note| {
                        (note.start_beat - base.start_beat).abs() < 1e-9 && note.midi == base.midi
                    })),
                    "disabling {name} must keep every plain melody note"
                );
                changed |= with != without;
            }
            assert!(
                changed,
                "disabling {name} must change at least one variation pass"
            );
        }
    }

    #[test]
    fn disabling_the_melody_alone_bar_keeps_the_countermelody_playing() {
        let arrangement = Arrangement::from_score(&simple_score(), 64.0);
        let all = VariationOptions::default();
        let off = VariationOptions {
            melody_alone_bar: false,
            ..all.clone()
        };
        assert!(
            (0..16).any(|pass| arrangement.variation_rest_bar(pass, &all).is_some()),
            "the default options should reserve a melody-alone bar"
        );
        for pass in 0..16 {
            assert!(arrangement.variation_rest_bar(pass, &off).is_none());
            assert!(
                !arrangement
                    .countermelody_notes_for_pass_with_options(pass, &off)
                    .is_empty(),
                "pass {pass} should keep the countermelody"
            );
        }
    }
}
