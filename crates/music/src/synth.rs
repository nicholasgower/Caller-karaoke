//! Small built-in synthesizer.
//!
//! This is not a sampled instrument library. It provides recognisable fiddle,
//! accordion, sax, clarinet, piano, and guitar timbres from procedural
//! oscillators, and acts as the fallback when the bundled Fluid R3 GM subset or
//! a user-supplied SF2 cannot be loaded (see `soundfont.rs`).
//!
//! Every pitched slot in the request's band is rendered separately: the part
//! chooses the notes, the voice chooses the timbre, the player count creates an
//! ensemble, and the slot's volume, pan, and depth place it in the output
//! layout.

use std::f32::consts::TAU;

use contra_core::{AccompanimentInstrument, BandSlot, BandVoice, MelodyInstrument, OutputMode};

use crate::arrange::EndingNote;
use crate::humanize::{Humanizer, Voice, VoiceTiming, chord_members};
use crate::layout;
use crate::render::{AudioBuffer, RenderRequest};
use crate::score::NoteEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Timbre {
    Fiddle,
    Accordion,
    /// Bright single reed: a square wave with the odd harmonics dominant.
    Sax,
    /// Hollow single reed: a darker square wave with no vibrato.
    Clarinet,
    Piano,
    Guitar,
}

pub fn render_builtin(request: &RenderRequest) -> AudioBuffer {
    let sample_rate = request.sample_rate.max(8_000);
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

    let mut samples = vec![0.0f32; total_samples as usize * channels];
    let band = crate::band::effective(request);

    render_count_in(
        &mut samples,
        request,
        &band,
        channels,
        count_in_samples,
        samples_per_beat,
        sample_rate,
    );

    let humanizer = Humanizer::new(request);
    for pass in 0..passes {
        let offset = count_in_samples as i64 + pass as i64 * pass_samples as i64;
        for slot in band.iter().filter(|slot| slot.is_pitched()) {
            let notes = crate::band::notes_for_pass(request, slot, pass);
            if notes.is_empty() {
                continue;
            }
            let Some(voice) = crate::band::humanized_voice(slot.part) else {
                continue;
            };
            render_section(
                &mut samples,
                channels,
                &notes,
                voice_timbre(slot.voice),
                slot.volume,
                offset,
                samples_per_beat,
                sample_rate,
                slot.pan,
                slot.player_count(),
                voice,
                &humanizer,
                request.output,
                slot.depth,
            );
        }
    }

    if let Some(ending) = &ending {
        let offset = (count_in_samples + loop_samples) as i64;
        for slot in band.iter().filter(|slot| slot.is_pitched()) {
            let notes = crate::band::ending_notes_for_slot(slot, ending);
            if notes.is_empty() {
                continue;
            }
            render_ending_section(
                &mut samples,
                channels,
                notes,
                voice_timbre(slot.voice),
                slot.volume,
                offset,
                sample_rate,
                slot.pan,
                slot.player_count(),
                request.output,
                slot.depth,
            );
        }
        apply_ending_fade(
            &mut samples,
            channels,
            sample_rate,
            (count_in_samples + loop_samples) as usize,
            ending.seconds,
            ending.seconds - ending.linger_start_seconds,
        );
    }

    apply_fades(&mut samples, channels, sample_rate);
    normalize(&mut samples);

    AudioBuffer {
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
    }
}

/// Render the four potatoes from the band's accompaniment slots.
///
/// A band without an accompaniment slot borrows its first pitched slot, so the
/// count-in never disappears or moves off the beat.
#[allow(clippy::too_many_arguments)]
fn render_count_in(
    samples: &mut [f32],
    request: &RenderRequest,
    band: &[BandSlot],
    channels: usize,
    count_in_samples: u64,
    samples_per_beat: f64,
    sample_rate: u32,
) {
    if count_in_samples == 0 {
        return;
    }
    let chord = request.arrangement.count_in_chord();
    let chord_notes = chord.voicing(55);
    let duration = (0.40 * samples_per_beat) as usize;
    let count = request.count_in_beats.round().clamp(0.0, 4.0) as usize;
    for index in crate::band::count_in_slots(band) {
        let slot = &band[index];
        let timbre = voice_timbre(slot.voice);
        let voices = crate::copies::voices(slot.pan, slot.player_count(), sample_rate);
        for beat in 0..count {
            let beat_start = (beat as f64 * samples_per_beat).round() as usize;
            for voice in &voices {
                let start = beat_start + voice.delay_samples as usize;
                render_note(
                    samples,
                    channels,
                    start,
                    duration,
                    chord.bass_midi(),
                    62,
                    timbre,
                    sample_rate as f32,
                    slot.volume * 0.8 * voice.gain,
                    request.output,
                    voice.pan,
                    slot.depth,
                    voice.detune,
                );
                for midi in &chord_notes {
                    render_note(
                        samples,
                        channels,
                        start,
                        duration,
                        *midi,
                        55,
                        timbre,
                        sample_rate as f32,
                        slot.volume * 0.6 * voice.gain,
                        request.output,
                        voice.pan,
                        slot.depth,
                        voice.detune,
                    );
                }
            }
        }
    }
}

fn melody_timbre(instrument: MelodyInstrument) -> Timbre {
    match instrument {
        MelodyInstrument::Fiddle => Timbre::Fiddle,
        MelodyInstrument::Accordion => Timbre::Accordion,
        MelodyInstrument::AltoSax | MelodyInstrument::TenorSax => Timbre::Sax,
        MelodyInstrument::Clarinet => Timbre::Clarinet,
    }
}

fn accompaniment_timbre(instrument: AccompanimentInstrument) -> Timbre {
    match instrument {
        AccompanimentInstrument::Piano => Timbre::Piano,
        AccompanimentInstrument::Guitar => Timbre::Guitar,
        AccompanimentInstrument::Accordion => Timbre::Accordion,
    }
}

/// The procedural voice for a band voice.
fn voice_timbre(voice: BandVoice) -> Timbre {
    match voice {
        BandVoice::Fiddle => melody_timbre(MelodyInstrument::Fiddle),
        BandVoice::Accordion => melody_timbre(MelodyInstrument::Accordion),
        BandVoice::AltoSax | BandVoice::TenorSax => melody_timbre(MelodyInstrument::AltoSax),
        BandVoice::Clarinet => melody_timbre(MelodyInstrument::Clarinet),
        BandVoice::Piano => accompaniment_timbre(AccompanimentInstrument::Piano),
        BandVoice::Guitar => accompaniment_timbre(AccompanimentInstrument::Guitar),
        // Percussion is rendered by `crate::percussion`; this is only a
        // defensive fallback for a hand-edited request.
        BandVoice::FootTaps | BandVoice::Drumset => Timbre::Piano,
    }
}

/// Render one part's notes once per player in the slot.
#[allow(clippy::too_many_arguments)]
fn render_section(
    samples: &mut [f32],
    channels: usize,
    notes: &[NoteEvent],
    timbre: Timbre,
    volume: f32,
    offset: i64,
    samples_per_beat: f64,
    sample_rate: u32,
    base_pan: f32,
    players: usize,
    voice: Voice,
    humanizer: &Humanizer,
    output: OutputMode,
    depth: f32,
) {
    let chord = chord_members(notes);
    for (copy, player) in crate::copies::voices(base_pan, players, sample_rate)
        .into_iter()
        .enumerate()
    {
        let timing = humanizer.voice(voice, copy);
        render_notes(
            samples,
            channels,
            notes,
            timbre,
            volume * player.gain,
            offset + player.delay_samples as i64,
            samples_per_beat,
            sample_rate,
            player.pan,
            player.detune,
            timing.as_ref(),
            &chord,
            output,
            depth,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_notes(
    samples: &mut [f32],
    channels: usize,
    notes: &[NoteEvent],
    timbre: Timbre,
    volume: f32,
    offset: i64,
    samples_per_beat: f64,
    sample_rate: u32,
    pan: f32,
    detune: f32,
    timing: Option<&VoiceTiming>,
    chord: &[bool],
    output: OutputMode,
    depth: f32,
) {
    for (index, note) in notes.iter().enumerate() {
        let humanized = timing.map_or(0.0, |timing| {
            timing.onset_offset_seconds(
                note.start_beat,
                note.duration_beats,
                note.midi,
                chord.get(index).copied().unwrap_or(false),
            )
        });
        let start = offset
            + (note.start_beat * samples_per_beat).round() as i64
            + (humanized * f64::from(sample_rate)).round() as i64;
        let duration = (note.duration_beats * samples_per_beat).round().max(1.0) as usize;
        if start < 0 {
            continue;
        }
        render_note(
            samples,
            channels,
            start as usize,
            duration,
            note.midi,
            note.velocity,
            timbre,
            sample_rate as f32,
            volume,
            output,
            pan,
            depth,
            detune,
        );
    }
}

/// Render the ending's notes once per player in the slot, timed in seconds
/// because the ending deliberately leaves the dance-beat grid.
#[allow(clippy::too_many_arguments)]
fn render_ending_section(
    samples: &mut [f32],
    channels: usize,
    notes: &[EndingNote],
    timbre: Timbre,
    volume: f32,
    offset: i64,
    sample_rate: u32,
    base_pan: f32,
    players: usize,
    output: OutputMode,
    depth: f32,
) {
    for voice in crate::copies::voices(base_pan, players, sample_rate) {
        render_ending_notes(
            samples,
            channels,
            notes,
            timbre,
            volume * voice.gain,
            offset + voice.delay_samples as i64,
            sample_rate,
            voice.pan,
            voice.detune,
            output,
            depth,
        );
    }
}

/// Render the ending's notes, which are timed in seconds because the ending
/// deliberately leaves the dance-beat grid.
#[allow(clippy::too_many_arguments)]
fn render_ending_notes(
    samples: &mut [f32],
    channels: usize,
    notes: &[EndingNote],
    timbre: Timbre,
    volume: f32,
    offset: i64,
    sample_rate: u32,
    pan: f32,
    detune: f32,
    output: OutputMode,
    depth: f32,
) {
    for note in notes {
        let start = offset + (note.start_seconds * sample_rate as f64).round() as i64;
        let duration = (note.duration_seconds * sample_rate as f64)
            .round()
            .max(1.0) as usize;
        if start < 0 {
            continue;
        }
        render_note(
            samples,
            channels,
            start as usize,
            duration,
            note.midi,
            note.velocity,
            timbre,
            sample_rate as f32,
            volume,
            output,
            pan,
            depth,
            detune,
        );
    }
}

/// Let the held final chord decay to silence instead of stopping at a buffer
/// edge. The fade never starts before the linger, so the last note still rings.
fn apply_ending_fade(
    samples: &mut [f32],
    channels: usize,
    sample_rate: u32,
    ending_start: usize,
    ending_seconds: f64,
    linger_seconds: f64,
) {
    crate::render::fade_ending(
        samples,
        channels,
        sample_rate,
        ending_start as u64,
        ending_seconds,
        linger_seconds,
    );
}

/// Add one mono note to every output channel with the slot's placement.
#[allow(clippy::too_many_arguments)]
fn render_note(
    out: &mut [f32],
    channels: usize,
    start: usize,
    duration: usize,
    midi: u8,
    velocity: u8,
    timbre: Timbre,
    sample_rate: f32,
    gain: f32,
    output: OutputMode,
    pan: f32,
    depth: f32,
    detune: f32,
) {
    if duration == 0 || start >= out.len() / channels {
        return;
    }
    let velocity = (velocity.max(1) as f32 / 127.0).clamp(0.05, 1.0);
    let gain = gain * velocity;
    let freq = midi_frequency(midi) * detune;
    let gains = layout::channel_gains(output, pan, depth);
    let gains = &gains[..channels];
    let duration = duration.min(out.len() / channels - start);

    match timbre {
        Timbre::Fiddle => render_fiddle(
            out,
            channels,
            start,
            duration,
            freq,
            gain,
            gains,
            sample_rate,
        ),
        Timbre::Accordion => render_accordion(
            out,
            channels,
            start,
            duration,
            freq,
            gain,
            gains,
            sample_rate,
        ),
        Timbre::Sax => render_sax(
            out,
            channels,
            start,
            duration,
            freq,
            gain,
            gains,
            sample_rate,
        ),
        Timbre::Clarinet => render_clarinet(
            out,
            channels,
            start,
            duration,
            freq,
            gain,
            gains,
            sample_rate,
        ),
        Timbre::Piano => render_piano(
            out,
            channels,
            start,
            duration,
            freq,
            gain,
            gains,
            sample_rate,
        ),
        Timbre::Guitar => render_guitar(
            out,
            channels,
            start,
            duration,
            freq,
            gain,
            gains,
            sample_rate,
        ),
    }
}

/// Add a mono sample to an interleaved frame with the channel gains.
fn mix_frame(out: &mut [f32], channels: usize, frame: usize, sample: f32, gains: &[f32]) {
    let base = frame * channels;
    for (channel, gain) in gains.iter().enumerate() {
        out[base + channel] += sample * gain;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_fiddle(
    out: &mut [f32],
    channels: usize,
    start: usize,
    duration: usize,
    freq: f32,
    gain: f32,
    gains: &[f32],
    sample_rate: f32,
) {
    let mut phase = 0.0f32;
    let mut lowpass = 0.0f32;
    for index in 0..duration {
        let t = index as f32 / sample_rate;
        let vibrato = 1.0 + 0.006 * (TAU * 5.2 * t).sin() * (t * 4.0).min(1.0);
        phase += freq * vibrato / sample_rate;
        phase -= phase.floor();
        let mut saw = 2.0 * phase - 1.0;
        // A gentle body resonance and bow noise.
        lowpass += 0.20 * (saw - lowpass);
        saw = lowpass + 0.08 * (TAU * freq * 0.5 * t).sin();
        let envelope = adsr(
            index as f32 / sample_rate,
            duration as f32 / sample_rate,
            0.045,
            0.06,
            0.82,
            0.10,
        );
        let sample = saw * gain * envelope * 0.55;
        mix_frame(out, channels, start + index, sample, gains);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_accordion(
    out: &mut [f32],
    channels: usize,
    start: usize,
    duration: usize,
    freq: f32,
    gain: f32,
    gains: &[f32],
    sample_rate: f32,
) {
    let mut phase_a = 0.0f32;
    let mut phase_b = 0.25f32;
    for index in 0..duration {
        let t = index as f32 / sample_rate;
        phase_a += freq * 0.996 / sample_rate;
        phase_b += freq * 1.004 / sample_rate;
        phase_a -= phase_a.floor();
        phase_b -= phase_b.floor();
        let saw = ((2.0 * phase_a - 1.0) * 0.55) + ((2.0 * phase_b - 1.0) * 0.45);
        let tremolo = 0.85 + 0.15 * (TAU * 5.5 * t).sin();
        let envelope = adsr(
            index as f32 / sample_rate,
            duration as f32 / sample_rate,
            0.02,
            0.04,
            0.85,
            0.08,
        );
        let sample = saw * gain * envelope * tremolo * 0.45;
        mix_frame(out, channels, start + index, sample, gains);
    }
}

/// A bright single reed: a square wave softened by a one-pole lowpass, with a
/// little vibrato and a hint of the octave above for the saxophone's edge.
///
/// The timbre stands in for both sax choices; the alto and tenor differ by
/// register in the arrangement, and the built-in voice is only the fallback
/// when no soundfont is available.
#[allow(clippy::too_many_arguments)]
fn render_sax(
    out: &mut [f32],
    channels: usize,
    start: usize,
    duration: usize,
    freq: f32,
    gain: f32,
    gains: &[f32],
    sample_rate: f32,
) {
    let mut phase = 0.0f32;
    let mut lowpass = 0.0f32;
    for index in 0..duration {
        let t = index as f32 / sample_rate;
        let vibrato = 1.0 + 0.004 * (TAU * 5.0 * t).sin() * (t * 6.0).min(1.0);
        phase += freq * vibrato / sample_rate;
        phase -= phase.floor();
        let square = if phase < 0.5 { 1.0 } else { -1.0 };
        lowpass += 0.30 * (square - lowpass);
        let edge = 0.12 * (TAU * freq * 2.0 * t).sin();
        let envelope = adsr(
            index as f32 / sample_rate,
            duration as f32 / sample_rate,
            0.025,
            0.05,
            0.86,
            0.10,
        );
        let sample = (lowpass + edge) * gain * envelope * 0.34;
        mix_frame(out, channels, start + index, sample, gains);
    }
}

/// A hollow single reed: the odd harmonics of a square wave behind a stronger
/// lowpass, with no vibrato. That is enough to read as a clarinet next to the
/// fiddle's bowed saw and the accordion's detuned reeds.
#[allow(clippy::too_many_arguments)]
fn render_clarinet(
    out: &mut [f32],
    channels: usize,
    start: usize,
    duration: usize,
    freq: f32,
    gain: f32,
    gains: &[f32],
    sample_rate: f32,
) {
    let mut phase = 0.0f32;
    let mut lowpass = 0.0f32;
    for index in 0..duration {
        let t = index as f32 / sample_rate;
        phase += freq / sample_rate;
        phase -= phase.floor();
        let square = if phase < 0.5 { 1.0 } else { -1.0 };
        lowpass += 0.12 * (square - lowpass);
        let body = 0.08 * (TAU * freq * 3.0 * t).sin();
        let envelope = adsr(
            index as f32 / sample_rate,
            duration as f32 / sample_rate,
            0.045,
            0.06,
            0.88,
            0.12,
        );
        let sample = (lowpass + body) * gain * envelope * 0.38;
        mix_frame(out, channels, start + index, sample, gains);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_piano(
    out: &mut [f32],
    channels: usize,
    start: usize,
    duration: usize,
    freq: f32,
    gain: f32,
    gains: &[f32],
    sample_rate: f32,
) {
    let partials = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
    for index in 0..duration {
        let t = index as f32 / sample_rate;
        let attack = (t / 0.004).min(1.0);
        let mut sample = 0.0f32;
        for (partial_index, partial) in partials.iter().enumerate() {
            let n = *partial;
            let amplitude = 1.0 / (1.6 + partial_index as f32).powf(1.25);
            let inharmonic = 1.0 + 0.0004 * n * n;
            let decay = (-t * (2.2 + partial_index as f32 * 0.55)).exp();
            sample += (TAU * freq * n * inharmonic * t).sin() * amplitude * decay;
        }
        let release = ((duration as f32 / sample_rate - t) / 0.08).clamp(0.0, 1.0);
        let sample = sample * gain * attack * release * 0.35;
        mix_frame(out, channels, start + index, sample, gains);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_guitar(
    out: &mut [f32],
    channels: usize,
    start: usize,
    duration: usize,
    freq: f32,
    gain: f32,
    gains: &[f32],
    sample_rate: f32,
) {
    let delay_len = ((sample_rate / freq).round() as usize).clamp(4, 4096);
    let mut delay = vec![0.0f32; delay_len];
    // Deterministic pluck excitation (xorshift).
    let mut state = 0x1234_5678u32 ^ freq.to_bits();
    for value in &mut delay {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        *value = ((state as f32 / u32::MAX as f32) * 2.0 - 1.0) * 0.4;
    }
    let mut index = 0usize;
    for sample_index in 0..duration {
        let current = delay[index];
        let next = (current + delay[(index + 1) % delay_len]) * 0.5 * 0.997;
        delay[index] = next;
        index = (index + 1) % delay_len;
        let envelope = ((duration as f32 / sample_rate - sample_index as f32 / sample_rate) / 0.12)
            .clamp(0.0, 1.0);
        let sample = current * gain * envelope * 0.55;
        mix_frame(out, channels, start + sample_index, sample, gains);
    }
}

fn adsr(t: f32, duration: f32, attack: f32, decay: f32, sustain: f32, release: f32) -> f32 {
    if t < attack {
        t / attack
    } else if t < attack + decay {
        let progress = (t - attack) / decay;
        1.0 + (sustain - 1.0) * progress
    } else if t > duration - release {
        ((duration - t) / release).clamp(0.0, 1.0) * sustain
    } else {
        sustain
    }
}

fn midi_frequency(midi: u8) -> f32 {
    440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0)
}

fn apply_fades(samples: &mut [f32], channels: usize, sample_rate: u32) {
    let frames = samples.len() / channels;
    let fade = ((sample_rate as f32 * 0.004).round() as usize).max(1);
    for index in 0..fade.min(frames) {
        let gain = index as f32 / fade as f32;
        for channel in 0..channels {
            samples[index * channels + channel] *= gain;
            let tail = frames - 1 - index;
            samples[tail * channels + channel] *= gain;
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
    use crate::abc::parse_abc;
    use crate::arrange::Arrangement;
    use contra_core::{BandPart, COUNT_IN_BEATS};

    const TUNE: &str = "X:1\nT:Test\nM:4/4\nL:1/8\nK:D\n\"D\"A2 A2 A2 A2 | \"G\"B2 B2 B2 B2|\n";

    fn request() -> RenderRequest {
        let score = parse_abc(TUNE).expect("parse");
        RenderRequest {
            arrangement: Arrangement::from_score(&score, 64.0),
            bpm: 240.0,
            melody: MelodyInstrument::Fiddle,
            accompaniment: AccompanimentInstrument::Piano,
            accompaniment_rhythm: contra_core::AccompanimentRhythm::Auto,
            second_accompaniment: contra_core::Countermelody::Accordion,
            // Empty means "use the legacy fields"; the band-list tests below
            // fill it explicitly, exactly as the app does.
            band: Vec::new(),
            output: contra_core::OutputMode::Stereo,
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
            count_in_beats: COUNT_IN_BEATS,
            sample_rate: 24_000,
            variation_passes: 1,
            ending: false,
            humanize_timing: true,
        }
    }

    /// RMS of the loop region, which excludes the count-in and the ending.
    fn loop_rms(buffer: &AudioBuffer) -> f64 {
        let channels = buffer.channels as usize;
        let start = buffer.loop_start_sample as usize * channels;
        let end = buffer.loop_end_sample as usize * channels;
        let sum: f64 = buffer.samples[start..end]
            .iter()
            .map(|sample| f64::from(*sample) * f64::from(*sample))
            .sum();
        (sum / (end - start) as f64).sqrt()
    }

    #[test]
    fn renders_count_in_and_loop() {
        let buffer = render_builtin(&request());
        assert_eq!(buffer.channels, 2);
        assert_eq!(buffer.samples.len() % 2, 0);
        assert!(buffer.count_in_samples > 0);
        assert!(buffer.frames() > buffer.count_in_samples as usize);
        assert!(buffer.samples.iter().any(|sample| sample.abs() > 0.001));
        assert!(buffer.samples.iter().all(|sample| sample.is_finite()));
        assert!(buffer.samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn playback_applies_the_selected_effects() {
        let dry = RenderRequest {
            effects: contra_core::AudioEffects {
                reverb: contra_core::ReverbPreset::Off,
                chorus: contra_core::ChorusPreset::Off,
                echo: contra_core::EchoPreset::Off,
            },
            ..request()
        };
        let wet = RenderRequest {
            effects: contra_core::AudioEffects::default(),
            ..request()
        };
        let dry = crate::render::render(&dry, None).expect("render dry");
        let wet = crate::render::render(&wet, None).expect("render wet");

        assert_eq!(dry.frames(), wet.frames());
        let difference = dry
            .samples
            .iter()
            .zip(&wet.samples)
            .fold(0.0f32, |worst, (a, b)| worst.max((a - b).abs()));
        assert!(
            difference > 1e-3,
            "the default medium-room reverb should change playback, not {difference}"
        );
        assert!(wet.samples.iter().all(|sample| sample.is_finite()));
        assert!(wet.samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn buffer_length_matches_tempo() {
        let buffer = render_builtin(&request());
        // 4 count-in beats + 64 loop beats at 240 BPM = 34.0 s.
        let expected = 34.0 * 24_000.0;
        assert!((buffer.frames() as f64 - expected).abs() < 2.0);
    }

    #[test]
    fn variation_passes_extend_the_buffer_but_not_the_beat_clock() {
        let mut request = request();
        request.variation_passes = crate::VARIATION_PASSES;
        let buffer = render_builtin(&request);

        let pass_seconds = 64.0 * 0.5;
        let expected =
            (COUNT_IN_BEATS * 0.5 + pass_seconds * crate::VARIATION_PASSES as f64) * 24_000.0;
        assert!((buffer.frames() as f64 - expected).abs() < 2.0);
        assert_eq!(
            buffer.loop_end_sample - buffer.loop_start_sample,
            buffer.loop_pass_samples * crate::VARIATION_PASSES as u64
        );

        // The transport wraps every pass, so later passes still read as 0..64.
        let beat = buffer.beat_at_sample(buffer.loop_start_sample + buffer.loop_pass_samples + 1);
        assert!((0.0..0.1).contains(&beat), "beat {beat}");
        assert!(buffer.samples.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn the_countermelody_adds_a_third_voice() {
        let mut trio = request();
        let with = render_builtin(&trio);
        trio.second_accompaniment = contra_core::Countermelody::Off;
        let without = render_builtin(&trio);

        assert_eq!(with.frames(), without.frames());
        let difference = with
            .samples
            .iter()
            .zip(&without.samples)
            .fold(0.0f32, |worst, (a, b)| worst.max((a - b).abs()));
        assert!(
            difference > 1e-3,
            "the countermelody should change the mix, not {difference}"
        );
        assert!(with.samples.iter().all(|sample| sample.is_finite()));
        assert!(with.samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn extra_copies_thicken_the_section_without_moving_the_grid() {
        let single = render_builtin(&request());
        let mut section = request();
        section.copies = contra_core::InstrumentCopies {
            melody: 2,
            accompaniment: 3,
            countermelody: 2,
            percussion: 1,
        };
        let thicker = render_builtin(&section);

        assert_eq!(single.frames(), thicker.frames());
        assert_eq!(single.loop_pass_samples, thicker.loop_pass_samples);
        assert_eq!(single.loop_start_sample, thicker.loop_start_sample);
        assert_eq!(single.loop_end_sample, thicker.loop_end_sample);
        let difference = single
            .samples
            .iter()
            .zip(&thicker.samples)
            .fold(0.0f32, |worst, (a, b)| worst.max((a - b).abs()));
        assert!(
            difference > 1e-3,
            "copies must change the rendered section, not {difference}"
        );
        assert!(thicker.samples.iter().all(|sample| sample.is_finite()));
        assert!(thicker.samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn copy_count_keeps_the_rendered_level() {
        for humanize in [false, true] {
            let mut single = request();
            single.humanize_timing = humanize;
            let reference = loop_rms(&render_builtin(&single));

            for count in 2..=contra_core::InstrumentCopies::MAX {
                let mut melody_only = request();
                melody_only.humanize_timing = humanize;
                melody_only.copies.melody = count;

                let mut band = request();
                band.humanize_timing = humanize;
                band.copies = contra_core::InstrumentCopies {
                    melody: count,
                    accompaniment: count,
                    countermelody: count,
                    percussion: count.min(contra_core::InstrumentCopies::MAX_PERCUSSION),
                };

                for (label, request) in [("melody", melody_only), ("band", band)] {
                    let level = loop_rms(&render_builtin(&request));
                    let decibels = 20.0 * (level / reference).log10();
                    // Human timing decorrelates the copies, so their power sum
                    // lands on the single player. Without it, fixed latencies can
                    // still reinforce coherently, and a little over 1 dB of
                    // variation is expected.
                    let tolerance = if humanize { 0.5 } else { 1.5 };
                    assert!(
                        decibels.abs() < tolerance,
                        "{label} with {count} copies (humanize {humanize}) moved the level by \
                         {decibels:.2} dB"
                    );
                }
            }
        }
    }

    #[test]
    fn humanized_renders_are_reproducible_and_keep_the_grid() {
        let mut grid_exact = request();
        grid_exact.humanize_timing = false;
        let plain = render_builtin(&grid_exact);
        let humanized = render_builtin(&request());
        let again = render_builtin(&request());

        assert_eq!(plain.frames(), humanized.frames());
        assert_eq!(plain.loop_pass_samples, humanized.loop_pass_samples);
        assert_eq!(plain.loop_start_sample, humanized.loop_start_sample);
        assert_eq!(plain.loop_end_sample, humanized.loop_end_sample);
        assert_eq!(
            humanized.samples, again.samples,
            "the same request must render bit-identically"
        );
        assert!(humanized.samples.iter().all(|sample| sample.is_finite()));
        assert!(humanized.samples.iter().all(|sample| sample.abs() <= 1.0));

        let difference = plain
            .samples
            .iter()
            .zip(&humanized.samples)
            .fold(0.0f32, |worst, (a, b)| worst.max((a - b).abs()));
        assert!(
            difference > 1e-4,
            "human timing should change the mix, not {difference}"
        );
    }

    #[test]
    fn a_rendered_ending_is_appended_outside_the_loop() {
        let plain = render_builtin(&request());
        let mut with_ending = request();
        with_ending.ending = true;
        let ended = render_builtin(&with_ending);

        assert_eq!(ended.loop_end_sample, plain.loop_end_sample);
        let start = ended.ending_start_sample.expect("ending region");
        assert_eq!(start, ended.loop_end_sample);

        // The gradual ending is a four-beat easing doubled in length plus a
        // four-second hold at 240 BPM, and it is added after the unchanged loop.
        let added = ended.frames() - plain.frames();
        let expected = (8.0 * 24_000.0) as usize;
        assert!(
            added.abs_diff(expected) < 100,
            "added {added} frames, expected about {expected}"
        );
        assert!(
            ended.samples[start as usize * 2..]
                .iter()
                .any(|sample| sample.abs() > 0.001),
            "the ending must contain audio"
        );
        assert!((ended.beat_at_sample(start) - ended.loop_beats()).abs() < 1e-9);
        assert!(ended.in_ending(start));
        assert!(!ended.in_ending(start - 1));

        // The hold decays instead of stopping at a hard buffer edge.
        let tail = &ended.samples[ended.samples.len() - 200..];
        assert!(tail.iter().all(|sample| sample.abs() < 0.02));
        assert!(
            ended.samples.iter().all(|sample| sample.is_finite()),
            "the ending must not produce NaN"
        );
        assert!(
            ended.samples.iter().all(|sample| sample.abs() <= 1.0),
            "the ending must stay inside full scale"
        );
        assert!(
            ended.samples[ended.samples.len() - 2..]
                .iter()
                .all(|sample| sample.abs() < 1e-6),
            "the final frame must reach silence"
        );
    }

    #[test]
    fn all_instruments_render() {
        for melody in MelodyInstrument::ALL {
            for accompaniment in [
                AccompanimentInstrument::Piano,
                AccompanimentInstrument::Guitar,
                AccompanimentInstrument::Accordion,
            ] {
                let mut request = request();
                request.melody = melody;
                request.accompaniment = accompaniment;
                let buffer = render_builtin(&request);
                assert!(buffer.samples.iter().any(|sample| sample.abs() > 0.001));
            }
        }
    }

    /// The winds must not silently fall back to the fiddle's bowed saw or the
    /// accordion's reeds: each one gets its own procedural voice.
    #[test]
    fn winds_have_their_own_fallback_timbre() {
        assert_eq!(melody_timbre(MelodyInstrument::AltoSax), Timbre::Sax);
        assert_eq!(melody_timbre(MelodyInstrument::TenorSax), Timbre::Sax);
        assert_eq!(melody_timbre(MelodyInstrument::Clarinet), Timbre::Clarinet);
        assert_ne!(melody_timbre(MelodyInstrument::Clarinet), Timbre::Sax);

        let render = |melody| {
            let mut request = request();
            request.melody = melody;
            render_builtin(&request)
        };
        let fiddle = render(MelodyInstrument::Fiddle);
        for melody in [
            MelodyInstrument::AltoSax,
            MelodyInstrument::TenorSax,
            MelodyInstrument::Clarinet,
        ] {
            let buffer = render(melody);
            let peak = buffer
                .samples
                .iter()
                .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
            assert!(peak > 0.02, "{melody:?} rendered at peak {peak}");
            assert!(buffer.samples.iter().all(|sample| sample.is_finite()));
            let difference = buffer
                .samples
                .iter()
                .zip(&fiddle.samples)
                .fold(0.0f32, |worst, (a, b)| worst.max((a - b).abs()));
            assert!(
                difference > 1e-3,
                "{melody:?} should not sound like the fiddle ({difference})"
            );
        }
    }

    fn band_request() -> RenderRequest {
        let mut request = request();
        request.band = contra_core::default_band();
        request.humanize_timing = false;
        request
    }

    fn loop_energy(buffer: &AudioBuffer) -> f32 {
        let channels = buffer.channels as usize;
        let start = buffer.loop_start_sample as usize * channels;
        let end = buffer.loop_end_sample as usize * channels;
        buffer.samples[start..end]
            .iter()
            .map(|sample| sample.abs())
            .sum()
    }

    #[test]
    fn the_default_band_is_rendered_from_the_list() {
        let buffer = crate::render::render(&band_request(), None).expect("render band");
        assert_eq!(buffer.channels, 2);
        assert!(loop_energy(&buffer) > 1.0, "the default band must sound");
        assert!(buffer.samples.iter().all(|sample| sample.is_finite()));
        assert!(buffer.samples.iter().all(|sample| sample.abs() <= 1.0));

        // Every part is present: mute one slot at a time and hear a change.
        let reference = loop_rms(&buffer);
        for index in 0..4 {
            let mut request = band_request();
            request.band[index].volume = 0.0;
            let muted = crate::render::render(&request, None).expect("render muted band");
            let decibels = 20.0 * (loop_rms(&muted) / reference).log10();
            assert!(
                decibels < -0.05,
                "muting slot {index} should lower the mix, moved {decibels:.2} dB"
            );
        }
    }

    #[test]
    fn band_renders_are_deterministic() {
        let request = band_request();
        let first = render_builtin(&request);
        let second = render_builtin(&request);
        assert_eq!(first.samples, second.samples);
        assert_eq!(first.loop_pass_samples, second.loop_pass_samples);
        assert!(first.samples.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn a_second_piano_is_an_independent_slot_not_a_copy() {
        let mut request = band_request();
        request.band = vec![
            BandSlot {
                part: BandPart::Accompaniment,
                voice: BandVoice::Piano,
                players: 1,
                volume: 0.8,
                pan: -0.5,
                depth: 0.0,
            },
            BandSlot {
                part: BandPart::Accompaniment,
                voice: BandVoice::Piano,
                players: 1,
                volume: 0.2,
                pan: 0.5,
                depth: 0.0,
            },
            BandSlot {
                part: BandPart::Melody,
                voice: BandVoice::Fiddle,
                players: 1,
                volume: 0.7,
                pan: 0.0,
                depth: 0.0,
            },
        ];
        let two = render_builtin(&request);
        request.band.truncate(2);
        request.band[1].volume = 0.8;
        let equal = render_builtin(&request);
        assert_eq!(two.frames(), equal.frames());
        let difference = two
            .samples
            .iter()
            .zip(&equal.samples)
            .fold(0.0f32, |worst, (a, b)| worst.max((a - b).abs()));
        assert!(
            difference > 1e-3,
            "the second piano must keep its own level and side"
        );
    }

    #[test]
    fn mono_is_the_stereo_downmix_and_surround_adds_channels() {
        let mut stereo_request = band_request();
        let stereo = render_builtin(&stereo_request);
        let mut mono_request = band_request();
        mono_request.output = OutputMode::Mono;
        let mono = render_builtin(&mono_request);
        assert_eq!(mono.channels, 1);
        assert_eq!(stereo.channels, 2);
        assert_eq!(mono.frames(), stereo.frames());
        // With dry effects the mono render is the sample-for-sample average of
        // the stereo render.
        mono_request.effects = contra_core::AudioEffects {
            reverb: contra_core::ReverbPreset::Off,
            chorus: contra_core::ChorusPreset::Off,
            echo: contra_core::EchoPreset::Off,
        };
        stereo_request.effects = mono_request.effects;
        let mono = render_builtin(&mono_request);
        let stereo = render_builtin(&stereo_request);
        let worst = mono
            .samples
            .iter()
            .enumerate()
            .map(|(frame, sample)| {
                let left = stereo.samples[frame * 2];
                let right = stereo.samples[frame * 2 + 1];
                (sample - (left + right) * 0.5).abs()
            })
            .fold(0.0f32, f32::max);
        assert!(worst < 1e-5, "mono downmix differs by {worst}");

        let mut surround_request = band_request();
        surround_request.output = OutputMode::Surround;
        let surround = render_builtin(&surround_request);
        assert_eq!(surround.channels, 5);
        assert_eq!(surround.frames(), stereo.frames());
        assert!(surround.samples.iter().all(|sample| sample.is_finite()));
        assert!(surround.samples.iter().all(|sample| sample.abs() <= 1.0));
        assert!(
            surround
                .samples
                .iter()
                .skip(3)
                .step_by(5)
                .any(|sample| sample.abs() > 1e-4),
            "the rear pair must carry the default band"
        );
    }

    #[test]
    fn depth_changes_the_surround_rear_channels() {
        let mut front = band_request();
        front.output = OutputMode::Surround;
        for slot in &mut front.band {
            slot.depth = 0.0;
        }
        let mut rear = front.clone();
        rear.band[0].depth = 0.9;

        let front = render_builtin(&front);
        let rear = render_builtin(&rear);
        assert_eq!(front.frames(), rear.frames());
        assert_eq!(front.channels, 5);

        let rear_energy = |buffer: &AudioBuffer| -> f32 {
            buffer
                .samples
                .as_chunks::<5>()
                .0
                .iter()
                .map(|frame| frame[3].abs() + frame[4].abs())
                .sum()
        };
        let front_rear = rear_energy(&front);
        let rear_rear = rear_energy(&rear);
        assert!(
            rear_rear > front_rear + 1.0,
            "depth must move the slot to the rear pair: {front_rear} -> {rear_rear}"
        );
        assert!(rear.samples.iter().all(|sample| sample.is_finite()));
        assert!(rear.samples.iter().all(|sample| sample.abs() <= 1.0));
    }
}
