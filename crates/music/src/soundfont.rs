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
use crate::copies::Part;
use crate::humanize::{Humanizer, Voice, chord_members};
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
    // Reverb and chorus are applied once, after every voice and the percussion
    // layer are mixed, so playback and MIDI export share the same settings.
    settings.enable_reverb_and_chorus = false;
    let mut synth = Synthesizer::new(soundfont, &settings)
        .map_err(|error| RenderError::SoundFont(error.to_string()))?;

    // One MIDI channel per copy, so every player keeps its own pan, velocity,
    // and note lifetime. Fifteen channels are available besides the drum
    // channel; the three melodic parts use at most twelve of them.
    configure_part(
        &mut synth,
        Part::Melody,
        request.melody.gm_program(),
        request.melody_volume,
        0.0,
        request.copies.melody_count(),
    );
    configure_part(
        &mut synth,
        Part::Accompaniment,
        request.accompaniment.gm_program(),
        request.accompaniment_volume,
        0.0,
        request.copies.accompaniment_count(),
    );
    if let Some(program) = request.second_accompaniment.gm_program() {
        // The old single countermelody channel sat at controller 80, which is
        // pan 0.26; keeping that base makes one copy sound as it always did.
        configure_part(
            &mut synth,
            Part::Countermelody,
            program,
            request.accompaniment_volume * 0.8,
            0.26,
            request.copies.countermelody_count(),
        );
    }

    let mut scheduled = Vec::new();
    let humanizer = Humanizer::new(request);
    for pass in 0..passes {
        let offset = count_in_samples + pass as u64 * pass_samples;
        let melody = request.arrangement.melody_notes_for_pass(pass);
        schedule_part(
            &mut scheduled,
            &melody,
            Part::Melody,
            request.copies.melody_count(),
            offset,
            samples_per_beat,
            sample_rate,
            &humanizer,
        );
        if pass == 0 {
            schedule_part(
                &mut scheduled,
                &request.arrangement.pickup,
                Part::Melody,
                request.copies.melody_count(),
                offset,
                samples_per_beat,
                sample_rate,
                &humanizer,
            );
        }
        let accompaniment = request
            .arrangement
            .accompaniment_notes_for_pass_with_rhythm(
                request.accompaniment,
                request.accompaniment_rhythm,
                pass,
            );
        schedule_part(
            &mut scheduled,
            &accompaniment,
            Part::Accompaniment,
            request.copies.accompaniment_count(),
            offset,
            samples_per_beat,
            sample_rate,
            &humanizer,
        );

        if request.second_accompaniment.instrument().is_some() {
            let countermelody = request.arrangement.countermelody_notes_for_pass(pass);
            schedule_part(
                &mut scheduled,
                &countermelody,
                Part::Countermelody,
                request.copies.countermelody_count(),
                offset,
                samples_per_beat,
                sample_rate,
                &humanizer,
            );
        }
    }
    schedule_count_in(
        &mut scheduled,
        request,
        samples_per_beat,
        count_in_samples,
        sample_rate,
    );
    if let Some(ending) = &ending {
        let offset = count_in_samples + loop_samples;
        schedule_ending_part(
            &mut scheduled,
            &ending.melody,
            Part::Melody,
            request.copies.melody_count(),
            offset,
            sample_rate,
        );
        schedule_ending_part(
            &mut scheduled,
            &ending.accompaniment,
            Part::Accompaniment,
            request.copies.accompaniment_count(),
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

/// Set a part's program, level, and pan on every copy channel.
///
/// The level goes through controller 7 rather than a gain applied to samples,
/// so the sampled voice keeps its own velocity response. General MIDI squares
/// the channel volume, so the section plan's linear gain is square-rooted
/// before it enters the controller; without that a section would fall away
/// faster than its power sum. Quantising the controller to an integer leaves
/// at most a fraction of a decibel of error.
fn configure_part(
    synth: &mut Synthesizer,
    part: Part,
    program: u8,
    volume: f32,
    base_pan: f32,
    copies: usize,
) {
    for (copy, voice) in crate::copies::voices(base_pan, copies, 1)
        .into_iter()
        .enumerate()
    {
        let channel = part.channel(copy);
        synth.process_midi_message(channel, 0xC0, i32::from(program), 0);
        let level = (volume.clamp(0.0, 1.0) * 110.0 * voice.gain.sqrt()).clamp(0.0, 127.0) as i32;
        synth.process_midi_message(channel, 0xB0, 7, level);
        synth.process_midi_message(channel, 0xB0, 10, crate::copies::pan_cc(voice.pan));
    }
}

/// Schedule one part's notes once per copy, each on its own channel.
///
/// Human timing follows the same per-copy curves as the built-in synth. A
/// note's offset shifts both its note-on and note-off by the same amount, so
/// its duration and the loop's sample count do not change.
#[allow(clippy::too_many_arguments)]
fn schedule_part(
    scheduled: &mut Vec<Scheduled>,
    notes: &[NoteEvent],
    part: Part,
    copies: usize,
    offset_samples: u64,
    samples_per_beat: f64,
    sample_rate: u32,
    humanizer: &Humanizer,
) {
    let chord = chord_members(notes);
    let voice = Voice::of(part);
    for (copy, player) in crate::copies::voices(0.0, copies, sample_rate)
        .into_iter()
        .enumerate()
    {
        let channel = part.channel(copy);
        let start_offset = offset_samples + player.delay_samples;
        let timing = humanizer.voice(voice, copy);
        for (index, note) in notes.iter().enumerate() {
            let humanized = timing.as_ref().map_or(0.0, |timing| {
                timing.onset_offset_seconds(
                    note.start_beat,
                    note.duration_beats,
                    note.midi,
                    chord.get(index).copied().unwrap_or(false),
                )
            });
            let start = start_offset as i64
                + (note.start_beat * samples_per_beat).round() as i64
                + (humanized * f64::from(sample_rate)).round() as i64;
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
                    velocity: (i32::from(note.velocity) + player.velocity_offset).clamp(1, 127),
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
}

/// Schedule ending notes once per copy, which are timed in seconds rather than
/// dance beats.
fn schedule_ending_part(
    scheduled: &mut Vec<Scheduled>,
    notes: &[EndingNote],
    part: Part,
    copies: usize,
    offset_samples: u64,
    sample_rate: u32,
) {
    for (copy, voice) in crate::copies::voices(0.0, copies, sample_rate)
        .into_iter()
        .enumerate()
    {
        let channel = part.channel(copy);
        let start_offset = offset_samples + voice.delay_samples;
        for note in notes {
            let start =
                start_offset as i64 + (note.start_seconds * sample_rate as f64).round() as i64;
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
                    velocity: (i32::from(note.velocity) + voice.velocity_offset).clamp(1, 127),
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
}

fn schedule_count_in(
    scheduled: &mut Vec<Scheduled>,
    request: &RenderRequest,
    samples_per_beat: f64,
    count_in_samples: u64,
    sample_rate: u32,
) {
    if count_in_samples == 0 {
        return;
    }
    let chord = request.arrangement.count_in_chord();
    let duration = (0.4 * samples_per_beat).round().max(1.0) as u64;
    let count = request.count_in_beats.round().clamp(0.0, 4.0) as usize;
    // The count-in is accompaniment, so every accompaniment copy counts in.
    for (copy, voice) in
        crate::copies::voices(0.0, request.copies.accompaniment_count(), sample_rate)
            .into_iter()
            .enumerate()
    {
        let channel = Part::Accompaniment.channel(copy);
        for beat in 0..count {
            let start = (beat as f64 * samples_per_beat).round() as u64 + voice.delay_samples;
            let mut notes = chord.voicing(55);
            notes.push(chord.bass_midi());
            for midi in notes {
                scheduled.push(Scheduled {
                    sample: start,
                    order: 1,
                    kind: ScheduledKind::NoteOn {
                        channel,
                        key: midi as i32,
                        velocity: (62 + voice.velocity_offset).clamp(1, 127),
                    },
                });
                scheduled.push(Scheduled {
                    sample: start + duration,
                    order: 0,
                    kind: ScheduledKind::NoteOff {
                        channel,
                        key: midi as i32,
                    },
                });
            }
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
