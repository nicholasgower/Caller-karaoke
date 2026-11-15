//! Turn a parsed score into a dance-length music loop and generate the
//! accompaniment.

use contra_core::{AccompanimentInstrument, AccompanimentRhythm};

use crate::chord::{Chord, resolve_chords};
use crate::score::{ChordEvent, Key, Meter, NoteEvent, Score};

/// Number of distinct accompaniment passes rendered back to back for one
/// practice tune. Pass 0 is the plain pattern; the others carry small
/// variations so a looping session does not repeat note for note.
pub const VARIATION_PASSES: u32 = 4;

/// How much longer the tune's last bar takes in the slow ending than at the
/// session tempo. Three times is a deliberate, audible ritardando, not a
/// subtle easing.
pub const ENDING_LAST_BAR_STRETCH: f64 = 3.0;

/// Shape of the ritardando inside the ending's last bar. Above 1 the bar starts
/// at the session tempo and the following beats stretch more and more, so the
/// ending eases into the slowdown instead of lurching.
pub const ENDING_SLOWDOWN_CURVE: f64 = 2.0;

/// Dance beats the final harmony is held before the audio fades out.
pub const ENDING_LINGER_BEATS: f64 = 8.0;

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
    /// The closing bar's backing plus the held final chord.
    pub accompaniment: Vec<EndingNote>,
    /// Beat in a normal 64-beat pass where the ending replaces the remaining
    /// music. It is the start of the tune's last bar.
    pub from_beat: f64,
    /// When the held final chord begins, in seconds from the ending's start.
    pub linger_start_seconds: f64,
    /// Total length of the ending in seconds.
    pub seconds: f64,
}

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
    /// The last bar is replayed at a falling tempo and the final harmony is
    /// held. The ending replaces that bar when the transport reaches
    /// [`Ending::from_beat`], so the tune's own closing bar is not heard twice:
    /// the caller hears the normal tune up to the last bar, then the same bar
    /// stretched into a cadence.
    pub fn ending(
        &self,
        bpm: f64,
        instrument: AccompanimentInstrument,
        rhythm: AccompanimentRhythm,
    ) -> Ending {
        let beat_seconds = 120.0 / bpm.max(1.0);
        let bar_beats = self.meter.bar_dance_beats().clamp(1.0, self.loop_beats);
        let bar_start = (self.loop_beats - bar_beats).max(0.0);
        let normal_bar_seconds = bar_beats * beat_seconds;
        let ritard_seconds = normal_bar_seconds * ENDING_LAST_BAR_STRETCH;
        let linger_seconds = ENDING_LINGER_BEATS * beat_seconds;
        // Start at the session tempo, then stretch each beat more than the one
        // before: the linear term keeps the opening of the bar honest and the
        // power term supplies the ritardando.
        let slowdown = (ritard_seconds - normal_bar_seconds).max(0.0);
        let warp = |beat: f64| {
            let progress = ((beat - bar_start) / bar_beats).clamp(0.0, 1.0);
            normal_bar_seconds * progress + slowdown * progress.powf(ENDING_SLOWDOWN_CURVE)
        };

        let mut melody = Vec::new();
        for note in self
            .melody
            .iter()
            .filter(|note| note.start_beat >= bar_start - 1e-9)
        {
            let start = warp(note.start_beat);
            let end = warp(note.end_beat());
            melody.push(EndingNote {
                start_seconds: start,
                duration_seconds: (end - start).max(beat_seconds * 0.12),
                midi: note.midi,
                velocity: note.velocity,
            });
        }
        melody.sort_by(|a, b| a.start_seconds.total_cmp(&b.start_seconds));

        // A tune can rest through its written last bar. Hold the last pitch it
        // did play, under the final chord, rather than ending with no melody.
        if melody.is_empty()
            && let Some(note) = self.melody.last()
        {
            melody.push(EndingNote {
                start_seconds: ritard_seconds,
                duration_seconds: linger_seconds * 0.96,
                midi: note.midi,
                velocity: note.velocity,
            });
        }

        // Hold the closing melody note through the final chord, unless the
        // fallback above already begins on the final chord.
        if let Some(last) = melody.last_mut()
            && last.start_seconds < ritard_seconds
        {
            last.duration_seconds =
                (ritard_seconds - last.start_seconds).max(0.05) + linger_seconds;
        }

        let mut accompaniment =
            self.ending_bar_accompaniment(instrument, rhythm, bar_start, bar_beats);
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
        ));
        accompaniment.sort_by(|a, b| a.start_seconds.total_cmp(&b.start_seconds));

        Ending {
            melody,
            accompaniment,
            from_beat: bar_start,
            linger_start_seconds: ritard_seconds,
            seconds: ritard_seconds + linger_seconds,
        }
    }

    /// The last bar's backing notes, expressed in beats from the bar start.
    fn ending_bar_accompaniment(
        &self,
        instrument: AccompanimentInstrument,
        rhythm: AccompanimentRhythm,
        bar_start: f64,
        bar_beats: f64,
    ) -> Vec<EndingNote> {
        let rhythm = resolve_rhythm(instrument, rhythm);
        let chord = self.chord_at(bar_start);
        let mut notes = Vec::new();
        self.render_rhythm(
            &mut notes,
            rhythm,
            &chord,
            bar_start,
            (bar_start + bar_beats).min(self.loop_beats),
        );
        notes
            .into_iter()
            .map(|note| EndingNote {
                start_seconds: note.start_beat - bar_start,
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
    /// same pattern with deterministic variations: a few extra chord stabs for
    /// emphasis, and now and then one bar where the accompaniment drops out so
    /// the melody plays alone. The variations never move existing notes, so the
    /// pulse and the 64-beat grid are unchanged.
    pub fn accompaniment_notes_for_pass_with_rhythm(
        &self,
        instrument: AccompanimentInstrument,
        rhythm: AccompanimentRhythm,
        pass: u32,
    ) -> Vec<NoteEvent> {
        let mut notes = self.plain_accompaniment(instrument, rhythm);
        if pass > 0 {
            self.vary_accompaniment(&mut notes, pass);
        }
        notes.sort_by(|a, b| {
            a.start_beat
                .total_cmp(&b.start_beat)
                .then(a.midi.cmp(&b.midi))
        });
        notes
    }

    /// The second accompaniment voice: a countermelody under the tune.
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
        if self.melody.is_empty() {
            return Vec::new();
        }
        let rest_bar = self.variation_rest_bar(pass);
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

    /// Apply one pass of variations in place.
    ///
    /// The seed comes from the arrangement itself, so a given tune always
    /// produces the same variations and the audio, like the rest of `music`,
    /// stays deterministic and offline.
    fn vary_accompaniment(&self, notes: &mut Vec<NoteEvent>, pass: u32) {
        let mut rng = VariationRng::new(self.variation_seed(), pass);
        let bars = self.bars().floor() as i64;

        let rest_bar = self.variation_rest_bar(pass);
        if let Some(bar) = rest_bar {
            let rest_start = bar as f64 * 2.0;
            let rest_end = (rest_start + 2.0).min(self.loop_beats);
            notes.retain(|note| note.start_beat < rest_start - 1e-9 || note.start_beat >= rest_end);
        }

        // A few short chord stabs on the back half of the bar, above the
        // regular pattern, add emphasis without disturbing the pulse.
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
    /// dancers always get a clear start. The choice is deterministic and shared
    /// by the accompaniment and the countermelody.
    fn variation_rest_bar(&self, pass: u32) -> Option<i64> {
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
        Some(first + rng.below((last - first + 1) as u64) as i64)
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
fn final_chord_notes(chord: &Chord, start: f64, linger_seconds: f64) -> Vec<EndingNote> {
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
        assert!(!notes.is_empty(), "the second accompaniment must play");
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
            let Some(bar) = first.variation_rest_bar(pass) else {
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
            "the bundled tune must produce the trio's second accompaniment"
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
        );
        let fast = arrangement.ending(
            240.0,
            AccompanimentInstrument::Guitar,
            AccompanimentRhythm::Auto,
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
                let ending = arrangement.ending(240.0, instrument, rhythm);
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
        );
        assert!((ending.from_beat - 62.0).abs() < 1e-9);
        assert!((ending.seconds - 7.0).abs() < 1e-9);
        assert!(!ending.melody.is_empty());
        assert!(!ending.accompaniment.is_empty());
    }
}
