//! The band the renderers actually play.
//!
//! [`RenderRequest::band`] is the user's list. Callers that still describe the
//! original three-role band may leave it empty; this module translates the
//! legacy fields into one slot per role so old call sites and tests keep
//! working. The list is always validated and always keeps at least one pitched
//! slot, which is the same invariant [`contra_core::Preferences::band_slots`]
//! enforces for the app.

use contra_core::{BandPart, BandSlot, BandVoice, default_band};

use crate::arrange::Ending;
use crate::humanize::Voice;
use crate::render::RenderRequest;
use crate::score::NoteEvent;

/// The validated band for a request.
pub fn effective(request: &RenderRequest) -> Vec<BandSlot> {
    let mut band = if request.band.is_empty() {
        legacy(request)
    } else {
        request.band.clone()
    };
    if band.is_empty() {
        band = default_band();
    }
    if !band.iter().any(BandSlot::is_pitched) {
        band.insert(0, BandSlot::default());
    }
    for slot in &mut band {
        slot.validate();
    }
    band
}

/// The one-slot-per-role band implied by the legacy request fields.
fn legacy(request: &RenderRequest) -> Vec<BandSlot> {
    let mut band = Vec::new();
    band.push(BandSlot {
        part: BandPart::Melody,
        voice: BandVoice::from_melody(request.melody),
        players: request.copies.melody,
        volume: request.melody_volume,
        pan: -0.15,
        depth: 0.0,
    });
    band.push(BandSlot {
        part: BandPart::Accompaniment,
        voice: BandVoice::from_accompaniment(request.accompaniment),
        players: request.copies.accompaniment,
        volume: request.accompaniment_volume,
        pan: 0.10,
        depth: 0.10,
    });
    if let Some(instrument) = request.second_accompaniment.instrument() {
        band.push(BandSlot {
            part: BandPart::Countermelody,
            voice: BandVoice::from_melody(instrument),
            players: request.copies.countermelody,
            volume: request.accompaniment_volume * 0.8,
            pan: 0.30,
            depth: 0.05,
        });
    }
    if let Some(voice) = BandVoice::from_percussion(request.percussion) {
        band.push(BandSlot {
            part: BandPart::Percussion,
            voice,
            players: request.copies.percussion,
            volume: request.percussion_volume,
            pan: 0.0,
            depth: 0.20,
        });
    }
    band
}

/// The notes a slot plays in one pass.
///
/// `pass` only affects the variation passes and the pickup; the 64-beat grid
/// itself is the arrangement's. Percussion is rendered by
/// [`crate::percussion`], so a percussion slot returns no pitched notes.
pub fn notes_for_pass(request: &RenderRequest, slot: &BandSlot, pass: u32) -> Vec<NoteEvent> {
    match slot.part {
        BandPart::Melody => {
            let mut notes = Vec::new();
            if pass == 0 {
                notes.extend(request.arrangement.pickup.iter().copied());
            }
            notes.extend(
                request
                    .arrangement
                    .melody_notes_for_pass_with_options(pass, &request.variations),
            );
            notes
        }
        BandPart::Countermelody => request
            .arrangement
            .countermelody_notes_for_pass_with_options(pass, &request.variations),
        BandPart::Accompaniment => {
            let instrument = slot
                .voice
                .accompaniment_instrument()
                .unwrap_or(contra_core::AccompanimentInstrument::Piano);
            request
                .arrangement
                .accompaniment_notes_for_pass_with_options(
                    instrument,
                    request.accompaniment_rhythm,
                    pass,
                    &request.variations,
                )
        }
        BandPart::Percussion => Vec::new(),
    }
}

/// The ending notes a slot plays, timed in seconds because the ending leaves
/// the dance-beat grid.
pub fn ending_notes_for_slot<'a>(
    slot: &BandSlot,
    ending: &'a Ending,
) -> &'a [crate::arrange::EndingNote] {
    match slot.part {
        BandPart::Melody => &ending.melody,
        BandPart::Accompaniment => &ending.accompaniment,
        BandPart::Countermelody | BandPart::Percussion => &[],
    }
}

/// The humanized voice for a slot's part.
pub fn humanized_voice(part: BandPart) -> Option<Voice> {
    match part {
        BandPart::Melody => Some(Voice::Melody),
        BandPart::Accompaniment => Some(Voice::Accompaniment),
        BandPart::Countermelody => Some(Voice::Countermelody),
        BandPart::Percussion => None,
    }
}

/// Slots that play the four-beat count-in.
///
/// The count-in is accompaniment-only, as it always was. A band without an
/// accompaniment slot still needs its four potatoes, so it borrows the first
/// pitched slot instead of rendering silence or moving the grid.
pub fn count_in_slots(band: &[BandSlot]) -> Vec<usize> {
    let accompaniment = band
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.part == BandPart::Accompaniment && slot.is_pitched())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if accompaniment.is_empty() {
        band.iter()
            .position(BandSlot::is_pitched)
            .into_iter()
            .collect()
    } else {
        accompaniment
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abc::parse_abc;
    use crate::arrange::Arrangement;

    fn request() -> RenderRequest {
        let score = parse_abc("X:1\nT:T\nM:4/4\nL:1/8\nK:D\n\"D\"A2 A2 A2 A2|\n").expect("parse");
        RenderRequest {
            arrangement: Arrangement::from_score(&score, 64.0),
            bpm: 240.0,
            melody: contra_core::MelodyInstrument::Fiddle,
            accompaniment: contra_core::AccompanimentInstrument::Piano,
            accompaniment_rhythm: contra_core::AccompanimentRhythm::Auto,
            second_accompaniment: contra_core::Countermelody::Accordion,
            band: Vec::new(),
            output: contra_core::OutputMode::Stereo,
            copies: contra_core::InstrumentCopies::default(),
            variations: contra_core::VariationOptions::default(),
            humanize_options: contra_core::HumanizeOptions::default(),
            ending_options: contra_core::EndingOptions::default(),
            dancer_sounds: false,
            dancer_sound_volume: 0.0,
            dancer_sound_events: Vec::new(),
            percussion: contra_core::Percussion::Off,
            percussion_rhythm: contra_core::PercussionRhythm::Auto,
            melody_volume: 0.75,
            accompaniment_volume: 0.65,
            percussion_volume: 0.5,
            effects: contra_core::AudioEffects::default(),
            count_in_beats: 4.0,
            sample_rate: 24_000,
            variation_passes: 1,
            ending: false,
            humanize_timing: true,
        }
    }

    #[test]
    fn an_empty_list_becomes_the_legacy_band() {
        let band = effective(&request());
        assert_eq!(band.len(), 3);
        assert_eq!(band[0].voice, BandVoice::Fiddle);
        assert_eq!(band[1].voice, BandVoice::Piano);
        assert_eq!(band[2].voice, BandVoice::Accordion);
    }

    #[test]
    fn a_percussion_only_list_gains_a_front_instrument() {
        let mut request = request();
        request.band = vec![BandSlot {
            part: BandPart::Percussion,
            voice: BandVoice::FootTaps,
            players: 1,
            volume: 0.5,
            pan: 0.0,
            depth: 0.2,
        }];
        let band = effective(&request);
        assert_eq!(band[0].voice, BandVoice::Fiddle);
        assert!(band.iter().any(|slot| slot.voice == BandVoice::FootTaps));
    }

    #[test]
    fn notes_follow_the_part_not_the_voice() {
        let request = request();
        let melody = BandSlot {
            part: BandPart::Melody,
            voice: BandVoice::Piano,
            ..BandSlot::default()
        };
        let accompaniment = BandSlot {
            part: BandPart::Accompaniment,
            voice: BandVoice::Fiddle,
            ..BandSlot::default()
        };
        let melody_notes = notes_for_pass(&request, &melody, 0);
        let accompaniment_notes = notes_for_pass(&request, &accompaniment, 0);
        assert!(!melody_notes.is_empty());
        assert!(!accompaniment_notes.is_empty());
        assert_ne!(
            melody_notes
                .iter()
                .map(|note| note.midi)
                .collect::<Vec<_>>(),
            accompaniment_notes
                .iter()
                .map(|note| note.midi)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn count_in_prefers_the_accompaniment_and_falls_back_safely() {
        let request = request();
        let mut band = effective(&request);
        assert_eq!(count_in_slots(&band), vec![1], "the piano counts in");

        band.retain(|slot| slot.part != BandPart::Accompaniment);
        assert_eq!(
            count_in_slots(&band),
            vec![0],
            "without an accompaniment slot the first pitched slot counts in"
        );
    }
}
