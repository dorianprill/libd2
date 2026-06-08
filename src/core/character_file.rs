use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crate::core::character_class::CharacterClass;
use crate::core::entity::player::Player;
use crate::core::game_state::GameState;
use crate::core::inventory::InventoryProfile;
use crate::core::unit_stat::UnitStat;
use crate::core::version::{detect_edition, CharacterStatus, GameEdition, SaveVersion};

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
const D2R_V105_LEVEL_OFFSET: usize = 0x1b;
const D2R_V105_MERC_NAME_SEED_OFFSET: usize = 0x0a3;
const D2R_V105_MERC_STATUS_OFFSET: usize = 0x0a7;
const D2R_V105_MERC_ID_OFFSET: usize = 0x0a9;
const D2R_V105_MERC_XP_OFFSET: usize = 0x0ab;
const D2R_V105_HEADER_LEN: usize = 0x150;
const LEGACY_FULL_EXPORT_PRE_STATS_LEN: usize = 0x2fd;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl Default for CharacterExportOptions {
    fn default() -> Self {
        Self { skills: None }
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
        append_empty_legacy_item_sections(&mut raw, snapshot.status.expansion);
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
    pub fn overlay_legacy_game_state(
        &self,
        state: &GameState,
        options: CharacterExportOptions,
    ) -> Result<Self, CharacterExportError> {
        if self.header.layout != CharacterHeaderLayout::Legacy {
            return Err(CharacterExportError::UnsupportedTemplateLayout {
                layout: self.header.layout,
            });
        }

        let snapshot = LegacyExportSnapshot::from_game_state(state, options)?;
        let stats_section = encode_character_stats_section(&snapshot.stats)?;
        let skills_section = encode_character_skills_section(snapshot.skills);

        let mut raw = self.raw.clone();
        write_legacy_header_snapshot(&mut raw, &snapshot);
        replace_legacy_export_sections(&mut raw, &stats_section, &skills_section)?;
        fix_header(&mut raw);

        Self::parse(raw).map_err(CharacterExportError::CharacterFile)
    }
}

#[derive(Debug)]
pub enum CharacterFileError {
    Io(io::Error),
    TooSmall {
        len: usize,
        required: usize,
    },
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
    let mut edition = detect_edition(save_version, status);
    if save_version.uses_resurrected_item_encoding() && class_id == CharacterClass::Warlock as u8 {
        edition = GameEdition::ReignOfTheWarlock;
    }
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
    })
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

        let Some(stat) = CharacterStat::from_id(id as u16) else {
            break;
        };
        let width = stat.bit_width() as usize;
        if bit_offset + width > total_bits {
            break;
        }

        let Some(value) = read_bits(raw, bit_offset, width) else {
            break;
        };
        bit_offset += width;
        entries.push(CharacterStatEntry {
            id: id as u16,
            stat: Some(stat),
            value,
        });
    }

    CharacterStats {
        marker_offset: Some(marker_offset),
        entries,
        terminator_found,
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
    class: CharacterClass,
    level: u8,
    map_id: u32,
    stats: Vec<(CharacterStat, u32)>,
    skills: [u8; 30],
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

        let status = CharacterStatus {
            hardcore: state.is_hardcore(),
            died: false,
            expansion: state.is_expansion()
                || matches!(class, CharacterClass::Druid | CharacterClass::Assassin),
            ladder: state.is_ladder(),
        };

        Ok(Self {
            name,
            status,
            class,
            level,
            map_id: state.map().map_id.unwrap_or_default(),
            stats,
            skills,
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
            stats.push((stat, value));
        }
    }

    if let Some(vitals) = player.vitals() {
        if let Some(life) = vitals.life() {
            insert_stat_if_absent(&mut stats, CharacterStat::HitPoints, life as u32);
        }
        if let Some(mana) = vitals.mana() {
            insert_stat_if_absent(&mut stats, CharacterStat::Mana, mana as u32);
        }
        if let Some(stamina) = vitals.stamina() {
            insert_stat_if_absent(&mut stats, CharacterStat::Stamina, stamina as u32);
        }
    }

    stats
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

fn write_empty_legacy_quests(raw: &mut [u8]) {
    write_u32_le(raw, LEGACY_QUEST_UNKNOWN_OFFSET, 1);
    raw[LEGACY_QUEST_HEADER_OFFSET..LEGACY_QUEST_HEADER_OFFSET + 4].copy_from_slice(b"Woo!");
    raw[LEGACY_QUEST_MAGIC_OFFSET..LEGACY_QUEST_MAGIC_OFFSET + 6]
        .copy_from_slice(&[6, 0, 0, 0, 0x2a, 0x01]);
}

fn write_empty_legacy_waypoints(raw: &mut [u8]) {
    raw[LEGACY_WAYPOINT_HEADER_OFFSET..LEGACY_WAYPOINT_HEADER_OFFSET + 2].copy_from_slice(b"WS");
    raw[LEGACY_WAYPOINT_MAGIC_OFFSET..LEGACY_WAYPOINT_MAGIC_OFFSET + 6]
        .copy_from_slice(&[6, 0, 0, 0, 0x2a, 0x01]);
    for difficulty in 0..3 {
        let offset =
            LEGACY_WAYPOINT_DIFFICULTIES_OFFSET + difficulty * LEGACY_WAYPOINT_DIFFICULTY_LEN;
        raw[offset] = 0x02;
        raw[offset + 1] = 0x01;
    }

    // Public parsers disagree by one byte at the waypoint/NPC boundary. Actual
    // saves keep this `0x01` trailer immediately before the `w4` NPC marker;
    // D2SLib treats it as the low byte of a `0x7701` NPC header.
    raw[LEGACY_WAYPOINT_TRAILER_OFFSET] = 0x01;
}

fn write_empty_legacy_npc_dialogs(raw: &mut [u8]) {
    raw[LEGACY_NPC_HEADER_OFFSET..LEGACY_NPC_HEADER_OFFSET + 2].copy_from_slice(b"w4");
}

fn append_empty_legacy_item_sections(raw: &mut Vec<u8>, is_expansion: bool) {
    append_empty_item_list(raw);
    append_empty_item_list(raw);

    if is_expansion {
        raw.extend_from_slice(SaveSectionMarker::Corpse.raw_bytes());
        raw.extend_from_slice(SaveSectionMarker::IronGolem.raw_bytes());
        raw.push(0);
    }
}

fn append_empty_item_list(raw: &mut Vec<u8>) {
    raw.extend_from_slice(SaveSectionMarker::ItemList.raw_bytes());
    raw.extend_from_slice(&0u16.to_le_bytes());
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
    raw[offset..offset + bytes.len()].copy_from_slice(bytes);
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
    write_u32_le(raw, FILE_SIZE_OFFSET, raw.len() as u32);
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
    use crate::core::game_state::GameState;
    use crate::core::inventory::GridSize;
    use crate::core::unit_stat::UnitStat;
    use crate::core::update::Update;
    use crate::core::version::{CharacterStatus, GameEdition, SaveVersion};
    use crate::{CharacterClass, ServerMessage, SkillDescription};

    use super::{
        calculate_checksum, CharacterExportError, CharacterExportOptions, CharacterFile,
        CharacterFileError, CharacterHeaderLayout, CharacterProgression, CharacterStat,
        SaveSectionMarker, CHECKSUM_OFFSET, D2R_LEGACY_NAME_OFFSET, D2R_V105_CLASS_OFFSET,
        D2R_V105_HEADER_LEN, D2R_V105_LEVEL_OFFSET, D2R_V105_MERC_ID_OFFSET,
        D2R_V105_MERC_NAME_SEED_OFFSET, D2R_V105_MERC_STATUS_OFFSET, D2R_V105_MERC_XP_OFFSET,
        D2R_V105_NAME_OFFSET, D2R_V105_PROGRESSION_OFFSET, D2R_V105_STATUS_OFFSET, D2S_MAGIC,
        FILE_SIZE_OFFSET, LEGACY_CLASS_OFFSET, LEGACY_FULL_EXPORT_PRE_STATS_LEN,
        LEGACY_LEVEL_OFFSET, LEGACY_NAME_OFFSET, LEGACY_NPC_HEADER_OFFSET,
        LEGACY_QUEST_HEADER_OFFSET, LEGACY_STATUS_OFFSET, LEGACY_WAYPOINT_HEADER_OFFSET,
        LEGACY_WAYPOINT_TRAILER_OFFSET, VERSION_OFFSET,
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
        fix_test_header(&mut raw);

        let file = CharacterFile::parse(raw).expect("valid Warlock file should parse");

        assert_eq!(file.header().edition, GameEdition::ReignOfTheWarlock);
        assert_eq!(file.header().class, Some(CharacterClass::Warlock));
        assert_eq!(file.inventory_profile().stash, Some(GridSize::new(10, 8)));
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
        assert_eq!(file.stat(CharacterStat::HitPoints), Some(777));
        assert_eq!(file.stat(CharacterStat::MaxHitPoints), Some(2048));
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
            .overlay_legacy_game_state(&state, CharacterExportOptions::new(skills))
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
            attribute: UnitStat::Level as u8,
            amount: 42,
        }));
        assert!(state.update(ServerMessage::SetAttributeU32 {
            attribute: UnitStat::Experience as u8,
            amount: 123_456,
        }));
        assert!(state.update(ServerMessage::LifeManaUpdate {
            bitfield: status_bitfield::<12>(&[
                (777, 15),
                (333, 15),
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
