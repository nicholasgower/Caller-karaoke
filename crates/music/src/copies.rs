//! Copy (section) plans shared by the synth, soundfont, and MIDI renderers.
//!
//! A copy is a separate player, not a volume boost. Each one gets its own pan
//! position, a few milliseconds of constant latency, and a small tuning or
//! velocity difference. Together they thicken a part the way two fiddles on
//! stage do.
//!
//! The first copy is the unchanged single player: unity gain, the part's base
//! pan, no latency, and no detune. A part with one copy therefore renders
//! sample-for-sample as it did before copies existed, and the extra players
//! only appear when the user asks for them.
//!
//! The latency is a fixed offset, not drift: every note of a copy is late by
//! the same few milliseconds, so all copies stay on the same 64-beat grid.

use contra_core::InstrumentCopies;

/// Most copies a melodic part can have.
pub const MAX_COPIES: usize = 4;

/// One part of the virtual band, used to assign MIDI channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Melody,
    Accompaniment,
    Countermelody,
}

impl Part {
    /// Channels reserved for this part's copies, in copy order.
    ///
    /// Channel 9 is the General MIDI drum channel, so the countermelody skips
    /// it. Three parts of four copies use fifteen channels in total, which is
    /// exactly what General MIDI offers without touching the drums.
    pub const fn channels(self) -> [i32; MAX_COPIES] {
        match self {
            Self::Melody => [0, 1, 2, 3],
            Self::Accompaniment => [4, 5, 6, 7],
            Self::Countermelody => [8, 10, 11, 12],
        }
    }

    /// The channel for one copy; `copy` is clamped to the plan.
    pub fn channel(self, copy: usize) -> i32 {
        self.channels()[copy.min(MAX_COPIES - 1)]
    }

    /// How many players this part has in `copies`.
    pub fn count(self, copies: InstrumentCopies) -> usize {
        match self {
            Self::Melody => copies.melody_count(),
            Self::Accompaniment => copies.accompaniment_count(),
            Self::Countermelody => copies.countermelody_count(),
        }
    }
}

/// One player in a part's section.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CopyVoice {
    /// Linear level relative to a single player.
    pub gain: f32,
    /// Final pan position in `-1..=1`, including the part's base pan.
    pub pan: f32,
    /// Constant latency in samples. Never negative, so no note starts early.
    pub delay_samples: u64,
    /// Pitch multiplier for the built-in synth; `1.0` means no detune.
    pub detune: f32,
    /// Velocity change for sampled and percussion notes; `0` for one player.
    pub velocity_offset: i32,
}

/// Per-copy spread: (pan offset, latency in seconds, detune in cents, velocity).
///
/// The first row is deliberately neutral: it is the single player.
const SPREAD: [(f32, f64, f32, i32); MAX_COPIES] = [
    (0.00, 0.000, 0.0, 0),
    (0.30, 0.004, 4.0, -3),
    (-0.30, 0.008, -4.0, 2),
    (0.14, 0.012, 7.0, -2),
];

/// Per-copy level taper before the section is scaled for headroom.
const TAPER: [f32; MAX_COPIES] = [1.0, 0.85, 0.75, 0.65];

/// The players for one part.
///
/// `base_pan` is the pan the part had as a single player. With one copy the
/// result is exactly that player. With more, the copies are spread around it
/// and every level is scaled by `1/sqrt(n)`, which keeps the section's loudness
/// close to the single player instead of doubling it. The final peak limiter
/// still protects the mix.
pub fn voices(base_pan: f32, count: usize, sample_rate: u32) -> Vec<CopyVoice> {
    let count = count.clamp(1, MAX_COPIES);
    let section = if count == 1 {
        1.0
    } else {
        1.0 / (count as f32).sqrt()
    };
    (0..count)
        .map(|index| {
            let (pan_offset, seconds, cents, velocity_offset) = SPREAD[index];
            CopyVoice {
                gain: section * TAPER[index],
                pan: (base_pan + pan_offset).clamp(-1.0, 1.0),
                delay_samples: (seconds * f64::from(sample_rate.max(8_000))).round() as u64,
                detune: 2.0f32.powf(cents / 1200.0),
                velocity_offset,
            }
        })
        .collect()
}

/// General MIDI pan controller value for a pan position in `-1..=1`.
pub fn pan_cc(pan: f32) -> i32 {
    ((pan.clamp(-1.0, 1.0) + 1.0) * 0.5 * 127.0).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 24_000;

    #[test]
    fn one_copy_is_the_unchanged_single_player() {
        let voice = voices(0.30, 1, SAMPLE_RATE);
        assert_eq!(voice.len(), 1);
        assert_eq!(
            voice[0],
            CopyVoice {
                gain: 1.0,
                pan: 0.30,
                delay_samples: 0,
                detune: 1.0,
                velocity_offset: 0,
            }
        );
    }

    #[test]
    fn copies_are_separate_players_not_a_level_boost() {
        let section = voices(0.0, 4, SAMPLE_RATE);
        assert_eq!(section.len(), 4);
        let pans: Vec<f32> = section.iter().map(|voice| voice.pan).collect();
        assert!(
            pans.windows(2).any(|pair| pair[0] != pair[1]),
            "copies must not share one pan position: {pans:?}"
        );
        let delays: Vec<u64> = section.iter().map(|voice| voice.delay_samples).collect();
        assert!(delays.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            section[0].detune, 1.0,
            "the first copy keeps the original tuning"
        );
        assert!(
            section[1..].iter().any(|voice| voice.detune != 1.0),
            "later copies are detuned for a section beat"
        );
        let total: f32 = section.iter().map(|voice| voice.gain).sum();
        assert!(
            total <= 2.0 && total > 1.0,
            "four copies must not quadruple the level, got {total}"
        );
    }

    #[test]
    fn copy_delays_are_a_few_milliseconds_and_never_early() {
        for voice in voices(0.0, 4, SAMPLE_RATE) {
            let seconds = voice.delay_samples as f64 / f64::from(SAMPLE_RATE);
            assert!((0.0..=0.020).contains(&seconds), "{seconds}");
        }
    }

    #[test]
    fn part_channels_are_distinct_and_avoid_the_drum_channel() {
        let mut used = Vec::new();
        for part in [Part::Melody, Part::Accompaniment, Part::Countermelody] {
            for channel in part.channels() {
                assert!((0..16).contains(&channel));
                assert_ne!(channel, 9, "{part:?} must not take the drum channel");
                assert!(!used.contains(&channel), "channel {channel} is reused");
                used.push(channel);
            }
        }
        assert_eq!(used.len(), 12);
        assert_eq!(Part::Countermelody.channel(3), 12);
        // A copy index past the plan clamps instead of reading out of bounds.
        assert_eq!(Part::Melody.channel(9), 3);
    }

    #[test]
    fn pan_controller_covers_the_full_range() {
        assert_eq!(pan_cc(-1.0), 0);
        assert_eq!(pan_cc(0.0), 64);
        assert_eq!(pan_cc(1.0), 127);
        // The old countermelody channel sat at controller value 80.
        assert_eq!(pan_cc(0.26), 80);
    }
}
