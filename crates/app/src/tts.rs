//! Optional speech for calls.
//!
//! On desktop the preferred backend is a local Kokoro neural voice (see
//! [`kokoro`]). The platform TTS backend remains available as a fallback so
//! the app still speaks calls on first run, offline, or on web/Android where
//! the neural model is not built. Both backends receive the reviewed dance-term
//! respellings from [`contra_core::pronounce`].

use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
use std::time::{Duration, Instant};

use crate::audio::AudioEngine;
use contra_core::CallTone;
use contra_core::pronounce;

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub(crate) mod kokoro;

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
use kokoro::{KokoroEvent, KokoroSpeech};

/// A selectable Kokoro voice.
///
/// The first three fields are used by the UI on every platform. The asset
/// fields are only used by the desktop neural-voice backend.
#[derive(Clone, Copy, Debug)]
pub struct VoiceOption {
    pub id: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
    #[allow(dead_code)] // desktop Kokoro backend only
    pub file: &'static str,
    #[allow(dead_code)] // desktop Kokoro backend only
    pub url: &'static str,
    #[allow(dead_code)] // desktop Kokoro backend only
    pub sha256: &'static str,
    #[allow(dead_code)] // desktop Kokoro backend only
    pub size: u64,
}

pub const DEFAULT_VOICE_ID: &str = "af_heart";

/// Kokoro voices offered in Preferences. All use the same 82M model and differ
/// only in the small style-embedding file.
pub const VOICES: &[VoiceOption] = &[
    VoiceOption {
        id: "af_heart",
        label: "Heart (US female)",
        detail: "Warm, clear American English female voice.",
        file: "af_heart.bin",
        url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/voices/af_heart.bin",
        sha256: "d583ccff3cdca2f7fae535cb998ac07e9fcb90f09737b9a41fa2734ec44a8f0b",
        size: 522_240,
    },
    VoiceOption {
        id: "af_bella",
        label: "Bella (US female)",
        detail: "Brighter American English female voice.",
        file: "af_bella.bin",
        url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/voices/af_bella.bin",
        sha256: "f69d836209b78eb8c66e75e3cda491e26ea838a3674257e9d4e5703cbaf55c8b",
        size: 522_240,
    },
    VoiceOption {
        id: "af_sarah",
        label: "Sarah (US female)",
        detail: "Clear, steady American English female voice.",
        file: "af_sarah.bin",
        url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/voices/af_sarah.bin",
        sha256: "4409fbc125afabacc615d94db5398d847006a737b0247d6892b7a9a0007a2f0a",
        size: 522_240,
    },
    VoiceOption {
        id: "am_michael",
        label: "Michael (US male)",
        detail: "Calm American English male voice.",
        file: "am_michael.bin",
        url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/voices/am_michael.bin",
        sha256: "1d1f21dd8da39c30705cd4c75d039d265e9bc4a2a93ed09bc9e1b1225eb95ba1",
        size: 522_240,
    },
    VoiceOption {
        id: "am_fenrir",
        label: "Fenrir (US male)",
        detail: "Deeper American English male voice.",
        file: "am_fenrir.bin",
        url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/voices/am_fenrir.bin",
        sha256: "c27989f741f7ee34d273a39d8a595cc0837d35f5ced9a29b7cc162614616df43",
        size: 522_240,
    },
    VoiceOption {
        id: "am_onyx",
        label: "Onyx (US male)",
        detail: "Deep, resonant American English male voice.",
        file: "am_onyx.bin",
        url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/voices/am_onyx.bin",
        sha256: "da5d135b424164916d75a68ffb4c2abce3d7d5ccc82dd1ee6cf447ce286145e6",
        size: 522_240,
    },
];

/// Find a voice by ID, falling back to the default.
pub fn voice(id: &str) -> &'static VoiceOption {
    VOICES
        .iter()
        .find(|voice| voice.id == id)
        .unwrap_or(&VOICES[0])
}

/// User-visible state of the speech backends.
#[derive(Clone, Debug, Default)]
pub struct SpeechStatus {
    pub message: String,
    pub progress: Option<f32>,
    pub ready: bool,
    pub error: Option<String>,
}

impl SpeechStatus {
    #[cfg(any(target_arch = "wasm32", target_os = "android"))]
    fn platform_only(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            progress: None,
            ready: true,
            error: None,
        }
    }
}

/// Speech coordinator.
pub struct Speech {
    platform: PlatformSpeech,
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    kokoro: Option<KokoroSpeech>,
    status: SpeechStatus,
    enabled: bool,
    rate: f32,
    tone: CallTone,
    /// Prepared deliveries per call text; see [`contra_core::Preferences::tts_deliveries`].
    ///
    /// The platform voice always speaks one delivery, so this is only read by
    /// the desktop coordinator.
    #[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
    deliveries: u8,
    /// Next delivery to use for each call text, so a repeated call rotates
    /// through the prepared versions instead of replaying one recording.
    #[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
    delivery_cursor: HashMap<String, u8>,
    last_calls: Vec<String>,
    voice_id: String,
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    pending_rate: Option<Instant>,
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    pending_kokoro_text: Option<String>,
    /// Test-only stand-in for the neural working set, because the test build
    /// uses the silent stub coordinator and never loads the real model.
    #[cfg(test)]
    neural_bytes_override: Option<u64>,
    /// Test-only stand-in for the synthesized-call cache, which lives on the
    /// worker thread in a real session.
    #[cfg(test)]
    speech_bytes_override: Option<u64>,
}

impl Speech {
    pub fn new(cache_dir: PathBuf, output_sample_rate: u32) -> Self {
        let platform = PlatformSpeech::new();
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            let kokoro = KokoroSpeech::spawn(cache_dir.clone(), output_sample_rate);
            let status = if kokoro.is_some() {
                SpeechStatus {
                    message: "Neural voice ready to prepare".into(),
                    progress: None,
                    ready: false,
                    error: None,
                }
            } else if platform.available() {
                SpeechStatus {
                    message: "Platform voice ready".into(),
                    progress: None,
                    ready: true,
                    error: None,
                }
            } else {
                SpeechStatus {
                    message: "Text-to-speech is not available on this system".into(),
                    error: platform.error().map(str::to_string),
                    ..SpeechStatus::default()
                }
            };
            Self {
                platform,
                kokoro,
                status,
                enabled: false,
                rate: 1.0,
                tone: CallTone::default(),
                deliveries: contra_core::DEFAULT_TTS_DELIVERIES,
                delivery_cursor: HashMap::new(),
                last_calls: Vec::new(),
                #[cfg(test)]
                neural_bytes_override: None,
                #[cfg(test)]
                speech_bytes_override: None,
                voice_id: DEFAULT_VOICE_ID.to_string(),
                pending_rate: None,
                pending_kokoro_text: None,
            }
        }
        #[cfg(any(target_arch = "wasm32", target_os = "android"))]
        {
            let _ = (cache_dir, output_sample_rate);
            let status = if platform.available() {
                SpeechStatus::platform_only("Platform voice ready")
            } else {
                SpeechStatus {
                    message: "Text-to-speech is not available on this system".into(),
                    error: platform.error().map(str::to_string),
                    ..SpeechStatus::default()
                }
            };
            Self {
                platform,
                status,
                enabled: false,
                rate: 1.0,
                tone: CallTone::default(),
                deliveries: contra_core::DEFAULT_TTS_DELIVERIES,
                delivery_cursor: HashMap::new(),
                last_calls: Vec::new(),
                #[cfg(test)]
                neural_bytes_override: None,
                #[cfg(test)]
                speech_bytes_override: None,
                voice_id: DEFAULT_VOICE_ID.to_string(),
            }
        }
    }

    /// A silent speech coordinator for headless screenshots.
    #[cfg(all(
        any(feature = "headless", test),
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    pub fn stub() -> Self {
        Self {
            platform: PlatformSpeech::stub(true, None),
            kokoro: None,
            status: SpeechStatus {
                message: "Headless voice ready".into(),
                progress: None,
                ready: true,
                error: None,
            },
            enabled: false,
            rate: 1.0,
            tone: CallTone::default(),
            deliveries: contra_core::DEFAULT_TTS_DELIVERIES,
            delivery_cursor: HashMap::new(),
            last_calls: Vec::new(),
            #[cfg(test)]
            neural_bytes_override: None,
            #[cfg(test)]
            speech_bytes_override: None,
            voice_id: DEFAULT_VOICE_ID.to_string(),
            pending_rate: None,
            pending_kokoro_text: None,
        }
    }

    /// A silent speech coordinator that reports no usable voice.
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    pub fn stub_unavailable() -> Self {
        Self {
            platform: PlatformSpeech::stub(false, Some("no system speech voice")),
            kokoro: None,
            status: SpeechStatus {
                message: "Text-to-speech is not available on this system".into(),
                progress: None,
                ready: false,
                error: Some("no system speech voice".into()),
            },
            enabled: false,
            rate: 1.0,
            tone: CallTone::default(),
            deliveries: contra_core::DEFAULT_TTS_DELIVERIES,
            delivery_cursor: HashMap::new(),
            last_calls: Vec::new(),
            #[cfg(test)]
            neural_bytes_override: None,
            #[cfg(test)]
            speech_bytes_override: None,
            voice_id: DEFAULT_VOICE_ID.to_string(),
            pending_rate: None,
            pending_kokoro_text: None,
        }
    }

    pub fn available(&self) -> bool {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            self.platform.available() || (self.kokoro.is_some() && self.status.error.is_none())
        }
        #[cfg(any(target_arch = "wasm32", target_os = "android"))]
        {
            self.platform.available()
        }
    }

    /// Estimated memory of the loaded neural voice, or zero when it is off.
    ///
    /// Enabling Speak calls starts the worker loading the quantized Kokoro
    /// model into ONNX Runtime, which is by far the largest optional
    /// allocation in the app. The platform voice uses no app memory.
    pub fn neural_working_set_bytes(&self) -> u64 {
        #[cfg(test)]
        if let Some(bytes) = self.neural_bytes_override {
            return if self.enabled { bytes } else { 0 };
        }
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            if self.enabled && self.kokoro.is_some() {
                return kokoro::working_set_bytes();
            }
        }
        0
    }

    /// Stand in for a loaded neural model in memory-report tests.
    #[cfg(test)]
    pub fn set_neural_bytes_for_test(&mut self, bytes: u64) {
        self.neural_bytes_override = Some(bytes);
    }

    /// Bytes held by the prepared call deliveries, or zero when speech is off.
    ///
    /// The buffers live on the Kokoro worker; it publishes their total after
    /// every cache change so the Preferences memory table can show them
    /// without blocking on the worker.
    pub fn speech_cache_bytes(&self) -> u64 {
        #[cfg(test)]
        if let Some(bytes) = self.speech_bytes_override {
            return if self.enabled { bytes } else { 0 };
        }
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            if self.enabled
                && let Some(kokoro) = &self.kokoro
            {
                return kokoro.cache_bytes();
            }
        }
        0
    }

    /// Stand in for a prepared-call cache in memory-report tests.
    #[cfg(test)]
    pub fn set_speech_bytes_for_test(&mut self, bytes: u64) {
        self.speech_bytes_override = Some(bytes);
    }

    pub fn error(&self) -> Option<&str> {
        self.status
            .error
            .as_deref()
            .or_else(|| self.platform.error())
    }

    pub fn status(&self) -> &SpeechStatus {
        &self.status
    }

    /// Whether speech was switched on, for the state fingerprint.
    #[cfg(test)]
    pub fn enabled_for_test(&self) -> bool {
        self.enabled
    }

    /// Number of prepared call texts.
    #[cfg(test)]
    pub fn prepared_calls(&self) -> usize {
        self.last_calls.len()
    }

    /// Delivery rate and tone, for the state fingerprint.
    #[cfg(test)]
    pub fn rate_and_tone(&self) -> (f32, CallTone) {
        (self.rate, self.tone)
    }

    /// Prepared deliveries per call, for the state fingerprint.
    #[cfg(test)]
    pub fn deliveries_for_test(&self) -> u8 {
        self.deliveries
    }

    pub fn can_retry(&self) -> bool {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            self.kokoro.is_some()
        }
        #[cfg(any(target_arch = "wasm32", target_os = "android"))]
        {
            false
        }
    }

    /// Select a neural voice. The worker downloads the small style file if it
    /// is not cached, clears its synthesized-call cache, and reloads.
    pub fn set_voice(&mut self, id: &str) {
        let voice = crate::tts::voice(id);
        if self.voice_id == voice.id {
            return;
        }
        self.voice_id = voice.id.to_string();
        self.delivery_cursor.clear();
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(kokoro) = &self.kokoro {
            if self.enabled {
                self.status.ready = false;
                self.status.progress = Some(0.0);
                self.status.message = format!("Preparing {}", voice.label);
            }
            kokoro.set_voice(voice.id);
            if self.enabled {
                kokoro.prewarm(self.last_calls.clone());
            }
        }
    }

    /// Set how many deliveries each call text is prepared with.
    ///
    /// The worker drops the in-memory buffers and reloads or re-synthesizes
    /// the new set; the on-disk cache keeps work that is still valid.
    pub fn set_deliveries(&mut self, deliveries: u8) {
        let deliveries = deliveries.clamp(
            contra_core::MIN_TTS_DELIVERIES,
            contra_core::MAX_TTS_DELIVERIES,
        );
        if self.deliveries == deliveries {
            return;
        }
        self.deliveries = deliveries;
        self.delivery_cursor.clear();
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(kokoro) = &self.kokoro {
            kokoro.set_deliveries(deliveries);
        }
    }

    pub fn is_busy(&self) -> bool {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            self.pending_rate.is_some()
                || (self.enabled
                    && self.status.error.is_none()
                    && (self.status.progress.is_some() || !self.status.ready))
        }
        #[cfg(any(target_arch = "wasm32", target_os = "android"))]
        {
            false
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        if enabled {
            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            if self.status.error.is_some() {
                self.retry();
            } else if let Some(kokoro) = &self.kokoro {
                kokoro.enable(true);
                kokoro.prewarm(self.last_calls.clone());
            }
        } else {
            self.delivery_cursor.clear();
            // Release the model and the prepared deliveries, as the memory
            // table and the documentation promise. Re-enabling reloads them.
            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            if let Some(kokoro) = &self.kokoro {
                kokoro.enable(false);
            }
            self.stop();
        }
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.clamp(
            contra_core::MIN_TTS_RATE * contra_core::TTS_RATE_SCALE,
            contra_core::MAX_TTS_RATE * contra_core::TTS_RATE_SCALE,
        );
        self.delivery_cursor.clear();
        self.platform.set_rate(self.rate);
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            self.pending_rate = Some(Instant::now());
        }
    }

    /// Change the delivery tone. A tone change invalidates synthesized calls
    /// because the pitch is baked into the audio, then re-prepares them so the
    /// first call after the change already uses the new voice.
    pub fn set_tone(&mut self, tone: CallTone) {
        if self.tone == tone {
            return;
        }
        self.tone = tone;
        self.delivery_cursor.clear();
        self.platform.set_tone(tone);
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(kokoro) = &self.kokoro {
            kokoro.set_tone(tone);
            if self.enabled {
                kokoro.prewarm(self.last_calls.clone());
            }
        }
    }

    /// Give the worker the current call list so it can synthesize ahead of
    /// playback. Safe to call while speech is disabled; the list is used when
    /// the user enables speech or when the voice finishes loading.
    pub fn prewarm(&mut self, calls: Vec<String>) {
        let mut unique = Vec::with_capacity(calls.len());
        for text in calls {
            let text = pronounce::kokoro_text(&text);
            if !text.is_empty() && !unique.contains(&text) {
                unique.push(text);
            }
        }
        if unique != self.last_calls {
            // A new call set starts its rotation at the base delivery again.
            self.delivery_cursor.clear();
        }
        self.last_calls = unique;
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if self.enabled
            && let Some(kokoro) = &self.kokoro
        {
            kokoro.prewarm(self.last_calls.clone());
        }
    }

    pub fn speak(&mut self, text: &str) {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if self.enabled && self.status.ready && self.status.error.is_none() && self.kokoro.is_some()
        {
            self.pending_kokoro_text = Some(text.to_string());
            let delivery = next_delivery(&mut self.delivery_cursor, text, self.deliveries);
            if let Some(kokoro) = &self.kokoro {
                kokoro.speak(text, delivery);
            }
            return;
        }
        self.platform.speak(&pronounce::platform_text(text));
    }

    pub fn stop(&mut self) {
        self.platform.stop();
        self.status.progress = None;
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            self.pending_kokoro_text = None;
            if let Some(kokoro) = &self.kokoro {
                kokoro.stop();
            }
        }
    }

    /// Apply completed synthesis and status messages. Call once per UI frame.
    pub fn poll(&mut self, audio: &AudioEngine) {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(since) = self.pending_rate
            && since.elapsed() >= Duration::from_millis(250)
        {
            self.pending_rate = None;
            if let Some(kokoro) = &self.kokoro {
                kokoro.set_rate(self.rate);
                if self.enabled {
                    kokoro.prewarm(self.last_calls.clone());
                }
            }
        }
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(kokoro) = &self.kokoro {
            while let Some(event) = kokoro.try_recv() {
                match event {
                    KokoroEvent::Status {
                        message,
                        progress,
                        ready,
                    } => {
                        self.status.message = message;
                        self.status.progress = progress;
                        self.status.ready = ready;
                        if ready {
                            self.status.error = None;
                        }
                    }
                    KokoroEvent::Audio(samples) => {
                        self.pending_kokoro_text = None;
                        audio.play_speech(samples);
                    }
                    KokoroEvent::Failed(error) => {
                        self.status.ready = false;
                        self.status.progress = None;
                        self.status.message = "Using the platform voice".into();
                        self.status.error = Some(error);
                        if let Some(text) = self.pending_kokoro_text.take() {
                            self.platform.speak(&pronounce::platform_text(&text));
                        }
                    }
                }
            }
        }
        let _ = audio;
    }

    /// Retry a failed neural-voice download or model load.
    pub fn retry(&mut self) {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(kokoro) = &self.kokoro {
            self.status.error = None;
            self.status.message = "Preparing the neural voice".into();
            self.status.ready = false;
            kokoro.retry(self.enabled, self.last_calls.clone());
        }
    }
}

/// The operating system's text-to-speech backend.
struct PlatformSpeech {
    tts: Option<tts::Tts>,
    error: Option<String>,
    rate: f32,
    tone: CallTone,
    #[cfg(any(feature = "headless", test))]
    available_override: Option<bool>,
}

impl PlatformSpeech {
    fn new() -> Self {
        let mut speech = Self {
            tts: None,
            error: None,
            rate: 1.0,
            tone: CallTone::default(),
            #[cfg(any(feature = "headless", test))]
            available_override: None,
        };
        match tts::Tts::default() {
            Ok(tts) => {
                speech.tts = Some(tts);
                speech.apply_rate();
                speech.apply_tone();
            }
            Err(error) => speech.error = Some(error.to_string()),
        }
        speech
    }

    /// A no-op platform backend for headless screenshots.
    #[cfg(any(feature = "headless", test))]
    fn stub(available: bool, error: Option<&str>) -> Self {
        Self {
            tts: None,
            error: error.map(str::to_string),
            rate: 1.0,
            tone: CallTone::default(),
            available_override: Some(available),
        }
    }

    fn available(&self) -> bool {
        #[cfg(any(feature = "headless", test))]
        if let Some(available) = self.available_override {
            return available;
        }
        self.tts.is_some()
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn set_rate(&mut self, rate: f32) {
        self.rate = rate.clamp(
            contra_core::MIN_TTS_RATE * contra_core::TTS_RATE_SCALE,
            contra_core::MAX_TTS_RATE * contra_core::TTS_RATE_SCALE,
        );
        self.apply_rate();
    }

    fn set_tone(&mut self, tone: CallTone) {
        if self.tone == tone {
            return;
        }
        self.tone = tone;
        self.apply_tone();
    }

    fn speak(&mut self, text: &str) {
        if let Some(tts) = &mut self.tts {
            let _ = tts.speak(text.to_string(), true);
        }
    }

    fn stop(&mut self) {
        if let Some(tts) = &mut self.tts {
            let _ = tts.stop();
        }
    }

    fn apply_rate(&mut self) {
        if let Some(tts) = &mut self.tts {
            let _ = tts.set_rate(self.rate);
            let _ = tts.set_volume(1.0);
        }
    }

    fn apply_tone(&mut self) {
        if let Some(tts) = &mut self.tts {
            let normal = tts.normal_pitch();
            let max = tts.max_pitch();
            let _ = tts.set_pitch(platform_pitch(normal, max, self.tone));
        }
    }
}

/// Pick the next prepared delivery for a call text and advance its rotation.
///
/// The counter lives per text, so a dancer hears every delivery of a repeated
/// call before any of them repeats. With one delivery the result is always
/// zero, which is the old single-rendering behaviour.
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn next_delivery(cursor: &mut HashMap<String, u8>, text: &str, deliveries: u8) -> u8 {
    let deliveries = deliveries.max(1);
    let next = cursor.entry(text.to_string()).or_insert(0);
    let delivery = *next % deliveries;
    *next = next.wrapping_add(1);
    delivery
}

/// Translate a [`CallTone`] into a platform TTS pitch value.
///
/// Backends disagree on the scale (WinRT and Android use roughly 0..2, Speech
/// Dispatcher uses -100..100), but all report a normal pitch and an upper
/// bound. Moving the same fraction of the way down from normal gives a similar
/// drop on each backend.
fn platform_pitch(normal: f32, max: f32, tone: CallTone) -> f32 {
    if tone == CallTone::Natural {
        return normal;
    }
    let headroom = (max - normal).max(0.0);
    normal - headroom * (1.0 - tone.pitch())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn voice_catalog_is_valid() {
        let mut ids = BTreeSet::new();
        for voice in VOICES {
            assert!(ids.insert(voice.id), "duplicate voice id {}", voice.id);
            assert!(voice.url.starts_with("https://"), "{}", voice.id);
            assert_eq!(voice.sha256.len(), 64, "{}", voice.id);
            assert!(voice.file.ends_with(".bin"), "{}", voice.id);
            assert_eq!(voice.size, 522_240, "{}", voice.id);
        }
        assert!(ids.contains(DEFAULT_VOICE_ID));
    }

    #[test]
    fn unknown_voice_falls_back_to_the_default() {
        assert_eq!(voice("not-a-voice").id, DEFAULT_VOICE_ID);
        assert_eq!(voice(DEFAULT_VOICE_ID).label, "Heart (US female)");
    }

    #[test]
    fn deliveries_rotate_per_call_text() {
        let mut cursor = HashMap::new();
        let seen: Vec<u8> = (0..9)
            .map(|_| next_delivery(&mut cursor, "swing your partner", 4))
            .collect();
        assert_eq!(seen, vec![0, 1, 2, 3, 0, 1, 2, 3, 0]);
        assert_eq!(
            next_delivery(&mut cursor, "balance the ring", 4),
            0,
            "each call text has its own rotation"
        );

        let mut single = HashMap::new();
        for _ in 0..3 {
            assert_eq!(next_delivery(&mut single, "swing", 1), 0);
        }
    }

    #[test]
    fn delivery_count_is_clamped_by_the_coordinator() {
        let mut speech = Speech::stub();
        speech.set_deliveries(9);
        assert_eq!(
            speech.deliveries_for_test(),
            contra_core::MAX_TTS_DELIVERIES
        );
        speech.set_deliveries(0);
        assert_eq!(
            speech.deliveries_for_test(),
            contra_core::MIN_TTS_DELIVERIES
        );
    }

    #[test]
    fn commanding_tone_lowers_the_platform_pitch() {
        // WinRT and Android style ranges.
        assert_eq!(platform_pitch(1.0, 2.0, CallTone::Natural), 1.0);
        let lower = platform_pitch(1.0, 2.0, CallTone::Commanding);
        assert!(lower < 1.0, "{lower}");
        assert!((lower - 0.94).abs() < 1e-6, "{lower}");

        // Speech Dispatcher uses -100..100 with normal at zero.
        let linux = platform_pitch(0.0, 100.0, CallTone::Commanding);
        assert!((linux + 6.0).abs() < 1e-6, "{linux}");

        // A backend without headroom must not be pushed out of range.
        assert_eq!(platform_pitch(0.5, 0.5, CallTone::Commanding), 0.5);
    }
}
