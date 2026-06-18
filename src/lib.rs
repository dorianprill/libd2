pub mod client;
pub mod core;

pub use self::client::Client;

pub use crate::core::act::Act;
pub use crate::core::area::Area;
pub use crate::core::character_class::CharacterClass;
pub use crate::core::character_file::{
    CharacterExportError, CharacterExportOptions, CharacterFile, CharacterFileError,
    CharacterHeader, CharacterHeaderLayout, CharacterItemLists, CharacterProgression,
    CharacterSkills, CharacterStat, CharacterStatEntry, CharacterStats, FollowerBlockHeader,
    IronGolemHeader, ItemListHeader, MercenaryHeader, SaveSectionMarker, calculate_checksum,
};
pub use crate::core::character_progression::{
    BaseStats, ClassGrowth, EXPERIENCE_BY_LEVEL, LEGACY_GOLD_MAX_ENCODED, MAX_CHARACTER_LEVEL,
    experience_for_level, max_inventory_gold, max_stash_gold, skill_points_from_level,
    stat_points_from_level,
};
pub use crate::core::coordinate::Coordinate;
pub use crate::core::data::{
    GameData, GameDataError, ItemRecord, LevelRecord, MonsterRecord, MonsterStateRecord,
    ObjectRecord,
};
pub use crate::core::entity::mercenary::{Mercenary, MercenaryClass};
pub use crate::core::entity::missile::Missile;
pub use crate::core::entity::npc::Npc;
pub use crate::core::entity::player::{
    PartyAffiliation, PartyId, PartyLifeFraction, Player, PlayerMovement, PlayerSkillLevels,
    PlayerVitals, RemotePartyInfo,
};
pub use crate::core::game_state::{
    Difficulty, GameMapState, GameServerType, GameState, ItemStatUpdate, Locale, PlayerCorpse,
    UnitKey, UnitStateSet,
};
pub use crate::core::inventory::{
    GridSize, InventoryProfile, ItemParent, ItemPosition, StoredItemContainer,
};
pub use crate::core::map::{
    CollisionFlag, CollisionGrid, GeneratedMap, GeneratedMapJsonError, MapGenerationError,
    MapGenerationRequest, MapGenerationRequestError, MapObject, MapObjectKind, MapPoint, MapSeed,
    MapSize, act_for_area, act_from_level_id, expand_rle_row, is_good_exit, is_valid_map_seed,
    rle_row_is_blocked,
};
pub use crate::core::mpq::{
    MpqArchive, MpqBlockEntry, MpqCompressionType, MpqError, MpqFileFlags, MpqFormatVersion,
    MpqHashEntry, MpqHashType, MpqHeader, decrypt_mpq_block, decrypt_mpq_block_range,
    encryption_table, mpq_decryption_key, mpq_hash,
};
pub use crate::core::network::connection::{
    CaptureInterfaceSelection, CaptureInterfaceSelectionReason, Connection, ConnectionEvent,
    ConnectionTransportWarning, D2gsBufferSnapshot, D2gsSessionResetReason, route_probe_local_ip,
    select_capture_interface,
};
pub use crate::core::network::d2gs::D2GSPacket;
pub use crate::core::object::WorldObject;
pub use crate::core::object::item::{
    Item, ItemAction, ItemAffixPair, ItemBufferCoord, ItemBufferId, ItemCategory, ItemCode,
    ItemContainer, ItemDestination, ItemDurability, ItemFlags, ItemOwner, ItemPacketData,
    ItemPlacement, ItemQuality, ItemRuneword, ItemStateFlags,
};
pub use crate::core::party::PartyAction;
pub use crate::core::protocol::ClientMessage;
pub use crate::core::protocol::ServerMessage;
pub use crate::core::protocol::server_message::{ServerMessageParseError, SkillDescription};
pub use crate::core::quest::{
    ACT_I_COMPLETE, ACT_II_COMPLETE, ACT_II_INTRO, ACT_III_COMPLETE, ACT_III_INTRO,
    ACT_IV_COMPLETE, ACT_IV_INTRO, ACT_V_COMPLETE, ACT_V_INTRO, DIFFICULTY_COMPLETED_WORD,
    EVE_OF_DESTRUCTION, LEGACY_PROGRESSION_OFFSET, PRISON_OF_ICE, PROGRESSION_HELL_COMPLETED,
    PROGRESSION_NIGHTMARE_UNLOCKED, PROGRESSION_NORMAL_UNLOCKED, PlayerQuestLog,
    QUEST_CLOSED_COMPLETE, QUEST_COMPLETION_MASK, QUEST_LOG_CLOSED,
    QUEST_PRISON_OF_ICE_SCROLL_CONSUMED, QUEST_REWARD_GRANTED, QUEST_REWARD_PENDING, QuestAct,
    QuestLogEntry, QuestLogEntryState, SAVE_QUEST_BYTES_PER_DIFFICULTY,
    SAVE_QUEST_SECTION_HEADER_AFTER_MARKER, SAVE_QUEST_SECTION_HEADER_BYTES,
    SAVE_QUEST_SECTION_MARKER, SAVE_QUEST_WORDS_PER_DIFFICULTY, SISTERS_TO_THE_SLAUGHTER,
    TERRORS_END, THE_GUARDIAN, THE_SEVEN_TOMBS, VISIBLE_QUEST_ACTS, VISIBLE_QUEST_INDICES,
    apply_progression_from_quests, base_resistance_bonus, consumed_resistance_scrolls,
    initial_template_quests, parse_legacy_quest_words, progression_from_quests, quest_is_completed,
    quest_name, quest_words_from_player_log, set_quest_completed, skill_points_from_quests,
    skill_points_reward_for_quest, stat_points_from_quests, stat_points_reward_for_quest,
    sync_quest_progression, write_legacy_quest_words,
};
pub use crate::core::skills::{
    SkillCategory, SkillRequirement, can_decrease_skill, can_increase_skill, decrease_skill,
    has_allocated_dependent_skill, has_required_level_for_skill_tree, increase_skill,
    missing_skill_prereqs, skill_categories, skill_depends_on, skill_name,
    skill_points_needed_to_increase, skill_requirement,
};
pub use crate::core::unit_stat::UnitStat;
pub use crate::core::version::{
    CharacterStatus, ExpansionMode, GameEdition, SaveVersion, detect_edition,
};
pub use crate::core::waypoint::{
    LEGACY_WAYPOINT_BYTES_PER_DIFFICULTY, LEGACY_WAYPOINT_DATA_BYTES,
    LEGACY_WAYPOINT_DIFFICULTY_HEADER_BYTES, LEGACY_WAYPOINT_SECTION_HEADER_AFTER_MARKER,
    LEGACY_WAYPOINT_SECTION_HEADER_BYTES, LEGACY_WAYPOINT_SECTION_MARKER, LEGACY_WAYPOINT_TRAILER,
    LEGACY_WAYPOINT_TRAILER_OFFSET, PlayerWaypointState, V105_WAYPOINT_SECTION_HEADER_AFTER_MARKER,
    WAYPOINT_ACTS, WAYPOINT_COUNT, WAYPOINT_NAMES, WaypointAct, parse_legacy_waypoints,
    write_legacy_waypoints, write_v105_waypoints,
};
