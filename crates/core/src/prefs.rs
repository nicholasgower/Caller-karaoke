//! Persisted application preferences.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::alias::AliasTable;
use crate::dance::SourceRef;
use crate::effects::AudioEffects;
use crate::roles::{RolePreset, RoleTerms};
use crate::time::Tempo;
use crate::timing::LeadSettings;

/// Factor from the displayed speech rate to the TTS engine's rate.
///
/// The UI keeps the familiar 1.0 default while the engine speaks at 1.2, which
/// is the delivery the calls were calibrated against.
pub const TTS_RATE_SCALE: f32 = 1.2;

/// Slowest speech rate the preferences expose, in displayed units.
pub const MIN_TTS_RATE: f32 = 0.4;

/// Fastest speech rate the preferences expose, in displayed units.
///
/// The old ceiling was 1.5 engine units; on the displayed scale that is
/// 1.25, rounded to one decimal place.
pub const MAX_TTS_RATE: f32 = 1.3;

/// The instrument that plays the tune.
///
/// The two original contra voices (fiddle and accordion) are joined by the
/// single-reed winds a pickup band often brings: alto sax, tenor sax, and
/// clarinet. Every choice has a General MIDI program in the bundled subset, so
/// the list only grows when the font carries the new program; the keep-list in
/// `scripts/soundfont-keep.txt` is the other half of that change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MelodyInstrument {
    #[default]
    Fiddle,
    Accordion,
    AltoSax,
    TenorSax,
    Clarinet,
}

impl MelodyInstrument {
    pub const ALL: [Self; 5] = [
        Self::Fiddle,
        Self::Accordion,
        Self::AltoSax,
        Self::TenorSax,
        Self::Clarinet,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Fiddle => "Fiddle",
            Self::Accordion => "Accordion",
            Self::AltoSax => "Alto sax",
            Self::TenorSax => "Tenor sax",
            Self::Clarinet => "Clarinet",
        }
    }

    /// General MIDI program number (zero-based) used with a loaded soundfont.
    pub fn gm_program(self) -> u8 {
        match self {
            Self::Fiddle => 40,    // Violin
            Self::Accordion => 21, // Accordion
            Self::AltoSax => 65,   // Alto Sax
            Self::TenorSax => 66,  // Tenor Sax
            Self::Clarinet => 71,  // Clarinet
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccompanimentInstrument {
    #[default]
    Piano,
    Guitar,
    Accordion,
}

impl AccompanimentInstrument {
    pub const ALL: [Self; 3] = [Self::Piano, Self::Guitar, Self::Accordion];

    pub fn label(self) -> &'static str {
        match self {
            Self::Piano => "Piano",
            Self::Guitar => "Guitar",
            Self::Accordion => "Accordion",
        }
    }

    /// General MIDI program number (zero-based) used with a loaded soundfont.
    pub fn gm_program(self) -> u8 {
        match self {
            Self::Piano => 0,      // Acoustic Grand Piano
            Self::Guitar => 25,    // Acoustic Guitar (steel)
            Self::Accordion => 21, // Accordion
        }
    }
}

/// The countermelody voice, which turns the duo into a trio.
///
/// The melody and [`AccompanimentInstrument`] provide two voices. The
/// countermelody is played by a melody instrument underneath the tune, so the
/// default session sounds like a small contra band rather than a solo with a
/// backing track. Set it to [`Self::Off`] for a duo.
///
/// This type was called `SecondAccompaniment`. The preference field is still
/// [`Preferences::second_accompaniment`] and the serialized values are
/// unchanged (`"Accordion"`, `"Fiddle"`, `"Off"`), so older preference files
/// keep loading and the rename needs no migration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Countermelody {
    /// An accordion countermelody under the tune (the default trio).
    #[default]
    Accordion,
    /// A second fiddle playing the countermelody.
    Fiddle,
    /// No countermelody; play the melody and one accompaniment.
    Off,
}

impl Countermelody {
    /// Every choice, in display order.
    pub const ALL: [Self; 3] = [Self::Accordion, Self::Fiddle, Self::Off];

    pub fn label(self) -> &'static str {
        match self {
            Self::Accordion => "Accordion",
            Self::Fiddle => "Fiddle",
            Self::Off => "Off",
        }
    }

    /// The melody instrument that plays the countermelody, if it is enabled.
    pub fn instrument(self) -> Option<MelodyInstrument> {
        match self {
            Self::Accordion => Some(MelodyInstrument::Accordion),
            Self::Fiddle => Some(MelodyInstrument::Fiddle),
            Self::Off => None,
        }
    }

    /// General MIDI program for the countermelody voice, if it is enabled.
    pub fn gm_program(self) -> Option<u8> {
        self.instrument().map(MelodyInstrument::gm_program)
    }
}

/// How many players each part of the virtual band has.
///
/// Contra bands thicken a line by doubling it: two fiddles, a piano and a
/// guitar, two accordions. Every copy is rendered as its own player with a
/// small independent pan, a few milliseconds of timing spread, and a slight
/// tuning or velocity difference, so a section sounds wider rather than merely
/// louder.
///
/// One is the single-player default, so an existing session sounds exactly the
/// same until the user adds a copy. Files written before this setting existed
/// get the default through `serde`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InstrumentCopies {
    /// Players on the melody line.
    pub melody: u8,
    /// Players on the chordal accompaniment.
    pub accompaniment: u8,
    /// Players on the countermelody.
    pub countermelody: u8,
    /// Players on the optional percussion layer.
    pub percussion: u8,
}

impl Default for InstrumentCopies {
    fn default() -> Self {
        Self {
            melody: 1,
            accompaniment: 1,
            countermelody: 1,
            percussion: 1,
        }
    }
}

impl InstrumentCopies {
    /// Fewest players a part can have.
    pub const MIN: u8 = 1;
    /// Most players a melodic part can have.
    ///
    /// Four keeps the three melodic parts inside the fifteen General MIDI
    /// channels that are not the drum channel.
    pub const MAX: u8 = 4;
    /// Most players the percussion layer can have.
    pub const MAX_PERCUSSION: u8 = 3;

    /// Clamp hand-edited or older values into the supported range.
    pub fn validate(&mut self) {
        self.melody = self.melody.clamp(Self::MIN, Self::MAX);
        self.accompaniment = self.accompaniment.clamp(Self::MIN, Self::MAX);
        self.countermelody = self.countermelody.clamp(Self::MIN, Self::MAX);
        self.percussion = self.percussion.clamp(Self::MIN, Self::MAX_PERCUSSION);
    }

    /// Melody players, clamped for a renderer.
    pub fn melody_count(self) -> usize {
        usize::from(self.melody.clamp(Self::MIN, Self::MAX))
    }

    /// Accompaniment players, clamped for a renderer.
    pub fn accompaniment_count(self) -> usize {
        usize::from(self.accompaniment.clamp(Self::MIN, Self::MAX))
    }

    /// Countermelody players, clamped for a renderer.
    pub fn countermelody_count(self) -> usize {
        usize::from(self.countermelody.clamp(Self::MIN, Self::MAX))
    }

    /// Percussion players, clamped for a renderer.
    pub fn percussion_count(self) -> usize {
        usize::from(self.percussion.clamp(Self::MIN, Self::MAX_PERCUSSION))
    }
}

/// Optional percussion layer under the band.
///
/// Percussion is off by default. When it is enabled, a soft foot tap on every
/// dance beat is the default; a simple drum kit is the alternative. Either way
/// it sits on the same dance-beat grid as the music and adds no timing. The
/// separate [`PercussionRhythm`] decides the pattern inside that grid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Percussion {
    /// No percussion (default).
    #[default]
    Off,
    /// A soft foot tap on each dance beat.
    FootTaps,
    /// A basic kit: kick, snare, and hi-hat.
    Drumset,
}

impl Percussion {
    /// Every choice, in display order.
    pub const ALL: [Self; 3] = [Self::Off, Self::FootTaps, Self::Drumset];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::FootTaps => "Foot taps",
            Self::Drumset => "Drumset",
        }
    }

    /// Whether the layer plays at all.
    pub fn is_enabled(self) -> bool {
        !matches!(self, Self::Off)
    }
}

/// Rhythm used to place the optional percussion layer.
///
/// Like the accompaniment rhythms, these only decide when the percussion
/// sounds inside the 64-beat grid; they never move the grid, the tempo, or the
/// call timeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PercussionRhythm {
    /// Use the pattern that suits the selected percussion style.
    #[default]
    Auto,
    /// The style's basic pattern: foot taps on each beat, or a kick/snare/hat kit.
    Basic,
    /// One accent on each dance beat.
    Beat,
    /// Accents between the dance beats.
    Offbeat,
    /// One accent per bar, with a sparse kit.
    HalfTime,
    /// Two accents per dance beat.
    DoubleTime,
    /// A swung long-short feel.
    Shuffle,
    /// A one-and-a gallop: an eighth note and two sixteenths, repeated.
    Gallop,
    /// Three even taps on every dance beat.
    Triplet,
    /// Alternating accents in steady eighth notes.
    Train,
}

impl PercussionRhythm {
    pub const ALL: [Self; 10] = [
        Self::Auto,
        Self::Basic,
        Self::Beat,
        Self::Offbeat,
        Self::HalfTime,
        Self::DoubleTime,
        Self::Shuffle,
        Self::Gallop,
        Self::Triplet,
        Self::Train,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Match style",
            Self::Basic => "Basic",
            Self::Beat => "On the beat",
            Self::Offbeat => "Offbeats",
            Self::HalfTime => "Half time",
            Self::DoubleTime => "Double time",
            Self::Shuffle => "Shuffle",
            Self::Gallop => "Gallop",
            Self::Triplet => "Triplet",
            Self::Train => "Train beat",
        }
    }
}

/// Rhythm pattern used to realise the accompaniment chords.
///
/// The pattern only decides when the chord and bass notes sound inside the
/// 64-beat grid; it never changes the grid, the tempo, or the call timeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccompanimentRhythm {
    /// Use the pattern that suits the selected accompaniment instrument.
    #[default]
    Auto,
    /// One sustained chord and bass note per dance beat.
    Block,
    /// Alternate bass and chord on each half beat (`oom-pah`).
    BoomChuck,
    /// Hold the bass and stab the chord off the beat.
    Offbeat,
    /// Broken-chord pattern in steady eighth notes.
    Arpeggio,
    /// Swung boom-chuck with a long-short feel.
    Shuffle,
    /// Alternating down and up strums on every half beat.
    Strum,
    /// One chord per harmony, held as long as the changes allow.
    Sustained,
}

impl AccompanimentRhythm {
    pub const ALL: [Self; 8] = [
        Self::Auto,
        Self::Block,
        Self::BoomChuck,
        Self::Offbeat,
        Self::Arpeggio,
        Self::Shuffle,
        Self::Strum,
        Self::Sustained,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Match instrument",
            Self::Block => "Block chords",
            Self::BoomChuck => "Boom-chuck",
            Self::Offbeat => "Offbeat stabs",
            Self::Arpeggio => "Arpeggio",
            Self::Shuffle => "Shuffle",
            Self::Strum => "Strum",
            Self::Sustained => "Sustained",
        }
    }
}

/// One part of the virtual band.
///
/// The band is a list of instrument groups (see [`BandSlot`]), not a fixed set
/// of four roles, so a user can add two fiddles, a piano, a guitar, and a
/// rhythm section without one replacing another.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BandPart {
    /// The tune itself.
    #[default]
    Melody,
    /// A second melodic line under the tune.
    Countermelody,
    /// Chords and bass.
    Accompaniment,
    /// Foot taps or a drum kit.
    Percussion,
}

impl BandPart {
    /// Every part, in display order.
    pub const ALL: [Self; 4] = [
        Self::Melody,
        Self::Countermelody,
        Self::Accompaniment,
        Self::Percussion,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Melody => "Melody",
            Self::Countermelody => "Countermelody",
            Self::Accompaniment => "Accompaniment",
            Self::Percussion => "Percussion",
        }
    }
}

/// An instrument a [`BandSlot`] can use.
///
/// This is a superset of [`MelodyInstrument`] and
/// [`AccompanimentInstrument`], because a band list can put any voice on any
/// part (a clarinet countermelody, two guitars, a fiddle playing chords).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BandVoice {
    #[default]
    Fiddle,
    Accordion,
    AltoSax,
    TenorSax,
    Clarinet,
    Piano,
    Guitar,
    FootTaps,
    Drumset,
}

impl BandVoice {
    /// Every voice, in display order.
    pub const ALL: [Self; 9] = [
        Self::Fiddle,
        Self::Accordion,
        Self::AltoSax,
        Self::TenorSax,
        Self::Clarinet,
        Self::Piano,
        Self::Guitar,
        Self::FootTaps,
        Self::Drumset,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Fiddle => "Fiddle",
            Self::Accordion => "Accordion",
            Self::AltoSax => "Alto sax",
            Self::TenorSax => "Tenor sax",
            Self::Clarinet => "Clarinet",
            Self::Piano => "Piano",
            Self::Guitar => "Guitar",
            Self::FootTaps => "Foot taps",
            Self::Drumset => "Drumset",
        }
    }

    /// General MIDI program for pitched voices.
    ///
    /// Percussion voices return `None`: they play on the GM drum channel and
    /// carry no program change.
    pub fn gm_program(self) -> Option<u8> {
        match self {
            Self::Fiddle => Some(40),    // Violin
            Self::Accordion => Some(21), // Accordion
            Self::AltoSax => Some(65),   // Alto Sax
            Self::TenorSax => Some(66),  // Tenor Sax
            Self::Clarinet => Some(71),  // Clarinet
            Self::Piano => Some(0),      // Acoustic Grand Piano
            Self::Guitar => Some(25),    // Acoustic Guitar (steel)
            Self::FootTaps | Self::Drumset => None,
        }
    }

    /// Whether this voice is a percussion layer rather than a pitched part.
    pub fn is_percussion(self) -> bool {
        matches!(self, Self::FootTaps | Self::Drumset)
    }

    /// The melody instrument closest to this voice, for code that still works
    /// with the original three-role model.
    pub fn melody_instrument(self) -> Option<MelodyInstrument> {
        match self {
            Self::Fiddle => Some(MelodyInstrument::Fiddle),
            Self::Accordion => Some(MelodyInstrument::Accordion),
            Self::AltoSax => Some(MelodyInstrument::AltoSax),
            Self::TenorSax => Some(MelodyInstrument::TenorSax),
            Self::Clarinet => Some(MelodyInstrument::Clarinet),
            _ => None,
        }
    }

    /// The melody voice for an original melody-instrument choice.
    pub fn from_melody(instrument: MelodyInstrument) -> Self {
        match instrument {
            MelodyInstrument::Fiddle => Self::Fiddle,
            MelodyInstrument::Accordion => Self::Accordion,
            MelodyInstrument::AltoSax => Self::AltoSax,
            MelodyInstrument::TenorSax => Self::TenorSax,
            MelodyInstrument::Clarinet => Self::Clarinet,
        }
    }

    /// The band voice for an original accompaniment-instrument choice.
    pub fn from_accompaniment(instrument: AccompanimentInstrument) -> Self {
        match instrument {
            AccompanimentInstrument::Piano => Self::Piano,
            AccompanimentInstrument::Guitar => Self::Guitar,
            AccompanimentInstrument::Accordion => Self::Accordion,
        }
    }

    /// The band voice for the legacy percussion style.
    pub fn from_percussion(style: Percussion) -> Option<Self> {
        match style {
            Percussion::Off => None,
            Percussion::FootTaps => Some(Self::FootTaps),
            Percussion::Drumset => Some(Self::Drumset),
        }
    }

    /// The accompaniment instrument whose chord pattern this voice uses.
    ///
    /// A sustained voice (fiddle, clarinet, sax) comps like an accordion; the
    /// arrangement only uses this to choose its `Auto` rhythm. The user's
    /// explicit rhythm still wins.
    pub fn accompaniment_instrument(self) -> Option<AccompanimentInstrument> {
        match self {
            Self::Piano => Some(AccompanimentInstrument::Piano),
            Self::Guitar => Some(AccompanimentInstrument::Guitar),
            Self::Accordion | Self::Fiddle | Self::AltoSax | Self::TenorSax | Self::Clarinet => {
                Some(AccompanimentInstrument::Accordion)
            }
            Self::FootTaps | Self::Drumset => None,
        }
    }

    /// The percussion style this voice plays, if it is a percussion voice.
    pub fn percussion_style(self) -> Option<Percussion> {
        match self {
            Self::FootTaps => Some(Percussion::FootTaps),
            Self::Drumset => Some(Percussion::Drumset),
            _ => None,
        }
    }

    /// Whether a slot with this voice contributes pitched notes.
    pub fn is_pitched(self) -> bool {
        !self.is_percussion()
    }
}

/// One line of the user's band: an instrument, how many players, and where it
/// stands in the stereo or surround image.
///
/// A slot is not a copy switch: two slots can use the same voice for different
/// parts (a fiddle melody and a fiddle countermelody), and a single slot can
/// carry several players who are rendered as an ensemble with a small spread.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BandSlot {
    pub part: BandPart,
    pub voice: BandVoice,
    /// Players in this group, 1-4 for pitched parts and 1-3 for percussion.
    pub players: u8,
    /// Mix level for this group, 0.0-2.0.
    pub volume: f32,
    /// Stereo/surround direction: -1 hard left, 0 centre, 1 hard right.
    pub pan: f32,
    /// Distance from the front of the room: 0 front, 1 rear. Used by the
    /// surround layout; the stereo layout folds it into a slight level trim.
    pub depth: f32,
}

impl Default for BandSlot {
    fn default() -> Self {
        Self {
            part: BandPart::Melody,
            voice: BandVoice::Fiddle,
            players: 1,
            volume: 0.75,
            pan: 0.0,
            depth: 0.0,
        }
    }
}

impl BandSlot {
    /// Fewest players a group can have.
    pub const MIN_PLAYERS: u8 = 1;
    /// Most players a pitched group can have.
    pub const MAX_PLAYERS: u8 = 4;
    /// Most players a percussion group can have.
    pub const MAX_PERCUSSION_PLAYERS: u8 = 3;

    /// Clamp hand-edited values into the supported range.
    pub fn validate(&mut self) {
        if self.voice.is_percussion() {
            self.part = BandPart::Percussion;
        }
        let max = if self.voice.is_percussion() {
            Self::MAX_PERCUSSION_PLAYERS
        } else {
            Self::MAX_PLAYERS
        };
        self.players = self.players.clamp(Self::MIN_PLAYERS, max);
        if !self.volume.is_finite() {
            self.volume = 0.7;
        }
        self.volume = self.volume.clamp(0.0, 2.0);
        if !self.pan.is_finite() {
            self.pan = 0.0;
        }
        self.pan = self.pan.clamp(-1.0, 1.0);
        if !self.depth.is_finite() {
            self.depth = 0.0;
        }
        self.depth = self.depth.clamp(0.0, 1.0);
    }

    /// Players clamped for a renderer.
    pub fn player_count(&self) -> usize {
        usize::from(self.players.clamp(
            Self::MIN_PLAYERS,
            if self.voice.is_percussion() {
                Self::MAX_PERCUSSION_PLAYERS
            } else {
                Self::MAX_PLAYERS
            },
        ))
    }

    /// Whether this slot contributes pitched notes rather than percussion.
    pub fn is_pitched(&self) -> bool {
        self.voice.is_pitched()
    }

    /// Whether this slot sits behind the listener in a surround layout.
    ///
    /// A slot only reads as rear once it is meaningfully behind the front
    /// stage; the default band's small depth offsets stay "front" in the label
    /// while still giving the rear pair a little of the sound.
    pub fn is_rear(&self) -> bool {
        self.depth >= 0.3
    }

    /// A compact direction label for the session summary and the UI.
    ///
    /// `pan` names the side and `depth` names front or rear; a slot in the
    /// middle reads "Centre front" or "Centre rear" rather than "middle".
    pub fn direction_label(&self) -> &'static str {
        let side = if self.pan < -0.2 {
            "Left"
        } else if self.pan > 0.2 {
            "Right"
        } else {
            "Centre"
        };
        match (side, self.is_rear()) {
            ("Left", false) => "Left front",
            ("Left", true) => "Left rear",
            ("Right", false) => "Right front",
            ("Right", true) => "Right rear",
            (_, false) => "Centre front",
            (_, true) => "Centre rear",
        }
    }
}

/// The band the user hears: two fiddles, a piano, a second fiddle line, and
/// foot taps by default.
pub fn default_band() -> Vec<BandSlot> {
    vec![
        BandSlot {
            part: BandPart::Melody,
            voice: BandVoice::Fiddle,
            players: 2,
            volume: 0.75,
            pan: -0.15,
            depth: 0.0,
        },
        BandSlot {
            part: BandPart::Accompaniment,
            voice: BandVoice::Piano,
            players: 1,
            volume: 0.65,
            pan: 0.10,
            depth: 0.10,
        },
        BandSlot {
            part: BandPart::Countermelody,
            voice: BandVoice::Fiddle,
            players: 2,
            volume: 0.60,
            pan: 0.30,
            depth: 0.05,
        },
        BandSlot {
            part: BandPart::Percussion,
            voice: BandVoice::FootTaps,
            players: 3,
            volume: 0.50,
            pan: 0.0,
            depth: 0.20,
        },
    ]
}

/// How many channels the music is mixed to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputMode {
    /// One channel, folded down.
    Mono,
    /// Two channels; the default and what every device supports.
    #[default]
    Stereo,
    /// A five-channel front/rear image (left, right, centre, rear left, rear
    /// right), downmixed by the audio engine when the device is stereo.
    Surround,
}

impl OutputMode {
    pub const ALL: [Self; 3] = [Self::Mono, Self::Stereo, Self::Surround];

    pub fn label(self) -> &'static str {
        match self {
            Self::Mono => "Mono",
            Self::Stereo => "Stereo",
            Self::Surround => "Surround",
        }
    }

    pub fn channels(self) -> u16 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
            Self::Surround => 5,
        }
    }
}

/// Which loop-to-loop variations are allowed.
///
/// The four rendered passes each draw from these choices.
/// Turning one off removes that kind of variation everywhere without changing
/// the 64-beat grid or the plain first pass.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VariationOptions {
    /// Shorten long notes on alternate passes (a more detached bowing).
    pub articulation: bool,
    /// Double a melody note with the chord tone below it.
    pub double_stops: bool,
    /// Add a short grace note before some longer notes.
    pub grace_notes: bool,
    /// Lift a held note an octave on later passes.
    pub octave_lift: bool,
    /// Add short chord stabs between the beats.
    pub chord_stabs: bool,
    /// Rarely leave one bar to the melody, resting the accompaniment.
    pub melody_alone_bar: bool,
}

impl Default for VariationOptions {
    fn default() -> Self {
        Self {
            articulation: true,
            double_stops: true,
            grace_notes: true,
            octave_lift: true,
            chord_stabs: true,
            melody_alone_bar: true,
        }
    }
}

impl VariationOptions {
    /// Names for the preferences checkboxes, in display order.
    pub const LABELS: [(&'static str, &'static str); 6] = [
        ("articulation", "Vary note lengths"),
        ("double_stops", "Add chord-tone double stops"),
        ("grace_notes", "Add grace notes"),
        ("octave_lift", "Lift held notes an octave"),
        ("chord_stabs", "Add chord stabs"),
        ("melody_alone_bar", "Let the melody play a bar alone"),
    ];
}

/// How much human timing the band gets.
///
/// The limits are deliberate and small: the dance grid stays exact, and every
/// onset is bounded (see `contra_music::humanize`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HumanizeOptions {
    /// Give each voice an independent slow tempo drift.
    pub tempo_drift: bool,
    /// Spread notes that begin together as a chord by a few milliseconds.
    pub chord_spread: bool,
    /// Scale for both effects, 0.0-1.0. One is the calibrated amount.
    pub amount: f32,
}

impl Default for HumanizeOptions {
    fn default() -> Self {
        Self {
            tempo_drift: true,
            chord_spread: true,
            amount: 1.0,
        }
    }
}

/// How the once-only ending is shaped.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EndingOptions {
    /// Reveal the ending gradually over the last four beats instead of jumping
    /// into a stretched final bar.
    pub gradual: bool,
    /// Resolve the final chord a perfect fifth below the written final chord.
    pub final_chord_down_fifth: bool,
}

impl Default for EndingOptions {
    fn default() -> Self {
        Self {
            gradual: true,
            final_chord_down_fifth: true,
        }
    }
}

/// Which saved tunes the rotation plays.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TuneSetSource {
    /// The explicit list in [`TuneRotation::tunes`].
    #[default]
    List,
    /// Every saved tune in one folder, in library order. The folder is looked
    /// up by name; a deleted folder falls back to the list.
    Folder(String),
}

impl TuneSetSource {
    pub fn label(&self) -> &str {
        match self {
            Self::List => "Chosen tunes",
            Self::Folder(name) => name,
        }
    }
}

/// A sound the dancers make during a figure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DancerSound {
    /// A foot stomp on the floor.
    Stomp,
    /// A hand clap.
    Clap,
}

/// One dancer sound placed on the dance grid.
///
/// The app derives these from the dance card (a petronella or a long line has a
/// stomp, a balance has a clap) and the renderer mixes them under the band, so
/// the music follows the dance rather than a fixed pattern.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DancerSoundEvent {
    /// Dance beat where the sound starts. May repeat every pass.
    pub beat: f64,
    pub sound: DancerSound,
    /// Relative level, 0.0-1.0.
    pub gain: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub const ALL: [Self; 2] = [Self::Dark, Self::Light];

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

/// How the spoken call is delivered.
///
/// The Kokoro model has no emotion or style input, and punctuation is reduced
/// to pauses before synthesis. Tone therefore comes from two model-level cues:
/// a small pitch shift that keeps its own contribution to duration neutral, and
/// a clear-speech pass that holds stressed vowels and lifts consonant presence.
/// Call start times stay on the syllable estimate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallTone {
    /// A lower, projected register with lengthened stressed vowels and a
    /// presence lift for consonants.
    #[default]
    Commanding,
    /// The voice's recorded pitch and even emphasis.
    Natural,
}

impl CallTone {
    pub const ALL: [Self; 2] = [Self::Commanding, Self::Natural];

    pub fn label(self) -> &'static str {
        match self {
            Self::Commanding => "Commanding caller",
            Self::Natural => "Natural",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Self::Commanding => {
                "A lower, projected register with crisper consonants and fuller \
                 stressed vowels. The lead stays on the syllable estimate; the \
                 safety padding covers the clearer delivery."
            }
            Self::Natural => "The voice's recorded pitch and even emphasis.",
        }
    }

    /// Pitch multiplier for the neural voice. `1.0` leaves the voice alone.
    pub fn pitch(self) -> f32 {
        match self {
            Self::Commanding => 0.94,
            Self::Natural => 1.0,
        }
    }

    /// Whether this tone asks the neural voice to sharpen its diction.
    ///
    /// The enunciation pass lengthens the stressed vowel in each content word
    /// and adds a presence/compression stage after synthesis.
    pub fn enunciation(self) -> bool {
        matches!(self, Self::Commanding)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundFontSetting {
    /// Display name, usually the file stem.
    pub name: String,
    /// Native/Android path. Web builds may keep the file only in memory.
    #[serde(default)]
    pub path: Option<String>,
}

/// A set of saved tunes played in turn during a long session.
///
/// One *pass* is one 64-beat time through the dance. A real band usually keeps
/// a tune for several minutes and changes at a repeat, so the default keeps
/// each tune for eight passes (about four minutes at 240 BPM).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TuneRotation {
    pub enabled: bool,
    /// Passes per tune before moving to the next entry.
    pub passes_per_tune: u32,
    /// Saved tunes in play order, by source. Resolved against the saved-tune
    /// library at runtime; a stale entry is skipped.
    pub tunes: Vec<SourceRef>,
    /// Whether the set is the explicit list or the contents of a folder.
    pub source: TuneSetSource,
}

impl Default for TuneRotation {
    fn default() -> Self {
        Self {
            enabled: false,
            passes_per_tune: 8,
            tunes: Vec::new(),
            source: TuneSetSource::List,
        }
    }
}

impl TuneRotation {
    /// Shortest rotation interval offered by the UI.
    pub const MIN_PASSES: u32 = 1;
    /// Longest rotation interval offered by the UI.
    pub const MAX_PASSES: u32 = 16;

    /// Whether the set has enough resolvable tunes to rotate.
    ///
    /// Two or more entries are required. The current tune may be one of them
    /// or not; a single entry can never change what is playing.
    pub fn can_rotate(tune_count: usize) -> bool {
        tune_count >= 2
    }

    /// Repair a hand-edited or older value: clamp the interval and drop
    /// duplicate source references.
    pub fn validate(&mut self) {
        self.passes_per_tune = self
            .passes_per_tune
            .clamp(Self::MIN_PASSES, Self::MAX_PASSES);
        let mut seen = std::collections::BTreeSet::new();
        self.tunes
            .retain(|tune| seen.insert((tune.provider.clone(), tune.id.clone())));
    }

    /// Seconds each tune stays, at a given tempo.
    ///
    /// One pass is the 64-beat dance loop, so this is the interval the session
    /// uses for its "about N minutes" label.
    pub fn seconds_per_tune(&self, bpm: f64) -> f64 {
        let bpm = bpm.max(1.0);
        f64::from(self.passes_per_tune) * 64.0 * (120.0 / bpm)
    }

    /// Whether a source is already in the set.
    pub fn contains(&self, source: &SourceRef) -> bool {
        self.tunes
            .iter()
            .any(|tune| tune.provider == source.provider && tune.id == source.id)
    }

    /// Add a tune if it is not already in the set.
    pub fn add(&mut self, source: SourceRef) {
        if !self.contains(&source) {
            self.tunes.push(source);
        }
    }

    /// Remove a tune by its provider and source id.
    pub fn remove(&mut self, provider: &str, id: &str) {
        self.tunes
            .retain(|tune| tune.provider != provider || tune.id != id);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub schema_version: u32,
    /// The role preset the user explicitly selected.
    ///
    /// Stored instead of inferred every frame: a custom vocabulary can happen
    /// to equal a named preset, and choosing Custom must stay Custom until the
    /// user picks something else. Older files have no field; the
    /// `role-preset-v11` migration infers it from [`Self::role_terms`].
    pub role_preset: RolePreset,
    pub role_terms: RoleTerms,
    pub aliases: AliasTable,
    pub melody: MelodyInstrument,
    /// The chordal accompaniment voice.
    pub accompaniment: AccompanimentInstrument,
    /// Pattern used for the accompaniment chords; see [`AccompanimentRhythm`].
    pub accompaniment_rhythm: AccompanimentRhythm,
    /// The countermelody voice that completes the default trio; see
    /// [`Countermelody`]. The field keeps its original name so older preference
    /// files keep loading.
    pub second_accompaniment: Countermelody,
    /// Players per part of the virtual band; see [`InstrumentCopies`].
    ///
    /// This is the pre-band-list model. It stays for older files and for code
    /// that still describes the three-role band; [`Self::band`] supersedes it
    /// once it is set.
    #[serde(default)]
    pub instrument_copies: InstrumentCopies,
    /// The instruments the user added to the band; see [`BandSlot`].
    ///
    /// Files written before this setting existed are migrated to an explicit
    /// list by `band-instruments-v12`.
    #[serde(default = "default_band")]
    pub band: Vec<BandSlot>,
    /// Mono, stereo, or surround output; see [`OutputMode`].
    #[serde(default)]
    pub output_mode: OutputMode,
    /// Level of the spoken calls, independent of the band mix.
    pub caller_volume: f32,
    /// Optional percussion layer; see [`Percussion`].
    pub percussion: Percussion,
    /// Rhythm used to place the percussion layer; see [`PercussionRhythm`].
    pub percussion_rhythm: PercussionRhythm,
    pub theme: Theme,
    /// Smooth transitions for tab changes, highlights, and screen entries.
    pub animations_enabled: bool,
    pub lead: LeadSettings,
    pub tempo_bpm: f64,
    pub tts_enabled: bool,
    /// Speaking rate as displayed in preferences. The engine receives
    /// [`Preferences::speech_rate`], which applies [`TTS_RATE_SCALE`].
    pub tts_rate: f32,
    #[serde(default = "default_tts_voice")]
    pub tts_voice: String,
    /// Delivery tone for spoken calls; see [`CallTone`].
    #[serde(default)]
    pub tts_tone: CallTone,
    /// Speak and show a randomly chosen alternative call alias for moves that
    /// have one. Off by default so calls stay predictable.
    pub shuffle_alternatives: bool,
    pub loop_enabled: bool,
    pub count_in_enabled: bool,
    /// Add small, deterministic variations to the melody and accompaniment on
    /// later loop passes (ornaments, emphasis notes; now and then one bar of
    /// melody alone).
    pub accompaniment_variations: bool,
    /// Which variation kinds are allowed; see [`VariationOptions`].
    pub variation_options: VariationOptions,
    /// Give each player a tiny independent tempo drift, with a controller that
    /// settles it back on the user's tempo, and spread chord notes by a few
    /// milliseconds. Audio only; the count-in, the 64-beat grid, the calls, and
    /// MIDI export stay exact.
    #[serde(default = "default_humanize_timing")]
    pub humanize_timing: bool,
    /// Which human timing effects are allowed; see [`HumanizeOptions`].
    pub humanize_options: HumanizeOptions,
    /// Shape of the once-only ending; see [`EndingOptions`].
    pub ending_options: EndingOptions,
    /// Include stomps and claps from the dancers on figures that call for them.
    pub dancer_sounds: bool,
    /// Level of the dancer sound layer.
    pub dancer_sound_volume: f32,
    /// Slider floor for the session tempo.
    pub tempo_min_bpm: f64,
    /// Slider ceiling for the session tempo.
    pub tempo_max_bpm: f64,
    pub show_original: bool,
    pub melody_volume: f32,
    pub accompaniment_volume: f32,
    /// Level of the optional percussion layer, when it is enabled.
    pub percussion_volume: f32,
    /// Reverb, chorus, and echo applied to in-app playback and to exported
    /// MIDI files.
    #[serde(alias = "midi_effects")]
    pub effects: AudioEffects,
    pub master_volume: f32,
    pub soundfont: Option<SoundFontSetting>,
    /// Provider id used by the **Download dance** screen.
    ///
    /// Stored as the provider's stable id rather than an enum, so adding a
    /// source does not need a change in `core`. An unknown id is resolved to
    /// [`DEFAULT_DANCE_SOURCE`] by the app.
    #[serde(default = "default_dance_source")]
    pub dance_source: String,
    pub web_proxy_url: Option<String>,
    pub web_proxy_token: Option<String>,
    pub last_dance: Option<SourceRef>,
    pub last_tune: Option<SourceRef>,
    /// Play several saved tunes in turn during a long session.
    pub tune_rotation: TuneRotation,
    /// Migration tags this file has already been through.
    ///
    /// Recorded by name rather than inferred from [`Self::schema_version`] so
    /// two branches that add a migration in the same version range both run
    /// after a merge. Missing in older files, which are seeded from the
    /// version; see [`Preferences::migrate_with`].
    #[serde(default)]
    pub applied_migrations: BTreeSet<String>,
}

/// The version written by this build.
///
/// Bump it when the shape of the file changes; gating a migration on it is not
/// required any more, because [`Preferences::migrate_with`] uses the tags in
/// [`Preferences::applied_migrations`].
pub const CURRENT_SCHEMA_VERSION: u32 = 12;

/// The dance source a new or older preference file downloads from.
///
/// ContraDB is the historical behaviour, so adding [`Preferences::dance_source`]
/// needs no migration: an absent field gets this value.
pub const DEFAULT_DANCE_SOURCE: &str = "contradb";

/// One upgrade step for a preference file written by an older build.
pub struct Migration {
    /// Stable identifier, stored in the file. Never rename or reuse one.
    pub tag: &'static str,
    /// The schema version that introduced the change, used only to seed tags
    /// for files written before tags existed.
    pub version: u32,
    /// Applies the change. It must leave values the user already changed
    /// alone.
    pub apply: fn(&mut Preferences),
}

/// Every migration this build knows, oldest first.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        tag: "lead-calibration-v2",
        version: 2,
        apply: migrate_v1_lead_calibration,
    },
    Migration {
        tag: "animations-v3",
        version: 3,
        apply: migrate_v2_animations,
    },
    Migration {
        tag: "accompaniment-variations-v4",
        version: 4,
        apply: migrate_v3_accompaniment_variations,
    },
    Migration {
        tag: "tts-voice-v5",
        version: 5,
        apply: migrate_v4_tts_voice,
    },
    Migration {
        tag: "accompaniment-rhythm-v6",
        version: 6,
        apply: migrate_v5_accompaniment_rhythm,
    },
    Migration {
        tag: "second-accompaniment-v7",
        version: 7,
        apply: migrate_v6_countermelody,
    },
    Migration {
        tag: "tune-rotation-v7",
        version: 7,
        apply: migrate_v6_tune_rotation,
    },
    Migration {
        tag: "tts-tone-v7",
        version: 7,
        apply: migrate_v6_tts_tone,
    },
    Migration {
        tag: "percussion-v8",
        version: 8,
        apply: migrate_v7_percussion,
    },
    Migration {
        tag: "percussion-rhythm-v9",
        version: 9,
        apply: migrate_v8_percussion_rhythm,
    },
    Migration {
        tag: "tts-voice-nicole-v9",
        version: 9,
        apply: migrate_v8_tts_voice_nicole,
    },
    Migration {
        tag: "audio-effects-v10",
        version: 10,
        apply: migrate_v9_audio_effects,
    },
    Migration {
        tag: "role-preset-v11",
        version: 11,
        apply: migrate_v10_role_preset,
    },
    Migration {
        tag: "echo-subtle-v12",
        version: 12,
        apply: migrate_v11_echo_subtle,
    },
    Migration {
        tag: "band-instruments-v12",
        version: 12,
        apply: migrate_v11_band_instruments,
    },
    Migration {
        tag: "dancer-sounds-v12",
        version: 12,
        apply: migrate_v11_dancer_sounds,
    },
];

fn migrate_v1_lead_calibration(prefs: &mut Preferences) {
    // v1 calibrated at 6.5 syllables/s, a 2-beat maximum, and small word and
    // pause allowances. The measured delivery is 4.0 syllables/s with no
    // additional gaps, and a 5-beat maximum.
    if (prefs.lead.syllables_per_second - 6.5).abs() < 1e-6 {
        prefs.lead.syllables_per_second = 4.0;
    }
    if (prefs.lead.max_beats - 2.0).abs() < 1e-6 {
        prefs.lead.max_beats = 5.0;
    }
    if (prefs.lead.word_gap_seconds - 0.04).abs() < 1e-6 {
        prefs.lead.word_gap_seconds = 0.0;
    }
    if (prefs.lead.extra_pause_seconds - 0.10).abs() < 1e-6 {
        prefs.lead.extra_pause_seconds = 0.0;
    }
}

fn migrate_v2_animations(_prefs: &mut Preferences) {
    // Animations are a new field; `serde` supplies the enabled default for
    // older files, so there is nothing to change.
}

fn migrate_v3_accompaniment_variations(_prefs: &mut Preferences) {
    // Idem for the accompaniment variations.
}

fn migrate_v4_tts_voice(prefs: &mut Preferences) {
    // v5 adds the selectable TTS voice and alternative-alias shuffling.
    // Existing files keep the previous default voice and start with shuffling
    // disabled.
    if prefs.tts_voice.trim().is_empty() {
        prefs.tts_voice = default_tts_voice();
    }
}

fn migrate_v5_accompaniment_rhythm(_prefs: &mut Preferences) {
    // v6 adds the selectable accompaniment rhythm and the adjustable safety
    // padding; `serde` supplies the defaults for older files.
}

fn migrate_v6_countermelody(_prefs: &mut Preferences) {
    // v7 adds the countermelody (then called the second accompaniment). Older
    // files have no value, so `serde` supplies the default (Accordion), which
    // gives every session the trio sound. A file that stores `Off` keeps it.
    // The tag keeps its original name because a tag must never be renamed.
}

fn migrate_v6_tune_rotation(_prefs: &mut Preferences) {
    // v7 adds the optional tune rotation; `serde` supplies the disabled
    // default for older files.
}

fn migrate_v6_tts_tone(_prefs: &mut Preferences) {
    // v7 adds the caller tone. `serde` supplies the Commanding default for
    // older files, which is the requested change, so there is nothing else to
    // adjust.
}

fn migrate_v8_tts_voice_nicole(prefs: &mut Preferences) {
    // v9 retires Nicole, whose soft delivery was too quiet to call a dance.
    // Sarah is the replacement US female voice, so a stored Nicole preference
    // moves there instead of falling back to the default. Every other voice is
    // left alone, and the app still treats an unknown ID as the default.
    if prefs.tts_voice == "af_nicole" {
        prefs.tts_voice = "af_sarah".to_string();
    }
}

fn migrate_v7_percussion(_prefs: &mut Preferences) {
    // v8 adds the optional percussion layer. `serde` supplies the disabled
    // default, so existing sessions keep their current sound until the user
    // turns percussion on.
}

fn migrate_v8_percussion_rhythm(_prefs: &mut Preferences) {
    // v9 adds the percussion rhythm. `serde` supplies Auto, which keeps the
    // style's existing pattern.
}

fn migrate_v9_audio_effects(_prefs: &mut Preferences) {
    // v10 adds the playback and exported-MIDI effects. `serde` supplies the
    // default medium-room reverb with no chorus or echo, so existing files get
    // the new room sound without changing anything else.
}

fn migrate_v10_role_preset(prefs: &mut Preferences) {
    // v11 stores the explicit role preset. Files written before it carry only
    // the four terms, so infer the preset from them: a custom vocabulary (or
    // terms that no preset defines) becomes Custom instead of showing the
    // wrong selection.
    prefs.role_preset = RolePreset::detect(&prefs.role_terms);
}

fn migrate_v11_echo_subtle(prefs: &mut Preferences) {
    // v12 makes a subtle echo the default. Files written with the old dry
    // default get the new room sound; a user who had already chosen an echo
    // keeps it. As with the earlier default-change migrations, this cannot
    // distinguish an explicit Off from the old default, so Off moves to
    // Subtle.
    if prefs.effects.echo == crate::effects::EchoPreset::Off {
        prefs.effects.echo = crate::effects::EchoPreset::Subtle;
    }
}

fn migrate_v11_band_instruments(prefs: &mut Preferences) {
    // v12 replaces the fixed three-role band with a list. A file that never
    // customised the old roles gets the new default band (two melody fiddles,
    // piano, two countermelody fiddles, three foot taps). A customised file
    // keeps its choices as one slot per role so nothing it heard changes.
    //
    // A file already written by v12 carries the list, and `serde` supplies the
    // new default for a missing one, so only older files are translated.
    if prefs.schema_version >= 12 {
        return;
    }
    let untouched = prefs.melody == MelodyInstrument::Fiddle
        && prefs.accompaniment == AccompanimentInstrument::Piano
        && prefs.second_accompaniment == Countermelody::Accordion
        && prefs.percussion == Percussion::Off
        && prefs.instrument_copies == InstrumentCopies::default();
    prefs.band = if untouched {
        default_band()
    } else {
        prefs.legacy_band()
    };
}

fn migrate_v11_dancer_sounds(_prefs: &mut Preferences) {
    // v12 adds the dancer sound layer. `serde` supplies the enabled default,
    // which is the requested behaviour for existing sessions, so there is
    // nothing to adjust.
}

/// Every tag in [`MIGRATIONS`], for a fresh file that needs none of them.
fn applied_tags() -> BTreeSet<String> {
    MIGRATIONS
        .iter()
        .map(|migration| migration.tag.to_owned())
        .collect()
}

/// What subsystems must react when preferences change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChangeSet {
    /// Calls must be rebuilt: role terms, aliases, lead settings, tempo,
    /// count-in, or alias shuffling changed.
    pub timeline: bool,
    /// Music must be re-rendered: arrangement, mix, or soundfont changed.
    pub audio: bool,
    /// Spoken calls must be synthesized again: the delivery tone changed.
    pub tone: bool,
}

impl ChangeSet {
    /// Whether anything has to be redone.
    pub fn any(self) -> bool {
        self.timeline || self.audio || self.tone
    }
}

fn default_tts_voice() -> String {
    "af_heart".to_string()
}

/// Human timing is on by default; it is part of the band's sound, not an
/// effect the user has to discover.
fn default_humanize_timing() -> bool {
    true
}

fn default_dance_source() -> String {
    DEFAULT_DANCE_SOURCE.to_string()
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            role_preset: RolePreset::LarksRobins,
            role_terms: RoleTerms::default(),
            aliases: AliasTable::default(),
            melody: MelodyInstrument::default(),
            accompaniment: AccompanimentInstrument::default(),
            accompaniment_rhythm: AccompanimentRhythm::default(),
            second_accompaniment: Countermelody::default(),
            instrument_copies: InstrumentCopies::default(),
            band: default_band(),
            output_mode: OutputMode::default(),
            caller_volume: 1.0,
            percussion: Percussion::default(),
            percussion_rhythm: PercussionRhythm::default(),
            theme: Theme::default(),
            animations_enabled: true,
            lead: LeadSettings::default(),
            tempo_bpm: Tempo::DEFAULT_BPM,
            tts_enabled: false,
            tts_rate: 1.0,
            tts_voice: default_tts_voice(),
            tts_tone: CallTone::default(),
            shuffle_alternatives: false,
            loop_enabled: true,
            count_in_enabled: true,
            accompaniment_variations: true,
            variation_options: VariationOptions::default(),
            humanize_timing: default_humanize_timing(),
            humanize_options: HumanizeOptions::default(),
            ending_options: EndingOptions::default(),
            dancer_sounds: true,
            dancer_sound_volume: 0.35,
            tempo_min_bpm: 200.0,
            tempo_max_bpm: 300.0,
            show_original: false,
            melody_volume: 0.75,
            accompaniment_volume: 0.65,
            percussion_volume: 0.5,
            effects: AudioEffects::default(),
            master_volume: 0.8,
            soundfont: None,
            dance_source: default_dance_source(),
            web_proxy_url: None,
            web_proxy_token: None,
            last_dance: None,
            last_tune: None,
            tune_rotation: TuneRotation::default(),
            applied_migrations: applied_tags(),
        }
    }
}

impl Preferences {
    pub fn tempo(&self) -> Tempo {
        Tempo::new(self.tempo_bpm)
    }

    /// The band to render, validated for the renderer.
    ///
    /// The list always contains at least one pitched slot. A file or UI that
    /// removed every pitched instrument falls back to a single front fiddle
    /// instead of rendering silence.
    pub fn band_slots(&self) -> Vec<BandSlot> {
        let mut band = self.band.clone();
        if band.is_empty() {
            band = default_band();
        }
        if !band.iter().any(BandSlot::is_pitched) {
            band.insert(0, BandSlot::default());
        }
        for slot in &mut band {
            slot.validate();
        }
        band
    }

    /// Whether the list contains at least one pitched instrument.
    pub fn has_pitched_band(&self) -> bool {
        self.band.iter().any(BandSlot::is_pitched)
    }

    /// Mutable access to the band list.
    pub fn band_mut(&mut self) -> &mut Vec<BandSlot> {
        &mut self.band
    }

    /// The first slot that plays pitched notes, if any.
    pub fn first_pitched_slot(&self) -> Option<usize> {
        self.band.iter().position(BandSlot::is_pitched)
    }

    /// The one-slot-per-role band implied by the original settings.
    fn legacy_band(&self) -> Vec<BandSlot> {
        let mut band = Vec::new();
        band.push(BandSlot {
            part: BandPart::Melody,
            voice: BandVoice::from_melody(self.melody),
            players: self.instrument_copies.melody,
            volume: self.melody_volume,
            pan: -0.15,
            depth: 0.0,
        });
        band.push(BandSlot {
            part: BandPart::Accompaniment,
            voice: BandVoice::from_accompaniment(self.accompaniment),
            players: self.instrument_copies.accompaniment,
            volume: self.accompaniment_volume,
            pan: 0.10,
            depth: 0.10,
        });
        if let Some(instrument) = self.second_accompaniment.instrument() {
            band.push(BandSlot {
                part: BandPart::Countermelody,
                voice: BandVoice::from_melody(instrument),
                players: self.instrument_copies.countermelody,
                volume: self.accompaniment_volume * 0.8,
                pan: 0.30,
                depth: 0.05,
            });
        }
        if let Some(voice) = BandVoice::from_percussion(self.percussion) {
            band.push(BandSlot {
                part: BandPart::Percussion,
                voice,
                players: self.instrument_copies.percussion,
                volume: self.percussion_volume,
                pan: 0.0,
                depth: 0.20,
            });
        }
        for slot in &mut band {
            slot.validate();
        }
        band
    }

    /// Select a role preset.
    ///
    /// A named preset replaces all four terms. Custom keeps the current words
    /// as the starting point for editing and stays selected: the selection is
    /// stored in [`Self::role_preset`], not re-detected from the terms, so a
    /// vocabulary that happens to match a named preset does not snap back.
    pub fn select_role_preset(&mut self, preset: RolePreset) {
        self.role_preset = preset;
        if preset != RolePreset::Custom {
            self.role_terms = RoleTerms::preset(preset);
        }
    }

    /// The rate handed to the speech engine.
    pub fn speech_rate(&self) -> f32 {
        self.tts_rate * TTS_RATE_SCALE
    }

    /// Clamp values that came from a preference file or a UI control.
    pub fn validate(&mut self) {
        if self.schema_version == 0 {
            self.schema_version = CURRENT_SCHEMA_VERSION;
        }
        if self.tts_voice.trim().is_empty() {
            self.tts_voice = default_tts_voice();
        }
        if self.dance_source.trim().is_empty() {
            self.dance_source = default_dance_source();
        }
        self.lead.validate();
        self.tempo_bpm = Tempo::new(self.tempo_bpm).bpm();
        self.tune_rotation.validate();
        self.tts_rate = self.tts_rate.clamp(MIN_TTS_RATE, MAX_TTS_RATE);
        self.instrument_copies.validate();
        for slot in self.band.iter_mut() {
            slot.validate();
        }
        // A band must always have at least one pitched slot. Removing the last
        // one (or loading a hand-edited file without one) falls back to a
        // single front fiddle instead of rendering silence.
        if self.band.is_empty() {
            self.band = default_band();
        } else if !self.band.iter().any(BandSlot::is_pitched) {
            self.band.insert(0, BandSlot::default());
        }
        self.caller_volume = self.caller_volume.clamp(0.0, 2.0);
        if !self.tempo_min_bpm.is_finite() {
            self.tempo_min_bpm = 200.0;
        }
        if !self.tempo_max_bpm.is_finite() {
            self.tempo_max_bpm = 300.0;
        }
        self.tempo_min_bpm = self.tempo_min_bpm.clamp(Tempo::MIN_BPM, Tempo::MAX_BPM);
        self.tempo_max_bpm = self.tempo_max_bpm.clamp(Tempo::MIN_BPM, Tempo::MAX_BPM);
        if self.tempo_min_bpm > self.tempo_max_bpm {
            std::mem::swap(&mut self.tempo_min_bpm, &mut self.tempo_max_bpm);
        }
        self.humanize_options.amount = self.humanize_options.amount.clamp(0.0, 1.0);
        self.dancer_sound_volume = self.dancer_sound_volume.clamp(0.0, 2.0);
        self.melody_volume = self.melody_volume.clamp(0.0, 2.0);
        self.accompaniment_volume = self.accompaniment_volume.clamp(0.0, 2.0);
        self.percussion_volume = self.percussion_volume.clamp(0.0, 2.0);
        self.master_volume = self.master_volume.clamp(0.0, 2.0);
        if self.web_proxy_url.as_deref().is_some_and(str::is_empty) {
            self.web_proxy_url = None;
        }
        if self.web_proxy_token.as_deref().is_some_and(str::is_empty) {
            self.web_proxy_token = None;
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut prefs: Self = serde_json::from_str(json)?;
        prefs.migrate();
        prefs.validate();
        Ok(prefs)
    }

    /// Upgrade stored preferences without overwriting values the user changed
    /// away from the previous defaults.
    fn migrate(&mut self) {
        self.migrate_with(MIGRATIONS);
    }

    /// Run every migration in `migrations` that this file has not recorded.
    ///
    /// Gating on tags rather than on a shared version number is what makes
    /// this merge-safe: when two branches add a setting, each adds a migration
    /// with its own tag, and after a merge both run. With the old
    /// `schema_version < n` chain the first block advanced the version past
    /// the second, so one migration silently never ran.
    fn migrate_with(&mut self, migrations: &[Migration]) {
        if self.applied_migrations.is_empty() {
            // A file written before tags existed records only a version;
            // treat everything at or below it as already applied.
            self.applied_migrations = migrations
                .iter()
                .filter(|migration| migration.version <= self.schema_version)
                .map(|migration| migration.tag.to_owned())
                .collect();
        }
        for migration in migrations {
            if self.applied_migrations.contains(migration.tag) {
                continue;
            }
            (migration.apply)(self);
            self.applied_migrations.insert(migration.tag.to_owned());
        }
        self.schema_version = self.schema_version.max(CURRENT_SCHEMA_VERSION);
    }

    /// What must be rebuilt after moving from `old` to `self`.
    ///
    /// A setting is wired to its subsystem here, next to the field it affects,
    /// so it cannot be saved without the right rebuild. The UI used to carry
    /// this list inline, where a forgotten entry silently did nothing until
    /// the next restart.
    pub fn changes_since(&self, old: &Self) -> ChangeSet {
        ChangeSet {
            timeline: self.role_terms != old.role_terms
                || self.aliases != old.aliases
                || self.lead != old.lead
                || self.tempo_bpm != old.tempo_bpm
                || self.count_in_enabled != old.count_in_enabled
                || self.shuffle_alternatives != old.shuffle_alternatives,
            audio: self.tempo_bpm != old.tempo_bpm
                || self.count_in_enabled != old.count_in_enabled
                || self.accompaniment_variations != old.accompaniment_variations
                || self.variation_options != old.variation_options
                || self.humanize_timing != old.humanize_timing
                || self.humanize_options != old.humanize_options
                || self.ending_options != old.ending_options
                || self.dancer_sounds != old.dancer_sounds
                || self.dancer_sound_volume != old.dancer_sound_volume
                || self.melody != old.melody
                || self.accompaniment != old.accompaniment
                || self.accompaniment_rhythm != old.accompaniment_rhythm
                || self.second_accompaniment != old.second_accompaniment
                || self.instrument_copies != old.instrument_copies
                || self.band != old.band
                || self.output_mode != old.output_mode
                || self.percussion != old.percussion
                || self.percussion_rhythm != old.percussion_rhythm
                || self.melody_volume != old.melody_volume
                || self.accompaniment_volume != old.accompaniment_volume
                || self.percussion_volume != old.percussion_volume
                || self.effects != old.effects
                || self.soundfont != old.soundfont,
            tone: self.tts_tone != old.tts_tone,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timing::LEAD_PADDING_SECONDS;

    #[test]
    fn defaults_match_product_spec() {
        let prefs = Preferences::default();
        assert_eq!(prefs.role_preset, RolePreset::LarksRobins);
        assert_eq!(prefs.role_terms.left_plural, "larks");
        assert_eq!(prefs.role_terms.right_plural, "robins");
        assert_eq!(prefs.melody, MelodyInstrument::Fiddle);
        assert_eq!(prefs.accompaniment, AccompanimentInstrument::Piano);
        assert_eq!(prefs.accompaniment_rhythm, AccompanimentRhythm::Auto);
        assert_eq!(prefs.second_accompaniment, Countermelody::Accordion);
        assert!(
            prefs.second_accompaniment.instrument().is_some(),
            "the default session is a trio"
        );
        assert_eq!(prefs.instrument_copies, InstrumentCopies::default());
        assert_eq!(prefs.instrument_copies.melody, 1);
        assert_eq!(prefs.instrument_copies.accompaniment, 1);
        assert_eq!(prefs.instrument_copies.countermelody, 1);
        assert_eq!(prefs.instrument_copies.percussion, 1);
        assert_eq!(prefs.percussion, Percussion::Off);
        assert!(!prefs.percussion.is_enabled());
        assert_eq!(prefs.effects, AudioEffects::default());
        assert_eq!(
            prefs.effects.reverb,
            crate::effects::ReverbPreset::MediumRoom
        );
        assert_eq!(prefs.theme, Theme::Dark);
        assert!(prefs.animations_enabled);
        assert_eq!(prefs.tempo_bpm, 240.0);
        assert!(prefs.loop_enabled);
        assert!(prefs.count_in_enabled);
        assert!(prefs.accompaniment_variations);
        assert!(prefs.humanize_timing);
        assert!(!prefs.tts_enabled);
        assert_eq!(prefs.tts_rate, 1.0);
        assert!((prefs.speech_rate() - 1.2).abs() < 1e-6);
        assert_eq!(prefs.tts_voice, "af_heart");
        assert_eq!(prefs.tts_tone, CallTone::Commanding);
        assert_eq!(prefs.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(!prefs.shuffle_alternatives);
        assert!(!prefs.tune_rotation.enabled);
        assert_eq!(prefs.tune_rotation.passes_per_tune, 8);
    }

    #[test]
    fn the_default_band_is_the_requested_lineup() {
        let prefs = Preferences::default();
        let band = prefs.band_slots();
        assert_eq!(band.len(), 4);
        assert_eq!(
            (band[0].part, band[0].voice, band[0].players),
            (BandPart::Melody, BandVoice::Fiddle, 2)
        );
        assert_eq!(
            (band[1].part, band[1].voice, band[1].players),
            (BandPart::Accompaniment, BandVoice::Piano, 1)
        );
        assert_eq!(
            (band[2].part, band[2].voice, band[2].players),
            (BandPart::Countermelody, BandVoice::Fiddle, 2)
        );
        assert_eq!(
            (band[3].part, band[3].voice, band[3].players),
            (BandPart::Percussion, BandVoice::FootTaps, 3)
        );
        assert!(prefs.dancer_sounds, "dancer sounds are on by default");
        assert_eq!(prefs.caller_volume, 1.0);
        assert_eq!(prefs.output_mode, OutputMode::Stereo);
        assert_eq!((prefs.tempo_min_bpm, prefs.tempo_max_bpm), (200.0, 300.0));
    }

    #[test]
    fn older_files_get_the_new_band_and_subtle_echo() {
        let prefs = Preferences::from_json(r#"{"schema_version":11}"#).expect("deserialize");
        assert_eq!(prefs.band, default_band());
        assert_eq!(prefs.effects.echo, crate::effects::EchoPreset::Subtle);
        assert!(prefs.dancer_sounds);
        assert_eq!(prefs.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn a_customised_old_band_keeps_its_own_choices() {
        let json = r#"{
            "schema_version": 11,
            "melody": "Clarinet",
            "accompaniment": "Guitar",
            "second_accompaniment": "Off",
            "percussion": "Drumset",
            "instrument_copies": {"melody": 3, "accompaniment": 2, "countermelody": 1, "percussion": 2}
        }"#;
        let prefs = Preferences::from_json(json).expect("deserialize");
        let band = prefs.band_slots();
        assert_eq!(band.len(), 3, "a duo plus a drum kit has three slots");
        assert_eq!(band[0].voice, BandVoice::Clarinet);
        assert_eq!(band[0].players, 3);
        assert_eq!(band[1].voice, BandVoice::Guitar);
        assert_eq!(band[1].players, 2);
        assert_eq!(band[2].voice, BandVoice::Drumset);
        assert_eq!(band[2].players, 2);
    }

    #[test]
    fn the_band_list_is_clamped_and_round_trips() {
        let mut prefs = Preferences::default();
        prefs.band_mut().push(BandSlot {
            part: BandPart::Accompaniment,
            voice: BandVoice::Guitar,
            players: 99,
            volume: 9.0,
            pan: -9.0,
            depth: 9.0,
        });
        prefs.band_mut()[0].players = 0;
        prefs.validate();
        let band = prefs.band_slots();
        assert_eq!(band.len(), 5);
        assert_eq!(band[0].players, 1);
        assert_eq!(band[4].players, BandSlot::MAX_PLAYERS);
        assert_eq!(band[4].volume, 2.0);
        assert_eq!(band[4].pan, -1.0);
        assert_eq!(band[4].depth, 1.0);

        let restored =
            Preferences::from_json(&prefs.to_json().expect("serialize")).expect("deserialize");
        assert_eq!(restored.band_slots(), band);
    }

    #[test]
    fn a_tune_set_can_be_a_folder() {
        let prefs = Preferences {
            tune_rotation: TuneRotation {
                enabled: true,
                source: TuneSetSource::Folder("Warmups".into()),
                ..TuneRotation::default()
            },
            ..Preferences::default()
        };
        let restored =
            Preferences::from_json(&prefs.to_json().expect("serialize")).expect("deserialize");
        assert_eq!(
            restored.tune_rotation.source,
            TuneSetSource::Folder("Warmups".into())
        );
    }

    #[test]
    fn older_files_keep_the_explicit_tune_list() {
        let prefs = Preferences::from_json(r#"{"schema_version":11}"#).expect("deserialize");
        assert_eq!(prefs.tune_rotation.source, TuneSetSource::List);
    }

    #[test]
    fn the_tempo_slider_range_is_validated() {
        let mut prefs = Preferences {
            tempo_min_bpm: 320.0,
            tempo_max_bpm: 120.0,
            ..Preferences::default()
        };
        prefs.validate();
        assert_eq!((prefs.tempo_min_bpm, prefs.tempo_max_bpm), (120.0, 320.0));
    }

    #[test]
    fn humanize_timing_is_on_by_default_and_marks_the_music_for_render() {
        let old = Preferences::default();
        assert!(old.humanize_timing);

        let changed = Preferences {
            humanize_timing: false,
            ..old.clone()
        };
        let changes = changed.changes_since(&old);
        assert!(changes.audio, "a humanize change must re-render the music");
        assert!(!changes.timeline, "it must not rebuild the call timeline");

        let json = changed.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert!(!restored.humanize_timing);
    }

    #[test]
    fn older_files_get_human_timing_by_default() {
        let prefs = Preferences::from_json(r#"{"schema_version": 11}"#).expect("deserialize");
        assert!(prefs.humanize_timing);
        assert_eq!(prefs.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn clamps_bad_values() {
        let mut prefs = Preferences {
            tempo_bpm: 10_000.0,
            melody_volume: 99.0,
            ..Preferences::default()
        };
        prefs.lead.syllables_per_second = 0.0;
        prefs.validate();
        assert_eq!(prefs.tempo_bpm, Tempo::MAX_BPM);
        assert_eq!(prefs.melody_volume, 2.0);
        assert!(prefs.lead.syllables_per_second >= 0.5);
    }

    #[test]
    fn clamps_tts_rate_to_the_displayed_range() {
        let mut prefs = Preferences {
            tts_rate: 9.0,
            ..Preferences::default()
        };
        prefs.validate();
        assert_eq!(prefs.tts_rate, MAX_TTS_RATE);
        prefs.tts_rate = 0.0;
        prefs.validate();
        assert_eq!(prefs.tts_rate, MIN_TTS_RATE);
    }

    #[test]
    fn the_dance_source_defaults_to_contradb_and_round_trips() {
        let prefs = Preferences::default();
        assert_eq!(prefs.dance_source, DEFAULT_DANCE_SOURCE);
        assert_eq!(prefs.dance_source, "contradb");

        let prefs = Preferences {
            dance_source: "callersbox".into(),
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.dance_source, "callersbox");
    }

    #[test]
    fn older_files_download_from_contradb() {
        let prefs = Preferences::from_json(r#"{"schema_version":11}"#).expect("deserialize");
        assert_eq!(prefs.dance_source, DEFAULT_DANCE_SOURCE);
    }

    #[test]
    fn a_blank_dance_source_falls_back_to_contradb() {
        let mut prefs = Preferences {
            dance_source: "   ".into(),
            ..Preferences::default()
        };
        prefs.validate();
        assert_eq!(prefs.dance_source, DEFAULT_DANCE_SOURCE);
    }

    #[test]
    fn json_round_trip() {
        let mut prefs = Preferences {
            tts_enabled: true,
            tempo_bpm: 220.0,
            ..Preferences::default()
        };
        prefs
            .aliases
            .set_override(crate::alias::MoveKind::Swing, "swing it");
        prefs
            .aliases
            .add_alternative(crate::alias::MoveKind::Swing, "swing it around");
        prefs.shuffle_alternatives = true;
        prefs.effects.reverb = crate::effects::ReverbPreset::Hall;
        prefs.effects.echo = crate::effects::EchoPreset::Strong;
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(prefs, restored);
    }

    #[test]
    fn alternative_alias_shuffling_is_off_by_default_and_round_trips() {
        let prefs = Preferences::default();
        assert!(!prefs.shuffle_alternatives);

        let prefs = Preferences {
            shuffle_alternatives: true,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert!(restored.shuffle_alternatives);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let prefs = Preferences::from_json("{}").expect("deserialize");
        assert_eq!(prefs, Preferences::default());
    }

    #[test]
    fn role_presets_round_trip_through_preferences() {
        for preset in RolePreset::ALL {
            let mut prefs = Preferences::default();
            prefs.select_role_preset(preset);
            let json = prefs.to_json().expect("serialize");
            let restored = Preferences::from_json(&json).expect("deserialize");
            assert_eq!(restored.role_preset, preset);
            if preset != RolePreset::Custom {
                assert_eq!(restored.role_terms, RoleTerms::preset(preset));
            }
        }
    }

    #[test]
    fn custom_role_terms_survive_a_preferences_round_trip() {
        let mut prefs = Preferences::default();
        prefs.select_role_preset(RolePreset::Custom);
        prefs.role_terms = RoleTerms {
            left_singular: "port".into(),
            left_plural: "ports".into(),
            right_singular: "starboard".into(),
            right_plural: "starboards".into(),
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.role_preset, RolePreset::Custom);
        assert_eq!(restored.role_terms, prefs.role_terms);
    }

    /// The stored selection is the source of truth. A custom vocabulary may
    /// match a named preset, but the menu must stay on Custom until the user
    /// chooses another preset.
    #[test]
    fn a_stored_custom_selection_is_not_redetected() {
        let mut prefs = Preferences::default();
        prefs.select_role_preset(RolePreset::Custom);
        prefs.role_terms = RoleTerms::preset(RolePreset::LarksRobins);
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.role_preset, RolePreset::Custom);
        assert_eq!(
            restored.role_terms,
            RoleTerms::preset(RolePreset::LarksRobins)
        );
    }

    #[test]
    fn older_files_infer_the_role_preset_from_their_terms() {
        let custom = r#"{
            "schema_version": 10,
            "role_terms": {
                "left_singular": "port",
                "left_plural": "ports",
                "right_singular": "starboard",
                "right_plural": "starboards"
            }
        }"#;
        let prefs = Preferences::from_json(custom).expect("deserialize");
        assert_eq!(prefs.role_preset, RolePreset::Custom);
        assert_eq!(prefs.role_terms.left_plural, "ports");

        let presets = r#"{
            "schema_version": 10,
            "role_terms": {
                "left_singular": "lady",
                "left_plural": "ladies",
                "right_singular": "gentleman",
                "right_plural": "gentlemen"
            }
        }"#;
        let prefs = Preferences::from_json(presets).expect("deserialize");
        assert_eq!(prefs.role_preset, RolePreset::LadiesGentlemen);
    }

    #[test]
    fn migrates_v1_lead_defaults_to_measured_calibration() {
        let mut old = Preferences {
            schema_version: 1,
            applied_migrations: BTreeSet::new(),
            ..Preferences::default()
        };
        old.lead.syllables_per_second = 6.5;
        old.lead.max_beats = 2.0;
        old.lead.word_gap_seconds = 0.04;
        old.lead.extra_pause_seconds = 0.10;
        let json = old.to_json().expect("serialize");
        let migrated = Preferences::from_json(&json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.tts_voice, "af_heart");
        assert_eq!(migrated.lead.syllables_per_second, 4.0);
        assert_eq!(migrated.lead.max_beats, 5.0);
        assert_eq!(migrated.lead.word_gap_seconds, 0.0);
        assert_eq!(migrated.lead.extra_pause_seconds, 0.0);
    }

    #[test]
    fn v2_preferences_enable_animations_by_default() {
        let json = r#"{"schema_version":2,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(migrated.animations_enabled);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v3_preferences_enable_accompaniment_variations_by_default() {
        let json = r#"{"schema_version":3,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(migrated.accompaniment_variations);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v4_preferences_use_default_voice_and_disable_alias_shuffling() {
        let json = r#"{"schema_version":4,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.tts_voice, "af_heart");
        assert!(!migrated.shuffle_alternatives);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v5_preferences_use_the_new_defaults() {
        let json = r#"{"schema_version":5,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.accompaniment_rhythm, AccompanimentRhythm::Auto);
        assert_eq!(migrated.lead.padding_seconds, LEAD_PADDING_SECONDS);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v6_preferences_use_the_commanding_tone_by_default() {
        let json = r#"{"schema_version":6,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.tts_tone, CallTone::Commanding);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v9_preferences_default_to_a_medium_room_audio_reverb() {
        let json = r#"{"schema_version":9,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.effects, AudioEffects::default());
        assert_eq!(
            migrated.effects.reverb,
            crate::effects::ReverbPreset::MediumRoom
        );
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn preferences_accept_the_old_midi_effects_key() {
        let json = r#"{"schema_version":10,"midi_effects":{"reverb":"Hall","chorus":"Rich","echo":"Strong"}}"#;
        let prefs = Preferences::from_json(json).expect("deserialize");
        assert_eq!(prefs.effects.reverb, crate::effects::ReverbPreset::Hall);
        assert_eq!(prefs.effects.chorus, crate::effects::ChorusPreset::Rich);
        assert_eq!(prefs.effects.echo, crate::effects::EchoPreset::Strong);
    }

    #[test]
    fn v7_preferences_move_nicole_to_sarah() {
        let json = r#"{"schema_version":7,"tts_voice":"af_nicole"}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.tts_voice, "af_sarah");
    }

    #[test]
    fn the_nicole_migration_leaves_other_voices_alone() {
        for voice in ["af_heart", "am_michael", "am_onyx"] {
            let json = format!(r#"{{"schema_version":7,"tts_voice":"{voice}"}}"#);
            let migrated = Preferences::from_json(&json).expect("deserialize");
            assert_eq!(migrated.tts_voice, voice);
        }
    }

    #[test]
    fn call_tone_round_trips() {
        let prefs = Preferences {
            tts_tone: CallTone::Natural,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.tts_tone, CallTone::Natural);
    }

    #[test]
    fn commanding_tone_lowers_the_neural_pitch() {
        assert!(CallTone::Commanding.pitch() < 1.0);
        assert_eq!(CallTone::Natural.pitch(), 1.0);
        assert!(CallTone::Commanding.enunciation());
        assert!(!CallTone::Natural.enunciation());
        assert_eq!(CallTone::default(), CallTone::Commanding);
    }

    #[test]
    fn accompaniment_rhythm_round_trips() {
        let prefs = Preferences {
            accompaniment_rhythm: AccompanimentRhythm::Shuffle,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.accompaniment_rhythm, AccompanimentRhythm::Shuffle);
    }

    #[test]
    fn the_countermelody_can_be_turned_off_and_round_trips() {
        let prefs = Preferences {
            second_accompaniment: Countermelody::Off,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.second_accompaniment, Countermelody::Off);
        assert_eq!(restored.second_accompaniment.instrument(), None);
    }

    #[test]
    fn the_countermelody_uses_a_melody_instrument() {
        assert_eq!(
            Countermelody::Accordion.instrument(),
            Some(MelodyInstrument::Accordion)
        );
        assert_eq!(
            Countermelody::Fiddle.instrument(),
            Some(MelodyInstrument::Fiddle)
        );
        assert_eq!(Countermelody::Off.gm_program(), None);
        assert_eq!(
            Countermelody::Accordion.gm_program(),
            Some(MelodyInstrument::Accordion.gm_program())
        );
    }

    #[test]
    fn instrument_copies_round_trip_and_clamp() {
        let prefs = Preferences {
            instrument_copies: InstrumentCopies {
                melody: 2,
                accompaniment: 3,
                countermelody: 4,
                percussion: 2,
            },
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.instrument_copies, prefs.instrument_copies);
        assert_eq!(restored.instrument_copies.melody_count(), 2);
        assert_eq!(restored.instrument_copies.accompaniment_count(), 3);
        assert_eq!(restored.instrument_copies.countermelody_count(), 4);
        assert_eq!(restored.instrument_copies.percussion_count(), 2);

        let mut hand_edited = InstrumentCopies {
            melody: 0,
            accompaniment: 99,
            countermelody: 0,
            percussion: 99,
        };
        hand_edited.validate();
        assert_eq!(
            hand_edited,
            InstrumentCopies {
                melody: InstrumentCopies::MIN,
                accompaniment: InstrumentCopies::MAX,
                countermelody: InstrumentCopies::MIN,
                percussion: InstrumentCopies::MAX_PERCUSSION,
            }
        );
        // The renderer accessors clamp too, so a value from a file that never
        // went through `validate` cannot ask for a channel the plan lacks.
        let raw = InstrumentCopies {
            melody: 0,
            accompaniment: 200,
            countermelody: 0,
            percussion: 200,
        };
        assert_eq!(raw.melody_count(), 1);
        assert_eq!(raw.accompaniment_count(), 4);
        assert_eq!(raw.percussion_count(), 3);
    }

    #[test]
    fn older_files_get_one_copy_of_every_part() {
        // The copy counts are a plain serde default: no migration tag and no
        // schema bump, because one player is the previous behaviour.
        let prefs = Preferences::from_json(r#"{"schema_version": 11}"#).expect("deserialize");
        assert_eq!(prefs.instrument_copies, InstrumentCopies::default());
        assert_eq!(prefs.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn percussion_is_off_by_default_and_round_trips() {
        let prefs = Preferences::default();
        assert_eq!(prefs.percussion, Percussion::Off);
        assert!(!prefs.percussion.is_enabled());
        assert_eq!(prefs.percussion_rhythm, PercussionRhythm::Auto);
        assert_eq!(prefs.percussion_volume, 0.5);

        for style in [Percussion::FootTaps, Percussion::Drumset] {
            for rhythm in PercussionRhythm::ALL {
                let prefs = Preferences {
                    percussion: style,
                    percussion_rhythm: rhythm,
                    percussion_volume: 0.8,
                    ..Preferences::default()
                };
                let json = prefs.to_json().expect("serialize");
                let restored = Preferences::from_json(&json).expect("deserialize");
                assert_eq!(restored.percussion, style);
                assert_eq!(restored.percussion_rhythm, rhythm);
                assert_eq!(restored.percussion_volume, 0.8);
                assert!(restored.percussion.is_enabled());
            }
        }
    }

    #[test]
    fn older_files_get_the_default_trio() {
        let json = r#"{"schema_version": 6}"#;
        let prefs = Preferences::from_json(json).expect("deserialize");
        assert_eq!(prefs.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(prefs.second_accompaniment, Countermelody::Accordion);
    }

    #[test]
    fn tune_rotation_is_off_by_default_and_round_trips() {
        let prefs = Preferences::default();
        assert!(!prefs.tune_rotation.enabled);
        assert_eq!(prefs.tune_rotation.passes_per_tune, 8);
        assert!(prefs.tune_rotation.tunes.is_empty());

        let mut prefs = Preferences {
            tune_rotation: TuneRotation {
                enabled: true,
                passes_per_tune: 4,
                tunes: vec![
                    SourceRef::new("abcsessiontunes", "1", "https://example.invalid/1"),
                    SourceRef::new("abcsessiontunes", "2", "https://example.invalid/2"),
                ],
                source: TuneSetSource::List,
            },
            ..Preferences::default()
        };
        prefs.validate();
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.tune_rotation, prefs.tune_rotation);
    }

    #[test]
    fn tune_rotation_adds_without_duplicating_and_clamps_the_interval() {
        let mut rotation = TuneRotation::default();
        let first = SourceRef::new("abcsessiontunes", "1", "https://example.invalid/1");
        rotation.add(first.clone());
        rotation.add(first.clone());
        assert_eq!(rotation.tunes.len(), 1);
        assert!(rotation.contains(&first));

        rotation.remove("abcsessiontunes", "1");
        assert!(rotation.tunes.is_empty());

        rotation.passes_per_tune = 0;
        rotation.validate();
        assert_eq!(rotation.passes_per_tune, TuneRotation::MIN_PASSES);
        rotation.passes_per_tune = 999;
        rotation.validate();
        assert_eq!(rotation.passes_per_tune, TuneRotation::MAX_PASSES);
    }

    #[test]
    fn tune_rotation_duration_follows_the_tempo() {
        let rotation = TuneRotation {
            passes_per_tune: 8,
            ..TuneRotation::default()
        };
        // Eight 32-second passes at 240 BPM.
        assert!((rotation.seconds_per_tune(240.0) - 256.0).abs() < 1e-9);
        assert!((rotation.seconds_per_tune(120.0) - 512.0).abs() < 1e-9);
    }

    #[test]
    fn custom_safety_padding_round_trips() {
        let mut prefs = Preferences::default();
        prefs.lead.padding_seconds = 0.8;
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.lead.padding_seconds, 0.8);
    }

    #[test]
    fn accompaniment_variations_can_be_disabled_and_round_trip() {
        let prefs = Preferences {
            accompaniment_variations: false,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert!(!restored.accompaniment_variations);
    }

    #[test]
    fn animations_can_be_disabled_and_round_trip() {
        let prefs = Preferences {
            animations_enabled: false,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert!(!restored.animations_enabled);
    }

    #[test]
    fn migration_preserves_custom_lead_values() {
        let mut old = Preferences {
            schema_version: 1,
            applied_migrations: BTreeSet::new(),
            ..Preferences::default()
        };
        old.lead.min_beats = 2.0;
        old.lead.syllables_per_second = 7.25;
        let json = old.to_json().expect("serialize");
        let migrated = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(migrated.lead.min_beats, 2.0);
        assert_eq!(migrated.lead.syllables_per_second, 7.25);
    }

    #[test]
    fn instrument_gm_programs_are_sensible() {
        assert_eq!(MelodyInstrument::Fiddle.gm_program(), 40);
        assert_eq!(MelodyInstrument::Accordion.gm_program(), 21);
        assert_eq!(MelodyInstrument::AltoSax.gm_program(), 65);
        assert_eq!(MelodyInstrument::TenorSax.gm_program(), 66);
        assert_eq!(MelodyInstrument::Clarinet.gm_program(), 71);
        assert_eq!(AccompanimentInstrument::Piano.gm_program(), 0);
        assert_eq!(AccompanimentInstrument::Guitar.gm_program(), 25);
    }

    /// Every melody choice must offer a distinct label and a distinct General
    /// MIDI program, and survive a preferences round trip: the picker shows the
    /// label, the renderer sends the program.
    #[test]
    fn every_melody_instrument_round_trips_with_its_own_label_and_program() {
        let mut labels = BTreeSet::new();
        let mut programs = BTreeSet::new();
        for melody in MelodyInstrument::ALL {
            let label = melody.label();
            assert!(!label.trim().is_empty());
            assert!(labels.insert(label), "duplicate label {label:?}");
            assert!(
                programs.insert(melody.gm_program()),
                "duplicate program {} for {label:?}",
                melody.gm_program()
            );

            let prefs = Preferences {
                melody,
                ..Preferences::default()
            };
            let json = prefs.to_json().expect("serialize");
            let restored = Preferences::from_json(&json).expect("deserialize");
            assert_eq!(restored.melody, melody);
            assert_eq!(restored.melody.label(), label);
        }
        assert_eq!(labels.len(), MelodyInstrument::ALL.len());
    }

    #[test]
    fn a_file_records_every_migration_it_has_seen() {
        let prefs = Preferences::from_json(r#"{"schema_version": 7}"#).expect("deserialize");
        assert_eq!(prefs.schema_version, CURRENT_SCHEMA_VERSION);
        for migration in MIGRATIONS {
            assert!(
                prefs.applied_migrations.contains(migration.tag),
                "{} should be recorded",
                migration.tag
            );
        }
    }

    /// Two branches that add a setting in the same release used to collide on
    /// `schema_version`: the first block advanced the version past the second,
    /// so one migration silently never ran.
    #[test]
    fn migrations_sharing_a_version_all_run() {
        let mut prefs = Preferences::default();
        let migrations = [
            Migration {
                tag: "branch-a",
                version: 7,
                apply: |prefs| prefs.tts_rate = 1.3,
            },
            Migration {
                tag: "branch-b",
                version: 7,
                apply: |prefs| prefs.master_volume = 0.25,
            },
        ];
        prefs.migrate_with(&migrations);
        assert_eq!(prefs.tts_rate, 1.3);
        assert_eq!(prefs.master_volume, 0.25);
        assert!(prefs.applied_migrations.contains("branch-a"));
        assert!(prefs.applied_migrations.contains("branch-b"));
    }

    #[test]
    fn a_recorded_migration_does_not_run_again() {
        let mut prefs = Preferences::default();
        prefs.applied_migrations.insert("branch-a".into());
        let migrations = [Migration {
            tag: "branch-a",
            version: 7,
            apply: |prefs| prefs.tts_rate = 0.5,
        }];
        prefs.migrate_with(&migrations);
        assert_eq!(prefs.tts_rate, 1.0);
    }

    #[test]
    fn a_file_without_tags_does_not_rerun_old_migrations() {
        // A v6 file still holds the v1 lead default only because the user
        // chose it; the version says the v2 migration is already done.
        let json = r#"{"schema_version": 7, "lead": {"syllables_per_second": 6.5}}"#;
        let prefs = Preferences::from_json(json).expect("deserialize");
        assert_eq!(prefs.lead.syllables_per_second, 6.5);
    }

    #[test]
    fn changes_since_routes_settings_to_their_subsystems() {
        let base = Preferences::default();
        assert_eq!(base.changes_since(&base), ChangeSet::default());

        let mut tempo = base.clone();
        tempo.tempo_bpm = 220.0;
        assert_eq!(
            tempo.changes_since(&base),
            ChangeSet {
                timeline: true,
                audio: true,
                tone: false
            }
        );

        let mut aliases = base.clone();
        aliases.aliases.set_custom("custom", "improper");
        assert_eq!(
            aliases.changes_since(&base),
            ChangeSet {
                timeline: true,
                audio: false,
                tone: false
            }
        );

        let mut melody = base.clone();
        melody.melody = MelodyInstrument::Accordion;
        assert_eq!(
            melody.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: true,
                tone: false
            }
        );

        let mut effects = base.clone();
        effects.effects.reverb = crate::effects::ReverbPreset::Hall;
        assert_eq!(
            effects.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: true,
                tone: false
            }
        );

        let mut tone = base.clone();
        tone.tts_tone = CallTone::Natural;
        assert_eq!(
            tone.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: false,
                tone: true
            }
        );

        let mut second = base.clone();
        second.second_accompaniment = Countermelody::Off;
        assert_eq!(
            second.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: true,
                tone: false
            }
        );

        let mut percussion = base.clone();
        percussion.percussion = Percussion::FootTaps;
        assert_eq!(
            percussion.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: true,
                tone: false
            }
        );

        let mut copies = base.clone();
        copies.instrument_copies.melody = 2;
        assert_eq!(
            copies.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: true,
                tone: false
            }
        );

        let mut rhythm_changed = base.clone();
        rhythm_changed.percussion_rhythm = PercussionRhythm::Shuffle;
        assert_eq!(
            rhythm_changed.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: true,
                tone: false
            }
        );

        // Volume and appearance changes are applied without a rebuild.
        let mut volume = base.clone();
        volume.master_volume = 0.1;
        assert!(!volume.changes_since(&base).any());

        let mut caller = base.clone();
        caller.caller_volume = 0.2;
        assert!(
            !caller.changes_since(&base).any(),
            "the caller level is applied live, not by re-rendering the band"
        );

        let mut output = base.clone();
        output.output_mode = OutputMode::Surround;
        assert!(
            output.changes_since(&base).audio,
            "the output layout changes the rendered channels"
        );

        let mut band = base.clone();
        band.band[0].volume = 0.4;
        assert!(
            band.changes_since(&base).audio,
            "a slot mix change re-renders the band"
        );

        let mut theme = base.clone();
        theme.theme = Theme::Light;
        assert!(!theme.changes_since(&base).any());
    }

    #[test]
    fn a_band_always_keeps_a_pitched_slot() {
        let mut prefs = Preferences {
            band: vec![BandSlot {
                part: BandPart::Percussion,
                voice: BandVoice::FootTaps,
                players: 1,
                volume: 0.5,
                pan: 0.0,
                depth: 0.2,
            }],
            ..Preferences::default()
        };
        prefs.validate();
        assert!(
            prefs.has_pitched_band(),
            "a percussion-only file must gain a playable instrument"
        );
        assert_eq!(
            prefs.band[0].voice,
            BandVoice::Fiddle,
            "the fallback comes first"
        );

        prefs.band.clear();
        prefs.validate();
        assert_eq!(
            prefs.band,
            default_band(),
            "an empty list becomes the default band"
        );
    }

    #[test]
    fn a_slot_reports_its_stage_direction() {
        let mut slot = BandSlot {
            pan: -0.7,
            depth: 0.0,
            ..BandSlot::default()
        };
        assert_eq!(slot.direction_label(), "Left front");
        slot.depth = 0.6;
        assert_eq!(slot.direction_label(), "Left rear");
        slot.pan = 0.0;
        assert_eq!(slot.direction_label(), "Centre rear");
        slot.depth = 0.0;
        slot.pan = 0.8;
        assert_eq!(slot.direction_label(), "Right front");
    }

    #[test]
    fn voices_map_to_the_legacy_roles() {
        assert_eq!(
            BandVoice::from_melody(MelodyInstrument::Clarinet),
            BandVoice::Clarinet
        );
        assert_eq!(
            BandVoice::from_accompaniment(AccompanimentInstrument::Guitar),
            BandVoice::Guitar
        );
        assert_eq!(
            BandVoice::from_percussion(Percussion::Off),
            None,
            "Off has no voice"
        );
        assert_eq!(
            BandVoice::Fiddle.accompaniment_instrument(),
            Some(AccompanimentInstrument::Accordion)
        );
        assert_eq!(BandVoice::Drumset.accompaniment_instrument(), None);
        assert!(BandVoice::Piano.is_pitched());
        assert!(!BandVoice::Drumset.is_pitched());
    }
}
