use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crate::core::character_class::CharacterClass;
use crate::core::entity::player::Player;
use crate::core::game_state::GameState;
use crate::core::inventory::InventoryProfile;
use crate::core::object::item::{Item, ItemContainer, ItemDestination, ItemOwner, ItemQuality};
use crate::core::unit_stat::UnitStat;
use crate::core::version::{
    CharacterStatus, ExpansionMode, GameEdition, SaveVersion, detect_edition,
};
use crate::core::{quest, waypoint};

const D2S_MAGIC: u32 = 0xaa55_aa55;
const VERSION_OFFSET: usize = 0x04;
const FILE_SIZE_OFFSET: usize = 0x08;
const CHECKSUM_OFFSET: usize = 0x0c;
const LEGACY_NAME_OFFSET: usize = 0x14;
const D2R_LEGACY_NAME_OFFSET: usize = 0x010b;
const D2R_V105_NAME_OFFSET: usize = 0x012b;
const CHARACTER_NAME_LEN: usize = 16;
const LEGACY_STATUS_OFFSET: usize = 0x24;
const LEGACY_CLASS_OFFSET: usize = 0x28;
const LEGACY_LEVEL_OFFSET: usize = 0x2b;
const D2R_V105_STATUS_OFFSET: usize = 0x14;
const D2R_V105_PROGRESSION_OFFSET: usize = 0x15;
const D2R_V105_CLASS_OFFSET: usize = 0x18;
const D2R_V105_RESERVED_VERSION_MARKERS_OFFSET: usize = 0x19;
const D2R_V105_LEVEL_OFFSET: usize = 0x1b;
const D2R_V105_RESERVED_CHECKSUM_MASK_OFFSET: usize = 0x24;
const D2R_V105_ASSIGNED_SKILLS_OFFSET: usize = 0x28;
const D2R_V105_LEFT_SKILL_OFFSET: usize = 0x68;
const D2R_V105_RIGHT_SKILL_OFFSET: usize = 0x6c;
const D2R_V105_LEFT_SWAP_SKILL_OFFSET: usize = 0x70;
const D2R_V105_RIGHT_SWAP_SKILL_OFFSET: usize = 0x74;
const D2R_V105_APPEARANCE_OFFSET: usize = 0x78;
const D2R_V105_DIFFICULTY_OFFSET: usize = 0x98;
const D2R_V105_MAP_ID_OFFSET: usize = 0x9b;
const D2R_V105_MERC_NAME_SEED_OFFSET: usize = 0x0a3;
const D2R_V105_MERC_STATUS_OFFSET: usize = 0x0a7;
const D2R_V105_MERC_ID_OFFSET: usize = 0x0a9;
const D2R_V105_MERC_XP_OFFSET: usize = 0x0ab;
const D2R_V105_MODE_MARKER_OFFSET: usize = 0x0f8;
const D2R_V105_HEADER_LEN: usize = 0x150;
const LEGACY_FULL_EXPORT_PRE_STATS_LEN: usize = 0x2fd;
const V105_FULL_EXPORT_PRE_STATS_LEN: usize = 0x341;
const V105_QUEST_HEADER_OFFSET: usize = 0x193;
const V105_WAYPOINT_HEADER_OFFSET: usize = 0x2bd;
const V105_NPC_HEADER_OFFSET: usize = 0x30d;
const LEGACY_ASSIGNED_SKILLS_OFFSET: usize = 0x38;
const LEGACY_LEFT_SKILL_OFFSET: usize = 0x78;
const LEGACY_RIGHT_SKILL_OFFSET: usize = 0x7c;
const LEGACY_LEFT_SWAP_SKILL_OFFSET: usize = 0x80;
const LEGACY_RIGHT_SWAP_SKILL_OFFSET: usize = 0x84;
const LEGACY_APPEARANCE_OFFSET: usize = 0x88;
const LEGACY_DIFFICULTY_OFFSET: usize = 0xa8;
const LEGACY_MAP_ID_OFFSET: usize = 0xab;
const LEGACY_MERCENARY_OFFSET: usize = 0xb1;
const LEGACY_REALM_DATA_OFFSET: usize = 0xbf;
const LEGACY_QUEST_UNKNOWN_OFFSET: usize = 0x14b;
const LEGACY_QUEST_HEADER_OFFSET: usize = 0x14f;
const LEGACY_QUEST_MAGIC_OFFSET: usize = 0x153;
const LEGACY_WAYPOINT_HEADER_OFFSET: usize = 0x279;
const LEGACY_WAYPOINT_MAGIC_OFFSET: usize = 0x27b;
const LEGACY_WAYPOINT_DIFFICULTIES_OFFSET: usize = 0x281;
const LEGACY_WAYPOINT_DIFFICULTY_LEN: usize = 24;
const LEGACY_WAYPOINT_TRAILER_OFFSET: usize = 0x2c9;
const LEGACY_NPC_HEADER_OFFSET: usize = 0x2ca;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveSectionMarker {
    Stats,
    Skills,
    ItemList,
    Corpse,
    IronGolem,
    Followers,
}

impl SaveSectionMarker {
    pub fn raw_bytes(self) -> &'static [u8; 2] {
        match self {
            // Literal .d2s section markers used by the game save format.
            Self::Stats => b"gf",
            Self::Skills => b"if",
            Self::ItemList => b"JM",
            Self::Corpse => b"jf",
            Self::IronGolem => b"kf",
            Self::Followers => b"lf",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Stats => "character stats",
            Self::Skills => "character skills",
            Self::ItemList => "item list",
            Self::Corpse => "corpse marker",
            Self::IronGolem => "iron golem",
            Self::Followers => "followers",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterHeader {
    pub layout: CharacterHeaderLayout,
    pub version_raw: u32,
    pub save_version: SaveVersion,
    pub file_size: u32,
    pub checksum: u32,
    pub name: String,
    pub status: CharacterStatus,
    pub progression: Option<CharacterProgression>,
    pub class_id: u8,
    pub class: Option<CharacterClass>,
    pub level: u8,
    pub mercenary: Option<MercenaryHeader>,
    pub edition: GameEdition,
    pub expansion_mode: ExpansionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterHeaderLayout {
    Legacy,
    ResurrectedLegacy,
    ResurrectedV105,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterProgression {
    Normal,
    Nightmare,
    Hell,
    Unknown(u8),
}

impl CharacterProgression {
    pub fn from_v105_byte(value: u8) -> Self {
        match value {
            0x00 => Self::Normal,
            0x05 => Self::Nightmare,
            0x0f => Self::Hell,
            other => Self::Unknown(other),
        }
    }

    pub fn to_v105_byte(self) -> u8 {
        match self {
            Self::Normal => 0x00,
            Self::Nightmare => 0x05,
            Self::Hell => 0x0f,
            Self::Unknown(other) => other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MercenaryHeader {
    pub name_seed: u32,
    pub status: u16,
    pub hireling_id: u16,
    pub experience: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterStats {
    pub marker_offset: Option<usize>,
    pub entries: Vec<CharacterStatEntry>,
    pub terminator_found: bool,
}

impl CharacterStats {
    pub fn missing() -> Self {
        Self {
            marker_offset: None,
            entries: Vec::new(),
            terminator_found: false,
        }
    }

    pub fn get(&self, stat: CharacterStat) -> Option<u32> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.stat == Some(stat))
            .map(|entry| entry.value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterStatEntry {
    pub id: u16,
    pub stat: Option<CharacterStat>,
    pub value: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CharacterStat {
    Strength = 0,
    Energy = 1,
    Dexterity = 2,
    Vitality = 3,
    StatPoints = 4,
    SkillPoints = 5,
    HitPoints = 6,
    MaxHitPoints = 7,
    Mana = 8,
    MaxMana = 9,
    Stamina = 10,
    MaxStamina = 11,
    Level = 12,
    Experience = 13,
    Gold = 14,
    StashedGold = 15,
}

impl CharacterStat {
    pub fn from_id(id: u16) -> Option<Self> {
        match id {
            0 => Some(Self::Strength),
            1 => Some(Self::Energy),
            2 => Some(Self::Dexterity),
            3 => Some(Self::Vitality),
            4 => Some(Self::StatPoints),
            5 => Some(Self::SkillPoints),
            6 => Some(Self::HitPoints),
            7 => Some(Self::MaxHitPoints),
            8 => Some(Self::Mana),
            9 => Some(Self::MaxMana),
            10 => Some(Self::Stamina),
            11 => Some(Self::MaxStamina),
            12 => Some(Self::Level),
            13 => Some(Self::Experience),
            14 => Some(Self::Gold),
            15 => Some(Self::StashedGold),
            _ => None,
        }
    }

    pub fn bit_width(self) -> u8 {
        match self {
            Self::Strength | Self::Energy | Self::Dexterity | Self::Vitality | Self::StatPoints => {
                10
            }
            Self::SkillPoints => 8,
            Self::HitPoints
            | Self::MaxHitPoints
            | Self::Mana
            | Self::MaxMana
            | Self::Stamina
            | Self::MaxStamina => 21,
            Self::Level => 7,
            Self::Experience => 32,
            Self::Gold | Self::StashedGold => 25,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterSkills {
    pub marker_offset: usize,
    pub levels: [u8; 30],
}

impl CharacterSkills {
    pub fn level_at_slot(&self, slot: usize) -> Option<u8> {
        self.levels.get(slot).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CharacterItemLists {
    pub item_lists: Vec<ItemListHeader>,
    pub corpse_marker_offset: Option<usize>,
    pub iron_golem: Option<IronGolemHeader>,
    pub follower_block: Option<FollowerBlockHeader>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemListHeader {
    pub marker_offset: usize,
    pub parent_item_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IronGolemHeader {
    pub marker_offset: usize,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowerBlockHeader {
    pub marker_offset: usize,
    pub follower_count: u16,
    pub payload_len: usize,
    pub complete_116_byte_payloads: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterFile {
    raw: Vec<u8>,
    header: CharacterHeader,
    inventory_profile: InventoryProfile,
    stats: CharacterStats,
    skills: Option<CharacterSkills>,
    item_lists: CharacterItemLists,
}

/// Options for exporting a legacy Classic/LoD `.d2s` snapshot from live state.
///
/// By default libd2 derives the 30-byte save `if` skill table from skill ids
/// captured in D2GS `0x94 PlayerSkillsInfo`. Tests and recovery tools can still
/// supply an explicit table when they need to preserve fixture bytes or when a
/// capture did not include the skill-list packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharacterExportOptions {
    skills: Option<[u8; 30]>,
}

impl CharacterExportOptions {
    pub fn new(skills: [u8; 30]) -> Self {
        Self {
            skills: Some(skills),
        }
    }

    pub fn from_game_state_skills() -> Self {
        Self::default()
    }

    pub fn empty_skills() -> Self {
        Self {
            skills: Some([0; 30]),
        }
    }

    pub fn skills(&self) -> Option<&[u8; 30]> {
        self.skills.as_ref()
    }
}

impl CharacterFile {
    pub fn parse(bytes: impl Into<Vec<u8>>) -> Result<Self, CharacterFileError> {
        let raw = bytes.into();
        let header = parse_header(&raw)?;
        let stats = parse_character_stats(&raw, header.layout);
        let skills = parse_character_skills(&raw, header.layout)?;
        let item_lists = parse_item_lists(&raw, header.layout);
        let inventory_profile = InventoryProfile::for_edition(header.edition);

        Ok(Self {
            raw,
            header,
            inventory_profile,
            stats,
            skills,
            item_lists,
        })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, CharacterFileError> {
        let bytes = fs::read(path)?;
        Self::parse(bytes)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), CharacterFileError> {
        fs::write(path, self.to_bytes())?;
        Ok(())
    }

    pub fn header(&self) -> &CharacterHeader {
        &self.header
    }

    pub fn inventory_profile(&self) -> InventoryProfile {
        self.inventory_profile
    }

    pub fn stats(&self) -> &CharacterStats {
        &self.stats
    }

    pub fn stat(&self, stat: CharacterStat) -> Option<u32> {
        self.stats.get(stat)
    }

    pub fn skills(&self) -> Option<&CharacterSkills> {
        self.skills.as_ref()
    }

    pub fn item_lists(&self) -> &CharacterItemLists {
        &self.item_lists
    }

    pub fn raw_bytes(&self) -> &[u8] {
        &self.raw
    }

    pub fn into_raw_bytes(self) -> Vec<u8> {
        self.raw
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut raw = self.raw.clone();
        fix_header(&mut raw);
        raw
    }

    pub fn set_header_fields(
        &mut self,
        name: &str,
        status: CharacterStatus,
        class: CharacterClass,
        level: u8,
        progression: Option<CharacterProgression>,
    ) -> Result<(), CharacterExportError> {
        let layout = self.header.layout;
        let bytes = name.as_bytes();
        let len = bytes.len().min(15);
        let name_offset = layout.name_offset();

        self.raw[name_offset..name_offset + 16].fill(0);
        self.raw[name_offset..name_offset + len].copy_from_slice(&bytes[..len]);

        let encoded_status = if layout == CharacterHeaderLayout::ResurrectedV105 {
            CharacterStatus {
                expansion: false,
                ..status
            }
        } else {
            status
        };

        self.raw[layout.status_offset()] = encoded_status.to_byte();
        self.raw[layout.class_offset()] = class as u8;
        self.raw[layout.level_offset()] = level;

        if let (Some(prog), Some(prog_offset)) = (progression, layout.progression_offset()) {
            self.raw[prog_offset] = prog.to_v105_byte();
        }

        self.header.name = name.to_string();
        self.header.status = encoded_status;
        self.header.class = Some(class);
        self.header.class_id = class as u8;
        self.header.level = level;
        self.header.progression = progression;
        self.header.edition = detect_character_edition(
            self.header.save_version,
            encoded_status,
            self.header.expansion_mode,
            self.header.class_id,
        );

        fix_header(&mut self.raw);
        Ok(())
    }

    pub fn set_expansion_mode(
        &mut self,
        expansion_mode: ExpansionMode,
    ) -> Result<(), CharacterExportError> {
        match self.header.layout {
            CharacterHeaderLayout::ResurrectedV105 => {
                let Some(marker) = expansion_mode.to_v105_marker() else {
                    return Err(CharacterExportError::UnsupportedExpansionMode { expansion_mode });
                };
                self.raw[D2R_V105_MODE_MARKER_OFFSET] = marker;
                self.raw[D2R_V105_STATUS_OFFSET] = CharacterStatus {
                    expansion: false,
                    ..self.header.status
                }
                .to_byte();
            }
            CharacterHeaderLayout::Legacy | CharacterHeaderLayout::ResurrectedLegacy => {
                self.header.status.expansion = expansion_mode.legacy_status_expansion();
                self.raw[self.header.layout.status_offset()] = self.header.status.to_byte();
            }
        }

        self.header.expansion_mode = expansion_mode;
        self.header.status =
            CharacterStatus::from_byte(self.raw[self.header.layout.status_offset()]);
        self.header.edition = detect_character_edition(
            self.header.save_version,
            self.header.status,
            self.header.expansion_mode,
            self.header.class_id,
        );
        fix_header(&mut self.raw);
        Ok(())
    }

    pub fn replace_stats_and_skills(
        &mut self,
        stats_section: &[u8],
        skills_section: &[u8],
    ) -> Result<(), CharacterExportError> {
        let start = self.header.layout.section_search_start();

        let stats_start = find_section_marker(&self.raw, SaveSectionMarker::Stats, start).ok_or(
            CharacterExportError::MissingSection {
                marker: SaveSectionMarker::Stats,
            },
        )?;

        let skills_start =
            find_section_marker(&self.raw, SaveSectionMarker::Skills, stats_start + 2).ok_or(
                CharacterExportError::MissingSection {
                    marker: SaveSectionMarker::Skills,
                },
            )?;

        if self.raw.len() < skills_start + 32 {
            return Err(CharacterExportError::MissingSection {
                marker: SaveSectionMarker::Skills,
            });
        }

        // Splice AFTER the 'gf' marker (which is 2 bytes)
        self.raw
            .splice(stats_start + 2..skills_start, stats_section.iter().copied());

        // Find the new skills start (since we just modified the length of the vector)
        let new_skills_start =
            find_section_marker(&self.raw, SaveSectionMarker::Skills, stats_start + 2).unwrap();

        // Splice AFTER the 'if' marker (which is 2 bytes)
        self.raw.splice(
            new_skills_start + 2..new_skills_start + 32,
            skills_section.iter().copied(),
        );

        fix_header(&mut self.raw);
        Ok(())
    }

    pub fn replace_quests(
        &mut self,
        quests: &[[u16; quest::SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3],
    ) -> Result<(), CharacterExportError> {
        let start = self.header.layout.section_search_start();
        quest::write_legacy_quest_words(&mut self.raw, start, quests);
        let progression = quest::progression_from_quests(quests);
        if self.header.layout == CharacterHeaderLayout::ResurrectedV105 {
            self.raw[D2R_V105_PROGRESSION_OFFSET] = progression;
            self.header.progression = Some(CharacterProgression::from_v105_byte(progression));
        } else {
            quest::apply_progression_from_quests(&mut self.raw, start, quests);
        }
        fix_header(&mut self.raw);
        Ok(())
    }

    pub fn replace_waypoints(
        &mut self,
        waypoints: &[[bool; waypoint::WAYPOINT_COUNT]; 3],
    ) -> Result<(), CharacterExportError> {
        let start = self.header.layout.section_search_start();
        if self.header.layout == CharacterHeaderLayout::ResurrectedV105 {
            waypoint::write_v105_waypoints(&mut self.raw, start, waypoints);
        } else {
            waypoint::write_legacy_waypoints(&mut self.raw, start, waypoints);
        }
        fix_header(&mut self.raw);
        Ok(())
    }

    /// Builds a standalone legacy Classic/LoD `.d2s` file from the local player
    /// in a reconstructed [`GameState`].
    ///
    /// The writer emits the fixed pre-stats save block used by LoD 1.10+ saves:
    /// header, empty hotkeys/appearance/location/merc fields, empty quest,
    /// waypoint, and NPC-dialog sections, then generated `gf` stats, generated
    /// `if` class skills, empty player item and corpse lists, and expansion-only
    /// empty merc/golem sections. Item records are intentionally left empty until
    /// live item-to-save serialization is implemented.
    pub fn export_legacy_from_game_state(
        state: &GameState,
        options: CharacterExportOptions,
    ) -> Result<Self, CharacterExportError> {
        let snapshot = LegacyExportSnapshot::from_game_state(state, options)?;
        let stats_section = encode_character_stats_section(&snapshot.stats)?;
        let skills_section = encode_character_skills_section(snapshot.skills);

        let mut raw = build_legacy_fixed_pre_stats_block(&snapshot);
        raw.extend_from_slice(&stats_section);
        raw.extend_from_slice(&skills_section);
        append_legacy_item_sections(&mut raw, state, snapshot.status.expansion);
        fix_header(&mut raw);

        Self::parse(raw).map_err(CharacterExportError::CharacterFile)
    }

    pub fn default_v105(class: CharacterClass, name: &str) -> Result<Self, CharacterExportError> {
        let expansion_mode = if class == CharacterClass::Warlock {
            ExpansionMode::RotW
        } else {
            ExpansionMode::Expansion
        };
        Self::default_v105_with_expansion_mode(class, name, expansion_mode)
    }

    pub fn default_rotw(class: CharacterClass, name: &str) -> Result<Self, CharacterExportError> {
        Self::default_v105_with_expansion_mode(class, name, ExpansionMode::RotW)
    }

    pub fn default_v105_with_expansion_mode(
        class: CharacterClass,
        name: &str,
        expansion_mode: ExpansionMode,
    ) -> Result<Self, CharacterExportError> {
        if class == CharacterClass::Warlock && expansion_mode != ExpansionMode::RotW {
            return Err(CharacterExportError::UnsupportedExpansionMode { expansion_mode });
        }

        let base = crate::core::character_progression::BaseStats::for_class(class);
        let status = CharacterStatus {
            hardcore: false,
            died: false,
            expansion: false,
            ladder: false,
        };
        let edition = if expansion_mode == ExpansionMode::RotW {
            GameEdition::ReignOfTheWarlock
        } else {
            GameEdition::Resurrected
        };
        let snapshot = LegacyExportSnapshot {
            name: name.to_string(),
            status,
            edition,
            expansion_mode,
            class,
            level: 1,
            map_id: 0,
            stats: vec![
                (CharacterStat::Strength, base.str),
                (CharacterStat::Energy, base.eng),
                (CharacterStat::Dexterity, base.dex),
                (CharacterStat::Vitality, base.vit),
                (CharacterStat::StatPoints, 0),
                (CharacterStat::SkillPoints, 0),
                (CharacterStat::HitPoints, base.hp << 8),
                (CharacterStat::MaxHitPoints, base.hp << 8),
                (CharacterStat::Mana, base.mana << 8),
                (CharacterStat::MaxMana, base.mana << 8),
                (CharacterStat::Stamina, base.stamina << 8),
                (CharacterStat::MaxStamina, base.stamina << 8),
                (CharacterStat::Level, 1),
                (CharacterStat::Experience, 0),
                (CharacterStat::Gold, 0),
                (CharacterStat::StashedGold, 0),
            ],
            skills: [0; 30],
            quests: [[0u16; quest::SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3],
            waypoints: [[false; waypoint::WAYPOINT_COUNT]; 3],
        };

        let stats_section = encode_character_stats_section(&snapshot.stats)?;
        let skills_section = encode_character_skills_section(snapshot.skills);

        let mut raw = build_v105_fixed_pre_stats_block(&snapshot);
        raw.extend_from_slice(&stats_section);
        raw.extend_from_slice(&skills_section);
        append_v105_item_sections(&mut raw, &GameState::default(), expansion_mode);
        fix_header(&mut raw);

        Self::parse(raw).map_err(CharacterExportError::CharacterFile)
    }

    /// Returns a copy of this legacy save with live state overlaid onto the
    /// header, `gf` stat section, and 30-byte `if` skill section.
    ///
    /// The rest of the file is left byte-for-byte intact apart from shifting
    /// later sections if the encoded stat bitstream changes length and repairing
    /// the size/checksum header fields. This is the preferred path for exports
    /// meant to start from a real Classic/LoD character file, because D2GS does
    /// not expose every save-only section needed to synthesize a complete save
    /// from scratch.
    pub fn overlay_game_state(
        &self,
        state: &GameState,
        options: CharacterExportOptions,
    ) -> Result<Self, CharacterExportError> {
        let snapshot = LegacyExportSnapshot::from_game_state(state, options)?;
        let stats_section = encode_character_stats_section(&snapshot.stats)?;
        let skills_section = encode_character_skills_section(snapshot.skills);

        let mut raw = self.raw.clone();
        write_legacy_header_snapshot(&mut raw, &snapshot); // For now keep using legacy writer
        replace_legacy_export_sections(&mut raw, &stats_section, &skills_section)?;
        fix_header(&mut raw);

        Self::parse(raw).map_err(CharacterExportError::CharacterFile)
    }
}

#[derive(Debug)]
pub enum CharacterFileError {
    BadMagic {
        found: u32,
    },
    FileSizeMismatch {
        header_size: u32,
        actual_size: usize,
    },
    ChecksumMismatch {
        expected: u32,
        calculated: u32,
    },
    InvalidFileSize {
        len: usize,
    },
    Io(io::Error),
    TooSmall {
        len: usize,
        required: usize,
    },
}

impl fmt::Display for CharacterFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {}", error),
            Self::TooSmall { len, required } => {
                write!(
                    formatter,
                    "D2S file is {} bytes, need at least {}",
                    len, required
                )
            }
            Self::BadMagic { found } => write!(formatter, "bad D2S magic: 0x{:08x}", found),
            Self::FileSizeMismatch {
                header_size,
                actual_size,
            } => write!(
                formatter,
                "D2S header size {} does not match actual size {}",
                header_size, actual_size
            ),
            Self::ChecksumMismatch {
                expected,
                calculated,
            } => write!(
                formatter,
                "D2S checksum 0x{:08x} does not match calculated 0x{:08x}",
                expected, calculated
            ),
            Self::InvalidFileSize { len } => write!(formatter, "D2S file is too large: {}", len),
        }
    }
}

impl std::error::Error for CharacterFileError {}

impl From<io::Error> for CharacterFileError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
pub enum CharacterExportError {
    NoLocalPlayer,
    LocalPlayerMissing {
        id: u32,
    },
    InvalidCharacterName {
        name: String,
    },
    StatValueTooLarge {
        stat: CharacterStat,
        value: u32,
        max: u32,
    },
    UnsupportedTemplateLayout {
        layout: CharacterHeaderLayout,
    },
    UnsupportedExpansionMode {
        expansion_mode: ExpansionMode,
    },
    MissingSection {
        marker: SaveSectionMarker,
    },
    CharacterFile(CharacterFileError),
}

impl fmt::Display for CharacterExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoLocalPlayer => write!(formatter, "game state has no local player id"),
            Self::LocalPlayerMissing { id } => {
                write!(
                    formatter,
                    "local player 0x{:08x} is not present in game state",
                    id
                )
            }
            Self::InvalidCharacterName { name } => {
                write!(formatter, "invalid legacy D2S character name {:?}", name)
            }
            Self::StatValueTooLarge { stat, value, max } => write!(
                formatter,
                "character stat {:?} value {} exceeds legacy D2S maximum {}",
                stat, value, max
            ),
            Self::UnsupportedTemplateLayout { layout } => {
                write!(formatter, "unsupported D2S template layout {:?}", layout)
            }
            Self::UnsupportedExpansionMode { expansion_mode } => {
                write!(
                    formatter,
                    "unsupported D2S expansion mode {:?}",
                    expansion_mode
                )
            }
            Self::MissingSection { marker } => {
                write!(
                    formatter,
                    "D2S template is missing {}",
                    marker.description()
                )
            }
            Self::CharacterFile(error) => write!(formatter, "character file error: {}", error),
        }
    }
}

impl std::error::Error for CharacterExportError {}

impl From<CharacterFileError> for CharacterExportError {
    fn from(error: CharacterFileError) -> Self {
        Self::CharacterFile(error)
    }
}

pub fn calculate_checksum(bytes: &[u8]) -> u32 {
    let mut checksum = 0u32;

    for (index, byte) in bytes.iter().copied().enumerate() {
        let mut value = if (CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4).contains(&index) {
            0
        } else {
            byte as u32
        };

        if checksum & 0x8000_0000 != 0 {
            value = value.wrapping_add(1);
        }

        checksum = checksum.wrapping_mul(2).wrapping_add(value);
    }

    checksum
}

fn parse_header(raw: &[u8]) -> Result<CharacterHeader, CharacterFileError> {
    require_len(raw, LEGACY_LEVEL_OFFSET + 1)?;

    let magic = read_u32_le(raw, 0)?;
    if magic != D2S_MAGIC {
        return Err(CharacterFileError::BadMagic { found: magic });
    }

    let version_raw = read_u32_le(raw, VERSION_OFFSET)?;
    let file_size = read_u32_le(raw, FILE_SIZE_OFFSET)?;
    if file_size as usize != raw.len() {
        return Err(CharacterFileError::FileSizeMismatch {
            header_size: file_size,
            actual_size: raw.len(),
        });
    }

    let checksum = read_u32_le(raw, CHECKSUM_OFFSET)?;
    let calculated = calculate_checksum(raw);
    if checksum != calculated {
        return Err(CharacterFileError::ChecksumMismatch {
            expected: checksum,
            calculated,
        });
    }

    let save_version = SaveVersion::from_raw(version_raw);
    let layout = header_layout(save_version);
    require_len(raw, layout.level_offset() + 1)?;
    require_len(raw, layout.name_offset() + CHARACTER_NAME_LEN)?;

    let status = CharacterStatus::from_byte(raw[layout.status_offset()]);
    let progression = layout
        .progression_offset()
        .map(|offset| CharacterProgression::from_v105_byte(raw[offset]));
    let class_id = raw[layout.class_offset()];
    let expansion_mode = expansion_mode_from_raw(raw, layout, status);
    let edition = detect_character_edition(save_version, status, expansion_mode, class_id);
    let mercenary = if layout == CharacterHeaderLayout::ResurrectedV105
        && raw.len() >= D2R_V105_MERC_XP_OFFSET + 4
    {
        Some(MercenaryHeader {
            name_seed: read_u32_le(raw, D2R_V105_MERC_NAME_SEED_OFFSET)?,
            status: read_u16_le(raw, D2R_V105_MERC_STATUS_OFFSET)?,
            hireling_id: read_u16_le(raw, D2R_V105_MERC_ID_OFFSET)?,
            experience: read_u32_le(raw, D2R_V105_MERC_XP_OFFSET)?,
        })
    } else {
        None
    };

    let name =
        fixed_c_string(&raw[layout.name_offset()..layout.name_offset() + CHARACTER_NAME_LEN]);

    Ok(CharacterHeader {
        layout,
        version_raw,
        save_version,
        file_size,
        checksum,
        name,
        status,
        progression,
        class_id,
        class: CharacterClass::from_id(class_id),
        level: raw[layout.level_offset()],
        mercenary,
        edition,
        expansion_mode,
    })
}

fn expansion_mode_from_raw(
    raw: &[u8],
    layout: CharacterHeaderLayout,
    status: CharacterStatus,
) -> ExpansionMode {
    if layout == CharacterHeaderLayout::ResurrectedV105 {
        raw.get(D2R_V105_MODE_MARKER_OFFSET)
            .copied()
            .map(ExpansionMode::from_v105_marker)
            .unwrap_or(ExpansionMode::Unknown(0))
    } else {
        ExpansionMode::from_legacy_status(status)
    }
}

fn detect_character_edition(
    save_version: SaveVersion,
    status: CharacterStatus,
    expansion_mode: ExpansionMode,
    class_id: u8,
) -> GameEdition {
    if save_version.uses_v105_header() {
        if expansion_mode == ExpansionMode::RotW || class_id == CharacterClass::Warlock as u8 {
            GameEdition::ReignOfTheWarlock
        } else {
            GameEdition::Resurrected
        }
    } else if save_version.uses_resurrected_item_encoding() {
        if class_id == CharacterClass::Warlock as u8 {
            GameEdition::ReignOfTheWarlock
        } else {
            GameEdition::Resurrected
        }
    } else {
        detect_edition(save_version, status)
    }
}

fn parse_character_stats(raw: &[u8], layout: CharacterHeaderLayout) -> CharacterStats {
    let Some(marker_offset) =
        find_section_marker(raw, SaveSectionMarker::Stats, layout.section_search_start())
    else {
        return CharacterStats::missing();
    };

    let mut bit_offset = (marker_offset + 2) * 8;
    let total_bits = raw.len() * 8;
    let mut entries = Vec::new();
    let mut terminator_found = false;

    for _ in 0..64 {
        let Some(id) = read_bits(raw, bit_offset, 9) else {
            break;
        };
        bit_offset += 9;
        if id == 0x1ff {
            terminator_found = true;
            break;
        }

        let stat = CharacterStat::from_id(id as u16);
        let width = stat
            .map(|stat| stat.bit_width() as usize)
            .unwrap_or_else(|| character_stat_save_width(id as u16));
        if bit_offset + width > total_bits {
            break;
        }

        let Some(value) = read_bits(raw, bit_offset, width) else {
            break;
        };
        bit_offset += width;
        entries.push(CharacterStatEntry {
            id: id as u16,
            stat,
            value,
        });
    }

    CharacterStats {
        marker_offset: Some(marker_offset),
        entries,
        terminator_found,
    }
}

fn character_stat_save_width(id: u16) -> usize {
    // Character attributes use ItemStatCost.txt CSvBits. Most item-derived stats
    // have zero character-save width, but they can still appear as ids in the
    // gf stream before hard character stats.
    match id {
        0..=4 => 10,
        5 => 8,
        6..=11 => 21,
        12 => 7,
        13 => 32,
        14 | 15 => 25,
        _ => 0,
    }
}

fn parse_character_skills(
    raw: &[u8],
    layout: CharacterHeaderLayout,
) -> Result<Option<CharacterSkills>, CharacterFileError> {
    let Some(marker_offset) = find_section_marker(
        raw,
        SaveSectionMarker::Skills,
        layout.section_search_start(),
    ) else {
        return Ok(None);
    };
    require_len(raw, marker_offset + 2 + 30)?;

    let mut levels = [0; 30];
    levels.copy_from_slice(&raw[marker_offset + 2..marker_offset + 2 + 30]);
    Ok(Some(CharacterSkills {
        marker_offset,
        levels,
    }))
}

fn parse_item_lists(raw: &[u8], layout: CharacterHeaderLayout) -> CharacterItemLists {
    let start = layout.section_search_start();
    let item_lists = find_item_list_headers(raw, start);
    let corpse_marker_offset = find_section_marker(raw, SaveSectionMarker::Corpse, start);
    let iron_golem =
        find_section_marker(raw, SaveSectionMarker::IronGolem, start).map(|marker_offset| {
            IronGolemHeader {
                marker_offset,
                active: raw.get(marker_offset + 2).copied() == Some(1),
            }
        });
    let follower_block =
        find_section_marker(raw, SaveSectionMarker::Followers, start).and_then(|marker_offset| {
            let follower_count = read_u16_at(raw, marker_offset + 2)?;
            let payload_start = marker_offset + 4;
            let payload_len = raw.len().saturating_sub(payload_start);
            Some(FollowerBlockHeader {
                marker_offset,
                follower_count,
                payload_len,
                complete_116_byte_payloads: payload_len == follower_count as usize * 116,
            })
        });

    CharacterItemLists {
        item_lists,
        corpse_marker_offset,
        iron_golem,
        follower_block,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyExportSnapshot {
    name: String,
    status: CharacterStatus,
    edition: GameEdition,
    expansion_mode: ExpansionMode,
    class: CharacterClass,
    level: u8,
    map_id: u32,
    stats: Vec<(CharacterStat, u32)>,
    skills: [u8; 30],
    quests: [[u16; quest::SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3],
    waypoints: [[bool; waypoint::WAYPOINT_COUNT]; 3],
}

impl LegacyExportSnapshot {
    fn from_game_state(
        state: &GameState,
        options: CharacterExportOptions,
    ) -> Result<Self, CharacterExportError> {
        let local_id = state
            .local_player_id()
            .ok_or(CharacterExportError::NoLocalPlayer)?;
        let player = state
            .player(local_id)
            .ok_or(CharacterExportError::LocalPlayerMissing { id: local_id })?;
        let name = player.name().to_owned();
        validate_legacy_character_name(&name)?;

        let class = player.class();
        let skills = options
            .skills
            .unwrap_or_else(|| player.legacy_save_skills());
        let level_value = player
            .stat(UnitStat::Level as u16)
            .unwrap_or_else(|| player.level())
            .max(1);
        let level =
            u8::try_from(level_value).map_err(|_| CharacterExportError::StatValueTooLarge {
                stat: CharacterStat::Level,
                value: level_value,
                max: u8::MAX as u32,
            })?;

        let mut stats = legacy_character_stats_from_player(player);
        upsert_stat(&mut stats, CharacterStat::Level, level_value);
        stats.sort_by_key(|(stat, _)| *stat as u16);
        validate_character_stats(&stats)?;

        let mut quests = [[0u16; quest::SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3];
        if let Some(quest_log) = state.player_quest_log() {
            quests[state.difficulty().index()] = quest::quest_words_from_player_log(quest_log);
        }
        let waypoints = *state.player_waypoints().as_difficulties();

        let status = CharacterStatus {
            hardcore: state.is_hardcore(),
            died: false,
            expansion: state.is_expansion()
                || matches!(class, CharacterClass::Druid | CharacterClass::Assassin),
            ladder: state.is_ladder(),
        };

        let edition = if class == CharacterClass::Warlock {
            GameEdition::ReignOfTheWarlock
        } else if status.expansion {
            GameEdition::LordOfDestruction
        } else {
            GameEdition::Classic
        };
        let expansion_mode = match edition {
            GameEdition::Classic => ExpansionMode::Classic,
            GameEdition::ReignOfTheWarlock => ExpansionMode::RotW,
            _ => ExpansionMode::Expansion,
        };

        Ok(Self {
            name,
            status,
            edition,
            expansion_mode,
            class,
            level,
            map_id: state.map().map_id.unwrap_or_default(),
            stats,
            skills,
            quests,
            waypoints,
        })
    }
}

fn legacy_character_stats_from_player(player: &Player) -> Vec<(CharacterStat, u32)> {
    let mut stats = Vec::new();

    for id in 0..=15 {
        let Some(stat) = CharacterStat::from_id(id) else {
            continue;
        };
        if let Some(value) = player.stat(id) {
            stats.push((stat, legacy_save_stat_value(stat, value)));
        }
    }

    if let Some(vitals) = player.vitals() {
        if let Some(life) = vitals.life() {
            insert_stat_if_absent(
                &mut stats,
                CharacterStat::HitPoints,
                legacy_save_stat_value(CharacterStat::HitPoints, life as u32),
            );
        }
        if let Some(mana) = vitals.mana() {
            insert_stat_if_absent(
                &mut stats,
                CharacterStat::Mana,
                legacy_save_stat_value(CharacterStat::Mana, mana as u32),
            );
        }
        if let Some(stamina) = vitals.stamina() {
            insert_stat_if_absent(
                &mut stats,
                CharacterStat::Stamina,
                legacy_save_stat_value(CharacterStat::Stamina, stamina as u32),
            );
        }
    }

    if let Some(max_life) = player.stat(UnitStat::LifeMax as u16) {
        upsert_stat(
            &mut stats,
            CharacterStat::HitPoints,
            legacy_save_stat_value(CharacterStat::HitPoints, max_life),
        );
    }
    if let Some(max_mana) = player.stat(UnitStat::ManaMax as u16) {
        upsert_stat(
            &mut stats,
            CharacterStat::Mana,
            legacy_save_stat_value(CharacterStat::Mana, max_mana),
        );
    }

    stats
}

fn legacy_save_stat_value(stat: CharacterStat, value: u32) -> u32 {
    if is_legacy_resource_stat(stat) {
        value.saturating_mul(1 << 8)
    } else {
        value
    }
}

fn is_legacy_resource_stat(stat: CharacterStat) -> bool {
    matches!(
        stat,
        CharacterStat::HitPoints
            | CharacterStat::MaxHitPoints
            | CharacterStat::Mana
            | CharacterStat::MaxMana
            | CharacterStat::Stamina
            | CharacterStat::MaxStamina
    )
}

fn validate_legacy_character_name(name: &str) -> Result<(), CharacterExportError> {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > CHARACTER_NAME_LEN - 1 || bytes.contains(&0) {
        return Err(CharacterExportError::InvalidCharacterName {
            name: name.to_owned(),
        });
    }

    Ok(())
}

fn validate_character_stats(stats: &[(CharacterStat, u32)]) -> Result<(), CharacterExportError> {
    for (stat, value) in stats {
        let width = stat.bit_width();
        let max = if width == 32 {
            u32::MAX
        } else {
            (1u32 << width) - 1
        };
        if *value > max {
            return Err(CharacterExportError::StatValueTooLarge {
                stat: *stat,
                value: *value,
                max,
            });
        }
    }

    Ok(())
}

fn upsert_stat(stats: &mut Vec<(CharacterStat, u32)>, stat: CharacterStat, value: u32) {
    if let Some((_, existing)) = stats.iter_mut().find(|(candidate, _)| *candidate == stat) {
        *existing = value;
    } else {
        stats.push((stat, value));
    }
}

fn insert_stat_if_absent(stats: &mut Vec<(CharacterStat, u32)>, stat: CharacterStat, value: u32) {
    if !stats.iter().any(|(candidate, _)| *candidate == stat) {
        stats.push((stat, value));
    }
}

fn encode_character_stats_section(
    stats: &[(CharacterStat, u32)],
) -> Result<Vec<u8>, CharacterExportError> {
    validate_character_stats(stats)?;

    let mut bits = Vec::new();
    bits.extend_from_slice(SaveSectionMarker::Stats.raw_bytes());
    let mut writer = SaveBitWriter::default();
    for (stat, value) in stats {
        writer.write_bits(*stat as u32, 9);
        writer.write_bits(*value, stat.bit_width() as usize);
    }
    writer.write_bits(0x1ff, 9);
    bits.extend_from_slice(&writer.finish());
    Ok(bits)
}

fn encode_character_skills_section(skills: [u8; 30]) -> Vec<u8> {
    let mut section = Vec::with_capacity(32);
    section.extend_from_slice(SaveSectionMarker::Skills.raw_bytes());
    section.extend_from_slice(&skills);
    section
}

fn build_legacy_fixed_pre_stats_block(snapshot: &LegacyExportSnapshot) -> Vec<u8> {
    let mut raw = vec![0; LEGACY_FULL_EXPORT_PRE_STATS_LEN];
    raw[0..4].copy_from_slice(&D2S_MAGIC.to_le_bytes());
    write_u32_le(&mut raw, VERSION_OFFSET, SaveVersion::Lod110Plus.raw());
    write_legacy_header_snapshot(&mut raw, snapshot);

    raw[0x29] = 0x10;
    raw[0x2a] = 0x1e;
    raw[0x34..0x38].fill(0xff);
    write_u32_le(&mut raw, LEGACY_MAP_ID_OFFSET, snapshot.map_id);

    write_empty_legacy_quests(&mut raw);
    write_empty_legacy_waypoints(&mut raw);
    write_empty_legacy_npc_dialogs(&mut raw);
    quest::write_legacy_quest_words(&mut raw, 0, &snapshot.quests);
    quest::apply_progression_from_quests(&mut raw, 0, &snapshot.quests);
    waypoint::write_legacy_waypoints(&mut raw, 0, &snapshot.waypoints);

    debug_assert_eq!(LEGACY_ASSIGNED_SKILLS_OFFSET + 64, LEGACY_LEFT_SKILL_OFFSET);
    debug_assert_eq!(LEGACY_LEFT_SKILL_OFFSET + 4, LEGACY_RIGHT_SKILL_OFFSET);
    debug_assert_eq!(LEGACY_RIGHT_SKILL_OFFSET + 4, LEGACY_LEFT_SWAP_SKILL_OFFSET);
    debug_assert_eq!(
        LEGACY_LEFT_SWAP_SKILL_OFFSET + 4,
        LEGACY_RIGHT_SWAP_SKILL_OFFSET
    );
    debug_assert_eq!(LEGACY_RIGHT_SWAP_SKILL_OFFSET + 4, LEGACY_APPEARANCE_OFFSET);
    debug_assert_eq!(LEGACY_DIFFICULTY_OFFSET + 3, LEGACY_MAP_ID_OFFSET);
    debug_assert_eq!(LEGACY_MERCENARY_OFFSET + 14, LEGACY_REALM_DATA_OFFSET);
    debug_assert_eq!(raw.len(), LEGACY_FULL_EXPORT_PRE_STATS_LEN);
    raw
}

fn build_v105_fixed_pre_stats_block(snapshot: &LegacyExportSnapshot) -> Vec<u8> {
    let mut raw = vec![0; V105_FULL_EXPORT_PRE_STATS_LEN];
    raw[0..4].copy_from_slice(&D2S_MAGIC.to_le_bytes());
    write_u32_le(&mut raw, VERSION_OFFSET, 105);

    write_fixed_character_name(&mut raw, D2R_V105_NAME_OFFSET, &snapshot.name);
    raw[D2R_V105_STATUS_OFFSET] = snapshot.status.to_byte();
    raw[D2R_V105_PROGRESSION_OFFSET] = quest::progression_from_quests(&snapshot.quests);
    raw[D2R_V105_CLASS_OFFSET] = snapshot.class as u8;
    raw[D2R_V105_RESERVED_VERSION_MARKERS_OFFSET..D2R_V105_RESERVED_VERSION_MARKERS_OFFSET + 2]
        .copy_from_slice(&[0x10, 0x1e]);
    raw[D2R_V105_LEVEL_OFFSET] = snapshot.level;
    raw[D2R_V105_RESERVED_CHECKSUM_MASK_OFFSET..D2R_V105_RESERVED_CHECKSUM_MASK_OFFSET + 4]
        .fill(0xff);
    for offset in (D2R_V105_ASSIGNED_SKILLS_OFFSET..D2R_V105_LEFT_SKILL_OFFSET).step_by(4) {
        raw[offset..offset + 4].copy_from_slice(&0x0000_ffffu32.to_le_bytes());
    }
    raw[D2R_V105_LEFT_SKILL_OFFSET..D2R_V105_LEFT_SKILL_OFFSET + 4]
        .copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    raw[D2R_V105_RIGHT_SKILL_OFFSET..D2R_V105_RIGHT_SKILL_OFFSET + 4]
        .copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    raw[D2R_V105_LEFT_SWAP_SKILL_OFFSET..D2R_V105_LEFT_SWAP_SKILL_OFFSET + 4]
        .copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    raw[D2R_V105_RIGHT_SWAP_SKILL_OFFSET..D2R_V105_RIGHT_SWAP_SKILL_OFFSET + 4]
        .copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    raw[D2R_V105_APPEARANCE_OFFSET..D2R_V105_APPEARANCE_OFFSET + 32].fill(0xff);
    raw[D2R_V105_DIFFICULTY_OFFSET] = 0x80;
    write_u32_le(&mut raw, D2R_V105_MAP_ID_OFFSET, snapshot.map_id);
    raw[D2R_V105_MODE_MARKER_OFFSET] = snapshot
        .expansion_mode
        .to_v105_marker()
        .unwrap_or(ExpansionMode::V105_EXPANSION_MARKER);

    // Quests
    let q_start = V105_QUEST_HEADER_OFFSET;
    raw[q_start..q_start + 4].copy_from_slice(&quest::SAVE_QUEST_SECTION_MARKER);
    raw[q_start + 4..q_start + 8].copy_from_slice(&0x06u32.to_le_bytes());
    raw[q_start + 8..q_start + 12].copy_from_slice(&0x2au32.to_le_bytes());

    quest::write_legacy_quest_words(&mut raw, D2R_V105_HEADER_LEN, &snapshot.quests);
    raw[D2R_V105_PROGRESSION_OFFSET] = quest::progression_from_quests(&snapshot.quests);

    // Waypoints
    let wp_start = V105_WAYPOINT_HEADER_OFFSET;
    raw[wp_start..wp_start + 2].copy_from_slice(&waypoint::LEGACY_WAYPOINT_SECTION_MARKER);
    raw[wp_start + 2..wp_start + 6].copy_from_slice(&0x01u32.to_le_bytes());
    raw[wp_start + 6..wp_start + 10].copy_from_slice(&0x50u32.to_le_bytes());

    for difficulty in 0..3 {
        let offset = wp_start + 10 + difficulty * LEGACY_WAYPOINT_DIFFICULTY_LEN;
        raw[offset] = 0x02;
        raw[offset + 1] = 0x01;
    }
    raw[wp_start + 0x52] = 0x01; // Trailer

    waypoint::write_v105_waypoints(&mut raw, D2R_V105_HEADER_LEN, &snapshot.waypoints);

    // NPC
    let npc_start = npc_offset(snapshot.edition);
    raw[npc_start..npc_start + 4].copy_from_slice(&[0x01, 0x77, 0x34, 0x00]);

    debug_assert_eq!(raw.len(), V105_FULL_EXPORT_PRE_STATS_LEN);
    raw
}

fn npc_offset(edition: GameEdition) -> usize {
    match edition {
        GameEdition::ReignOfTheWarlock => V105_NPC_HEADER_OFFSET,
        _ => V105_NPC_HEADER_OFFSET, // For now same for all V105
    }
}

fn write_empty_legacy_quests(raw: &mut [u8]) {
    write_u32_le(raw, LEGACY_QUEST_UNKNOWN_OFFSET, 1);
    raw[LEGACY_QUEST_HEADER_OFFSET..LEGACY_QUEST_HEADER_OFFSET + 4]
        .copy_from_slice(&quest::SAVE_QUEST_SECTION_MARKER);
    raw[LEGACY_QUEST_MAGIC_OFFSET..LEGACY_QUEST_MAGIC_OFFSET + 6]
        .copy_from_slice(&quest::SAVE_QUEST_SECTION_HEADER_AFTER_MARKER);
}

fn write_empty_legacy_waypoints(raw: &mut [u8]) {
    raw[LEGACY_WAYPOINT_HEADER_OFFSET..LEGACY_WAYPOINT_HEADER_OFFSET + 2]
        .copy_from_slice(&waypoint::LEGACY_WAYPOINT_SECTION_MARKER);
    raw[LEGACY_WAYPOINT_MAGIC_OFFSET..LEGACY_WAYPOINT_MAGIC_OFFSET + 6]
        .copy_from_slice(&waypoint::LEGACY_WAYPOINT_SECTION_HEADER_AFTER_MARKER);
    for difficulty in 0..3 {
        let offset =
            LEGACY_WAYPOINT_DIFFICULTIES_OFFSET + difficulty * LEGACY_WAYPOINT_DIFFICULTY_LEN;
        raw[offset] = 0x02;
        raw[offset + 1] = 0x01;
    }

    raw[LEGACY_WAYPOINT_TRAILER_OFFSET] = 0x01;
}

fn write_empty_legacy_npc_dialogs(raw: &mut [u8]) {
    raw[LEGACY_NPC_HEADER_OFFSET..LEGACY_NPC_HEADER_OFFSET + 2].copy_from_slice(b"w4");
}

fn append_legacy_item_sections(raw: &mut Vec<u8>, state: &GameState, is_expansion: bool) {
    append_player_inventory_item_list(raw, state);
    append_empty_item_list(raw);

    if is_expansion {
        raw.extend_from_slice(SaveSectionMarker::Corpse.raw_bytes());
        raw.extend_from_slice(SaveSectionMarker::IronGolem.raw_bytes());
        raw.push(0);
    }
}

fn append_v105_item_sections(raw: &mut Vec<u8>, state: &GameState, expansion_mode: ExpansionMode) {
    append_player_inventory_item_list(raw, state);
    append_empty_item_list(raw);

    if expansion_mode != ExpansionMode::Classic {
        raw.extend_from_slice(SaveSectionMarker::Corpse.raw_bytes());
        raw.extend_from_slice(SaveSectionMarker::IronGolem.raw_bytes());
        raw.push(0);
    }

    if expansion_mode == ExpansionMode::RotW {
        raw.extend_from_slice(&1u16.to_le_bytes());
        raw.extend_from_slice(SaveSectionMarker::Followers.raw_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
    }
}

fn append_empty_item_list(raw: &mut Vec<u8>) {
    raw.extend_from_slice(SaveSectionMarker::ItemList.raw_bytes());
    raw.extend_from_slice(&0u16.to_le_bytes());
}

fn append_player_inventory_item_list(raw: &mut Vec<u8>, state: &GameState) {
    let Some(local_player_id) = state.local_player_id() else {
        append_empty_item_list(raw);
        return;
    };

    let socketed_children = local_socketed_children(state);
    let mut top_level_items: Vec<_> = state
        .items()
        .iter()
        .filter_map(|(item_id, item)| {
            is_local_export_item(item, local_player_id).then_some(*item_id)
        })
        .collect();
    top_level_items.sort_by_key(|item_id| inventory_sort_key(state.item(*item_id)));

    let mut encoded_items = Vec::new();
    for item_id in top_level_items {
        let mut visited = HashSet::new();
        if let Some(bytes) =
            encode_legacy_item_tree(state, item_id, &socketed_children, &mut visited)
        {
            encoded_items.push(bytes);
        }
    }

    raw.extend_from_slice(SaveSectionMarker::ItemList.raw_bytes());
    raw.extend_from_slice(&(encoded_items.len() as u16).to_le_bytes());
    for item_bytes in encoded_items {
        raw.extend_from_slice(&item_bytes);
    }
}

fn local_socketed_children(state: &GameState) -> HashMap<u32, Vec<u32>> {
    let mut children = HashMap::<u32, Vec<u32>>::new();
    for (item_id, item) in state.items() {
        if let ItemOwner::Unit {
            unit_type: 0x04,
            unit_id,
        } = item.owner()
        {
            children.entry(unit_id).or_default().push(*item_id);
        }
    }
    for child_ids in children.values_mut() {
        child_ids.sort_unstable();
    }
    children
}

fn is_local_export_item(item: &Item, local_player_id: u32) -> bool {
    matches!(
        item.owner(),
        ItemOwner::Unit {
            unit_type: 0x00,
            unit_id,
        } if unit_id == local_player_id
    ) && matches!(
        save_item_location(item),
        Some(SaveItemLocation::Inventory { .. } | SaveItemLocation::Equipped { .. })
    )
}

fn inventory_sort_key(item: Option<&Item>) -> (u8, u8, u8, u32) {
    let Some(item) = item else {
        return (u8::MAX, u8::MAX, u8::MAX, u32::MAX);
    };
    let Some(location) = save_item_location(item) else {
        return (u8::MAX, u8::MAX, u8::MAX, item.id());
    };
    match location {
        SaveItemLocation::Equipped { slot } => (0, slot, 0, item.id()),
        SaveItemLocation::Inventory { x, y } => (1, y, x, item.id()),
        SaveItemLocation::Socketed => (2, 0, 0, item.id()),
    }
}

fn encode_legacy_item_tree(
    state: &GameState,
    item_id: u32,
    socketed_children: &HashMap<u32, Vec<u32>>,
    visited: &mut HashSet<u32>,
) -> Option<Vec<u8>> {
    if !visited.insert(item_id) {
        return None;
    }

    let item = state.item(item_id)?;
    let mut encoded_children = Vec::new();
    if let Some(child_ids) = socketed_children.get(&item_id) {
        for child_id in child_ids {
            if let Some(bytes) =
                encode_legacy_item_tree(state, *child_id, socketed_children, visited)
            {
                encoded_children.push(bytes);
            }
        }
    }

    let mut bytes = encode_legacy_item(item, encoded_children.len() as u8)?;
    for child_bytes in encoded_children {
        bytes.extend_from_slice(&child_bytes);
    }
    Some(bytes)
}

fn encode_legacy_item(item: &Item, socketed_children: u8) -> Option<Vec<u8>> {
    let packet = item.packet_data()?;
    if packet.flags.is_ear() || packet.flags.is_gamble() {
        return None;
    }

    let code = packet.code.as_ref()?;
    let location = save_item_location(item)?;
    let mut writer = SaveBitWriter::default();
    writer.write_bits(0x4d4a, 16);
    writer.write_bits(packet.flags.bits(), 32);
    writer.write_bits(packet.version as u32, 10);

    match location {
        SaveItemLocation::Inventory { x, y } => {
            writer.write_bits(0, 3);
            writer.write_bits(0, 4);
            writer.write_bits(x as u32, 4);
            writer.write_bits(y as u32, 4);
            writer.write_bits(0, 3);
        }
        SaveItemLocation::Equipped { slot } => {
            writer.write_bits(1, 3);
            writer.write_bits(slot as u32, 4);
            writer.write_bits(0, 4);
            writer.write_bits(0, 4);
            writer.write_bits(0, 3);
        }
        SaveItemLocation::Socketed => {
            writer.write_bits(0x6, 3);
            writer.write_bits(0, 4);
            writer.write_bits(0, 4);
            writer.write_bits(0, 4);
            writer.write_bits(0, 3);
        }
    }

    for byte in code.raw() {
        writer.write_bits(byte as u32, 8);
    }

    let socket_count_bits = if packet.flags.is_simple_item() { 1 } else { 3 };
    writer.write_bits(socketed_children as u32, socket_count_bits);
    if packet.flags.is_simple_item() {
        writer.align_to_byte();
        return Some(writer.finish());
    }

    writer.write_bits(item.id(), 32);
    writer.write_bits(packet.level? as u32, 7);
    let quality = packet.quality?;
    writer.write_bits(save_item_quality(quality) as u32, 4);

    writer.write_bool(packet.graphic.is_some());
    if let Some(graphic) = packet.graphic {
        writer.write_bits(graphic as u32, 3);
    }

    writer.write_bool(false);

    match quality {
        ItemQuality::Inferior | ItemQuality::Superior => {
            writer.write_bits(packet.quality_modifier.unwrap_or_default() as u32, 3);
        }
        ItemQuality::Magic => {
            writer.write_bits(packet.magic_prefix.unwrap_or_default() as u32, 11);
            writer.write_bits(packet.magic_suffix.unwrap_or_default() as u32, 11);
        }
        ItemQuality::Rare | ItemQuality::Crafted => {
            let rare_name = packet.rare_name.unwrap_or([0, 0]);
            writer.write_bits(rare_name[0] as u32, 8);
            writer.write_bits(rare_name[1] as u32, 8);
            for affix in packet.rare_affixes {
                writer.write_bool(affix.prefix.is_some());
                if let Some(prefix) = affix.prefix {
                    writer.write_bits(prefix as u32, 11);
                }
                writer.write_bool(affix.suffix.is_some());
                if let Some(suffix) = affix.suffix {
                    writer.write_bits(suffix as u32, 11);
                }
            }
        }
        ItemQuality::Set | ItemQuality::Unique => {
            writer.write_bits(packet.code_extra.unwrap_or_default() as u32, 12);
        }
        ItemQuality::NotApplicable | ItemQuality::Normal | ItemQuality::Unknown(_) => {}
    }

    if let Some(runeword) = packet.runeword {
        writer.write_bits(runeword.id as u32, 12);
        writer.write_bits(5, 4);
    }

    if let Some(name) = packet.personalized_name.as_deref() {
        write_legacy_item_name(&mut writer, name);
    }

    if matches!(code.as_str(), "tbk" | "ibk") {
        writer.write_bits(packet.magic_suffix.unwrap_or_default() as u32, 5);
    }

    writer.write_bool(false);

    if item.category_kind() == crate::core::object::item::ItemCategory::Armor {
        writer.write_bits(
            packet.defense.unwrap_or_default().saturating_add(10) as u32,
            11,
        );
    }

    if matches!(
        item.category_kind(),
        crate::core::object::item::ItemCategory::Armor
            | crate::core::object::item::ItemCategory::Weapon
            | crate::core::object::item::ItemCategory::Weapon2
            | crate::core::object::item::ItemCategory::Shield
    ) && let Some(durability) = packet.durability
    {
        writer.write_bits(durability.max as u32, 8);
        writer.write_bits(durability.current as u32, 8);
        writer.write_bool(false);
    }

    if packet.flags.is_socketed() {
        writer.write_bits(packet.sockets.unwrap_or_default() as u32, 4);
    }

    let tail_offset = packet_stat_list_offset(item)?;
    writer.write_raw_tail(item.raw_bitstream(), tail_offset);
    writer.align_to_byte();
    Some(writer.finish())
}

fn save_item_quality(quality: ItemQuality) -> u8 {
    match quality {
        ItemQuality::NotApplicable => 0,
        ItemQuality::Inferior => 1,
        ItemQuality::Normal => 2,
        ItemQuality::Superior => 3,
        ItemQuality::Magic => 4,
        ItemQuality::Set => 5,
        ItemQuality::Rare => 6,
        ItemQuality::Unique => 7,
        ItemQuality::Crafted => 8,
        ItemQuality::Unknown(value) => value,
    }
}

fn write_legacy_item_name(writer: &mut SaveBitWriter, name: &str) {
    for byte in name.bytes() {
        writer.write_bits(byte as u32, 7);
    }
    writer.write_bits(0, 7);
}

fn packet_stat_list_offset(item: &Item) -> Option<usize> {
    let packet = item.packet_data()?;
    let raw = item.raw_bitstream();
    let mut bit_offset = 32 + 8 + 2;
    let destination = ItemDestination::from_packet_value(read_bits(raw, bit_offset, 3)? as u8);
    bit_offset += 3;

    bit_offset += match destination {
        ItemDestination::Ground => 16 + 16,
        _ => 4 + 4 + 3 + 4,
    };

    if packet.flags.is_ear() {
        return None;
    }

    bit_offset += 8 * 4;
    if packet.code.as_ref()?.as_str() == "gld" {
        return None;
    }

    bit_offset += 3;
    if packet.flags.is_simple_item() || packet.flags.is_gamble() {
        return Some(bit_offset);
    }

    bit_offset += 7 + 4;
    let has_graphic = read_bits(raw, bit_offset, 1)? != 0;
    bit_offset += 1;
    if has_graphic {
        bit_offset += 3;
    }

    let has_color = read_bits(raw, bit_offset, 1)? != 0;
    bit_offset += 1;
    if has_color {
        bit_offset += 11;
    }

    if packet.flags.is_identified() {
        match packet.quality? {
            ItemQuality::Unique => {
                if packet.code.as_ref()?.as_str() != "std" {
                    bit_offset += 12;
                }
            }
            ItemQuality::Inferior | ItemQuality::Superior => {
                bit_offset += 3;
            }
            ItemQuality::Magic => {
                bit_offset += 11 + 11;
            }
            ItemQuality::Rare | ItemQuality::Crafted => {
                bit_offset += 8 + 8;
                for affix in packet.rare_affixes {
                    bit_offset += 1;
                    if affix.prefix.is_some() {
                        bit_offset += 11;
                    }
                    bit_offset += 1;
                    if affix.suffix.is_some() {
                        bit_offset += 11;
                    }
                }
            }
            ItemQuality::Set => {
                bit_offset += 12;
            }
            ItemQuality::NotApplicable | ItemQuality::Normal | ItemQuality::Unknown(_) => {}
        }

        if packet.runeword.is_some() {
            bit_offset += 12 + 4;
        }
        if let Some(name) = packet.personalized_name.as_deref() {
            bit_offset += (name.len() + 1) * 8;
        }
        if item.category_kind() == crate::core::object::item::ItemCategory::Armor {
            bit_offset += 11;
        }
        if matches!(
            item.category_kind(),
            crate::core::object::item::ItemCategory::Armor
                | crate::core::object::item::ItemCategory::Weapon
                | crate::core::object::item::ItemCategory::Weapon2
                | crate::core::object::item::ItemCategory::Shield
        ) && packet.durability.is_some()
        {
            bit_offset += 8 + 8 + 1;
        }
        if packet.flags.is_socketed() {
            bit_offset += 4;
        }
    }

    Some(bit_offset)
}

fn save_item_location(item: &Item) -> Option<SaveItemLocation> {
    let packet = item.packet_data()?;
    match item.owner() {
        ItemOwner::Unit {
            unit_type: 0x04, ..
        } => Some(SaveItemLocation::Socketed),
        _ => match packet.placement {
            crate::core::object::item::ItemPlacement::Ground { .. } => None,
            crate::core::object::item::ItemPlacement::Container {
                x, y, container, ..
            } if ItemContainer::from_packet_value(container) == ItemContainer::Inventory => {
                Some(SaveItemLocation::Inventory { x, y })
            }
            crate::core::object::item::ItemPlacement::Container {
                equipment_location, ..
            } if packet.destination == ItemDestination::Equipment
                || packet.placement.container_kind() == Some(ItemContainer::Equipment) =>
            {
                Some(SaveItemLocation::Equipped {
                    slot: equipment_location,
                })
            }
            _ => None,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SaveItemLocation {
    Inventory { x: u8, y: u8 },
    Equipped { slot: u8 },
    Socketed,
}

fn replace_legacy_export_sections(
    raw: &mut Vec<u8>,
    stats_section: &[u8],
    skills_section: &[u8],
) -> Result<(), CharacterExportError> {
    let stats_start = find_section_marker(raw, SaveSectionMarker::Stats, 0).ok_or(
        CharacterExportError::MissingSection {
            marker: SaveSectionMarker::Stats,
        },
    )?;
    let skills_start = find_section_marker(raw, SaveSectionMarker::Skills, stats_start + 2).ok_or(
        CharacterExportError::MissingSection {
            marker: SaveSectionMarker::Skills,
        },
    )?;
    if raw.len() < skills_start + skills_section.len() {
        return Err(CharacterExportError::MissingSection {
            marker: SaveSectionMarker::Skills,
        });
    }

    raw.splice(stats_start..skills_start, stats_section.iter().copied());
    let new_skills_start = stats_start + stats_section.len();
    raw.splice(
        new_skills_start..new_skills_start + skills_section.len(),
        skills_section.iter().copied(),
    );
    Ok(())
}

fn write_legacy_header_snapshot(raw: &mut [u8], snapshot: &LegacyExportSnapshot) {
    write_fixed_character_name(raw, LEGACY_NAME_OFFSET, &snapshot.name);
    raw[LEGACY_STATUS_OFFSET] = snapshot.status.to_byte();
    raw[LEGACY_CLASS_OFFSET] = snapshot.class as u8;
    raw[LEGACY_LEVEL_OFFSET] = snapshot.level;
}

fn write_fixed_character_name(raw: &mut [u8], offset: usize, name: &str) {
    let bytes = name.as_bytes();
    raw[offset..offset + CHARACTER_NAME_LEN].fill(0);
    let len = bytes.len().min(CHARACTER_NAME_LEN - 1);
    raw[offset..offset + len].copy_from_slice(&bytes[..len]);
}

fn find_item_list_headers(raw: &[u8], start: usize) -> Vec<ItemListHeader> {
    let mut headers = Vec::new();
    let mut offset = start;

    while let Some(marker_offset) = find_section_marker(raw, SaveSectionMarker::ItemList, offset) {
        let Some(parent_item_count) = read_u16_at(raw, marker_offset + 2) else {
            break;
        };
        headers.push(ItemListHeader {
            marker_offset,
            parent_item_count,
        });
        offset = marker_offset + 4;
    }

    headers
}

fn find_section_marker(raw: &[u8], marker: SaveSectionMarker, start: usize) -> Option<usize> {
    find_marker(raw, marker.raw_bytes(), start)
}

fn find_marker(raw: &[u8], marker: &[u8], start: usize) -> Option<usize> {
    raw.get(start..)?
        .windows(marker.len())
        .position(|window| window == marker)
        .map(|position| start + position)
}

fn fix_header(raw: &mut [u8]) {
    let len = raw.len() as u32;
    write_u32_le(raw, FILE_SIZE_OFFSET, len);
    write_u32_le(raw, CHECKSUM_OFFSET, 0);
    let checksum = calculate_checksum(raw);
    write_u32_le(raw, CHECKSUM_OFFSET, checksum);
}

fn require_len(raw: &[u8], required: usize) -> Result<(), CharacterFileError> {
    if raw.len() < required {
        return Err(CharacterFileError::TooSmall {
            len: raw.len(),
            required,
        });
    }

    Ok(())
}

fn read_u32_le(raw: &[u8], offset: usize) -> Result<u32, CharacterFileError> {
    require_len(raw, offset + 4)?;
    let mut bytes = [0; 4];
    bytes.copy_from_slice(&raw[offset..offset + 4]);
    Ok(u32::from_le_bytes(bytes))
}

fn read_u16_le(raw: &[u8], offset: usize) -> Result<u16, CharacterFileError> {
    require_len(raw, offset + 2)?;
    let mut bytes = [0; 2];
    bytes.copy_from_slice(&raw[offset..offset + 2]);
    Ok(u16::from_le_bytes(bytes))
}

fn read_u16_at(raw: &[u8], offset: usize) -> Option<u16> {
    let bytes = raw.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn write_u32_le(raw: &mut [u8], offset: usize, value: u32) {
    raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn read_bits(raw: &[u8], bit_offset: usize, count: usize) -> Option<u32> {
    if count > 32 || bit_offset.checked_add(count)? > raw.len() * 8 {
        return None;
    }

    let mut value = 0;
    for index in 0..count {
        let position = bit_offset + index;
        let bit = (raw[position / 8] >> (position % 8)) & 1;
        value |= (bit as u32) << index;
    }

    Some(value)
}

#[derive(Default)]
struct SaveBitWriter {
    bytes: Vec<u8>,
    bit_offset: usize,
}

impl SaveBitWriter {
    fn write_bits(&mut self, value: u32, count: usize) {
        for index in 0..count {
            if self.bit_offset / 8 == self.bytes.len() {
                self.bytes.push(0);
            }
            let bit = ((value >> index) & 1) as u8;
            self.bytes[self.bit_offset / 8] |= bit << (self.bit_offset % 8);
            self.bit_offset += 1;
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn write_bool(&mut self, value: bool) {
        self.write_bits(value as u32, 1);
    }

    fn write_raw_tail(&mut self, raw: &[u8], bit_offset: usize) {
        for position in bit_offset..raw.len() * 8 {
            let bit = (raw[position / 8] >> (position % 8)) & 1;
            self.write_bits(bit as u32, 1);
        }
    }

    fn align_to_byte(&mut self) {
        let remainder = self.bit_offset % 8;
        if remainder != 0 {
            self.write_bits(0, 8 - remainder);
        }
    }
}

fn header_layout(version: SaveVersion) -> CharacterHeaderLayout {
    if version.uses_v105_header() {
        CharacterHeaderLayout::ResurrectedV105
    } else if version.uses_resurrected_item_encoding() {
        CharacterHeaderLayout::ResurrectedLegacy
    } else {
        CharacterHeaderLayout::Legacy
    }
}

impl CharacterHeaderLayout {
    pub const fn uses_v105_mode_marker(self) -> bool {
        matches!(self, Self::ResurrectedV105)
    }

    fn name_offset(self) -> usize {
        match self {
            Self::Legacy => LEGACY_NAME_OFFSET,
            Self::ResurrectedLegacy => D2R_LEGACY_NAME_OFFSET,
            Self::ResurrectedV105 => D2R_V105_NAME_OFFSET,
        }
    }

    fn status_offset(self) -> usize {
        match self {
            Self::Legacy | Self::ResurrectedLegacy => LEGACY_STATUS_OFFSET,
            Self::ResurrectedV105 => D2R_V105_STATUS_OFFSET,
        }
    }

    fn progression_offset(self) -> Option<usize> {
        match self {
            Self::Legacy | Self::ResurrectedLegacy => None,
            Self::ResurrectedV105 => Some(D2R_V105_PROGRESSION_OFFSET),
        }
    }

    fn class_offset(self) -> usize {
        match self {
            Self::Legacy | Self::ResurrectedLegacy => LEGACY_CLASS_OFFSET,
            Self::ResurrectedV105 => D2R_V105_CLASS_OFFSET,
        }
    }

    fn level_offset(self) -> usize {
        match self {
            Self::Legacy | Self::ResurrectedLegacy => LEGACY_LEVEL_OFFSET,
            Self::ResurrectedV105 => D2R_V105_LEVEL_OFFSET,
        }
    }

    fn section_search_start(self) -> usize {
        match self {
            Self::Legacy | Self::ResurrectedLegacy => 0,
            Self::ResurrectedV105 => D2R_V105_HEADER_LEN,
        }
    }
}

fn fixed_c_string(bytes: &[u8]) -> String {
    let len = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

#[cfg(test)]
mod tests {
    use crate::core::inventory::GridSize;
    use crate::core::object::item::{Item, ItemContainer, ItemDestination, ItemFlags};
    use crate::core::unit_stat::UnitStat;
    use crate::core::update::Update;
    use crate::core::version::{CharacterStatus, ExpansionMode, GameEdition, SaveVersion};
    use crate::core::{game_state::GameState, quest, waypoint};
    use crate::{CharacterClass, ServerMessage, SkillDescription};

    use super::{
        CHECKSUM_OFFSET, CharacterExportError, CharacterExportOptions, CharacterFile,
        CharacterFileError, CharacterHeaderLayout, CharacterProgression, CharacterStat,
        D2R_LEGACY_NAME_OFFSET, D2R_V105_CLASS_OFFSET, D2R_V105_HEADER_LEN, D2R_V105_LEVEL_OFFSET,
        D2R_V105_MERC_ID_OFFSET, D2R_V105_MERC_NAME_SEED_OFFSET, D2R_V105_MERC_STATUS_OFFSET,
        D2R_V105_MERC_XP_OFFSET, D2R_V105_MODE_MARKER_OFFSET, D2R_V105_NAME_OFFSET,
        D2R_V105_PROGRESSION_OFFSET, D2R_V105_RESERVED_CHECKSUM_MASK_OFFSET,
        D2R_V105_RESERVED_VERSION_MARKERS_OFFSET, D2R_V105_STATUS_OFFSET, D2S_MAGIC,
        FILE_SIZE_OFFSET, LEGACY_CLASS_OFFSET, LEGACY_FULL_EXPORT_PRE_STATS_LEN,
        LEGACY_LEVEL_OFFSET, LEGACY_NAME_OFFSET, LEGACY_NPC_HEADER_OFFSET,
        LEGACY_QUEST_HEADER_OFFSET, LEGACY_STATUS_OFFSET, LEGACY_WAYPOINT_HEADER_OFFSET,
        LEGACY_WAYPOINT_TRAILER_OFFSET, SaveSectionMarker, V105_NPC_HEADER_OFFSET,
        V105_WAYPOINT_HEADER_OFFSET, VERSION_OFFSET, calculate_checksum, read_bits,
    };

    #[test]
    fn save_section_markers_map_to_literal_d2s_bytes() {
        assert_eq!(SaveSectionMarker::Stats.raw_bytes(), b"gf");
        assert_eq!(SaveSectionMarker::Skills.raw_bytes(), b"if");
        assert_eq!(SaveSectionMarker::ItemList.raw_bytes(), b"JM");
        assert_eq!(SaveSectionMarker::Corpse.raw_bytes(), b"jf");
        assert_eq!(SaveSectionMarker::IronGolem.raw_bytes(), b"kf");
        assert_eq!(SaveSectionMarker::Followers.raw_bytes(), b"lf");
        assert_eq!(SaveSectionMarker::ItemList.description(), "item list");
    }

    #[test]
    fn parses_lod_header_and_selects_lod_inventory_profile() {
        let mut raw = build_save(0x60, "Cinder", 96);
        raw[LEGACY_STATUS_OFFSET] = CharacterStatus {
            expansion: true,
            ladder: true,
            ..CharacterStatus::default()
        }
        .to_byte();
        raw[LEGACY_CLASS_OFFSET] = CharacterClass::Sorceress as u8;
        raw[LEGACY_LEVEL_OFFSET] = 87;
        fix_test_header(&mut raw);

        let file = CharacterFile::parse(raw).expect("valid LoD file should parse");

        assert_eq!(file.header().edition, GameEdition::LordOfDestruction);
        assert_eq!(file.header().save_version, SaveVersion::Lod110Plus);
        assert_eq!(file.header().name, "Cinder");
        assert_eq!(file.header().class, Some(CharacterClass::Sorceress));
        assert_eq!(file.header().level, 87);
        assert_eq!(file.inventory_profile().stash, Some(GridSize::new(6, 8)));
    }

    #[test]
    fn parses_classic_header_from_non_expansion_status() {
        let mut raw = build_save(0x60, "NoXpac", 96);
        raw[LEGACY_CLASS_OFFSET] = CharacterClass::Barbarian as u8;
        fix_test_header(&mut raw);

        let file = CharacterFile::parse(raw).expect("valid classic file should parse");

        assert_eq!(file.header().edition, GameEdition::Classic);
        assert_eq!(file.inventory_profile().stash, Some(GridSize::new(6, 4)));
    }

    #[test]
    fn parses_resurrected_name_from_d2r_header_location() {
        let mut raw = build_save(0x62, "LegacyIgnored", 320);
        write_fixed_name(&mut raw, LEGACY_NAME_OFFSET, "OldName");
        write_fixed_name(&mut raw, D2R_LEGACY_NAME_OFFSET, "Modern");
        raw[LEGACY_CLASS_OFFSET] = CharacterClass::Paladin as u8;
        fix_test_header(&mut raw);

        let file = CharacterFile::parse(raw).expect("valid D2R file should parse");

        assert_eq!(file.header().edition, GameEdition::Resurrected);
        assert_eq!(
            file.header().layout,
            CharacterHeaderLayout::ResurrectedLegacy
        );
        assert_eq!(file.header().save_version, SaveVersion::Resurrected(0x62));
        assert_eq!(file.header().name, "Modern");
        assert_eq!(file.inventory_profile().stash, Some(GridSize::new(10, 10)));
        assert_eq!(file.inventory_profile().shared_stash_pages, 3);
    }

    #[test]
    fn parses_resurrected_v105_header_offsets_and_merc_header() {
        let mut raw = build_v105_save("Aster", CharacterClass::Paladin, 91);
        raw[D2R_V105_STATUS_OFFSET] = CharacterStatus {
            hardcore: true,
            ..CharacterStatus::default()
        }
        .to_byte();
        raw[D2R_V105_PROGRESSION_OFFSET] = 0x0f;
        raw[D2R_V105_MERC_NAME_SEED_OFFSET..D2R_V105_MERC_NAME_SEED_OFFSET + 4]
            .copy_from_slice(&0x1122_3344u32.to_le_bytes());
        raw[D2R_V105_MERC_STATUS_OFFSET..D2R_V105_MERC_STATUS_OFFSET + 2]
            .copy_from_slice(&13u16.to_le_bytes());
        raw[D2R_V105_MERC_ID_OFFSET..D2R_V105_MERC_ID_OFFSET + 2]
            .copy_from_slice(&35u16.to_le_bytes());
        raw[D2R_V105_MERC_XP_OFFSET..D2R_V105_MERC_XP_OFFSET + 4]
            .copy_from_slice(&123_456u32.to_le_bytes());
        fix_test_header(&mut raw);

        let file = CharacterFile::parse(raw).expect("valid D2R v105 file should parse");
        let mercenary = file.header().mercenary.expect("merc header should parse");

        assert_eq!(file.header().layout, CharacterHeaderLayout::ResurrectedV105);
        assert_eq!(file.header().save_version, SaveVersion::Resurrected(105));
        assert_eq!(file.header().name, "Aster");
        assert_eq!(file.header().class, Some(CharacterClass::Paladin));
        assert_eq!(file.header().level, 91);
        assert_eq!(file.header().progression, Some(CharacterProgression::Hell));
        assert_eq!(mercenary.name_seed, 0x1122_3344);
        assert_eq!(mercenary.status, 13);
        assert_eq!(mercenary.hireling_id, 35);
        assert_eq!(mercenary.experience, 123_456);
    }

    #[test]
    fn warlock_v105_save_selects_reign_inventory_profile() {
        let mut raw = build_v105_save("Malphas", CharacterClass::Warlock, 80);
        raw[D2R_V105_MODE_MARKER_OFFSET] = ExpansionMode::V105_ROTW_MARKER;
        fix_test_header(&mut raw);

        let file = CharacterFile::parse(raw).expect("valid Warlock file should parse");

        assert_eq!(file.header().edition, GameEdition::ReignOfTheWarlock);
        assert_eq!(file.header().expansion_mode, ExpansionMode::RotW);
        assert_eq!(file.header().class, Some(CharacterClass::Warlock));
        assert_eq!(file.inventory_profile().stash, Some(GridSize::new(10, 8)));
    }

    #[test]
    fn default_rotw_v105_uses_native_rotw_signatures() {
        let file =
            CharacterFile::default_rotw(CharacterClass::Amazon, "RotWAma").expect("template");
        let raw = file.raw_bytes();

        assert_eq!(file.header().edition, GameEdition::ReignOfTheWarlock);
        assert_eq!(file.header().expansion_mode, ExpansionMode::RotW);
        assert_eq!(raw[D2R_V105_STATUS_OFFSET], 0x00);
        assert_eq!(
            raw[D2R_V105_MODE_MARKER_OFFSET],
            ExpansionMode::V105_ROTW_MARKER
        );
        assert_eq!(
            &raw[D2R_V105_RESERVED_VERSION_MARKERS_OFFSET
                ..D2R_V105_RESERVED_VERSION_MARKERS_OFFSET + 2],
            &[0x10, 0x1e]
        );
        assert_eq!(
            &raw[D2R_V105_RESERVED_CHECKSUM_MASK_OFFSET
                ..D2R_V105_RESERVED_CHECKSUM_MASK_OFFSET + 4],
            &[0xff, 0xff, 0xff, 0xff]
        );
        assert_eq!(
            &raw[V105_WAYPOINT_HEADER_OFFSET..V105_WAYPOINT_HEADER_OFFSET + 8],
            &[0x57, 0x53, 0x01, 0x00, 0x00, 0x00, 0x50, 0x00]
        );
        assert_eq!(
            &raw[V105_NPC_HEADER_OFFSET..V105_NPC_HEADER_OFFSET + 4],
            &[0x01, 0x77, 0x34, 0x00]
        );
        assert!(raw.ends_with(&[
            0x4a, 0x4d, 0x00, 0x00, 0x6a, 0x66, 0x6b, 0x66, 0x00, 0x01, 0x00, 0x6c, 0x66, 0x00,
            0x00,
        ]));
    }

    #[test]
    fn v105_replace_quests_writes_v105_progression_without_legacy_overwrite() {
        let mut file =
            CharacterFile::default_rotw(CharacterClass::Amazon, "Quested").expect("template");
        let mut quests = quest::initial_template_quests();
        quest::set_quest_completed(&mut quests[2][quest::EVE_OF_DESTRUCTION], true);

        file.replace_quests(&quests).expect("quests should update");

        assert_eq!(
            file.raw_bytes()[D2R_V105_PROGRESSION_OFFSET],
            quest::PROGRESSION_HELL_COMPLETED
        );
        assert_eq!(
            &file.raw_bytes()[D2R_V105_RESERVED_CHECKSUM_MASK_OFFSET
                ..D2R_V105_RESERVED_CHECKSUM_MASK_OFFSET + 4],
            &[0xff, 0xff, 0xff, 0xff]
        );
    }

    #[test]
    fn v105_replace_waypoints_preserves_v105_waypoint_header() {
        let mut file =
            CharacterFile::default_rotw(CharacterClass::Amazon, "Waypoints").expect("template");
        let mut waypoints = [[false; waypoint::WAYPOINT_COUNT]; 3];
        waypoints[2][38] = true;

        file.replace_waypoints(&waypoints)
            .expect("waypoints should update");

        assert_eq!(
            &file.raw_bytes()[V105_WAYPOINT_HEADER_OFFSET..V105_WAYPOINT_HEADER_OFFSET + 8],
            &[0x57, 0x53, 0x01, 0x00, 0x00, 0x00, 0x50, 0x00]
        );
    }

    #[test]
    fn sorceress_base_strength_matches_fresh_rotw_save() {
        let file =
            CharacterFile::default_rotw(CharacterClass::Sorceress, "RotWSorc").expect("template");

        assert_eq!(file.stat(CharacterStat::Strength), Some(10));
    }

    #[test]
    fn parses_v105_character_stats_and_skills_sections() {
        let mut raw = build_v105_save("Stats", CharacterClass::Necromancer, 42);
        raw.extend_from_slice(&encode_character_stats(&[
            (CharacterStat::Strength, 70),
            (CharacterStat::Energy, 45),
            (CharacterStat::Level, 42),
            (CharacterStat::Experience, 1_312_287),
        ]));
        raw.extend_from_slice(b"if");
        let mut skill_levels = [0; 30];
        skill_levels[0] = 1;
        skill_levels[12] = 20;
        raw.extend_from_slice(&skill_levels);
        fix_test_header(&mut raw);

        let file = CharacterFile::parse(raw).expect("valid stats file should parse");
        let skills = file.skills().expect("skills should parse");

        assert!(file.stats().terminator_found);
        assert_eq!(file.stat(CharacterStat::Strength), Some(70));
        assert_eq!(file.stat(CharacterStat::Level), Some(42));
        assert_eq!(file.stat(CharacterStat::Experience), Some(1_312_287));
        assert_eq!(skills.level_at_slot(0), Some(1));
        assert_eq!(skills.level_at_slot(12), Some(20));
    }

    #[test]
    fn parses_character_stats_after_zero_width_unknown_stats() {
        let mut raw = build_v105_save("Stats", CharacterClass::Necromancer, 42);
        let mut stats = Vec::new();
        stats.extend_from_slice(b"gf");
        let mut writer = TestBitWriter::default();
        writer.write_bits(16, 9);
        writer.write_bits(CharacterStat::Strength as u32, 9);
        writer.write_bits(30, CharacterStat::Strength.bit_width() as usize);
        writer.write_bits(16, 9);
        writer.write_bits(CharacterStat::Strength as u32, 9);
        writer.write_bits(70, CharacterStat::Strength.bit_width() as usize);
        writer.write_bits(CharacterStat::Level as u32, 9);
        writer.write_bits(42, CharacterStat::Level.bit_width() as usize);
        writer.write_bits(0x1ff, 9);
        stats.extend_from_slice(&writer.finish());
        raw.extend_from_slice(&stats);
        raw.extend_from_slice(b"if");
        raw.extend_from_slice(&[0; 30]);
        fix_test_header(&mut raw);

        let file = CharacterFile::parse(raw).expect("valid stats file should parse");

        assert!(file.stats().terminator_found);
        assert!(file.stats().entries.iter().any(|entry| entry.id == 16));
        assert_eq!(file.stat(CharacterStat::Strength), Some(70));
        assert_eq!(file.stat(CharacterStat::Level), Some(42));
    }

    #[test]
    fn parses_v105_item_related_section_headers() {
        let mut raw = build_v105_save("Items", CharacterClass::Warlock, 80);
        let first_jm = raw.len();
        raw.extend_from_slice(b"JM");
        raw.extend_from_slice(&2u16.to_le_bytes());
        raw.extend_from_slice(b"jf");
        let merc_jm = raw.len();
        raw.extend_from_slice(b"JM");
        raw.extend_from_slice(&1u16.to_le_bytes());
        let golem = raw.len();
        raw.extend_from_slice(b"kf\0");
        let follower = raw.len();
        raw.extend_from_slice(b"lf");
        raw.extend_from_slice(&1u16.to_le_bytes());
        raw.extend_from_slice(&[0x5a; 116]);
        fix_test_header(&mut raw);

        let file = CharacterFile::parse(raw).expect("valid item headers should parse");
        let item_lists = file.item_lists();

        assert_eq!(item_lists.item_lists.len(), 2);
        assert_eq!(item_lists.item_lists[0].marker_offset, first_jm);
        assert_eq!(item_lists.item_lists[0].parent_item_count, 2);
        assert_eq!(item_lists.item_lists[1].marker_offset, merc_jm);
        assert_eq!(item_lists.item_lists[1].parent_item_count, 1);
        assert_eq!(item_lists.corpse_marker_offset, Some(first_jm + 4));
        assert_eq!(
            item_lists.iron_golem.expect("golem marker").marker_offset,
            golem
        );
        assert!(!item_lists.iron_golem.expect("golem marker").active);

        let follower_block = item_lists.follower_block.expect("follower block");
        assert_eq!(follower_block.marker_offset, follower);
        assert_eq!(follower_block.follower_count, 1);
        assert_eq!(follower_block.payload_len, 116);
        assert!(follower_block.complete_116_byte_payloads);
    }

    #[test]
    fn checksum_validation_rejects_mutated_file() {
        let mut raw = build_save(0x60, "Broken", 96);
        fix_test_header(&mut raw);
        raw[LEGACY_LEVEL_OFFSET] = raw[LEGACY_LEVEL_OFFSET].wrapping_add(1);

        let error = CharacterFile::parse(raw).expect_err("checksum should fail");

        assert!(matches!(
            error,
            CharacterFileError::ChecksumMismatch {
                expected: _,
                calculated: _
            }
        ));
    }

    #[test]
    fn to_bytes_recalculates_size_and_checksum() {
        let mut raw = build_save(0x60, "Writer", 96);
        fix_test_header(&mut raw);
        let mut file = CharacterFile::parse(raw).expect("valid file should parse");
        file.raw.push(0x7f);

        let written = file.to_bytes();
        let size = u32::from_le_bytes(
            written[FILE_SIZE_OFFSET..FILE_SIZE_OFFSET + 4]
                .try_into()
                .unwrap(),
        );
        let checksum = u32::from_le_bytes(
            written[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4]
                .try_into()
                .unwrap(),
        );

        assert_eq!(size as usize, written.len());
        assert_eq!(checksum, calculate_checksum(&written));
    }

    #[test]
    fn exports_full_legacy_d2s_from_game_state_stats_and_skills() {
        let state = build_export_state();

        let file = CharacterFile::export_legacy_from_game_state(
            &state,
            CharacterExportOptions::from_game_state_skills(),
        )
        .expect("state should export");
        let header = file.header();

        assert_eq!(header.layout, CharacterHeaderLayout::Legacy);
        assert_eq!(header.save_version, SaveVersion::Lod110Plus);
        assert_eq!(header.edition, GameEdition::LordOfDestruction);
        assert_eq!(header.name, "Exported");
        assert_eq!(header.class, Some(CharacterClass::Sorceress));
        assert_eq!(header.level, 42);
        assert!(header.status.expansion);
        assert!(header.status.hardcore);
        assert!(header.status.ladder);
        assert_eq!(file.stat(CharacterStat::Strength), Some(50));
        assert_eq!(file.stat(CharacterStat::Energy), Some(35));
        assert_eq!(file.stat(CharacterStat::Level), Some(42));
        assert_eq!(file.stat(CharacterStat::Experience), Some(123_456));
        assert_eq!(file.stat(CharacterStat::Gold), Some(12_345));
        assert_eq!(file.stat(CharacterStat::StashedGold), Some(54_321));
        assert_eq!(file.stat(CharacterStat::HitPoints), Some(2048));
        assert_eq!(file.stat(CharacterStat::MaxHitPoints), Some(2048));
        assert_eq!(file.stat(CharacterStat::Mana), Some(4096));
        assert_eq!(file.stat(CharacterStat::MaxMana), Some(4096));
        let parsed_skills = file.skills().expect("skills should be exported");
        assert_eq!(parsed_skills.level_at_slot(0), Some(1));
        assert_eq!(parsed_skills.level_at_slot(12), Some(20));
        assert_eq!(
            super::find_section_marker(file.raw_bytes(), SaveSectionMarker::Stats, 0),
            Some(LEGACY_FULL_EXPORT_PRE_STATS_LEN)
        );
        assert_eq!(
            &file.raw_bytes()[LEGACY_QUEST_HEADER_OFFSET..LEGACY_QUEST_HEADER_OFFSET + 4],
            b"Woo!"
        );
        assert_eq!(
            &file.raw_bytes()[LEGACY_WAYPOINT_HEADER_OFFSET..LEGACY_WAYPOINT_HEADER_OFFSET + 2],
            b"WS"
        );
        assert_eq!(file.raw_bytes()[LEGACY_WAYPOINT_TRAILER_OFFSET], 0x01);
        let quests = quest::parse_legacy_quest_words(file.raw_bytes(), 0).expect("quests exported");
        assert_eq!(
            quests[2][quest::QuestLogEntry::DenOfEvil.index()],
            quest::QUEST_CLOSED_COMPLETE
        );
        assert_eq!(
            quests[2][quest::QuestLogEntry::PrisonOfIce.index()]
                & quest::QUEST_PRISON_OF_ICE_SCROLL_CONSUMED,
            quest::QUEST_PRISON_OF_ICE_SCROLL_CONSUMED
        );
        let waypoints =
            waypoint::parse_legacy_waypoints(file.raw_bytes(), 0).expect("waypoints exported");
        assert!(!waypoints[0][0]);
        assert!(waypoints[2][0]);
        assert!(waypoints[2][38]);
        assert_eq!(
            &file.raw_bytes()[LEGACY_NPC_HEADER_OFFSET..LEGACY_NPC_HEADER_OFFSET + 2],
            b"w4"
        );
        assert_eq!(file.item_lists().item_lists.len(), 2);
        assert_eq!(file.item_lists().item_lists[0].parent_item_count, 0);
        assert_eq!(file.item_lists().item_lists[1].parent_item_count, 0);
        assert!(!file.item_lists().iron_golem.unwrap().active);

        let reparsed =
            CharacterFile::parse(file.to_bytes()).expect("exported D2S should parse again");
        assert_eq!(
            reparsed.header().checksum,
            calculate_checksum(reparsed.raw_bytes())
        );
    }

    #[test]
    fn overlays_legacy_template_with_state_stats_and_skills() {
        let mut raw = build_save(0x60, "Template", 96);
        raw[LEGACY_STATUS_OFFSET] = CharacterStatus {
            expansion: true,
            ..CharacterStatus::default()
        }
        .to_byte();
        raw[LEGACY_CLASS_OFFSET] = CharacterClass::Amazon as u8;
        raw[LEGACY_LEVEL_OFFSET] = 1;
        raw.extend_from_slice(&encode_character_stats(&[
            (CharacterStat::Strength, 10),
            (CharacterStat::Level, 1),
        ]));
        raw.extend_from_slice(b"if");
        raw.extend_from_slice(&[3; 30]);
        raw.extend_from_slice(b"JM");
        raw.extend_from_slice(&7u16.to_le_bytes());
        raw.extend_from_slice(b"preserved-trailer");
        fix_test_header(&mut raw);
        let template = CharacterFile::parse(raw).expect("template should parse");

        let state = build_export_state();
        let mut skills = [0; 30];
        skills[7] = 9;
        let exported = template
            .overlay_game_state(&state, CharacterExportOptions::new(skills))
            .expect("template overlay should export");

        assert_eq!(exported.header().name, "Exported");
        assert_eq!(exported.header().class, Some(CharacterClass::Sorceress));
        assert_eq!(exported.stat(CharacterStat::Strength), Some(50));
        assert_eq!(exported.stat(CharacterStat::Level), Some(42));
        assert_eq!(exported.skills().unwrap().level_at_slot(7), Some(9));
        assert_eq!(exported.item_lists().item_lists.len(), 1);
        assert_eq!(exported.item_lists().item_lists[0].parent_item_count, 7);
        assert!(exported.raw_bytes().ends_with(b"preserved-trailer"));
    }

    #[test]
    fn export_requires_local_player() {
        let error = CharacterFile::export_legacy_from_game_state(
            &GameState::default(),
            CharacterExportOptions::default(),
        )
        .expect_err("empty state cannot export");

        assert!(matches!(error, CharacterExportError::NoLocalPlayer));
    }

    #[test]
    fn exports_local_inventory_items_into_player_item_list() {
        let mut state = build_export_state();
        state.items.insert(
            0x2000,
            Item::from_owned_packet(
                0x2000,
                0x04,
                0x10,
                0,
                0x1000,
                inventory_item_bitstream("cm1", 2, 3, 0x02),
            ),
        );
        state.items.insert(
            0x3000,
            Item::from_owned_packet(
                0x3000,
                0x04,
                0x10,
                0,
                0x1000,
                inventory_item_bitstream("cm2", 1, 1, 0x0A),
            ),
        );

        let file = CharacterFile::export_legacy_from_game_state(
            &state,
            CharacterExportOptions::from_game_state_skills(),
        )
        .expect("state with inventory item should export");

        assert!(!file.item_lists().item_lists.is_empty());
        assert_eq!(file.item_lists().item_lists[0].parent_item_count, 1);

        let first_list = file.item_lists().item_lists[0].marker_offset;
        assert_eq!(
            &file.raw_bytes()[first_list + 4..first_list + 6],
            SaveSectionMarker::ItemList.raw_bytes()
        );
        let item_offset = first_list + 4;
        assert_eq!(
            read_bits(file.raw_bytes(), item_offset * 8 + 58, 3),
            Some(0)
        );
        assert_eq!(
            read_bits(file.raw_bytes(), item_offset * 8 + 61, 4),
            Some(0)
        );
        assert_eq!(
            read_bits(file.raw_bytes(), item_offset * 8 + 65, 4),
            Some(2)
        );
        assert_eq!(
            read_bits(file.raw_bytes(), item_offset * 8 + 69, 4),
            Some(3)
        );
    }

    #[test]
    fn exports_local_equipped_items_into_player_item_list() {
        let mut state = build_export_state();
        state.items.insert(
            0x2000,
            Item::from_owned_packet(
                0x2000,
                0x06,
                0x01,
                0,
                0x1000,
                equipped_item_bitstream("qui", 3),
            ),
        );

        let file = CharacterFile::export_legacy_from_game_state(
            &state,
            CharacterExportOptions::from_game_state_skills(),
        )
        .expect("state with equipped item should export");

        assert!(!file.item_lists().item_lists.is_empty());
        assert_eq!(file.item_lists().item_lists[0].parent_item_count, 1);

        let first_list = file.item_lists().item_lists[0].marker_offset;
        let item_offset = first_list + 4;
        assert_eq!(
            read_bits(file.raw_bytes(), item_offset * 8 + 58, 3),
            Some(1)
        );
        assert_eq!(
            read_bits(file.raw_bytes(), item_offset * 8 + 61, 4),
            Some(3)
        );
        assert_eq!(
            read_bits(file.raw_bytes(), item_offset * 8 + 65, 4),
            Some(0)
        );
        assert_eq!(
            read_bits(file.raw_bytes(), item_offset * 8 + 69, 4),
            Some(0)
        );
    }

    fn build_save(version: u32, name: &str, len: usize) -> Vec<u8> {
        let mut raw = vec![0; len];
        raw[0..4].copy_from_slice(&D2S_MAGIC.to_le_bytes());
        raw[VERSION_OFFSET..VERSION_OFFSET + 4].copy_from_slice(&version.to_le_bytes());
        raw[FILE_SIZE_OFFSET..FILE_SIZE_OFFSET + 4].copy_from_slice(&(len as u32).to_le_bytes());
        write_fixed_name(&mut raw, LEGACY_NAME_OFFSET, name);
        raw[LEGACY_LEVEL_OFFSET] = 1;
        raw
    }

    fn build_v105_save(name: &str, class: CharacterClass, level: u8) -> Vec<u8> {
        let mut raw = vec![0; D2R_V105_HEADER_LEN];
        raw[0..4].copy_from_slice(&D2S_MAGIC.to_le_bytes());
        raw[VERSION_OFFSET..VERSION_OFFSET + 4].copy_from_slice(&105u32.to_le_bytes());
        raw[FILE_SIZE_OFFSET..FILE_SIZE_OFFSET + 4]
            .copy_from_slice(&(D2R_V105_HEADER_LEN as u32).to_le_bytes());
        raw[D2R_V105_CLASS_OFFSET] = class as u8;
        raw[D2R_V105_LEVEL_OFFSET] = level;
        write_fixed_name(&mut raw, D2R_V105_NAME_OFFSET, name);
        raw
    }

    fn write_fixed_name(raw: &mut [u8], offset: usize, name: &str) {
        let bytes = name.as_bytes();
        let len = bytes.len().min(15);
        raw[offset..offset + 16].fill(0);
        raw[offset..offset + len].copy_from_slice(&bytes[..len]);
    }

    fn fix_test_header(raw: &mut [u8]) {
        let len = raw.len() as u32;
        raw[FILE_SIZE_OFFSET..FILE_SIZE_OFFSET + 4].copy_from_slice(&len.to_le_bytes());
        raw[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4].fill(0);
        let checksum = calculate_checksum(raw);
        raw[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4].copy_from_slice(&checksum.to_le_bytes());
    }

    fn build_export_state() -> GameState {
        let mut state = GameState::default();
        assert!(state.update(ServerMessage::GameFlags {
            difficulty: 2,
            arena_flags: 0x0000_0800,
            is_expansion: 1,
            is_ladder: 1,
        }));
        assert!(state.update(ServerMessage::LoadAct {
            act: 0,
            map_id: 0x1234_5678,
            area_id: 1,
            automap: 0,
        }));
        assert!(state.update(ServerMessage::AssignPlayer {
            unit_id: 0x1000,
            class: CharacterClass::Sorceress as u8,
            szname: name16("Exported"),
            x: 5100,
            y: 5200,
        }));
        assert!(state.update(ServerMessage::PlayerSkillsInfo {
            skills_count: 2,
            player_id: 0x1000,
            skills: vec![
                SkillDescription {
                    skill: 36,
                    level: 1,
                },
                SkillDescription {
                    skill: 48,
                    level: 20,
                },
            ],
        }));
        assert!(state.update(ServerMessage::GameHandshake {
            unit_type: 0,
            unit_id: 0x1000,
        }));
        assert!(state.update(ServerMessage::SetAttributeU16 {
            attribute: UnitStat::Strength as u8,
            amount: 50,
        }));
        assert!(state.update(ServerMessage::SetAttributeU16 {
            attribute: UnitStat::Energy as u8,
            amount: 35,
        }));
        assert!(state.update(ServerMessage::SetAttributeU16 {
            attribute: UnitStat::LifeMax as u8,
            amount: 2048,
        }));
        assert!(state.update(ServerMessage::SetAttributeU16 {
            attribute: UnitStat::ManaMax as u8,
            amount: 4096,
        }));
        assert!(state.update(ServerMessage::SetAttributeU16 {
            attribute: UnitStat::Level as u8,
            amount: 42,
        }));
        assert!(state.update(ServerMessage::SetAttributeU32 {
            attribute: UnitStat::Experience as u8,
            amount: 123_456,
        }));
        assert!(state.update(ServerMessage::SetAttributeU32 {
            attribute: UnitStat::GoldOnCharacter as u8,
            amount: 12_345,
        }));
        assert!(state.update(ServerMessage::SetAttributeU32 {
            attribute: UnitStat::GoldInStash as u8,
            amount: 54_321,
        }));
        let mut quest_bits = [0u8; 41];
        quest_bits[quest::QuestLogEntry::DenOfEvil.index()] =
            quest::QuestLogEntryState::COMPLETED_BIT
                | quest::QuestLogEntryState::REQUIREMENT_COMPLETED_BIT;
        quest_bits[quest::QuestLogEntry::PrisonOfIce.index()] =
            quest::QuestLogEntryState::COMPLETED_BIT;
        assert!(state.update(ServerMessage::PlayerQuestLogInfo { quest_bits }));
        assert!(state.update(ServerMessage::WaypointMenu {
            unit_id: 0x1000,
            unknown: 0,
            waypoint_bits: [0b0000_0001, 0, 0, 0, 0b0100_0000, 0, 0, 0],
            unused: [0; 6],
        }));
        assert!(state.update(ServerMessage::LifeManaUpdate {
            bitfield: status_bitfield::<12>(&[
                (8, 15),
                (16, 15),
                (555, 15),
                (5101, 16),
                (5201, 16),
                (0, 8),
                (0, 8),
            ]),
        }));
        state
    }

    fn name16(name: &str) -> [u8; 16] {
        let mut bytes = [0; 16];
        let name = name.as_bytes();
        let len = name.len().min(bytes.len());
        bytes[..len].copy_from_slice(&name[..len]);
        bytes
    }

    fn status_bitfield<const N: usize>(values: &[(u32, usize)]) -> [u8; N] {
        let mut bytes = [0; N];
        let mut bit_offset = 0;
        for (value, count) in values {
            for index in 0..*count {
                let bit = ((value >> index) & 1) as u8;
                bytes[bit_offset / 8] |= bit << (bit_offset % 8);
                bit_offset += 1;
            }
        }
        bytes
    }

    fn inventory_item_bitstream(code: &str, x: u8, y: u8, container: u8) -> Vec<u8> {
        let mut writer = TestBitWriter::default();
        writer.write_bits(ItemFlags::IDENTIFIED, 32);
        writer.write_bits(0x60, 8);
        writer.write_bits(0, 2);
        writer.write_bits(ItemDestination::Cursor.packet_value() as u32, 3);
        writer.write_bits(0, 4);
        writer.write_bits(x as u32, 4);
        writer.write_bits(y as u32, 3);
        writer.write_bits(container as u32, 4);
        write_item_code(&mut writer, code);
        writer.write_bits(0, 3);
        writer.write_bits(12, 7);
        writer.write_bits(2, 4);
        writer.write_bits(0, 1);
        writer.write_bits(0, 1);
        writer.write_bits(0, 1);
        writer.write_bits(0x1ff, 9);
        writer.finish()
    }

    fn equipped_item_bitstream(code: &str, equipment_location: u8) -> Vec<u8> {
        let mut writer = TestBitWriter::default();
        writer.write_bits(ItemFlags::IDENTIFIED | ItemFlags::EQUIPPED, 32);
        writer.write_bits(0x60, 8);
        writer.write_bits(0, 2);
        writer.write_bits(ItemDestination::Equipment.packet_value() as u32, 3);
        writer.write_bits(equipment_location as u32, 4);
        writer.write_bits(0, 4);
        writer.write_bits(0, 3);
        writer.write_bits(ItemContainer::Equipment.packet_value() as u32, 4);
        write_item_code(&mut writer, code);
        writer.write_bits(0, 3);
        writer.write_bits(12, 7);
        writer.write_bits(2, 4);
        writer.write_bits(0, 1);
        writer.write_bits(0, 1);
        writer.write_bits(0, 1);
        writer.write_bits(0x1ff, 9);
        writer.finish()
    }

    fn write_item_code(writer: &mut TestBitWriter, code: &str) {
        let mut raw = [b' '; 4];
        let bytes = code.as_bytes();
        let len = bytes.len().min(4);
        raw[..len].copy_from_slice(&bytes[..len]);
        for byte in raw {
            writer.write_bits(byte as u32, 8);
        }
    }

    fn encode_character_stats(stats: &[(CharacterStat, u32)]) -> Vec<u8> {
        let mut bits = Vec::new();
        bits.extend_from_slice(b"gf");
        let mut writer = TestBitWriter::default();
        for (stat, value) in stats {
            writer.write_bits(*stat as u32, 9);
            writer.write_bits(*value, stat.bit_width() as usize);
        }
        writer.write_bits(0x1ff, 9);
        bits.extend_from_slice(&writer.finish());
        bits
    }

    #[derive(Default)]
    struct TestBitWriter {
        bytes: Vec<u8>,
        bit_offset: usize,
    }

    impl TestBitWriter {
        fn write_bits(&mut self, value: u32, count: usize) {
            for index in 0..count {
                if self.bit_offset / 8 == self.bytes.len() {
                    self.bytes.push(0);
                }
                let bit = ((value >> index) & 1) as u8;
                self.bytes[self.bit_offset / 8] |= bit << (self.bit_offset % 8);
                self.bit_offset += 1;
            }
        }

        fn finish(self) -> Vec<u8> {
            self.bytes
        }
    }
}
