//! Optional SoundFont rendering via `rustysynth`.
//!
//! The app bundles a trimmed Fluid R3 GM (MIT) subset and renders with it by
//! default. This module loads whichever bank is active — the bundled subset or
//! one the user supplied — and renders the same note events as the built-in
//! synth using General MIDI programs.
//!
//! Each pitched [`contra_core::BandSlot`] is rendered by its own synthesizer
//! instance with one MIDI channel per player. The slot's pan and player spread
//! become channel pan controllers; the slot's depth is applied after the stereo
//! render, when the pair is placed into the mono, stereo, or surround layout.

use std::io::Cursor;
use std::sync::Arc;

use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

use crate::arrange::EndingNote;
use crate::humanize::{Humanizer, Voice};
use crate::layout;
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
    // Four seconds per chunk keeps thread-spawn overhead low while the extra
    // per-slot buffers stay small (about 1.5 MiB each at 48 kHz stereo).
    let chunk_frames = (request.sample_rate.max(16_000) as usize * 4)
        .next_multiple_of(64)
        .max(64);
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);
    render_soundfont_inner(request, soundfont, chunk_frames, workers)
}

/// The render body, with an explicit chunk size and worker cap so tests can
/// prove that chunking and parallelism do not change the mix.
fn render_soundfont_inner(
    request: &RenderRequest,
    soundfont: &Arc<SoundFont>,
    chunk_frames: usize,
    max_workers: usize,
) -> Result<AudioBuffer, RenderError> {
    let chunk_frames = chunk_frames.next_multiple_of(64).max(64);
    let sample_rate = request.sample_rate.max(16_000);
    let channels = usize::from(request.output.channels());
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
            &request.ending_options,
        )
    });
    let ending_samples = ending
        .as_ref()
        .map(|ending| (ending.seconds * sample_rate as f64).round() as u64)
        .unwrap_or(0);
    let total_samples = count_in_samples + loop_samples + ending_samples;

    let band = crate::band::effective(request);
    let count_in_slots = crate::band::count_in_slots(&band);
    let mut samples = vec![0.0f32; total_samples as usize * channels];

    // Each pitched slot is an independent synthesizer with its own program,
    // channels, and note schedule. Build the slot renderers up front, then
    // render the timeline in chunks with one worker per slot. Chunking keeps
    // the extra buffers small (a few seconds of audio) instead of holding one
    // full 50 MiB copy per slot. Results are summed in slot order, so the mix
    // is deterministic regardless of which worker finishes first.
    let mut renderers = Vec::new();
    for (index, slot) in band.iter().enumerate() {
        if !slot.is_pitched() {
            continue;
        }
        let Some(program) = slot.voice.gm_program() else {
            continue;
        };
        renderers.push(SlotRenderer::new(
            request,
            slot,
            program,
            count_in_slots.contains(&index),
            ending.as_ref(),
            soundfont,
            total_samples,
            samples_per_beat,
            pass_samples,
            passes,
            sample_rate,
        )?);
    }

    let workers = max_workers.min(renderers.len());
    let mut position = 0u64;
    while position < total_samples {
        let frames = ((total_samples - position) as usize).min(chunk_frames);
        let offset = position as usize * channels;
        let end = offset + frames * channels;
        if workers > 1 {
            let chunks = std::thread::scope(|scope| {
                let handles = renderers
                    .iter_mut()
                    .map(|renderer| {
                        scope.spawn(move || {
                            let mut chunk = vec![0.0f32; frames * channels];
                            renderer.render_chunk(frames, &mut chunk);
                            chunk
                        })
                    })
                    .collect::<Vec<_>>();
                handles
                    .into_iter()
                    .map(|handle| {
                        handle.join().map_err(|_| {
                            RenderError::SoundFont("the render worker panicked".to_string())
                        })
                    })
                    .collect::<Result<Vec<_>, RenderError>>()
            })?;
            for chunk in chunks {
                for (out, add) in samples[offset..end].iter_mut().zip(chunk) {
                    *out += add;
                }
            }
        } else {
            for renderer in &mut renderers {
                renderer.render_chunk(frames, &mut samples[offset..end]);
            }
        }
        position += frames as u64;
    }

    if let Some(ending) = &ending {
        crate::render::fade_ending(
            &mut samples,
            channels,
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
        channels: request.output.channels(),
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

/// One slot's persistent synthesizer, schedule, and small block buffers.
///
/// A renderer survives across chunks so the sampled voices keep their state.
/// `render_chunk` advances it by at most `frames` samples and writes the routed
/// output into the caller's slice.
struct SlotRenderer {
    synth: Synthesizer,
    scheduled: Vec<Scheduled>,
    event_index: usize,
    position: u64,
    total_samples: u64,
    left: [f32; 64],
    right: [f32; 64],
    gains: [f32; 5],
    channels: usize,
}

impl SlotRenderer {
    #[allow(clippy::too_many_arguments)]
    fn new(
        request: &RenderRequest,
        slot: &contra_core::BandSlot,
        program: u8,
        count_in: bool,
        ending: Option<&crate::arrange::Ending>,
        soundfont: &Arc<SoundFont>,
        total_samples: u64,
        samples_per_beat: f64,
        pass_samples: u64,
        passes: u32,
        sample_rate: u32,
    ) -> Result<Self, RenderError> {
        let players = slot.player_count();
        let mut settings = SynthesizerSettings::new(sample_rate as i32);
        settings.block_size = 64;
        settings.maximum_polyphony = 128;
        // Reverb and chorus are applied once, after every voice and the
        // percussion layer are mixed, so playback and MIDI export share the
        // same settings.
        settings.enable_reverb_and_chorus = false;
        let mut synth = Synthesizer::new(soundfont, &settings)
            .map_err(|error| RenderError::SoundFont(error.to_string()))?;

        configure_slot(&mut synth, program, slot.volume, slot.pan, players);

        let count_in_samples = (request.count_in_beats.max(0.0) * samples_per_beat).round() as u64;
        let humanizer = Humanizer::new(request);
        let voice = crate::band::humanized_voice(slot.part);
        let mut scheduled = Vec::new();
        if count_in {
            schedule_count_in(
                &mut scheduled,
                request,
                players,
                count_in_samples,
                samples_per_beat,
                sample_rate,
            );
        }
        for pass in 0..passes {
            let notes = crate::band::notes_for_pass(request, slot, pass);
            if notes.is_empty() {
                continue;
            }
            let offset = count_in_samples + pass as u64 * pass_samples;
            if let Some(voice) = voice {
                schedule_notes(
                    &mut scheduled,
                    &notes,
                    players,
                    offset,
                    samples_per_beat,
                    sample_rate,
                    voice,
                    &humanizer,
                );
            }
        }
        if let Some(ending) = ending {
            let notes = crate::band::ending_notes_for_slot(slot, ending);
            if !notes.is_empty() {
                schedule_ending_notes(
                    &mut scheduled,
                    notes,
                    players,
                    count_in_samples + pass_samples * passes as u64,
                    sample_rate,
                );
            }
        }

        scheduled.sort_by(|a, b| a.sample.cmp(&b.sample).then(a.order.cmp(&b.order)));

        let channels = usize::from(request.output.channels());
        let gains = layout::stereo_gains(request.output, slot.depth);
        Ok(Self {
            synth,
            scheduled,
            event_index: 0,
            position: 0,
            total_samples,
            left: [0.0f32; 64],
            right: [0.0f32; 64],
            gains,
            channels,
        })
    }

    /// Render up to `frames` samples into the routed output slice.
    fn render_chunk(&mut self, frames: usize, out: &mut [f32]) {
        let start = self.position;
        let end = (start + frames as u64).min(self.total_samples);
        let mut position = start;
        while position < end {
            let block_end = (position + 64).min(end);
            while self.event_index < self.scheduled.len()
                && self.scheduled[self.event_index].sample < block_end
            {
                match &self.scheduled[self.event_index].kind {
                    ScheduledKind::NoteOn {
                        channel,
                        key,
                        velocity,
                    } => self.synth.note_on(*channel, *key, *velocity),
                    ScheduledKind::NoteOff { channel, key } => self.synth.note_off(*channel, *key),
                }
                self.event_index += 1;
            }
            let length = (block_end - position) as usize;
            self.left[..length].fill(0.0);
            self.right[..length].fill(0.0);
            self.synth
                .render(&mut self.left[..length], &mut self.right[..length]);
            let offset = (position - start) as usize * self.channels;
            let routed = &mut out[offset..offset + length * self.channels];
            route_block(
                routed,
                self.channels,
                0,
                &self.left,
                &self.right,
                length,
                &self.gains,
            );
            position = block_end;
        }
        self.position = position;
    }
}

/// Sum a stereo block into the output layout.
#[allow(clippy::too_many_arguments)]
fn route_block(
    samples: &mut [f32],
    channels: usize,
    position: u64,
    left: &[f32],
    right: &[f32],
    length: usize,
    gains: &[f32],
) {
    for index in 0..length {
        let frame = position as usize + index;
        let (left, right) = (left[index], right[index]);
        for channel in 0..channels {
            let input = if channels == 1 {
                (left + right) * 0.5
            } else {
                layout::stereo_channel_input(channel, left, right)
            };
            samples[frame * channels + channel] += input * gains[channel];
        }
    }
}

/// Set a slot's program, level, and pan on every player channel.
///
/// The level goes through controller 7 rather than a gain applied to samples,
/// so the sampled voice keeps its own velocity response. General MIDI squares
/// the channel volume, so the section plan's linear gain is square-rooted
/// before it enters the controller; without that a section would fall away
/// faster than its power sum. Quantising the controller to an integer leaves
/// at most a fraction of a decibel of error.
fn configure_slot(
    synth: &mut Synthesizer,
    program: u8,
    volume: f32,
    base_pan: f32,
    players: usize,
) {
    for (copy, voice) in crate::copies::voices(base_pan, players, 1)
        .into_iter()
        .enumerate()
    {
        let channel = copy as i32;
        synth.process_midi_message(channel, 0xC0, i32::from(program), 0);
        let level = (volume.clamp(0.0, 1.0) * 110.0 * voice.gain.sqrt()).clamp(0.0, 127.0) as i32;
        synth.process_midi_message(channel, 0xB0, 7, level);
        synth.process_midi_message(channel, 0xB0, 10, crate::copies::pan_cc(voice.pan));
    }
}

/// Schedule one slot's notes once per player, each on its own channel.
///
/// Human timing follows the same per-copy curves as the built-in synth. A
/// note's offset shifts both its note-on and note-off by the same amount, so
/// its duration and the loop's sample count do not change.
#[allow(clippy::too_many_arguments)]
fn schedule_notes(
    scheduled: &mut Vec<Scheduled>,
    notes: &[NoteEvent],
    players: usize,
    offset_samples: u64,
    samples_per_beat: f64,
    sample_rate: u32,
    voice: Voice,
    humanizer: &Humanizer,
) {
    let chord = crate::humanize::chord_members(notes);
    for (copy, player) in crate::copies::voices(0.0, players, sample_rate)
        .into_iter()
        .enumerate()
    {
        let channel = copy as i32;
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

/// Schedule ending notes once per player, which are timed in seconds rather
/// than dance beats.
fn schedule_ending_notes(
    scheduled: &mut Vec<Scheduled>,
    notes: &[EndingNote],
    players: usize,
    offset_samples: u64,
    sample_rate: u32,
) {
    for (copy, voice) in crate::copies::voices(0.0, players, sample_rate)
        .into_iter()
        .enumerate()
    {
        let channel = copy as i32;
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

/// Schedule the four-beat count-in for one slot.
fn schedule_count_in(
    scheduled: &mut Vec<Scheduled>,
    request: &RenderRequest,
    players: usize,
    count_in_samples: u64,
    samples_per_beat: f64,
    sample_rate: u32,
) {
    if count_in_samples == 0 {
        return;
    }
    let chord = request.arrangement.count_in_chord();
    let duration = (0.4 * samples_per_beat).round().max(1.0) as u64;
    let count = request.count_in_beats.round().clamp(0.0, 4.0) as usize;
    for (copy, voice) in crate::copies::voices(0.0, players, sample_rate)
        .into_iter()
        .enumerate()
    {
        let channel = copy as i32;
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
    use contra_core::OutputMode;

    const TEST_FONT: &[u8] = include_bytes!("../../../assets/defaults/fluid-r3-gm-subset.sf2");

    fn request() -> RenderRequest {
        let score = crate::abc::parse_abc(
            "X:1\nT:Chunk Test\nM:4/4\nL:1/8\nK:D\n\"D\"A2 A2 A2 A2 | \"G\"B2 B2 B2 B2|\n",
        )
        .expect("the test tune parses");
        RenderRequest {
            arrangement: crate::arrange::Arrangement::from_score(&score, 64.0),
            bpm: 240.0,
            melody: contra_core::MelodyInstrument::Fiddle,
            accompaniment: contra_core::AccompanimentInstrument::Piano,
            accompaniment_rhythm: contra_core::AccompanimentRhythm::Auto,
            second_accompaniment: contra_core::Countermelody::Accordion,
            band: contra_core::default_band(),
            output: OutputMode::Stereo,
            copies: contra_core::InstrumentCopies::default(),
            variations: contra_core::VariationOptions::default(),
            humanize_options: contra_core::HumanizeOptions::default(),
            ending_options: contra_core::EndingOptions::default(),
            dancer_sounds: false,
            dancer_sound_volume: 0.0,
            dancer_sound_events: Vec::new(),
            percussion: contra_core::Percussion::Off,
            percussion_rhythm: contra_core::PercussionRhythm::Auto,
            melody_volume: 0.75,
            accompaniment_volume: 0.65,
            percussion_volume: 0.5,
            effects: contra_core::AudioEffects::default(),
            count_in_beats: contra_core::COUNT_IN_BEATS,
            sample_rate: 24_000,
            variation_passes: crate::VARIATION_PASSES,
            ending: true,
            humanize_timing: true,
        }
    }

    #[test]
    fn chunking_and_parallel_slots_produce_the_same_mix() {
        let font = load_soundfont(TEST_FONT).expect("the bundled test font loads");
        let request = request();
        // Force different chunk sizes and worker counts. Both must match the
        // sequential render sample for sample: the sum order is fixed and the
        // chunk boundary is aligned to the synth's 64-sample block.
        let sequential =
            render_soundfont_inner(&request, &font, 24_000, 1).expect("sequential render");
        let parallel =
            render_soundfont_inner(&request, &font, 96_000, usize::MAX).expect("parallel render");
        assert_eq!(sequential.samples.len(), parallel.samples.len());
        assert!(
            sequential.samples == parallel.samples,
            "chunked parallel rendering changed the mix"
        );
    }

    #[test]
    fn invalid_soundfont_is_an_error() {
        assert!(load_soundfont(b"not an sf2").is_err());
    }

    #[test]
    fn routing_a_stereo_pair_into_a_surround_frame_keeps_every_channel() {
        let left = [0.5, 0.0];
        let right = [-0.25, 0.0];
        let gains = layout::stereo_gains(OutputMode::Surround, 0.5);
        let mut samples = [0.0f32; 10];
        route_block(&mut samples, 5, 0, &left, &right, 2, &gains);
        assert!(samples[0] != 0.0, "front left");
        assert!(samples[1] != 0.0, "front right");
        assert!(samples[2] != 0.0, "centre");
        assert!(samples[3] != 0.0, "rear left");
        assert!(samples[4] != 0.0, "rear right");
        // The second frame is silence, not a repetition of the first.
        assert_eq!(&samples[5..], &[0.0; 5]);
    }

    #[test]
    fn routing_a_stereo_pair_to_mono_averages_it() {
        let gains = layout::stereo_gains(OutputMode::Mono, 0.0);
        let mut samples = [0.0f32; 1];
        route_block(&mut samples, 1, 0, &[0.8], &[0.2], 1, &gains);
        assert!((samples[0] - 0.5).abs() < 1e-6);
    }
}
