//! Turn a dance card into the stomps and claps the dancers make.
//!
//! The sounds are derived from the figure text rather than from a fixed
//! pattern, so the band follows the dance: a petronella or a long line gets a
//! foot stomp, and a balance or a clap gets hand claps. Everything is placed on
//! the dance-beat grid, so the music, the calls, and the sounds share one clock.

use crate::dance::Dance;
use crate::prefs::{DancerSound, DancerSoundEvent};

/// The one loop length the sounds are placed inside.
///
/// A standard dance is 64 beats. Longer cards are clipped to the music loop so
/// a sound never lands past the end of the rendered pass.
const LOOP_BEATS: f64 = 64.0;

/// Relative level of a foot stomp.
const STOMP_GAIN: f32 = 0.9;
/// Relative level of a hand clap.
const CLAP_GAIN: f32 = 0.8;

/// Derive the dancer sounds for a dance card.
///
/// The result is deterministic, sorted by beat, and contains no duplicate
/// beat/sound pair. A figure that does not ask for a stomp or a clap produces
/// nothing, so a plain dance leaves the band alone.
pub fn dancer_sounds(dance: &Dance) -> Vec<DancerSoundEvent> {
    let mut events = Vec::new();
    let mut start = 0.0;
    for figure in &dance.figures {
        let end = (start + figure.beats).min(LOOP_BEATS);
        if end <= start {
            break;
        }
        let text = figure.text.to_lowercase();
        let stomps = text.contains("petronella") || text.contains("long line");
        let claps = text.contains("balanc") || text.contains("clap");

        if stomps {
            // A stomp marks the start of the figure and its last beat, which
            // is where the travelling move plants and turns.
            push(&mut events, start, DancerSound::Stomp, STOMP_GAIN);
            push(
                &mut events,
                (end - 1.0).max(start),
                DancerSound::Stomp,
                STOMP_GAIN,
            );
        }
        if claps {
            // A balance is a two-beat move; the clap lands with the first
            // step. A figure that says "clap" gets it on its beat as well.
            push(&mut events, start, DancerSound::Clap, CLAP_GAIN);
            if figure.beats >= 2.0 {
                push(&mut events, start + 1.0, DancerSound::Clap, CLAP_GAIN);
            }
        }
        start += figure.beats;
    }

    events.retain(|event| event.beat >= 0.0 && event.beat < LOOP_BEATS);
    events.sort_by(|a, b| {
        a.beat
            .total_cmp(&b.beat)
            .then(sound_order(a.sound).cmp(&sound_order(b.sound)))
    });
    events.dedup_by(|a, b| a.beat.to_bits() == b.beat.to_bits() && a.sound == b.sound);
    events
}

fn push(events: &mut Vec<DancerSoundEvent>, beat: f64, sound: DancerSound, gain: f32) {
    if beat.is_finite() && (0.0..LOOP_BEATS).contains(&beat) {
        events.push(DancerSoundEvent { beat, sound, gain });
    }
}

fn sound_order(sound: DancerSound) -> u8 {
    match sound {
        DancerSound::Stomp => 0,
        DancerSound::Clap => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dance::{Figure, SourceRef};

    fn dance(figures: Vec<Figure>) -> Dance {
        Dance {
            source: SourceRef::new("test", "1", "https://example.invalid/1"),
            title: "Test".into(),
            choreographer: "Tester".into(),
            formation: "improper".into(),
            preamble: String::new(),
            notes: String::new(),
            figures,
        }
    }

    #[test]
    fn a_petronella_produces_stomps_on_the_grid() {
        let events = dancer_sounds(&dance(vec![Figure::new(
            Some("A1".into()),
            8.0,
            "petronella",
        )]));
        assert_eq!(events.len(), 2, "start and end of the figure: {events:?}");
        assert!(events.iter().all(|event| event.sound == DancerSound::Stomp));
        assert!(
            events
                .iter()
                .all(|event| event.beat >= 0.0 && event.beat < 64.0)
        );
        assert!((events[0].beat - 0.0).abs() < 1e-9);
        assert!((events[1].beat - 7.0).abs() < 1e-9);
    }

    #[test]
    fn long_lines_produce_stomps() {
        let events = dancer_sounds(&dance(vec![Figure::new(
            Some("B1".into()),
            8.0,
            "long lines forward and back",
        )]));
        assert!(!events.is_empty());
        assert!(events.iter().all(|event| event.sound == DancerSound::Stomp));
    }

    #[test]
    fn a_balance_produces_claps_on_its_two_beats() {
        let events = dancer_sounds(&dance(vec![Figure::new(
            Some("A1".into()),
            4.0,
            "balance and swing",
        )]));
        assert_eq!(events.len(), 2, "{events:?}");
        assert!(events.iter().all(|event| event.sound == DancerSound::Clap));
        assert!((events[0].beat - 0.0).abs() < 1e-9);
        assert!((events[1].beat - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_plain_dance_produces_no_sounds() {
        let events = dancer_sounds(&dance(vec![
            Figure::new(Some("A1".into()), 8.0, "neighbors do si do once"),
            Figure::new(None, 8.0, "ladles allemande left 1½"),
        ]));
        assert!(events.is_empty(), "{events:?}");
    }

    #[test]
    fn events_stay_inside_the_loop_and_are_deterministic() {
        let card = dance(vec![
            Figure::new(Some("A1".into()), 8.0, "petronella"),
            Figure::new(None, 8.0, "balance and swing"),
            Figure::new(Some("B1".into()), 8.0, "long lines forward and back"),
        ]);
        let first = dancer_sounds(&card);
        let second = dancer_sounds(&card);
        assert_eq!(first, second);
        assert!(first.iter().all(|event| {
            event.beat >= 0.0 && event.beat < 64.0 && event.gain > 0.0 && event.gain <= 1.0
        }));
    }

    #[test]
    fn a_card_longer_than_the_loop_is_clipped() {
        let mut figures = Vec::new();
        for index in 0..12 {
            figures.push(Figure::new(Some(format!("A{index}")), 8.0, "petronella"));
        }
        let events = dancer_sounds(&dance(figures));
        assert!(!events.is_empty());
        assert!(events.iter().all(|event| event.beat < 64.0));
    }
}
