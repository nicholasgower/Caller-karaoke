//! Optional text-to-speech for calls.

pub struct Speech {
    tts: Option<tts::Tts>,
    error: Option<String>,
    rate: f32,
}

impl Speech {
    pub fn new() -> Self {
        let mut speech = Self {
            tts: None,
            error: None,
            rate: 1.1,
        };
        match tts::Tts::default() {
            Ok(tts) => {
                speech.tts = Some(tts);
                speech.apply_rate();
            }
            Err(error) => {
                speech.error = Some(error.to_string());
            }
        }
        speech
    }

    pub fn available(&self) -> bool {
        self.tts.is_some()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.clamp(0.5, 2.0);
        self.apply_rate();
    }

    pub fn speak(&mut self, text: &str) {
        if let Some(tts) = &mut self.tts {
            let _ = tts.speak(text.to_string(), true);
        }
    }

    pub fn stop(&mut self) {
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

impl Default for Speech {
    fn default() -> Self {
        Self::new()
    }
}
