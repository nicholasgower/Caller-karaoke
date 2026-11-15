//! Offline audio rendering.

use std::sync::Arc;

use contra_core::{AccompanimentInstrument, AccompanimentRhythm, MelodyInstrument};
use rustysynth::SoundFont;

use crate::arrange::Arrangement;

#[derive(Clone, Debug)]
pub struct RenderRequest {
    pub arrangement: Arrangement,
    pub bpm: f64,
    pub melody: MelodyInstrument,
    pub accompaniment: AccompanimentInstrument,
    pub accompaniment_rhythm: AccompanimentRhythm,
    pub melody_volume: f32,
    pub accompaniment_volume: f32,
    pub count_in_beats: f64,
    pub sample_rate: u32,
    /// Number of 64-beat accompaniment passes to render back to back.
    ///
    /// One pass is the plain loop. With [`crate::VARIATION_PASSES`] the
    /// melody repeats unchanged while the accompaniment varies on each pass,
    /// and the transport simply loops the whole region.
    pub variation_passes: u32,
}

#[derive(Clone, Debug)]
pub struct AudioBuffer {
    /// Interleaved stereo f32 samples.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    /// Sample index where the four-beat count-in ends and the loop begins.
    pub count_in_samples: u64,
    /// Loop bounds for the transport. The count-in only plays once.
    pub loop_start_sample: u64,
    /// End of the last rendered pass.
    pub loop_end_sample: u64,
    /// Samples in one 64-beat pass. The transport maps sample positions back
    /// into this one-pass range so the dance card, calls, and progress bar do
    /// not see the variation passes.
    pub loop_pass_samples: u64,
    pub samples_per_beat: f64,
}

impl AudioBuffer {
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels as usize
    }

    pub fn duration_seconds(&self) -> f64 {
        self.frames() as f64 / self.sample_rate as f64
    }

    pub fn beat_at_sample(&self, sample: u64) -> f64 {
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
    let buffer = match soundfont {
        Some(soundfont) => crate::soundfont::render_soundfont(request, soundfont)?,
        None => crate::synth::render_builtin(request),
    };
    if buffer.samples.is_empty() {
        return Err(RenderError::Empty);
    }
    Ok(buffer)
}
