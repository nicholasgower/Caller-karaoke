//! Local Kokoro-82M neural voice.
//!
//! The model and voice are fetched on first use, verified against pinned
//! SHA-256 digests, and cached in the application cache. Inference runs on a
//! worker thread so the UI and audio callback never block.
//!
//! Text is phonemized with Misaki's pure-Rust G2P (the front end Kokoro was
//! trained on). The reviewed dance-term overrides in
//! [`contra_core::pronounce`] are injected into that lexicon, then the
//! resulting IPA is normalized to Kokoro's trained alphabet before tokenizing.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

use contra_core::CallTone;
use contra_core::pronounce;
#[cfg(target_os = "linux")]
use flate2::read::GzDecoder;
use misaki_rs::lexicon::PhonemeEntry;
use misaki_rs::{G2P, Language};
use ort::session::Session;
use ort::value::Tensor;
use sha2::{Digest, Sha256};
#[cfg(target_os = "linux")]
use tar::Archive;

const SAMPLE_RATE: u32 = 24_000;
const VOICE_ROWS: usize = 510;
const VOICE_DIM: usize = 256;
const PROGRESS_STEP: u64 = 256 * 1024;

struct Asset {
    name: &'static str,
    url: &'static str,
    sha256: &'static str,
    size: u64,
}

const MODEL: Asset = Asset {
    name: "model_quantized.onnx",
    url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/onnx/model_quantized.onnx",
    sha256: "fbae9257e1e05ffc727e951ef9b9c98418e6d79f1c9b6b13bd59f5c9028a1478",
    size: 92_361_116,
};
const CONFIG: Asset = Asset {
    name: "config.json",
    url: "https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/config.json",
    sha256: "5abb01e2403b072bf03d04fde160443e209d7a0dad49a423be15196b9b43c17f",
    size: 2_351,
};

/// Estimated resident memory of a loaded neural voice.
///
/// The quantized model file is loaded into an ONNX Runtime session; the runtime
/// keeps the weights plus per-inference arenas. 1.5× the file is a deliberately
/// conservative estimate for the session, and the one-hot voice row and the
/// runtime library are counted at their measured sizes.
pub(crate) fn working_set_bytes() -> u64 {
    MODEL.size + MODEL.size / 2 + 522_240 + runtime_bytes()
}

/// Size of the ONNX Runtime library this build loads.
pub(crate) fn runtime_bytes() -> u64 {
    #[cfg(windows)]
    {
        include_bytes!(concat!(env!("OUT_DIR"), "/onnxruntime.dll")).len() as u64
    }
    #[cfg(target_os = "linux")]
    {
        ONNX_RUNTIME_SO_SIZE
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        0
    }
}

#[cfg(target_os = "linux")]
const ONNX_RUNTIME: Asset = Asset {
    name: "onnxruntime.tgz",
    url: "https://github.com/microsoft/onnxruntime/releases/download/v1.31.0/onnxruntime-linux-x64-1.31.0.tgz",
    sha256: "cc5c72baf5ae5c8238a6841f0897227be2d02826b9cf98eaf02fdefaeeb03a57",
    size: 11_408_003,
};

#[cfg(target_os = "linux")]
const ONNX_RUNTIME_SO_SHA256: &str =
    "469849977cb60744ea160927e1a3a0643fd4c55c0aff743251c6d293402e334e";
#[cfg(target_os = "linux")]
const ONNX_RUNTIME_SO_SIZE: u64 = 29_345_384;

pub(crate) enum KokoroEvent {
    Status {
        message: String,
        progress: Option<f32>,
        ready: bool,
    },
    Audio(Vec<f32>),
    Failed(String),
}

enum Command {
    Enable(bool),
    Prewarm(Vec<String>),
    Speak { text: String, generation: u64 },
    Stop,
    SetRate(f32),
    SetTone(CallTone),
    SetVoice(String),
    Retry { enabled: bool, calls: Vec<String> },
}

enum PrewarmAction {
    Continue,
    Restart,
    Abort,
}

/// Handle to the Kokoro worker thread.
pub(crate) struct KokoroSpeech {
    tx: Sender<Command>,
    rx: Receiver<KokoroEvent>,
    generation: Arc<AtomicU64>,
}

impl KokoroSpeech {
    pub(crate) fn spawn(cache_dir: PathBuf, output_sample_rate: u32) -> Option<Self> {
        let (tx, commands) = channel();
        let (events, rx) = channel();
        let generation = Arc::new(AtomicU64::new(0));
        let worker_generation = Arc::clone(&generation);
        std::thread::Builder::new()
            .name("contra-kokoro".into())
            .spawn(move || {
                Worker::new(cache_dir, output_sample_rate).run(commands, events, worker_generation);
            })
            .ok()?;
        Some(Self { tx, rx, generation })
    }

    pub(crate) fn enable(&self, enabled: bool) {
        let _ = self.tx.send(Command::Enable(enabled));
    }

    pub(crate) fn prewarm(&self, calls: Vec<String>) {
        let _ = self.tx.send(Command::Prewarm(calls));
    }

    pub(crate) fn speak(&self, text: &str) {
        let generation = self.generation.load(Ordering::Relaxed);
        let _ = self.tx.send(Command::Speak {
            text: pronounce::kokoro_text(text),
            generation,
        });
    }

    pub(crate) fn stop(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
        let _ = self.tx.send(Command::Stop);
    }

    pub(crate) fn set_rate(&self, rate: f32) {
        let _ = self.tx.send(Command::SetRate(rate));
    }

    pub(crate) fn set_tone(&self, tone: CallTone) {
        let _ = self.tx.send(Command::SetTone(tone));
    }

    pub(crate) fn set_voice(&self, id: &str) {
        let _ = self.tx.send(Command::SetVoice(id.to_string()));
    }

    pub(crate) fn retry(&self, enabled: bool, calls: Vec<String>) {
        let _ = self.tx.send(Command::Retry { enabled, calls });
    }

    pub(crate) fn try_recv(&self) -> Option<KokoroEvent> {
        self.rx.try_recv().ok()
    }
}

struct Worker {
    cache_dir: PathBuf,
    output_sample_rate: u32,
    rate: f32,
    tone: CallTone,
    voice: &'static crate::tts::VoiceOption,
    enabled: bool,
    engine: Option<Engine>,
    environment: Option<ort::environment::Environment>,
    cache: HashMap<String, Vec<f32>>,
    last_calls: Vec<String>,
    failed: Option<String>,
}

impl Worker {
    fn new(cache_dir: PathBuf, output_sample_rate: u32) -> Self {
        Self {
            cache_dir,
            output_sample_rate,
            rate: 1.0,
            tone: CallTone::default(),
            voice: crate::tts::voice(crate::tts::DEFAULT_VOICE_ID),
            enabled: false,
            engine: None,
            environment: None,
            cache: HashMap::new(),
            last_calls: Vec::new(),
            failed: None,
        }
    }

    fn run(
        mut self,
        commands: Receiver<Command>,
        events: Sender<KokoroEvent>,
        generation: Arc<AtomicU64>,
    ) {
        while let Ok(command) = commands.recv() {
            match command {
                Command::Enable(enabled) => {
                    self.set_enabled(enabled);
                    if enabled {
                        self.ensure_ready(&events);
                        self.prewarm(&commands, &events, &generation);
                    }
                }
                Command::Prewarm(calls) => {
                    self.last_calls = calls;
                    if self.enabled {
                        self.ensure_ready(&events);
                        self.prewarm(&commands, &events, &generation);
                    }
                }
                Command::Speak {
                    text,
                    generation: asked,
                } => {
                    if !self.enabled {
                        continue;
                    }
                    if self.engine.is_none() {
                        self.ensure_ready(&events);
                    }
                    if self.engine.is_none() {
                        continue;
                    }
                    self.speak_now(&text, asked, &events, &generation);
                }
                Command::Stop => {
                    // The handle bumps the generation counter before sending
                    // this command, so in-flight synthesis is discarded.
                }
                Command::SetRate(rate) => {
                    self.rate = rate.clamp(
                        contra_core::MIN_TTS_RATE * contra_core::TTS_RATE_SCALE,
                        contra_core::MAX_TTS_RATE * contra_core::TTS_RATE_SCALE,
                    );
                    self.cache.clear();
                }
                Command::SetTone(tone) => {
                    if self.tone == tone {
                        continue;
                    }
                    self.tone = tone;
                    self.cache.clear();
                }
                Command::SetVoice(id) => {
                    let voice = crate::tts::voice(&id);
                    if self.voice.id == voice.id {
                        continue;
                    }
                    self.voice = voice;
                    self.cache.clear();
                    self.engine = None;
                    self.failed = None;
                    if self.enabled {
                        self.ensure_ready(&events);
                        self.prewarm(&commands, &events, &generation);
                    }
                }
                Command::Retry { enabled, calls } => {
                    self.failed = None;
                    self.enabled = enabled;
                    self.last_calls = calls;
                    if enabled {
                        self.ensure_ready(&events);
                        self.prewarm(&commands, &events, &generation);
                    }
                }
            }
        }
    }

    fn speak_now(
        &mut self,
        text: &str,
        asked: u64,
        events: &Sender<KokoroEvent>,
        generation: &AtomicU64,
    ) {
        let samples = if let Some(samples) = self.cache.get(text) {
            Some(samples.clone())
        } else {
            self.synthesize(text, events)
        };
        if let Some(samples) = samples
            && generation.load(Ordering::Relaxed) == asked
        {
            let _ = events.send(KokoroEvent::Audio(samples));
        }
    }

    /// Turn the voice on or off.
    ///
    /// Turning it off releases the quantized model and the synthesized-call
    /// cache. ONNX Runtime otherwise keeps roughly a hundred megabytes of
    /// session state alive for the rest of the process, even though nothing
    /// will speak until the control is switched on again; re-enabling reloads
    /// the already-cached files.
    fn set_enabled(&mut self, enabled: bool) -> PrewarmAction {
        self.enabled = enabled;
        if enabled {
            PrewarmAction::Restart
        } else {
            self.engine = None;
            self.cache.clear();
            PrewarmAction::Abort
        }
    }

    fn ensure_ready(&mut self, events: &Sender<KokoroEvent>) {
        if self.engine.is_some() || self.failed.is_some() {
            return;
        }
        let _ = events.send(KokoroEvent::Status {
            message: "Preparing the neural voice".into(),
            progress: Some(0.0),
            ready: false,
        });
        let mut report = |message: &str, done: u64, total: u64| {
            let progress = if total == 0 {
                None
            } else {
                Some((done as f32 / total as f32).clamp(0.0, 1.0))
            };
            let _ = events.send(KokoroEvent::Status {
                message: message.into(),
                progress,
                ready: false,
            });
        };
        let environment = match &self.environment {
            Some(environment) => environment.clone(),
            None => match init_onnx(&self.cache_dir, &mut report) {
                Ok(environment) => {
                    self.environment = Some(environment.clone());
                    environment
                }
                Err(error) => {
                    self.failed = Some(error.clone());
                    let _ = events.send(KokoroEvent::Failed(error));
                    return;
                }
            },
        };
        match Engine::load(&self.cache_dir, self.voice, &environment, &mut report) {
            Ok(engine) => {
                self.engine = Some(engine);
                self.failed = None;
                let _ = events.send(KokoroEvent::Status {
                    message: format!("{} ready", self.voice.label),
                    progress: None,
                    ready: true,
                });
            }
            Err(error) => {
                self.failed = Some(error.clone());
                let _ = events.send(KokoroEvent::Failed(error));
            }
        }
    }

    fn prewarm(
        &mut self,
        commands: &Receiver<Command>,
        events: &Sender<KokoroEvent>,
        generation: &AtomicU64,
    ) {
        if self.engine.is_none() {
            return;
        }
        let start_generation = generation.load(Ordering::Relaxed);
        let mut calls = self.last_calls.clone();
        let mut index = 0;
        let mut failed = false;
        while index < calls.len() {
            // Prewarming must not starve live calls: handle anything the UI
            // queued while the last phrase was rendering.
            while let Ok(command) = commands.try_recv() {
                match self.handle_prewarm_command(command, events, generation) {
                    PrewarmAction::Continue => {}
                    PrewarmAction::Restart => {
                        calls = self.last_calls.clone();
                        index = 0;
                    }
                    PrewarmAction::Abort => return,
                }
            }
            if generation.load(Ordering::Relaxed) != start_generation {
                return;
            }
            let text = &calls[index];
            if !self.cache.contains_key(text) {
                if self.synthesize(text, events).is_some() {
                    let total = calls.len();
                    let _ = events.send(KokoroEvent::Status {
                        message: format!("Preparing calls ({}/{total})", index + 1),
                        progress: Some((index + 1) as f32 / total as f32),
                        ready: true,
                    });
                } else {
                    failed = true;
                }
            }
            index += 1;
        }
        if !failed {
            let _ = events.send(KokoroEvent::Status {
                message: format!("{} ready", self.voice.label),
                progress: None,
                ready: true,
            });
        }
    }

    fn handle_prewarm_command(
        &mut self,
        command: Command,
        events: &Sender<KokoroEvent>,
        generation: &AtomicU64,
    ) -> PrewarmAction {
        match command {
            Command::Speak {
                text,
                generation: asked,
            } => {
                if self.enabled && self.engine.is_some() {
                    self.speak_now(&text, asked, events, generation);
                }
                PrewarmAction::Continue
            }
            Command::Stop => PrewarmAction::Abort,
            Command::SetRate(rate) => {
                self.rate = rate.clamp(
                    contra_core::MIN_TTS_RATE * contra_core::TTS_RATE_SCALE,
                    contra_core::MAX_TTS_RATE * contra_core::TTS_RATE_SCALE,
                );
                self.cache.clear();
                PrewarmAction::Abort
            }
            Command::SetTone(tone) => {
                if self.tone == tone {
                    return PrewarmAction::Continue;
                }
                self.tone = tone;
                self.cache.clear();
                PrewarmAction::Restart
            }
            Command::SetVoice(id) => {
                let voice = crate::tts::voice(&id);
                if self.voice.id == voice.id {
                    return PrewarmAction::Continue;
                }
                self.voice = voice;
                self.cache.clear();
                self.engine = None;
                self.failed = None;
                if self.enabled {
                    self.ensure_ready(events);
                    if self.engine.is_some() {
                        PrewarmAction::Restart
                    } else {
                        PrewarmAction::Abort
                    }
                } else {
                    PrewarmAction::Abort
                }
            }
            Command::Prewarm(calls) => {
                self.last_calls = calls;
                PrewarmAction::Restart
            }
            Command::Enable(enabled) => self.set_enabled(enabled),
            Command::Retry { enabled, calls } => {
                self.failed = None;
                self.enabled = enabled;
                self.last_calls = calls;
                if enabled {
                    self.ensure_ready(events);
                    if self.engine.is_some() {
                        PrewarmAction::Restart
                    } else {
                        PrewarmAction::Abort
                    }
                } else {
                    PrewarmAction::Abort
                }
            }
        }
    }

    fn synthesize(&mut self, text: &str, events: &Sender<KokoroEvent>) -> Option<Vec<f32>> {
        let engine = self.engine.as_mut()?;
        match engine.synthesize(text, self.rate, self.tone, self.output_sample_rate) {
            Ok(samples) => {
                self.cache.insert(text.to_string(), samples.clone());
                Some(samples)
            }
            Err(error) => {
                let _ = events.send(KokoroEvent::Failed(error));
                None
            }
        }
    }
}

struct Engine {
    _environment: ort::environment::Environment,
    session: Session,
    vocab: HashMap<char, u32>,
    voice: Vec<f32>,
    g2p: G2P,
}

impl Engine {
    fn load(
        cache_dir: &Path,
        voice: &crate::tts::VoiceOption,
        environment: &ort::environment::Environment,
        mut progress: impl FnMut(&str, u64, u64),
    ) -> Result<Self, String> {
        let assets = ensure_assets(cache_dir, voice, &mut progress)?;
        let config = fs::read_to_string(&assets.config)
            .map_err(|error| format!("could not read the Kokoro config: {error}"))?;
        let config: serde_json::Value = serde_json::from_str(&config)
            .map_err(|error| format!("could not parse the Kokoro config: {error}"))?;
        let mut vocab = HashMap::new();
        if let Some(entries) = config.get("vocab").and_then(serde_json::Value::as_object) {
            for (key, value) in entries {
                let mut chars = key.chars();
                let Some(ch) = chars.next() else { continue };
                if chars.next().is_some() {
                    continue;
                }
                if let Some(id) = value.as_u64() {
                    vocab.insert(ch, id as u32);
                }
            }
        }
        if vocab.len() < 100 {
            return Err("the Kokoro vocabulary is incomplete".into());
        }

        let voice = read_voice(&assets.voice)?;
        let threads = std::thread::available_parallelism()
            .map(|value| value.get().clamp(1, 4))
            .unwrap_or(1);
        let session = Session::builder(environment)
            .map_err(|error| format!("could not create the Kokoro session: {error}"))?
            .with_intra_threads(threads)
            .map_err(|error| format!("could not configure the Kokoro session: {error}"))?
            .commit_from_file(&assets.model)
            .map_err(|error| format!("could not load the Kokoro model: {error}"))?;

        let mut g2p = G2P::new(Language::EnglishUS);
        for (word, phones) in pronounce::KOKORO_OVERRIDES {
            g2p.lexicon.golds.insert(
                (*word).to_string(),
                PhonemeEntry::Simple((*phones).to_string()),
            );
            g2p.lexicon.golds.insert(
                capitalize(word),
                PhonemeEntry::Simple((*phones).to_string()),
            );
        }

        Ok(Self {
            _environment: environment.clone(),
            session,
            vocab,
            voice,
            g2p,
        })
    }

    fn synthesize(
        &mut self,
        text: &str,
        rate: f32,
        tone: CallTone,
        output_sample_rate: u32,
    ) -> Result<Vec<f32>, String> {
        let input = pronounce::kokoro_text(text);
        let (raw, _) = self
            .g2p
            .g2p(&input)
            .map_err(|error| format!("could not phonemize {text:?}: {error}"))?;
        let mut phonemes = to_kokoro_phonemes(&raw);
        phonemes = smooth_count_phonemes(&phonemes);
        if tone.enunciation() {
            phonemes = enunciate_phonemes(&phonemes);
        }
        self.render(&phonemes, text, rate, tone, output_sample_rate)
    }

    fn render(
        &mut self,
        phonemes: &str,
        text: &str,
        rate: f32,
        tone: CallTone,
        output_sample_rate: u32,
    ) -> Result<Vec<f32>, String> {
        if phonemes.is_empty() {
            return Err(format!("no phonemes for {text:?}"));
        }
        let unknown: Vec<char> = phonemes
            .chars()
            .filter(|ch| !ch.is_whitespace() && !self.vocab.contains_key(ch))
            .collect();
        if !unknown.is_empty() {
            return Err(format!(
                "the voice has no symbols for {} in {text:?}",
                unknown.iter().collect::<String>()
            ));
        }

        let mut tokens: Vec<i64> = Vec::with_capacity(phonemes.chars().count() + 2);
        tokens.push(0);
        for ch in phonemes.chars() {
            if let Some(id) = self.vocab.get(&ch) {
                tokens.push(*id as i64);
            }
        }
        tokens.push(0);
        if tokens.len() <= 2 {
            return Err(format!("no tokens for {text:?}"));
        }
        let row = (tokens.len() - 1).min(VOICE_ROWS - 1);
        let style = self.voice[row * VOICE_DIM..(row + 1) * VOICE_DIM].to_vec();

        let input_ids = Tensor::from_array((vec![1, tokens.len()], tokens))
            .map_err(|error| format!("could not build the Kokoro input: {error}"))?;
        let style = Tensor::from_array((vec![1, VOICE_DIM], style))
            .map_err(|error| format!("could not build the Kokoro style: {error}"))?;
        let speed = Tensor::from_array((vec![1], vec![synthesis_speed(rate, tone.pitch())]))
            .map_err(|error| format!("could not build the Kokoro speed: {error}"))?;
        let outputs = self
            .session
            .run(ort::inputs![
                "input_ids" => input_ids,
                "style" => style,
                "speed" => speed,
            ])
            .map_err(|error| format!("Kokoro inference failed: {error}"))?;
        let output = outputs
            .get("waveform")
            .ok_or_else(|| "Kokoro returned no audio".to_string())?;
        let (_, audio) = output
            .try_extract_tensor::<f32>()
            .map_err(|error| format!("could not read Kokoro audio: {error}"))?;
        let mut audio = audio.to_vec();
        for sample in &mut audio {
            if !sample.is_finite() {
                *sample = 0.0;
            }
        }
        trim_silence(&mut audio);
        fade(&mut audio);
        // A lower tone is produced by synthesizing the same words a little
        // faster and then playing the samples back a little slower. The two
        // cancel in duration, so the call still fits its timeline slot; only
        // the pitch and formants move down.
        let mut samples = resample_linear(
            &audio,
            SAMPLE_RATE,
            pitch_shifted_rate(output_sample_rate, tone.pitch()),
        );
        if tone.enunciation() {
            enunciate_audio(&mut samples, output_sample_rate);
        }
        Ok(samples)
    }
}

/// Kokoro's `speed` input for a target pitch and playback rate.
///
/// A pitch multiplier below one needs a faster synthesis so that the later
/// slow-down does not lengthen the call.
fn synthesis_speed(rate: f32, pitch: f32) -> f32 {
    rate / pitch.clamp(0.5, 2.0)
}

/// The rate the shifted samples are resampled to.
///
/// This is deliberately not the device rate: the buffer is played at
/// `output_sample_rate`, so resampling it to a higher rate is what lowers the
/// pitch and lengthens the samples. [`synthesis_speed`] compensates for the
/// length.
fn pitch_shifted_rate(output_sample_rate: u32, pitch: f32) -> u32 {
    ((output_sample_rate as f32 / pitch.clamp(0.5, 2.0)).round() as u32).max(1)
}

/// Lengthen the primary stressed vowel in each content word.
///
/// Kokoro's front end already marks stress, but the vowels are often short.
/// Adding the IPA length mark after a stressed vowel makes the word land more
/// deliberately, which is the core of a caller's diction. Function words have
/// no primary stress and are left alone, so the stress contrast stays natural.
fn enunciate_phonemes(phonemes: &str) -> String {
    let mut out = String::with_capacity(phonemes.len() + 8);
    let mut chars = phonemes.chars().peekable();
    while let Some(ch) = chars.next() {
        out.push(ch);
        if ch != 'ˈ' {
            continue;
        }
        if chars.peek().is_some_and(|next| is_vowel(*next)) {
            if let Some(vowel) = chars.next() {
                out.push(vowel);
            }
            if chars.peek() != Some(&'ː') {
                out.push('ː');
            }
        }
    }
    out
}

/// Connect the count tail "and a half" into one caller phrase.
///
/// Kokoro treats every space as a pause, so "once and a half" comes out as four
/// separated words. Callers reduce it to "once 'n' a half"; this substitution
/// gives the model the same connected phrase without changing the card text.
fn smooth_count_phonemes(phonemes: &str) -> String {
    phonemes
        .replace("ænd ɐ h", "nəh")
        .replace("ænd ə h", "nəh")
        .replace("ɐn ɐ h", "nəh")
        .replace("ɐn ə h", "nəh")
        .replace("ən ɐ h", "nəh")
}

/// Vowels in Kokoro's trained alphabet (plus the IPA symbols Misaki emits
/// before normalization).
fn is_vowel(ch: char) -> bool {
    matches!(
        ch,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'A'
            | 'I'
            | 'O'
            | 'W'
            | 'Y'
            | 'æ'
            | 'ɑ'
            | 'ɒ'
            | 'ɔ'
            | 'ɛ'
            | 'ɜ'
            | 'ɪ'
            | 'ʊ'
            | 'ʌ'
            | 'ɐ'
            | 'ə'
            | 'ɚ'
            | 'ɝ'
            | 'ɘ'
            | 'ɞ'
            | 'ɨ'
            | 'ʉ'
            | 'ɯ'
            | 'ɤ'
            | 'ɵ'
            | 'ø'
            | 'œ'
            | 'ᵻ'
    )
}

/// A second-order IIR section in transposed direct form II.
#[derive(Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn high_pass(sample_rate: f32, frequency: f32, q: f32) -> Self {
        let w0 = std::f32::consts::TAU * frequency / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        Self::normalized(
            (1.0 + cos) * 0.5,
            -(1.0 + cos),
            (1.0 + cos) * 0.5,
            1.0 + alpha,
            -2.0 * cos,
            1.0 - alpha,
        )
    }

    fn peaking(sample_rate: f32, frequency: f32, gain_db: f32, q: f32) -> Self {
        let w0 = std::f32::consts::TAU * frequency / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a = 10f32.powf(gain_db / 40.0);
        Self::normalized(
            1.0 + alpha * a,
            -2.0 * cos,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cos,
            1.0 - alpha / a,
        )
    }

    fn normalized(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }
}

/// Sharpen the neural voice after resampling.
///
/// The pitch shift lowers the formants, which can blur consonants. A gentle
/// high-pass and low-mid cut remove rumble and mud, a presence lift brings out
/// consonant energy around 3 kHz, and a slow compressor evens the syllables so
/// final consonants are not lost. The peak guard keeps speech from clipping the
/// music mix.
fn enunciate_audio(samples: &mut [f32], sample_rate: u32) {
    if samples.is_empty() {
        return;
    }
    let sample_rate = sample_rate as f32;
    let mut high_pass = Biquad::high_pass(sample_rate, 90.0, 0.707);
    let mut de_mud = Biquad::peaking(sample_rate, 300.0, -1.5, 0.9);
    let mut presence = Biquad::peaking(sample_rate, 2_800.0, 3.0, 1.0);
    for sample in samples.iter_mut() {
        let mut value = *sample;
        value = high_pass.process(value);
        value = de_mud.process(value);
        value = presence.process(value);
        *sample = value;
    }
    compress(samples, sample_rate);
}

fn compress(samples: &mut [f32], sample_rate: f32) {
    const THRESHOLD_DB: f32 = -22.0;
    const RATIO: f32 = 2.5;
    const MAKEUP_DB: f32 = 2.0;
    const ATTACK_SECONDS: f32 = 0.005;
    const RELEASE_SECONDS: f32 = 0.080;

    let attack = (-1.0 / (ATTACK_SECONDS * sample_rate)).exp();
    let release = (-1.0 / (RELEASE_SECONDS * sample_rate)).exp();
    let makeup = 10f32.powf(MAKEUP_DB / 20.0);
    let mut envelope = 0.0f32;
    for sample in samples.iter_mut() {
        let level = sample.abs();
        let coefficient = if level > envelope { attack } else { release };
        envelope = coefficient * envelope + (1.0 - coefficient) * level;
        let gain = if envelope > 0.0 {
            let envelope_db = 20.0 * envelope.log10();
            if envelope_db > THRESHOLD_DB {
                let reduction = (envelope_db - THRESHOLD_DB) * (1.0 - 1.0 / RATIO);
                10f32.powf(-reduction / 20.0)
            } else {
                1.0
            }
        } else {
            1.0
        };
        *sample *= gain * makeup;
    }
    let peak = samples
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    if peak > 0.98 {
        let scale = 0.98 / peak;
        for sample in samples.iter_mut() {
            *sample *= scale;
        }
    }
}

struct Assets {
    model: PathBuf,
    voice: PathBuf,
    config: PathBuf,
}

fn ensure_assets(
    cache_dir: &Path,
    voice: &crate::tts::VoiceOption,
    progress: &mut dyn FnMut(&str, u64, u64),
) -> Result<Assets, String> {
    let directory = cache_dir.join("kokoro-v1.0");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create the voice cache: {error}"))?;
    let model = ensure_asset(&directory, &MODEL, progress)?;
    let config = ensure_asset(&directory, &CONFIG, progress)?;
    let voice_asset = Asset {
        name: voice.file,
        url: voice.url,
        sha256: voice.sha256,
        size: voice.size,
    };
    let voice = ensure_asset(&directory, &voice_asset, progress)?;
    Ok(Assets {
        model,
        voice,
        config,
    })
}

fn ensure_asset(
    directory: &Path,
    asset: &Asset,
    progress: &mut dyn FnMut(&str, u64, u64),
) -> Result<PathBuf, String> {
    let path = directory.join(asset.name);
    let valid = fs::metadata(&path)
        .map(|metadata| metadata.len() == asset.size)
        .unwrap_or(false)
        && verify_sha256(&path, asset.sha256).unwrap_or(false);
    if !valid {
        download(asset, &path, progress)?;
    }
    Ok(path)
}

fn verify_sha256(path: &Path, expected: &str) -> Result<bool, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(digest == expected)
}

fn download(
    asset: &Asset,
    path: &Path,
    progress: &mut dyn FnMut(&str, u64, u64),
) -> Result<(), String> {
    let temporary = path.with_extension("part");
    let _ = fs::remove_file(&temporary);
    let mut response = ureq::get(asset.url)
        .header("User-Agent", "contra-karaoke/0.1.0")
        .call()
        .map_err(|error| format!("could not download {}: {error}", asset.name))?;
    let mut reader = response.body_mut().as_reader();
    let mut file = fs::File::create(&temporary)
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    let mut hasher = Sha256::new();
    let mut done = 0u64;
    let mut reported = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("could not read {}: {error}", asset.name))?;
        if read == 0 {
            break;
        }
        done += read as u64;
        if done > asset.size + 1024 * 1024 {
            let _ = fs::remove_file(&temporary);
            return Err(format!("{} was larger than expected", asset.name));
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .map_err(|error| format!("could not save {}: {error}", asset.name))?;
        if done - reported >= PROGRESS_STEP {
            reported = done;
            progress(
                &format!("Downloading the neural voice ({})", asset.name),
                done,
                asset.size,
            );
        }
    }
    file.flush()
        .map_err(|error| format!("could not save {}: {error}", asset.name))?;
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if digest != asset.sha256 || done != asset.size {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "{} failed its integrity check (got {done} bytes, sha256 {digest})",
            asset.name
        ));
    }
    fs::rename(&temporary, path)
        .map_err(|error| format!("could not finish saving {}: {error}", asset.name))?;
    progress(
        &format!("Downloaded {}", asset.name),
        asset.size,
        asset.size,
    );
    Ok(())
}

fn read_voice(path: &Path) -> Result<Vec<f32>, String> {
    let bytes = fs::read(path).map_err(|error| format!("could not read the voice: {error}"))?;
    if bytes.len() < VOICE_ROWS * VOICE_DIM * 4 {
        return Err("the Kokoro voice file is too small".into());
    }
    let voice = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect();
    Ok(voice)
}

#[cfg(windows)]
fn init_onnx(
    cache_dir: &Path,
    _progress: &mut dyn FnMut(&str, u64, u64),
) -> Result<ort::environment::Environment, String> {
    // The runtime DLL is embedded so the portable Windows build remains a
    // single executable. It is copied to the cache before ONNX Runtime loads
    // it; the file is reused on later runs.
    const RUNTIME: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/onnxruntime.dll"));
    let directory = cache_dir.join("onnxruntime");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create the ONNX Runtime cache: {error}"))?;
    let path = directory.join("onnxruntime.dll");
    let valid = fs::metadata(&path)
        .map(|metadata| metadata.len() as usize == RUNTIME.len())
        .unwrap_or(false);
    if !valid {
        let temporary = directory.join("onnxruntime.dll.part");
        fs::write(&temporary, RUNTIME)
            .map_err(|error| format!("could not unpack ONNX Runtime: {error}"))?;
        fs::rename(&temporary, &path)
            .map_err(|error| format!("could not install ONNX Runtime: {error}"))?;
    }
    ort::init_from(&path)
        .map_err(|error| {
            format!(
                "could not load ONNX Runtime from {}: {error}",
                path.display()
            )
        })?
        .with_name("contra-karaoke")
        .build()
        .map_err(|error| format!("could not start ONNX Runtime: {error}"))
}

#[cfg(target_os = "linux")]
fn init_onnx(
    cache_dir: &Path,
    progress: &mut dyn FnMut(&str, u64, u64),
) -> Result<ort::environment::Environment, String> {
    let path = ensure_onnx_runtime(cache_dir, progress)?;
    ort::init_from(&path)
        .map_err(|error| {
            format!(
                "could not load ONNX Runtime from {}: {error}",
                path.display()
            )
        })?
        .with_name("contra-karaoke")
        .build()
        .map_err(|error| format!("could not start ONNX Runtime: {error}"))
}

#[cfg(all(unix, not(target_os = "linux")))]
fn init_onnx(
    _cache_dir: &Path,
    _progress: &mut dyn FnMut(&str, u64, u64),
) -> Result<ort::environment::Environment, String> {
    Err("the neural voice is not available on this platform".into())
}

#[cfg(target_os = "linux")]
fn ensure_onnx_runtime(
    cache_dir: &Path,
    progress: &mut dyn FnMut(&str, u64, u64),
) -> Result<PathBuf, String> {
    let directory = cache_dir.join("onnxruntime");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create the ONNX Runtime cache: {error}"))?;
    let path = directory.join("libonnxruntime.so.1.31.0");
    let valid = fs::metadata(&path)
        .map(|metadata| metadata.len() == ONNX_RUNTIME_SO_SIZE)
        .unwrap_or(false)
        && verify_sha256(&path, ONNX_RUNTIME_SO_SHA256).unwrap_or(false);
    if valid {
        return Ok(path);
    }

    let package = directory.join(ONNX_RUNTIME.name);
    let valid = fs::metadata(&package)
        .map(|metadata| metadata.len() == ONNX_RUNTIME.size)
        .unwrap_or(false);
    if !valid {
        download(&ONNX_RUNTIME, &package, progress)?;
    }

    let file = fs::File::open(&package)
        .map_err(|error| format!("could not open the ONNX Runtime archive: {error}"))?;
    let decoder = GzDecoder::new(file);
    let mut tar_archive = Archive::new(decoder);
    let mut extracted = false;
    let entries = tar_archive
        .entries()
        .map_err(|error| format!("could not read the ONNX Runtime archive: {error}"))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|error| format!("could not read an ONNX Runtime entry: {error}"))?;
        let name = entry
            .path()
            .map_err(|error| format!("could not read an ONNX Runtime path: {error}"))?
            .to_string_lossy()
            .into_owned();
        if !name.ends_with("lib/libonnxruntime.so.1.31.0") {
            continue;
        }
        let temporary = directory.join("libonnxruntime.so.part");
        let mut output = fs::File::create(&temporary)
            .map_err(|error| format!("could not write ONNX Runtime: {error}"))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|error| format!("could not extract ONNX Runtime: {error}"))?;
        output
            .flush()
            .map_err(|error| format!("could not finish writing ONNX Runtime: {error}"))?;
        if !verify_sha256(&temporary, ONNX_RUNTIME_SO_SHA256)? {
            let _ = fs::remove_file(&temporary);
            return Err("the extracted ONNX Runtime library failed its integrity check".into());
        }
        fs::rename(&temporary, &path)
            .map_err(|error| format!("could not install ONNX Runtime: {error}"))?;
        extracted = true;
        break;
    }
    if !extracted {
        return Err("the ONNX Runtime library was not found in the archive".into());
    }
    let _ = fs::remove_file(&package);
    Ok(path)
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Normalize Misaki's IPA to Kokoro's trained token alphabet.
///
/// Misaki writes diphthongs and affricates as joined sequences with U+200D;
/// Kokoro has single trained tokens for them (`A`, `I`, `O`, `W`, `Y`, `ʤ`,
/// `ʧ`). Remaining joiners are dropped.
fn to_kokoro_phonemes(raw: &str) -> String {
    let mut phonemes = raw.to_string();
    for (from, to) in [
        ("d\u{200D}ʒ", "ʤ"),
        ("t\u{200D}ʃ", "ʧ"),
        ("e\u{200D}ɪ", "A"),
        ("a\u{200D}ɪ", "I"),
        ("o\u{200D}ʊ", "O"),
        ("a\u{200D}ʊ", "W"),
        ("ɔ\u{200D}ɪ", "Y"),
    ] {
        phonemes = phonemes.replace(from, to);
    }
    phonemes = phonemes.replace('\u{200D}', "");
    phonemes = phonemes.replace('g', "ɡ");
    phonemes.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn trim_silence(samples: &mut Vec<f32>) {
    const THRESHOLD: f32 = 1e-3;
    let Some(start) = samples.iter().position(|sample| sample.abs() > THRESHOLD) else {
        samples.clear();
        return;
    };
    let Some(end) = samples.iter().rposition(|sample| sample.abs() > THRESHOLD) else {
        samples.clear();
        return;
    };
    let preroll = (SAMPLE_RATE as usize / 200).min(start);
    let tail = (SAMPLE_RATE as usize / 100).min(samples.len().saturating_sub(end + 1));
    let start = start.saturating_sub(preroll);
    let end = (end + 1 + tail).min(samples.len());
    samples.drain(..start);
    samples.truncate(end - start);
}

fn fade(samples: &mut [f32]) {
    let length = (SAMPLE_RATE as usize / 200).min(samples.len() / 2);
    if length == 0 {
        return;
    }
    for index in 0..length {
        let gain = index as f32 / length as f32;
        samples[index] *= gain;
        let end = samples.len() - 1 - index;
        samples[end] *= gain;
    }
}

fn resample_linear(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let output_len = (input.len() as u64 * to as u64 / from as u64) as usize;
    if output_len == 0 {
        return Vec::new();
    }
    let ratio = from as f64 / to as f64;
    let mut output = Vec::with_capacity(output_len);
    for index in 0..output_len {
        let position = index as f64 * ratio;
        let left = position.floor() as usize;
        let right = (left + 1).min(input.len() - 1);
        let fraction = (position - left as f64) as f32;
        output.push(input[left] * (1.0 - fraction) + input[right] * fraction);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;

    static G2P: LazyLock<G2P> = LazyLock::new(|| {
        let mut g2p = G2P::new(Language::EnglishUS);
        for (word, phones) in pronounce::KOKORO_OVERRIDES {
            g2p.lexicon.golds.insert(
                (*word).to_string(),
                PhonemeEntry::Simple((*phones).to_string()),
            );
            g2p.lexicon.golds.insert(
                capitalize(word),
                PhonemeEntry::Simple((*phones).to_string()),
            );
        }
        g2p
    });

    fn phones(text: &str) -> String {
        let (raw, _) = G2P.g2p(&pronounce::kokoro_text(text)).expect("phonemes");
        to_kokoro_phonemes(&raw)
    }

    #[test]
    fn disabling_the_voice_releases_its_cache() {
        let mut worker = Worker::new(std::env::temp_dir(), 48_000);
        worker.engine = None;
        worker.cache.insert("a call".into(), vec![0.25; 1_024]);
        worker.enabled = true;

        assert!(matches!(worker.set_enabled(false), PrewarmAction::Abort));
        assert!(!worker.enabled);
        assert!(worker.engine.is_none());
        assert!(
            worker.cache.is_empty(),
            "synthesized calls must not stay resident after speech is turned off"
        );
    }

    #[test]
    fn reviewed_dance_terms_are_pronounced_correctly() {
        assert!(
            phones("poussette").contains("puˈsɛt"),
            "{}",
            phones("poussette")
        );
        assert!(
            phones("petronella").contains("pˈɛtɹOnˌɛlə"),
            "{}",
            phones("petronella")
        );
        assert!(
            phones("promenade").contains("pɹˌɑmənˈAd"),
            "{}",
            phones("promenade")
        );
        assert!(
            phones("star promenade").contains("pɹˌɑmənˈAd"),
            "{}",
            phones("star promenade")
        );
        assert!(
            phones("Rory O'More").contains("mˈɔːɹ"),
            "{}",
            phones("Rory O'More")
        );
        assert!(phones("gyre").contains("ʤˈI"), "{}", phones("gyre"));
        assert!(
            phones("larks allemande left once and a half").contains("ˈæləmˌænd"),
            "{}",
            phones("larks allemande left once and a half")
        );
        assert!(
            phones("contra corners").contains("kˈɑntɹə"),
            "{}",
            phones("contra corners")
        );
        assert!(
            phones("do-si-do").contains("dˈO sˈi"),
            "{}",
            phones("do-si-do")
        );
        assert_eq!(phones("do-si-do"), "dˈO sˈiː dˈO");
        assert!(
            !phones("do-si-do").contains("dˈuː"),
            "{}",
            phones("do-si-do")
        );
    }

    #[test]
    fn all_default_calls_produce_only_kokoro_symbols() {
        // The reviewed calls used by the default aliases and the bundled dance.
        let calls = [
            "swing your lark",
            "swing your neighbor",
            "larks do-si-do once",
            "larks allemande left once and a half",
            "larks allemande orbit",
            "larks balance",
            "balance the ring",
            "larks balance and swing",
            "larks chain",
            "circle left three places",
            "star right four places",
            "star promenade",
            "long lines forward and back",
            "hey",
            "dolphin hey",
            "larks gyre",
            "promenade",
            "right and left through",
            "pass through",
            "pass by",
            "roll away",
            "petronella",
            "box the gnat",
            "swat the flea",
            "mad robin",
            "Rory O'More",
            "slide left",
            "square through",
            "give and take",
            "butterfly whirl",
            "California twirl",
            "gate",
            "poussette",
            "arch and dive",
            "cross trails",
            "figure eight",
            "turn alone",
            "down the hall",
            "up the hall",
            "form long waves",
            "form an ocean wave",
            "contra corners",
            "meltdown swing",
            "revolving door",
            "see saw",
        ];
        for call in calls {
            let phonemes = phones(call);
            assert!(!phonemes.is_empty(), "{call}");
            assert!(!phonemes.contains('?'), "{call}: {phonemes}");
            assert!(!phonemes.contains('\u{200D}'), "{call}: {phonemes}");

            // The clear-speech pass may only add the length mark; any other
            // symbol would be an unverified change to the pronunciation.
            let enunciated = enunciate_phonemes(&phonemes);
            assert!(
                enunciated
                    .chars()
                    .all(|ch| ch == 'ː' || phonemes.contains(ch)),
                "{call}: {enunciated}"
            );
        }
    }

    #[test]
    fn role_terms_counts_and_alternate_names_are_clear() {
        for call in [
            "robins swing",
            "ladies chain",
            "gentlemen allemande left",
            "gentlespoons balance and swing",
            "leads swing",
            "follows swing",
            "women chain",
            "men chain",
            "lefts allemande left",
            "rights allemande right",
            "first corner allemande right",
            "second corner do-si-do",
            "shadow swing",
            "neighbors swing",
            "partners promenade",
            "gypsy",
            "do si do",
            "circle left once and a half",
            "star right twice and a half",
            "allemande left three times",
            "circle right four places",
        ] {
            let phonemes = phones(call);
            assert!(!phonemes.is_empty(), "{call}");
            assert!(!phonemes.contains('?'), "{call}: {phonemes}");
            assert!(!phonemes.contains('\u{200D}'), "{call}: {phonemes}");
        }
        assert!(phones("gypsy").contains("ʤˈɪp"), "{}", phones("gypsy"));
        assert!(phones("robins").contains("ɹ"), "{}", phones("robins"));
        assert!(
            phones("gentlespoons").contains("spu"),
            "{}",
            phones("gentlespoons")
        );
    }

    #[test]
    fn bundled_dance_calls_are_clear() {
        let dance = crate::defaults::default_dance();
        let timeline = contra_core::PracticeTimeline::build(
            &dance,
            &contra_core::AliasTable::default(),
            &contra_core::RoleTerms::default(),
            &contra_core::LeadSettings::default(),
            contra_core::Tempo::default(),
            contra_core::TimelineConfig::default(),
        )
        .expect("timeline");
        for call in &timeline.calls {
            let phonemes = phones(&call.text);
            assert!(!phonemes.is_empty(), "{}", call.text);
            assert!(!phonemes.contains('?'), "{}: {phonemes}", call.text);
            assert!(!phonemes.contains('\u{200D}'), "{}: {phonemes}", call.text);
        }
    }

    #[test]
    fn resamples_and_trims_audio() {
        let samples = vec![0.0; 100];
        let output = resample_linear(&samples, 24_000, 48_000);
        assert_eq!(output.len(), 200);
        let mut silence = vec![0.0; 100];
        trim_silence(&mut silence);
        assert!(silence.is_empty());
    }

    #[test]
    fn commanding_tone_keeps_the_call_length() {
        let output_rate = 48_000;
        let pitch = CallTone::Commanding.pitch();
        let speed = synthesis_speed(1.0, pitch);
        let shifted = pitch_shifted_rate(output_rate, pitch);
        assert!(speed > 1.0, "{speed}");
        assert!(shifted > output_rate, "{shifted}");

        // The faster synthesis and the slower playback cancel out.
        let length = (1.0 / speed) * (shifted as f32 / output_rate as f32);
        assert!((length - 1.0).abs() < 0.002, "{length}");

        // The natural tone must be the old path, sample for sample.
        assert_eq!(synthesis_speed(1.0, 1.0), 1.0);
        assert_eq!(pitch_shifted_rate(output_rate, 1.0), output_rate);
    }

    #[test]
    fn enunciation_lengthens_stressed_vowels_once() {
        assert_eq!(enunciate_phonemes("lˈɛft"), "lˈɛːft");
        assert_eq!(enunciate_phonemes("dˈO sˈiː dˈO"), "dˈOː sˈiː dˈOː");
        assert_eq!(enunciate_phonemes("pˈɑːɹtnɚ"), "pˈɑːɹtnɚ");
        assert_eq!(enunciate_phonemes("ænd ɐ"), "ænd ɐ");
        assert_eq!(
            enunciate_phonemes("pˈɛtɹOnˌɛlə"),
            "pˈɛːtɹOnˌɛlə",
            "secondary stress stays secondary"
        );

        let once = enunciate_phonemes("lˈɛft");
        assert_eq!(enunciate_phonemes(&once), once, "the pass is idempotent");
    }

    #[test]
    fn half_counts_are_spoken_as_one_phrase() {
        assert_eq!(smooth_count_phonemes("wˈʌns ænd ɐ hˈæf"), "wˈʌns nəhˈæf");
        assert_eq!(smooth_count_phonemes("twˈIs ænd ɐ hˈæf"), "twˈIs nəhˈæf");
        assert_eq!(smooth_count_phonemes("wˈʌns ɐn ɐ hˈæf"), "wˈʌns nəhˈæf");
        assert_eq!(smooth_count_phonemes("kˈɑntɹə"), "kˈɑntɹə");

        let once = smooth_count_phonemes("wˈʌns ænd ɐ hˈæf");
        assert_eq!(smooth_count_phonemes(&once), once, "the pass is idempotent");

        // The spoken form reaches this through the text normalizer.
        assert_eq!(
            smooth_count_phonemes(&phones("once and a half")),
            "wˈʌns nəhˈæf"
        );
    }

    #[test]
    fn enunciation_makes_presence_and_dynamics_distinct() {
        let rate = 48_000;
        let mut low = sine_wave(300.0, rate, 0.2);
        let mut high = sine_wave(3_000.0, rate, 0.2);
        enunciate_audio(&mut low, rate);
        enunciate_audio(&mut high, rate);
        assert!(
            rms(&high) > rms(&low) * 1.2,
            "presence lift should favour consonant energy: {} vs {}",
            rms(&high),
            rms(&low)
        );

        let mut loud = sine_wave(500.0, rate, 0.8);
        let mut quiet = sine_wave(500.0, rate, 0.05);
        compress(&mut loud, rate as f32);
        compress(&mut quiet, rate as f32);
        let ratio_before = 0.8 / 0.05;
        let ratio_after = rms(&loud) / rms(&quiet);
        assert!(
            ratio_after < ratio_before * 0.8,
            "compression should reduce the dynamic range: {ratio_after} vs {ratio_before}"
        );

        let mut samples = sine_wave(500.0, rate, 1.0);
        enunciate_audio(&mut samples, rate);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        let peak = samples
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!(peak <= 0.98, "{peak}");
    }

    fn sine_wave(frequency: f32, sample_rate: u32, amplitude: f32) -> Vec<f32> {
        let rate = sample_rate as f32;
        (0..sample_rate)
            .map(|index| {
                amplitude * (std::f32::consts::TAU * frequency * index as f32 / rate).sin()
            })
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
    }

    #[test]
    #[ignore = "downloads and loads the Kokoro model"]
    fn synthesizes_a_reviewed_call_from_the_real_model() {
        let cache = std::env::var("CONTRA_KOKORO_CACHE")
            .expect("set CONTRA_KOKORO_CACHE to a directory with the Kokoro assets");
        let environment =
            init_onnx(Path::new(&cache), &mut |_, _, _| {}).expect("load ONNX Runtime");
        let voice = crate::tts::voice(crate::tts::DEFAULT_VOICE_ID);
        let mut engine = Engine::load(Path::new(&cache), voice, &environment, |_, _, _| {})
            .expect("load the voice");
        let samples = engine
            .synthesize(
                "larks allemande left once and a half",
                1.0,
                CallTone::Commanding,
                48_000,
            )
            .expect("synthesize a call");
        assert!(samples.len() > 24_000, "the call was too short");
        let peak = samples
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!(peak > 0.01, "the call was silent");

        // The corrected terms must also reach the model as non-silent audio.
        for text in [
            "poussette",
            "petronella",
            "promenade",
            "Rory O'More",
            "gentlespoons",
            "neighbors do-si-do once",
        ] {
            let samples = engine
                .synthesize(text, 1.0, CallTone::Commanding, 48_000)
                .expect(text);
            let peak = samples
                .iter()
                .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
            assert!(peak > 0.01, "{text} was silent");
        }

        // Every selectable voice must download, load, and speak.
        for voice in crate::tts::VOICES {
            let mut engine = Engine::load(Path::new(&cache), voice, &environment, |_, _, _| {})
                .unwrap_or_else(|error| panic!("{}: {error}", voice.id));
            let samples = engine
                .synthesize("swing your partner", 1.0, CallTone::Commanding, 48_000)
                .unwrap_or_else(|error| panic!("{}: {error}", voice.id));
            let peak = samples
                .iter()
                .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
            assert!(peak > 0.01, "{} was silent", voice.id);
        }
    }
}
