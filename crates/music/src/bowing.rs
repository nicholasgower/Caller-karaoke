//! Fiddle bowing for the tune line.
//!
//! Two rules shape the line:
//!
//! * A quarter note, two eighths, and another quarter are a *bow-change*
//!   figure: the two eighths are re-struck on their own bows, never slurred
//!   together, and carried with a small accent so the figure lifts instead of
//!   running straight through.
//! * Four to eight successive eighth notes are taken in one bow: each eighth is
//!   tied into the next, so the run flows as a single stroke instead of being
//!   chopped into separate bows.
//!
//! The score model has no slur, so the rules are written the way a player hears
//! them. A bow change shortens the note a little to leave a fresh attack; a tie
//! rings each note into the next. Note starts never move, so the dance grid,
//! the calls, and the 64-beat loop are untouched.
//!
//! The melody and countermelody parts are fiddle lines by default; a different
//! voice playing one of those parts inherits the same bowing.

use caller_core::BowingOptions;

use crate::score::{Meter, NoteEvent};

/// Velocity added to the two eighth notes of a bow-change figure.
///
/// The melody floor is 92 and its ceiling 124, so this lifts an offbeat eighth
/// to 114: clearly above the quarter notes around it (95-110) without running
/// into the top of the MIDI range.
pub const BOW_CHANGE_ACCENT: u8 = 22;

/// Fraction of each eighth kept when it is re-struck.
///
/// A bow change is an attack, not a staccato: most of the note rings, and only
/// a short gap separates it from the next stroke.
pub const BOW_CHANGE_DURATION: f64 = 0.90;

/// Fewest successive eighths that are tied into one bow.
pub const MIN_TIED_EIGHTHS: usize = 4;

/// Most successive eighths that are tied into one bow.
///
/// A 4/4 bar holds eight eighth notes, so this is one bar of running eighths;
/// a longer run is left alone rather than slurred across more than the rule
/// covers.
pub const MAX_TIED_EIGHTHS: usize = 8;

/// How far a tied eighth rings into the next one, as a fraction of the gap.
///
/// The notes overlap slightly so the bow does not stop between them. Repeated
/// pitches are excluded because a fiddle re-strikes those and overlapping the
/// same key would cut the second note short in an exported MIDI file.
pub const TIE_OVERLAP: f64 = 0.10;

/// Highest velocity the tune line may reach.
///
/// The arranger leaves a little headroom below full velocity so the mix and the
/// effects chain never have to clip a melody note.
pub const TUNE_VELOCITY_CEILING: u8 = 124;

/// Duration comparison tolerance, in dance beats.
///
/// The ABC parser multiplies exact fractions, so even a compound-meter eighth
/// is within a rounding error of its nominal length. Broken rhythms and tuplets
/// are deliberately outside the figures and must not match them.
const EPSILON: f64 = 1e-6;

/// One onset of the written line, with its chord notes kept together.
struct Onset {
    start: f64,
    /// Longest note at this onset: the value the rhythm is read from.
    beats: f64,
    indices: Vec<usize>,
}

/// Apply both fiddle rules to a written line and return how many it applied.
///
/// This is the default for callers that do not carry user settings; the app
/// uses [`apply_with_options`] so each rule can be switched off.
pub fn apply(notes: &mut [NoteEvent], meter: Meter) -> usize {
    apply_with_options(notes, meter, &BowingOptions::default())
}

/// Apply the selected fiddle rules to a written line and return how many it
/// applied.
///
/// A bow-change figure is a quarter note, two eighths starting exactly where
/// the previous note ends, and a quarter note closing the group. A tie is a run
/// of [`MIN_TIED_EIGHTHS`] to [`MAX_TIED_EIGHTHS`] eighths in succession. A
/// rest, a tie across a bar line that changes the length, or any other rhythm
/// breaks both patterns.
pub fn apply_with_options(notes: &mut [NoteEvent], meter: Meter, options: &BowingOptions) -> usize {
    let quarter = meter.dance_beats_per_quarter();
    if !quarter.is_finite() || quarter <= 0.0 {
        return 0;
    }
    let eighth = quarter * 0.5;
    let onsets = onsets(notes);
    // The rules never claim the same notes (two eighths between quarters are
    // not a run of four), so the order only decides which count is reported
    // first.
    let mut applied = 0;
    if options.tied_runs {
        applied += tie_runs(notes, &onsets, eighth);
    }
    if options.bow_change {
        applied += bow_change_figures(notes, &onsets, quarter, eighth);
    }
    applied
}

/// Tie every qualifying run of eighths in place.
fn tie_runs(notes: &mut [NoteEvent], onsets: &[Onset], eighth: f64) -> usize {
    let mut applied = 0;
    let mut index = 0;
    while index < onsets.len() {
        if !close(onsets[index].beats, eighth) {
            index += 1;
            continue;
        }
        let start = index;
        while index + 1 < onsets.len()
            && close(onsets[index + 1].beats, eighth)
            && contiguous(&onsets[index], &onsets[index + 1])
        {
            index += 1;
        }
        let count = index - start + 1;
        if (MIN_TIED_EIGHTHS..=MAX_TIED_EIGHTHS).contains(&count) {
            for pair in onsets[start..=index].windows(2) {
                let gap = pair[1].start - pair[0].start;
                let repeats_a_pitch = pair[0].indices.iter().any(|first| {
                    pair[1]
                        .indices
                        .iter()
                        .any(|second| notes[*first].midi == notes[*second].midi)
                });
                let duration = if repeats_a_pitch {
                    // A repeated pitch is a new stroke, not a tie; keep the
                    // notes edge to edge so a MIDI player cannot cut one short.
                    gap
                } else {
                    gap * (1.0 + TIE_OVERLAP)
                };
                for &note_index in &pair[0].indices {
                    notes[note_index].duration_beats =
                        notes[note_index].duration_beats.max(duration);
                }
            }
            applied += 1;
        }
        index += 1;
    }
    applied
}

/// Accent and re-strike every qualifying bow-change figure in place.
fn bow_change_figures(
    notes: &mut [NoteEvent],
    onsets: &[Onset],
    quarter: f64,
    eighth: f64,
) -> usize {
    let mut applied = 0;

    for window in onsets.windows(4) {
        let [first, first_eighth, second_eighth, last] = window else {
            continue;
        };
        if !close(first.beats, quarter)
            || !contiguous(first, first_eighth)
            || !close(first_eighth.beats, eighth)
            || !contiguous(first_eighth, second_eighth)
            || !close(second_eighth.beats, eighth)
            || !contiguous(second_eighth, last)
            || !close(last.beats, quarter)
        {
            continue;
        }

        for onset in [first_eighth, second_eighth] {
            for &index in &onset.indices {
                let note = &mut notes[index];
                note.velocity = note
                    .velocity
                    .saturating_add(BOW_CHANGE_ACCENT)
                    .min(TUNE_VELOCITY_CEILING);
                note.duration_beats *= BOW_CHANGE_DURATION;
            }
        }
        applied += 1;
    }

    applied
}

/// Whether `next` starts exactly where `previous` ends.
fn contiguous(previous: &Onset, next: &Onset) -> bool {
    (previous.start + previous.beats - next.start).abs() <= EPSILON
}

/// Group the line by onset so a double stop counts as one rhythmic event.
fn onsets(notes: &[NoteEvent]) -> Vec<Onset> {
    let mut order: Vec<usize> = (0..notes.len()).collect();
    order.sort_by(|&a, &b| {
        notes[a]
            .start_beat
            .total_cmp(&notes[b].start_beat)
            .then(notes[a].midi.cmp(&notes[b].midi))
    });

    let mut onsets: Vec<Onset> = Vec::new();
    for index in order {
        let note = notes[index];
        match onsets.last_mut() {
            Some(onset) if (onset.start - note.start_beat).abs() <= EPSILON => {
                onset.beats = onset.beats.max(note.duration_beats);
                onset.indices.push(index);
            }
            _ => onsets.push(Onset {
                start: note.start_beat,
                beats: note.duration_beats,
                indices: vec![index],
            }),
        }
    }
    onsets
}

fn close(value: f64, expected: f64) -> bool {
    (value - expected).abs() <= EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOUR_FOUR: Meter = Meter {
        numerator: 4,
        denominator: 4,
    };
    const SIX_EIGHT: Meter = Meter {
        numerator: 6,
        denominator: 8,
    };

    fn note(start: f64, duration: f64, midi: u8) -> NoteEvent {
        NoteEvent {
            start_beat: start,
            duration_beats: duration,
            midi,
            velocity: 92,
        }
    }

    /// Quarter, two eighths, quarter in a 4/4 bar.
    fn reel_figure() -> Vec<NoteEvent> {
        vec![
            note(0.0, 0.5, 60),
            note(0.5, 0.25, 62),
            note(0.75, 0.25, 64),
            note(1.0, 0.5, 65),
        ]
    }

    /// `count` successive eighths in 4/4, starting at `start`.
    fn eighth_run(start: f64, count: usize) -> Vec<NoteEvent> {
        (0..count)
            .map(|index| note(start + index as f64 * 0.25, 0.25, 60 + index as u8))
            .collect()
    }

    #[test]
    fn accents_and_re_strokes_the_two_eighths_between_quarters() {
        let mut notes = reel_figure();
        assert_eq!(apply(&mut notes, FOUR_FOUR), 1);
        // The quarters are untouched.
        assert_eq!(notes[0], note(0.0, 0.5, 60));
        assert_eq!(notes[3], note(1.0, 0.5, 65));
        // The eighths are accented and detached for the new bow.
        for index in [1, 2] {
            assert_eq!(notes[index].velocity, 92 + BOW_CHANGE_ACCENT);
            assert!(
                notes[index].duration_beats < 0.25,
                "the eighth at {} should be re-struck, not slurred",
                notes[index].start_beat
            );
        }
    }

    #[test]
    fn note_starts_never_move() {
        let mut notes = reel_figure();
        let starts: Vec<f64> = notes.iter().map(|note| note.start_beat).collect();
        apply(&mut notes, FOUR_FOUR);
        let after: Vec<f64> = notes.iter().map(|note| note.start_beat).collect();
        assert_eq!(starts, after, "the bow change must stay on the dance grid");
    }

    #[test]
    fn a_gap_or_a_different_rhythm_is_not_the_figure() {
        // A rest between the quarter and the eighths.
        let mut gapped = vec![
            note(0.0, 0.5, 60),
            note(0.75, 0.25, 62),
            note(1.0, 0.25, 64),
            note(1.25, 0.5, 65),
        ];
        assert_eq!(apply(&mut gapped, FOUR_FOUR), 0);

        // Three even eighths are too short for a tie and are not a bow-change
        // figure either.
        let mut short_run: Vec<NoteEvent> = (0..3)
            .map(|index| note(index as f64 * 0.25, 0.25, 60 + index as u8))
            .collect();
        assert_eq!(apply(&mut short_run, FOUR_FOUR), 0);

        // Two eighths followed by no closing quarter.
        let mut open = vec![
            note(0.0, 0.5, 60),
            note(0.5, 0.25, 62),
            note(0.75, 0.25, 64),
        ];
        assert_eq!(apply(&mut open, FOUR_FOUR), 0);
    }

    #[test]
    fn a_double_stop_counts_as_one_rhythmic_onset() {
        let mut notes = reel_figure();
        // Add a lower string to the first eighth; both notes are one stroke.
        notes.push(note(0.5, 0.25, 55));
        assert_eq!(apply(&mut notes, FOUR_FOUR), 1);
        for note in notes
            .iter()
            .filter(|note| (note.start_beat - 0.5).abs() < 1e-9)
        {
            assert_eq!(note.velocity, 92 + BOW_CHANGE_ACCENT);
            assert!(note.duration_beats < 0.25);
        }
    }

    #[test]
    fn compound_meter_uses_the_meter_s_quarter_and_eighth() {
        // A 6/8 jig: quarter = 2/3 dance beat, eighth = 1/3.
        let quarter = SIX_EIGHT.dance_beats_per_quarter();
        let eighth = quarter * 0.5;
        let mut notes = vec![
            note(0.0, quarter, 67),
            note(quarter, eighth, 69),
            note(quarter + eighth, eighth, 71),
            note(quarter + eighth * 2.0, quarter, 72),
        ];
        assert_eq!(apply(&mut notes, SIX_EIGHT), 1);
        assert_eq!(notes[1].velocity, 92 + BOW_CHANGE_ACCENT);
        assert_eq!(notes[2].velocity, 92 + BOW_CHANGE_ACCENT);
        assert!(notes[1].duration_beats < eighth);
    }

    #[test]
    fn four_successive_eighths_are_tied_into_one_bow() {
        let mut notes = eighth_run(0.0, 4);
        assert_eq!(apply(&mut notes, FOUR_FOUR), 1);
        let tied = 0.25 * (1.0 + TIE_OVERLAP);
        for note in &notes[..3] {
            assert!(
                (note.duration_beats - tied).abs() < 1e-9,
                "the eighth at {} should ring into the next one",
                note.start_beat
            );
        }
        assert_eq!(
            notes[3].duration_beats, 0.25,
            "the last eighth ends the bow and keeps its written length"
        );
        assert!(
            notes
                .windows(2)
                .all(|pair| pair[0].start_beat < pair[1].start_beat)
        );
    }

    #[test]
    fn eight_eighths_fill_one_bow_and_nine_are_left_alone() {
        let mut eight = eighth_run(0.0, 8);
        assert_eq!(apply(&mut eight, FOUR_FOUR), 1);
        assert!(eight[..7].iter().all(|note| note.duration_beats > 0.25));
        assert_eq!(eight[7].duration_beats, 0.25);

        let mut nine = eighth_run(0.0, 9);
        assert_eq!(apply(&mut nine, FOUR_FOUR), 0);
        assert!(
            nine.iter().all(|note| note.duration_beats == 0.25),
            "a run longer than one bar is not tied"
        );
    }

    #[test]
    fn a_repeated_pitch_is_re_struck_rather_than_overlapped() {
        // Same pitch four times: a fiddle has to change bow, and overlapping
        // the same key would cut the later note short in MIDI.
        let mut notes: Vec<NoteEvent> = (0..4)
            .map(|index| note(index as f64 * 0.25, 0.25, 60))
            .collect();
        assert_eq!(apply(&mut notes, FOUR_FOUR), 1);
        assert!(notes.iter().all(|note| note.duration_beats == 0.25));
    }

    #[test]
    fn compound_meter_ties_a_run_of_eighths() {
        let eighth = SIX_EIGHT.dance_beats_per_quarter() * 0.5;
        let mut notes: Vec<NoteEvent> = (0..4)
            .map(|index| note(index as f64 * eighth, eighth, 67 + index as u8))
            .collect();
        assert_eq!(apply(&mut notes, SIX_EIGHT), 1);
        assert!(notes[0].duration_beats > eighth);
        assert_eq!(notes[3].duration_beats, eighth);
    }

    #[test]
    fn a_run_after_a_quarter_is_tied_without_the_bow_change_accent() {
        // Q E E E E Q: four eighths in one bow, so the figure rule must not
        // claim the two middle eighths.
        let mut notes = vec![note(0.0, 0.5, 60)];
        notes.extend(eighth_run(0.5, 4));
        notes.push(note(1.5, 0.5, 72));
        assert_eq!(apply(&mut notes, FOUR_FOUR), 1);
        for note in &notes[1..5] {
            assert_eq!(note.velocity, 92, "a tied eighth is not re-accented");
            assert!(note.duration_beats >= 0.25);
        }
    }

    #[test]
    fn each_rule_can_be_switched_off_on_its_own() {
        // A bow-change figure followed by a four-eighth run.
        let line = || {
            let mut notes = reel_figure();
            notes.extend(eighth_run(1.5, 4));
            notes
        };

        let mut both = line();
        assert_eq!(apply(&mut both, FOUR_FOUR), 2);

        let mut bow_only = line();
        let bow_options = BowingOptions {
            bow_change: true,
            tied_runs: false,
        };
        assert_eq!(
            apply_with_options(&mut bow_only, FOUR_FOUR, &bow_options),
            1
        );
        assert_eq!(bow_only[1].velocity, 92 + BOW_CHANGE_ACCENT);
        assert!(
            bow_only[4..7]
                .iter()
                .all(|note| note.duration_beats == 0.25),
            "the run must stay un-tied when the tie rule is off"
        );

        let mut tie_only = line();
        let tie_options = BowingOptions {
            bow_change: false,
            tied_runs: true,
        };
        assert_eq!(
            apply_with_options(&mut tie_only, FOUR_FOUR, &tie_options),
            1
        );
        assert_eq!(tie_only[1].velocity, 92);
        assert!(
            tie_only[4].duration_beats > 0.25,
            "the run must still be tied when the figure rule is off"
        );

        let original = line();
        let mut none = original.clone();
        let off = BowingOptions {
            bow_change: false,
            tied_runs: false,
        };
        assert_eq!(apply_with_options(&mut none, FOUR_FOUR, &off), 0);
        assert_eq!(
            none, original,
            "both rules off must leave the line exactly as written"
        );
    }

    #[test]
    fn every_figure_in_a_line_is_found() {
        // Two figures in a row: Q E E Q Q E E Q.
        let mut notes = reel_figure();
        let mut second = reel_figure();
        for note in &mut second {
            note.start_beat += 1.5;
            note.midi += 1;
        }
        notes.extend(second);
        assert_eq!(apply(&mut notes, FOUR_FOUR), 2);
        assert_eq!(notes[1].velocity, 92 + BOW_CHANGE_ACCENT);
        assert_eq!(notes[5].velocity, 92 + BOW_CHANGE_ACCENT);
    }
}
