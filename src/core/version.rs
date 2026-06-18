#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEdition {
    Classic,
    LordOfDestruction,
    Resurrected,
    ReignOfTheWarlock,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveVersion {
    PreLod(u32),
    Classic108,
    Lod107Or108,
    Standard109,
    Lod110Plus,
    Resurrected(u32),
    Unknown(u32),
}

/// In-game expansion mode for a character save.
///
/// Older Classic/LoD saves encode this through the legacy status expansion bit.
/// Resurrected v105-family saves encode it through a separate mode marker byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionMode {
    Classic,
    Expansion,
    RotW,
    Unknown(u8),
}

impl ExpansionMode {
    pub const V105_CLASSIC_MARKER: u8 = 0x01;
    pub const V105_EXPANSION_MARKER: u8 = 0x02;
    pub const V105_ROTW_MARKER: u8 = 0x03;

    pub const fn from_legacy_status(status: CharacterStatus) -> Self {
        if status.expansion {
            Self::Expansion
        } else {
            Self::Classic
        }
    }

    pub const fn from_v105_marker(marker: u8) -> Self {
        match marker {
            Self::V105_CLASSIC_MARKER => Self::Classic,
            Self::V105_EXPANSION_MARKER => Self::Expansion,
            Self::V105_ROTW_MARKER => Self::RotW,
            other => Self::Unknown(other),
        }
    }

    pub const fn to_v105_marker(self) -> Option<u8> {
        match self {
            Self::Classic => Some(Self::V105_CLASSIC_MARKER),
            Self::Expansion => Some(Self::V105_EXPANSION_MARKER),
            Self::RotW => Some(Self::V105_ROTW_MARKER),
            Self::Unknown(_) => None,
        }
    }

    pub const fn legacy_status_expansion(self) -> bool {
        matches!(self, Self::Expansion | Self::RotW)
    }
}

impl SaveVersion {
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            0x47 => Self::PreLod(raw),
            0x57 => Self::Lod107Or108,
            0x59 => Self::Classic108,
            0x5c => Self::Standard109,
            0x60 => Self::Lod110Plus,
            0x61..=u32::MAX => Self::Resurrected(raw),
            other => Self::Unknown(other),
        }
    }

    pub fn raw(self) -> u32 {
        match self {
            Self::PreLod(raw) | Self::Resurrected(raw) | Self::Unknown(raw) => raw,
            Self::Lod107Or108 => 0x57,
            Self::Classic108 => 0x59,
            Self::Standard109 => 0x5c,
            Self::Lod110Plus => 0x60,
        }
    }

    pub fn uses_resurrected_item_encoding(self) -> bool {
        matches!(self, Self::Resurrected(_))
    }

    pub fn uses_v105_header(self) -> bool {
        matches!(self, Self::Resurrected(raw) if raw >= 105)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharacterStatus {
    pub hardcore: bool,
    pub died: bool,
    pub expansion: bool,
    pub ladder: bool,
}

impl CharacterStatus {
    const HARDCORE: u8 = 1 << 2;
    const DIED: u8 = 1 << 3;
    const EXPANSION: u8 = 1 << 5;
    const LADDER: u8 = 1 << 6;

    pub fn from_byte(value: u8) -> Self {
        Self {
            hardcore: value & Self::HARDCORE != 0,
            died: value & Self::DIED != 0,
            expansion: value & Self::EXPANSION != 0,
            ladder: value & Self::LADDER != 0,
        }
    }

    pub fn to_byte(self) -> u8 {
        let mut value = 0;

        if self.hardcore {
            value |= Self::HARDCORE;
        }
        if self.died {
            value |= Self::DIED;
        }
        if self.expansion {
            value |= Self::EXPANSION;
        }
        if self.ladder {
            value |= Self::LADDER;
        }

        value
    }
}

pub fn detect_edition(version: SaveVersion, status: CharacterStatus) -> GameEdition {
    if version.uses_resurrected_item_encoding() {
        GameEdition::Resurrected
    } else if status.expansion {
        GameEdition::LordOfDestruction
    } else {
        GameEdition::Classic
    }
}

#[cfg(test)]
mod tests {
    use super::{CharacterStatus, ExpansionMode, GameEdition, SaveVersion, detect_edition};

    #[test]
    fn character_status_round_trips_known_bits() {
        let status = CharacterStatus {
            hardcore: true,
            died: false,
            expansion: true,
            ladder: true,
        };

        assert_eq!(CharacterStatus::from_byte(status.to_byte()), status);
    }

    #[test]
    fn save_version_and_status_detect_runtime_edition() {
        assert_eq!(
            detect_edition(SaveVersion::Lod110Plus, CharacterStatus::default()),
            GameEdition::Classic
        );

        assert_eq!(
            detect_edition(
                SaveVersion::Lod110Plus,
                CharacterStatus {
                    expansion: true,
                    ..CharacterStatus::default()
                }
            ),
            GameEdition::LordOfDestruction
        );

        assert_eq!(
            detect_edition(SaveVersion::from_raw(0x62), CharacterStatus::default()),
            GameEdition::Resurrected
        );
    }

    #[test]
    fn expansion_mode_maps_v105_markers() {
        assert_eq!(
            ExpansionMode::from_v105_marker(0x01),
            ExpansionMode::Classic
        );
        assert_eq!(
            ExpansionMode::from_v105_marker(0x02),
            ExpansionMode::Expansion
        );
        assert_eq!(ExpansionMode::from_v105_marker(0x03), ExpansionMode::RotW);
        assert_eq!(
            ExpansionMode::from_v105_marker(0x7f),
            ExpansionMode::Unknown(0x7f)
        );
        assert_eq!(ExpansionMode::RotW.to_v105_marker(), Some(0x03));
        assert_eq!(ExpansionMode::Unknown(0).to_v105_marker(), None);
    }
}
