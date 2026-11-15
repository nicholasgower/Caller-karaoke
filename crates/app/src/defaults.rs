//! Bundled defaults so the app works on first launch without the network.

use contra_core::{Dance, SourceRef, Tune};

const DEFAULT_DANCE_JSON: &str = include_str!("../../../assets/defaults/another_last_dance.json");
const DEFAULT_ABC: &str = include_str!("../../../assets/defaults/sandy_boys.abc");

pub fn default_dance() -> Dance {
    serde_json::from_str(DEFAULT_DANCE_JSON).expect("bundled dance JSON is valid")
}

pub fn default_tune() -> Tune {
    Tune {
        source: SourceRef::new(
            "abcsessiontunes",
            "75815",
            "https://abcsessiontunes.com/tunes/75815",
        ),
        title: "Sandy Boys".into(),
        composer: String::new(),
        key: "A mix".into(),
        meter: "4/4".into(),
        rhythm: "reel".into(),
        abc: DEFAULT_ABC.to_string(),
        credits: "ABC transcription: Josh Larios · chords from Sarah Comer".into(),
    }
}
