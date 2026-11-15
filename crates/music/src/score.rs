//! Musical score types shared by the ABC parser, arranger, synth, and MIDI
//! exporter. All times are in **dance beats** (half a bar).

use serde::{Deserialize, Serialize};

/// A time signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meter {
    pub numerator: u32,
    pub denominator: u32,
}

impl Default for Meter {
    fn default() -> Self {
        Self {
            numerator: 4,
            denominator: 4,
        }
    }
}

impl Meter {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() || text.eq_ignore_ascii_case("none") {
            return None;
        }
        if text.eq_ignore_ascii_case("C") {
            return Some(Self {
                numerator: 4,
                denominator: 4,
            });
        }
        if text.eq_ignore_ascii_case("C|") {
            return Some(Self {
                numerator: 2,
                denominator: 2,
            });
        }
        let (numerator, denominator) = text.split_once('/')?;
        let numerator = numerator.trim().parse::<u32>().ok()?;
        let denominator = denominator.trim().parse::<u32>().ok()?;
        if numerator == 0 || denominator == 0 {
            return None;
        }
        Some(Self {
            numerator,
            denominator,
        })
    }

    pub fn quarters_per_bar(self) -> f64 {
        self.numerator as f64 * 4.0 / self.denominator as f64
    }

    /// One bar is two dance beats by definition, so this also makes the
    /// lead-sheet and the dance card share a grid.
    pub fn bar_dance_beats(self) -> f64 {
        2.0
    }

    pub fn dance_beats_per_quarter(self) -> f64 {
        2.0 / self.quarters_per_bar()
    }

    pub fn dance_beats_per_whole_note(self) -> f64 {
        4.0 * self.dance_beats_per_quarter()
    }

    pub fn default_unit_note_length(self) -> f64 {
        // ABC 2.1: 1/16 for meters less than 3/4, otherwise 1/8.
        if self.quarters_per_bar() < 3.0 {
            1.0 / 16.0
        } else {
            1.0 / 8.0
        }
    }

    pub fn is_compound(self) -> bool {
        self.denominator == 8 && self.numerator % 3 == 0 && self.numerator > 3
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Major,
    Minor,
    Dorian,
    Mixolydian,
    Phrygian,
    Lydian,
    Locrian,
}

impl Mode {
    pub fn intervals(self) -> [i32; 7] {
        match self {
            Mode::Major => [0, 2, 4, 5, 7, 9, 11],
            Mode::Minor => [0, 2, 3, 5, 7, 8, 10],
            Mode::Dorian => [0, 2, 3, 5, 7, 9, 10],
            Mode::Mixolydian => [0, 2, 4, 5, 7, 9, 10],
            Mode::Phrygian => [0, 1, 3, 5, 7, 8, 10],
            Mode::Lydian => [0, 2, 4, 6, 7, 9, 11],
            Mode::Locrian => [0, 1, 3, 5, 6, 8, 10],
        }
    }
}

/// A key signature, including mode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    pub tonic_pitch_class: i32,
    pub tonic_name: String,
    pub mode: Mode,
    pub accidentals: [i32; 7],
}

impl Default for Key {
    fn default() -> Self {
        Self::parse("C").expect("C major is a valid key")
    }
}

impl Key {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return Some(Self::default());
        }
        let mut chars = text.chars().peekable();
        let letter = chars.next()?.to_ascii_uppercase();
        let letter_index = letter_index(letter)?;
        let mut tonic = natural_pitch_class(letter)?;
        if let Some(accidental) = chars.peek() {
            match accidental {
                '#' => {
                    tonic += 1;
                    chars.next();
                }
                'b' | 'B' => {
                    tonic -= 1;
                    chars.next();
                }
                _ => {}
            }
        }
        let rest: String = chars.collect::<String>().to_lowercase();
        let rest = rest.trim();
        let mode = if rest.contains("mix") {
            Mode::Mixolydian
        } else if rest.contains("dor") {
            Mode::Dorian
        } else if rest.contains("phr") {
            Mode::Phrygian
        } else if rest.contains("lyd") {
            Mode::Lydian
        } else if rest.contains("loc") {
            Mode::Locrian
        } else if rest.starts_with("maj") || rest.starts_with("major") {
            Mode::Major
        } else if rest.starts_with('m') || rest.starts_with("min") || rest.contains("aeo") {
            Mode::Minor
        } else {
            Mode::Major
        };

        let tonic = tonic.rem_euclid(12);
        let mut accidentals = [0i32; 7];
        let intervals = mode.intervals();
        for degree in 0..7 {
            let target = (tonic + intervals[degree]).rem_euclid(12);
            let note_letter = (letter_index + degree as usize) % 7;
            let natural = match note_letter {
                0 => 0,
                1 => 2,
                2 => 4,
                3 => 5,
                4 => 7,
                5 => 9,
                6 => 11,
                _ => unreachable!(),
            };
            let mut diff = target - natural;
            if diff > 6 {
                diff -= 12;
            }
            if diff < -6 {
                diff += 12;
            }
            accidentals[note_letter] = diff.clamp(-2, 2);
        }

        let mut tonic_name = letter.to_string();
        if tonic != natural_pitch_class(letter)? {
            tonic_name.push(
                if tonic == (natural_pitch_class(letter)? + 1).rem_euclid(12) {
                    '#'
                } else {
                    'b'
                },
            );
        }

        Some(Self {
            tonic_pitch_class: tonic,
            tonic_name,
            mode,
            accidentals,
        })
    }

    pub fn accidental_for(&self, letter: char) -> i32 {
        letter_index(letter)
            .and_then(|index| self.accidentals.get(index).copied())
            .unwrap_or(0)
    }

    pub fn midi_for(&self, letter: char, octave: i8, explicit: Option<i32>) -> Option<u8> {
        let index = letter_index(letter)?;
        let natural = natural_pitch_class(letter)?;
        let alter = explicit.unwrap_or(self.accidentals[index]);
        let pitch_class = (natural + alter).rem_euclid(12);
        let mut midi = 12 * (octave as i32 + 4) + pitch_class;
        while midi < 24 {
            midi += 12;
        }
        while midi > 108 {
            midi -= 12;
        }
        Some(midi.clamp(0, 127) as u8)
    }

    pub fn tonic_triad(&self) -> [u8; 3] {
        let root = 60 + self.tonic_pitch_class.rem_euclid(12);
        let third = if matches!(self.mode, Mode::Minor | Mode::Dorian | Mode::Phrygian) {
            3
        } else {
            4
        };
        [
            root.clamp(0, 127) as u8,
            (root + third).clamp(0, 127) as u8,
            (root + 7).clamp(0, 127) as u8,
        ]
    }
}

fn letter_index(letter: char) -> Option<usize> {
    match letter.to_ascii_uppercase() {
        'C' => Some(0),
        'D' => Some(1),
        'E' => Some(2),
        'F' => Some(3),
        'G' => Some(4),
        'A' => Some(5),
        'B' => Some(6),
        _ => None,
    }
}

fn natural_pitch_class(letter: char) -> Option<i32> {
    match letter.to_ascii_uppercase() {
        'C' => Some(0),
        'D' => Some(2),
        'E' => Some(4),
        'F' => Some(5),
        'G' => Some(7),
        'A' => Some(9),
        'B' => Some(11),
        _ => None,
    }
}

/// A pitched note with a start and duration in dance beats.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoteEvent {
    pub start_beat: f64,
    pub duration_beats: f64,
    pub midi: u8,
    pub velocity: u8,
}

impl NoteEvent {
    pub fn end_beat(&self) -> f64 {
        self.start_beat + self.duration_beats
    }
}

/// A chord symbol from the ABC and its resolved duration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChordEvent {
    pub start_beat: f64,
    pub duration_beats: f64,
    pub symbol: String,
}

impl ChordEvent {
    pub fn end_beat(&self) -> f64 {
        self.start_beat + self.duration_beats
    }
}

/// A parsed ABC tune, normalised to dance beats.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub title: String,
    pub meter: Meter,
    pub key: Key,
    pub unit_note_length: f64,
    pub notes: Vec<NoteEvent>,
    pub chords: Vec<ChordEvent>,
    /// Absolute times of bar lines, used to detect pickups and align repeats.
    pub bar_lines: Vec<f64>,
    pub duration_beats: f64,
    pub warnings: Vec<String>,
}

impl Score {
    pub fn bars(&self) -> f64 {
        self.duration_beats / 2.0
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// Return a copy shifted so `start_beat` becomes zero, clipped to `length`.
    pub fn slice(&self, start_beat: f64, length: f64) -> Self {
        let end = start_beat + length;
        let mut notes = Vec::new();
        for note in &self.notes {
            let note_end = note.end_beat();
            if note_end <= start_beat || note.start_beat >= end {
                continue;
            }
            let clipped_start = note.start_beat.max(start_beat);
            let clipped_end = note_end.min(end);
            notes.push(NoteEvent {
                start_beat: clipped_start - start_beat,
                duration_beats: clipped_end - clipped_start,
                ..*note
            });
        }
        let mut chords = Vec::new();
        for chord in &self.chords {
            let chord_end = chord.end_beat();
            if chord_end <= start_beat || chord.start_beat >= end {
                continue;
            }
            let clipped_start = chord.start_beat.max(start_beat);
            let clipped_end = chord_end.min(end);
            chords.push(ChordEvent {
                start_beat: clipped_start - start_beat,
                duration_beats: clipped_end - clipped_start,
                ..chord.clone()
            });
        }
        let mut out = self.clone();
        out.notes = notes;
        out.chords = chords;
        out.duration_beats = length;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meters() {
        assert_eq!(
            Meter::parse("4/4"),
            Some(Meter {
                numerator: 4,
                denominator: 4
            })
        );
        assert_eq!(
            Meter::parse("6/8"),
            Some(Meter {
                numerator: 6,
                denominator: 8
            })
        );
        assert_eq!(Meter::parse("C|").unwrap().quarters_per_bar(), 4.0);
        assert_eq!(
            Meter::parse("6/8").unwrap().dance_beats_per_quarter(),
            2.0 / 3.0
        );
    }

    #[test]
    fn parses_keys_and_signatures() {
        let a_mix = Key::parse("A mix").expect("key");
        assert_eq!(a_mix.tonic_name, "A");
        assert_eq!(a_mix.mode, Mode::Mixolydian);
        assert_eq!(a_mix.accidental_for('C'), 1);
        assert_eq!(a_mix.accidental_for('F'), 1);
        assert_eq!(a_mix.accidental_for('G'), 0);

        let g = Key::parse("G").expect("key");
        assert_eq!(g.accidental_for('F'), 1);

        let d_minor = Key::parse("Dm").expect("key");
        assert_eq!(d_minor.mode, Mode::Minor);
        assert_eq!(d_minor.accidental_for('B'), -1);
    }

    #[test]
    fn maps_abc_octaves_to_midi() {
        let key = Key::parse("C").expect("key");
        assert_eq!(key.midi_for('C', 1, None), Some(60));
        assert_eq!(key.midi_for('C', 2, None), Some(72));
        assert_eq!(key.midi_for('A', 1, None), Some(69));
    }

    #[test]
    fn slice_clips_notes() {
        let key = Key::default();
        let score = Score {
            title: "x".into(),
            meter: Meter::default(),
            key,
            unit_note_length: 0.125,
            notes: vec![NoteEvent {
                start_beat: 1.0,
                duration_beats: 2.0,
                midi: 60,
                velocity: 80,
            }],
            chords: Vec::new(),
            bar_lines: Vec::new(),
            duration_beats: 4.0,
            warnings: Vec::new(),
        };
        let sliced = score.slice(2.0, 2.0);
        assert_eq!(sliced.notes.len(), 1);
        assert_eq!(sliced.notes[0].start_beat, 0.0);
        assert_eq!(sliced.notes[0].duration_beats, 1.0);
    }
}
