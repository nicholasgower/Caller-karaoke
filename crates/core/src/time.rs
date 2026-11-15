//! Explicit musical time units.
//!
//! Contra Karaoke has three time domains and they must not be mixed
//! implicitly:
//!
//! * [`Beats`] — dance beats (half a bar; 64 per standard 32-bar tune).
//! * [`Seconds`] — wall-clock time.
//! * [`Samples`] — audio sample frames.
//!
//! The tempo maps dance beats to seconds. At the default 240 BPM one dance
//! beat lasts 0.5 seconds, so a 64-beat dance lasts 32 seconds.

use serde::{Deserialize, Serialize};

/// A number of dance beats. May be negative (count-in / lead-in).
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Beats(pub f64);

impl Beats {
    pub const ZERO: Self = Self(0.0);

    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self(self.0.clamp(min.0, max.0))
    }
}

impl From<f64> for Beats {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<Beats> for f64 {
    fn from(value: Beats) -> Self {
        value.0
    }
}

impl std::ops::Add for Beats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Beats {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Mul<f64> for Beats {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

/// Wall-clock seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Seconds(pub f64);

impl Seconds {
    pub const ZERO: Self = Self(0.0);

    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub fn from_beats(beats: Beats, tempo: Tempo) -> Self {
        Self(beats.0 * tempo.seconds_per_dance_beat().0)
    }

    pub fn to_beats(self, tempo: Tempo) -> Beats {
        Beats(self.0 / tempo.seconds_per_dance_beat().0)
    }

    pub fn to_samples(self, sample_rate: u32) -> Samples {
        let frames = (self.0 * sample_rate as f64).round();
        Samples(frames.max(0.0) as u64)
    }
}

impl From<f64> for Seconds {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<Seconds> for f64 {
    fn from(value: Seconds) -> Self {
        value.0
    }
}

/// An integer number of audio sample frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Samples(pub u64);

impl Samples {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn to_seconds(self, sample_rate: u32) -> Seconds {
        Seconds(self.0 as f64 / sample_rate as f64)
    }

    pub fn to_beats(self, sample_rate: u32, tempo: Tempo) -> Beats {
        self.to_seconds(sample_rate).to_beats(tempo)
    }
}

/// The session tempo.
///
/// `bpm` is the quarter-note pulse for reels, which is how contra musicians
/// usually quote tempo. One dance beat is **two quarter notes**, i.e.
/// `seconds_per_dance_beat = 120 / bpm`. The default of 240 therefore gives a
/// 0.5 s dance beat and a 32 s standard dance.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tempo {
    bpm: f64,
}

impl Tempo {
    pub const DEFAULT_BPM: f64 = 240.0;
    pub const MIN_BPM: f64 = 120.0;
    pub const MAX_BPM: f64 = 320.0;

    pub fn new(bpm: f64) -> Self {
        Self {
            bpm: if bpm.is_finite() {
                bpm.clamp(Self::MIN_BPM, Self::MAX_BPM)
            } else {
                Self::DEFAULT_BPM
            },
        }
    }

    pub fn bpm(self) -> f64 {
        self.bpm
    }

    pub fn seconds_per_dance_beat(self) -> Seconds {
        Seconds(120.0 / self.bpm)
    }

    pub fn seconds_per_beat(self) -> Seconds {
        self.seconds_per_dance_beat()
    }
}

impl Default for Tempo {
    fn default() -> Self {
        Self::new(Self::DEFAULT_BPM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tempo_is_contra_tempo() {
        let tempo = Tempo::default();
        assert_eq!(tempo.bpm(), 240.0);
        assert_eq!(tempo.seconds_per_dance_beat().0, 0.5);
    }

    #[test]
    fn sixty_four_beats_is_thirty_two_seconds() {
        let tempo = Tempo::default();
        assert_eq!(
            Seconds::from_beats(Beats(64.0), tempo).0,
            32.0,
            "a standard contra dance should last 32 seconds at 240 BPM"
        );
    }

    #[test]
    fn jig_bar_and_reel_bar_have_the_same_duration() {
        // Both are two dance beats; the sequencer maps note lengths to beats.
        let tempo = Tempo::default();
        assert_eq!(Seconds::from_beats(Beats(2.0), tempo).0, 1.0);
    }

    #[test]
    fn tempo_is_clamped() {
        assert_eq!(Tempo::new(10.0).bpm(), Tempo::MIN_BPM);
        assert_eq!(Tempo::new(900.0).bpm(), Tempo::MAX_BPM);
        assert_eq!(Tempo::new(f64::NAN).bpm(), Tempo::DEFAULT_BPM);
    }

    #[test]
    fn samples_round_trip() {
        let samples = Samples(48_000);
        assert_eq!(samples.to_seconds(48_000).0, 1.0);
    }
}
