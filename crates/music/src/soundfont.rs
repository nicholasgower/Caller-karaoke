//! Optional SoundFont rendering via `rustysynth`.
//!
//! The app bundles a trimmed Fluid R3 GM (MIT) subset and renders with it by
//! default. This module loads whichever bank is active — the bundled subset or
//! one the user supplied — and renders the same note events as the built-in
//! synth using General MIDI programs.

use std::io::Cursor;
use std::sync::Arc;

use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

use crate::arrange::EndingNote;
use crate::render::{AudioBuffer, RenderError, RenderRequest};
use crate::score::NoteEvent;

/// Load an SF2 file from memory.
pub fn load_soundfont(bytes: &[u8]) -> Result<Arc<SoundFont>, RenderError> {
    let mut cursor = Cursor::new(bytes);
    let soundfont =
        SoundFont::new(&mut cursor).map_err(|error| RenderError::SoundFont(error.to_string()))?;
    Ok(Arc::new(soundfont))
}

struct Scheduled {
    sample: u64,
    order: u8,
    kind: ScheduledKind,
}

enum ScheduledKind {
    NoteOn {
        channel: i32,
        key: i32,
        velocity: i32,
    },
    NoteOff {
        channel: i32,
        key: i32,
    },
}

pub fn render_soundfont(
    request: &RenderRequest,
    soundfont: &Arc<SoundFont>,
) -> Result<AudioBuffer, RenderError> {
    let sample_rate = request.sample_rate.max(16_000);
    let beat_seconds = 120.0 / request.bpm.max(1.0);
    let samples_per_beat = beat_seconds * sample_rate as f64;
    let count_in_samples = (request.count_in_beats.max(0.0) * samples_per_beat).round() as u64;
    let pass_samples = (request.arrangement.loop_beats.max(1.0) * samples_per_beat).round() as u64;
    let passes = request.variation_passes.max(1);
    let loop_samples = pass_samples * passes as u64;
    let ending = request.ending.then(|| {
        request.arrangement.ending(
            request.bpm,
            request.accompaniment,
            request.accompaniment_rhythm,
        )
    });
    let ending_samples = ending
        .as_ref()
        .map(|ending| (ending.seconds * sample_rate as f64).round() as u64)
        .unwrap_or(0);
    let total_samples = count_in_samples + loop_samples + ending_samples;

    let mut settings = SynthesizerSettings::new(sample_rate as i32);
    settings.block_size = 64;
    settings.maximum_polyphony = 128;
    settings.enable_reverb_and_chorus = true;
    let mut synth = Synthesizer::new(soundfont, &settings)
        .map_err(|error| RenderError::SoundFont(error.to_string()))?;

    synth.process_midi_message(0, 0xC0, request.melody.gm_program() as i32, 0);
    synth.process_midi_message(1, 0xC0, request.accompaniment.gm_program() as i32, 0);
    synth.process_midi_message(
        0,
        0xB0,
        7,
        (request.melody_volume.clamp(0.0, 1.0) * 110.0) as i32,
    );
    synth.process_midi_message(
        1,
        0xB0,
        7,
        (request.accompaniment_volume.clamp(0.0, 1.0) * 110.0) as i32,
    );

    let mut scheduled = Vec::new();
    for pass in 0..passes {
        let offset = count_in_samples + pass as u64 * pass_samples;
        schedule_notes(
            &mut scheduled,
            &request.arrangement.melody,
            0,
            offset,
            samples_per_beat,
        );
        if pass == 0 {
            schedule_notes(
                &mut scheduled,
                &request.arrangement.pickup,
                0,
                offset,
                samples_per_beat,
            );
        }
        let accompaniment = request
            .arrangement
            .accompaniment_notes_for_pass_with_rhythm(
                request.accompaniment,
                request.accompaniment_rhythm,
                pass,
            );
        schedule_notes(&mut scheduled, &accompaniment, 1, offset, samples_per_beat);
    }
    schedule_count_in(&mut scheduled, request, samples_per_beat, count_in_samples);
    if let Some(ending) = &ending {
        let offset = count_in_samples + loop_samples;
        schedule_ending_notes(&mut scheduled, &ending.melody, 0, offset, sample_rate);
        schedule_ending_notes(
            &mut scheduled,
            &ending.accompaniment,
            1,
            offset,
            sample_rate,
        );
    }

    scheduled.sort_by(|a, b| a.sample.cmp(&b.sample).then(a.order.cmp(&b.order)));

    let mut samples = vec![0.0f32; total_samples as usize * 2];
    let mut left = [0.0f32; 64];
    let mut right = [0.0f32; 64];
    let mut event_index = 0usize;
    let mut position = 0u64;

    while position < total_samples {
        let block_end = (position + 64).min(total_samples);
        while event_index < scheduled.len() && scheduled[event_index].sample < block_end {
            match &scheduled[event_index].kind {
                ScheduledKind::NoteOn {
                    channel,
                    key,
                    velocity,
                } => synth.note_on(*channel, *key, *velocity),
                ScheduledKind::NoteOff { channel, key } => synth.note_off(*channel, *key),
            }
            event_index += 1;
        }
        let length = (block_end - position) as usize;
        for value in left.iter_mut().take(length) {
            *value = 0.0;
        }
        for value in right.iter_mut().take(length) {
            *value = 0.0;
        }
        synth.render(&mut left[..length], &mut right[..length]);
        for index in 0..length {
            let frame = (position as usize + index) * 2;
            samples[frame] = left[index];
            samples[frame + 1] = right[index];
        }
        position = block_end;
    }

    if let Some(ending) = &ending {
        crate::render::fade_ending(
            &mut samples,
            sample_rate,
            count_in_samples + loop_samples,
            ending.seconds,
            ending.seconds - ending.linger_start_seconds,
        );
    }
    normalize(&mut samples);

    Ok(AudioBuffer {
        samples,
        sample_rate,
        channels: 2,
        count_in_samples,
        loop_start_sample: count_in_samples,
        loop_end_sample: count_in_samples + loop_samples,
        loop_pass_samples: pass_samples,
        samples_per_beat,
        ending_from_beat: ending
            .as_ref()
            .map(|ending| ending.from_beat)
            .unwrap_or(0.0),
        ending_start_sample: ending.is_some().then_some(count_in_samples + loop_samples),
    })
}

fn schedule_notes(
    scheduled: &mut Vec<Scheduled>,
    notes: &[NoteEvent],
    channel: i32,
    offset_samples: u64,
    samples_per_beat: f64,
) {
    for note in notes {
        let start = offset_samples as i64 + (note.start_beat * samples_per_beat).round() as i64;
        if start < 0 {
            continue;
        }
        let start = start as u64;
        let duration = (note.duration_beats * samples_per_beat).round().max(1.0) as u64;
        scheduled.push(Scheduled {
            sample: start,
            order: 1,
            kind: ScheduledKind::NoteOn {
                channel,
                key: note.midi as i32,
                velocity: note.velocity.clamp(1, 127) as i32,
            },
        });
        scheduled.push(Scheduled {
            sample: start + duration,
            order: 0,
            kind: ScheduledKind::NoteOff {
                channel,
                key: note.midi as i32,
            },
        });
    }
}

/// Schedule ending notes, which are timed in seconds rather than dance beats.
fn schedule_ending_notes(
    scheduled: &mut Vec<Scheduled>,
    notes: &[EndingNote],
    channel: i32,
    offset_samples: u64,
    sample_rate: u32,
) {
    for note in notes {
        let start =
            offset_samples as i64 + (note.start_seconds * sample_rate as f64).round() as i64;
        if start < 0 {
            continue;
        }
        let start = start as u64;
        let duration = (note.duration_seconds * sample_rate as f64)
            .round()
            .max(1.0) as u64;
        scheduled.push(Scheduled {
            sample: start,
            order: 1,
            kind: ScheduledKind::NoteOn {
                channel,
                key: note.midi as i32,
                velocity: note.velocity.clamp(1, 127) as i32,
            },
        });
        scheduled.push(Scheduled {
            sample: start + duration,
            order: 0,
            kind: ScheduledKind::NoteOff {
                channel,
                key: note.midi as i32,
            },
        });
    }
}

fn schedule_count_in(
    scheduled: &mut Vec<Scheduled>,
    request: &RenderRequest,
    samples_per_beat: f64,
    count_in_samples: u64,
) {
    if count_in_samples == 0 {
        return;
    }
    let chord = request.arrangement.count_in_chord();
    let duration = (0.4 * samples_per_beat).round().max(1.0) as u64;
    let count = request.count_in_beats.round().clamp(0.0, 4.0) as usize;
    for beat in 0..count {
        let start = (beat as f64 * samples_per_beat).round() as u64;
        let mut notes = chord.voicing(55);
        notes.push(chord.bass_midi());
        for midi in notes {
            scheduled.push(Scheduled {
                sample: start,
                order: 1,
                kind: ScheduledKind::NoteOn {
                    channel: 1,
                    key: midi as i32,
                    velocity: 62,
                },
            });
            scheduled.push(Scheduled {
                sample: start + duration,
                order: 0,
                kind: ScheduledKind::NoteOff {
                    channel: 1,
                    key: midi as i32,
                },
            });
        }
    }
}

fn normalize(samples: &mut [f32]) {
    let peak = samples
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    if peak > 0.95 {
        let scale = 0.95 / peak;
        for sample in samples {
            *sample *= scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_soundfont_is_an_error() {
        assert!(load_soundfont(b"not an sf2").is_err());
    }
}
