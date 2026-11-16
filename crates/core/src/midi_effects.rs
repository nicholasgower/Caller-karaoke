//! Effect sends written into an exported Standard MIDI File.
//!
//! A MIDI file cannot carry rendered audio, so these settings travel as the
//! General MIDI effect sends a player applies while it renders the file:
//! reverb (controller 91), chorus (controller 93), and echo/delay (controller
//! 94, the GM2 delay send). The reverb choice also selects a GM2 reverb type,
//! so a GM2 player really uses a room of the requested size instead of its
//! default algorithm.
//!
//! They deliberately do not affect in-app playback. The soundfont synthesizer
//! already adds its own reverb and chorus while rendering the loop.

use serde::{Deserialize, Serialize};

/// Reverb character requested from the MIDI player.
///
/// The send level is a General MIDI controller value; the GM2 type selects one
/// of the standard room algorithms on players that support it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReverbPreset {
    /// No reverb send.
    Off,
    /// A small, short room.
    SmallRoom,
    /// A medium room, the default: audible but not washy.
    #[default]
    MediumRoom,
    /// A large room suitable for a lively band.
    LargeRoom,
    /// A concert hall with the longest tail.
    Hall,
}

impl ReverbPreset {
    /// Every choice, in display order.
    pub const ALL: [Self; 5] = [
        Self::Off,
        Self::SmallRoom,
        Self::MediumRoom,
        Self::LargeRoom,
        Self::Hall,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::SmallRoom => "Small room",
            Self::MediumRoom => "Medium room",
            Self::LargeRoom => "Large room",
            Self::Hall => "Hall",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Self::Off => "No reverb send.",
            Self::SmallRoom => "A short, intimate room (about 5 m).",
            Self::MediumRoom => "A medium room (about 10 m), the default.",
            Self::LargeRoom => "A large room suitable for a live band.",
            Self::Hall => "A concert hall with the longest tail.",
        }
    }

    /// General MIDI reverb send level (controller 91), 0-127.
    pub fn send_level(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::SmallRoom => 38,
            Self::MediumRoom => 64,
            Self::LargeRoom => 88,
            Self::Hall => 112,
        }
    }

    /// Index into the GM2 reverb types, or `None` when reverb is off.
    ///
    /// The order matches the table in the GM2 global-parameter control:
    /// small room, medium room, large room, large hall.
    pub fn gm2_type_index(self) -> Option<usize> {
        match self {
            Self::Off => None,
            Self::SmallRoom => Some(0),
            Self::MediumRoom => Some(1),
            Self::LargeRoom => Some(2),
            Self::Hall => Some(3),
        }
    }
}

/// How much chorus the exported file asks the player to add.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChorusPreset {
    /// No chorus send.
    #[default]
    Off,
    /// A light shimmer that keeps the tune clear.
    Subtle,
    /// A wide chorus, useful for a very dry player.
    Rich,
}

impl ChorusPreset {
    /// Every choice, in display order.
    pub const ALL: [Self; 3] = [Self::Off, Self::Subtle, Self::Rich];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Subtle => "Subtle",
            Self::Rich => "Rich",
        }
    }

    /// General MIDI chorus send level (controller 93), 0-127.
    pub fn send_level(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Subtle => 40,
            Self::Rich => 88,
        }
    }
}

/// How much echo/delay the exported file asks the player to add.
///
/// This uses the GM2 delay send (controller 94). Players that only implement
/// General MIDI level 1 treat the controller as an unspecified effect depth and
/// may ignore it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EchoPreset {
    /// No echo send.
    #[default]
    Off,
    /// A short slapback that widens the tune without smearing the beat.
    Subtle,
    /// A longer echo for a spacious, programmatic sound.
    Strong,
}

impl EchoPreset {
    /// Every choice, in display order.
    pub const ALL: [Self; 3] = [Self::Off, Self::Subtle, Self::Strong];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Subtle => "Subtle",
            Self::Strong => "Strong",
        }
    }

    /// General MIDI delay/echo send level (controller 94), 0-127.
    pub fn send_level(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Subtle => 48,
            Self::Strong => 96,
        }
    }
}

/// Reverb, chorus, and echo sends for an exported MIDI file.
///
/// The default is a medium-room reverb with no chorus or echo, which gives a
/// dry contra band a little room without changing the tune.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MidiEffects {
    pub reverb: ReverbPreset,
    pub chorus: ChorusPreset,
    pub echo: EchoPreset,
}

impl MidiEffects {
    /// Whether every send is off.
    pub fn is_dry(self) -> bool {
        self.reverb == ReverbPreset::Off
            && self.chorus == ChorusPreset::Off
            && self.echo == EchoPreset::Off
    }

    /// Controller/value pairs to write at the start of every channel track.
    ///
    /// Controller 91 is the reverb send, 93 the chorus send, and 94 the GM2
    /// delay send. All three are written even when a send is off, so the file
    /// overrides a player's own default effect rather than inheriting it.
    pub fn sends(self) -> [(u8, u8); 3] {
        [
            (91, self.reverb.send_level()),
            (93, self.chorus.send_level()),
            (94, self.echo.send_level()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_a_medium_room_with_no_chorus_or_echo() {
        let effects = MidiEffects::default();
        assert_eq!(effects.reverb, ReverbPreset::MediumRoom);
        assert_eq!(effects.chorus, ChorusPreset::Off);
        assert_eq!(effects.echo, EchoPreset::Off);
        assert_eq!(effects.sends(), [(91, 64), (93, 0), (94, 0)]);
        assert!(!effects.is_dry());
    }

    #[test]
    fn every_preset_has_a_distinct_send_level_in_range() {
        for preset in ReverbPreset::ALL {
            assert!(preset.send_level() <= 127);
        }
        for preset in ChorusPreset::ALL {
            assert!(preset.send_level() <= 127);
        }
        for preset in EchoPreset::ALL {
            assert!(preset.send_level() <= 127);
        }
        assert!(ReverbPreset::Hall.send_level() > ReverbPreset::MediumRoom.send_level());
        assert!(ChorusPreset::Rich.send_level() > ChorusPreset::Subtle.send_level());
        assert!(EchoPreset::Strong.send_level() > EchoPreset::Subtle.send_level());
    }

    #[test]
    fn off_is_completely_dry() {
        let effects = MidiEffects {
            reverb: ReverbPreset::Off,
            chorus: ChorusPreset::Off,
            echo: EchoPreset::Off,
        };
        assert!(effects.is_dry());
        assert_eq!(effects.sends(), [(91, 0), (93, 0), (94, 0)]);
    }
}
