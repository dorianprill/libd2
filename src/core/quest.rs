/// Packet-derived local-player quest-log state for one difficulty.
///
/// Legacy D2GS packet `0x52 PlayerQuestLog` carries 41 bytes. That count matches
/// the 41 two-byte quest-log words in the legacy `.d2s` quest section for one
/// difficulty, so this type models the packet as the low byte of each quest-log
/// slot in save-file order. That gives stable names for mandatory quests, optional
/// quests, act-travel markers, and the few non-quest slots preserved by the
/// original layout without inventing richer semantics than the packet exposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerQuestLog {
    entries: [u8; Self::ENTRY_COUNT],
}

impl PlayerQuestLog {
    pub const ENTRY_COUNT: usize = 41;

    pub fn new(entries: [u8; Self::ENTRY_COUNT]) -> Self {
        Self { entries }
    }

    pub fn raw_entries(&self) -> &[u8; Self::ENTRY_COUNT] {
        &self.entries
    }

    pub fn entry(&self, entry: QuestLogEntry) -> QuestLogEntryState {
        QuestLogEntryState::new(self.entries[entry.index()])
    }
}

impl Default for PlayerQuestLog {
    fn default() -> Self {
        Self {
            entries: [0; Self::ENTRY_COUNT],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestLogEntryState {
    raw: u8,
}

impl QuestLogEntryState {
    pub const COMPLETED_BIT: u8 = 1 << 0;
    pub const REQUIREMENT_COMPLETED_BIT: u8 = 1 << 1;

    pub const fn new(raw: u8) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> u8 {
        self.raw
    }

    pub const fn is_set(self) -> bool {
        self.raw != 0
    }

    pub const fn is_completed(self) -> bool {
        self.raw & Self::COMPLETED_BIT != 0
    }

    pub const fn is_requirement_completed(self) -> bool {
        self.raw & Self::REQUIREMENT_COMPLETED_BIT != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum QuestLogEntry {
    ActIIntroduction = 0,
    DenOfEvil = 1,
    SistersBurialGrounds = 2,
    ToolsOfTheTrade = 3,
    TheSearchForCain = 4,
    TheForgottenTower = 5,
    SistersToTheSlaughter = 6,
    TravelToActII = 7,
    ActIIIntroduction = 8,
    RadamentsLair = 9,
    TheHoradricStaff = 10,
    TaintedSun = 11,
    ArcaneSanctuary = 12,
    TheSummoner = 13,
    TheSevenTombs = 14,
    TravelToActIII = 15,
    ActIIIIntroduction = 16,
    LamEsensTome = 17,
    KhalimsWill = 18,
    BladeOfTheOldReligion = 19,
    TheGoldenBird = 20,
    TheBlackenedTemple = 21,
    TheGuardian = 22,
    TravelToActIV = 23,
    ActIVIntroduction = 24,
    TheFallenAngel = 25,
    TerrorsEnd = 26,
    Hellforge = 27,
    ActIVUnused1 = 28,
    ActIVUnused2 = 29,
    ActIVUnused3 = 30,
    TravelToActV = 31,
    PostTerrorsEndCain = 32,
    ActVPadding1 = 33,
    ActVPadding2 = 34,
    SiegeOnHarrogath = 35,
    RescueOnMountArreat = 36,
    PrisonOfIce = 37,
    BetrayalOfHarrogath = 38,
    RiteOfPassage = 39,
    EveOfDestruction = 40,
}

impl QuestLogEntry {
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Quest-log words stored per difficulty in legacy `.d2s` files.
pub const SAVE_QUEST_WORDS_PER_DIFFICULTY: usize = 48;
/// Quest-log bytes stored per difficulty in legacy `.d2s` files.
pub const SAVE_QUEST_BYTES_PER_DIFFICULTY: usize = SAVE_QUEST_WORDS_PER_DIFFICULTY * 2;
/// Bytes from the `Woo!` marker through the fixed quest-section header.
pub const SAVE_QUEST_SECTION_HEADER_BYTES: usize = 10;
/// Legacy quest-section marker.
pub const SAVE_QUEST_SECTION_MARKER: [u8; 4] = *b"Woo!";
/// Fixed bytes after `Woo!` in legacy quest sections.
pub const SAVE_QUEST_SECTION_HEADER_AFTER_MARKER: [u8; 6] = [0x06, 0x00, 0x00, 0x00, 0x2a, 0x01];

/// Quest reward/completion bit granted by the game.
pub const QUEST_REWARD_GRANTED: u16 = 0x0001;
/// Quest reward bit used while an NPC reward is pending.
pub const QUEST_REWARD_PENDING: u16 = 0x0002;
/// Act V Prison of Ice bit that tracks consumed Malah resistance scrolls.
pub const QUEST_PRISON_OF_ICE_SCROLL_CONSUMED: u16 = 0x0080;
/// Quest-history bit used by already completed quests.
pub const QUEST_LOG_CLOSED: u16 = 0x1000;
/// Completion bits d2sed/libd2 actively normalize when toggling quests.
pub const QUEST_COMPLETION_MASK: u16 =
    QUEST_REWARD_GRANTED | QUEST_REWARD_PENDING | QUEST_LOG_CLOSED;
/// Normalized completed quest word for visible reward quests.
pub const QUEST_CLOSED_COMPLETE: u16 = QUEST_REWARD_GRANTED | QUEST_LOG_CLOSED;
/// Hidden word used after completing a difficulty in legacy saves.
pub const DIFFICULTY_COMPLETED_WORD: u16 = 0x8001;

/// Legacy header progression byte offset.
pub const LEGACY_PROGRESSION_OFFSET: usize = 0x25;
/// Header progression after Normal Baal is completed.
pub const PROGRESSION_NORMAL_UNLOCKED: u8 = 0x05;
/// Header progression after Nightmare Baal is completed.
pub const PROGRESSION_NIGHTMARE_UNLOCKED: u8 = 0x0a;
/// Header progression after Hell Baal is completed.
pub const PROGRESSION_HELL_COMPLETED: u8 = 0x0f;

/// Save quest word index: hidden Act I completion/travel marker.
pub const ACT_I_COMPLETE: usize = 7;
/// Save quest word index: hidden Act II intro marker.
pub const ACT_II_INTRO: usize = 8;
/// Save quest word index: hidden Act II completion/travel marker.
pub const ACT_II_COMPLETE: usize = 15;
/// Save quest word index: hidden Act III intro marker.
pub const ACT_III_INTRO: usize = 16;
/// Save quest word index: hidden Act III completion/travel marker.
pub const ACT_III_COMPLETE: usize = 23;
/// Save quest word index: hidden Act IV intro marker.
pub const ACT_IV_INTRO: usize = 24;
/// Save quest word index: hidden Act IV completion marker.
pub const ACT_IV_COMPLETE: usize = 28;
/// Save quest word index: hidden Act V intro marker.
pub const ACT_V_INTRO: usize = 32;
/// Save quest word index: hidden difficulty completion marker.
pub const ACT_V_COMPLETE: usize = 41;

/// Save quest word index: Sisters to the Slaughter.
pub const SISTERS_TO_THE_SLAUGHTER: usize = QuestLogEntry::SistersToTheSlaughter as usize;
/// Save quest word index: The Seven Tombs.
pub const THE_SEVEN_TOMBS: usize = QuestLogEntry::TheSevenTombs as usize;
/// Save quest word index: The Guardian.
pub const THE_GUARDIAN: usize = QuestLogEntry::TheGuardian as usize;
/// Save quest word index: Terror's End.
pub const TERRORS_END: usize = QuestLogEntry::TerrorsEnd as usize;
/// Save quest word index: Prison of Ice.
pub const PRISON_OF_ICE: usize = QuestLogEntry::PrisonOfIce as usize;
/// Save quest word index: Eve of Destruction.
pub const EVE_OF_DESTRUCTION: usize = QuestLogEntry::EveOfDestruction as usize;

/// Visible quest indices in the same order shown by the in-game quest log.
pub const VISIBLE_QUEST_INDICES: [usize; 27] = [
    1, 2, 4, 5, 3, 6, 9, 10, 11, 12, 13, 14, 20, 19, 18, 17, 21, 22, 25, 26, 27, 35, 36, 37, 38,
    39, 40,
];

const ACT_I_QUESTS: [usize; 6] = [1, 2, 4, 5, 3, 6];
const ACT_II_QUESTS: [usize; 6] = [9, 10, 11, 12, 13, 14];
const ACT_III_QUESTS: [usize; 6] = [20, 19, 18, 17, 21, 22];
const ACT_IV_QUESTS: [usize; 3] = [25, 26, 27];
const ACT_V_QUESTS: [usize; 6] = [35, 36, 37, 38, 39, 40];

/// Visible quest group for one act.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestAct {
    /// Act display name.
    pub name: &'static str,
    /// Visible save quest word indices in in-game order.
    pub quest_indices: &'static [usize],
}

/// Visible quest groups in act order.
pub const VISIBLE_QUEST_ACTS: [QuestAct; 5] = [
    QuestAct {
        name: "Act I",
        quest_indices: &ACT_I_QUESTS,
    },
    QuestAct {
        name: "Act II",
        quest_indices: &ACT_II_QUESTS,
    },
    QuestAct {
        name: "Act III",
        quest_indices: &ACT_III_QUESTS,
    },
    QuestAct {
        name: "Act IV",
        quest_indices: &ACT_IV_QUESTS,
    },
    QuestAct {
        name: "Act V",
        quest_indices: &ACT_V_QUESTS,
    },
];

/// Returns the English quest name for a save quest word index.
pub const fn quest_name(index: usize) -> &'static str {
    match index {
        1 => "Den of Evil",
        2 => "Sisters' Burial Grounds",
        3 => "Tools of the Trade",
        4 => "The Search for Cain",
        5 => "The Forgotten Tower",
        6 => "Sisters to the Slaughter",
        9 => "Radament's Lair",
        10 => "The Horadric Staff",
        11 => "Tainted Sun",
        12 => "Arcane Sanctuary",
        13 => "The Summoner",
        14 => "The Seven Tombs",
        17 => "Lam Esen's Tome",
        18 => "Khalim's Will",
        19 => "Blade of the Old Religion",
        20 => "The Golden Bird",
        21 => "The Blackened Temple",
        22 => "The Guardian",
        25 => "The Fallen Angel",
        26 => "Terror's End",
        27 => "Hellforge",
        35 => "Siege on Harrogath",
        36 => "Rescue on Mount Arreat",
        37 => "Prison of Ice",
        38 => "Betrayal of Harrogath",
        39 => "Rite of Passage",
        40 => "Eve of Destruction",
        _ => "Unknown Quest",
    }
}

/// Returns whether a save quest word is completed.
pub const fn quest_is_completed(word: u16) -> bool {
    word & QUEST_REWARD_GRANTED != 0
}

/// Sets or clears normalized completion bits on a save quest word.
pub fn set_quest_completed(word: &mut u16, completed: bool) {
    if completed {
        *word &= !QUEST_REWARD_PENDING;
        *word |= QUEST_CLOSED_COMPLETE;
    } else {
        *word &= !QUEST_COMPLETION_MASK;
    }
}

fn set_bool_word(word: &mut u16, completed: bool) {
    *word = if completed { QUEST_REWARD_GRANTED } else { 0 };
}

fn set_difficulty_completed_word(word: &mut u16, completed: bool) {
    *word = if completed {
        DIFFICULTY_COMPLETED_WORD
    } else {
        0
    };
}

/// Normalizes hidden progression/reward bits for one difficulty's quest words.
pub fn sync_quest_progression(quests: &mut [u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]) {
    for &idx in &VISIBLE_QUEST_INDICES {
        if quest_is_completed(quests[idx]) {
            quests[idx] &= !QUEST_REWARD_PENDING;
            quests[idx] |= QUEST_LOG_CLOSED;
        }
    }
    if quest_is_completed(quests[PRISON_OF_ICE]) {
        quests[PRISON_OF_ICE] |= QUEST_PRISON_OF_ICE_SCROLL_CONSUMED;
    } else {
        quests[PRISON_OF_ICE] &= !QUEST_PRISON_OF_ICE_SCROLL_CONSUMED;
    }

    let act_i_complete = quest_is_completed(quests[SISTERS_TO_THE_SLAUGHTER]);
    let act_ii_complete = quest_is_completed(quests[THE_SEVEN_TOMBS]);
    let act_iii_complete = quest_is_completed(quests[THE_GUARDIAN]);
    let act_iv_complete = quest_is_completed(quests[TERRORS_END]);
    let act_v_complete = quest_is_completed(quests[EVE_OF_DESTRUCTION]);

    set_bool_word(&mut quests[ACT_I_COMPLETE], act_i_complete);
    if act_i_complete {
        set_bool_word(&mut quests[ACT_II_INTRO], true);
    }

    set_bool_word(&mut quests[ACT_II_COMPLETE], act_ii_complete);
    if act_ii_complete {
        set_bool_word(&mut quests[ACT_III_INTRO], true);
    }

    set_bool_word(&mut quests[ACT_III_COMPLETE], act_iii_complete);
    if act_iii_complete {
        set_bool_word(&mut quests[ACT_IV_INTRO], true);
    }

    set_bool_word(&mut quests[ACT_IV_COMPLETE], act_iv_complete);
    if act_iv_complete {
        set_bool_word(&mut quests[ACT_V_INTRO], true);
    }

    set_difficulty_completed_word(&mut quests[ACT_V_COMPLETE], act_v_complete);
}

/// Builds initial quest words with standard act-introduction markers.
pub fn initial_template_quests() -> [[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3] {
    let mut quests = [[0u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3];
    for difficulty in &mut quests {
        for &idx in &[
            QuestLogEntry::ActIIntroduction.index(),
            ACT_II_INTRO,
            ACT_III_INTRO,
            ACT_IV_INTRO,
            ACT_V_INTRO,
        ] {
            difficulty[idx] = QUEST_REWARD_GRANTED;
        }
    }
    quests
}

/// Parses legacy `.d2s` quest words from a raw save buffer.
pub fn parse_legacy_quest_words(
    raw: &[u8],
    start_offset: usize,
) -> Option<[[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3]> {
    let search_space = raw.get(start_offset..)?;
    let marker = search_space
        .windows(SAVE_QUEST_SECTION_MARKER.len())
        .position(|window| window == SAVE_QUEST_SECTION_MARKER)?
        + start_offset;
    let mut offset = marker + SAVE_QUEST_SECTION_HEADER_BYTES;
    let mut quests = [[0u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3];

    for difficulty in &mut quests {
        if offset + SAVE_QUEST_BYTES_PER_DIFFICULTY <= raw.len() {
            for (idx, word) in difficulty.iter_mut().enumerate() {
                let pos = offset + idx * 2;
                *word = u16::from_le_bytes([raw[pos], raw[pos + 1]]);
            }
            offset += SAVE_QUEST_BYTES_PER_DIFFICULTY;
        }
    }

    Some(quests)
}

/// Writes legacy `.d2s` quest words into a raw save buffer when the section exists.
pub fn write_legacy_quest_words(
    raw: &mut [u8],
    start_offset: usize,
    quests: &[[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3],
) -> bool {
    let Some(search_space) = raw.get(start_offset..) else {
        return false;
    };
    let Some(marker_rel) = search_space
        .windows(SAVE_QUEST_SECTION_MARKER.len())
        .position(|window| window == SAVE_QUEST_SECTION_MARKER)
    else {
        return false;
    };
    let marker = marker_rel + start_offset;

    if marker + SAVE_QUEST_SECTION_HEADER_BYTES <= raw.len() {
        raw[marker + SAVE_QUEST_SECTION_MARKER.len()..marker + SAVE_QUEST_SECTION_HEADER_BYTES]
            .copy_from_slice(&SAVE_QUEST_SECTION_HEADER_AFTER_MARKER);
    }

    let mut offset = marker + SAVE_QUEST_SECTION_HEADER_BYTES;
    for difficulty in quests {
        if offset + SAVE_QUEST_BYTES_PER_DIFFICULTY <= raw.len() {
            for (idx, &word) in difficulty.iter().enumerate() {
                let pos = offset + idx * 2;
                raw[pos..pos + 2].copy_from_slice(&word.to_le_bytes());
            }
            offset += SAVE_QUEST_BYTES_PER_DIFFICULTY;
        }
    }

    true
}

/// Converts packet `0x52 PlayerQuestLog` entries into legacy save quest words
/// for one difficulty.
///
/// The packet carries one byte per active quest-log slot. Visible completed
/// quests are promoted to the save-file "closed complete" state so exported
/// characters do not receive reward prompts again. Hidden travel/introduction
/// slots keep their low-byte packet value, then derived act/difficulty
/// progression is normalized from the visible completion flags.
pub fn quest_words_from_player_log(log: &PlayerQuestLog) -> [u16; SAVE_QUEST_WORDS_PER_DIFFICULTY] {
    let mut quests = [0u16; SAVE_QUEST_WORDS_PER_DIFFICULTY];

    for (idx, &raw) in log.raw_entries().iter().enumerate() {
        if idx >= SAVE_QUEST_WORDS_PER_DIFFICULTY {
            break;
        }

        let mut word = raw as u16;
        if VISIBLE_QUEST_INDICES.contains(&idx) && QuestLogEntryState::new(raw).is_completed() {
            set_quest_completed(&mut word, true);
        }
        quests[idx] = word;
    }

    sync_quest_progression(&mut quests);
    quests
}

/// Returns the legacy header progression byte implied by completed difficulties.
pub fn progression_from_quests(quests: &[[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3]) -> u8 {
    if quest_is_completed(quests[2][EVE_OF_DESTRUCTION]) {
        PROGRESSION_HELL_COMPLETED
    } else if quest_is_completed(quests[1][EVE_OF_DESTRUCTION]) {
        PROGRESSION_NIGHTMARE_UNLOCKED
    } else if quest_is_completed(quests[0][EVE_OF_DESTRUCTION]) {
        PROGRESSION_NORMAL_UNLOCKED
    } else {
        0
    }
}

/// Raises the legacy save header progression byte to match completed quests.
pub fn apply_progression_from_quests(
    raw: &mut [u8],
    _start_offset: usize,
    quests: &[[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3],
) {
    let progression = progression_from_quests(quests);
    // Just a basic fallback, D2R needs exact header layout offset.
    if let Some(byte) = raw.get_mut(LEGACY_PROGRESSION_OFFSET) {
        *byte = (*byte).max(progression);
    }
}

/// Returns one boolean per difficulty indicating consumed Malah scroll reward.
pub fn consumed_resistance_scrolls(
    quests: &[[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3],
) -> [bool; 3] {
    [
        quests[0][PRISON_OF_ICE] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED != 0,
        quests[1][PRISON_OF_ICE] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED != 0,
        quests[2][PRISON_OF_ICE] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED != 0,
    ]
}

/// Returns the total permanent all-resistance bonus from consumed Malah scrolls.
pub fn base_resistance_bonus(quests: &[[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3]) -> u32 {
    consumed_resistance_scrolls(quests)
        .iter()
        .filter(|&&consumed| consumed)
        .count() as u32
        * 10
}

/// Returns stat points granted by completing a visible quest index.
pub const fn stat_points_reward_for_quest(index: usize) -> u32 {
    if index == QuestLogEntry::LamEsensTome as usize {
        5
    } else {
        0
    }
}

/// Returns skill points granted by completing a visible quest index.
pub const fn skill_points_reward_for_quest(index: usize) -> u32 {
    match index {
        1 | 9 => 1,
        25 => 2,
        _ => 0,
    }
}

/// Returns stat points granted by completed quest rewards in all difficulties.
pub fn stat_points_from_quests(quests: &[[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3]) -> u32 {
    quests
        .iter()
        .filter(|difficulty| quest_is_completed(difficulty[QuestLogEntry::LamEsensTome.index()]))
        .count() as u32
        * stat_points_reward_for_quest(QuestLogEntry::LamEsensTome.index())
}

/// Returns skill points granted by completed quest rewards in all difficulties.
pub fn skill_points_from_quests(quests: &[[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3]) -> u32 {
    quests
        .iter()
        .map(|difficulty| {
            [1, 9, 25]
                .iter()
                .filter(|&&index| quest_is_completed(difficulty[index]))
                .map(|&index| skill_points_reward_for_quest(index))
                .sum::<u32>()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{
        ACT_IV_COMPLETE, ACT_V_COMPLETE, ACT_V_INTRO, DIFFICULTY_COMPLETED_WORD,
        EVE_OF_DESTRUCTION, LEGACY_PROGRESSION_OFFSET, PRISON_OF_ICE, PROGRESSION_HELL_COMPLETED,
        PlayerQuestLog, QUEST_LOG_CLOSED, QUEST_PRISON_OF_ICE_SCROLL_CONSUMED,
        QUEST_REWARD_GRANTED, QUEST_REWARD_PENDING, QuestLogEntry, QuestLogEntryState,
        consumed_resistance_scrolls, initial_template_quests, parse_legacy_quest_words,
        progression_from_quests, quest_is_completed, set_quest_completed, sync_quest_progression,
        write_legacy_quest_words,
    };

    #[test]
    fn player_quest_log_exposes_named_entries() {
        let mut entries = [0u8; PlayerQuestLog::ENTRY_COUNT];
        entries[QuestLogEntry::TheSearchForCain.index()] =
            QuestLogEntryState::COMPLETED_BIT | QuestLogEntryState::REQUIREMENT_COMPLETED_BIT;

        let log = PlayerQuestLog::new(entries);
        let cain = log.entry(QuestLogEntry::TheSearchForCain);

        assert!(cain.is_completed());
        assert!(cain.is_requirement_completed());
        assert_eq!(cain.raw(), 0b11);
    }

    #[test]
    fn sync_quest_progression_sets_hidden_act_progression_words() {
        let mut quests = [0u16; 48];
        for &idx in &super::VISIBLE_QUEST_INDICES {
            set_quest_completed(&mut quests[idx], true);
        }

        sync_quest_progression(&mut quests);

        for &idx in &super::VISIBLE_QUEST_INDICES {
            assert!(quest_is_completed(quests[idx]));
            assert_eq!(quests[idx] & QUEST_REWARD_PENDING, 0);
            assert_eq!(quests[idx] & QUEST_LOG_CLOSED, QUEST_LOG_CLOSED);
        }
        assert_eq!(quests[ACT_IV_COMPLETE], QUEST_REWARD_GRANTED);
        assert_eq!(quests[ACT_V_INTRO], QUEST_REWARD_GRANTED);
        assert_eq!(quests[ACT_V_COMPLETE], DIFFICULTY_COMPLETED_WORD);
        assert_eq!(
            quests[PRISON_OF_ICE] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED,
            QUEST_PRISON_OF_ICE_SCROLL_CONSUMED
        );
    }

    #[test]
    fn progression_tracks_completed_difficulties() {
        let mut quests = initial_template_quests();
        assert_eq!(progression_from_quests(&quests), 0);

        set_quest_completed(&mut quests[0][EVE_OF_DESTRUCTION], true);
        assert_eq!(
            progression_from_quests(&quests),
            super::PROGRESSION_NORMAL_UNLOCKED
        );

        set_quest_completed(&mut quests[2][EVE_OF_DESTRUCTION], true);
        assert_eq!(progression_from_quests(&quests), PROGRESSION_HELL_COMPLETED);

        let mut raw = [0u8; 0x30];
        super::apply_progression_from_quests(&mut raw, 0, &quests);
        assert_eq!(raw[LEGACY_PROGRESSION_OFFSET], PROGRESSION_HELL_COMPLETED);
    }

    #[test]
    fn prison_of_ice_scrolls_drive_resistance_bonus() {
        let mut quests = initial_template_quests();
        set_quest_completed(&mut quests[0][PRISON_OF_ICE], true);
        sync_quest_progression(&mut quests[0]);

        assert_eq!(consumed_resistance_scrolls(&quests), [true, false, false]);
        assert_eq!(super::base_resistance_bonus(&quests), 10);
    }

    #[test]
    fn quest_rewards_sum_across_difficulties() {
        let mut quests = initial_template_quests();
        for diff in &mut quests {
            set_quest_completed(&mut diff[QuestLogEntry::DenOfEvil.index()], true);
            set_quest_completed(&mut diff[QuestLogEntry::RadamentsLair.index()], true);
            set_quest_completed(&mut diff[QuestLogEntry::TheFallenAngel.index()], true);
            set_quest_completed(&mut diff[QuestLogEntry::LamEsensTome.index()], true);
        }

        assert_eq!(super::skill_points_from_quests(&quests), 12);
        assert_eq!(super::stat_points_from_quests(&quests), 15);
    }

    #[test]
    fn legacy_quest_words_parse_and_write_round_trip() {
        let mut raw = vec![0u8; 10 + 3 * 96];
        raw[0..4].copy_from_slice(&super::SAVE_QUEST_SECTION_MARKER);
        raw[4..10].copy_from_slice(&super::SAVE_QUEST_SECTION_HEADER_AFTER_MARKER);

        let mut quests = initial_template_quests();
        set_quest_completed(&mut quests[0][QuestLogEntry::DenOfEvil.index()], true);
        set_quest_completed(
            &mut quests[2][QuestLogEntry::EveOfDestruction.index()],
            true,
        );

        assert!(write_legacy_quest_words(&mut raw, 0, &quests));
        assert_eq!(parse_legacy_quest_words(&raw, 0), Some(quests));
    }

    #[test]
    fn player_quest_log_exports_completed_visible_quests_and_hidden_markers() {
        let mut entries = [0u8; PlayerQuestLog::ENTRY_COUNT];
        entries[QuestLogEntry::ActIIntroduction.index()] = QUEST_REWARD_GRANTED as u8;
        entries[QuestLogEntry::DenOfEvil.index()] =
            QuestLogEntryState::COMPLETED_BIT | QuestLogEntryState::REQUIREMENT_COMPLETED_BIT;
        entries[QuestLogEntry::SistersToTheSlaughter.index()] = QuestLogEntryState::COMPLETED_BIT;
        entries[QuestLogEntry::TravelToActII.index()] = QUEST_REWARD_GRANTED as u8;
        entries[QuestLogEntry::PrisonOfIce.index()] = QuestLogEntryState::COMPLETED_BIT;

        let quests = super::quest_words_from_player_log(&PlayerQuestLog::new(entries));

        assert_eq!(
            quests[QuestLogEntry::ActIIntroduction.index()],
            QUEST_REWARD_GRANTED
        );
        assert_eq!(
            quests[QuestLogEntry::DenOfEvil.index()],
            super::QUEST_CLOSED_COMPLETE
        );
        assert_eq!(
            quests[QuestLogEntry::TravelToActII.index()],
            QUEST_REWARD_GRANTED
        );
        assert_eq!(
            quests[QuestLogEntry::PrisonOfIce.index()] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED,
            QUEST_PRISON_OF_ICE_SCROLL_CONSUMED
        );
    }
}
