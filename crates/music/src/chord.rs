//! Chord symbols, accompaniment voicings, and simple chord inference.

use crate::score::{ChordEvent, Key, Mode, NoteEvent, Score};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChordQuality {
    Major,
    Minor,
    Dominant7,
    Minor7,
    Major7,
    Diminished,
    Diminished7,
    Augmented,
    Sus2,
    Sus4,
    Major6,
    Minor6,
    Other,
}

impl ChordQuality {
    pub fn intervals(self) -> &'static [i32] {
        match self {
            Self::Major => &[0, 4, 7],
            Self::Minor => &[0, 3, 7],
            Self::Dominant7 => &[0, 4, 7, 10],
            Self::Minor7 => &[0, 3, 7, 10],
            Self::Major7 => &[0, 4, 7, 11],
            Self::Diminished => &[0, 3, 6],
            Self::Diminished7 => &[0, 3, 6, 9],
            Self::Augmented => &[0, 4, 8],
            Self::Sus2 => &[0, 2, 7],
            Self::Sus4 => &[0, 5, 7],
            Self::Major6 => &[0, 4, 7, 9],
            Self::Minor6 => &[0, 3, 7, 9],
            Self::Other => &[0, 4, 7],
        }
    }

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Major => "",
            Self::Minor => "m",
            Self::Dominant7 => "7",
            Self::Minor7 => "m7",
            Self::Major7 => "maj7",
            Self::Diminished => "dim",
            Self::Diminished7 => "dim7",
            Self::Augmented => "aug",
            Self::Sus2 => "sus2",
            Self::Sus4 => "sus4",
            Self::Major6 => "6",
            Self::Minor6 => "m6",
            Self::Other => "",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chord {
    pub root: i32,
    pub quality: ChordQuality,
    pub bass: Option<i32>,
}

impl Chord {
    pub fn parse(symbol: &str) -> Option<Self> {
        let text = symbol.trim();
        let (main, bass) = match text.split_once('/') {
            Some((main, bass)) => (main, Some(parse_pitch_class(bass)?)),
            None => (text, None),
        };
        let mut chars = main.chars();
        let letter = chars.next()?.to_ascii_uppercase();
        let mut root = match letter {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return None,
        };
        let mut rest = chars.as_str();
        if let Some(accidental) = rest.chars().next() {
            match accidental {
                '#' => {
                    root = (root + 1) % 12;
                    rest = &rest[1..];
                }
                'b' => {
                    root = (root + 11) % 12;
                    rest = &rest[1..];
                }
                _ => {}
            }
        }
        let rest = rest.trim();
        let quality = match rest {
            "" | "maj" | "M" | "major" => ChordQuality::Major,
            "m" | "min" | "-" | "minor" => ChordQuality::Minor,
            "7" | "dom7" => ChordQuality::Dominant7,
            "m7" | "min7" | "-7" => ChordQuality::Minor7,
            "maj7" | "M7" | "Δ7" => ChordQuality::Major7,
            "dim" | "°" | "o" => ChordQuality::Diminished,
            "dim7" | "°7" | "o7" => ChordQuality::Diminished7,
            "aug" | "+" | "#5" => ChordQuality::Augmented,
            "sus2" => ChordQuality::Sus2,
            "sus4" | "sus" => ChordQuality::Sus4,
            "6" => ChordQuality::Major6,
            "m6" | "min6" => ChordQuality::Minor6,
            // Common extensions that can be treated as their base triad/7th.
            "9" | "add9" | "7b9" | "7#9" => ChordQuality::Dominant7,
            "m9" | "min9" => ChordQuality::Minor7,
            "maj9" | "M9" => ChordQuality::Major7,
            "madd9" => ChordQuality::Minor,
            _ => return None,
        };
        Some(Self {
            root,
            quality,
            bass,
        })
    }

    pub fn from_root_quality(root: i32, quality: ChordQuality) -> Self {
        Self {
            root: root.rem_euclid(12),
            quality,
            bass: None,
        }
    }

    pub fn symbol(&self) -> String {
        let name = pitch_name(self.root);
        format!("{}{}", name, self.quality.suffix())
    }

    /// Root note in the bass register.
    pub fn bass_midi(&self) -> u8 {
        let root = self.bass.unwrap_or(self.root);
        (36 + root.rem_euclid(12)) as u8
    }

    /// Close-position chord voicing in the accompaniment register.
    pub fn voicing(&self, low: i32) -> Vec<u8> {
        let root_pc = self.root.rem_euclid(12);
        let base = low + (root_pc - low).rem_euclid(12);
        let mut notes = Vec::new();
        for interval in self.quality.intervals() {
            notes.push((base + interval).clamp(0, 127) as u8);
        }
        // Add the root an octave above for a fuller accompaniment.
        let top = base + 12;
        if top <= 108 {
            notes.push(top as u8);
        }
        notes.sort_unstable();
        notes.dedup();
        notes
    }
}

fn parse_pitch_class(text: &str) -> Option<i32> {
    let text = text.trim();
    let mut chars = text.chars();
    let letter = chars.next()?.to_ascii_uppercase();
    let mut pitch = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    if let Some(accidental) = chars.next() {
        match accidental {
            '#' => pitch = (pitch + 1) % 12,
            'b' => pitch = (pitch + 11) % 12,
            _ => return None,
        }
    }
    if chars.next().is_some() {
        return None;
    }
    Some(pitch)
}

fn pitch_name(pitch: i32) -> &'static str {
    match pitch.rem_euclid(12) {
        0 => "C",
        1 => "C#",
        2 => "D",
        3 => "D#",
        4 => "E",
        5 => "F",
        6 => "F#",
        7 => "G",
        8 => "G#",
        9 => "A",
        10 => "A#",
        11 => "B",
        _ => unreachable!(),
    }
}

/// Resolve the score's chord track, inferring one chord per bar when the ABC
/// has no chord symbols.
pub fn resolve_chords(score: &Score) -> Vec<ChordEvent> {
    if !score.chords.is_empty() {
        let mut chords = score.chords.clone();
        for chord in &mut chords {
            if Chord::parse(&chord.symbol).is_none() {
                chord.symbol = tonic_symbol(&score.key);
            }
        }
        return chords;
    }

    let mut out = Vec::new();
    let bars = score.duration_beats / 2.0;
    let bar_count = bars.ceil() as usize;
    let mut previous = tonic_chord(&score.key);
    for bar in 0..bar_count {
        let start = bar as f64 * 2.0;
        let end = start + 2.0;
        let chord = best_chord_for_bar(score, &score.key, start, end).unwrap_or(previous.clone());
        let symbol = chord.symbol();
        previous = chord;
        out.push(ChordEvent {
            start_beat: start,
            duration_beats: (end.min(score.duration_beats) - start).max(0.125),
            symbol,
        });
    }
    if out.is_empty() {
        out.push(ChordEvent {
            start_beat: 0.0,
            duration_beats: score.duration_beats.max(2.0),
            symbol: tonic_symbol(&score.key),
        });
    }
    out
}

fn best_chord_for_bar(score: &Score, key: &Key, start: f64, end: f64) -> Option<Chord> {
    let notes: Vec<&NoteEvent> = score
        .notes
        .iter()
        .filter(|note| note.start_beat < end && note.end_beat() > start)
        .collect();
    if notes.is_empty() {
        return Some(tonic_chord(key));
    }

    let mut candidates = Vec::new();
    for degree in [0usize, 3, 4, 5, 1] {
        if let Some(chord) = diatonic_triad(key, degree) {
            candidates.push(chord);
        }
    }

    let mut best = candidates.first()?.clone();
    let mut best_score = f64::NEG_INFINITY;
    for chord in candidates {
        let tones: Vec<i32> = chord
            .quality
            .intervals()
            .iter()
            .map(|interval| (chord.root + interval).rem_euclid(12))
            .collect();
        let mut score_value = 0.0;
        for note in &notes {
            let pc = (note.midi as i32).rem_euclid(12);
            let weight = note.duration_beats.max(0.25);
            if tones.contains(&pc) {
                score_value += weight;
            }
            if (note.midi as i32 % 12) == chord.root {
                score_value += weight * 0.5;
            }
        }
        // Prefer the tonic on ties.
        if score_value > best_score + 1e-9 {
            best_score = score_value;
            best = chord;
        }
    }
    Some(best)
}

fn diatonic_triad(key: &Key, degree: usize) -> Option<Chord> {
    let intervals = key.mode.intervals();
    let root_interval = intervals[degree % 7];
    let third_interval = intervals[(degree + 2) % 7];
    let fifth_interval = intervals[(degree + 4) % 7];
    let third = adjust_degree(third_interval, degree + 2, root_interval);
    let fifth = adjust_degree(fifth_interval, degree + 4, root_interval);
    let quality = match (third - root_interval, fifth - root_interval) {
        (4, 7) => ChordQuality::Major,
        (3, 7) => ChordQuality::Minor,
        (3, 6) => ChordQuality::Diminished,
        (4, 8) => ChordQuality::Augmented,
        _ => ChordQuality::Major,
    };
    Some(Chord::from_root_quality(
        key.tonic_pitch_class + root_interval,
        quality,
    ))
}

fn adjust_degree(interval: i32, degree: usize, root_interval: i32) -> i32 {
    if degree / 7 > 0 && interval < root_interval {
        interval + 12
    } else {
        interval
    }
}

fn tonic_chord(key: &Key) -> Chord {
    let quality = match key.mode {
        Mode::Minor | Mode::Dorian | Mode::Phrygian => ChordQuality::Minor,
        _ => ChordQuality::Major,
    };
    Chord::from_root_quality(key.tonic_pitch_class, quality)
}

fn tonic_symbol(key: &Key) -> String {
    tonic_chord(key).symbol()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::{Key, NoteEvent, Score};

    #[test]
    fn parses_common_symbols() {
        assert_eq!(Chord::parse("A").unwrap().root, 9);
        assert_eq!(Chord::parse("G/B").unwrap().bass, Some(11));
        assert_eq!(Chord::parse("E7").unwrap().quality, ChordQuality::Dominant7);
        assert_eq!(Chord::parse("Dm").unwrap().quality, ChordQuality::Minor);
        assert_eq!(Chord::parse("C#m7").unwrap().root, 1);
        assert_eq!(Chord::parse("Fine"), None);
    }

    #[test]
    fn voicing_stays_in_range() {
        let chord = Chord::parse("A").unwrap();
        let notes = chord.voicing(55);
        assert!(!notes.is_empty());
        assert!(notes.iter().all(|note| *note >= 55 && *note <= 90));
        assert!(notes.contains(&(57 + 4)));
    }

    #[test]
    fn infers_chords_for_a_simple_melody() {
        let key = Key::parse("D").unwrap();
        let score = Score {
            title: "test".into(),
            meter: crate::score::Meter::default(),
            key,
            unit_note_length: 0.125,
            notes: vec![
                NoteEvent {
                    start_beat: 0.0,
                    duration_beats: 2.0,
                    midi: 62, // D
                    velocity: 80,
                },
                NoteEvent {
                    start_beat: 2.0,
                    duration_beats: 2.0,
                    midi: 67, // G
                    velocity: 80,
                },
            ],
            chords: Vec::new(),
            bar_lines: Vec::new(),
            duration_beats: 4.0,
            warnings: Vec::new(),
        };
        let chords = resolve_chords(&score);
        assert_eq!(chords.len(), 2);
        assert_eq!(chords[0].symbol, "D");
        assert_eq!(chords[1].symbol, "G");
    }
}
