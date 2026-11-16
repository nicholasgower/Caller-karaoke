//! Standard MIDI File export.
//!
//! The exported file is the arranged 64-beat loop (not the count-in), with a
//! melody track, an accompaniment track, and — when the countermelody is on —
//! a countermelody track. A part with more than one copy gets one track per
//! copy, on its own channel, so the section is explicit in a DAW.
//!
//! Copies are musically identical to the single player: the same notes on the
//! same beats. The pan position of each copy is written as controller 10, but
//! the few milliseconds of latency playback uses as a section effect are not,
//! so the file stays grid-exact and deterministic. The percussion layer keeps
//! one track at every copy count because General MIDI has a single drum
//! channel. When variations are enabled the file contains the same passes as
//! playback, so the melody is accented and varied instead of repeated
//! identically.

use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use contra_core::{
    AccompanimentInstrument, AccompanimentRhythm, AudioEffects, BandPart, BandSlot, Countermelody,
    InstrumentCopies, MelodyInstrument, Percussion, PercussionRhythm, VariationOptions,
};

use crate::arrange::Arrangement;
use crate::copies::Part;
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
    second_accompaniment: Countermelody,
    percussion: Percussion,
    percussion_rhythm: PercussionRhythm,
    bpm: f64,
    effects: AudioEffects,
    copies: InstrumentCopies,
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
        effects,
        copies,
    )
}

/// Build a Standard MIDI File for the arranged loop.
///
/// `variation_passes` is the number of 64-beat passes to write back to back,
/// matching [`crate::RenderRequest::variation_passes`]. One pass is the plain
/// loop; later passes carry the melody and accompaniment variations.
///
/// `effects` becomes the General MIDI effect sends at the start of every
/// channel track; see [`apply_effects`].
///
/// `copies` writes one track per copy of each melodic part, each on its own
/// channel with a pan controller so a section is visible in a DAW. The
/// percussion layer stays one track at every copy count; General MIDI has one
/// drum channel.
///
/// The export is grid-exact. The playback-only human timing (see
/// [`crate::humanize`]) is deliberately not written, so the file lines up with
/// the dance and can be edited on the beat.
#[allow(clippy::too_many_arguments)]
pub fn export_midi_with_passes(
    arrangement: &Arrangement,
    melody: MelodyInstrument,
    accompaniment: AccompanimentInstrument,
    accompaniment_rhythm: AccompanimentRhythm,
    second_accompaniment: Countermelody,
    percussion: Percussion,
    percussion_rhythm: PercussionRhythm,
    bpm: f64,
    variation_passes: u32,
    effects: AudioEffects,
    copies: InstrumentCopies,
) -> Vec<u8> {
    export_midi_with_options(
        arrangement,
        melody,
        accompaniment,
        accompaniment_rhythm,
        second_accompaniment,
        percussion,
        percussion_rhythm,
        bpm,
        variation_passes,
        &VariationOptions::default(),
        effects,
        copies,
    )
}

/// Build a Standard MIDI File with explicit variation choices.
///
/// This mirrors [`crate::RenderRequest::variations`] so an exported file matches
/// what playback renders when individual variation kinds are turned off.
#[allow(clippy::too_many_arguments)]
pub fn export_midi_with_options(
    arrangement: &Arrangement,
    melody: MelodyInstrument,
    accompaniment: AccompanimentInstrument,
    accompaniment_rhythm: AccompanimentRhythm,
    second_accompaniment: Countermelody,
    percussion: Percussion,
    percussion_rhythm: PercussionRhythm,
    bpm: f64,
    variation_passes: u32,
    variations: &VariationOptions,
    effects: AudioEffects,
    copies: InstrumentCopies,
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
                .melody_notes_for_pass_with_options(pass, variations)
                .into_iter()
                .map(|note| shifted(note, offset)),
        );
        accompaniment_notes.extend(
            arrangement
                .accompaniment_notes_for_pass_with_options(
                    accompaniment,
                    accompaniment_rhythm,
                    pass,
                    variations,
                )
                .into_iter()
                .map(|note| shifted(note, offset)),
        );
        if second_accompaniment.instrument().is_some() {
            countermelody_notes.extend(
                arrangement
                    .countermelody_notes_for_pass_with_options(pass, variations)
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

    let mut tracks = Vec::new();
    push_part_tracks(
        &mut tracks,
        Part::Melody,
        melody.gm_program(),
        &MELODY_TRACK_NAMES,
        &melody_notes,
        copies.melody_count(),
        true,
        bpm,
        arrangement,
        end_beats,
    );
    push_part_tracks(
        &mut tracks,
        Part::Accompaniment,
        accompaniment.gm_program(),
        &ACCOMPANIMENT_TRACK_NAMES,
        &accompaniment_notes,
        copies.accompaniment_count(),
        false,
        bpm,
        arrangement,
        end_beats,
    );
    if let Some(instrument) = second_accompaniment.instrument() {
        push_part_tracks(
            &mut tracks,
            Part::Countermelody,
            instrument.gm_program(),
            &COUNTERMELODY_TRACK_NAMES,
            &countermelody_notes,
            copies.countermelody_count(),
            false,
            bpm,
            arrangement,
            end_beats,
        );
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
            None,
            None,
        ));
    }

    let mut smf = Smf {
        header: Header::new(
            Format::Parallel,
            Timing::Metrical(u15::new(TICKS_PER_QUARTER)),
        ),
        tracks,
    };
    apply_effects(&mut smf, effects);
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes)
        .expect("writing an in-memory MIDI file cannot fail");
    bytes
}

/// Build a Standard MIDI File from the user's band list.
///
/// One track and one General MIDI channel per player, in band order, so two
/// pianos or a clarinet countermelody keep their own program, level, and pan.
/// Channel 9 is the drum channel, so fifteen pitched players fit; a larger band
/// writes its first fifteen players and the documentation records the limit.
/// The percussion rows are merged onto the single drum channel.
///
/// The export is grid-exact: the millisecond player spread and the human timing
/// of playback are performance effects and are not written, so the file lines
/// up with the dance.
#[allow(clippy::too_many_arguments)]
pub fn export_midi_with_band(
    arrangement: &Arrangement,
    band: &[BandSlot],
    accompaniment_rhythm: AccompanimentRhythm,
    percussion_rhythm: PercussionRhythm,
    bpm: f64,
    variation_passes: u32,
    variations: &VariationOptions,
    effects: AudioEffects,
) -> Vec<u8> {
    let passes = variation_passes.max(1);
    let end_beats = arrangement.loop_beats * passes as f64;
    let mut channels = (0u8..16).filter(|channel| *channel != 9);
    let mut tracks = Vec::new();
    let mut metadata_used = false;
    let mut percussion_slots = Vec::new();
    let mut percussion_volume = 0.0f32;
    let mut percussion_notes = Vec::new();

    for slot in band.iter().filter(|slot| slot.is_pitched()) {
        let Some(program) = slot.voice.gm_program() else {
            continue;
        };
        let Some(first_channel) = channels.next() else {
            break;
        };
        let names = match slot.part {
            BandPart::Melody => &MELODY_TRACK_NAMES,
            BandPart::Accompaniment => &ACCOMPANIMENT_TRACK_NAMES,
            BandPart::Countermelody => &COUNTERMELODY_TRACK_NAMES,
            BandPart::Percussion => continue,
        };
        let players = slot.player_count();
        let notes = gather_band_notes(arrangement, slot, accompaniment_rhythm, passes, variations);
        let voices = crate::copies::voices(slot.pan, players, 1);
        for (copy, voice) in voices.into_iter().enumerate() {
            let channel = if copy == 0 {
                first_channel
            } else {
                let Some(channel) = channels.next() else {
                    break;
                };
                channel
            };
            let level =
                (slot.volume.clamp(0.0, 1.0) * 110.0 * voice.gain.sqrt()).clamp(0.0, 127.0) as u8;
            tracks.push(build_track(
                channel,
                program,
                copy_track_name(names, players, copy),
                &notes,
                !metadata_used,
                bpm,
                arrangement,
                end_beats,
                Some(crate::copies::pan_cc(voice.pan) as u8),
                Some(level),
            ));
            metadata_used = true;
        }
    }

    for slot in band.iter().filter(|slot| !slot.is_pitched()) {
        if let Some(style) = slot.voice.percussion_style() {
            percussion_volume = percussion_volume.max(slot.volume);
            percussion_slots.push(style);
        }
    }
    if !percussion_slots.is_empty() {
        for pass in 0..passes {
            let offset = pass as f64 * arrangement.loop_beats;
            for style in &percussion_slots {
                percussion_notes.extend(
                    crate::percussion::notes(
                        arrangement.loop_beats,
                        arrangement.meter,
                        *style,
                        percussion_rhythm,
                    )
                    .into_iter()
                    .map(|note| shifted(note, offset)),
                );
            }
        }
        tracks.push(build_track(
            9,
            0,
            "Percussion",
            &percussion_notes,
            false,
            bpm,
            arrangement,
            end_beats,
            None,
            Some((percussion_volume.clamp(0.0, 1.0) * 110.0) as u8),
        ));
    }

    let mut smf = Smf {
        header: Header::new(
            Format::Parallel,
            Timing::Metrical(u15::new(TICKS_PER_QUARTER)),
        ),
        tracks,
    };
    apply_effects(&mut smf, effects);
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes)
        .expect("writing an in-memory MIDI file cannot fail");
    bytes
}

/// The notes a band slot plays across all variation passes.
fn gather_band_notes(
    arrangement: &Arrangement,
    slot: &BandSlot,
    rhythm: AccompanimentRhythm,
    passes: u32,
    variations: &VariationOptions,
) -> Vec<NoteEvent> {
    let mut notes = Vec::new();
    for pass in 0..passes {
        let offset = pass as f64 * arrangement.loop_beats;
        let pass_notes = match slot.part {
            BandPart::Melody => arrangement.melody_notes_for_pass_with_options(pass, variations),
            BandPart::Countermelody => {
                arrangement.countermelody_notes_for_pass_with_options(pass, variations)
            }
            BandPart::Accompaniment => {
                let instrument = slot
                    .voice
                    .accompaniment_instrument()
                    .unwrap_or(AccompanimentInstrument::Piano);
                arrangement
                    .accompaniment_notes_for_pass_with_options(instrument, rhythm, pass, variations)
            }
            BandPart::Percussion => Vec::new(),
        };
        notes.extend(pass_notes.into_iter().map(|note| shifted(note, offset)));
    }
    notes
}

/// Add the selected General MIDI effect sends to an already built MIDI file.
///
/// Every channel track receives controller 91 (reverb send), 93 (chorus send),
/// and 94 (GM2 delay/echo send) at tick zero. The first track also receives a
/// GM2 reverb-type SysEx message when reverb is on, so a GM2 player chooses the
/// requested room; General MIDI level 1 players use the controller send alone.
///
/// This is deliberately a separate pass so a caller can post-process a file it
/// already has, not only the export built by this module.
pub fn apply_effects(smf: &mut Smf<'_>, effects: AudioEffects) {
    // Replace any sends already in the file so a second pass changes the
    // effect instead of stacking a second, older controller behind the new one.
    for track in &mut smf.tracks {
        track.retain(|event| !is_effect_send(event) && !is_gm2_reverb_sysex(event));
    }

    if let Some(index) = effects.reverb.gm2_type_index()
        && let Some(track) = smf.tracks.first_mut()
    {
        let position = track_name_position(track);
        track.insert(
            position,
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::SysEx(&GM2_REVERB_TYPES[index]),
            },
        );
    }

    let sends = effects.sends();
    for track in &mut smf.tracks {
        let Some(channel) = track.iter().find_map(|event| match event.kind {
            TrackEventKind::Midi { channel, .. } => Some(channel),
            _ => None,
        }) else {
            continue;
        };
        let position = track_name_position(track);
        let events: Vec<TrackEvent<'static>> = sends
            .iter()
            .map(|(controller, value)| TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::Controller {
                        controller: u7::new(*controller),
                        value: u7::new(*value),
                    },
                },
            })
            .collect();
        track.splice(position..position, events);
    }
}

/// Keep a leading track name first; put the new events after it.
fn track_name_position(track: &[TrackEvent<'_>]) -> usize {
    usize::from(
        track.first().is_some_and(|event| {
            matches!(event.kind, TrackEventKind::Meta(MetaMessage::TrackName(_)))
        }),
    )
}

/// The three General MIDI effect send controllers this module owns.
const EFFECT_CONTROLLERS: [u8; 3] = [91, 93, 94];

fn is_effect_send(event: &TrackEvent<'_>) -> bool {
    matches!(
        event.kind,
        TrackEventKind::Midi {
            message: MidiMessage::Controller { controller, .. },
            ..
        } if EFFECT_CONTROLLERS.contains(&controller.as_int())
    )
}

fn is_gm2_reverb_sysex(event: &TrackEvent<'_>) -> bool {
    matches!(
        event.kind,
        TrackEventKind::SysEx(data)
            if data.len() == 12
                && data.starts_with(&[0x7E, 0x7F, 0x09, 0x04, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00])
    )
}

/// GM2 global-parameter control for the reverb type.
///
/// `F0 7E 7F 09 04 01 01 01 01 01 00 <type> F7`, without the leading `F0`
/// because the SMF stores the SysEx payload only.
const fn gm2_reverb_type(type_value: u8) -> [u8; 12] {
    [
        0x7E, 0x7F, 0x09, 0x04, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, type_value, 0xF7,
    ]
}

/// Small room, medium room, large room, and large hall, in GM2 type order.
const GM2_REVERB_TYPES: [[u8; 12]; 4] = [
    gm2_reverb_type(0x00),
    gm2_reverb_type(0x01),
    gm2_reverb_type(0x02),
    gm2_reverb_type(0x04),
];

/// Track names for a part's copies.
///
/// Index 0 is the plain single-player name; index 1 onwards is the numbered
/// name for copy 0, 1, ... of a section, so a two-fiddle part reads
/// "Melody 1" and "Melody 2".
const MELODY_TRACK_NAMES: [&str; 5] = ["Melody", "Melody 1", "Melody 2", "Melody 3", "Melody 4"];
const ACCOMPANIMENT_TRACK_NAMES: [&str; 5] = [
    "Accompaniment",
    "Accompaniment 1",
    "Accompaniment 2",
    "Accompaniment 3",
    "Accompaniment 4",
];
const COUNTERMELODY_TRACK_NAMES: [&str; 5] = [
    "Countermelody",
    "Countermelody 1",
    "Countermelody 2",
    "Countermelody 3",
    "Countermelody 4",
];

fn copy_track_name(names: &[&'static str; 5], copies: usize, copy: usize) -> &'static str {
    if copies <= 1 {
        names[0]
    } else {
        names[(copy + 1).min(4)]
    }
}

/// Append one track per copy of a part.
#[allow(clippy::too_many_arguments)]
fn push_part_tracks(
    tracks: &mut Vec<Vec<TrackEvent<'static>>>,
    part: Part,
    program: u8,
    names: &[&'static str; 5],
    notes: &[NoteEvent],
    copies: usize,
    include_metadata: bool,
    bpm: f64,
    arrangement: &Arrangement,
    end_beats: f64,
) {
    for (copy, voice) in crate::copies::voices(0.0, copies, 1)
        .into_iter()
        .enumerate()
    {
        tracks.push(build_track(
            part.channel(copy) as u8,
            program,
            copy_track_name(names, copies, copy),
            notes,
            include_metadata && copy == 0,
            bpm,
            arrangement,
            end_beats,
            (copies > 1).then(|| crate::copies::pan_cc(voice.pan) as u8),
            None,
        ));
    }
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
    pan: Option<u8>,
    volume: Option<u8>,
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

    // A section puts each copy on its own channel with its own pan. A single
    // player writes no pan controller, exactly as before copies existed.
    if let Some(volume) = volume {
        events.push((
            0,
            0,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::Controller {
                    controller: u7::new(7),
                    value: u7::new(volume),
                },
            },
        ));
    }
    if let Some(pan) = pan {
        events.push((
            0,
            0,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::Controller {
                    controller: u7::new(10),
                    value: u7::new(pan),
                },
            },
        ));
    }

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
    use contra_core::BandVoice;

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
            Countermelody::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            AudioEffects::default(),
            InstrumentCopies::default(),
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

    fn track_names<'a>(smf: &'a Smf<'_>) -> Vec<&'a str> {
        smf.tracks
            .iter()
            .filter_map(|track| match track.first().map(|event| &event.kind) {
                Some(TrackEventKind::Meta(MetaMessage::TrackName(name))) => {
                    std::str::from_utf8(name).ok()
                }
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
            Countermelody::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            1,
            AudioEffects::default(),
            InstrumentCopies::default(),
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
    fn variation_options_change_the_exported_notes() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let quiet = VariationOptions {
            chord_stabs: false,
            ..VariationOptions::default()
        };
        let all = export_midi_with_options(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            Countermelody::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            4,
            &VariationOptions::default(),
            AudioEffects::default(),
            InstrumentCopies::default(),
        );
        let without = export_midi_with_options(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            Countermelody::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            4,
            &quiet,
            AudioEffects::default(),
            InstrumentCopies::default(),
        );
        assert_ne!(
            all, without,
            "the export should follow the variation choices"
        );
        let all = Smf::parse(&all).expect("parse midi");
        let without = Smf::parse(&without).expect("parse midi");
        assert!(
            note_velocities(&all, 1).len() > note_velocities(&without, 1).len(),
            "chord stabs add accompaniment notes"
        );
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
            Countermelody::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            1,
            AudioEffects::default(),
            InstrumentCopies::default(),
        );
        let varied = export_midi_with_passes(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            Countermelody::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            4,
            AudioEffects::default(),
            InstrumentCopies::default(),
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
            Countermelody::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            AudioEffects::default(),
            InstrumentCopies::default(),
        );
        let sustained = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Sustained,
            Countermelody::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            AudioEffects::default(),
            InstrumentCopies::default(),
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
            Countermelody::Accordion,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            AudioEffects::default(),
            InstrumentCopies::default(),
        );
        let second = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            Countermelody::Accordion,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            AudioEffects::default(),
            InstrumentCopies::default(),
        );
        assert_eq!(first, second);
    }

    #[test]
    fn trio_exports_a_countermelody_track() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let trio = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            Countermelody::Accordion,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            AudioEffects::default(),
            InstrumentCopies::default(),
        );
        let smf = Smf::parse(&trio).expect("parse midi");
        assert_eq!(smf.tracks.len(), 3);
        let counter_channel = Part::Countermelody.channel(0) as u8;
        let counter_notes = smf.tracks[2]
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    TrackEventKind::Midi {
                        channel,
                        message: MidiMessage::NoteOn { vel, .. },
                    } if channel.as_int() == counter_channel && vel.as_int() > 0
                )
            })
            .count();
        assert!(counter_notes > 0, "the countermelody must sound");
    }

    #[test]
    fn copies_export_one_track_and_channel_per_player() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let copies = InstrumentCopies {
            melody: 2,
            accompaniment: 2,
            countermelody: 2,
            percussion: 2,
        };
        let bytes = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            Countermelody::Accordion,
            Percussion::Drumset,
            PercussionRhythm::Auto,
            240.0,
            AudioEffects::default(),
            copies,
        );
        let smf = Smf::parse(&bytes).expect("parse midi");
        // Two copies of each melodic part plus one drum track: General MIDI has
        // a single drum channel, so percussion never doubles in the file.
        assert_eq!(smf.tracks.len(), 7);
        assert_eq!(
            track_names(&smf),
            vec![
                "Melody 1",
                "Melody 2",
                "Accompaniment 1",
                "Accompaniment 2",
                "Countermelody 1",
                "Countermelody 2",
                "Percussion",
            ]
        );

        // Every copy plays the same notes on the same beats.
        assert_eq!(
            note_velocities(&smf, 0).len(),
            note_velocities(&smf, 1).len()
        );
        assert_eq!(
            note_velocities(&smf, 2).len(),
            note_velocities(&smf, 3).len()
        );
        assert_eq!(
            track_end_tick(&smf, 1),
            64 * TICKS_PER_DANCE_BEAT * u64::from(crate::VARIATION_PASSES)
        );

        // The copies are spread in the stereo field and use distinct channels.
        let pans = controller_values(&smf, 10);
        assert!(
            pans.iter().any(|value| *value != 64),
            "copies need pan spread"
        );
        let mut channels: Vec<u8> = smf
            .tracks
            .iter()
            .filter_map(|track| {
                track.iter().find_map(|event| match event.kind {
                    TrackEventKind::Midi { channel, .. } => Some(channel.as_int()),
                    _ => None,
                })
            })
            .collect();
        channels.sort_unstable();
        channels.dedup();
        assert_eq!(channels, vec![0, 1, 4, 5, 8, 9, 10]);
        assert!(channels.iter().all(|channel| *channel < 16));
    }

    #[test]
    fn one_copy_writes_no_pan_controller() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let bytes = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            Countermelody::Accordion,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            AudioEffects::default(),
            InstrumentCopies::default(),
        );
        let smf = Smf::parse(&bytes).expect("parse midi");
        assert!(controller_values(&smf, 10).is_empty());
        assert_eq!(
            track_names(&smf),
            vec!["Melody", "Accompaniment", "Countermelody"]
        );
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
            Countermelody::Accordion,
            Percussion::Drumset,
            PercussionRhythm::Auto,
            240.0,
            AudioEffects::default(),
            InstrumentCopies::default(),
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

    fn controller_values(smf: &Smf, controller: u8) -> Vec<u8> {
        smf.tracks
            .iter()
            .flat_map(|track| track.iter())
            .filter_map(|event| match event.kind {
                TrackEventKind::Midi {
                    message:
                        MidiMessage::Controller {
                            controller: found,
                            value,
                        },
                    ..
                } if found.as_int() == controller => Some(value.as_int()),
                _ => None,
            })
            .collect()
    }

    fn has_gm2_reverb_type(smf: &Smf, type_value: u8) -> bool {
        smf.tracks
            .iter()
            .flat_map(|track| track.iter())
            .any(|event| match event.kind {
                TrackEventKind::SysEx(data) => {
                    data.len() == 12
                        && data.starts_with(&[
                            0x7E, 0x7F, 0x09, 0x04, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
                        ])
                        && data[10] == type_value
                        && data[11] == 0xF7
                }
                _ => false,
            })
    }

    #[test]
    fn default_export_requests_a_medium_room_reverb() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let bytes = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            Countermelody::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            AudioEffects::default(),
            InstrumentCopies::default(),
        );
        let smf = Smf::parse(&bytes).expect("parse midi");

        assert_eq!(controller_values(&smf, 91), vec![64, 64]);
        assert_eq!(controller_values(&smf, 93), vec![0, 0]);
        assert_eq!(controller_values(&smf, 94), vec![48, 48]);
        assert!(has_gm2_reverb_type(&smf, 0x01));
    }

    #[test]
    fn effect_sends_follow_the_selected_presets() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let effects = AudioEffects {
            reverb: contra_core::ReverbPreset::Off,
            chorus: contra_core::ChorusPreset::Rich,
            echo: contra_core::EchoPreset::Strong,
        };
        let bytes = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            Countermelody::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            effects,
            InstrumentCopies::default(),
        );
        let smf = Smf::parse(&bytes).expect("parse midi");

        assert_eq!(controller_values(&smf, 91), vec![0, 0]);
        assert_eq!(controller_values(&smf, 93), vec![88, 88]);
        assert_eq!(controller_values(&smf, 94), vec![96, 96]);
        assert!(!has_gm2_reverb_type(&smf, 0x01));
    }

    #[test]
    fn effects_can_be_applied_to_an_existing_midi_file() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let bytes = export_midi(
            &arrangement,
            MelodyInstrument::Fiddle,
            AccompanimentInstrument::Piano,
            AccompanimentRhythm::Auto,
            Countermelody::Off,
            Percussion::Off,
            PercussionRhythm::Auto,
            240.0,
            AudioEffects::default(),
            InstrumentCopies::default(),
        );
        let mut smf = Smf::parse(&bytes).expect("parse midi");
        apply_effects(
            &mut smf,
            AudioEffects {
                reverb: contra_core::ReverbPreset::Hall,
                chorus: contra_core::ChorusPreset::Subtle,
                echo: contra_core::EchoPreset::Off,
            },
        );
        let mut changed = Vec::new();
        smf.write_std(&mut changed).expect("write midi");
        let smf = Smf::parse(&changed).expect("parse midi");

        assert_eq!(controller_values(&smf, 91), vec![112, 112]);
        assert_eq!(controller_values(&smf, 93), vec![40, 40]);
        assert_eq!(controller_values(&smf, 94), vec![0, 0]);
        assert!(has_gm2_reverb_type(&smf, 0x04));
        assert!(
            !has_gm2_reverb_type(&smf, 0x01),
            "applying a second pass must replace the previous room"
        );
    }

    #[test]
    fn the_band_list_exports_one_track_per_player() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let band = contra_core::default_band();
        let bytes = export_midi_with_band(
            &arrangement,
            &band,
            AccompanimentRhythm::Auto,
            PercussionRhythm::Auto,
            240.0,
            1,
            &VariationOptions::default(),
            AudioEffects::default(),
        );
        let smf = Smf::parse(&bytes).expect("parse midi");
        // Default band: two melody fiddles, one piano, two countermelody
        // fiddles, and the merged percussion track.
        assert_eq!(smf.tracks.len(), 6);
        let names = track_names(&smf);
        assert!(names.contains(&"Melody 1"));
        assert!(names.contains(&"Melody 2"));
        assert!(names.contains(&"Accompaniment"));
        assert!(names.contains(&"Countermelody 2"));
        assert!(names.contains(&"Percussion"));
        assert!(
            smf.tracks
                .iter()
                .flat_map(|track| track.iter())
                .any(|event| matches!(
                    event.kind,
                    TrackEventKind::Midi {
                        message: MidiMessage::NoteOn { .. },
                        ..
                    }
                ))
        );
        assert!(
            smf.tracks
                .iter()
                .flat_map(|track| track.iter())
                .any(|event| matches!(
                    event.kind,
                    TrackEventKind::Midi {
                        message: MidiMessage::Controller {
                            controller,
                            ..
                        },
                        ..
                    } if controller.as_int() == 7
                )),
            "band slots write their own level"
        );
    }

    #[test]
    fn a_second_piano_exports_its_own_program_and_channel() {
        let score = parse_abc(TUNE).expect("parse");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let band = vec![
            BandSlot {
                part: BandPart::Accompaniment,
                voice: BandVoice::Piano,
                players: 1,
                volume: 0.8,
                pan: -0.4,
                depth: 0.0,
            },
            BandSlot {
                part: BandPart::Accompaniment,
                voice: BandVoice::Piano,
                players: 1,
                volume: 0.3,
                pan: 0.4,
                depth: 0.0,
            },
        ];
        let bytes = export_midi_with_band(
            &arrangement,
            &band,
            AccompanimentRhythm::Auto,
            PercussionRhythm::Auto,
            240.0,
            1,
            &VariationOptions::default(),
            AudioEffects::default(),
        );
        let smf = Smf::parse(&bytes).expect("parse midi");
        // Two independent accompaniment rows, not a copy count: two tracks on
        // two channels, each with the piano program.
        assert_eq!(smf.tracks.len(), 2);
        let channels = smf
            .tracks
            .iter()
            .filter_map(|track| {
                track.iter().find_map(|event| match event.kind {
                    TrackEventKind::Midi { channel, .. } => Some(channel.as_int()),
                    _ => None,
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(channels.len(), 2);
        assert_ne!(channels[0], channels[1], "each piano gets its own channel");
    }
}
