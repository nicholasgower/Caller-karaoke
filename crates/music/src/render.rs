//! Offline audio rendering.

use std::sync::Arc;

use contra_core::{AccompanimentInstrument, MelodyInstrument};
use rustysynth::SoundFont;

use crate::arrange::Arrangement;

#[derive(Clone, Debug)]
pub struct RenderRequest {
    pub arrangement: Arrangement,
    pub bpm: f64,
    pub melody: MelodyInstrument,
    pub accompaniment: AccompanimentInstrument,
    pub melody_volume: f32,
    pub accompaniment_volume: f32,
    pub count_in_beats: f64,
    pub sample_rate: u32,
}

#[derive(Clone, Debug)]
pub struct AudioBuffer {
    /// Interleaved stereo f32 samples.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    /// Sample index where the three-beat count-in ends and the loop begins.
    pub count_in_samples: u64,
    /// Loop bounds for the transport. The count-in only plays once.
    pub loop_start_sample: u64,
    pub loop_end_sample: u64,
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
        (sample as f64 - self.count_in_samples as f64) / self.samples_per_beat
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
