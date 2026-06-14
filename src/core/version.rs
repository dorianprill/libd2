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
    use super::{CharacterStatus, GameEdition, SaveVersion, detect_edition};

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
}
