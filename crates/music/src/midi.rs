//! Standard MIDI File export.
//!
//! The exported file is the arranged 64-beat loop (not the count-in), with a
//! melody track, an accompaniment track, and — when the second accompaniment
//! is on — a countermelody track. When variations are enabled the file contains
//! the same passes as playback, so the melody is accented and varied instead of
//! repeated identically.

use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use contra_core::{
    AccompanimentInstrument, AccompanimentRhythm, MelodyInstrument, Percussion, PercussionRhythm,
    SecondAccompaniment,
};

use crate::arrange::Arrangement;
use crate::score::NoteEvent;

pub const TICKS_PER_QUARTER: u16 = 480;
pub const TICKS_PER_DANCE_BEAT: u64 = TICKS_PER_QUARTER as u64 * 2;

/// Build a Standard MIDI File with the default set of variation passes.
#[allow(clippy::too_many_arguments)]
pub fn export_midi(
    arrangement: &Arrangement,
    melody: MelodyInstrument,
    accompaniment: AccompanimentInstrument,
    accompaniment_rhythm: AccompanimentRhythm,
    second_accompaniment: SecondAccompaniment,
    percussion: Percussion,
    percussion_rhythm: PercussionRhythm,
    bpm: f64,
) -> Vec<u8> {
    export_midi_with_passes(
        arrangement,
        melody,
        accompaniment,
        accompaniment_rhythm,
        second_accompaniment,
        percussion,
        percussion_rhythm,
        bpm,
        crate::VARIATION_PASSES,
    )
}

/// Build a Standard MIDI File for the arranged loop.
///
/// `variation_passes` is the number of 64-beat passes to write back to back,
/// matching [`crate::RenderRequest::variation_passes`]. One pass is the plain
/// loop; later passes carry the melody and accompaniment variations.
#[allow(clippy::too_many_arguments)]
pub fn export_midi_with_passes(
    arrangement: &Arrangement,
    melody: MelodyInstrument,
    accompaniment: AccompanimentInstrument,
    accompaniment_rhythm: AccompanimentRhythm,
    second_accompaniment: SecondAccompaniment,
    percussion: Percussion,
    percussion_rhythm: PercussionRhythm,
    bpm: f64,
    variation_passes: u32,
) -> Vec<u8> {
    let passes = variation_passes.max(1);
    let mut melody_notes = Vec::new();
    let mut accompaniment_notes = Vec::new();
    let mut countermelody_notes = Vec::new();
    let mut percussion_notes = Vec::new();
    for pass in 0..passes {
        let offset = pass as f64 * arrangement.loop_beats;
        melody_notes.extend(
            arrangement
                .melody_notes_for_pass(pass)
                .into_iter()
                .map(|note| shifted(note, offset)),
        );
        accompaniment_notes.extend(
            arrangement
                .accompaniment_notes_for_pass_with_rhythm(accompaniment, accompaniment_rhythm, pass)
                .into_iter()
                .map(|note| shifted(note, offset)),
        );
        if second_accompaniment.instrument().is_some() {
            countermelody_notes.extend(
                arrangement
                    .countermelody_notes_for_pass(pass)
                    .into_iter()
                    .map(|note| shifted(note, offset)),
            );
        }
        if percussion.is_enabled() {
            percussion_notes.extend(
                crate::percussion::notes(
                    arrangement.loop_beats,
                    arrangement.meter,
                    percussion,
                    percussion_rhythm,
                )
                .into_iter()
                .map(|note| shifted(note, offset)),
            );
        }
    }
    let end_beats = arrangement.loop_beats * passes as f64;

    let mut tracks = vec![
        build_track(
            0,
            melody.gm_program(),
            "Melody",
            &melody_notes,
            true,
            bpm,
            arrangement,
            end_beats,
        ),
        build_track(
            1,
            accompaniment.gm_program(),
            "Accompaniment",
            &accompaniment_notes,
            false,
            bpm,
            arrangement,
            end_beats,
        ),
    ];
    if let Some(instrument) = second_accompaniment.instrument() {
        tracks.push(build_track(
            2,
            instrument.gm_program(),
            "Second accompaniment",
            &countermelody_notes,
            false,
            bpm,
            arrangement,
            end_beats,
        ));
    }
    if percussion.is_enabled() {
        tracks.push(build_track(
            9,
            0,
            "Percussion",
            &percussion_notes,
            false,
            bpm,
            arrangement,
            end_beats,
        ));
    }

    let smf = Smf {
        header: Header::new(
            Format::Parallel,
            Timing::Metrical(u15::new(TICKS_PER_QUARTER)),
        ),
        tracks,
    };
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes)
        .expect("writing an in-memory MIDI file cannot fail");
    bytes
}

fn shifted(mut note: NoteEvent, offset_beats: f64) -> NoteEvent {
    note.start_beat += offset_beats;
    note
}

// One argument per independent track property; grouping them into a struct
// would obscure the call sites more than it helps.
#[allow(clippy::too_many_arguments)]
fn build_track(
    channel: u8,
    program: u8,
    name: &'static str,
    notes: &[NoteEvent],
    include_metadata: bool,
    bpm: f64,
    arrangement: &Arrangement,
    end_beats: f64,
) -> Vec<TrackEvent<'static>> {
    let mut events: Vec<(u64, u8, TrackEventKind<'static>)> = Vec::new();
    let channel = u4::new(channel);

    events.push((
        0,
        0,
        TrackEventKind::Meta(MetaMessage::TrackName(name.as_bytes())),
    ));

    if include_metadata {
        let micros_per_beat = (60_000_000.0 / bpm.max(1.0)).round() as u32;
        events.push((
            0,
            0,
            TrackEventKind::Meta(MetaMessage::Tempo(u24::new(micros_per_beat))),
        ));
        let denominator = arrangement
            .meter
            .denominator
            .max(1)
            .next_power_of_two()
            .trailing_zeros() as u8;
        events.push((
            0,
            0,
            TrackEventKind::Meta(MetaMessage::TimeSignature(
                arrangement.meter.numerator.min(255) as u8,
                denominator,
                24,
                8,
            )),
        ));
        let (sharps, minor) = key_signature(&arrangement.key);
        events.push((
            0,
            0,
            TrackEventKind::Meta(MetaMessage::KeySignature(sharps, minor)),
        ));
    }

    events.push((
        0,
        0,
        TrackEventKind::Midi {
            channel,
            message: MidiMessage::ProgramChange {
                program: u7::new(program),
            },
        },
    ));

    for note in notes {
        let start = (note.start_beat * TICKS_PER_DANCE_BEAT as f64).round() as u64;
        let end = (note.end_beat() * TICKS_PER_DANCE_BEAT as f64).round() as u64;
        let end = end.max(start + 1);
        events.push((
            start,
            1,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOn {
                    key: u7::new(note.midi),
                    vel: u7::new(note.velocity),
                },
            },
        ));
        events.push((
            end,
            0,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOff {
                    key: u7::new(note.midi),
                    vel: u7::new(0),
                },
            },
        ));
    }

    events.push((
        (end_beats * TICKS_PER_DANCE_BEAT as f64).round() as u64,
        2,
        TrackEventKind::Meta(MetaMessage::EndOfTrack),
    ));

    events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let mut track = Vec::with_capacity(events.len());
    let mut previous_tick = 0u64;
    for (tick, _, kind) in events {
        let delta = tick
            .saturating_sub(previous_tick)
            .min(u28::max_value().as_int() as u64);
        track.push(TrackEvent {
            delta: u28::new(delta as u32),
            kind,
        });
        previous_tick = tick;
    }
    track
}

fn key_signature(key: &crate::score::Key) -> (i8, bool) {
    let sharps = key.accidentals.iter().filter(|value| **value > 0).count() as i8;
    let flats = key.accidentals.iter().filter(|value| **value < 0).count() as i8;
    let minor = matches!(
        key.mode,
        crate::score::Mode::Minor | crate::score::Mode::Dorian | crate::score::Mode::Phrygian
    );
    ((sharps - flats).clamp(-7, 7), minor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abc::parse_abc;

    const TUNE: &str = "X:1\nT:Test\nM:4/4\nL:1/8\nK:D\n\"D\"A2 A2 A2 A2 | \"G\"B2 B2 B2 B2|\n";

    #[test]
    fn exports_a_parseable_midi_file() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let bytes = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            SecondAccompaniment::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
        );
        assert!(bytes.starts_with(b"MThd"));
        let smf = Smf::parse(&bytes).expect("parse midi");
        assert_eq!(smf.tracks.len(), 2);
        let note_on_count: usize = smf
            .tracks
            .iter()
            .flat_map(|track| track.iter())
            .filter(|event| {
                matches!(
                    event.kind,
                    TrackEventKind::Midi {
                        message: MidiMessage::NoteOn { vel, .. },
                        ..
                    } if vel.as_int() > 0
                )
            })
            .count();
        assert!(note_on_count > 0);
    }

    fn note_velocities(smf: &Smf, track: usize) -> Vec<u8> {
        smf.tracks[track]
            .iter()
            .filter_map(|event| match event.kind {
                TrackEventKind::Midi {
                    message: MidiMessage::NoteOn { vel, .. },
                    ..
                } if vel.as_int() > 0 => Some(vel.as_int()),
                _ => None,
            })
            .collect()
    }

    fn track_end_tick(smf: &Smf, track: usize) -> u64 {
        smf.tracks[track]
            .iter()
            .map(|event| event.delta.as_int() as u64)
            .sum()
    }

    #[test]
    fn melody_carries_more_velocity_than_the_accompaniment() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let bytes = export_midi_with_passes(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            SecondAccompaniment::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            1,
        );
        let smf = Smf::parse(&bytes).expect("parse midi");
        let melody = note_velocities(&smf, 0);
        let accompaniment = note_velocities(&smf, 1);
        assert!(!melody.is_empty() && !accompaniment.is_empty());
        let mean = |values: &[u8]| {
            values.iter().map(|value| *value as f64).sum::<f64>() / values.len() as f64
        };
        assert!(
            mean(&melody) > mean(&accompaniment) + 20.0,
            "melody mean {:.1} should sit well above accompaniment mean {:.1}",
            mean(&melody),
            mean(&accompaniment)
        );
        assert!(melody.iter().filter(|velocity| **velocity >= 92).count() > melody.len() / 2);
    }

    #[test]
    fn variation_passes_extend_the_export() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let plain = export_midi_with_passes(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            SecondAccompaniment::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            1,
        );
        let varied = export_midi_with_passes(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            SecondAccompaniment::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            4,
        );
        let plain = Smf::parse(&plain).expect("parse midi");
        let varied = Smf::parse(&varied).expect("parse midi");
        let pass_ticks = 64 * TICKS_PER_DANCE_BEAT;
        assert_eq!(track_end_tick(&plain, 0), pass_ticks);
        assert_eq!(track_end_tick(&varied, 0), pass_ticks * 4);
        assert_eq!(track_end_tick(&varied, 1), pass_ticks * 4);
        assert!(
            note_velocities(&varied, 0).len() > note_velocities(&plain, 0).len() * 3,
            "the varied export should keep the tune and add ornaments"
        );
    }

    #[test]
    fn export_uses_the_selected_rhythm() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let block = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Block,
            SecondAccompaniment::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
        );
        let sustained = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Sustained,
            SecondAccompaniment::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
        );
        assert_ne!(
            block, sustained,
            "changing the accompaniment rhythm must change the exported notes"
        );
    }

    #[test]
    fn midi_is_deterministic() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let first = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            SecondAccompaniment::Accordion,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
        );
        let second = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            SecondAccompaniment::Accordion,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
        );
        assert_eq!(first, second);
    }

    #[test]
    fn trio_exports_a_second_accompaniment_track() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let trio = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            SecondAccompaniment::Accordion,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
        );
        let smf = Smf::parse(&trio).expect("parse midi");
        assert_eq!(smf.tracks.len(), 3);
        let counter_notes = smf.tracks[2]
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    TrackEventKind::Midi {
                        channel,
                        message: MidiMessage::NoteOn { vel, .. },
                    } if channel.as_int() == 2 && vel.as_int() > 0
                )
            })
            .count();
        assert!(counter_notes > 0, "the second accompaniment must sound");
    }

    #[test]
    fn percussion_exports_on_the_general_midi_drum_channel() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let bytes = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            SecondAccompaniment::Accordion,
            Percussion::Drumset,
            PercussionRhythm::Auto,
            240.0,
        );
        let smf = Smf::parse(&bytes).expect("parse midi");
        assert_eq!(smf.tracks.len(), 4);
        let drum_notes = smf.tracks[3]
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    TrackEventKind::Midi {
                        channel,
                        message: MidiMessage::NoteOn { vel, .. },
                    } if channel.as_int() == 9 && vel.as_int() > 0
                )
            })
            .count();
        assert!(drum_notes > 0, "the percussion track must sound");
    }
}
