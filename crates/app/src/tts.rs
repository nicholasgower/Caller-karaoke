//! Optional speech for calls.
//!
//! On desktop the preferred backend is a local Kokoro neural voice (see
//! [`kokoro`]). The platform TTS backend remains available as a fallback so
//! the app still speaks calls on first run, offline, or on web/Android where
//! the neural model is not built. Both backends receive the reviewed dance-term
//! respellings from [`contra_core::pronounce`].

use std::path::PathBuf;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
use std::time::{Duration, Instant};

use crate::audio::AudioEngine;
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
        id: "af_nicole",
        label: "Nicole (US female)",
        detail: "Soft American English female voice.",
        file: "af_nicole.bin",
        url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/voices/af_nicole.bin",
        sha256: "cd2191ab31b914ed7b318416b0e4440fdf392ddad9106a060819aa600a64f59a",
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
    last_calls: Vec<String>,
    voice_id: String,
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    pending_rate: Option<Instant>,
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    pending_kokoro_text: Option<String>,
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
                last_calls: Vec::new(),
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
                last_calls: Vec::new(),
                voice_id: DEFAULT_VOICE_ID.to_string(),
            }
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

    pub fn error(&self) -> Option<&str> {
        self.status
            .error
            .as_deref()
            .or_else(|| self.platform.error())
    }

    pub fn status(&self) -> &SpeechStatus {
        &self.status
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
        if !enabled {
            self.stop();
            return;
        }
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if self.status.error.is_some() {
            self.retry();
        } else if let Some(kokoro) = &self.kokoro {
            kokoro.enable(true);
            kokoro.prewarm(self.last_calls.clone());
        }
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.clamp(0.5, 2.0);
        self.platform.set_rate(self.rate);
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            self.pending_rate = Some(Instant::now());
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
            if let Some(kokoro) = &self.kokoro {
                kokoro.speak(text);
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
}

impl PlatformSpeech {
    fn new() -> Self {
        let mut speech = Self {
            tts: None,
            error: None,
            rate: 1.0,
        };
        match tts::Tts::default() {
            Ok(tts) => {
                speech.tts = Some(tts);
                speech.apply_rate();
            }
            Err(error) => speech.error = Some(error.to_string()),
        }
        speech
    }

    fn available(&self) -> bool {
        self.tts.is_some()
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn set_rate(&mut self, rate: f32) {
        self.rate = rate.clamp(0.5, 2.0);
        self.apply_rate();
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
}
