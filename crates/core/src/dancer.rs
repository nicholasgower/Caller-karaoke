//! Turn a dance card into the stomps and claps the dancers make.
//!
//! The whole layer is **temporarily disabled in 0.1.0**: the derived rhythms do
//! not match the sounds dancers actually make. Public derivation returns no
//! events, so the app and renderer stay wired up. Flip `DANCER_SOUNDS_ENABLED`
//! when the rhythms are corrected; foot stomps keep their own switch because
//! their timing is the known-wrong part. See the open item in
//! `docs/roadmap/open-items.md`.

use crate::dance::Dance;
use crate::prefs::{DancerSound, DancerSoundEvent};

/// The one loop length the sounds are placed inside.
///
/// A standard dance is 64 beats. Longer cards are clipped to the music loop so
/// a sound never lands past the end of the rendered pass.
const LOOP_BEATS: f64 = 64.0;

/// Whether the app emits dancer sounds at all.
///
/// Disabled until the derived rhythms match what dancers actually do. The
/// derivation below is kept intact and unit-tested so re-enabling is a one-line
/// change once the open item in `docs/roadmap/open-items.md` is resolved.
const DANCER_SOUNDS_ENABLED: bool = false;

/// Whether the derivation emits foot stomps.
///
/// Disabled until the stomp rhythm matches the moves dancers actually make.
/// The current guess (one at the figure's start and one on its last beat) is
/// wrong for petronellas and long lines.
const FOOT_STOMPS_ENABLED: bool = false;

/// Relative level of a foot stomp.
const STOMP_GAIN: f32 = 0.9;
/// Relative level of a hand clap.
const CLAP_GAIN: f32 = 0.8;

/// Derive the dancer sounds for a dance card.
///
/// The result is deterministic, sorted by beat, and contains no duplicate
/// beat/sound pair. A figure that does not ask for a sound produces nothing, so
/// a plain dance leaves the band alone.
///
/// The layer is temporarily disabled and this returns no events; see
/// `DANCER_SOUNDS_ENABLED`.
pub fn dancer_sounds(dance: &Dance) -> Vec<DancerSoundEvent> {
    if !DANCER_SOUNDS_ENABLED {
        return Vec::new();
    }
    derive_dancer_sounds(dance)
}

/// The mapping from figure text to sounds, kept for when the layer returns.
fn derive_dancer_sounds(dance: &Dance) -> Vec<DancerSoundEvent> {
    let mut events = Vec::new();
    let mut start = 0.0;
    for figure in &dance.figures {
        let end = (start + figure.beats).min(LOOP_BEATS);
        if end <= start {
            break;
        }
        let text = figure.text.to_lowercase();
        let stomps =
            FOOT_STOMPS_ENABLED && (text.contains("petronella") || text.contains("long line"));
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

    // These tests pin the derivation that returns when the layer is re-enabled.
    // The public entry point is disabled and is covered first.

    #[test]
    fn the_public_derivation_is_temporarily_disabled() {
        let card = dance(vec![
            Figure::new(Some("A1".into()), 8.0, "petronella"),
            Figure::new(None, 8.0, "balance and swing"),
            Figure::new(Some("B1".into()), 8.0, "long lines forward and back"),
        ]);
        assert!(dancer_sounds(&card).is_empty());
    }

    #[test]
    fn foot_stomps_stay_disabled_within_the_derivation() {
        // When `FOOT_STOMPS_ENABLED` is turned back on, these figures should
        // each produce a stomp at the figure's start and on its last beat.
        for text in ["petronella", "long lines forward and back"] {
            let events =
                derive_dancer_sounds(&dance(vec![Figure::new(Some("A1".into()), 8.0, text)]));
            assert!(events.is_empty(), "{text}: {events:?}");
        }
    }

    #[test]
    fn a_balance_produces_claps_on_its_two_beats() {
        let events = derive_dancer_sounds(&dance(vec![Figure::new(
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
        let events = derive_dancer_sounds(&dance(vec![
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
        let first = derive_dancer_sounds(&card);
        let second = derive_dancer_sounds(&card);
        assert_eq!(first, second);
        assert!(first.iter().all(|event| {
            event.beat >= 0.0 && event.beat < 64.0 && event.gain > 0.0 && event.gain <= 1.0
        }));
    }

    #[test]
    fn a_card_longer_than_the_loop_is_clipped() {
        let mut figures = Vec::new();
        for index in 0..12 {
            figures.push(Figure::new(
                Some(format!("A{index}")),
                8.0,
                "balance and swing",
            ));
        }
        let events = derive_dancer_sounds(&dance(figures));
        assert!(!events.is_empty());
        assert!(events.iter().all(|event| event.beat < 64.0));
    }
}
