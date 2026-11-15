//! ABC → score sequencing.
//!
//! `abc-parser` handles ABC syntax (including quoted chord symbols as
//! annotations); this module turns its AST into absolute-time note and chord
//! events in dance beats. Repeats are expanded here because the parser is
//! grammar-only.

use std::collections::HashMap;

use abc_parser::abc;
use abc_parser::datatypes::writer::ToABC;
use abc_parser::datatypes::{
    Accidental, HeaderLine, InfoField, Length, MusicSymbol, Note as AbcNote, Rest, TuneLine,
};

use crate::score::{ChordEvent, Key, Meter, NoteEvent, Score};

#[derive(Debug, thiserror::Error)]
pub enum AbcError {
    #[error("ABC parse error: {0}")]
    Parse(String),
    #[error("ABC input contains no tune")]
    NoTune,
    #[error("tune has no music body")]
    NoBody,
    #[error("tune has no playable notes")]
    NoNotes,
}

type Result<T> = std::result::Result<T, AbcError>;

/// Parse an ABC tune into a score whose times are dance beats.
pub fn parse_abc(input: &str) -> Result<Score> {
    let book = abc::tune_book(input).map_err(|error| AbcError::Parse(format!("{error:?}")))?;
    let tune = book.tunes.first().ok_or(AbcError::NoTune)?;
    let body = tune.body.as_ref().ok_or(AbcError::NoBody)?;

    let mut title = String::new();
    let mut meter = Meter::default();
    let mut key = Key::default();
    let mut unit = None;
    let mut rhythm = String::new();
    let mut warnings = Vec::new();
    let mut meter_seen = false;
    let mut key_seen = false;

    for line in &tune.header.lines {
        let HeaderLine::Field(InfoField(field, value), _) = line else {
            continue;
        };
        match *field {
            'T' if title.is_empty() => title = value.trim().to_string(),
            'R' if !value.trim().is_empty() => rhythm = value.trim().to_string(),
            'M' => match Meter::parse(value) {
                Some(parsed) => {
                    meter = parsed;
                    meter_seen = true;
                }
                None => warnings.push(format!("Unsupported meter '{value}', using 4/4")),
            },
            'L' => match parse_fraction(value) {
                Some(parsed) => unit = Some(parsed),
                None => warnings.push(format!("Unsupported unit note length '{value}'")),
            },
            'K' => match Key::parse(value) {
                Some(parsed) => {
                    key = parsed;
                    key_seen = true;
                }
                None => warnings.push(format!("Unsupported key '{value}', using C major")),
            },
            _ => {}
        }
    }

    if !meter_seen {
        warnings.push("No meter in ABC; assuming 4/4".into());
    }
    if !key_seen {
        warnings.push("No key in ABC; assuming C major".into());
    }
    if !rhythm.is_empty() {
        // Stored only as a warning-free metadata hint; the caller/tune model
        // carries the display rhythm.
    }

    let mut sequencer = Sequencer::new(
        meter,
        key,
        unit.unwrap_or_else(|| meter.default_unit_note_length()),
    );

    for line in &body.lines {
        match line {
            TuneLine::Music(music) => {
                for symbol in &music.symbols {
                    sequencer.process(symbol, 1.0);
                }
            }
            TuneLine::Symbol(_) | TuneLine::Lyric(_) | TuneLine::Comment(_) => {}
        }
    }

    sequencer.finish();
    if sequencer.notes.is_empty() {
        return Err(AbcError::NoNotes);
    }

    warnings.extend(sequencer.warnings);
    Ok(Score {
        title: if title.is_empty() {
            "Untitled".into()
        } else {
            title
        },
        meter: sequencer.meter,
        key: sequencer.key,
        unit_note_length: sequencer.unit,
        notes: sequencer.notes,
        chords: sequencer.chords,
        bar_lines: sequencer.bar_lines,
        duration_beats: sequencer.time,
        warnings,
    })
}

struct RepeatStart {
    note_index: usize,
    chord_index: usize,
    time: f64,
}

struct TupletState {
    remaining: u32,
    factor: f64,
}

struct Sequencer {
    meter: Meter,
    key: Key,
    unit: f64,
    time: f64,
    notes: Vec<NoteEvent>,
    chords: Vec<ChordEvent>,
    bar_lines: Vec<f64>,
    warnings: Vec<String>,
    bar_accidentals: HashMap<(char, i8), i32>,
    pending_tie: Option<(u8, usize)>,
    repeat_start: Option<RepeatStart>,
    tuplet: Option<TupletState>,
    ending_warned: bool,
}

impl Sequencer {
    fn new(meter: Meter, key: Key, unit: f64) -> Self {
        Self {
            meter,
            key,
            unit,
            time: 0.0,
            notes: Vec::new(),
            chords: Vec::new(),
            bar_lines: Vec::new(),
            warnings: Vec::new(),
            bar_accidentals: HashMap::new(),
            pending_tie: None,
            repeat_start: None,
            tuplet: None,
            ending_warned: false,
        }
    }

    fn finish(&mut self) {
        self.chords
            .sort_by(|a, b| a.start_beat.total_cmp(&b.start_beat));
        self.chords
            .dedup_by(|a, b| (a.start_beat - b.start_beat).abs() < 1e-6);
        for index in 0..self.chords.len() {
            let end = self
                .chords
                .get(index + 1)
                .map(|next| next.start_beat)
                .unwrap_or(self.time);
            self.chords[index].duration_beats = (end - self.chords[index].start_beat).max(0.0);
        }
        if self.repeat_start.is_some() {
            self.warnings.push("Repeat was not closed".into());
        }
    }

    fn process(&mut self, symbol: &MusicSymbol, factor: f64) {
        match symbol {
            MusicSymbol::Note {
                accidental,
                note,
                octave,
                length,
                tie,
            } => {
                let duration = self.next_duration(length.as_ref(), factor);
                let letter = abc_note_char(*note);
                let explicit = accidental.map(accidental_offset);
                if let Some(value) = explicit {
                    self.bar_accidentals.insert((letter, *octave), value);
                }
                let alter =
                    explicit.or_else(|| self.bar_accidentals.get(&(letter, *octave)).copied());
                let Some(midi) = self.key.midi_for(letter, *octave, alter) else {
                    self.warnings.push(format!("Could not pitch note {letter}"));
                    self.time += duration;
                    return;
                };
                self.add_note(midi, duration, tie.is_some());
                self.time += duration;
            }
            MusicSymbol::Chord { notes, length, tie } => {
                let duration = self.next_duration(length.as_ref(), factor);
                let pitches = self.chord_pitches(notes);
                if pitches.is_empty() {
                    self.warnings.push("Empty ABC chord".into());
                } else {
                    let single = pitches.len() == 1;
                    for midi in &pitches {
                        let index = self.notes.len();
                        self.notes.push(NoteEvent {
                            start_beat: self.time,
                            duration_beats: duration,
                            midi: *midi,
                            velocity: 78,
                        });
                        if single && tie.is_some() {
                            self.pending_tie = Some((*midi, index));
                        }
                    }
                }
                self.time += duration;
            }
            MusicSymbol::GraceNotes { notes, .. } => {
                // Grace notes are ornamental; they do not take written time.
                // We currently skip them rather than rendering a short
                // approximation, which keeps the dance grid exact.
                let _ = notes;
            }
            MusicSymbol::Tuplet { p, q, .. } => {
                let q = q.unwrap_or_else(|| default_tuplet_q(*p));
                if *p > 0 {
                    self.tuplet = Some(TupletState {
                        remaining: *p,
                        factor: q as f64 / *p as f64,
                    });
                }
            }
            MusicSymbol::BrokenRhythm {
                rhythm,
                before,
                after,
            } => {
                let (long, short) = broken_factors(rhythm);
                let (before_factor, after_factor) = if rhythm.contains('>') {
                    (long, short)
                } else {
                    (short, long)
                };
                for symbol in before {
                    self.process(symbol, factor * before_factor);
                }
                for symbol in after {
                    self.process(symbol, factor * after_factor);
                }
            }
            MusicSymbol::Annotation(annotation) => {
                let raw = annotation.to_abc();
                if let Some(symbol) = chord_symbol_from_annotation(&raw) {
                    self.chords.push(ChordEvent {
                        start_beat: self.time,
                        duration_beats: 0.0,
                        symbol,
                    });
                }
            }
            MusicSymbol::Bar(bar, ending) => {
                if ending.is_some() && !self.ending_warned {
                    self.warnings
                        .push("First/second repeat endings are approximated".into());
                    self.ending_warned = true;
                }
                self.bar_lines.push(self.time);
                if bar.contains(":|") {
                    self.close_repeat();
                }
                if bar.contains("|:") {
                    self.repeat_start = Some(RepeatStart {
                        note_index: self.notes.len(),
                        chord_index: self.chords.len(),
                        time: self.time,
                    });
                }
                self.bar_accidentals.clear();
            }
            MusicSymbol::Ending(_) => {
                if !self.ending_warned {
                    self.warnings
                        .push("First/second repeat endings are approximated".into());
                    self.ending_warned = true;
                }
            }
            MusicSymbol::Rest(rest) => {
                let duration = match rest {
                    Rest::Note(length) | Rest::NoteHidden(length) => {
                        self.next_duration(length.as_ref(), factor)
                    }
                    Rest::Measure(length) | Rest::MeasureHidden(length) => {
                        let bars = length.as_ref().map(length_factor).unwrap_or(1.0);
                        self.meter.bar_dance_beats() * bars * factor
                    }
                };
                self.time += duration;
            }
            MusicSymbol::InlineField(InfoField(field, value), _) => match *field {
                'K' => {
                    if let Some(parsed) = Key::parse(value) {
                        self.key = parsed;
                    }
                }
                'M' => {
                    if let Some(parsed) = Meter::parse(value) {
                        self.meter = parsed;
                    }
                }
                'L' => {
                    if let Some(parsed) = parse_fraction(value) {
                        self.unit = parsed;
                    }
                }
                _ => {}
            },
            MusicSymbol::Decoration(_)
            | MusicSymbol::Slur(_)
            | MusicSymbol::Comment(_)
            | MusicSymbol::Beam(_)
            | MusicSymbol::Spacer
            | MusicSymbol::Space(_)
            | MusicSymbol::Reserved(_) => {}
        }
    }

    fn add_note(&mut self, midi: u8, duration: f64, tie: bool) {
        if let Some((tied_midi, index)) = self.pending_tie {
            if tied_midi == midi {
                if let Some(note) = self.notes.get_mut(index) {
                    note.duration_beats += duration;
                }
                self.pending_tie = if tie { Some((midi, index)) } else { None };
                return;
            }
            self.pending_tie = None;
        }

        let index = self.notes.len();
        self.notes.push(NoteEvent {
            start_beat: self.time,
            duration_beats: duration,
            midi,
            velocity: 80,
        });
        self.pending_tie = if tie { Some((midi, index)) } else { None };
    }

    fn chord_pitches(&mut self, notes: &[MusicSymbol]) -> Vec<u8> {
        let mut pitches = Vec::new();
        for symbol in notes {
            match symbol {
                MusicSymbol::Note {
                    accidental,
                    note,
                    octave,
                    ..
                } => {
                    let letter = abc_note_char(*note);
                    let explicit = accidental.map(accidental_offset);
                    if let Some(value) = explicit {
                        self.bar_accidentals.insert((letter, *octave), value);
                    }
                    let alter =
                        explicit.or_else(|| self.bar_accidentals.get(&(letter, *octave)).copied());
                    if let Some(midi) = self.key.midi_for(letter, *octave, alter) {
                        pitches.push(midi);
                    }
                }
                MusicSymbol::Chord { notes, .. } => {
                    pitches.extend(self.chord_pitches(notes));
                }
                _ => {}
            }
        }
        pitches
    }

    fn close_repeat(&mut self) {
        let start = self.repeat_start.take().unwrap_or(RepeatStart {
            note_index: 0,
            chord_index: 0,
            time: 0.0,
        });
        let delta = self.time - start.time;
        if delta <= 0.0 {
            return;
        }

        let copied_notes: Vec<NoteEvent> = self
            .notes
            .iter()
            .skip(start.note_index)
            .map(|note| NoteEvent {
                start_beat: note.start_beat + delta,
                ..*note
            })
            .collect();
        let copied_chords: Vec<ChordEvent> = self
            .chords
            .iter()
            .skip(start.chord_index)
            .map(|chord| ChordEvent {
                start_beat: chord.start_beat + delta,
                ..chord.clone()
            })
            .collect();

        self.notes.extend(copied_notes);
        self.chords.extend(copied_chords);
        self.time += delta;
        self.bar_accidentals.clear();
        self.tuplet = None;
        self.pending_tie = None;
    }

    fn next_duration(&mut self, length: Option<&Length>, factor: f64) -> f64 {
        let base = self.base_duration(length);
        let mut duration = base * factor;
        if let Some(tuplet) = &mut self.tuplet {
            duration *= tuplet.factor;
            tuplet.remaining = tuplet.remaining.saturating_sub(1);
            if tuplet.remaining == 0 {
                self.tuplet = None;
            }
        }
        duration.max(0.0)
    }

    fn base_duration(&self, length: Option<&Length>) -> f64 {
        let multiplier = length.map(length_factor).unwrap_or(1.0);
        // unit is a fraction of a whole note; a whole note is 4 quarters.
        self.unit * multiplier * 4.0 * self.meter.dance_beats_per_quarter()
    }
}

fn abc_note_char(note: AbcNote) -> char {
    match note {
        AbcNote::C => 'C',
        AbcNote::D => 'D',
        AbcNote::E => 'E',
        AbcNote::F => 'F',
        AbcNote::G => 'G',
        AbcNote::A => 'A',
        AbcNote::B => 'B',
    }
}

fn accidental_offset(accidental: Accidental) -> i32 {
    match accidental {
        Accidental::Natural => 0,
        Accidental::Sharp => 1,
        Accidental::Flat => -1,
        Accidental::DoubleSharp => 2,
        Accidental::DoubleFlat => -2,
    }
}

fn parse_fraction(text: &str) -> Option<f64> {
    let text = text.trim();
    if let Some((numerator, denominator)) = text.split_once('/') {
        let numerator = numerator.trim().parse::<f64>().ok()?;
        let denominator = denominator.trim().parse::<f64>().ok()?;
        if denominator == 0.0 {
            return None;
        }
        Some(numerator / denominator)
    } else {
        text.parse::<f64>().ok()
    }
}

fn length_factor(length: &Length) -> f64 {
    let text = length.to_abc();
    parse_length_text(&text).unwrap_or(1.0)
}

fn parse_length_text(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() || text == "1" {
        return Some(1.0);
    }
    if text.chars().all(|c| c == '/') {
        return Some(0.5_f64.powi(text.len() as i32));
    }
    if let Some(rest) = text.strip_prefix('/') {
        if let Ok(denominator) = rest.parse::<f64>()
            && denominator != 0.0
        {
            return Some(1.0 / denominator);
        }
        return None;
    }
    if let Some((numerator, denominator)) = text.split_once('/') {
        let numerator = numerator.trim().parse::<f64>().ok()?;
        let denominator = denominator.trim().parse::<f64>().ok()?;
        if denominator == 0.0 {
            return None;
        }
        return Some(numerator / denominator);
    }
    text.parse::<f64>().ok()
}

fn default_tuplet_q(p: u32) -> u32 {
    match p {
        2 => 3,
        3 => 2,
        4 => 3,
        5 => 4,
        6 => 4,
        7 => 4,
        8 => 4,
        _ => 2,
    }
}

fn broken_factors(rhythm: &str) -> (f64, f64) {
    let count = rhythm
        .chars()
        .filter(|c| *c == '>' || *c == '<')
        .count()
        .max(1) as i32;
    let short = 0.5_f64.powi(count);
    let long = 2.0 - short;
    (long, short)
}

fn chord_symbol_from_annotation(annotation: &str) -> Option<String> {
    let mut text = annotation.trim().trim_matches('"');
    // Placements: ^ above, _ below, < left, > right, @ auto.
    while text.starts_with('^')
        || text.starts_with('_')
        || text.starts_with('<')
        || text.starts_with('>')
        || text.starts_with('@')
    {
        text = &text[1..];
    }
    let text = text.trim();
    let mut chars = text.chars();
    let first = chars.next()?;
    if !matches!(first, 'A'..='G') {
        return None;
    }
    let mut rest = chars.as_str();
    if let Some(accidental) = rest.chars().next()
        && (accidental == '#' || accidental == 'b')
    {
        rest = &rest[1..];
    }
    // Chord suffixes and slash basses. Reject ordinary words such as "Fine".
    if rest.chars().all(|c| {
        c.is_ascii_digit()
            || c.is_ascii_alphabetic()
            || matches!(
                c,
                '#' | 'b'
                    | '+'
                    | '-'
                    | '/'
                    | '('
                    | ')'
                    | '°'
                    | 'ø'
                    | ' '
                    | 'm'
                    | 'a'
                    | 'j'
                    | 's'
                    | 'u'
                    | 'd'
                    | 'i'
                    | 'v'
            )
    }) && !rest
        .chars()
        .any(|c| c == 'e' || c == 'n' || c == 'o' || c == 't')
    {
        Some(text.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SANDY_BOYS: &str = r#"X:8
T:Sandy Boys
M:4/4
L:1/8
K:A mix
eg |: "A"a2 g2 e2 g2 | ed c2 A2 a2- | a2 g2 e2 g2 | "G"a3 b a2 eg |
"A"a2 g2 e2 g2 | ed c2 A2 AB | c2 cc "E7"d2 c2 | "A"A3 B A2 cd :|
|: "A"e4 e3 e | d2 c2 A3 A | "A"[ce]2 [ce][ce] "D"[df]2 [df][df] |"A"[ce]2 [ce][ce] "D"[df]2 [df][df] |
"A"e4 e3 e | d2 c2 A2 AB | [ce]2 cc "E7"d2 c2 | "A"A3 B A2 :|"#;

    #[test]
    fn parses_sandy_boys() {
        let score = parse_abc(SANDY_BOYS).expect("parse");
        assert_eq!(score.title, "Sandy Boys");
        assert_eq!(score.key.tonic_name, "A");
        assert_eq!(score.meter.numerator, 4);
        assert!(!score.notes.is_empty());
        assert!(!score.chords.is_empty());
        assert!(score.duration_beats > 0.0);
        // 16 bars per pass, repeated twice = 64 dance beats.
        assert!(
            (score.duration_beats - 64.0).abs() < 2.0,
            "duration was {}",
            score.duration_beats
        );
    }

    #[test]
    fn parses_chords_and_ties() {
        let abc = "X:1\nT:Test\nM:4/4\nL:1/8\nK:D\n\"D\"A2- A2 \"G\"B2 B2|\n";
        let score = parse_abc(abc).expect("parse");
        assert_eq!(score.chords.first().map(|c| c.symbol.as_str()), Some("D"));
        assert!(score.notes.iter().any(|n| n.duration_beats >= 1.0));
    }

    #[test]
    fn parses_simple_repeat() {
        let abc = "X:1\nT:Repeat\nM:4/4\nL:1/8\nK:D\n|: A2 A2 A2 A2 :|\n";
        let score = parse_abc(abc).expect("parse");
        assert_eq!(score.notes.len(), 8);
    }

    #[test]
    fn rejects_empty_tune() {
        assert!(matches!(
            parse_abc("X:1\nT:Empty\nK:D\n"),
            Err(AbcError::NoBody)
        ));
    }

    #[test]
    fn length_text_parsing() {
        assert_eq!(parse_length_text("2"), Some(2.0));
        assert_eq!(parse_length_text("3/2"), Some(1.5));
        assert_eq!(parse_length_text("/"), Some(0.5));
        assert_eq!(parse_length_text("///"), Some(0.125));
        assert_eq!(parse_length_text("/5"), Some(0.2));
    }

    #[test]
    fn chord_annotation_filter() {
        assert_eq!(chord_symbol_from_annotation("\"A\""), Some("A".to_string()));
        assert_eq!(
            chord_symbol_from_annotation("\"^G/B\""),
            Some("G/B".to_string())
        );
        assert_eq!(chord_symbol_from_annotation("\"Fine\""), None);
    }
}
