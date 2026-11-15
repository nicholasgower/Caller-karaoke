//! Turn a parsed score into a dance-length music loop and generate the
//! accompaniment.

use contra_core::AccompanimentInstrument;

use crate::chord::{Chord, resolve_chords};
use crate::score::{ChordEvent, Key, Meter, NoteEvent, Score};

/// The music for one practice loop.
#[derive(Clone, Debug)]
pub struct Arrangement {
    /// Melody notes with times relative to the start of the loop.
    pub melody: Vec<NoteEvent>,
    /// Notes that belong before beat 0 (a pickup, played in the count-in).
    pub pickup: Vec<NoteEvent>,
    pub chords: Vec<ChordEvent>,
    pub loop_beats: f64,
    pub meter: Meter,
    pub key: Key,
    pub warnings: Vec<String>,
}

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
            meter: score.meter,
            key: score.key.clone(),
            warnings,
        }
    }

    pub fn bars(&self) -> f64 {
        self.loop_beats / 2.0
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

    /// Generate the backing track for the selected instrument.
    pub fn accompaniment_notes(&self, instrument: AccompanimentInstrument) -> Vec<NoteEvent> {
        let mut notes = Vec::new();
        for chord_event in &self.chords {
            let chord = Chord::parse(&chord_event.symbol).unwrap_or_else(|| self.count_in_chord());
            let start = chord_event.start_beat;
            let end = chord_event.end_beat().min(self.loop_beats);
            if end <= start {
                continue;
            }
            match instrument {
                AccompanimentInstrument::Piano => {
                    let mut step = 0;
                    let mut beat = start;
                    while beat < end {
                        if step % 2 == 0 {
                            notes.push(NoteEvent {
                                start_beat: beat,
                                duration_beats: 0.45,
                                midi: chord.bass_midi(),
                                velocity: 58,
                            });
                        } else {
                            for midi in chord.voicing(55) {
                                notes.push(NoteEvent {
                                    start_beat: beat,
                                    duration_beats: 0.45,
                                    midi,
                                    velocity: 52,
                                });
                            }
                        }
                        beat += 0.5;
                        step += 1;
                    }
                }
                AccompanimentInstrument::Guitar => {
                    let mut step = 0;
                    let mut beat = start;
                    while beat < end {
                        for (index, midi) in chord.voicing(55).into_iter().enumerate() {
                            notes.push(NoteEvent {
                                start_beat: beat + index as f64 * 0.025,
                                duration_beats: 0.4,
                                midi,
                                velocity: 54,
                            });
                        }
                        beat += 0.5;
                        step += 1;
                        if step > 4096 {
                            break;
                        }
                    }
                }
                AccompanimentInstrument::Accordion => {
                    let duration = (end - start).min(2.0) * 0.95;
                    notes.push(NoteEvent {
                        start_beat: start,
                        duration_beats: duration,
                        midi: chord.bass_midi(),
                        velocity: 55,
                    });
                    for midi in chord.voicing(60) {
                        notes.push(NoteEvent {
                            start_beat: start,
                            duration_beats: duration,
                            midi,
                            velocity: 50,
                        });
                    }
                }
            }
        }
        notes
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
        assert_eq!(arrangement.loop_beats, 64.0);
        assert!(!arrangement.pickup.is_empty(), "Sandy Boys has a pickup");
        assert!(!arrangement.melody.is_empty());
        assert!(!arrangement.chords.is_empty());
        assert!(
            arrangement
                .melody
                .iter()
                .all(|note| note.start_beat >= 0.0 && note.start_beat < 64.0)
        );
        assert_eq!(arrangement.count_in_chord().symbol(), "A");
    }
}
