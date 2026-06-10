//! Static Diablo II game data loaded from legacy MPQ archives.
//!
//! Classic and Lord of Destruction network packets identify many things by
//! numeric ids or compact item codes. Monster assignment packet `0xAC`, for
//! example, carries a monster class id; the human-readable name lives in
//! `data/global/excel/monstats.bin`, which in turn points into the language
//! `.tbl` files. This module provides a read-only loader for those legacy MPQ
//! assets so packet-derived game state can be decorated without reading game
//! memory.
//!
//! The loader is intentionally scoped to the MPQ-based Classic/LoD install
//! layout: `patch_d2.mpq` overrides `d2exp.mpq`, which overrides
//! `d2data.mpq`. D2R/RotW use different asset packaging and should get a
//! separate loader rather than being forced through this format.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::core::mpq::{MpqArchive, MpqError};

const PATCH_OFFSET: usize = 10_000;
const EXPANSION_OFFSET: usize = 20_000;

const MONSTATS_RECORD_SIZE: usize = 424;
const MONSTATS2_RECORD_SIZE: usize = 0x134;
const OBJECTS_RECORD_SIZE: usize = 448;
const LEVELS_RECORD_SIZE: usize = 544;
const ITEM_RECORD_SIZE: usize = 424;

const LANGUAGE_FILES: &[(&str, LanguageTableKind)] = &[
    ("data/local/LNG/ENG/string.tbl", LanguageTableKind::Classic),
    (
        "data/local/LNG/ENG/expansionstring.tbl",
        LanguageTableKind::Expansion,
    ),
    (
        "data/local/LNG/ENG/patchstring.tbl",
        LanguageTableKind::Patch,
    ),
];

const ITEM_FILES: &[&str] = &[
    "data/global/excel/Weapons.bin",
    "data/global/excel/Armor.bin",
    "data/global/excel/Misc.bin",
];

/// Read-only static data needed to resolve packet ids and item codes.
#[derive(Debug, Clone, Default)]
pub struct GameData {
    source_path: Option<PathBuf>,
    language: LanguageTables,
    monsters: HashMap<u32, MonsterRecord>,
    monster_states: HashMap<u32, [u8; 16]>,
    monster_count: usize,
    objects: HashMap<u32, ObjectRecord>,
    levels: HashMap<u32, LevelRecord>,
    items_by_code: HashMap<String, ItemRecord>,
}

impl GameData {
    /// Loads Classic/LoD static data from an installed Diablo II directory.
    ///
    /// The directory is only read. Archive precedence follows the game client:
    /// `patch_d2.mpq` first, then `d2exp.mpq`, then `d2data.mpq`. The first
    /// implementation loads the English legacy language tables under
    /// `data/local/LNG/ENG`; other locales need a small path-selection layer.
    pub fn load_classic_lod_install(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let path = path.as_ref();
        let archives = ClassicLodArchives::open(path)?;
        let mut data = Self {
            source_path: Some(path.to_path_buf()),
            ..Self::default()
        };

        for (file, kind) in LANGUAGE_FILES {
            if let Some(bytes) = archives.read_file(file)? {
                data.language.add_table(*kind, parse_language_tbl(&bytes)?);
            }
        }

        if let Some(bytes) = archives.read_file("data/global/excel/monstats.bin")? {
            for monster in parse_monstats_bin(&bytes)? {
                data.monster_count += 1;
                data.monsters.insert(monster.id, monster);
            }
        }

        if let Some(bytes) = archives.read_file("data/global/excel/MonStats2.bin")? {
            for state in parse_monstats2_bin(&bytes)? {
                data.monster_states.insert(state.id, state.states);
            }
        }

        if let Some(bytes) = archives.read_file("data/global/excel/objects.bin")? {
            for (index, object) in parse_objects_bin(&bytes)?.into_iter().enumerate() {
                data.objects.insert(index as u32, object);
            }
        }

        if let Some(bytes) = archives.read_file("data/global/excel/levels.bin")? {
            for level in parse_levels_bin(&bytes)? {
                data.levels.insert(level.id, level);
            }
        }

        for item_file in ITEM_FILES {
            if let Some(bytes) = archives.read_file(item_file)? {
                for item in parse_items_bin(&bytes)? {
                    data.items_by_code.insert(item.code.clone(), item);
                }
            }
        }

        Ok(data)
    }

    /// Source install path, when the data came from a local Classic/LoD tree.
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Number of normal monster records loaded from `MonStats.bin`.
    pub fn monster_count(&self) -> usize {
        self.monster_count
    }

    /// Number of object records loaded from `Objects.bin`.
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Number of level records loaded from `Levels.bin`.
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Number of item records loaded from `Weapons.bin`, `Armor.bin`, and
    /// `Misc.bin`.
    pub fn item_count(&self) -> usize {
        self.items_by_code.len()
    }

    /// Resolves a monster class id from packet `0xAC MonsterAssign`.
    pub fn monster_name(&self, monster_id: u16) -> Option<&str> {
        let monster_id = monster_id as usize;
        if monster_id >= self.monster_count {
            return SUPER_UNIQUE_NAMES
                .get(monster_id - self.monster_count)
                .copied();
        }

        self.monsters
            .get(&(monster_id as u32))
            .and_then(|monster| self.language.get_by_index(monster.name_lang_id as usize))
    }

    /// Returns the state byte list from `MonStats2.bin` for a base monster id.
    ///
    /// These bytes are used by some NPC assignment variants to interpret state
    /// values sent in the packet bitstream.
    pub fn monster_states(&self, monster_id: u16) -> Option<&[u8; 16]> {
        self.monster_states.get(&(monster_id as u32))
    }

    /// Resolves an object class id from world-object packets.
    pub fn object_name(&self, object_id: u16) -> Option<&str> {
        self.objects.get(&(object_id as u32)).map(|object| {
            if object.name.is_empty() {
                object.token.as_str()
            } else {
                object.name.as_str()
            }
        })
    }

    /// Resolves a level id from map/warp packets.
    pub fn level_name(&self, level_id: u16) -> Option<&str> {
        self.levels.get(&(level_id as u32)).map(|level| {
            if level.name.is_empty() {
                level.warp.as_str()
            } else {
                level.name.as_str()
            }
        })
    }

    /// Resolves a three-letter item code from item packets.
    pub fn item_name(&self, code: &str) -> Option<&str> {
        self.items_by_code
            .get(code)
            .and_then(|item| self.language.get_by_index(item.name_id as usize))
    }
}

#[derive(Debug)]
struct ClassicLodArchives {
    archives: Vec<MpqArchive>,
}

impl ClassicLodArchives {
    fn open(path: &Path) -> Result<Self, GameDataError> {
        let mut archives = Vec::new();
        for archive_name in ["patch_d2.mpq", "d2exp.mpq", "d2data.mpq"] {
            let archive_path = path.join(archive_name);
            if archive_path.exists() {
                archives.push(MpqArchive::open(&archive_path)?);
            }
        }

        if archives.is_empty() {
            return Err(GameDataError::MissingArchives {
                path: path.to_path_buf(),
            });
        }

        Ok(Self { archives })
    }

    fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>, GameDataError> {
        for archive in &self.archives {
            if let Some(bytes) = archive.read_file(path)? {
                return Ok(Some(bytes));
            }
        }
        Ok(None)
    }
}

/// Error returned while loading static game data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameDataError {
    MissingArchives { path: PathBuf },
    Mpq(MpqError),
    Parse { file: &'static str, message: String },
}

impl fmt::Display for GameDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArchives { path } => write!(
                formatter,
                "no Classic/LoD MPQ archives found in {}",
                path.display()
            ),
            Self::Mpq(error) => write!(formatter, "{error}"),
            Self::Parse { file, message } => write!(formatter, "failed to parse {file}: {message}"),
        }
    }
}

impl std::error::Error for GameDataError {}

impl From<MpqError> for GameDataError {
    fn from(value: MpqError) -> Self {
        Self::Mpq(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LanguageTableKind {
    Classic,
    Patch,
    Expansion,
}

#[derive(Debug, Clone, Default)]
struct LanguageTables {
    classic: LanguageTable,
    patch: LanguageTable,
    expansion: LanguageTable,
}

impl LanguageTables {
    fn add_table(&mut self, kind: LanguageTableKind, table: LanguageTable) {
        match kind {
            LanguageTableKind::Classic => self.classic = table,
            LanguageTableKind::Patch => self.patch = table,
            LanguageTableKind::Expansion => self.expansion = table,
        }
    }

    fn get_by_index(&self, index: usize) -> Option<&str> {
        if index >= EXPANSION_OFFSET {
            return self.expansion.get(index - EXPANSION_OFFSET);
        }
        if index >= PATCH_OFFSET {
            return self.patch.get(index - PATCH_OFFSET);
        }
        self.classic.get(index)
    }
}

#[derive(Debug, Clone, Default)]
struct LanguageTable {
    by_index: Vec<Option<String>>,
    by_key: HashMap<String, String>,
}

impl LanguageTable {
    fn add(&mut self, key: String, index: usize, value: String) {
        if self.by_index.len() <= index {
            self.by_index.resize_with(index + 1, || None);
        }
        self.by_index[index] = Some(value.clone());

        if key.eq_ignore_ascii_case("x") || value.trim().is_empty() {
            return;
        }
        self.by_key.entry(key).or_insert(value);
    }

    fn get(&self, index: usize) -> Option<&str> {
        self.by_index.get(index).and_then(|value| value.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterRecord {
    pub id: u32,
    pub base_id: u32,
    pub base_next_id: u32,
    pub name_lang_id: u16,
    pub description_lang_id: u16,
    pub flags: u32,
    pub code: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterStateRecord {
    pub id: u32,
    pub states: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRecord {
    pub name: String,
    pub name_wide: String,
    pub token: String,
    pub is_door: bool,
    pub is_visibility_blocked: bool,
    pub orientation: u8,
    pub x_offset: u32,
    pub y_offset: u32,
    pub x_space: u8,
    pub y_space: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub subclass: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelRecord {
    pub id: u32,
    pub palette: u8,
    pub act: u8,
    pub is_teleport_enabled: bool,
    pub is_raining: bool,
    pub is_inside: bool,
    pub name: String,
    pub warp: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRecord {
    pub code: String,
    pub name_id: u16,
}

fn parse_language_tbl(bytes: &[u8]) -> Result<LanguageTable, GameDataError> {
    const FILE: &str = "language tbl";
    const HEADER_LEN: usize = 21;
    const HASH_NODE_LEN: usize = 17;

    ensure_len(bytes, HEADER_LEN, FILE)?;
    let count = read_u16(bytes, 0x02, FILE)? as usize;
    let count_hash = read_u32(bytes, 0x04, FILE)? as usize;
    let indexes_offset = HEADER_LEN;
    let hash_offset = indexes_offset + count * 2;
    ensure_len(bytes, hash_offset + count_hash * HASH_NODE_LEN, FILE)?;

    let mut table = LanguageTable::default();
    for node_index in 0..count_hash {
        let offset = hash_offset + node_index * HASH_NODE_LEN;
        let active = bytes[offset];
        if active == 0 {
            continue;
        }

        let index = read_u16(bytes, offset + 0x01, FILE)? as usize;
        let key_offset = read_u32(bytes, offset + 0x07, FILE)? as usize;
        let value_offset = read_u32(bytes, offset + 0x0b, FILE)? as usize;
        let value_len = read_u16(bytes, offset + 0x0f, FILE)? as usize;

        let key = read_c_string(bytes, key_offset, FILE)?;
        let value = if value_len > 0 && value_offset + value_len <= bytes.len() {
            let raw = &bytes[value_offset..value_offset + value_len.saturating_sub(1)];
            lossy_trimmed_string(raw)
        } else {
            read_c_string(bytes, value_offset, FILE)?
        };
        table.add(key, index, value);
    }

    Ok(table)
}

fn parse_monstats_bin(bytes: &[u8]) -> Result<Vec<MonsterRecord>, GameDataError> {
    counted_records(bytes, MONSTATS_RECORD_SIZE, "MonStats.bin", |record| {
        Ok(MonsterRecord {
            id: read_u16(record, 0x00, "MonStats.bin")? as u32,
            base_id: read_u16(record, 0x02, "MonStats.bin")? as u32,
            base_next_id: read_u16(record, 0x04, "MonStats.bin")? as u32,
            name_lang_id: read_u16(record, 0x06, "MonStats.bin")?,
            description_lang_id: read_u16(record, 0x08, "MonStats.bin")?,
            flags: read_u32(record, 0x0c, "MonStats.bin")?,
            code: read_u32(record, 0x10, "MonStats.bin")?,
        })
    })
}

fn parse_monstats2_bin(bytes: &[u8]) -> Result<Vec<MonsterStateRecord>, GameDataError> {
    counted_records(bytes, MONSTATS2_RECORD_SIZE, "MonStats2.bin", |record| {
        let mut states = [0; 16];
        states.copy_from_slice(&record[0x15..0x25]);
        Ok(MonsterStateRecord {
            id: read_u32(record, 0x00, "MonStats2.bin")?,
            states,
        })
    })
}

fn parse_objects_bin(bytes: &[u8]) -> Result<Vec<ObjectRecord>, GameDataError> {
    counted_records(bytes, OBJECTS_RECORD_SIZE, "Objects.bin", |record| {
        Ok(ObjectRecord {
            name: read_fixed_string(record, 0x00, 64, "Objects.bin")?,
            name_wide: read_fixed_string(record, 0x40, 64, "Objects.bin")?,
            token: read_fixed_string(record, 0x80, 3, "Objects.bin")?,
            is_door: record[0x13a] != 0,
            is_visibility_blocked: record[0x13b] != 0,
            orientation: record[0x13c],
            x_offset: read_u32(record, 0x148, "Objects.bin")?,
            y_offset: read_u32(record, 0x14c, "Objects.bin")?,
            x_space: record[0x162],
            y_space: record[0x163],
            red: record[0x164],
            green: record[0x165],
            blue: record[0x166],
            subclass: record[0x167],
        })
    })
}

fn parse_levels_bin(bytes: &[u8]) -> Result<Vec<LevelRecord>, GameDataError> {
    counted_records(bytes, LEVELS_RECORD_SIZE, "Levels.bin", |record| {
        Ok(LevelRecord {
            id: read_u16(record, 0x00, "Levels.bin")? as u32,
            palette: record[0x02],
            act: record[0x03],
            is_teleport_enabled: record[0x04] == 1,
            is_raining: record[0x05] == 1,
            is_inside: record[0x08] == 1,
            name: read_fixed_string(record, 0xf5, 40, "Levels.bin")?,
            warp: read_fixed_string(record, 0xf5 + 40, 40, "Levels.bin")?,
        })
    })
}

fn parse_items_bin(bytes: &[u8]) -> Result<Vec<ItemRecord>, GameDataError> {
    counted_records(bytes, ITEM_RECORD_SIZE, "item bin", |record| {
        Ok(ItemRecord {
            code: read_fixed_string(record, 0x80, 4, "item bin")?,
            name_id: read_u16(record, 0xf4, "item bin")?,
        })
    })
}

fn counted_records<T>(
    bytes: &[u8],
    record_size: usize,
    file: &'static str,
    mut parse: impl FnMut(&[u8]) -> Result<T, GameDataError>,
) -> Result<Vec<T>, GameDataError> {
    ensure_len(bytes, 4, file)?;
    let count = read_u32(bytes, 0, file)? as usize;
    let records_len = count
        .checked_mul(record_size)
        .ok_or_else(|| parse_error(file, "record table length overflows usize"))?;
    ensure_len(bytes, 4 + records_len, file)?;

    let mut records = Vec::with_capacity(count);
    for index in 0..count {
        let offset = 4 + index * record_size;
        records.push(parse(&bytes[offset..offset + record_size])?);
    }
    Ok(records)
}

fn read_fixed_string(
    bytes: &[u8],
    offset: usize,
    len: usize,
    file: &'static str,
) -> Result<String, GameDataError> {
    ensure_len(bytes, offset + len, file)?;
    let raw = &bytes[offset..offset + len];
    let nul = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
    Ok(lossy_trimmed_string(&raw[..nul]))
}

fn read_c_string(bytes: &[u8], offset: usize, file: &'static str) -> Result<String, GameDataError> {
    if offset >= bytes.len() {
        return Err(parse_error(
            file,
            format!("string offset {offset} is outside file"),
        ));
    }
    let raw = &bytes[offset..];
    let nul = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
    Ok(lossy_trimmed_string(&raw[..nul]))
}

fn lossy_trimmed_string(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw)
        .trim_end_matches(['\0', ' '])
        .to_owned()
}

fn ensure_len(bytes: &[u8], required: usize, file: &'static str) -> Result<(), GameDataError> {
    if bytes.len() < required {
        Err(parse_error(
            file,
            format!("file is {} bytes, need at least {required}", bytes.len()),
        ))
    } else {
        Ok(())
    }
}

fn read_u16(bytes: &[u8], offset: usize, file: &'static str) -> Result<u16, GameDataError> {
    ensure_len(bytes, offset + 2, file)?;
    Ok(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
}

fn read_u32(bytes: &[u8], offset: usize, file: &'static str) -> Result<u32, GameDataError> {
    ensure_len(bytes, offset + 4, file)?;
    Ok(u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}

fn parse_error(file: &'static str, message: impl Into<String>) -> GameDataError {
    GameDataError::Parse {
        file,
        message: message.into(),
    }
}

/// Super unique names are not language-table driven in Blaine's packet path.
const SUPER_UNIQUE_NAMES: &[&str] = &[
    "Bishibosh",
    "Bonebreaker",
    "Coldcrow",
    "Rakanishu",
    "Treehead WoodFist",
    "Griswold",
    "The Countess",
    "Pitspawn Fouldog",
    "Flamespike the Crawler",
    "Bone Ash",
    "Radament",
    "Bloodwitch the Wild",
    "Fangskin",
    "Beetleburst",
    "Creeping Feature",
    "Coldworm the Burrower",
    "Fire Eye",
    "Dark Elder",
    "The Summoner",
    "Ancient Kaa the Soulless",
    "The Smith",
    "Sszark the Burning",
    "Witch Doctor Endugu",
    "Stormtree",
    "Battlemaid Sarina",
    "Icehawk Riftwing",
    "Ismail Vilehand",
    "Geleb Flamefinger",
    "Bremm Sparkfist",
    "Toorc Icefist",
    "Wyand Voidbringer",
    "Maffer Dragonhand",
    "Darkwing",
    "The Tormentor",
    "Taintbreeder",
    "Riftwraith the Cannibal",
    "Infector of Souls",
    "Lord De Seis",
    "Grand Vizier of Chaos",
    "The Cow King",
    "Corpsefire",
    "Hephasto The Armorer",
    "Shenk the Overseer",
    "Talic",
    "Madawc",
    "Korlic",
    "Axe Dweller",
    "Bonesaw Breaker",
    "Dac Farren",
    "Eldritch the Rectifier",
    "Eyeback the Unleashed",
    "Thresh Socket",
    "Pindleskin",
    "Snapchip Shatter",
    "Hell's Belle",
    "Vinvear Molech",
    "Sharptooth Slayer",
    "Magma Torquer",
    "Blaze Ripper",
    "Frozenstein",
    "Nihlathak Boss",
    "Colenzo the Annihilator",
    "Achmel the Cursed",
    "Bartuc the Bloody",
    "Ventar the Unholy",
    "Lister the Tormentor",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mpq::MpqArchive;

    #[test]
    fn parses_language_tbl_from_mpq_fixture() {
        let archive = MpqArchive::open("tests/fixtures/mpq/test-pkware.mpq").expect("mpq");
        let bytes = archive
            .read_file("strings/pd2/patchstring.tbl")
            .expect("extract")
            .expect("patchstring");

        let table = parse_language_tbl(&bytes).expect("tbl parse");

        assert!(!table.by_index.is_empty());
        assert!(!table.by_key.is_empty());
    }

    #[test]
    fn parses_synthetic_monstats_record() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1u32.to_le_bytes());
        let mut record = vec![0; MONSTATS_RECORD_SIZE];
        record[0x00..0x02].copy_from_slice(&7u16.to_le_bytes());
        record[0x02..0x04].copy_from_slice(&6u16.to_le_bytes());
        record[0x04..0x06].copy_from_slice(&8u16.to_le_bytes());
        record[0x06..0x08].copy_from_slice(&123u16.to_le_bytes());
        record[0x08..0x0a].copy_from_slice(&124u16.to_le_bytes());
        record[0x0c..0x10].copy_from_slice(&0x1122_3344u32.to_le_bytes());
        record[0x10..0x14].copy_from_slice(&0x5566_7788u32.to_le_bytes());
        bytes.extend_from_slice(&record);

        let records = parse_monstats_bin(&bytes).expect("monstats");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, 7);
        assert_eq!(records[0].name_lang_id, 123);
        assert_eq!(records[0].flags, 0x1122_3344);
    }

    #[test]
    fn parses_synthetic_object_level_and_item_records() {
        let mut objects = Vec::new();
        objects.extend_from_slice(&1u32.to_le_bytes());
        let mut object = vec![0; OBJECTS_RECORD_SIZE];
        object[0..6].copy_from_slice(b"Chest\0");
        object[0x80..0x83].copy_from_slice(b"CHE");
        object[0x13a] = 1;
        object[0x148..0x14c].copy_from_slice(&3u32.to_le_bytes());
        object[0x14c..0x150].copy_from_slice(&4u32.to_le_bytes());
        objects.extend_from_slice(&object);

        let mut levels = Vec::new();
        levels.extend_from_slice(&1u32.to_le_bytes());
        let mut level = vec![0; LEVELS_RECORD_SIZE];
        level[0..2].copy_from_slice(&2u16.to_le_bytes());
        level[3] = 1;
        level[0xf5..0xf5 + 7].copy_from_slice(b"Town\0\0\0");
        levels.extend_from_slice(&level);

        let mut items = Vec::new();
        items.extend_from_slice(&1u32.to_le_bytes());
        let mut item = vec![0; ITEM_RECORD_SIZE];
        item[0x80..0x84].copy_from_slice(b"cap ");
        item[0xf4..0xf6].copy_from_slice(&50u16.to_le_bytes());
        items.extend_from_slice(&item);

        assert_eq!(
            parse_objects_bin(&objects).expect("objects")[0].name,
            "Chest"
        );
        assert_eq!(parse_levels_bin(&levels).expect("levels")[0].name, "Town");
        assert_eq!(parse_items_bin(&items).expect("items")[0].code, "cap");
    }

    #[test]
    fn loads_local_lod_install_when_available() {
        let Some(path) = local_test_install() else {
            return;
        };

        let data = GameData::load_classic_lod_install(&path).expect("local LoD static data");

        assert!(data.monster_count() > 0);
        assert!(data.object_count() > 0);
        assert!(data.level_count() > 0);
        assert!(data.item_count() > 0);
        assert!(data.monster_name(0).is_some());
    }

    fn local_test_install() -> Option<PathBuf> {
        std::env::var_os("LIBD2_D2_INSTALL").map(PathBuf::from)
    }
}
