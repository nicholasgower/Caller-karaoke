//! Offline audio rendering.

use std::sync::Arc;

use contra_core::{
    AccompanimentInstrument, AccompanimentRhythm, AudioEffects, BandSlot, Countermelody,
    DancerSoundEvent, EndingOptions, HumanizeOptions, InstrumentCopies, MelodyInstrument,
    OutputMode, Percussion, PercussionRhythm, VariationOptions,
};
use rustysynth::SoundFont;

use crate::arrange::Arrangement;

#[derive(Clone, Debug)]
pub struct RenderRequest {
    pub arrangement: Arrangement,
    pub bpm: f64,
    pub melody: MelodyInstrument,
    pub accompaniment: AccompanimentInstrument,
    pub accompaniment_rhythm: AccompanimentRhythm,
    /// The countermelody voice that completes the trio; [`Countermelody::Off`]
    /// renders a duo.
    pub second_accompaniment: Countermelody,
    /// The instruments the user added to the band. One entry per instrument
    /// group, with its own players, level, and position.
    pub band: Vec<BandSlot>,
    /// Mono, stereo, or surround output.
    pub output: OutputMode,
    /// Players per part. One is the single-player default; extra copies are
    /// rendered as separate players with their own pan and micro-timing. See
    /// [`crate::copies`].
    ///
    /// Kept for callers that still describe the three-role band; [`Self::band`]
    /// is the list the renderer uses.
    pub copies: InstrumentCopies,
    /// Which loop-to-loop variations are allowed.
    pub variations: VariationOptions,
    /// Which human timing effects are allowed.
    pub humanize_options: HumanizeOptions,
    /// Shape of the once-only ending.
    pub ending_options: EndingOptions,
    /// Include stomps and claps from the dancers.
    pub dancer_sounds: bool,
    /// Level of the dancer sound layer.
    pub dancer_sound_volume: f32,
    /// Dancer sounds placed on the dance grid.
    pub dancer_sound_events: Vec<DancerSoundEvent>,
    /// Optional percussion layer; [`Percussion::Off`] renders no percussion.
    pub percussion: Percussion,
    /// Rhythm used to place the percussion layer.
    pub percussion_rhythm: PercussionRhythm,
    pub melody_volume: f32,
    pub accompaniment_volume: f32,
    /// Level of the optional percussion layer.
    pub percussion_volume: f32,
    /// Reverb, chorus, and echo applied after all voices and percussion are
    /// mixed. See [`crate::effects`].
    pub effects: AudioEffects,
    pub count_in_beats: f64,
    pub sample_rate: u32,
    /// Number of 64-beat passes to render back to back.
    ///
    /// One pass is the plain loop. With [`crate::VARIATION_PASSES`] the melody
    /// and the accompaniment both vary on later passes, and the transport
    /// simply loops the whole region.
    pub variation_passes: u32,
    /// Append the once-only slow ending after the loop.
    ///
    /// The transport can splice into it on request; when it is not requested,
    /// the ending region is never played.
    pub ending: bool,
    /// Give every voice a tiny independent tempo drift and spread chord notes
    /// by a few milliseconds; see [`crate::humanize`]. The 64-beat grid, the
    /// count-in, and the loop length are unaffected.
    pub humanize_timing: bool,
}

#[derive(Clone, Debug)]
pub struct AudioBuffer {
    /// Interleaved f32 samples in the channel order of the output layout.
    ///
    /// `channels` is 1 for mono, 2 for stereo, and 5 for surround (front left,
    /// front right, centre, rear left, rear right).
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    /// Sample index where the four-beat count-in ends and the loop begins.
    pub count_in_samples: u64,
    /// Loop bounds for the transport. The count-in only plays once.
    pub loop_start_sample: u64,
    /// End of the last rendered loop pass. The once-only ending, when present,
    /// follows this sample.
    pub loop_end_sample: u64,
    /// Samples in one 64-beat pass. The transport maps sample positions back
    /// into this one-pass range so the dance card, calls, and progress bar do
    /// not see the variation passes.
    pub loop_pass_samples: u64,
    pub samples_per_beat: f64,
    /// Beat in a pass where the slow ending replaces the remaining music.
    ///
    /// This is the start of the tune's last bar, not the end of the pass, so
    /// the ending can stretch that bar instead of repeating it.
    pub ending_from_beat: f64,
    /// First sample of the once-only ending region; `None` when the render has
    /// no ending.
    pub ending_start_sample: Option<u64>,
}

impl AudioBuffer {
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels as usize
    }

    pub fn duration_seconds(&self) -> f64 {
        self.frames() as f64 / self.sample_rate as f64
    }

    /// Musical length of one pass, in dance beats.
    pub fn loop_beats(&self) -> f64 {
        self.loop_pass_samples as f64 / self.samples_per_beat.max(1e-9)
    }

    /// Whether `sample` lies in the once-only ending.
    pub fn in_ending(&self, sample: u64) -> bool {
        self.ending_start_sample
            .is_some_and(|start| sample >= start)
    }

    pub fn beat_at_sample(&self, sample: u64) -> f64 {
        if self.in_ending(sample) {
            return self.loop_beats();
        }
        let offset = sample as i64 - self.count_in_samples as i64;
        if offset < 0 {
            return offset as f64 / self.samples_per_beat;
        }
        let pass_samples = self.loop_pass_samples.max(1);
        (offset as u64 % pass_samples) as f64 / self.samples_per_beat
    }

    pub fn sample_at_beat(&self, beat: f64) -> i64 {
        (self.count_in_samples as f64 + beat * self.samples_per_beat).round() as i64
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("soundfont error: {0}")]
    SoundFont(String),
    #[error("audio rendering produced no samples")]
    Empty,
}

/// Render a request with the built-in synth, or with a supplied soundfont.
pub fn render(
    request: &RenderRequest,
    soundfont: Option<&Arc<SoundFont>>,
) -> Result<AudioBuffer, RenderError> {
    let mut buffer = match soundfont {
        Some(soundfont) => crate::soundfont::render_soundfont(request, soundfont)?,
        None => crate::synth::render_builtin(request),
    };
    crate::percussion::mix(&mut buffer, request);
    crate::effects::apply(
        &mut buffer.samples,
        buffer.sample_rate,
        buffer.channels,
        request.effects,
    );
    // Dancer sounds go on last, dry: a clap must land on its beat, not swim
    // through the room or echo.
    crate::dancer::mix(&mut buffer, request);
    if buffer.samples.is_empty() {
        return Err(RenderError::Empty);
    }
    Ok(buffer)
}

/// Fade the last part of the once-only ending to silence.
///
/// The fade starts inside the linger, never before it, so the final note is
/// still heard at full length and then decays instead of stopping at a hard
/// buffer edge.
pub(crate) fn fade_ending(
    samples: &mut [f32],
    channels: usize,
    sample_rate: u32,
    ending_start: u64,
    ending_seconds: f64,
    linger_seconds: f64,
) {
    let channels = channels.max(1);
    let frames = samples.len() / channels;
    // The transport jumps into the ending at a note boundary; a few
    // milliseconds of fade-in hide the sample discontinuity.
    let fade_in = ((sample_rate as f64 * 0.004).round() as usize).max(1);
    let start = ending_start as usize;
    for index in 0..fade_in {
        let frame = start + index;
        if frame >= frames {
            break;
        }
        let gain = index as f32 / fade_in as f32;
        for channel in 0..channels {
            samples[frame * channels + channel] *= gain;
        }
    }

    let end =
        ((ending_start as f64 + ending_seconds * sample_rate as f64).round() as usize).min(frames);
    let fade = ((linger_seconds * 0.45).clamp(0.3, 2.0) * sample_rate as f64).round() as usize;
    let fade = fade.min(end);
    for index in 0..fade {
        let frame = end - 1 - index;
        // `index` walks backwards from the last sample, so the very last sample
        // gets zero and the start of the fade keeps full level.
        let gain = (index as f32 / fade.max(1) as f32).powf(0.8);
        for channel in 0..channels {
            samples[frame * channels + channel] *= gain;
        }
    }
}
