//! Output layouts for mono, stereo, and surround renders.
//!
//! The renderers write interleaved samples in the channel order of the selected
//! [`OutputMode`]. This module owns the one place that turns a slot's side
//! (`pan`) and distance (`depth`) into per-channel gains, so the built-in synth,
//! the soundfont renderer, and the percussion layer all place an instrument the
//! same way.
//!
//! Surround is a five-channel front/rear image: front left, front right,
//! centre, rear left, rear right. A slot at the front uses the front pair and a
//! little centre; a slot with depth moves to the rear pair with an equal-power
//! crossfade. Stereo keeps the historical constant-power pan law, so the
//! default output is unchanged. Mono sums the two sides and keeps their average
//! level.

use caller_core::OutputMode;

/// The most channels any layout uses.
pub const MAX_CHANNELS: usize = 5;

/// Level of the centre speaker relative to a phantom-centre front pair.
///
/// The centre only carries the centre image and a small share of the signal, so
/// a front-centre instrument is anchored without a three-way level jump.
pub const CENTRE_WEIGHT: f32 = 0.35;

/// Rear pair level relative to the front pair at the same distance.
///
/// A rear stage is a little quieter than the front in a real room.
pub const REAR_TRIM: f32 = 0.90;

/// Channel index names for the surround layout.
pub const FRONT_LEFT: usize = 0;
pub const FRONT_RIGHT: usize = 1;
pub const CENTRE: usize = 2;
pub const REAR_LEFT: usize = 3;
pub const REAR_RIGHT: usize = 4;

/// Constant-power gains for one side of the stereo field.
pub fn pan_gains(pan: f32) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    ((0.5 * (1.0 - pan)).sqrt(), (0.5 * (1.0 + pan)).sqrt())
}

/// Equal-power front/rear split for a depth in `0..=1`.
pub fn depth_gains(depth: f32) -> (f32, f32) {
    let depth = depth.clamp(0.0, 1.0);
    let angle = depth * std::f32::consts::FRAC_PI_2;
    (angle.cos().max(0.0), angle.sin().max(0.0))
}

/// Per-channel gains for a source with its own pan and depth.
///
/// Only the first `mode.channels()` entries are meaningful.
pub fn channel_gains(mode: OutputMode, pan: f32, depth: f32) -> [f32; MAX_CHANNELS] {
    let (left, right) = pan_gains(pan);
    let mut gains = [0.0; MAX_CHANNELS];
    match mode {
        OutputMode::Mono => {
            // The audio callback used to fold a stereo buffer with
            // `(left + right) * 0.5`; doing it here keeps that level and makes
            // the mono render exactly the downmix of the stereo render.
            gains[0] = (left + right) * 0.5;
        }
        OutputMode::Stereo => {
            gains[FRONT_LEFT] = left;
            gains[FRONT_RIGHT] = right;
        }
        OutputMode::Surround => {
            let (front, rear) = depth_gains(depth);
            gains[FRONT_LEFT] = front * left;
            gains[FRONT_RIGHT] = front * right;
            // The centre carries the phantom-centre component, tapering to
            // nothing as the source moves to one side.
            gains[CENTRE] = front * CENTRE_WEIGHT * (1.0 - pan.abs().clamp(0.0, 1.0));
            gains[REAR_LEFT] = rear * REAR_TRIM * left;
            gains[REAR_RIGHT] = rear * REAR_TRIM * right;
        }
    }
    gains
}

/// Per-channel gains for a source that is already a stereo pair.
///
/// The soundfont renderer places each player through General MIDI pan first
/// and produces a stereo pair; this second stage only moves that pair between
/// the front and rear stages. Stereo therefore copies the pair untouched.
pub fn stereo_gains(mode: OutputMode, depth: f32) -> [f32; MAX_CHANNELS] {
    let mut gains = [0.0; MAX_CHANNELS];
    match mode {
        OutputMode::Mono => {
            // The caller of `stereo_gains` supplies the averaged pair as
            // channel zero, so the level is the same downmix as `channel_gains`.
            gains[0] = 1.0;
        }
        OutputMode::Stereo => {
            gains[FRONT_LEFT] = 1.0;
            gains[FRONT_RIGHT] = 1.0;
        }
        OutputMode::Surround => {
            let (front, rear) = depth_gains(depth);
            gains[FRONT_LEFT] = front;
            gains[FRONT_RIGHT] = front;
            gains[CENTRE] = front * CENTRE_WEIGHT;
            gains[REAR_LEFT] = rear * REAR_TRIM;
            gains[REAR_RIGHT] = rear * REAR_TRIM;
        }
    }
    gains
}

/// The input a stereo-rendered pair contributes to one output channel.
///
/// Front left and right take the matching side; the centre and any extra
/// channel take the mono sum, which is what keeps a downmix intelligible.
pub fn stereo_channel_input(channel: usize, left: f32, right: f32) -> f32 {
    match channel {
        FRONT_LEFT | REAR_LEFT => left,
        FRONT_RIGHT | REAR_RIGHT => right,
        _ => (left + right) * 0.5,
    }
}

/// Per-channel gains for the spoken caller, which always sits at the front.
///
/// The caller is not panned with the band: it must stay intelligible whichever
/// instrument the user moved. In stereo and mono it is equal in both channels,
/// as it always was; in surround it is anchored in the centre with a little of
/// each front speaker so it does not collapse on a stereo-downmixed device.
pub fn caller_gains(mode: OutputMode) -> [f32; MAX_CHANNELS] {
    let mut gains = [0.0; MAX_CHANNELS];
    match mode {
        OutputMode::Mono => gains[0] = 1.0,
        OutputMode::Stereo => {
            gains[FRONT_LEFT] = 1.0;
            gains[FRONT_RIGHT] = 1.0;
        }
        OutputMode::Surround => {
            gains[FRONT_LEFT] = 0.5;
            gains[FRONT_RIGHT] = 0.5;
            gains[CENTRE] = 0.8;
        }
    }
    gains
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_keeps_the_constant_power_pan_law() {
        let centre = channel_gains(OutputMode::Stereo, 0.0, 0.0);
        assert!((centre[FRONT_LEFT] - 0.5f32.sqrt()).abs() < 1e-6);
        assert!((centre[FRONT_RIGHT] - 0.5f32.sqrt()).abs() < 1e-6);
        let left = channel_gains(OutputMode::Stereo, -1.0, 0.0);
        assert!((left[FRONT_LEFT] - 1.0).abs() < 1e-6);
        assert!(left[FRONT_RIGHT] < 1e-6);
        // Stereo ignores depth; it is a front/rear layout property.
        assert_eq!(
            channel_gains(OutputMode::Stereo, 0.3, 0.0),
            channel_gains(OutputMode::Stereo, 0.3, 1.0)
        );
    }

    #[test]
    fn mono_is_the_stereo_downmix() {
        for pan in [-1.0, -0.4, 0.0, 0.6, 1.0] {
            let stereo = channel_gains(OutputMode::Stereo, pan, 0.0);
            let mono = channel_gains(OutputMode::Mono, pan, 0.0);
            assert!((mono[0] - (stereo[FRONT_LEFT] + stereo[FRONT_RIGHT]) * 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn surround_moves_a_deep_slot_to_the_rear_pair() {
        let front = channel_gains(OutputMode::Surround, 0.0, 0.0);
        assert!(front[FRONT_LEFT] > front[REAR_LEFT] + 0.5);
        let rear = channel_gains(OutputMode::Surround, 0.0, 1.0);
        assert!(
            rear[REAR_LEFT] > 0.5,
            "a rear slot must reach the rear pair"
        );
        assert!(rear[FRONT_LEFT] < 1e-6, "a hard-rear slot is not in front");
        assert!(rear[CENTRE].abs() < 1e-6);
        // Part-way depth keeps some of both stages.
        let middle = channel_gains(OutputMode::Surround, 0.0, 0.5);
        assert!(middle[FRONT_LEFT] > 0.1 && middle[REAR_LEFT] > 0.1);
    }

    #[test]
    fn every_layout_stays_finite_and_bounded() {
        for mode in OutputMode::ALL {
            for pan in [-1.0, -0.15, 0.0, 0.3, 1.0] {
                for depth in [0.0, 0.25, 1.0] {
                    for gain in channel_gains(mode, pan, depth) {
                        assert!(gain.is_finite());
                        assert!((0.0..=1.0).contains(&gain));
                    }
                    for gain in stereo_gains(mode, depth) {
                        assert!(gain.is_finite());
                        assert!((0.0..=1.0).contains(&gain));
                    }
                }
            }
        }
    }
}
